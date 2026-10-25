//! Replay, save/load and branching.
//!
//! A replay is the game seed, the ordered per-seat deck snapshots, the rules
//! revision, and the ordered list of accepted actions. Re-running it must
//! reproduce the same canonical state digest. Version 2 is the multi-seat
//! format; version-1 two-seat files are explicitly rejected, never reinterpreted.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::{Action, Submission};
use crate::deck::DeckList;
use crate::engine::Game;
use crate::error::EngineError;
use crate::event::Event;
use crate::ids::{CommanderId, MAX_PLAYERS, MIN_PLAYERS, PlayerId};
use crate::sandbox::SandboxRecord;
use crate::settings::AiSettings;
use crate::state::{Outcome, Profile};

pub const REPLAY_VERSION: u32 = 6;
pub const SAVE_VERSION: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayStep {
    pub player: PlayerId,
    pub revision: u64,
    pub decision: u64,
    pub action: Action,
}

/// Explicit per-seat commander designation in the command-zone practice
/// profile. Saved with the replay so setup is never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderSetup {
    pub seat: PlayerId,
    pub card: crate::card::CardId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub version: u32,
    pub seed: u64,
    pub first_player: PlayerId,
    /// Ordered seat snapshots; index == seat. The length is the seat count.
    pub seats: Vec<DeckList>,
    /// Empty for ordinary 20-life practice. Non-empty means the explicit
    /// command-zone practice profile with exactly one card per seat.
    pub commanders: Vec<CommanderSetup>,
    /// Explicit named profile, serialized so starting life and command-zone
    /// semantics are never inferred from the presence of commanders alone.
    pub profile: Profile,
    pub rules_revision: String,
    pub ai: AiSettings,
    pub steps: Vec<ReplayStep>,
    /// Outcome recorded when the actions were first played, if the game ended.
    pub outcome: Option<Outcome>,
    /// Set when the original run was cut off by a run cap. Not a result.
    pub truncated: Option<String>,
    pub final_digest: String,
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error(
        "unsupported replay/save version {got}; this build writes replay {expected}. \
         Older two-seat files are not silently reinterpreted as multi-seat files"
    )]
    Version { got: u32, expected: u32 },
    #[error("unsupported seat count {got}; supported range is {MIN_PLAYERS}..={MAX_PLAYERS}")]
    SeatCount { got: usize },
    #[error("invalid seat layout: {reason}")]
    SeatOrder { reason: String },
    #[error("recorded first player P{expected} does not match the re-created game's P{got}")]
    FirstPlayerMismatch { expected: PlayerId, got: PlayerId },
    #[error(
        "recorded rules revision {recorded} does not match this build's pinned revision {current}"
    )]
    RulesRevisionMismatch { recorded: String, current: String },
    #[error("save invariant violated: {reason}")]
    Invariant { reason: String },
    #[error("could not construct the recorded game: {source}")]
    Setup {
        #[source]
        source: EngineError,
    },
    #[error("step {index} (P{player}, {kind}) was rejected: {source}")]
    Step {
        index: usize,
        player: PlayerId,
        kind: &'static str,
        #[source]
        source: EngineError,
    },
    #[error("replay ended early: digest {got} != recorded {expected}")]
    DigestMismatch { expected: String, got: String },
    #[error("save digest {stored} does not match its own state {recomputed}")]
    SaveDigestMismatch { stored: String, recomputed: String },
    #[error("replay/save record is malformed: {detail}")]
    Malformed { detail: String },
}

#[derive(Deserialize)]
struct VersionProbe {
    version: Option<u32>,
}

fn probe_version(text: &str, expected: u32) -> Result<(), ReplayError> {
    let probe: VersionProbe = serde_json::from_str(text).map_err(|e| ReplayError::Malformed {
        detail: e.to_string(),
    })?;
    let got = probe.version.ok_or_else(|| ReplayError::Malformed {
        detail: "missing version field".to_string(),
    })?;
    if got != expected {
        return Err(ReplayError::Version { got, expected });
    }
    Ok(())
}

impl Replay {
    pub fn new(
        seed: u64,
        first_player: PlayerId,
        seats: Vec<DeckList>,
        ai: AiSettings,
    ) -> Result<Self, ReplayError> {
        let replay = Replay {
            version: REPLAY_VERSION,
            seed,
            first_player,
            seats,
            commanders: Vec::new(),
            profile: Profile::Practice20,
            rules_revision: crate::RULES_REVISION.to_string(),
            ai,
            steps: Vec::new(),
            outcome: None,
            truncated: None,
            final_digest: String::new(),
        };
        replay.validate()?;
        Ok(replay)
    }

    pub fn new_command_zone(
        seed: u64,
        first_player: PlayerId,
        seats: Vec<DeckList>,
        commanders: Vec<CommanderSetup>,
        ai: AiSettings,
    ) -> Result<Self, ReplayError> {
        let mut replay = Replay::new(seed, first_player, seats, ai)?;
        replay.commanders = commanders;
        replay.profile = Profile::CommandZone20;
        replay.validate()?;
        Ok(replay)
    }

    pub fn new_commander_combat(
        seed: u64,
        first_player: PlayerId,
        seats: Vec<DeckList>,
        commanders: Vec<CommanderSetup>,
        ai: AiSettings,
    ) -> Result<Self, ReplayError> {
        let mut replay = Replay::new(seed, first_player, seats, ai)?;
        replay.commanders = commanders;
        replay.profile = Profile::CommanderCombat40;
        replay.validate()?;
        Ok(replay)
    }

    fn validate(&self) -> Result<(), ReplayError> {
        let got = self.seats.len();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&got) {
            return Err(ReplayError::SeatCount { got });
        }
        if self.profile.has_command_zone() {
            if self.commanders.len() != got {
                return Err(ReplayError::SeatOrder {
                    reason: format!(
                        "{} commander spec(s) for {got} seat(s); exactly one per seat is required",
                        self.commanders.len()
                    ),
                });
            }
            for seat in 0..got as PlayerId {
                let entries: Vec<&CommanderSetup> = self
                    .commanders
                    .iter()
                    .filter(|setup| setup.seat == seat)
                    .collect();
                if entries.len() != 1 {
                    return Err(ReplayError::SeatOrder {
                        reason: format!("P{seat} must have exactly one commander"),
                    });
                }
                if !entries[0].card.is_supported_commander() {
                    return Err(ReplayError::SeatOrder {
                        reason: format!(
                            "{} is not in the supported command-zone practice commander profile",
                            entries[0].card.name()
                        ),
                    });
                }
            }
        } else if !self.commanders.is_empty() {
            return Err(ReplayError::SeatOrder {
                reason: format!("{} cannot carry a commander setup", self.profile.label()),
            });
        }
        if self.first_player as usize >= got {
            return Err(ReplayError::SeatOrder {
                reason: format!(
                    "first player P{} is outside the {got}-seat game",
                    self.first_player
                ),
            });
        }
        if self.rules_revision != crate::RULES_REVISION {
            return Err(ReplayError::RulesRevisionMismatch {
                recorded: self.rules_revision.clone(),
                current: crate::RULES_REVISION.to_string(),
            });
        }
        self.ai
            .validate_for(got)
            .map_err(|reason| ReplayError::SeatOrder { reason })?;
        Ok(())
    }

    /// Cut a new branch line from this recorded line at an explicitly chosen
    /// action boundary: the branch keeps exactly the first `keep` recorded
    /// steps, and anything played from there is new branching history. The
    /// branch keeps the same seed and deck snapshots, so playing it forward is
    /// deterministic. The recorded outcome and final digest are cleared — they
    /// describe the base line, not the branch.
    pub fn branch_at(&self, keep: usize) -> Result<Replay, ReplayError> {
        self.validate()?;
        if keep > self.steps.len() {
            return Err(ReplayError::Invariant {
                reason: format!(
                    "branch point {keep} is beyond the {} recorded action(s)",
                    self.steps.len()
                ),
            });
        }
        let mut branch = self.clone();
        branch.steps.truncate(keep);
        branch.outcome = None;
        branch.truncated = None;
        branch.final_digest.clear();
        Ok(branch)
    }

    /// Replay only the recorded line and hand back the resulting state and
    /// log, without a digest assertion against the recording. Used by the
    /// branch application path where the recorded tail belongs to the base.
    pub fn run_recorded(&self) -> Result<(Game, Vec<Event>), ReplayError> {
        if self.version != REPLAY_VERSION {
            return Err(ReplayError::Version {
                got: self.version,
                expected: REPLAY_VERSION,
            });
        }
        self.validate()?;
        let specs: Vec<(PlayerId, crate::card::CardId)> = self
            .commanders
            .iter()
            .map(|setup| (setup.seat, setup.card))
            .collect();
        let (mut game, mut log) = match self.profile {
            Profile::Practice20 => Game::new_logged(self.seed, &self.seats),
            Profile::CommandZone20 => Game::new_command_zone_logged(self.seed, &self.seats, &specs),
            Profile::CommanderCombat40 => {
                Game::new_commander_combat_logged(self.seed, &self.seats, &specs)
            }
        }
        .map_err(|source| ReplayError::Setup { source })?;
        if game.state.first_player != self.first_player {
            return Err(ReplayError::FirstPlayerMismatch {
                expected: self.first_player,
                got: game.state.first_player,
            });
        }
        for (index, step) in self.steps.iter().enumerate() {
            let submission = Submission {
                player: step.player,
                revision: step.revision,
                decision: step.decision,
                action: step.action.clone(),
            };
            match game.submit(&submission) {
                Ok(events) => log.extend(events),
                Err(source) => {
                    return Err(ReplayError::Step {
                        index,
                        player: step.player,
                        kind: step.action.kind(),
                        source,
                    });
                }
            }
        }
        Ok((game, log))
    }

    pub fn record(&mut self, step: ReplayStep) {
        self.steps.push(step);
    }

    pub fn run(&self) -> Result<(Game, Vec<Event>), ReplayError> {
        if self.version != REPLAY_VERSION {
            return Err(ReplayError::Version {
                got: self.version,
                expected: REPLAY_VERSION,
            });
        }
        self.validate()?;
        let specs: Vec<(PlayerId, crate::card::CardId)> = self
            .commanders
            .iter()
            .map(|setup| (setup.seat, setup.card))
            .collect();
        let (mut game, mut log) = match self.profile {
            Profile::Practice20 => Game::new_logged(self.seed, &self.seats),
            Profile::CommandZone20 => Game::new_command_zone_logged(self.seed, &self.seats, &specs),
            Profile::CommanderCombat40 => {
                Game::new_commander_combat_logged(self.seed, &self.seats, &specs)
            }
        }
        .map_err(|source| ReplayError::Setup { source })?;
        if game.state.first_player != self.first_player {
            return Err(ReplayError::FirstPlayerMismatch {
                expected: self.first_player,
                got: game.state.first_player,
            });
        }
        for (index, step) in self.steps.iter().enumerate() {
            let submission = Submission {
                player: step.player,
                revision: step.revision,
                decision: step.decision,
                action: step.action.clone(),
            };
            match game.submit(&submission) {
                Ok(events) => log.extend(events),
                Err(source) => {
                    return Err(ReplayError::Step {
                        index,
                        player: step.player,
                        kind: step.action.kind(),
                        source,
                    });
                }
            }
        }
        if !self.final_digest.is_empty() {
            let got = game.digest();
            if got != self.final_digest {
                return Err(ReplayError::DigestMismatch {
                    expected: self.final_digest.clone(),
                    got,
                });
            }
        }
        Ok((game, log))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("replay serializes")
    }

    pub fn from_json(text: &str) -> Result<Self, ReplayError> {
        probe_version(text, REPLAY_VERSION)?;
        let replay: Replay = serde_json::from_str(text).map_err(|e| ReplayError::Malformed {
            detail: e.to_string(),
        })?;
        replay.validate()?;
        Ok(replay)
    }
}

/// One compared point between two recorded lines. Only the public axis of the
/// accepted action (seat, decision serial and action kind) is recorded — never
/// a card name, target or zone contents — so a comparison shown by default
/// leaks no private or future information beyond what both lines already
/// recorded publicly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchDiffLine {
    pub step: usize,
    pub player: PlayerId,
    pub decision: u64,
    pub base: Option<&'static str>,
    pub branch: Option<&'static str>,
}

/// Public comparison of two replay lines that share the same setup. `branch` is
/// expected to agree with `base` over its common prefix (a branch is derived
/// from the base at an explicit action boundary); the lines start at the first
/// step where the accepted actions differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchDiff {
    /// Step index where the two lines first differ; `None` when they agree
    /// over the whole branch length so far.
    pub diverged_at: Option<usize>,
    pub lines: Vec<BranchDiffLine>,
    /// The base line is longer than the branch's current recorded history.
    /// Its continuation past the branch's live boundary is deliberately left
    /// out of `lines` and surfaced only as this note — never as action kinds —
    /// because on a live branch that tail is the base's future.
    pub base_tail_unrevealed: bool,
}

impl BranchDiff {
    /// Rendered comparison; every line is public-action-kind only.
    pub fn render(&self) -> String {
        let mut text = String::new();
        match self.diverged_at {
            Some(step) => text.push_str(&format!(
                "branches first differ at recorded action #{step}; then:\n"
            )),
            None => text.push_str(
                "branches agree so far: every accepted action on the branch is still on the base line\n",
            ),
        }
        for line in self.lines.iter().take(64) {
            text.push_str(&format!(
                "  #{step} P{player} (decision {decision}): base {} | branch {}\n",
                line.base.unwrap_or("(nothing — line has ended here)"),
                line.branch.unwrap_or("(nothing — line has ended here)"),
                step = line.step,
                player = line.player,
                decision = line.decision,
            ));
        }
        if self.lines.len() > 64 {
            text.push_str(&format!(
                "  … {} more compared action(s) not shown\n",
                self.lines.len() - 64
            ));
        }
        if self.base_tail_unrevealed {
            text.push_str(
                "  (the base line still extends past the branch's current recorded history — its continuation is not revealed here)\n",
            );
        }
        text
    }
}

/// Compare two recorded lines from the same setup. Legal-action *kinds* are the
/// vocabulary shown; the exact arguments (cards, targets, counts) are deliberately
/// omitted so the default comparison cannot leak private or future information.
/// The comparison also never reaches past the branch's own recorded history: a
/// diverged branch is typically still live, so the base's continuation after
/// the branch's last step is left out entirely and marked with
/// `base_tail_unrevealed` instead of being rendered.
pub fn compare_branches(base: &Replay, branch: &Replay) -> Result<BranchDiff, ReplayError> {
    let same_setup = base.seed == branch.seed
        && base.seats == branch.seats
        && base.profile == branch.profile
        && base.commanders == branch.commanders
        && base.first_player == branch.first_player;
    if !same_setup {
        return Err(ReplayError::Invariant {
            reason: "the two lines were not recorded from the same setup".to_string(),
        });
    }
    if base
        .steps
        .first()
        .zip(branch.steps.first())
        .is_some_and(|(a, b)| a != b)
    {
        return Err(ReplayError::Invariant {
            reason: "the two lines do not share their first recorded action".to_string(),
        });
    }
    let diverged_at =
        (0..branch.steps.len()).find(|index| base.steps.get(*index) != branch.steps.get(*index));
    let mut lines = Vec::new();
    if let Some(at) = diverged_at {
        // The compared span is bounded by the BRANCH's own recorded history.
        // Judging agreement uses only branch steps; once the branch line has
        // ended, the base's continuation is its future and is never rendered
        // as action kinds (it would reveal what the base line still holds in
        // store to whoever is playing the live branch).
        for step in at..branch.steps.len() {
            lines.push(BranchDiffLine {
                step,
                player: branch.steps[step].player,
                decision: branch.steps[step].decision,
                base: base.steps.get(step).map(|step| step.action.kind()),
                branch: branch.steps.get(step).map(|step| step.action.kind()),
            });
        }
    }
    let base_tail_unrevealed = base.steps.len() > branch.steps.len();
    Ok(BranchDiff {
        diverged_at,
        lines,
        base_tail_unrevealed,
    })
}

/// A live session snapshot: the full engine state plus what produced it. Loading
/// restores pending decisions, RNG position and AI settings exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSave {
    pub version: u32,
    pub game: crate::state::GameState,
    /// Ordered deck snapshots; must match `game.players` and `replay.seats`.
    pub seats: Vec<DeckList>,
    pub ai: AiSettings,
    pub replay: Replay,
    pub log: Vec<Event>,
    /// True once a sandbox tool has been used; certified analysis is void from
    /// that point (the UI labels this).
    pub sandbox_modified: bool,
    /// Typed record of every manual table override used in this session. Empty
    /// means no override was used; a non-empty record taints the game for the
    /// rest of the session, survives save/load and can never be presented as a
    /// legal match result.
    #[serde(default)]
    pub sandbox: SandboxRecord,
    pub digest: String,
}

impl SessionSave {
    pub fn capture(game: &Game, replay: Replay, log: Vec<Event>, sandbox_modified: bool) -> Self {
        SessionSave {
            version: SAVE_VERSION,
            game: game.state.clone(),
            seats: replay.seats.clone(),
            ai: replay.ai.clone(),
            replay,
            log,
            sandbox_modified,
            sandbox: SandboxRecord::default(),
            digest: game.digest(),
        }
    }

    /// Capture a session together with its typed sandbox record. The boolean
    /// flag is derived from the record, so the two can never disagree.
    pub fn capture_recorded(
        game: &Game,
        replay: Replay,
        log: Vec<Event>,
        sandbox: SandboxRecord,
    ) -> Self {
        let mut save = SessionSave::capture(game, replay, log, sandbox.is_tainted());
        save.sandbox = sandbox;
        save
    }

    /// True when any manual table override was used in this session.
    pub fn tainted(&self) -> bool {
        self.sandbox_modified || self.sandbox.is_tainted()
    }

    /// The sentence a result surface must use: a sandbox game is never a legal
    /// match result.
    pub fn record_line(&self) -> String {
        self.sandbox.record_line(self.game.outcome.as_ref())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("session serializes")
    }

    pub fn from_json(text: &str) -> Result<Self, ReplayError> {
        probe_version(text, SAVE_VERSION)?;
        let save: SessionSave = serde_json::from_str(text).map_err(|e| ReplayError::Malformed {
            detail: e.to_string(),
        })?;
        save.validate()?;
        Ok(save)
    }

    fn validate(&self) -> Result<(), ReplayError> {
        let seats = self.seats.len();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&seats) {
            return Err(ReplayError::SeatCount { got: seats });
        }
        if self.game.players.len() != seats {
            return Err(ReplayError::SeatOrder {
                reason: format!(
                    "save lists {seats} deck snapshot(s) but its state has {} seat(s)",
                    self.game.players.len()
                ),
            });
        }
        self.game
            .validate()
            .map_err(|source| ReplayError::Setup { source })?;
        self.ai
            .validate_for(seats)
            .map_err(|reason| ReplayError::SeatOrder { reason })?;
        if self.replay.version != REPLAY_VERSION {
            return Err(ReplayError::Version {
                got: self.replay.version,
                expected: REPLAY_VERSION,
            });
        }
        self.replay.validate()?;
        let expected_setup: Vec<(crate::ids::PlayerId, crate::card::CardId)> = self
            .replay
            .commanders
            .iter()
            .filter(|setup| self.game.is_live(setup.seat))
            .map(|setup| (setup.seat, setup.card))
            .collect();
        let actual_setup: Vec<(crate::ids::PlayerId, crate::card::CardId)> = self
            .game
            .commanders
            .iter()
            .map(|registry| (registry.owner, registry.card))
            .collect();
        if self.game.profile != self.replay.profile || expected_setup != actual_setup {
            return Err(ReplayError::Invariant {
                reason: format!(
                    "saved profile {:?}/commander setup does not match the game profile {:?}/registry",
                    self.replay.profile, self.game.profile
                ),
            });
        }
        // Identity binding: setup assigned CommanderId(i+1) by the supplied
        // replay commander order, not seat+1. Bind every live id to that
        // original seat+card, and every retired id to an original seat that has
        // actually departed. This rejects 1<->2 swaps and wrong-owner retirements
        // without inventing a per-id owner table.
        if self.game.profile.has_command_zone() {
            for (index, setup) in self.replay.commanders.iter().enumerate() {
                let id = CommanderId(index as u32 + 1);
                if let Some(live) = self.game.commanders.iter().find(|c| c.id == id) {
                    if live.owner != setup.seat || live.card != setup.card {
                        return Err(ReplayError::Invariant {
                            reason: format!(
                                "saved live {id} is bound to P{} {} but the original setup entry {index} is P{} {}",
                                live.owner,
                                live.card.name(),
                                setup.seat,
                                setup.card.name()
                            ),
                        });
                    }
                } else if self.game.retired_commanders.contains(&id) {
                    if self.game.is_live(setup.seat) {
                        return Err(ReplayError::Invariant {
                            reason: format!(
                                "saved {id} is retired but its original owner P{} is still live",
                                setup.seat
                            ),
                        });
                    }
                } else {
                    return Err(ReplayError::Invariant {
                        reason: format!(
                            "saved identity {id} is neither live nor retired in the original setup"
                        ),
                    });
                }
            }
        }
        if self.replay.first_player != self.game.first_player {
            return Err(ReplayError::Invariant {
                reason: format!(
                    "save first player P{} != replay first player P{}",
                    self.game.first_player, self.replay.first_player
                ),
            });
        }
        if self.replay.rules_revision != self.game.rules_revision {
            return Err(ReplayError::Invariant {
                reason: format!(
                    "save rules revision {} != replay rules revision {}",
                    self.game.rules_revision, self.replay.rules_revision
                ),
            });
        }
        if self.replay.seats != self.seats {
            return Err(ReplayError::SeatOrder {
                reason: "save deck snapshots do not match the embedded replay".to_string(),
            });
        }
        if self.replay.seed != self.game.seed {
            return Err(ReplayError::SeatOrder {
                reason: "save seed does not match the embedded replay".to_string(),
            });
        }
        let recomputed = crate::digest::digest(&self.game);
        if recomputed != self.digest {
            return Err(ReplayError::SaveDigestMismatch {
                stored: self.digest.clone(),
                recomputed,
            });
        }

        // The typed record and the boolean flag describe the same fact. A save
        // that carries overrides but claims to be untainted is refused: clearing
        // the flag by hand must not launder a sandbox game into a match result.
        if self.sandbox.is_tainted() && !self.sandbox_modified {
            return Err(ReplayError::Invariant {
                reason: "the save records sandbox overrides but does not mark the session sandbox-modified".to_string(),
            });
        }

        // A non-sandbox save must be exactly the state its embedded action
        // sequence produces. `Replay::run` is bounded by the step list and is
        // called on the load worker, never the UI frame thread.
        if !self.tainted() {
            const MAX_SAVE_REPLAY_STEPS: usize = 1_000_000;
            if self.replay.steps.len() > MAX_SAVE_REPLAY_STEPS {
                return Err(ReplayError::Malformed {
                    detail: format!(
                        "embedded replay has {} steps (limit {MAX_SAVE_REPLAY_STEPS})",
                        self.replay.steps.len()
                    ),
                });
            }
            let (replayed, _) = self.replay.run()?;
            if replayed.state != self.game {
                return Err(ReplayError::Invariant {
                    reason: format!(
                        "embedded replay produced digest {} but the saved state is {}",
                        replayed.digest(),
                        crate::digest::digest(&self.game)
                    ),
                });
            }
        }
        Ok(())
    }

    pub fn game(&self) -> Game {
        Game {
            state: self.game.clone(),
        }
    }
}
