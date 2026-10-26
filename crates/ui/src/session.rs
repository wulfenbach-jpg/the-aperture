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
    DeciderConfig, DeciderPolicy, Decision, DecisionBinding, DecisionEnvelope, DecisionSource,
    HeuristicPolicy, Knowledge, MIN_OPTIONS, Policy, SearchCancellation, search,
};
use aperture_core::sandbox::{SandboxOp as EngineSandboxOp, SandboxRecord};
use aperture_core::view::{PendingKind, PlayerView};
use aperture_core::{
    Action, AiSettings, BlockDecl, CardId, CommanderSetup, DeckList, Event, Game, ObjectId, P0,
    PlayerId, Profile, Replay, ReplayStep, SessionSave, Submission,
};
use aperture_import::DeckReport;
use aperture_import::start::{DeckCertification, plan_start};

use crate::review::{MoveReview, NOT_YOUR_DECISION, REVIEW_BUDGET, ReviewBudget};

/// Everything a completed replay-branch job produces: the base recorded line,
/// the branched line cut at the chosen action boundary, and the state the
/// recorded prefix replays to. Applied only by the main thread.
pub struct BranchOutcome {
    pub parent: Replay,
    pub branched: Replay,
    pub game: Game,
    pub log: Vec<Event>,
}

/// Parse a study file into a recorded line. A session save contributes its
/// embedded replay (after a full digest validation); a bare replay file is
/// used directly. Anything else is a refusal — never guessed.
pub fn parse_replay_file(path: &Path) -> Result<Replay, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("the file is not valid JSON: {e}"))?;
    if value.get("game").is_some() {
        let save = SessionSave::from_json(&text).map_err(|e| e.to_string())?;
        Ok(save.replay)
    } else {
        aperture_core::Replay::from_json(&text).map_err(|e| e.to_string())
    }
}

/// A manual table control as the UI exposes it. Every variant is a sandbox
/// override: none of them is a legal-match action.
pub enum SandboxOp {
    Draw(u8),
    Mill(u8),
    /// Untap every permanent the human seat controls.
    Untap,
    /// Look at the top N and keep them on top (a scry choice, CR 701.22a).
    Scry(u8),
    /// Look at the top N and put them on the bottom in the same order.
    ScryBottom(u8),
    /// Look at the top N and keep them on top (a surveil choice, CR 701.25a).
    Surveil(u8),
    /// Look at the top N and put them into the graveyard.
    SurveilGraveyard(u8),
    /// Reveal the top N to every seat (public information).
    RevealTop(u8),
    /// Add (or remove, negative) +1/+1 counters on a permanent.
    Counter(ObjectId, i32),
    /// Set a permanent's +1/+1 counter count exactly.
    SetCounters(ObjectId, u32),
    /// Set a seat's life total (tabletop life counter).
    SetLife(PlayerId, i32),
}

/// One seat's certified import: the exact planned list plus the audit summary
/// the user must see before a match can start. Certifying one seat produces
/// three separate answers — Commander format legality, engine support and seat
/// fit — and only an actually startable seat carries a list.
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
    /// The commander this deck designates, when the engine can seat it.
    pub commander: Option<CardId>,
    /// Commander format legality, kept separate from engine support.
    pub format_status: String,
    /// Engine support verdict for the certified list.
    pub support_status: String,
    /// Every unsupported card with its exact reason (never truncated).
    pub unsupported: Vec<aperture_import::UnsupportedCard>,
    /// The profile this import was certified for.
    pub profile: String,
}

impl SeatImport {
    /// True only when the whole list passed the certification for its profile.
    pub fn certified(&self) -> bool {
        self.list.is_some()
    }

    /// One-line status for the import panel.
    pub fn status_line(&self) -> String {
        format!(
            "'{}' — format: {} · engine support: {} ({}/{} cards)",
            self.name, self.format_status, self.support_status, self.supported, self.total
        )
    }
}

/// A finished background job, delivered to `poll_workers`.
pub enum WorkerMessage {
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
    Branch {
        generation: u64,
        path: PathBuf,
        keep: usize,
        result: Result<Box<BranchOutcome>, String>,
    },
    Ai {
        generation: u64,
        revision: u64,
        decision_serial: u64,
        seat: PlayerId,
        decision: Decision,
    },
    /// A finished move review, bound to the decision it was computed for.
    MoveReview {
        generation: u64,
        revision: u64,
        decision_serial: u64,
        seat: PlayerId,
        report: Box<MoveReview>,
    },
}

/// File formats the UI shows as a label per seat.
fn seat_label(seat: PlayerId) -> String {
    format!("P{seat}")
}

/// Build a `SeatImport` from a start-flow certification. The list is `Some` only
/// when the engine support axis and the seat fit both pass, so an unsupported or
/// unseatable deck can never start; the two verdicts stay separately visible.
pub fn seat_import_from_certification(seat: PlayerId, cert: DeckCertification) -> SeatImport {
    let plan = plan_start(&cert).ok();
    let commander = cert.commander.as_ref().and_then(|c| c.card());
    let refusal = if cert.refusals.is_empty() {
        None
    } else {
        Some(cert.refusals.join(" | "))
    };
    SeatImport {
        seat,
        name: cert.deck_name.clone(),
        supported: cert.support.supported_cards,
        total: cert.support.total_cards
            + cert
                .commander
                .as_ref()
                .map(|c| c.quantity as u32)
                .unwrap_or(0),
        summary: cert.render(),
        list: plan.as_ref().map(|p| p.deck.clone()),
        refusal,
        commander,
        format_status: cert.format.status.as_str().to_string(),
        support_status: match cert.support.status {
            aperture_import::commander::SupportStatus::Supported => "supported".to_string(),
            aperture_import::commander::SupportStatus::Unsupported => "unsupported".to_string(),
        },
        unsupported: cert.support.unsupported.clone(),
        profile: cert.profile.clone(),
    }
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
        commander: None,
        format_status: "not_checked (legacy practice report)".to_string(),
        support_status: if report.launchable {
            "supported".to_string()
        } else {
            "unsupported".to_string()
        },
        unsupported: report.unsupported.clone(),
        profile: "practice-20".to_string(),
    }
}

/// Import one seat from a pasted Archidekt URL for `profile` (blocking; run
/// off-thread). The URL guard, redirect policy, timeout and size cap are the
/// existing provider rules; nothing is fetched on the frame thread.
pub fn import_seat_url_for(
    seat: PlayerId,
    url: &str,
    cache_dir: &Path,
    profile: Profile,
) -> Result<SeatImport, String> {
    let cert =
        aperture_import::start::certify_url(url, cache_dir, profile).map_err(|e| e.to_string())?;
    Ok(seat_import_from_certification(seat, cert))
}

/// Import one seat from a pasted text list for `profile` (blocking; run
/// off-thread).
pub fn import_seat_text_for(
    seat: PlayerId,
    name: &str,
    text: &str,
    profile: Profile,
) -> Result<SeatImport, String> {
    let cert =
        aperture_import::start::certify_text(name, text, profile).map_err(|e| e.to_string())?;
    Ok(seat_import_from_certification(seat, cert))
}

/// Import one seat from a deck file for `profile` (blocking; run off-thread).
pub fn import_seat_file_for(
    seat: PlayerId,
    path: &Path,
    profile: Profile,
) -> Result<SeatImport, String> {
    let cert = aperture_import::start::certify_file(path, profile).map_err(|e| e.to_string())?;
    Ok(seat_import_from_certification(seat, cert))
}

/// Import one seat from a pasted Archidekt URL (practice profile; blocking).
pub fn import_seat_url(seat: PlayerId, url: &str, cache_dir: &Path) -> Result<SeatImport, String> {
    import_seat_url_for(seat, url, cache_dir, Profile::Practice20)
}

/// Import one seat from a pasted text list (practice profile; blocking).
pub fn import_seat_text(seat: PlayerId, name: &str, text: &str) -> Result<SeatImport, String> {
    import_seat_text_for(seat, name, text, Profile::Practice20)
}

/// Import one seat from a text file path (practice profile; blocking).
pub fn import_seat_file(seat: PlayerId, path: &Path) -> Result<SeatImport, String> {
    import_seat_file_for(seat, path, Profile::Practice20)
}

pub struct UiSession {
    pub game: Game,
    pub log: Vec<Event>,
    pub replay: Replay,
    pub ai: AiSettings,
    pub seats: Vec<DeckList>,
    /// Explicit named game profile. New games and loads keep it; there is no
    /// implicit fallback to the two-seat 20-life table.
    pub profile: Profile,
    pub seed: u64,
    pub sandbox_modified: bool,
    /// Typed record of every manual table override used in this session.
    pub sandbox_record: SandboxRecord,
    pub human_plays_ai_seat: bool,
    pub heuristic: HeuristicPolicy,
    pub decider: Option<DeciderPolicy>,
    /// Kept so background AI work can build its own (thread-local) policy.
    pub decider_config: Option<DeciderConfig>,
    pub note: Option<String>,
    pub message: String,
    pub revealed: Vec<String>,
    pub selected_attackers: Vec<ObjectId>,
    pub selected_defender: Option<PlayerId>,
    pub selected_blocks: Vec<(ObjectId, ObjectId)>,
    pub selected_cards: Vec<ObjectId>,
    pub target_pick: Option<(ObjectId, Vec<Action>)>,
    pub inspect: Option<String>,
    pub decider_status: Option<String>,
    pub save_path: String,
    /// Branch/study point: a full snapshot plus the label shown to the user.
    pub branch: Option<(String, SessionSave)>,
    /// The recorded line a live branch was cut from, so accepted actions can
    /// be compared against it. Referenced only by its public recorded axis.
    pub branch_source: Option<Replay>,
    /// Branch-from-replay file path and chosen divergence action index (UI inputs).
    pub branch_path: String,
    pub branch_at: usize,
    /// True when the explicit postgame omniscient review mode is shown.
    pub review_mode: bool,
    /// Cached omniscient review text, computed once when review is entered.
    pub review_cache: Option<String>,
    /// Bumped whenever live work must be invalidated (new game, load, branch
    /// return, sandbox edit, import start, settings change).
    pub generation: u64,
    /// Per-seat import inputs (index == seat).
    pub seat_url: Vec<String>,
    pub seat_text: Vec<String>,
    pub seat_path: Vec<String>,
    pub seat_import: Vec<Option<SeatImport>>,
    pub seat_error: Vec<Option<String>>,
    pub import_busy: Vec<bool>,
    pub ai_busy: bool,
    pub health_busy: bool,
    /// A save or load worker is running.
    pub io_busy: bool,
    /// Set when an explicitly requested deck file could not be used; the session
    /// then refuses that deck instead of silently loading the bundled one.
    pub deck_refusal: Option<String>,
    /// Short note about the last discarded/aborted background result.
    pub busy_note: Option<String>,
    /// The last accepted move review and the decision it is bound to.
    pub review: Option<MoveReview>,
    /// A move-review worker is running.
    pub review_busy: bool,
    /// Cancellation handle for the running move review, so a table change stops
    /// the search instead of letting it finish work nobody will read.
    review_cancel: Option<SearchCancellation>,
    /// Exposed for deterministic delayed-queue tests: tests enqueue finished
    /// worker results directly instead of racing real threads.
    pub worker_tx: Sender<WorkerMessage>,
    worker_rx: Receiver<WorkerMessage>,
}

impl UiSession {
    pub fn new(
        seed: u64,
        seats: Vec<DeckList>,
        ai: AiSettings,
        use_decider: bool,
        human_plays_ai_seat: bool,
    ) -> Result<Self, String> {
        let seat_count = seats.len();
        if !(aperture_core::MIN_PLAYERS..=aperture_core::MAX_PLAYERS).contains(&seat_count) {
            return Err(format!(
                "{seat_count} seats supplied; the table supports 2..=4"
            ));
        }
        ai.validate_for(seat_count)?;
        let (game, log) = Game::new_logged(seed, &seats)
            .map_err(|err| format!("could not create the game: {err}"))?;
        let mut replay = Replay::new(seed, game.state.first_player, seats.clone(), ai.clone())
            .map_err(|err| format!("could not create the replay: {err}"))?;
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
        Ok(UiSession {
            game,
            log,
            replay,
            ai,
            seats,
            profile: Profile::Practice20,
            seed,
            sandbox_modified: false,
            sandbox_record: SandboxRecord::default(),
            human_plays_ai_seat,
            heuristic: HeuristicPolicy::new(),
            decider,
            decider_config,
            note,
            message: String::new(),
            revealed: Vec::new(),
            selected_attackers: Vec::new(),
            selected_defender: None,
            selected_blocks: Vec::new(),
            selected_cards: Vec::new(),
            target_pick: None,
            inspect: None,
            decider_status: None,
            save_path: "the-aperture-save.json".to_string(),
            branch: None,
            branch_source: None,
            branch_path: "the-aperture-save.json".to_string(),
            branch_at: 0,
            review_mode: false,
            review_cache: None,
            generation: 0,
            seat_url: vec![String::new(); seat_count],
            seat_text: vec![String::new(); seat_count],
            seat_path: vec![String::new(); seat_count],
            seat_import: vec![None; seat_count],
            seat_error: vec![None; seat_count],
            import_busy: vec![false; seat_count],
            ai_busy: false,
            health_busy: false,
            io_busy: false,
            deck_refusal: None,
            busy_note: None,
            review: None,
            review_busy: false,
            review_cancel: None,
            worker_tx,
            worker_rx,
        })
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
        self.import_busy = vec![false; self.seats.len()];
        if let Some(cancel) = self.review_cancel.take() {
            cancel.cancel();
        }
        self.review_busy = false;
        if self.review.take().is_some() {
            self.busy_note = Some(
                "discarded the previous move review [stale_generation] (the table changed)"
                    .to_string(),
            );
        }
    }

    /// Ask a background thread to compute the current AI decision. `work`
    /// receives the masked view snapshot and must not touch the engine.
    /// Returns false when it is not the AI's turn or one is already running.
    pub fn request_ai_decision<F>(&mut self, work: F) -> bool
    where
        F: FnOnce(PlayerView) -> Decision + Send + 'static,
    {
        self.request_ai_decision_bound(move |view, _binding| work(view))
    }

    /// Generation-aware worker request: the closure also receives the exact
    /// viewer/revision/decision/generation binding observed before the spawn, so
    /// a decider envelope can be validated against it.
    fn request_ai_decision_bound<F>(&mut self, work: F) -> bool
    where
        F: FnOnce(PlayerView, DecisionBinding) -> Decision + Send + 'static,
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
        let binding = DecisionBinding {
            viewer: seat,
            revision,
            decision: decision_serial,
            generation,
        };
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let decision = work(view, binding);
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
    /// baseline heuristic otherwise), built on the worker thread. The decider
    /// answer is applied only through an envelope bound to this snapshot.
    pub fn request_ai_decision_default(&mut self) -> bool {
        let config = if self.ai.use_decider {
            self.decider_config.clone()
        } else {
            None
        };
        self.request_ai_decision_bound(move |view, binding| match config {
            Some(config) => {
                // A single legal option is forced, not a model question. Check
                // before preparing an envelope: prepare would reject fewer than
                // two options and mislabel a forced move as a fallback.
                if view.legal.len() < MIN_OPTIONS {
                    return Decision {
                        action: HeuristicPolicy::new().decide(&view).action,
                        source: DecisionSource::Forced,
                        note: Some(format!(
                            "only {} legal option(s): the choice is forced and no model call was made",
                            view.legal.len()
                        )),
                        preference: None,
                    };
                }
                match DeciderPolicy::new(config) {
                    Ok(mut policy) => match DecisionEnvelope::prepare(&view, binding) {
                        Ok(envelope) => policy.decide_bound(&view, &envelope, binding),
                        Err(err) => Decision {
                            action: HeuristicPolicy::new().decide(&view).action,
                            source: DecisionSource::DeciderFallback,
                            note: Some(format!(
                                "no decision request sent [{}]: {err}",
                                err.reason_code()
                            )),
                            preference: None,
                        },
                    },
                    Err(err) => Decision {
                        action: HeuristicPolicy::new().decide(&view).action,
                        source: DecisionSource::DeciderFallback,
                        note: Some(format!(
                            "decider not usable [{}], used the heuristic: {err}",
                            err.reason_code()
                        )),
                        preference: None,
                    },
                }
            }
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
                WorkerMessage::MoveReview {
                    generation,
                    revision,
                    decision_serial,
                    seat,
                    report,
                } => {
                    if generation != self.generation {
                        // The seat's busy flag belongs to the discarded run:
                        // clear it, or a table change during a review would
                        // leave the panel spinning with no way to retry.
                        self.review_busy = false;
                        self.busy_note = Some(
                            "discarded a stale move review [stale_generation] (the table changed)"
                                .to_string(),
                        );
                        continue;
                    }
                    self.review_busy = false;
                    let stale_code = if self.game.revision() != revision {
                        Some("stale_revision")
                    } else if self.game.decision() != decision_serial {
                        Some("stale_decision")
                    } else if self.human_seat() != seat {
                        Some("wrong_seat")
                    } else {
                        None
                    };
                    if let Some(code) = stale_code {
                        self.busy_note = Some(format!(
                            "discarded a stale move review [{code}] (the decision point moved)"
                        ));
                        continue;
                    }
                    let summary = report.summary_line();
                    self.review = Some(*report);
                    applied.push(summary);
                }
                WorkerMessage::Ai {
                    generation,
                    revision,
                    decision_serial,
                    seat,
                    decision,
                } => {
                    if generation != self.generation {
                        self.busy_note = Some(
                            "discarded a stale AI decision [stale_generation] (the game changed)"
                                .to_string(),
                        );
                        continue;
                    }
                    self.ai_busy = false;
                    let stale_code = if self.game.revision() != revision {
                        Some("stale_revision")
                    } else if self.game.decision() != decision_serial {
                        Some("stale_decision")
                    } else if self.owner() != Some(seat) {
                        Some("wrong_seat")
                    } else {
                        None
                    };
                    if let Some(code) = stale_code {
                        self.busy_note = Some(format!(
                            "discarded a stale AI decision [{code}] (the decision point moved)"
                        ));
                        continue;
                    }
                    // The engine remains the final authority on legality; this
                    // pre-check only notices that the completion is not one the
                    // table is enumerating right now. An unenumerated action can
                    // still be legal (a cleanup discard accepts any legal set of
                    // cards, CR 514.1), so try it, and when the engine refuses it
                    // apply a plainly legal offered action. A dropped answer must
                    // never leave the table waiting forever.
                    let offered: Vec<Action> = self
                        .view(seat)
                        .legal
                        .iter()
                        .map(|legal| legal.action.clone())
                        .collect();
                    if offered.contains(&decision.action) {
                        applied.push(self.apply_ai_decision(decision));
                        continue;
                    }
                    self.note = decision.note.clone();
                    let source = decision.source.label().to_string();
                    if self.submit(decision.action).is_ok() {
                        // Engine-validated, so it is legal at application time even
                        // though the offer under-describes the choice space.
                        applied.push(source);
                        continue;
                    }
                    let refusal = self.message.clone();
                    match self.view(seat).legal.first().cloned() {
                        Some(legal) => {
                            let label = legal.label.clone();
                            match self.submit(legal.action) {
                                Ok(()) => {
                                    // The refused attempt left a "refused: …" line in
                                    // `message`; the table has moved on, so clear it.
                                    self.message.clear();
                                    self.busy_note = Some(format!(
                                        "rejected an AI decision [illegal_choice] ({refusal}); applied the engine's legal offer \"{label}\" instead"
                                    ));
                                    applied.push(format!(
                                        "applied \"{label}\" after refusing an illegal completion"
                                    ));
                                }
                                Err(second) => {
                                    // Genuinely impossible: the engine refused both
                                    // its own offer and the completion.
                                    self.busy_note = Some(format!(
                                        "discarded an AI decision [illegal_choice] ({refusal}); the offered action \"{label}\" was also refused ({second})"
                                    ));
                                }
                            }
                        }
                        None => {
                            self.busy_note = Some(format!(
                                "discarded an AI decision [illegal_choice] ({refusal}); the table offers no action at this decision"
                            ));
                        }
                    }
                }
                WorkerMessage::Import {
                    generation,
                    seat,
                    outcome,
                } => {
                    let index = seat as usize;
                    if generation != self.generation {
                        // The seat's busy flag belongs to the discarded run: clear
                        // it, otherwise a session change during a fetch would
                        // leave the panel spinning forever with no way to retry.
                        if let Some(busy) = self.import_busy.get_mut(index) {
                            *busy = false;
                        }
                        self.busy_note =
                            Some(format!("discarded a stale {} import", seat_label(seat)));
                        continue;
                    }
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
                WorkerMessage::Branch {
                    generation,
                    path,
                    keep,
                    result,
                } => {
                    self.io_busy = false;
                    if generation != self.generation {
                        self.busy_note =
                            Some("discarded a stale branch (the game changed)".to_string());
                        continue;
                    }
                    match result {
                        Ok(outcome) => {
                            self.apply_replay_branch(*outcome, keep);
                            let line = format!(
                                "branched from {} at recorded action #{keep} — the recorded line is replayed to digest {}, new actions from here are branch history",
                                path.display(),
                                &self.game.digest()[..12]
                            );
                            self.message = line.clone();
                            applied.push(line);
                        }
                        Err(err) => {
                            self.message = format!("branch failed: {err}");
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
        self.ai_busy
            || self.health_busy
            || self.io_busy
            || self.review_busy
            || self.import_busy.iter().any(|b| *b)
    }

    /// Ask a background worker to review the reviewing seat's current decision
    /// point with the default review budget. Returns false when a review is
    /// already running. Never blocks the caller and never mutates the engine.
    pub fn request_move_review(&mut self) -> bool {
        self.request_move_review_with(REVIEW_BUDGET)
    }

    /// The same request with an explicit budget (tests use a smaller one; the
    /// evidence journey records the real one).
    pub fn request_move_review_with(&mut self, budget: ReviewBudget) -> bool {
        if self.review_busy {
            return false;
        }
        let seat = self.human_seat();
        // The masked view of the seat being reviewed is the only observation
        // the analysis may use; the engine stays the authority on the main
        // thread and receives nothing from the worker.
        let view = self.view(seat);
        let generation = self.generation;
        let revision = self.game.revision();
        let decision_serial = self.game.decision();
        if view.legal.is_empty() {
            // No engine offer for this viewer: there is nothing to rank, and
            // starting a search that must fail would only hide the reason.
            self.review = Some(MoveReview::refused(
                seat,
                generation,
                revision,
                decision_serial,
                budget,
                NOT_YOUR_DECISION.to_string(),
            ));
            self.review_busy = false;
            return true;
        }
        // The worker gets a copy of the engine world and the masked view. The
        // search reconstructs its own world from the view; the copy is only the
        // engine authority it validates against.
        let world = self.game.clone();
        let knowledge = Knowledge {
            own_deck: self.seats.get(seat as usize).cloned(),
        };
        let config = budget.config();
        let cancel = SearchCancellation::new();
        self.review_cancel = Some(cancel.clone());
        let tx = self.worker_tx.clone();
        thread::spawn(move || {
            let report = match search(&world, &view, &knowledge, &config, &cancel) {
                Ok(outcome) => MoveReview::from_outcome(
                    seat,
                    generation,
                    revision,
                    decision_serial,
                    budget,
                    &outcome,
                    &view.legal,
                ),
                Err(refusal) => MoveReview::refused(
                    seat,
                    generation,
                    revision,
                    decision_serial,
                    budget,
                    refusal.label(),
                ),
            };
            let _ = tx.send(WorkerMessage::MoveReview {
                generation,
                revision,
                decision_serial,
                seat,
                report: Box::new(report),
            });
        });
        self.review_busy = true;
        true
    }

    /// Why the stored review no longer describes the live table, if it does not.
    pub fn review_fresh(&self) -> Option<&'static str> {
        let review = self.review.as_ref()?;
        if review.generation != self.generation {
            Some("stale_generation")
        } else if review.revision != self.game.revision() {
            Some("stale_revision")
        } else if review.decision_serial != self.game.decision() {
            Some("stale_decision")
        } else if review.seat != self.human_seat() {
            Some("wrong_seat")
        } else {
            None
        }
    }

    // ------------------------------------------------------------- per-seat import

    /// The exact planned lists, when every seat is certified.
    pub fn imported_lists(&self) -> Option<Vec<DeckList>> {
        let mut lists = Vec::with_capacity(self.seats.len());
        for seat in 0..self.seats.len() {
            let import = self.seat_import.get(seat)?.as_ref()?;
            lists.push(import.list.clone()?);
        }
        Some(lists)
    }

    /// "Start imported match" is enabled only when every seat is certified.
    pub fn can_start_imported(&self) -> bool {
        self.imported_lists().is_some()
    }

    /// Start a fresh match from the certified imports. The exact planned lists
    /// are applied with no substitutions, and the new game/replay embed them as
    /// snapshots.
    pub fn start_imported_match(&mut self, seed: u64) -> Result<Vec<String>, String> {
        let lists = self.imported_lists().ok_or_else(|| {
            "every seat needs a certified deck before an imported match can start".to_string()
        })?;
        let names: Vec<String> = lists.iter().map(|deck| deck.name.clone()).collect();
        self.seats = lists;
        self.deck_refusal = None;
        self.new_game(seed);
        self.message = format!(
            "started an imported match: {} — the exact planned lists were applied, no substitutions",
            names
                .iter()
                .enumerate()
                .map(|(seat, name)| format!("P{seat} '{name}'"))
                .collect::<Vec<_>>()
                .join(" vs ")
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
        self.sandbox_record = save.sandbox.clone();
        self.selected_attackers.clear();
        self.selected_defender = None;
        self.selected_blocks.clear();
        self.selected_cards.clear();
        self.target_pick = None;
        self.note = None;
        self.message = format!(
            "returned to the branch point at {label}; the board, log and replay are back to that moment"
        );
        self.review_mode = false;
        self.review_cache = None;
        Ok(())
    }

    pub fn view(&self, seat: PlayerId) -> PlayerView {
        self.game.view_for(seat, &self.log)
    }

    pub fn owner(&self) -> Option<PlayerId> {
        self.game.decision_owner()
    }

    pub fn human_seat(&self) -> PlayerId {
        self.ai.human_seat
    }

    /// True when a policy should act for `seat` now.
    pub fn seat_is_ai(&self, seat: PlayerId) -> bool {
        self.ai.seat_is_ai(seat) || (seat == self.human_seat() && self.human_plays_ai_seat)
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
        if self.review_mode {
            self.message =
                "postgame review is a read-only view; leave review mode to act on the table"
                    .to_string();
            return Err(self.message.clone());
        }
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
                // An accepted action changes the game, so any in-flight load
                // (or AI/import/health result) captured before it is stale.
                self.invalidate_pending_work();
                self.log.extend(events);
                self.replay.record(ReplayStep {
                    player: owner,
                    revision,
                    decision,
                    action,
                });
                self.selected_attackers.clear();
                self.selected_defender = None;
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

    /// Bundled practice lists for a table of `seat_count` seats, alternating
    /// the two compiled 40-card decks so every seat starts with a complete,
    /// supported list. Ordinary practice always uses these lists.
    pub fn bundled_seats(seat_count: usize) -> Result<Vec<DeckList>, String> {
        Self::bundled_seats_for_profile(seat_count, Profile::Practice20)
    }

    /// Default libraries for the selected named profile. The command profiles
    /// get the compiled command-zone practice list (white sources plus the
    /// supported command-zone cards) for every seat; the supplied Isamaru is
    /// separate and is never part of the library. Ordinary practice keeps the
    /// alternating Temur/Izzet bundles.
    pub fn bundled_seats_for_profile(
        seat_count: usize,
        profile: Profile,
    ) -> Result<Vec<DeckList>, String> {
        if !(aperture_core::MIN_PLAYERS..=aperture_core::MAX_PLAYERS).contains(&seat_count) {
            return Err(format!(
                "{seat_count} seats requested; the native table supports 2..=4"
            ));
        }
        Ok((0..seat_count)
            .map(|seat| {
                let (name, text) = if profile.has_command_zone() {
                    ("Command zone practice", BUNDLED_COMMAND)
                } else if seat % 2 == 0 {
                    ("Temur Beats", BUNDLED_A)
                } else {
                    ("Izzet Control", BUNDLED_B)
                };
                let mut deck = bundled_practice(name, text);
                if seat > 1 {
                    deck.name = format!("{} P{seat}", deck.name);
                }
                deck
            })
            .collect())
    }

    /// The explicit one-supported-commander-per-seat setup for the named
    /// command profiles. Empty for ordinary practice. A seat whose import
    /// certified its own commander uses that commander; every other seat falls
    /// back to the bundled one. Nothing here can invent a commander: the
    /// supported-pool check already ran during certification.
    pub fn commander_setup(&self) -> Vec<CommanderSetup> {
        if !self.profile.has_command_zone() {
            return Vec::new();
        }
        (0..self.seats.len())
            .map(|seat| CommanderSetup {
                seat: seat as PlayerId,
                card: self
                    .imported_commander(seat as PlayerId)
                    .unwrap_or(CardId::IsamaruHoundOfKonda),
            })
            .collect()
    }

    /// The commander a certified command-zone import designated for `seat`.
    pub fn imported_commander(&self, seat: PlayerId) -> Option<CardId> {
        let import = self.seat_import.get(seat as usize)?.as_ref()?;
        if !import.certified() {
            return None;
        }
        import.commander
    }

    /// Build a brand-new engine, log and replay for the configured seats and
    /// profile. Both the engine and the replay use the same explicit profile
    /// constructors, so a 3/4-seat or 40-life table can never silently fall
    /// back to the two-seat 20-life game.
    fn build_fresh(&self, seed: u64) -> Result<(Game, Vec<Event>, Replay), String> {
        let specs: Vec<(PlayerId, CardId)> = self
            .commander_setup()
            .into_iter()
            .map(|setup| (setup.seat, setup.card))
            .collect();
        let (game, log) = match self.profile {
            Profile::Practice20 => Game::new_logged(seed, &self.seats),
            Profile::CommandZone20 => Game::new_command_zone_logged(seed, &self.seats, &specs),
            Profile::CommanderCombat40 => {
                Game::new_commander_combat_logged(seed, &self.seats, &specs)
            }
        }
        .map_err(|err| format!("could not create the game: {err}"))?;
        let commanders = self.commander_setup();
        let mut replay = match self.profile {
            Profile::Practice20 => Replay::new(
                seed,
                game.state.first_player,
                self.seats.clone(),
                self.ai.clone(),
            ),
            Profile::CommandZone20 => Replay::new_command_zone(
                seed,
                game.state.first_player,
                self.seats.clone(),
                commanders,
                self.ai.clone(),
            ),
            Profile::CommanderCombat40 => Replay::new_commander_combat(
                seed,
                game.state.first_player,
                self.seats.clone(),
                commanders,
                self.ai.clone(),
            ),
        }
        .map_err(|err| format!("could not create the replay: {err}"))?;
        replay.steps.clear();
        Ok((game, log, replay))
    }

    /// Start a new table at `seat_count` seats and the selected named profile.
    /// The human is P0; every other seat is driven by the heuristic AI. The
    /// selection is retained by later `new_game` calls.
    pub fn start_table(
        &mut self,
        seed: u64,
        seat_count: usize,
        profile: Profile,
    ) -> Result<(), String> {
        let seats = Self::bundled_seats_for_profile(seat_count, profile)?;
        self.seats = seats;
        self.profile = profile;
        self.ai.human_seat = P0;
        self.ai.ai_seats = (1..seat_count as PlayerId).collect();
        self.ai.enabled = true;
        self.human_plays_ai_seat = false;
        self.seat_url = vec![String::new(); seat_count];
        self.seat_text = vec![String::new(); seat_count];
        self.seat_path = vec![String::new(); seat_count];
        self.seat_import = vec![None; seat_count];
        self.seat_error = vec![None; seat_count];
        self.import_busy = vec![false; seat_count];
        self.new_game(seed);
        Ok(())
    }

    /// Resize every per-seat UI vector after a seat-count change or a load.
    fn resize_seat_ui(&mut self) {
        let count = self.seats.len();
        self.seat_url.resize(count, String::new());
        self.seat_text.resize(count, String::new());
        self.seat_path.resize(count, String::new());
        self.seat_import.resize(count, None);
        self.seat_error.resize(count, None);
        self.import_busy.resize(count, false);
    }

    pub fn new_game(&mut self, seed: u64) {
        self.invalidate_pending_work();
        self.branch = None;
        self.branch_source = None;
        self.review_mode = false;
        self.review_cache = None;
        self.seed = seed;
        self.resize_seat_ui();
        let (game, log, replay) = self
            .build_fresh(seed)
            .unwrap_or_else(|err| panic!("validated session seats failed to start: {err}"));
        self.game = game;
        self.log = log;
        self.replay = replay;
        self.sandbox_modified = false;
        self.sandbox_record = SandboxRecord::default();
        self.revealed.clear();
        self.message.clear();
        self.note = None;
        self.selected_attackers.clear();
        self.selected_defender = None;
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
        // The boolean flag and the typed record are both carried: a save that
        // was loaded from an older file can be tainted without a record, and
        // re-saving it must never clear that taint.
        let mut save = SessionSave::capture(
            &self.game,
            self.replay.clone(),
            self.log.clone(),
            self.tainted(),
        );
        save.sandbox = self.sandbox_record.clone();
        save
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
                let save = SessionSave::from_json(&text).map_err(|e| e.to_string())?;
                Ok(Box::new(save))
            })();
            let _ = tx.send(WorkerMessage::Load {
                generation,
                path,
                result,
            });
        });
    }

    /// Branch from a recorded line: parse the study file (session save or bare
    /// replay) on a background worker, cut the line at the chosen action
    /// boundary, and replay that recorded prefix under the same seed. The
    /// result is applied back on the main thread only when its generation
    /// still matches, exactly like a load.
    pub fn request_replay_branch(&mut self, path: PathBuf, keep: usize) {
        let generation = self.generation;
        let tx = self.worker_tx.clone();
        self.io_busy = true;
        thread::spawn(move || {
            let result = (|| -> Result<Box<BranchOutcome>, String> {
                let parent = parse_replay_file(&path)?;
                const MAX_BRANCH_STEPS: usize = 1_000_000;
                if parent.steps.len() > MAX_BRANCH_STEPS {
                    return Err(format!(
                        "the recorded line has {} action(s) (limit {MAX_BRANCH_STEPS})",
                        parent.steps.len()
                    ));
                }
                let branched = parent.branch_at(keep).map_err(|e| e.to_string())?;
                let (game, log) = branched.run_recorded().map_err(|e| e.to_string())?;
                Ok(Box::new(BranchOutcome {
                    parent,
                    branched,
                    game,
                    log,
                }))
            })();
            let _ = tx.send(WorkerMessage::Branch {
                generation,
                path,
                keep,
                result,
            });
        });
    }

    /// Apply a completed replay branch (main thread only). The live session
    /// becomes the branch: the recorded prefix state, its log, and the branch
    /// line kept open for later accepted actions. The base line is remembered
    /// for the public accepted-action comparison.
    fn apply_replay_branch(&mut self, outcome: BranchOutcome, keep: usize) {
        self.invalidate_pending_work();
        self.game = outcome.game;
        self.log = outcome.log;
        self.replay = outcome.branched;
        self.branch_source = Some(outcome.parent);
        self.branch = None;
        self.review_mode = false;
        self.review_cache = None;
        self.sandbox_modified = false;
        self.sandbox_record = SandboxRecord::default();
        self.selected_attackers.clear();
        self.selected_defender = None;
        self.selected_blocks.clear();
        self.selected_cards.clear();
        self.target_pick = None;
        self.note = None;
        self.revealed.clear();
        self.branch_at = keep;
        self.resize_seat_ui();
        self.ai = self.replay.ai.clone();
    }

    /// The public comparison of accepted actions between the line this branch
    /// was cut from and the live branch, as far as both are recorded. Only
    /// action kinds, seats and decision serials are shown — never card names,
    /// targets or zone contents.
    pub fn branch_comparison(&self) -> Option<aperture_core::BranchDiff> {
        aperture_core::compare_branches(self.branch_source.as_ref()?, &self.replay).ok()
    }

    /// Left alone only when both branch lines agree over the branch length so far.
    pub fn branches_agree(&self) -> bool {
        self.branch_comparison()
            .is_some_and(|diff| diff.diverged_at.is_none())
    }

    /// Apply a parsed snapshot to the live session (main thread only).
    fn apply_save(&mut self, save: &SessionSave) {
        self.invalidate_pending_work();
        self.game = save.game();
        self.log = save.log.clone();
        self.replay = save.replay.clone();
        self.ai = save.ai.clone();
        self.seats = save.seats.clone();
        self.profile = save.game.profile;
        self.resize_seat_ui();
        self.sandbox_modified = save.sandbox_modified;
        self.sandbox_record = save.sandbox.clone();
        self.review_mode = false;
        self.review_cache = None;
        self.branch_source = None;
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

    /// Apply one manual table control.
    ///
    /// These are **sandbox overrides**, not legal-match actions: each one goes
    /// through the engine's deterministic sandbox path, is recorded in the
    /// session's typed taint record (which survives save/load) and marks the
    /// whole session as sandbox-modified. A refusal changes nothing and reports
    /// the engine's reason.
    pub fn sandbox(&mut self, op: SandboxOp) -> Result<(), String> {
        if self.review_mode {
            self.message =
                "postgame review is read-only: sandbox tools are off while review is shown"
                    .to_string();
            return Err(self.message.clone());
        }
        let seat = self.human_seat();
        let engine_op = match op {
            SandboxOp::Draw(count) => EngineSandboxOp::Draw { seat, count },
            SandboxOp::Mill(count) => EngineSandboxOp::Mill { seat, count },
            SandboxOp::Untap => EngineSandboxOp::Untap { seat },
            SandboxOp::Scry(count) => EngineSandboxOp::Scry {
                seat,
                count,
                to_top: true,
            },
            SandboxOp::ScryBottom(count) => EngineSandboxOp::Scry {
                seat,
                count,
                to_top: false,
            },
            SandboxOp::Surveil(count) => EngineSandboxOp::Surveil {
                seat,
                count,
                to_top: true,
            },
            SandboxOp::SurveilGraveyard(count) => EngineSandboxOp::Surveil {
                seat,
                count,
                to_top: false,
            },
            SandboxOp::RevealTop(count) => EngineSandboxOp::Reveal { seat, count },
            SandboxOp::Counter(object, delta) => EngineSandboxOp::AddCounters { object, delta },
            SandboxOp::SetCounters(object, count) => EngineSandboxOp::SetCounters { object, count },
            SandboxOp::SetLife(target, life) => EngineSandboxOp::SetLife { seat: target, life },
        };
        match self.game.sandbox(engine_op) {
            Ok(outcome) => {
                self.invalidate_pending_work();
                self.sandbox_modified = true;
                let revealed_names = match outcome.events.last() {
                    Some(Event::SandboxOverride {
                        kind: aperture_core::SandboxKind::Reveal,
                        count,
                        ..
                    }) => self
                        .game
                        .state
                        .player(seat)
                        .library
                        .iter()
                        .take(*count as usize)
                        .map(|id| self.game.state.object(*id).card.name().to_string())
                        .collect(),
                    _ => Vec::new(),
                };
                self.log.extend(outcome.events);
                self.sandbox_record.push(outcome.edit.clone());
                if !revealed_names.is_empty() {
                    self.revealed = revealed_names;
                }
                self.message = format!(
                    "SANDBOX override applied — {}. This session is tainted: no certified replay and no legal match result from here on.",
                    outcome.edit.detail
                );
                Ok(())
            }
            Err(err) => {
                self.message = format!("sandbox refused: {err}");
                Err(err.to_string())
            }
        }
    }

    /// True when this session's competitive record is tainted by an override.
    pub fn tainted(&self) -> bool {
        self.sandbox_modified || self.sandbox_record.is_tainted()
    }

    /// The one sentence any result surface must use.
    pub fn record_line(&self) -> String {
        self.sandbox_record
            .record_line(self.game.state.outcome.as_ref(), self.tainted())
    }

    // ------------------------------------------------------------------- undo

    /// Undo is offered only while the recorded line still describes this exact
    /// position: after a sandbox override the line no longer produces the state,
    /// so undoing would silently substitute a different game.
    pub fn can_undo(&self) -> bool {
        !self.review_mode && !self.tainted() && !self.replay.steps.is_empty()
    }

    /// Undo the most recent accepted action.
    ///
    /// The position is rebuilt by replaying the deterministic recorded prefix
    /// from the same seed, so the random state, revision and decision serial are
    /// restored *exactly* rather than patched. The log is rebuilt from that
    /// prefix too, so nothing the undone action revealed (a draw, a scry look, a
    /// private choice) is still readable anywhere in the session.
    pub fn undo_last_action(&mut self) -> Result<String, String> {
        if self.review_mode {
            return Err("postgame review is read-only: leave review before undoing".to_string());
        }
        if self.tainted() {
            let message = "undo is unavailable after a sandbox override: the recorded line no longer produces this position. Start a new game or use Return to branch.".to_string();
            self.message = message.clone();
            return Err(message);
        }
        let Some(last) = self.replay.steps.last().cloned() else {
            let message = "there is no accepted action to undo".to_string();
            self.message = message.clone();
            return Err(message);
        };
        let keep = self.replay.steps.len() - 1;
        let branched = self
            .replay
            .branch_at(keep)
            .map_err(|err| format!("could not cut the recorded line: {err}"))?;
        let (game, log) = branched
            .run_recorded()
            .map_err(|err| format!("could not replay the recorded prefix: {err}"))?;
        self.invalidate_pending_work();
        self.game = game;
        self.log = log;
        self.replay = branched;
        self.review_mode = false;
        self.review_cache = None;
        self.selected_attackers.clear();
        self.selected_defender = None;
        self.selected_blocks.clear();
        self.selected_cards.clear();
        self.target_pick = None;
        self.note = None;
        let label = format!("P{}'s {}", last.player, last.action.kind());
        self.message = format!(
            "undid {label}. The position was rebuilt from the recorded prefix, so the random state and decision serial are exact; anything that action revealed is gone from the log."
        );
        Ok(label)
    }

    /// Pending-choice helper text for the UI.
    pub fn pending_kind(&self, view: &PlayerView) -> Option<PendingKind> {
        view.pending_kind
    }

    /// Enter the explicit postgame omniscient review mode. Only possible once
    /// the game has actually ended; while the review is up the session refuses
    /// every action submission, so the omniscient pane can never be a live
    /// policy input. The omniscient text is computed once and cached.
    pub fn enter_review(&mut self) -> Result<(), String> {
        if self.game.state.outcome.is_none() {
            return Err(
                "postgame review is only available once the game has actually ended".to_string(),
            );
        }
        self.review_mode = true;
        self.review_cache =
            Some(aperture_core::view::render_review(&self.game).expect("review is gated"));
        self.message = "REVIEW: omniscient postgame view. Acting is disabled; choose 'Leave review' to go back.".to_string();
        Ok(())
    }

    /// Leave review mode. The session stays exactly as it was; review never
    /// changes state and never feeds any policy or decision input.
    pub fn leave_review(&mut self) {
        self.review_mode = false;
        self.review_cache = None;
        self.message.clear();
    }

    pub fn conclude(&self) -> Option<String> {
        self.game
            .state
            .outcome
            .as_ref()
            .map(|outcome| match outcome {
                aperture_core::Outcome::Win { winner, reason } => {
                    if self.tainted() {
                        format!(
                            "P{winner} wins — {reason} — SANDBOX-MODIFIED, not a legal match result ({})",
                            self.sandbox_record.summary()
                        )
                    } else {
                        format!("P{winner} wins — {reason}")
                    }
                }
                aperture_core::Outcome::Draw { reason } => {
                    if self.tainted() {
                        format!(
                            "the game is a draw — {reason} — SANDBOX-MODIFIED, not a legal match result ({})",
                            self.sandbox_record.summary()
                        )
                    } else {
                        format!("the game is a draw — {reason}")
                    }
                }
            })
    }
}

/// Level of the practice profile shown in the UI header.
pub fn profile_line() -> String {
    format!(
        "practice slice · supported pool: {} cards · 2-4 live seats (native table: 2-4) · 20-life ordinary or bounded 40-life one-Isamaru-per-seat profile · London mulligan · NOT Commander",
        aperture_core::catalog::pool_size()
    )
}

pub const BUNDLED_A: &str = include_str!("../../../decks/temur-beats.txt");
pub const BUNDLED_B: &str = include_str!("../../../decks/izzet-control.txt");
pub const BUNDLED_COMMAND: &str = include_str!("../../../decks/command-zone-practice.txt");

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
    "SANDBOX OVERRIDES — manual table controls; each one taints this session's record (never a legal match action)"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_session(seed: u64) -> UiSession {
        let a = DeckList::parse_practice("Temur Beats", BUNDLED_A).expect("deck A");
        let b = DeckList::parse_practice("Izzet Control", BUNDLED_B).expect("deck B");
        UiSession::new(seed, vec![a, b], AiSettings::default(), false, false)
            .expect("two-seat session")
    }

    /// A review budget small enough to finish inside a test but large enough to
    /// score the opening offers.
    fn test_budget() -> ReviewBudget {
        ReviewBudget {
            seeds: 1,
            node_budget: 400,
            max_rollout_nodes: 120,
            max_millis: 1_200,
            max_candidates: 4,
        }
    }

    /// Poll the worker channel until the move review has been applied, exactly
    /// as the frame loop does.
    fn wait_for_review(ui: &mut UiSession) -> Vec<String> {
        let mut applied = Vec::new();
        for _ in 0..4_000 {
            applied.extend(ui.poll_workers());
            if !ui.review_busy {
                return applied;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("the move review worker never finished");
    }

    /// Keep every opening hand so the table reaches a real priority decision.
    fn finish_opening(ui: &mut UiSession) {
        let mut guard = 0;
        while ui.game.state.pending.is_some() {
            ui.submit(Action::Mulligan { keep: true })
                .unwrap_or_else(|err| panic!("opening keep refused: {err}"));
            guard += 1;
            assert!(guard < 40, "the opening procedure did not finish");
        }
    }

    #[test]
    fn a_move_review_refusal_is_typed_and_ranks_nothing() {
        let mut ui = test_session(11);
        assert!(
            ui.game.state.pending.is_some(),
            "the fixture must start at the opening (mulligan) decision"
        );
        assert!(
            ui.request_move_review(),
            "the first review request must be accepted"
        );
        assert!(
            ui.review_busy,
            "the request must hand the work to a worker instead of computing it inline"
        );
        wait_for_review(&mut ui);
        let review = ui
            .review
            .as_ref()
            .expect("a refusal panel is still a panel");
        let reason = review
            .refusal
            .clone()
            .expect("search must refuse the mulligan decision with a typed reason");
        assert!(
            reason.contains("non-priority decision") && reason.contains("mulligan"),
            "the panel must show the search's own typed reason: {reason}"
        );
        assert!(
            review.candidates.is_empty() && review.ranked().is_empty(),
            "a refused review must rank nothing: {review:?}"
        );
        let shown = review.policy_section().lines.join("\n");
        assert!(
            shown.contains(&reason),
            "the typed reason must reach the panel verbatim: {shown}"
        );
        assert!(
            !shown.contains("mean ") && !shown.contains("1. "),
            "a refusal must not print a scored ordering: {shown}"
        );
    }

    #[test]
    fn a_move_review_ranks_only_engine_offered_actions() {
        let mut ui = test_session(12);
        finish_opening(&mut ui);
        let seat = ui.human_seat();
        // Play the AI's decisions until priority reaches the human seat: the
        // review is of the reviewing seat's own decision point.
        let mut guard = 0;
        while ui.owner() != Some(seat) {
            ui.step_ai();
            guard += 1;
            assert!(
                guard < 400,
                "the fixture never reached the human's priority"
            );
        }
        assert_eq!(
            ui.owner(),
            Some(seat),
            "the fixture must reach the human seat's priority"
        );
        let offers: Vec<String> = ui
            .view(seat)
            .legal
            .iter()
            .map(|legal| legal.label.clone())
            .collect();
        assert!(
            !offers.is_empty(),
            "the priority decision must offer at least one action"
        );
        assert!(
            ui.request_move_review_with(test_budget()),
            "the review request must be accepted at a priority decision"
        );
        assert!(
            ui.review_busy && ui.worker_busy(),
            "the review must run on the background worker"
        );
        wait_for_review(&mut ui);
        let review = ui.review.as_ref().expect("a review");
        assert!(
            review.refusal.is_none(),
            "a priority decision must not be refused: {:?}",
            review.refusal
        );
        assert!(
            !review.candidates.is_empty(),
            "the review must score the engine's offers: {review:?}"
        );
        for candidate in &review.candidates {
            assert!(
                offers.contains(&candidate.label),
                "the review invented a candidate the engine never offered: {candidate:?} not in {offers:?}"
            );
        }
        assert!(
            review.decisions_used > 0,
            "the panel must report the engine decisions the analysis used"
        );
        assert!(
            review
                .fidelity
                .contains("no real hand or library order read"),
            "the search's own fidelity disclosure must reach the panel: {}",
            review.fidelity
        );
        assert!(
            review.budget == test_budget(),
            "the panel must report the budget that was actually used"
        );
    }

    #[test]
    fn a_move_review_never_submits_or_changes_the_table() {
        let mut ui = test_session(15);
        finish_opening(&mut ui);
        let digest = ui.game.digest();
        let revision = ui.game.revision();
        let decision = ui.game.decision();
        let log_len = ui.log.len();
        let replay_len = ui.replay.steps.len();
        let undo_before = ui.can_undo();
        assert!(ui.request_move_review_with(test_budget()));
        wait_for_review(&mut ui);
        assert!(ui.review.is_some(), "the review must be applied");
        assert_eq!(ui.game.digest(), digest, "the review changed the table");
        assert_eq!(
            ui.game.revision(),
            revision,
            "the review moved the revision"
        );
        assert_eq!(
            ui.game.decision(),
            decision,
            "the review moved the decision"
        );
        assert_eq!(ui.log.len(), log_len, "the review wrote to the log");
        assert_eq!(
            ui.replay.steps.len(),
            replay_len,
            "the review recorded an action"
        );
        assert!(
            !ui.review_mode,
            "a move review must not switch the session into omniscient postgame review"
        );
        assert_eq!(
            ui.can_undo(),
            undo_before,
            "a move review must not change whether undo is offered"
        );
    }

    #[test]
    fn a_stale_move_review_does_not_replace_the_current_panel() {
        let mut ui = test_session(14);
        let stale_generation = ui.generation;
        let stale = MoveReview::refused(
            ui.human_seat(),
            stale_generation,
            ui.game.revision(),
            ui.game.decision(),
            REVIEW_BUDGET,
            "an older decision point".to_string(),
        );
        // The table moves on before the worker answers.
        ui.submit(Action::Mulligan { keep: true })
            .expect("the opening keep must be accepted");
        ui.worker_tx
            .send(WorkerMessage::MoveReview {
                generation: stale.generation,
                revision: stale.revision,
                decision_serial: stale.decision_serial,
                seat: stale.seat,
                report: Box::new(stale),
            })
            .expect("queue the stale review");
        let current = MoveReview::refused(
            ui.human_seat(),
            ui.generation,
            ui.game.revision(),
            ui.game.decision(),
            REVIEW_BUDGET,
            "the current decision point".to_string(),
        );
        ui.review = Some(current.clone());

        let applied = ui.poll_workers();
        assert!(
            ui.busy_note
                .as_deref()
                .unwrap_or("")
                .contains("[stale_generation]"),
            "a stale review must be discarded with a stale code, got {:?}",
            ui.busy_note
        );
        assert_eq!(
            ui.review,
            Some(current),
            "a stale review replaced the current panel"
        );
        assert!(
            applied
                .iter()
                .all(|line| !line.contains("move review ready")),
            "a stale review must not report success: {applied:?}"
        );
    }

    #[test]
    fn a_move_review_for_another_seat_is_discarded() {
        let mut ui = test_session(16);
        let generation = ui.generation;
        let report = MoveReview::refused(
            0,
            generation,
            ui.game.revision(),
            ui.game.decision(),
            REVIEW_BUDGET,
            "reviewed for the wrong seat".to_string(),
        );
        // The reviewing seat changes while the worker runs.
        ui.ai.human_seat = 1;
        ui.ai.ai_seats = vec![0];
        ui.worker_tx
            .send(WorkerMessage::MoveReview {
                generation,
                revision: report.revision,
                decision_serial: report.decision_serial,
                seat: 0,
                report: Box::new(report),
            })
            .expect("queue the wrong-seat review");
        ui.poll_workers();
        assert!(
            ui.busy_note
                .as_deref()
                .unwrap_or("")
                .contains("[wrong_seat]"),
            "a review for another seat must be discarded, got {:?}",
            ui.busy_note
        );
        assert!(
            ui.review.is_none(),
            "a review for another seat must not become the panel"
        );
    }

    #[test]
    fn a_table_change_drops_the_review_and_stops_the_worker() {
        let mut ui = test_session(17);
        finish_opening(&mut ui);
        ui.review = Some(MoveReview::refused(
            ui.human_seat(),
            ui.generation,
            ui.game.revision(),
            ui.game.decision(),
            REVIEW_BUDGET,
            "the previous decision".to_string(),
        ));
        assert!(ui.request_move_review_with(REVIEW_BUDGET));
        ui.new_game(21);
        assert!(
            ui.review.is_none(),
            "a table change must drop the review of the old decision"
        );
        assert!(!ui.review_busy, "the review worker flag must be cleared");
        assert!(
            ui.busy_note
                .as_deref()
                .unwrap_or("")
                .contains("[stale_generation]"),
            "dropping the review must be reported, got {:?}",
            ui.busy_note
        );
    }

    /// Deterministic delayed-load regression: the load message is enqueued with
    /// the generation observed before an accepted action, then released after
    /// that action. The action must invalidate it, so the current game, log,
    /// replay and decision survive and the user sees a stale-load reason.
    #[test]
    fn an_accepted_action_invalidates_an_in_flight_load() {
        let mut ui = test_session(71);
        let other = test_session(72);
        let delayed = SessionSave::capture(
            &other.game,
            other.replay.clone(),
            other.log.clone(),
            other.sandbox_modified,
        );

        let stale_generation = ui.generation;
        assert!(
            ui.submit(Action::Mulligan { keep: true }).is_ok(),
            "the opening keep must be accepted"
        );
        let digest_after = ui.game.digest();
        let decision_after = ui.game.decision();
        let log_after = ui.log.len();
        let replay_after = ui.replay.steps.len();

        ui.worker_tx
            .send(WorkerMessage::Load {
                generation: stale_generation,
                path: PathBuf::from("delayed-load.json"),
                result: Ok(Box::new(delayed)),
            })
            .expect("queue the delayed load");
        let applied = ui.poll_workers();

        assert!(
            ui.busy_note.as_deref().unwrap_or("").contains("stale load"),
            "expected a stale-load reason, got {:?}",
            ui.busy_note
        );
        assert_eq!(ui.game.digest(), digest_after, "state was overwritten");
        assert_eq!(ui.game.decision(), decision_after, "decision moved");
        assert_eq!(ui.log.len(), log_after, "log was replaced");
        assert_eq!(ui.replay.steps.len(), replay_after, "replay was replaced");
        assert!(
            applied.iter().all(|line| !line.starts_with("loaded ")),
            "a stale load must not report success: {applied:?}"
        );
    }
}
