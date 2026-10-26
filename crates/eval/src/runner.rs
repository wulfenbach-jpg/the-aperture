//! The paired-seed evaluation runner: one real engine game per declared
//! (seed, rotation), policies selected only from engine-offered actions, every
//! accepted step recorded into a replay, and every emitted replay verified by
//! replaying it through the core engine.
//!
//! Labels matter: this driver runs on explicitly declared practice fixtures.
//! Its reports are practice/tooling evidence and NEVER satisfy the certified
//! corpus gate (#17 -> original #26 acceptance).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use aperture_ai::{
    Decision, DecisionSource, Knowledge, SearchCancellation, SearchConfig, fallback, search,
};
use aperture_core::{Action, DeckList, Event, Game, PlayerId, Replay, ReplayStep, Submission};

use crate::config::EvalConfig;
use crate::report::{GameRecord, Strategy};
use crate::sha256_hex;

const REPEAT_LIMIT: u8 = 3;

/// Seat 0 uses deck 0 and seat 1 deck 1 in all rotations; what rotates is which
/// seat the search policy occupies.
pub fn search_seat(rotation: u32) -> PlayerId {
    if rotation % 2 == 1 { 1 } else { 0 }
}

pub fn strategy_for(seat: PlayerId, rotation: u32) -> Strategy {
    if seat == search_seat(rotation) {
        Strategy::Search
    } else {
        Strategy::Heuristic
    }
}

pub struct SessionConfig {
    pub eval: EvalConfig,
    pub decks: Vec<DeckList>,
    /// sha256 of each declared fixture file, in `eval.decks` order.
    pub fixture_hashes: Vec<String>,
}

impl SessionConfig {
    /// Build session decks from a config that points at practice fixture files
    /// under `repo_root`.
    pub fn from_files(eval: EvalConfig, repo_root: &Path) -> anyhow::Result<Self> {
        eval.validate().map_err(anyhow::Error::msg)?;
        let mut decks = Vec::new();
        let mut hashes = Vec::new();
        for fixture in &eval.decks {
            let path = repo_root.join(&fixture.path);
            let bytes = fs::read(&path)
                .map_err(|e| anyhow::anyhow!("reading deck fixture {}: {e}", fixture.path))?;
            hashes.push(sha256_hex(&bytes));
            let title = String::from_utf8(bytes)
                .map_err(|e| anyhow::anyhow!("deck fixture {}: {e}", fixture.path))?;
            let deck = DeckList::parse_practice(&fixture.name, &title)
                .map_err(|e| anyhow::anyhow!("deck fixture {}: {e}", fixture.path))?;
            decks.push(deck);
        }
        Ok(SessionConfig {
            eval,
            decks,
            fixture_hashes: hashes,
        })
    }
}

pub fn run_session(session: &SessionConfig) -> anyhow::Result<Vec<GameRecord>> {
    let mut games = Vec::new();
    for &seed in &session.eval.seeds {
        for rotation in 0..session.eval.rotations {
            games.push(run_game(session, seed, rotation)?);
        }
    }
    Ok(games)
}

struct Stats {
    search_ms: u64,
    search_decisions: u64,
    sources: BTreeMap<String, usize>,
    fallback_notes: Vec<String>,
}

/// Honest audit label for one decision point.
fn source_label(owner: PlayerId, decision: &Decision, search_at: PlayerId) -> String {
    if owner == search_at {
        if decision.source == DecisionSource::Search {
            "search".to_string()
        } else {
            "heuristic (search-seat fallback)".to_string()
        }
    } else if decision.source == DecisionSource::Forced {
        "forced".to_string()
    } else {
        decision.source.label().to_string()
    }
}

pub fn run_game(session: &SessionConfig, seed: u64, rotation: u32) -> anyhow::Result<GameRecord> {
    let cfg = &session.eval;
    let started = Instant::now();
    let search_at = search_seat(rotation);
    let decks_ref: &[DeckList] = &session.decks;
    let mut game = Game::new(seed, decks_ref)
        .map_err(|e| anyhow::anyhow!("game {seed} rot{rotation}: {e}"))?;

    let mut log: Vec<Event> = Vec::new();
    let mut replay = Replay::new(
        seed,
        game.state.first_player,
        session.decks.clone(),
        aperture_core::AiSettings::default(),
    )
    .map_err(|e| anyhow::anyhow!("replay {seed} rot{rotation} setup: {e}"))?;

    let mut stats = Stats {
        search_ms: 0,
        search_decisions: 0,
        sources: BTreeMap::new(),
        fallback_notes: Vec::new(),
    };
    let search_knowledge = Knowledge {
        own_deck: Some(session.decks[search_at as usize].clone()),
    };
    let search_config = SearchConfig {
        seeds: cfg.search_seeds,
        node_budget: cfg.search_node_budget,
        max_rollout_nodes: cfg.search_max_rollout_nodes,
        max_millis: cfg.search_max_millis,
        max_candidates: cfg.search_max_candidates,
    };

    // Driver loop. Contract (mirrors the ai selfplay driver): a capped or
    // failed run is reported as such and never counted as a win/loss/draw.
    let mut steps = 0usize;
    let mut truncated: Option<String> = None;
    let mut failed = false;
    let mut repeats: BTreeMap<String, u8> = BTreeMap::new();
    let cancel = SearchCancellation::new();

    while truncated.is_none() && steps < cfg.max_steps {
        if game.state.is_over() {
            break;
        }
        if cfg.max_millis > 0 && started.elapsed().as_millis() as u64 >= cfg.max_millis {
            truncated = Some(format!("wall-clock cap of {} ms reached", cfg.max_millis));
            break;
        }
        let Some(owner) = game.decision_owner() else {
            truncated = Some("no decision point available although the game is not over".into());
            failed = true;
            break;
        };
        let view = game.view_for(owner, &log);
        let decision = if owner == search_at {
            let at = std::time::Instant::now();
            let point_ok = view.pending_kind.is_none();
            let make = if point_ok {
                search(&game, &view, &search_knowledge, &search_config, &cancel)
            } else {
                Err(aperture_ai::SearchUnavailable::PendingDecision {
                    kind: "eval driver: pending decision".to_string(),
                })
            };
            match make {
                Ok(outcome) => {
                    stats.search_ms += at.elapsed().as_millis() as u64;
                    stats.search_decisions += 1;
                    outcome.decision
                }
                Err(unavailable) => {
                    let note = unavailable.label();
                    if stats.fallback_notes.len() < 5 && !stats.fallback_notes.contains(&note) {
                        stats.fallback_notes.push(note);
                    }
                    fallback(&view)
                }
            }
        } else {
            fallback(&view)
        };
        *stats
            .sources
            .entry(source_label(owner, &decision, search_at))
            .or_insert(0) += 1;

        let revision = game.revision();
        let decision_id = game.decision();
        let submission = Submission {
            player: owner,
            revision,
            decision: decision_id,
            action: decision.action.clone(),
        };
        match game.submit(&submission) {
            Ok(events) => {
                log.extend(events);
                replay.record(ReplayStep {
                    player: owner,
                    revision,
                    decision: decision_id,
                    action: decision.action,
                });
                steps += 1;
            }
            Err(err) => {
                let note = format!("P{owner} policy action was rejected ({err}); passing instead");
                log.push(aperture_core::Event::Truncated {
                    reason: note.clone(),
                });
                if stats.fallback_notes.len() < 5 && !stats.fallback_notes.contains(&note) {
                    stats.fallback_notes.push(note);
                }
                let pass = Submission {
                    player: owner,
                    revision: game.revision(),
                    decision: game.decision(),
                    action: Action::Pass,
                };
                match game.submit(&pass) {
                    Ok(events) => {
                        log.extend(events);
                        replay.record(ReplayStep {
                            player: owner,
                            revision: pass.revision,
                            decision: pass.decision,
                            action: pass.action,
                        });
                        steps += 1;
                    }
                    Err(second) => {
                        truncated =
                            Some(format!("no legal action available for P{owner}: {second}"));
                        failed = true;
                        break;
                    }
                }
            }
        }
        let seen = repeats.entry(game.digest()).or_insert(0);
        *seen += 1;
        if *seen >= REPEAT_LIMIT {
            truncated = Some(format!(
                "state repetition detected at step {steps} (digest {}); stopping",
                &game.digest()[..12]
            ));
            break;
        }
    }

    if truncated.is_none() && !game.state.is_over() {
        truncated = Some(format!("step cap of {} reached", cfg.max_steps));
    }
    if let Some(reason) = truncated.clone() {
        log.push(aperture_core::Event::Truncated {
            reason: reason.clone(),
        });
        replay.truncated = Some(reason);
    }

    let (winner, reason) = match &game.state.outcome {
        Some(aperture_core::Outcome::Win { winner, reason }) => (Some(*winner), reason.clone()),
        Some(aperture_core::Outcome::Draw { reason }) => (None, reason.clone()),
        None => (None, String::new()),
    };
    let status = if failed {
        "failed".to_string()
    } else if game.state.is_over() {
        "terminal".to_string()
    } else {
        "capped".to_string()
    };

    replay.final_digest = game.digest();

    // Replay verification: round-trip through JSON, replay every recorded step
    // through the core engine, and require the core validator's exact
    // final-digest check.
    let replay_json = replay.to_json();
    let (replay_verified, replay_error) =
        match Replay::from_json(&replay_json).and_then(|r| r.run()) {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };

    Ok(GameRecord {
        seed,
        rotation,
        seat0: strategy_for(0, rotation),
        seat1: strategy_for(1, rotation),
        search_seat: Some(search_at),
        status,
        winner: winner.map(|p| p as u8),
        reason,
        steps,
        wall_ms: started.elapsed().as_millis() as u64,
        search_ms: stats.search_ms,
        search_decisions: stats.search_decisions,
        sources: stats.sources,
        fallback_notes: stats.fallback_notes,
        digest: game.digest(),
        replay_verified,
        replay_error,
        truncated,
        replay_json,
    })
}
