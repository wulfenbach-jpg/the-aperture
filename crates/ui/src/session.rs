//! UI session state: the engine, the event log, the replay, AI settings, the
//! sandbox toolset, save/load and import. All strict actions go through the
//! engine's transactional `submit`; sandbox tools are clearly separated and
//! permanently mark the session as sandbox-modified.

use std::path::Path;

use aperture_ai::{DeciderConfig, DeciderPolicy, HeuristicPolicy, Policy};
use aperture_core::view::{PendingKind, PlayerView};
use aperture_core::{
    Action, AiSettings, BlockDecl, DeckList, Event, Game, ObjectId, P0, P1, PlayerId, Replay,
    ReplayStep, SessionSave, Submission, Zone,
};

pub enum SandboxOp {
    Draw(u8),
    Mill(u8),
    RevealTop(u8),
    Counter(ObjectId, i32),
}

pub struct UiSession {
    pub game: Game,
    pub log: Vec<Event>,
    pub replay: Replay,
    pub ai: AiSettings,
    pub deck_a: DeckList,
    pub deck_b: DeckList,
    pub seed: u64,
    pub sandbox_modified: bool,
    pub human_plays_ai_seat: bool,
    pub heuristic: HeuristicPolicy,
    pub decider: Option<DeciderPolicy>,
    pub note: Option<String>,
    pub message: String,
    pub revealed: Vec<String>,
    pub selected_attackers: Vec<ObjectId>,
    pub selected_blocks: Vec<(ObjectId, ObjectId)>,
    pub selected_cards: Vec<ObjectId>,
    pub target_pick: Option<(ObjectId, Vec<Action>)>,
    pub inspect: Option<String>,
    pub import_url: String,
    pub import_report: Option<String>,
    pub decider_status: Option<String>,
    pub save_path: String,
}

impl UiSession {
    pub fn new(
        seed: u64,
        deck_a: DeckList,
        deck_b: DeckList,
        ai: AiSettings,
        use_decider: bool,
        human_plays_ai_seat: bool,
    ) -> Self {
        let (game, log) = Game::new_logged(seed, &deck_a, &deck_b);
        let mut replay = Replay::new(seed, deck_a.clone(), deck_b.clone(), ai.clone());
        replay.steps.clear();
        let decider = if use_decider {
            match DeciderPolicy::new(DeciderConfig {
                timeout_ms: ai.decider_timeout_ms,
                ..DeciderConfig::default()
            }) {
                Ok(policy) => Some(policy),
                Err(err) => {
                    return UiSession {
                        game,
                        log,
                        replay,
                        ai,
                        deck_a,
                        deck_b,
                        seed,
                        sandbox_modified: false,
                        human_plays_ai_seat,
                        heuristic: HeuristicPolicy::new(),
                        decider: None,
                        note: Some(format!("decider not usable: {err}")),
                        message: String::new(),
                        revealed: Vec::new(),
                        selected_attackers: Vec::new(),
                        selected_blocks: Vec::new(),
                        selected_cards: Vec::new(),
                        target_pick: None,
                        inspect: None,
                        import_url: String::new(),
                        import_report: None,
                        decider_status: None,
                        save_path: "the-aperture-save.json".to_string(),
                    };
                }
            }
        } else {
            None
        };
        UiSession {
            game,
            log,
            replay,
            ai,
            deck_a,
            deck_b,
            seed,
            sandbox_modified: false,
            human_plays_ai_seat,
            heuristic: HeuristicPolicy::new(),
            decider,
            note: None,
            message: String::new(),
            revealed: Vec::new(),
            selected_attackers: Vec::new(),
            selected_blocks: Vec::new(),
            selected_cards: Vec::new(),
            target_pick: None,
            inspect: None,
            import_url: String::new(),
            import_report: None,
            decider_status: None,
            save_path: "the-aperture-save.json".to_string(),
        }
    }

    pub fn view(&self, seat: PlayerId) -> PlayerView {
        self.game.view_for(seat, &self.log)
    }

    pub fn owner(&self) -> Option<PlayerId> {
        self.game.decision_owner()
    }

    pub fn human_seat(&self) -> PlayerId {
        if self.ai.ai_seat == P0 { P1 } else { P0 }
    }

    /// True when a policy should act for `seat` now.
    pub fn seat_is_ai(&self, seat: PlayerId) -> bool {
        seat == self.ai.ai_seat || (seat == self.human_seat() && self.human_plays_ai_seat)
    }

    pub fn submit(&mut self, action: Action) -> Result<(), String> {
        let Some(owner) = self.owner() else {
            return Err("no player holds priority right now".to_string());
        };
        let revision = self.game.revision();
        let decision = self.game.decision();
        let submission = Submission {
            player: owner,
            revision,
            decision,
            action: action.clone(),
        };
        match self.game.submit(&submission) {
            Ok(events) => {
                self.log.extend(events);
                self.replay.record(ReplayStep {
                    player: owner,
                    revision,
                    decision,
                    action,
                });
                self.selected_attackers.clear();
                self.selected_blocks.clear();
                self.selected_cards.clear();
                self.target_pick = None;
                self.message.clear();
                Ok(())
            }
            Err(err) => {
                self.message = format!("refused: {err}");
                Err(err.to_string())
            }
        }
    }

    /// One AI decision for the seat that must act. Returns the decision source.
    pub fn step_ai(&mut self) -> Option<String> {
        let owner = self.owner()?;
        if !self.seat_is_ai(owner) {
            return None;
        }
        let view = self.view(owner);
        let decision = match (self.ai.use_decider, self.decider.as_mut()) {
            (true, Some(policy)) => policy.decide(&view),
            _ => self.heuristic.decide(&view),
        };
        self.note = decision.note.clone();
        let source = decision.source.label().to_string();
        if let Err(err) = self.submit(decision.action) {
            self.note = Some(format!("policy decision refused by the engine: {err}"));
            let _ = self.submit(Action::Pass);
        }
        Some(source)
    }

    pub fn new_game(&mut self, seed: u64) {
        self.seed = seed;
        let (game, log) = Game::new_logged(seed, &self.deck_a, &self.deck_b);
        self.game = game;
        self.log = log;
        self.replay = Replay::new(
            seed,
            self.deck_a.clone(),
            self.deck_b.clone(),
            self.ai.clone(),
        );
        self.replay.steps.clear();
        self.sandbox_modified = false;
        self.revealed.clear();
        self.message.clear();
        self.note = None;
    }

    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        // The save must describe the settings that are live right now, not the
        // ones the replay was created with.
        self.replay.ai = self.ai.clone();
        self.replay.outcome = self.game.state.outcome.clone();
        self.replay.final_digest = self.game.digest();
        let save = SessionSave::capture(
            &self.game,
            self.replay.clone(),
            self.log.clone(),
            self.sandbox_modified,
        );
        std::fs::write(path, save.to_json()).map_err(|e| e.to_string())?;
        self.message = format!(
            "saved to {} (digest {}){}",
            path.display(),
            &self.game.digest()[..12],
            if self.sandbox_modified {
                " — SANDBOX-MODIFIED: certified replay is void"
            } else {
                ""
            }
        );
        Ok(())
    }

    pub fn load(&mut self, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let save = SessionSave::from_json(&text).map_err(|e| e.to_string())?;
        self.game = save.game();
        self.log = save.log.clone();
        self.replay = save.replay.clone();
        self.ai = save.ai.clone();
        self.deck_a = save.deck_a.clone();
        self.deck_b = save.deck_b.clone();
        self.sandbox_modified = save.sandbox_modified;
        self.message = format!(
            "loaded {} — digest {} (stored {}), {} action(s) in the replay",
            path.display(),
            &self.game.digest()[..12],
            &save.digest[..12],
            save.replay.steps.len()
        );
        Ok(())
    }

    /// Sandbox tools. These bypass the rules on purpose; the session is marked
    /// so no certified claim can be made about the resulting position.
    pub fn sandbox(&mut self, op: SandboxOp) {
        let mut description = String::new();
        match op {
            SandboxOp::Draw(count) => {
                for _ in 0..count {
                    let seat = self.human_seat();
                    let next = self.game.state.players[seat as usize]
                        .library
                        .first()
                        .copied();
                    if let Some(id) = next {
                        self.game.state.players[seat as usize].library.remove(0);
                        self.game.state.players[seat as usize].hand.push(id);
                        if let Some(object) = self.game.state.objects.get_mut(&id) {
                            object.zone = Zone::Hand;
                        }
                        description = format!("SANDBOX: P{seat} drew a card outside the rules");
                    } else {
                        description = "SANDBOX: library is empty".to_string();
                        break;
                    }
                }
            }
            SandboxOp::Mill(count) => {
                for _ in 0..count {
                    let seat = self.human_seat();
                    let next = self.game.state.players[seat as usize]
                        .library
                        .first()
                        .copied();
                    if let Some(id) = next {
                        self.game.state.players[seat as usize].library.remove(0);
                        self.game.state.players[seat as usize].graveyard.push(id);
                        if let Some(object) = self.game.state.objects.get_mut(&id) {
                            object.zone = Zone::Graveyard;
                        }
                        description = format!("SANDBOX: P{seat} milled a card outside the rules");
                    } else {
                        description = "SANDBOX: library is empty".to_string();
                        break;
                    }
                }
            }
            SandboxOp::RevealTop(count) => {
                let seat = self.human_seat();
                let library = self.game.state.players[seat as usize].library.clone();
                self.revealed = library
                    .iter()
                    .take(count as usize)
                    .map(|id| self.game.state.object(*id).card.name().to_string())
                    .collect();
                description = format!(
                    "SANDBOX: revealed the top {} card(s) of P{seat}'s library to the table",
                    self.revealed.len()
                );
            }
            SandboxOp::Counter(id, delta) => {
                if let Some(object) = self.game.state.objects.get_mut(&id) {
                    let current = object.plus_one_counters as i32;
                    object.plus_one_counters = (current + delta).max(0) as u32;
                    description = format!(
                        "SANDBOX: {} now has {} +1/+1 counter(s) (storage only; combat math ignores them)",
                        object.card.name(),
                        object.plus_one_counters
                    );
                }
            }
        }
        if !description.is_empty() {
            self.sandbox_modified = true;
            self.log.push(Event::StateBasedAction { description });
        }
    }

    pub fn import_url(&mut self, cache_dir: &Path) {
        match aperture_import::import_url(&self.import_url, cache_dir) {
            Ok(report) => {
                let planned = match aperture_import::plan_deck(&report) {
                    Ok(list) => format!(
                        "\nPLAN: playable, {} card(s) across {} distinct cards",
                        list.size(),
                        list.cards.len()
                    ),
                    Err(err) => format!("\nPLAN REFUSED: {err}"),
                };
                self.import_report = Some(format!(
                    "{} '{}'\nsource: {}\ncards: {} ({} supported, {} unsupported, {} excluded, {} sideboard, {} commander)\n{} {}\nnotes: {}\nrefusal: {}{}",
                    report.provider,
                    report.deck_name,
                    report.source,
                    report.total_cards,
                    report.supported_cards,
                    report.unsupported_cards,
                    report.excluded.len(),
                    report.sideboard.len(),
                    report.commander.len(),
                    match &report.raw_sha256 {
                        Some(sha) => format!("response sha256: {}", &sha[..16.min(sha.len())]),
                        None => "(no cached response hash)".to_string(),
                    },
                    match &report.cached_path {
                        Some(path) => format!("cached at {}", path.display()),
                        None => String::new(),
                    },
                    report.notes.join(" | "),
                    report.refusal.clone().unwrap_or_else(|| "none".to_string()),
                    planned
                ));
            }
            Err(err) => {
                self.import_report = Some(format!("import failed: {err}"));
            }
        }
    }

    pub fn decider_check(&mut self) {
        let config = DeciderConfig {
            timeout_ms: self.ai.decider_timeout_ms,
            ..DeciderConfig::default()
        };
        match aperture_ai::DeciderClient::new(config) {
            Ok(client) => match client.health() {
                Ok(true) => {
                    self.decider_status =
                        Some("decider-2b /health reachable on http://127.0.0.1:8765".to_string())
                }
                Ok(false) => {
                    self.decider_status =
                        Some("decider-2b answered, but with an error status".to_string())
                }
                Err(err) => {
                    self.decider_status =
                        Some(format!("decider-2b unreachable ({err}); heuristic active"))
                }
            },
            Err(err) => self.decider_status = Some(format!("decider config refused: {err}")),
        }
    }

    /// Pending-choice helper text for the UI.
    pub fn pending_kind(&self, view: &PlayerView) -> Option<PendingKind> {
        view.pending_kind
    }

    pub fn conclude(&self) -> Option<String> {
        self.game
            .state
            .outcome
            .as_ref()
            .map(|outcome| match outcome {
                aperture_core::Outcome::Win { winner, reason } => {
                    format!("P{winner} wins — {reason}")
                }
            })
    }
}

/// Level of the practice profile shown in the UI header.
pub fn profile_line() -> String {
    format!(
        "practice slice · supported pool: {} cards · 1v1 · 20 life · London mulligan · NOT Commander",
        aperture_core::catalog::pool_size()
    )
}

pub fn bundled_deck(path: &Path, fallback: &str, name: &str) -> DeckList {
    if path.exists()
        && let Ok(text) = std::fs::read_to_string(path)
        && let Ok(list) = DeckList::parse_practice(name, &text)
    {
        return list;
    }
    DeckList::parse_practice(name, fallback).unwrap_or_else(|_| DeckList::new(name, Vec::new()))
}

pub const BUNDLED_A: &str = include_str!("../../../decks/temur-beats.txt");
pub const BUNDLED_B: &str = include_str!("../../../decks/izzet-control.txt");

pub fn sandbox_label() -> &'static str {
    "SANDBOX — these tools bypass the rules and void certified replay"
}

pub fn block_list(pairs: &[(ObjectId, ObjectId)]) -> Vec<BlockDecl> {
    pairs
        .iter()
        .map(|(blocker, attacker)| BlockDecl {
            blocker: *blocker,
            attacker: *attacker,
        })
        .collect()
}
