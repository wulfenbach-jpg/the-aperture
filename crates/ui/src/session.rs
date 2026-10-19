//! UI session state: the engine, the event log, the replay, AI settings, the
//! sandbox toolset, save/load and per-seat deck import. All strict actions go
//! through the engine's transactional `submit`; sandbox tools are clearly
//! separated and permanently mark the session as sandbox-modified.
//!
//! Every model, network and file operation the UI needs runs on a background
//! thread and is applied back on the main thread only after its generation,
//! engine revision, decision serial and seat still match (`poll_workers`). The
//! engine itself stays authoritative on the main thread: a worker returns a
//! proposal, never a mutation.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use aperture_ai::{
    DeciderConfig, DeciderPolicy, Decision, DecisionSource, HeuristicPolicy, Policy,
};
use aperture_core::view::{PendingKind, PlayerView};
use aperture_core::{
    Action, AiSettings, BlockDecl, DeckList, Event, Game, ObjectId, P0, P1, PlayerId, Replay,
    ReplayStep, SessionSave, Submission, Zone,
};
use aperture_import::DeckReport;

pub enum SandboxOp {
    Draw(u8),
    Mill(u8),
    RevealTop(u8),
    Counter(ObjectId, i32),
}

/// One seat's certified import: the exact planned list plus the audit summary
/// the user must see before a match can start.
#[derive(Debug, Clone)]
pub struct SeatImport {
    pub seat: PlayerId,
    pub name: String,
    pub supported: u32,
    pub total: u32,
    /// Human-readable support/profile/refusal summary.
    pub summary: String,
    /// The exact planned list; `None` when the import was refused.
    pub list: Option<DeckList>,
    pub refusal: Option<String>,
}

impl SeatImport {
    /// True only when the whole list passed the practice-profile analysis.
    pub fn certified(&self) -> bool {
        self.list.is_some()
    }
}

/// A finished background job, delivered to `poll_workers`.
enum WorkerMessage {
    Import {
        generation: u64,
        seat: PlayerId,
        outcome: Result<Box<SeatImport>, String>,
    },
    Health {
        generation: u64,
        status: String,
    },
    Save {
        result: String,
    },
    Load {
        generation: u64,
        path: PathBuf,
        result: Result<Box<SessionSave>, String>,
    },
    Ai {
        generation: u64,
        revision: u64,
        decision_serial: u64,
        seat: PlayerId,
        decision: Decision,
    },
}

/// File formats the UI shows as a label per seat.
fn seat_label(seat: PlayerId) -> &'static str {
    if seat == P0 { "P0" } else { "P1" }
}

/// Build a `SeatImport` from a provider report: the list is `Some` only when
/// the practice profile certifies it, so an unsupported deck can never start.
pub fn seat_import_from_report(seat: PlayerId, report: DeckReport) -> SeatImport {
    let list = aperture_import::plan_deck(&report).ok();
    let mut summary = format!(
        "{} '{}'\nsource: {}\ncards: {} ({} supported, {} unsupported, {} excluded, {} sideboard, {} commander)\nprofile: {}",
        report.provider,
        report.deck_name,
        report.source,
        report.total_cards,
        report.supported_cards,
        report.unsupported_cards,
        report.excluded.len(),
        report.sideboard.len(),
        report.commander.len(),
        report.legality.join("; ")
    );
    if let Some(sha) = &report.raw_sha256 {
        summary.push_str(&format!("\nresponse sha256: {}", &sha[..16.min(sha.len())]));
    }
    if let Some(path) = &report.cached_path {
        summary.push_str(&format!("\ncached at {}", path.display()));
    }
    if !report.notes.is_empty() {
        summary.push_str(&format!("\nnotes: {}", report.notes.join(" | ")));
    }
    summary.push_str(&format!(
        "\nrefusal: {}",
        report.refusal.clone().unwrap_or_else(|| "none".to_string())
    ));
    // A refused plan still surfaces the cards that blocked it, so the reason is
    // never just "unsupported".
    for unsupported in report.unsupported.iter().take(4) {
        summary.push_str(&format!(
            "\nunsupported: {} x{} — {}",
            unsupported.name, unsupported.quantity, unsupported.reason
        ));
    }
    SeatImport {
        seat,
        name: report.deck_name.clone(),
        supported: report.supported_cards,
        total: report.total_cards,
        summary,
        list,
        refusal: report.refusal.clone(),
    }
}

/// Import one seat from a pasted Archidekt URL (blocking; run off-thread).
pub fn import_seat_url(seat: PlayerId, url: &str, cache_dir: &Path) -> Result<SeatImport, String> {
    let report = aperture_import::import_url(url, cache_dir).map_err(|e| e.to_string())?;
    Ok(seat_import_from_report(seat, report))
}

/// Import one seat from a pasted text list (blocking; run off-thread).
pub fn import_seat_text(seat: PlayerId, name: &str, text: &str) -> Result<SeatImport, String> {
    let report = aperture_import::text_report(name, text).map_err(|e| e.to_string())?;
    Ok(seat_import_from_report(seat, report))
}

/// Import one seat from a text file path (blocking; run off-thread).
pub fn import_seat_file(seat: PlayerId, path: &Path) -> Result<SeatImport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported list");
    import_seat_text(seat, name, &text)
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
    /// Kept so background AI work can build its own (thread-local) policy.
    pub decider_config: Option<DeciderConfig>,
    pub note: Option<String>,
    pub message: String,
    pub revealed: Vec<String>,
    pub selected_attackers: Vec<ObjectId>,
    pub selected_blocks: Vec<(ObjectId, ObjectId)>,
    pub selected_cards: Vec<ObjectId>,
    pub target_pick: Option<(ObjectId, Vec<Action>)>,
    pub inspect: Option<String>,
    pub decider_status: Option<String>,
    pub save_path: String,
    /// Branch/study point: a full snapshot plus the label shown to the user.
    pub branch: Option<(String, SessionSave)>,
    /// Bumped whenever live work must be invalidated (new game, load, branch
    /// return, sandbox edit, import start, settings change).
    pub generation: u64,
    /// Per-seat import inputs (human seat first is not assumed: index == seat).
    pub seat_url: [String; 2],
    pub seat_text: [String; 2],
    pub seat_path: [String; 2],
    pub seat_import: [Option<SeatImport>; 2],
    pub seat_error: [Option<String>; 2],
    pub import_busy: [bool; 2],
    pub ai_busy: bool,
    pub health_busy: bool,
    /// A save or load worker is running.
    pub io_busy: bool,
    /// Set when an explicitly requested deck file could not be used; the session
    /// then refuses that deck instead of silently loading the bundled one.
    pub deck_refusal: Option<String>,
    /// Short note about the last discarded/aborted background result.
    pub busy_note: Option<String>,
    worker_tx: Sender<WorkerMessage>,
    worker_rx: Receiver<WorkerMessage>,
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
        let config = DeciderConfig {
            timeout_ms: ai.decider_timeout_ms,
            ..DeciderConfig::default()
        };
        let (decider, note) = if use_decider {
            match DeciderPolicy::new(config.clone()) {
                Ok(policy) => (Some(policy), None),
                Err(err) => (None, Some(format!("decider not usable: {err}"))),
            }
        } else {
            (None, None)
        };
        let decider_config = if use_decider { Some(config) } else { None };
        let (worker_tx, worker_rx) = channel();
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
            decider_config,
            note,
            message: String::new(),
            revealed: Vec::new(),
            selected_attackers: Vec::new(),
            selected_blocks: Vec::new(),
            selected_cards: Vec::new(),
            target_pick: None,
            inspect: None,
            decider_status: None,
            save_path: "the-aperture-save.json".to_string(),
            branch: None,
            generation: 0,
            seat_url: [String::new(), String::new()],
            seat_text: [String::new(), String::new()],
            seat_path: [String::new(), String::new()],
            seat_import: [None, None],
            seat_error: [None, None],
            import_busy: [false, false],
            ai_busy: false,
            health_busy: false,
            io_busy: false,
            deck_refusal: None,
            busy_note: None,
            worker_tx,
            worker_rx,
        }
    }

    // ---------------------------------------------------------------- workers

    /// Invalidate every in-flight background result. Called on any change that
    /// makes a pending answer meaningless, including a new game with the same
    /// seed (the generation, not the digest, is what guards staleness).
    pub fn invalidate_pending_work(&mut self) {
        self.generation += 1;
        self.ai_busy = false;
        self.health_busy = false;
        self.io_busy = false;
        self.import_busy = [false, false];
    }

    /// Ask a background thread to compute the current AI decision. `work`
    /// receives the masked view snapshot and must not touch the engine.
    /// Returns false when it is not the AI's turn or one is already running.
    pub fn request_ai_decision<F>(&mut self, work: F) -> bool
    where
        F: FnOnce(PlayerView) -> Decision + Send + 'static,
    {
        if self.ai_busy {
            return false;
        }
        let Some(seat) = self.owner() else {
            return false;
        };
        if !self.seat_is_ai(seat) {
            return false;
        }
        let view = self.view(seat);
        let revision = self.game.revision();
        let decision_serial = self.game.decision();
        let generation = self.generation;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let decision = work(view);
            let _ = tx.send(WorkerMessage::Ai {
                generation,
                revision,
                decision_serial,
                seat,
                decision,
            });
        });
        self.ai_busy = true;
        true
    }

    /// Convenience wrapper: the configured policy (decider when opted in,
    /// baseline heuristic otherwise), built on the worker thread.
    pub fn request_ai_decision_default(&mut self) -> bool {
        let config = if self.ai.use_decider {
            self.decider_config.clone()
        } else {
            None
        };
        self.request_ai_decision(move |view| match config {
            Some(config) => match DeciderPolicy::new(config) {
                Ok(mut policy) => policy.decide(&view),
                Err(err) => Decision {
                    action: HeuristicPolicy::new().decide(&view).action,
                    source: DecisionSource::DeciderFallback,
                    note: Some(format!("decider not usable, used the heuristic: {err}")),
                    preference: None,
                },
            },
            None => HeuristicPolicy::new().decide(&view),
        })
    }

    /// Run one import for `seat` on a background thread. `work` returns the
    /// certified import or a refusal message.
    pub fn request_import<F>(&mut self, seat: PlayerId, work: F) -> bool
    where
        F: FnOnce() -> Result<SeatImport, String> + Send + 'static,
    {
        let index = seat as usize;
        if self.import_busy[index] {
            return false;
        }
        self.import_busy[index] = true;
        self.seat_error[index] = None;
        let generation = self.generation;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let outcome = work().map(Box::new);
            let _ = tx.send(WorkerMessage::Import {
                generation,
                seat,
                outcome,
            });
        });
        true
    }

    /// Run the model health check on a background thread.
    pub fn request_health<F>(&mut self, work: F) -> bool
    where
        F: FnOnce() -> String + Send + 'static,
    {
        if self.health_busy {
            return false;
        }
        self.health_busy = true;
        let generation = self.generation;
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let status = work();
            let _ = tx.send(WorkerMessage::Health { generation, status });
        });
        true
    }

    /// Drain finished background work without blocking. Applies only results
    /// whose generation, revision, decision serial and seat still match.
    /// Returns one human-readable line per applied result.
    pub fn poll_workers(&mut self) -> Vec<String> {
        let mut applied = Vec::new();
        while let Ok(message) = self.worker_rx.try_recv() {
            match message {
                WorkerMessage::Ai {
                    generation,
                    revision,
                    decision_serial,
                    seat,
                    decision,
                } => {
                    if generation != self.generation {
                        self.busy_note =
                            Some("discarded a stale AI decision (the game changed)".to_string());
                        continue;
                    }
                    self.ai_busy = false;
                    if self.game.revision() != revision
                        || self.game.decision() != decision_serial
                        || self.owner() != Some(seat)
                    {
                        self.busy_note = Some(
                            "discarded a stale AI decision (the decision point moved)".to_string(),
                        );
                        continue;
                    }
                    applied.push(self.apply_ai_decision(decision));
                }
                WorkerMessage::Import {
                    generation,
                    seat,
                    outcome,
                } => {
                    if generation != self.generation {
                        self.busy_note =
                            Some(format!("discarded a stale {} import", seat_label(seat)));
                        continue;
                    }
                    let index = seat as usize;
                    self.import_busy[index] = false;
                    match outcome {
                        Ok(mut import) => {
                            import.seat = seat;
                            self.seat_import[index] = Some(*import);
                            self.seat_error[index] = None;
                            applied.push(format!("{} import ready", seat_label(seat)));
                        }
                        Err(err) => {
                            self.seat_error[index] = Some(err.clone());
                            applied.push(format!("{} import failed: {err}", seat_label(seat)));
                        }
                    }
                }
                WorkerMessage::Health { generation, status } => {
                    if generation != self.generation {
                        continue;
                    }
                    self.health_busy = false;
                    self.decider_status = Some(status);
                }
                WorkerMessage::Save { result } => {
                    self.io_busy = false;
                    self.message = result.clone();
                    applied.push(result);
                }
                WorkerMessage::Load {
                    generation,
                    path,
                    result,
                } => {
                    self.io_busy = false;
                    if generation != self.generation {
                        self.busy_note =
                            Some("discarded a stale load (the game changed)".to_string());
                        continue;
                    }
                    match result {
                        Ok(save) => {
                            self.apply_save(&save);
                            let line = format!(
                                "loaded {} — digest {} (stored {}), {} action(s) in the replay",
                                path.display(),
                                &self.game.digest()[..12],
                                &save.digest[..12],
                                save.replay.steps.len()
                            );
                            self.message = line.clone();
                            applied.push(line);
                        }
                        Err(err) => {
                            self.message = format!("load failed: {err}");
                            applied.push(self.message.clone());
                        }
                    }
                }
            }
        }
        applied
    }

    /// True when a background worker is running.
    pub fn worker_busy(&self) -> bool {
        self.ai_busy || self.health_busy || self.io_busy || self.import_busy.iter().any(|b| *b)
    }

    // ------------------------------------------------------------- per-seat import

    /// The two exact planned lists, when both seats are certified.
    pub fn imported_lists(&self) -> Option<(DeckList, DeckList)> {
        let a = self.seat_import[0].as_ref()?.list.clone()?;
        let b = self.seat_import[1].as_ref()?.list.clone()?;
        Some((a, b))
    }

    /// "Start imported match" is enabled only when BOTH seats are certified.
    pub fn can_start_imported(&self) -> bool {
        self.imported_lists().is_some()
    }

    /// Start a fresh match from the two certified imports. The exact planned
    /// lists are applied with no substitutions, and the new game/replay embed
    /// them as snapshots.
    pub fn start_imported_match(&mut self, seed: u64) -> Result<(String, String), String> {
        let (a, b) = self.imported_lists().ok_or_else(|| {
            "both seats need a certified deck before an imported match can start".to_string()
        })?;
        let names = (a.name.clone(), b.name.clone());
        self.deck_a = a;
        self.deck_b = b;
        self.deck_refusal = None;
        self.new_game(seed);
        self.message = format!(
            "started an imported match: '{}' (P0) vs '{}' (P1) — the exact planned lists were applied, no substitutions",
            names.0, names.1
        );
        Ok(names)
    }

    /// Mark the current position as a study branch point.
    pub fn set_branch(&mut self) {
        self.replay.ai = self.ai.clone();
        let digest = self.game.digest();
        let label = format!(
            "turn {} {} · digest {} · {} logged event(s)",
            self.game.state.turn.number,
            self.game.state.turn.step.name(),
            &digest[..12],
            self.log.len()
        );
        let save = SessionSave::capture(
            &self.game,
            self.replay.clone(),
            self.log.clone(),
            self.sandbox_modified,
        );
        self.branch = Some((label.clone(), save));
        self.message = format!(
            "branch point set at {label}. Hidden-information boundary: returning here restores only what was known at that moment; anything revealed later (draws, scries, the opponent's plays after it) is out of scope again."
        );
    }

    /// Return to the branch point, discarding everything played after it.
    pub fn return_to_branch(&mut self) -> Result<(), String> {
        let Some((label, save)) = self.branch.clone() else {
            return Err("no branch point has been set".to_string());
        };
        self.invalidate_pending_work();
        self.game = save.game();
        self.log = save.log.clone();
        self.replay = save.replay.clone();
        self.ai = save.ai.clone();
        self.sandbox_modified = save.sandbox_modified;
        self.selected_attackers.clear();
        self.selected_blocks.clear();
        self.selected_cards.clear();
        self.target_pick = None;
        self.note = None;
        self.message = format!(
            "returned to the branch point at {label}; the board, log and replay are back to that moment"
        );
        Ok(())
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

    /// Opt in to or out of the local model. Rebuilds the sync policy, points the
    /// background worker at the new setting, and invalidates any in-flight work.
    pub fn set_use_decider(&mut self, on: bool) {
        self.ai.use_decider = on;
        let config = DeciderConfig {
            timeout_ms: self.ai.decider_timeout_ms,
            ..DeciderConfig::default()
        };
        self.decider_config = if on { Some(config.clone()) } else { None };
        self.decider = if on {
            DeciderPolicy::new(config).ok()
        } else {
            None
        };
        self.invalidate_pending_work();
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

    /// Submit an action on behalf of the human seat. Refuses when the decision
    /// belongs to the AI seat, so a UI control can never act as the opponent
    /// just because `owner()` happens to return that seat.
    pub fn submit_human(&mut self, action: Action) -> Result<(), String> {
        let Some(owner) = self.owner() else {
            return Err("no player holds priority right now".to_string());
        };
        if owner != self.human_seat() {
            let message = format!(
                "refused: the current decision belongs to P{owner} (not your seat P{}); the AI acts for that seat",
                self.human_seat()
            );
            self.message = message.clone();
            return Err(message);
        }
        self.submit(action)
    }

    fn apply_ai_decision(&mut self, decision: Decision) -> String {
        self.note = decision.note.clone();
        let source = decision.source.label().to_string();
        if let Err(err) = self.submit(decision.action) {
            self.note = Some(format!("policy decision refused by the engine: {err}"));
            let _ = self.submit(Action::Pass);
        }
        source
    }

    /// One AI decision for the seat that must act, computed synchronously.
    /// Used by tests and non-UI drivers; the UI uses the background workers.
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
        Some(self.apply_ai_decision(decision))
    }

    pub fn new_game(&mut self, seed: u64) {
        self.invalidate_pending_work();
        self.branch = None;
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
        self.selected_attackers.clear();
        self.selected_blocks.clear();
        self.selected_cards.clear();
        self.target_pick = None;
    }

    /// Snapshot the session exactly as `save` would, without touching the disk.
    /// The serialization happens on the main thread (microseconds); the file
    /// write happens on a worker.
    fn session_snapshot(&mut self) -> SessionSave {
        // The save must describe the settings that are live right now, not the
        // ones the replay was created with.
        self.replay.ai = self.ai.clone();
        self.replay.outcome = self.game.state.outcome.clone();
        self.replay.final_digest = self.game.digest();
        SessionSave::capture(
            &self.game,
            self.replay.clone(),
            self.log.clone(),
            self.sandbox_modified,
        )
    }

    /// Write the session to `path` on a background worker. The frame callback
    /// never touches the filesystem.
    pub fn request_save(&mut self, path: PathBuf) {
        let save = self.session_snapshot();
        let sandbox = self.sandbox_modified;
        let tx = self.worker_tx.clone();
        self.io_busy = true;
        thread::spawn(move || {
            let result = match std::fs::write(&path, save.to_json()) {
                Ok(()) => format!(
                    "saved to {} (digest {}){}",
                    path.display(),
                    &save.digest[..12],
                    if sandbox {
                        " — SANDBOX-MODIFIED: certified replay is void"
                    } else {
                        ""
                    }
                ),
                Err(err) => format!("save failed: {err}"),
            };
            let _ = tx.send(WorkerMessage::Save { result });
        });
    }

    /// Read and parse a save file on a background worker; the parsed snapshot is
    /// applied back on the main thread by `poll_workers`.
    pub fn request_load(&mut self, path: PathBuf) {
        let generation = self.generation;
        let tx = self.worker_tx.clone();
        self.io_busy = true;
        thread::spawn(move || {
            let result = (|| -> Result<Box<SessionSave>, String> {
                let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                SessionSave::from_json(&text)
                    .map_err(|e| e.to_string())
                    .map(Box::new)
            })();
            let _ = tx.send(WorkerMessage::Load {
                generation,
                path,
                result,
            });
        });
    }

    /// Apply a parsed snapshot to the live session (main thread only).
    fn apply_save(&mut self, save: &SessionSave) {
        self.invalidate_pending_work();
        self.game = save.game();
        self.log = save.log.clone();
        self.replay = save.replay.clone();
        self.ai = save.ai.clone();
        self.deck_a = save.deck_a.clone();
        self.deck_b = save.deck_b.clone();
        self.sandbox_modified = save.sandbox_modified;
    }

    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        let save = self.session_snapshot();
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
        self.apply_save(&save);
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
            self.invalidate_pending_work();
            self.sandbox_modified = true;
            self.log.push(Event::StateBasedAction { description });
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

pub const BUNDLED_A: &str = include_str!("../../../decks/temur-beats.txt");
pub const BUNDLED_B: &str = include_str!("../../../decks/izzet-control.txt");

/// Built-in practice deck from the compiled bundle. Used only when no custom
/// deck path was requested; an explicitly requested path is never substituted.
pub fn bundled_practice(name: &str, bundle: &str) -> DeckList {
    DeckList::parse_practice(name, bundle).unwrap_or_else(|_| DeckList::new(name, Vec::new()))
}

/// Load an explicitly requested deck file. On any failure the caller gets an
/// empty list plus a refusal message: the bundled deck is never silently used
/// in place of the user's file.
pub fn load_explicit_deck(path: &Path, name: &str) -> (DeckList, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(text) => match DeckList::parse_practice(name, &text) {
            Ok(list) => (list, None),
            Err(err) => (
                DeckList::new(name, Vec::new()),
                Some(format!(
                    "deck file {} is invalid: {err}. Refusing to substitute the bundled practice deck.",
                    path.display()
                )),
            ),
        },
        Err(err) => (
            DeckList::new(name, Vec::new()),
            Some(format!(
                "deck file {} could not be read: {err}. Refusing to substitute the bundled practice deck.",
                path.display()
            )),
        ),
    }
}

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
