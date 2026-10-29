//! The card table: header, real 2x2 seat panels for 2-4 seats, stack, own hand,
//! legal actions, coaching, log, pending-choice controls, sandbox, per-seat deck
//! import and save/load. Every model/network/file operation runs on a background
//! worker; the frame callback never blocks.
//!
//! The renderer reads player-authored card data only from the engine's masked
//! `PlayerView`. Opponents contribute counts and public zones (battlefield,
//! graveyard, exile, command zone), never hand or library contents.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use aperture_ai::{DeciderClient, DeciderConfig};
use aperture_core::view::{CardView, PlayerView, ZONE_ORDER};
use aperture_core::{Action, AiSettings, ObjectId, P0, PlayerId, Profile, Zone, catalog, coach};
use eframe::egui::{self, RichText};

use crate::perf::{Conditions, PerformanceProbe};
use crate::session::{
    SandboxOp, UiSession, block_list, import_seat_file_for, import_seat_text_for,
    import_seat_url_for, load_explicit_deck,
};
use crate::theme;

pub struct Startup {
    pub smoke: Option<PathBuf>,
    pub seed: u64,
    pub deck_a: PathBuf,
    pub deck_b: PathBuf,
    /// True when the user explicitly asked for `deck_a`/`deck_b`; an explicit
    /// path that fails is refused instead of silently replaced.
    pub deck_a_explicit: bool,
    pub deck_b_explicit: bool,
    pub ai_seat: u8,
    pub human_autoplay: bool,
    pub autoplay: bool,
    pub use_decider: bool,
    /// Initial native table size (2..=4).
    pub seats: u8,
    /// Initial explicit named profile.
    pub profile: Profile,
    /// Optional initial window size for smoke captures and viewport checks.
    pub window: Option<(f32, f32)>,
    /// Optional prefilled study file for branching a recorded line.
    pub study_file: Option<String>,
    /// Optional prefilled divergence action index for `study_file`.
    pub branch_at: usize,
    /// Opt-in native frame instrumentation report path.
    pub performance_report: Option<PathBuf>,
    /// Conditions to disclose in that report (`--performance-note`): they are
    /// reported as declared conditions. This binary injects no synthetic
    /// delay, so the report's `synthetic_delays` list stays empty.
    pub performance_conditions: Vec<String>,
    /// Initial accessibility settings for a capture/launch (`--a11y
    /// high-contrast,reduced-motion`). Only seeds the two settings the
    /// Accessibility panel exposes; the panel still owns them afterwards.
    pub initial_high_contrast: bool,
    pub initial_reduced_motion: bool,
}

/// A destructive action that must be confirmed before it runs. Everything in
/// this list discards or replaces live table state, so it never fires on the
/// first click and it never silently substitutes a deck or a game.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingAction {
    NewGame {
        seed: u64,
    },
    NewTable {
        seed: u64,
        seats: usize,
        profile: Profile,
    },
    Load {
        path: PathBuf,
    },
    Branch {
        path: PathBuf,
        keep: usize,
    },
    ResetSandboxGame {
        seed: u64,
    },
    StartImported {
        seed: u64,
    },
}

impl PendingAction {
    fn question(&self) -> String {
        match self {
            PendingAction::NewGame { seed } => format!(
                "start a new game with seed {seed}? The live position, its log and any unsaved changes are discarded."
            ),
            PendingAction::NewTable {
                seed,
                seats,
                profile,
            } => format!(
                "start a new {seats}-seat table with profile {} ({} life) and seed {seed}? The live position is discarded.",
                profile.label(),
                profile.starting_life()
            ),
            PendingAction::Load { path } => format!(
                "load {}? The live position, its log and unsaved changes are discarded.",
                path.display()
            ),
            PendingAction::Branch { path, keep } => format!(
                "branch from {} at recorded action #{keep}? The live position is replaced by the replayed recorded prefix.",
                path.display()
            ),
            PendingAction::ResetSandboxGame { seed } => format!(
                "reset this sandbox game with seed {seed}? The live position is discarded and the sandbox taint is cleared."
            ),
            PendingAction::StartImported { seed } => format!(
                "start the imported match with seed {seed}? The live position is discarded and the certified lists replace it."
            ),
        }
    }

    fn confirm_label(&self) -> &'static str {
        match self {
            PendingAction::NewGame { .. } => "Confirm new game",
            PendingAction::NewTable { .. } => "Confirm new table",
            PendingAction::Load { .. } => "Confirm load",
            PendingAction::Branch { .. } => "Confirm branch",
            PendingAction::ResetSandboxGame { .. } => "Confirm reset",
            PendingAction::StartImported { .. } => "Confirm imported match",
        }
    }
}

/// Whether a status message is a failure the player must not mistake for a
/// hint. Every refusal and failed destructive action is painted in the error
/// colour; success messages and study notes are not.
fn message_is_error(message: &str) -> bool {
    message.starts_with("refused")
        || message.starts_with("sandbox refused")
        || message.starts_with("load failed")
        || message.starts_with("branch failed")
        || message.starts_with("undo is unavailable")
        || message.starts_with("cancel")
        || message.starts_with("there is no accepted action")
}

pub struct ApertureApp {
    session: UiSession,
    startup: Startup,
    smoke_started: Instant,
    smoke_requested: bool,
    smoke_written: Arc<AtomicBool>,
    last_ai_step: Instant,
    show_sandbox: bool,
    show_import: bool,
    show_zones: bool,
    /// Move-review panel: ranked engine-offered alternatives for the reviewing
    /// seat's current decision, computed on the background worker.
    show_review: bool,
    show_coach: bool,
    show_study: bool,
    show_help: bool,
    show_accessibility: bool,
    high_contrast: bool,
    reduced_motion: bool,
    /// Last (high contrast, reduced motion) pair whose style was installed.
    observed_theme: (bool, bool),
    /// Opt-in native frame probe; `None` unless --performance-report was used.
    perf: Option<PerformanceProbe>,
    /// Table setup controls; kept in sync with the live table.
    setup_seats: u8,
    setup_profile: Profile,
    /// Last live settings the controls were synced from. Comparing these
    /// (instead of overwriting the controls every frame) preserves a user's
    /// pending selection until they start the new table.
    observed_seats: u8,
    observed_profile: Profile,
    /// Zone-inspector inputs: one shared, searchable inspector over every zone
    /// the human seat may legally see.
    zone_query: String,
    zone_seat_filter: Option<PlayerId>,
    zone_zone_filter: Option<Zone>,
    /// Destructive action waiting for an explicit confirmation.
    confirm: Option<PendingAction>,
    /// Counter-editor target and life-editor inputs (sandbox overrides).
    counter_object: Option<ObjectId>,
    life_seat: PlayerId,
    life_value: i32,
    zoom: f32,
    frame_count: u64,
}

fn seat_label(seat: PlayerId) -> String {
    format!("P{seat}")
}

fn zone_label(zone: Zone) -> &'static str {
    match zone {
        Zone::Battlefield => "battlefield",
        Zone::Hand => "hand",
        Zone::Library => "library",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Command => "command",
        Zone::Stack => "stack",
        Zone::Removed => "removed",
    }
}

/// Load the startup decks. A built-in default uses the compiled practice
/// bundle; an explicitly requested path is read (and refused loudly on error).
fn startup_decks(
    startup: &Startup,
) -> (
    aperture_core::DeckList,
    aperture_core::DeckList,
    Option<String>,
) {
    let defaults =
        UiSession::bundled_seats_for_profile(startup.seats.clamp(2, 4) as usize, startup.profile)
            .expect("the native table supports 2..=4 bundled seats");
    let (deck_a, err_a) = if startup.deck_a_explicit {
        load_explicit_deck(&startup.deck_a, "Temur Beats")
    } else {
        (defaults[0].clone(), None)
    };
    let (deck_b, err_b) = if startup.deck_b_explicit {
        load_explicit_deck(&startup.deck_b, "Izzet Control")
    } else {
        (defaults[1].clone(), None)
    };
    let refusal = match (err_a, err_b) {
        (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    (deck_a, deck_b, refusal)
}

/// Public data for one seat panel, assembled from the masked view.
struct SeatPanelData {
    seat: PlayerId,
    name: String,
    deck_name: String,
    life: i32,
    hand_count: usize,
    library_count: usize,
    eliminated: bool,
    battlefield: Vec<CardView>,
    hand: Vec<CardView>,
    graveyard: Vec<CardView>,
    exile: Vec<CardView>,
    command_zone: Vec<CardView>,
}

impl SeatPanelData {
    fn from_view(view: &PlayerView, seats: &[aperture_core::DeckList], seat: PlayerId) -> Self {
        if seat == view.viewer {
            SeatPanelData {
                seat,
                name: view.viewer_name.clone(),
                deck_name: seats
                    .get(seat as usize)
                    .map(|deck| deck.name.clone())
                    .unwrap_or_default(),
                life: view.own_life,
                hand_count: view.own_hand.len(),
                library_count: view.own_library_count,
                eliminated: view.own_eliminated,
                battlefield: view.own_battlefield.clone(),
                hand: view.own_hand.clone(),
                graveyard: view.own_graveyard.clone(),
                exile: view.own_exile.clone(),
                command_zone: view.own_command_zone.clone(),
            }
        } else if let Some(opponent) = view.opponent_view(seat) {
            SeatPanelData {
                seat,
                name: opponent.name.clone(),
                deck_name: opponent.deck_name.clone(),
                life: opponent.life,
                hand_count: opponent.hand_count,
                library_count: opponent.library_count,
                eliminated: opponent.eliminated,
                battlefield: opponent.battlefield.clone(),
                hand: Vec::new(),
                graveyard: opponent.graveyard.clone(),
                exile: opponent.exile.clone(),
                command_zone: opponent.command_zone.clone(),
            }
        } else {
            SeatPanelData {
                seat,
                name: seats
                    .get(seat as usize)
                    .map(|deck| deck.name.clone())
                    .unwrap_or_default(),
                deck_name: seats
                    .get(seat as usize)
                    .map(|deck| deck.name.clone())
                    .unwrap_or_default(),
                life: 0,
                hand_count: 0,
                library_count: 0,
                eliminated: true,
                battlefield: Vec::new(),
                hand: Vec::new(),
                graveyard: Vec::new(),
                exile: Vec::new(),
                command_zone: Vec::new(),
            }
        }
    }
}

/// Conditions disclosed in the opt-in performance report.
///
/// `--performance-note` describes what was running while the samples were
/// collected, so it is reported as a declared condition. Nothing in this
/// binary injects a synthetic delay, so the report never claims one; a future
/// harness that does inject one must add its own explicit source here.
fn performance_conditions(startup: &Startup) -> Conditions {
    Conditions {
        synthetic_delays: Vec::new(),
        notes: startup
            .performance_conditions
            .iter()
            .map(|note| note.trim().to_string())
            .filter(|note| !note.is_empty())
            .collect(),
    }
}

impl ApertureApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        theme::apply(&cc.egui_ctx);
        // A smoke run never spends its frames on model calls: the decider is
        // disabled for the whole capture even when --decider was passed, and a
        // note is printed so the capture is not mistaken for an opt-in run.
        let use_decider = startup.use_decider && startup.smoke.is_none();
        if startup.use_decider && startup.smoke.is_some() {
            eprintln!("smoke mode: Decider-2B calls are disabled for the capture");
        }
        let initial_profile = startup.profile;
        let seat_count = startup.seats.clamp(2, 4) as usize;
        let (deck_a, deck_b, deck_refusal) = startup_decks(&startup);
        let mut seats = UiSession::bundled_seats_for_profile(seat_count, initial_profile)
            .expect("the native table supports 2..=4 bundled seats");
        seats[0] = deck_a;
        seats[1] = deck_b;
        let human_seat = if seat_count == 2 && startup.ai_seat == P0 {
            // Backwards-compatible two-seat choice: the old --ai-seat 0 made
            // P1 the human's seat. The four-seat table is always human P0.
            1
        } else {
            P0
        };
        let ai = AiSettings {
            human_seat,
            ai_seats: (0..seat_count as PlayerId)
                .filter(|seat| *seat != human_seat)
                .collect(),
            autoplay: startup.autoplay || startup.smoke.is_some(),
            use_decider,
            ..AiSettings::default()
        };
        let mut session = UiSession::new(
            startup.seed,
            seats,
            ai,
            use_decider,
            startup.human_autoplay || startup.smoke.is_some(),
        )
        .expect("the native table always starts with valid bundled seat lists");
        session.profile = initial_profile;
        // Rebuild through the explicit profile constructor so a 40-life table
        // (or a 3/4-seat table) is never silently replaced by practice-20.
        session.new_game(startup.seed);
        session.deck_refusal = deck_refusal.clone();
        // A study launch prefills the branch-from-replay inputs; the panel is
        // opened with the Study button (B) and the branch is still an
        // explicitly user-confirmed action.
        if let Some(study_file) = startup.study_file.clone() {
            session.branch_path = study_file;
            session.branch_at = startup.branch_at;
            session.branch_source = None;
        }
        // A smoke run should show a developed board, not the opening hand.
        if startup.smoke.is_some() {
            for _ in 0..240 {
                if session.owner().is_none() || session.game.state.is_over() {
                    break;
                }
                session.step_ai();
            }
        }
        let perf = startup.performance_report.clone().map(|path| {
            PerformanceProbe::new(path).with_conditions(performance_conditions(&startup))
        });
        let initial_high_contrast = startup.initial_high_contrast;
        let initial_reduced_motion = startup.initial_reduced_motion;
        ApertureApp {
            session,
            startup,
            smoke_started: Instant::now(),
            smoke_requested: false,
            smoke_written: Arc::new(AtomicBool::new(false)),
            last_ai_step: Instant::now(),
            show_sandbox: false,
            show_import: deck_refusal.is_some(),
            show_zones: false,
            show_review: false,
            show_coach: true,
            show_study: false,
            show_help: false,
            show_accessibility: false,
            high_contrast: initial_high_contrast,
            reduced_motion: initial_reduced_motion,
            observed_theme: (initial_high_contrast, initial_reduced_motion),
            perf,
            setup_seats: seat_count as u8,
            setup_profile: initial_profile,
            observed_seats: seat_count as u8,
            observed_profile: initial_profile,
            zone_query: String::new(),
            zone_seat_filter: None,
            zone_zone_filter: None,
            confirm: None,
            counter_object: None,
            life_seat: P0,
            life_value: 20,
            zoom: 1.0,
            frame_count: 0,
        }
    }

    /// Ask before a study branch replaces the live position. The Study panel
    /// button and the G shortcut both come through here, so the keyboard path
    /// can never run the destructive replace that the button gates.
    fn ask_branch(&mut self, path: PathBuf, keep: usize) {
        if self.session.io_busy {
            self.session.message =
                "refused: a table operation is still running; the branch was not requested"
                    .to_string();
            return;
        }
        if path.as_os_str().is_empty() {
            self.session.message =
                "refused: choose a study file (a session save or a replay) before branching"
                    .to_string();
            return;
        }
        self.confirm = Some(PendingAction::Branch { path, keep });
    }

    /// Run a pending destructive action after its confirmation.
    fn apply_pending(&mut self, action: PendingAction) {
        self.confirm = None;
        match action {
            PendingAction::NewGame { seed } => {
                self.session.new_game(seed);
                self.session.message = format!("started a new game with seed {seed}");
            }
            PendingAction::NewTable {
                seed,
                seats,
                profile,
            } => match self.session.start_table(seed, seats, profile) {
                Ok(()) => {
                    self.session.deck_refusal = None;
                    self.session.message = format!(
                        "started a {seats}-seat table, profile {} ({} life); human P0, other seats heuristic AI",
                        profile.label(),
                        profile.starting_life()
                    );
                }
                // A failed table start is an error state, not a hint: it uses
                // the same `refused:` prefix the error-colour detector reads.
                Err(err) => self.session.message = format!("refused: {err}"),
            },
            PendingAction::Load { path } => self.session.request_load(path),
            PendingAction::Branch { path, keep } => self.session.request_replay_branch(path, keep),
            PendingAction::ResetSandboxGame { seed } => {
                self.session.new_game(seed);
                self.session.message = format!(
                    "reset the table with seed {seed}: the sandbox taint is cleared with the old position"
                );
            }
            PendingAction::StartImported { seed } => {
                if let Err(err) = self.session.start_imported_match(seed) {
                    self.session.message = format!("refused: {err}");
                }
            }
        }
    }

    /// The confirmation bar content. Nothing destructive runs until the user
    /// presses the explicit confirm button.
    fn confirm_bar(&mut self, ui: &mut egui::Ui) {
        let Some(pending) = self.confirm.clone() else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("CONFIRM: {}", pending.question()))
                    .color(theme::warn(ui))
                    .strong(),
            );
            if ui.button(pending.confirm_label()).clicked() {
                self.apply_pending(pending.clone());
            }
            if ui.button("Cancel").clicked() {
                self.confirm = None;
                self.session.message = "cancelled — nothing was changed".to_string();
            }
        });
    }

    fn tick_ai(&mut self, ctx: &egui::Context) {
        if self.session.review_mode {
            // The omniscient review is explicitly not a policy input: no AI,
            // no autoplay pacing and no decision work runs while it is shown.
            return;
        }
        if self.session.game.state.is_over() {
            return;
        }
        let owner = self.session.owner();
        let ai_turn = owner.is_some_and(|seat| self.session.seat_is_ai(seat));
        let paced = self.session.ai.autoplay
            && ai_turn
            && !self.session.ai_busy
            && self.last_ai_step.elapsed() >= Duration::from_millis(600);
        if paced && self.session.request_ai_decision_default() {
            self.last_ai_step = Instant::now();
        }
        if self.session.ai.autoplay && !self.session.game.state.is_over() {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }

    fn set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        // 200% is the usable upper bound: the fixed strip stays reachable
        // inside its own bounded scroll area at the 1024x640 minimum.
        self.zoom = zoom.clamp(0.7, 2.0);
        ctx.set_zoom_factor(self.zoom);
    }

    /// Keyboard controls that do not conflict with a focused text field. Tab
    /// and Shift+Tab move egui focus; Enter activates the focused control.
    fn keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Esc is handled before the text-focus guard so it can dismiss the
        // accessibility panel even from a focused field. When the panel is
        // open Esc closes only the panel and returns: it never falls through
        // to cancelling a confirmation or any other state in the same press.
        let esc = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        if esc && self.show_accessibility {
            self.show_accessibility = false;
            return;
        }
        if ctx.text_edit_focused() {
            return;
        }
        let (
            pass,
            new_game,
            save,
            load,
            import,
            zones,
            sandbox,
            coach,
            study,
            review,
            help,
            accessibility,
            plus,
            minus,
            reset_zoom,
            undo,
        ) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::P),
                input.key_pressed(egui::Key::N),
                input.key_pressed(egui::Key::S),
                input.key_pressed(egui::Key::L),
                input.key_pressed(egui::Key::I),
                input.key_pressed(egui::Key::Z),
                input.key_pressed(egui::Key::K),
                input.key_pressed(egui::Key::C),
                input.key_pressed(egui::Key::B),
                input.key_pressed(egui::Key::R),
                input.key_pressed(egui::Key::H),
                input.key_pressed(egui::Key::A),
                input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
                input.key_pressed(egui::Key::Minus),
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: egui::Key::Num0,
                            pressed: true,
                            modifiers,
                            ..
                        } if modifiers.command
                    )
                }),
                input.key_pressed(egui::Key::U),
            )
        });
        if review {
            if self.session.review_mode {
                self.session.leave_review();
            } else if let Err(err) = self.session.enter_review() {
                self.session.message = err.clone();
            }
        }
        if study {
            self.show_study = !self.show_study;
        }
        // G asks the same branch question the Study panel button asks, using
        // the study file and action index currently shown there. It never
        // replaces the live position on its own.
        if ctx.input(|input| input.key_pressed(egui::Key::G)) {
            let path = PathBuf::from(self.session.branch_path.clone());
            let keep = self.session.branch_at;
            self.ask_branch(path, keep);
        }
        if pass && self.session.owner() == Some(self.session.human_seat()) {
            let _ = self.session.submit_human(Action::Pass);
        }
        if new_game {
            // Destructive: ask first. The confirm bar is part of the primary
            // strip, so the question never hides behind a scrolling panel.
            let seed = self.session.seed.wrapping_add(1);
            self.confirm = Some(PendingAction::NewGame { seed });
        }
        if undo && let Err(err) = self.session.undo_last_action() {
            self.session.message = err;
        }
        if save {
            self.session
                .request_save(PathBuf::from(self.session.save_path.clone()));
        }
        if load {
            self.confirm = Some(PendingAction::Load {
                path: PathBuf::from(self.session.save_path.clone()),
            });
        }
        if import {
            self.show_import = !self.show_import;
        }
        if zones {
            self.show_zones = !self.show_zones;
        }
        if sandbox {
            self.show_sandbox = !self.show_sandbox;
        }
        if coach {
            self.show_coach = !self.show_coach;
        }
        // V opens the move-review panel and asks the worker for a fresh
        // analysis of the decision in front of the reviewing seat. It never
        // acts on the table.
        if ctx.input(|input| input.key_pressed(egui::Key::V)) {
            self.show_review = !self.show_review;
            if self.show_review {
                self.request_move_review();
            }
        }
        if help {
            self.show_help = !self.show_help;
        }
        if accessibility {
            self.show_accessibility = !self.show_accessibility;
        }
        if reset_zoom {
            self.set_zoom(ctx, 1.0);
        }
        if plus {
            let zoom = self.zoom + 0.1;
            self.set_zoom(ctx, zoom);
        }
        if minus {
            let zoom = self.zoom - 0.1;
            self.set_zoom(ctx, zoom);
        }
        if esc {
            // Esc cancels a pending target choice and closes the inspector or an
            // unconfirmed destructive action; it never performs anything.
            self.session.target_pick = None;
            self.show_zones = false;
            self.confirm = None;
        }
    }

    /// Install the selected contrast/animation mode when it changes. egui
    /// applies the `active` widget style to keyboard focus, so the installed
    /// 2 px focus stroke is what Tab/Enter users see. No widget is focused
    /// programmatically here.
    fn apply_theme_if_needed(&mut self, ctx: &egui::Context) {
        // The palette is per-context data, so publish it on every pass; only
        // the (more expensive) style install runs when the mode changes.
        theme::publish_palette(ctx, self.high_contrast);
        let mode = (self.high_contrast, self.reduced_motion);
        if self.frame_count <= 4 || self.observed_theme != mode {
            theme::apply_mode(ctx, self.high_contrast, self.reduced_motion);
            self.observed_theme = mode;
        }
    }

    /// Busy indicator. Reduced motion renders static text instead of an
    /// animated spinner; the worker polling itself is unaffected.
    fn working_indicator(&self, ui: &mut egui::Ui) {
        if self.reduced_motion {
            ui.label(RichText::new("Working...").color(theme::brass(ui)));
        } else {
            ui.spinner();
        }
    }

    fn accessibility_panel(&mut self, ctx: &egui::Context) {
        if !self.show_accessibility {
            return;
        }
        let mut open = true;
        let mut high_contrast = self.high_contrast;
        let mut reduced_motion = self.reduced_motion;
        egui::Window::new("Accessibility (A)")
            .open(&mut open)
            .resizable(false)
            .default_width(430.0)
            .show(ctx, |ui| {
                theme::muted(
                    ui,
                    "Explicit settings for this window. Tab / Shift+Tab move focus, Enter activates the focused control, and the focused control gets a 2 px outline.",
                );
                ui.separator();
                ui.checkbox(
                    &mut high_contrast,
                    "High contrast (white text, near-black surfaces, bright borders)",
                );
                ui.label(format!(
                    "High contrast: {}",
                    if high_contrast {
                        "ON — white text on near-black surfaces, bright borders and accent text"
                    } else {
                        "OFF — original charcoal/brass palette"
                    }
                ));
                ui.checkbox(
                    &mut reduced_motion,
                    "Reduced motion (static busy text, no animation)",
                );
                ui.label(format!(
                    "Reduced motion: {}",
                    if reduced_motion {
                        "ON — spinners are replaced by static 'Working...' text and animation time is 0"
                    } else {
                        "OFF — normal spinner animation"
                    }
                ));
                ui.separator();
                theme::muted(
                    ui,
                    "Esc closes this panel first; a second Esc cancels the next pending UI state (never a destructive action). A opens or closes this panel only when no text field is focused.",
                );
                if let Some(perf) = &self.perf {
                    ui.separator();
                    ui.label(format!(
                        "Performance report: {}",
                        perf.status().unwrap_or_else(|| format!(
                            "collecting {:.1}s of {:.0}s",
                            perf.elapsed().as_secs_f32(),
                            10.0
                        ))
                    ));
                }
            });
        self.show_accessibility = open;
        self.high_contrast = high_contrast;
        self.reduced_motion = reduced_motion;
    }

    fn inspect_card(&mut self, card: &CardView) {
        let counters = if card.plus_one_counters > 0 {
            format!(
                "\n+1/+1 counters: {} (sandbox storage only; not a rules-supported counter system)",
                card.plus_one_counters
            )
        } else {
            String::new()
        };
        let pump = if card.pump_power != 0 || card.pump_toughness != 0 {
            format!(
                "\nuntil-end-of-turn pump: {:+}/{:+}",
                card.pump_power, card.pump_toughness
            )
        } else {
            String::new()
        };
        // The zone is read from this viewer's masked projection: a shared
        // inspector is called from hand, a public zone, the battlefield and the
        // command zone, so it never hardcodes or invents a location.
        let view = self.session.view(self.session.human_seat());
        let zone_line = match view.zone_of(card.id) {
            Some(zone) => format!("zone: {}", zone_label(zone)),
            None => "zone: not projected in your view".to_string(),
        };
        self.session.inspect = Some(format!(
            "{} — {}\n{} (mana cost)\ncontroller P{}{}{}\n{}\n\n{}",
            card.name,
            if card.card.def().is_land() {
                "land"
            } else {
                "permanent / spell"
            },
            card.cost.unwrap_or("(no cost)"),
            card.controller,
            if card.tapped { "\ntapped" } else { "" },
            zone_line,
            counters + &pump,
            card.oracle
        ));
    }

    fn pending_controls(&mut self, ui: &mut egui::Ui, view: &PlayerView) {
        let Some(kind) = view.pending_kind else {
            return;
        };
        theme::heading(ui, "Pending choice");
        if let Some(description) = &view.pending {
            ui.label(RichText::new(description).color(theme::warn(ui)));
        }
        match kind {
            aperture_core::view::PendingKind::Mulligan => {
                ui.horizontal(|ui| {
                    if ui.button("Keep this hand").clicked() {
                        let _ = self.session.submit_human(Action::Mulligan { keep: true });
                    }
                    if ui.button("Mulligan").clicked() {
                        let _ = self.session.submit_human(Action::Mulligan { keep: false });
                    }
                });
                theme::muted(
                    ui,
                    "London mulligan: a fresh seven, then one card on the bottom per mulligan (CR 103.5)",
                );
            }
            aperture_core::view::PendingKind::CombatDamageAssignment { attacker, card } => {
                ui.label(format!(
                    "{} ({attacker}) is blocked — assign its combat damage as though it weren't?",
                    card.name()
                ));
                ui.horizontal(|ui| {
                    if ui.button("As though unblocked").clicked() {
                        let _ = self
                            .session
                            .submit_human(Action::AnswerCombatDamageAssignment {
                                as_though_unblocked: true,
                            });
                    }
                    if ui.button("Normally").clicked() {
                        let _ = self
                            .session
                            .submit_human(Action::AnswerCombatDamageAssignment {
                                as_though_unblocked: false,
                            });
                    }
                });
                theme::muted(
                    ui,
                    "CR 510.1: all of its combat damage goes to the defending player and none to the blockers",
                );
            }
            aperture_core::view::PendingKind::OrderCard { index, mode } => {
                let name = view
                    .pending_viewed
                    .get(index)
                    .map(|card| card.name)
                    .unwrap_or("(unknown card)");
                ui.label(format!("Looking at: {name}"));
                ui.horizontal(|ui| {
                    if ui.button(format!("Put {name} on top")).clicked() {
                        let _ = self
                            .session
                            .submit_human(Action::OrderCard { to_top: true });
                    }
                    let off_label = match mode {
                        aperture_core::OrderMode::Scry => format!("Put {name} on the bottom"),
                        aperture_core::OrderMode::Surveil => {
                            format!("Put {name} into the graveyard")
                        }
                    };
                    if ui.button(off_label).clicked() {
                        let _ = self
                            .session
                            .submit_human(Action::OrderCard { to_top: false });
                    }
                });
            }
            aperture_core::view::PendingKind::DeclareAttackers => {
                let legal = self.session.game.legal_attackers(view.viewer);
                let defenders: Vec<PlayerId> = view
                    .opponents
                    .iter()
                    .filter(|o| !o.eliminated)
                    .map(|o| o.seat)
                    .collect();
                if !defenders.is_empty()
                    && self
                        .session
                        .selected_defender
                        .is_none_or(|seat| !defenders.contains(&seat))
                {
                    self.session.selected_defender = defenders.first().copied();
                }
                ui.label("Declare attackers (structured multi-select):");
                for id in legal {
                    let selected = self.session.selected_attackers.contains(&id);
                    let label = self
                        .session
                        .game
                        .state
                        .try_object(id)
                        .map(|o| format!("{} ({}/{})", o.name(), o.power(), o.toughness()))
                        .unwrap_or_default();
                    if ui.selectable_label(selected, label).clicked() {
                        if selected {
                            self.session.selected_attackers.retain(|x| *x != id);
                        } else {
                            self.session.selected_attackers.push(id);
                        }
                    }
                }
                ui.label("Defending seat:");
                for defender in defenders {
                    let selected = self.session.selected_defender == Some(defender);
                    if ui
                        .selectable_label(selected, format!("P{defender}"))
                        .clicked()
                    {
                        self.session.selected_defender = Some(defender);
                    }
                }
                if ui
                    .button(format!(
                        "Declare {} attacker(s)",
                        self.session.selected_attackers.len()
                    ))
                    .clicked()
                {
                    let attackers = self.session.selected_attackers.clone();
                    let defender = if attackers.is_empty() {
                        None
                    } else {
                        self.session.selected_defender
                    };
                    let _ = self.session.submit_human(Action::DeclareAttackers {
                        attackers,
                        defender,
                    });
                }
            }
            aperture_core::view::PendingKind::DeclareBlockers => {
                let defender = view.viewer;
                let blockers = self.session.game.legal_blockers(defender);
                let attackers = view.attackers.clone();
                ui.label("Assign blockers:");
                for blocker in blockers {
                    let name = self
                        .session
                        .game
                        .state
                        .try_object(blocker)
                        .map(|o| o.name())
                        .unwrap_or("creature");
                    ui.horizontal(|ui| {
                        ui.label(format!("{name} blocks:"));
                        for attacker in &attackers {
                            let attacker_name = self
                                .session
                                .game
                                .state
                                .try_object(*attacker)
                                .map(|o| o.name())
                                .unwrap_or("attacker");
                            let chosen = self
                                .session
                                .selected_blocks
                                .iter()
                                .any(|(b, a)| *b == blocker && *a == *attacker);
                            if ui
                                .selectable_label(
                                    chosen,
                                    format!("{attacker_name} #{:?}", attacker.0),
                                )
                                .clicked()
                            {
                                self.session.selected_blocks.retain(|(b, _)| *b != blocker);
                                if !chosen {
                                    self.session.selected_blocks.push((blocker, *attacker));
                                }
                            }
                        }
                    });
                }
                if ui
                    .button(format!(
                        "Declare {} block(s)",
                        self.session.selected_blocks.len()
                    ))
                    .clicked()
                {
                    let blocks = block_list(&self.session.selected_blocks);
                    let _ = self
                        .session
                        .submit_human(Action::DeclareBlockers { blocks });
                }
            }
            aperture_core::view::PendingKind::CommanderReplacement => {
                ui.label("Commander replacement choice (CR 903.9b):");
                let choices: Vec<(Action, String)> = view
                    .legal
                    .iter()
                    .filter_map(|legal| match &legal.action {
                        Action::AnswerCommanderReplacement { .. } => {
                            Some((legal.action.clone(), legal.label.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                for (action, label) in choices {
                    if ui.button(label).clicked() {
                        let _ = self.session.submit_human(action);
                    }
                }
            }
            aperture_core::view::PendingKind::CommanderDraw => {
                ui.label("Commander draw replacement choice (CR 903.9b):");
                for legal in view
                    .legal
                    .iter()
                    .filter(|legal| matches!(legal.action, Action::AnswerCommanderDraw { .. }))
                {
                    if ui.button(legal.label.clone()).clicked() {
                        let _ = self.session.submit_human(legal.action.clone());
                    }
                }
            }
            aperture_core::view::PendingKind::CommanderReturn => {
                ui.label("Commander return choice (CR 903.9a):");
                let choices: Vec<(Action, String)> = view
                    .legal
                    .iter()
                    .filter_map(|legal| match &legal.action {
                        Action::AnswerCommanderReturn { .. } => {
                            Some((legal.action.clone(), legal.label.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                for (action, label) in choices {
                    if ui.button(label).clicked() {
                        let _ = self.session.submit_human(action);
                    }
                }
            }
            aperture_core::view::PendingKind::ApnapSacrifice => {
                ui.label("Choose one of your permanents to sacrifice:");
                let choices: Vec<(Action, String)> = view
                    .legal
                    .iter()
                    .filter_map(|legal| match &legal.action {
                        Action::ChooseSacrifice { .. } => {
                            Some((legal.action.clone(), legal.label.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                for (action, label) in choices {
                    if ui.button(label).clicked() {
                        let _ = self.session.submit_human(action);
                    }
                }
            }
            aperture_core::view::PendingKind::ApnapReveal => {
                ui.label("Reveal any number of creature cards from your hand:");
                let choices: Vec<(Action, String)> = view
                    .legal
                    .iter()
                    .filter_map(|legal| match &legal.action {
                        Action::RevealCreatureCards { .. } => {
                            Some((legal.action.clone(), legal.label.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                for (action, label) in choices {
                    if ui.button(label).clicked() {
                        let _ = self.session.submit_human(action);
                    }
                }
            }
            aperture_core::view::PendingKind::Discard { count }
            | aperture_core::view::PendingKind::BottomCards { count } => {
                let is_discard = matches!(kind, aperture_core::view::PendingKind::Discard { .. });
                ui.label(format!(
                    "Choose exactly {count} card(s) to {}:",
                    if is_discard {
                        "discard"
                    } else {
                        "put on the bottom"
                    }
                ));
                for card in &view.own_hand {
                    let selected = self.session.selected_cards.contains(&card.id);
                    let label = format!("{} {}", card.name, card.cost.unwrap_or(""));
                    if ui.selectable_label(selected, label).clicked() {
                        if selected {
                            self.session.selected_cards.retain(|x| *x != card.id);
                        } else {
                            self.session.selected_cards.push(card.id);
                        }
                    }
                }
                let ready = self.session.selected_cards.len() == count as usize;
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(if is_discard { "Discard" } else { "Bottom" }),
                    )
                    .clicked()
                {
                    let cards = self.session.selected_cards.clone();
                    let action = if is_discard {
                        Action::Discard { cards }
                    } else {
                        Action::BottomCards { cards }
                    };
                    let _ = self.session.submit_human(action);
                }
            }
            aperture_core::view::PendingKind::TriggerTarget => {
                ui.label("Choose a target for the triggered ability (CR 603.3d):");
                for legal in view
                    .legal
                    .iter()
                    .filter(|legal| matches!(legal.action, Action::ChooseTriggerTarget { .. }))
                {
                    if ui.button(legal.label.clone()).clicked() {
                        let _ = self.session.submit_human(legal.action.clone());
                    }
                }
            }
            aperture_core::view::PendingKind::TriggerChoice => {
                ui.label("Optional triggered ability (CR 603.5):");
                for legal in view
                    .legal
                    .iter()
                    .filter(|legal| matches!(legal.action, Action::AnswerTriggerChoice { .. }))
                {
                    if ui.button(legal.label.clone()).clicked() {
                        let _ = self.session.submit_human(legal.action.clone());
                    }
                }
            }
        }
    }

    fn card_label(card: &CardView) -> String {
        let mut label = String::new();
        label.push_str(card.name);
        if let Some(cost) = card.cost {
            label.push_str(&format!("  {cost}"));
        }
        if let (Some(power), Some(toughness)) = (card.power, card.toughness) {
            label.push_str(&format!("  {power}/{toughness}"));
        }
        if card.tapped {
            label.push_str("  [tapped]");
        }
        if card.damage > 0 {
            label.push_str(&format!("  [{} dmg]", card.damage));
        }
        if card.plus_one_counters > 0 {
            // Stored by the sandbox only: the engine's combat math ignores
            // these counters, so the label never implies rules support.
            label.push_str(&format!("  [{} +1/+1 sandbox]", card.plus_one_counters));
        }
        if card.pump_power != 0 || card.pump_toughness != 0 {
            label.push_str(&format!(
                "  [pump {:+}/{:+}]",
                card.pump_power, card.pump_toughness
            ));
        }
        label
    }

    /// A battlefield row. A click always inspects the card: the counter editor
    /// in the sandbox panel is the only way to edit counters, so a stray click
    /// on a permanent can never silently mutate the game.
    fn battlefield_ui(&mut self, ui: &mut egui::Ui, cards: &[CardView]) {
        if cards.is_empty() {
            theme::muted(ui, "(no permanents)");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            let mut inspect_card: Option<CardView> = None;
            for card in cards {
                let label = Self::card_label(card);
                let text = RichText::new(label).color(if card.card.def().is_land() {
                    theme::muted_color(ui)
                } else {
                    theme::text_color(ui)
                });
                let response = ui.add(egui::Button::new(text).min_size(egui::vec2(104.0, 32.0)));
                if response.clicked() || response.secondary_clicked() {
                    inspect_card = Some(card.clone());
                }
                response.on_hover_text(format!(
                    "{} — click to inspect. {}",
                    card.oracle,
                    if card.plus_one_counters > 0 {
                        format!(
                            "{} +1/+1 sandbox counter(s); edit them in the sandbox counter editor.",
                            card.plus_one_counters
                        )
                    } else {
                        "Counters are edited in the sandbox panel.".to_string()
                    }
                ));
            }
            if let Some(card) = inspect_card {
                self.inspect_card(&card);
            }
        });
    }

    fn seat_panel(
        &mut self,
        ui: &mut egui::Ui,
        data: &SeatPanelData,
        view: &PlayerView,
        is_own: bool,
    ) {
        let seat = data.seat;
        let active = view.active == seat;
        let priority = view.priority == Some(seat);
        let pending_owner = view.pending_owner == Some(seat) && view.pending.is_some();
        ui.group(|ui| {
            ui.set_min_width(300.0);
            ui.horizontal_wrapped(|ui| {
                let role = if is_own {
                    " · YOU"
                } else {
                    " · heuristic AI"
                };
                ui.label(
                    RichText::new(format!("P{seat}{role}"))
                        .color(theme::brass(ui))
                        .strong(),
                );
                if active {
                    ui.label(RichText::new("ACTIVE").color(theme::good(ui)).small());
                }
                if priority {
                    ui.label(RichText::new("PRIORITY").color(theme::brass(ui)).small());
                }
                if pending_owner {
                    ui.label(RichText::new("PENDING").color(theme::warn(ui)).small());
                }
                if data.eliminated {
                    ui.label(RichText::new("ELIMINATED").color(theme::bad(ui)).strong());
                }
            });
            ui.label(
                RichText::new(format!(
                    "{} · {} · life {} · hand {} · library {}",
                    data.name, data.deck_name, data.life, data.hand_count, data.library_count
                ))
                .small(),
            );
            ui.horizontal_wrapped(|ui| {
                for (zone, count) in [
                    (Zone::Graveyard, data.graveyard.len()),
                    (Zone::Exile, data.exile.len()),
                    (Zone::Command, data.command_zone.len()),
                ] {
                    if ui
                        .small_button(format!("{} {count}", zone_label(zone)))
                        .clicked()
                    {
                        self.zone_seat_filter = Some(seat);
                        self.zone_zone_filter = Some(zone);
                        self.show_zones = true;
                    }
                }
                theme::muted(
                    ui,
                    if is_own {
                        "public zones"
                    } else {
                        "public zones; hand/library contents hidden"
                    },
                );
            });
            for commander in view.commanders.iter().filter(|c| c.owner == seat) {
                ui.label(
                    RichText::new(format!(
                        "Commander {} · tax {{+{}}} · zone: {}",
                        commander.name,
                        commander.tax,
                        commander
                            .zone
                            .map(zone_label)
                            .unwrap_or("hidden (hand/library)")
                    ))
                    .small()
                    .color(theme::brass(ui)),
                );
            }
            let damage: Vec<String> = view
                .commander_damage
                .iter()
                .filter(|entry| entry.recipient == seat && entry.amount > 0)
                .map(|entry| {
                    let name = view
                        .commanders
                        .iter()
                        .find(|commander| commander.id == entry.commander)
                        .map(|commander| commander.name)
                        .unwrap_or("commander");
                    format!("{name} dmg {}", entry.amount)
                })
                .collect();
            if !damage.is_empty() {
                ui.label(
                    RichText::new(format!("Commander damage: {}", damage.join(" · ")))
                        .small()
                        .color(theme::warn(ui)),
                );
            }
            self.battlefield_ui(ui, &data.battlefield);
            if is_own {
                theme::heading(ui, "YOUR HAND");
                if data.hand.is_empty() {
                    theme::muted(ui, "(empty)");
                }
                egui::ScrollArea::vertical()
                    .id_salt("own-hand")
                    .max_height(170.0)
                    .show(ui, |ui| {
                        let legal = view.legal.clone();
                        let mut play: Option<Action> = None;
                        for card in &data.hand {
                            let matching: Vec<Action> = legal
                                .iter()
                                .filter(|entry| match (&entry.action, card.card.def().is_land()) {
                                    (Action::PlayLand { card: id }, true) => *id == card.id,
                                    (Action::CastSpell { card: id, .. }, false) => *id == card.id,
                                    _ => false,
                                })
                                .map(|entry| entry.action.clone())
                                .collect();
                            let label = Self::card_label(card);
                            let enabled = !matching.is_empty();
                            let response = ui.add_enabled(
                                enabled,
                                egui::Button::new(RichText::new(label).color(if enabled {
                                    theme::text_color(ui)
                                } else {
                                    theme::muted_color(ui)
                                })),
                            );
                            if response.clicked() {
                                match matching.len() {
                                    0 => self.inspect_card(card),
                                    1 => play = Some(matching[0].clone()),
                                    _ => {
                                        self.session.target_pick = Some((card.id, matching.clone()))
                                    }
                                }
                            }
                            if response.secondary_clicked() {
                                self.inspect_card(card);
                            }
                            response.on_hover_text(format!(
                                "{} — click to {}",
                                card.oracle,
                                if enabled { "cast/play" } else { "inspect" }
                            ));
                        }
                        if let Some(action) = play {
                            let _ = self.session.submit_human(action);
                        }
                    });
            }
        });
    }

    fn import_panel(&mut self, ui: &mut egui::Ui) {
        self.import_panel_contents(ui);
    }

    fn import_panel_contents(&mut self, ui: &mut egui::Ui) {
        let profile = self.session.profile;
        theme::heading(ui, "Deck import — one certified list per seat (2-4 seats)");
        theme::muted(
            ui,
            format!(
                "URL form: https://archidekt.com/decks/<id>[/slug] (host-allowlisted, canonical API built from the id). \
                 Text fallback: paste a list or point at a file. Each seat is certified for the selected profile \
                 ({}, {}-life) on two separate axes — Commander format legality and engine support — and the exact \
                 unsupported cards are listed below before start is enabled. Unsupported input is refused, never \
                 replaced by a bundled deck.",
                profile.label(),
                profile.starting_life()
            ),
        );
        for seat in 0..self.session.seats.len() as PlayerId {
            let index = seat as usize;
            let busy = self.session.import_busy[index];
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}{}",
                        seat_label(seat),
                        if self.session.seat_is_ai(seat) {
                            " (AI seat)"
                        } else {
                            " (your seat)"
                        }
                    ))
                    .color(theme::brass(ui))
                    .strong(),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("URL:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.session.seat_url[index])
                        .desired_width(420.0),
                );
                if ui
                    .add_enabled(!busy, egui::Button::new("Import URL"))
                    .clicked()
                {
                    let url = self.session.seat_url[index].clone();
                    let cache = PathBuf::from("cache");
                    self.session.request_import(seat, move || {
                        import_seat_url_for(seat, &url, &cache, profile)
                    });
                }
            });
            ui.label("Text fallback (paste a deck list):");
            ui.add(
                egui::TextEdit::multiline(&mut self.session.seat_text[index])
                    .desired_rows(3)
                    .desired_width(470.0),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(!busy, egui::Button::new("Import pasted text"))
                    .clicked()
                {
                    let text = self.session.seat_text[index].clone();
                    let name = format!("P{seat} pasted list");
                    self.session.request_import(seat, move || {
                        import_seat_text_for(seat, &name, &text, profile)
                    });
                }
                ui.label("file:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.session.seat_path[index])
                        .desired_width(280.0),
                );
                if ui
                    .add_enabled(!busy, egui::Button::new("Import file"))
                    .clicked()
                {
                    let path = PathBuf::from(self.session.seat_path[index].clone());
                    self.session
                        .request_import(seat, move || import_seat_file_for(seat, &path, profile));
                }
                if busy {
                    self.working_indicator(ui);
                    if !self.reduced_motion {
                        ui.label("working…");
                    }
                }
            });
            if let Some(err) = &self.session.seat_error[index] {
                ui.label(RichText::new(format!("ERROR: {err}")).color(theme::bad(ui)));
            }
            if let Some(import) = &self.session.seat_import[index] {
                let (status, color) = if import.certified() {
                    ("CERTIFIED", theme::good(ui))
                } else {
                    ("REFUSED", theme::bad(ui))
                };
                ui.label(
                    RichText::new(format!("{status} — {}", import.status_line()))
                        .color(color)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!(
                        "format legality (Commander construction): {} · engine support: {} ({}/{} cards) · profile {}",
                        import.format_status, import.support_status, import.supported, import.total, import.profile
                    ))
                    .small(),
                );
                if let Some(list) = &import.list {
                    let pool = aperture_core::catalog::pool_size();
                    ui.label(RichText::new(format!(
                        "planned list applied verbatim: {} card(s) across {} distinct cards; every card is in the {pool}-card pool{}",
                        list.size(),
                        list.cards.len(),
                        import
                            .commander
                            .map(|c| format!("; commander {}", c.name()))
                            .unwrap_or_default()
                    )));
                }
                if import.unsupported.is_empty() {
                    ui.label(RichText::new("unsupported cards: none").small());
                } else {
                    ui.label(
                        RichText::new(format!(
                            "unsupported cards ({} card(s), exact reasons):",
                            import.unsupported.len()
                        ))
                        .color(theme::bad(ui))
                        .small(),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .id_salt(("unsupported", index))
                        .show(ui, |ui| {
                            for unsupported in &import.unsupported {
                                ui.label(
                                    RichText::new(format!(
                                        "  {} x{} — {}",
                                        unsupported.name, unsupported.quantity, unsupported.reason
                                    ))
                                    .monospace()
                                    .small(),
                                );
                            }
                        });
                }
                if let Some(refusal) = &import.refusal {
                    ui.label(
                        RichText::new(format!("refused: {refusal}"))
                            .color(theme::bad(ui))
                            .small(),
                    );
                }
                egui::CollapsingHeader::new("full certification report")
                    .id_salt(("import-report", index))
                    .show(ui, |ui| {
                        ui.label(RichText::new(import.summary.clone()).monospace().small());
                    });
            }
        }
        ui.separator();
        let ready = self.session.can_start_imported();
        ui.horizontal_wrapped(|ui| {
            let start_label = if ready {
                "Start imported match"
            } else {
                "Start imported match (disabled — every seat must be certified)"
            };
            if ui
                .add_enabled(ready, egui::Button::new(start_label))
                .clicked()
            {
                let seed = self.session.seed.wrapping_add(1);
                self.confirm = Some(PendingAction::StartImported { seed });
            }
            if !ready {
                theme::muted(
                    ui,
                    "disabled until EVERY seat is certified (an unsupported deck cannot start)",
                );
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.session.health_busy,
                    egui::Button::new("Check local Decider-2B health"),
                )
                .clicked()
            {
                let timeout = self.session.ai.decider_timeout_ms;
                self.session.request_health(move || {
                    let config = DeciderConfig {
                        timeout_ms: timeout,
                        ..DeciderConfig::default()
                    };
                    match DeciderClient::new(config) {
                        Ok(client) => match client.health() {
                            Ok(true) => {
                                "decider-2b /health reachable on http://127.0.0.1:8765".to_string()
                            }
                            Ok(false) => {
                                "decider-2b answered, but with an error status".to_string()
                            }
                            Err(err) => {
                                format!("decider-2b unreachable ({err}); heuristic active")
                            }
                        },
                        Err(err) => format!("decider config refused: {err}"),
                    }
                });
            }
            if self.session.health_busy {
                self.working_indicator(ui);
                ui.label("checking local health…");
            }
        });
        if let Some(status) = &self.session.decider_status {
            ui.label(RichText::new(status).color(theme::brass(ui)).small());
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("save/load path:");
            ui.text_edit_singleline(&mut self.session.save_path);
        });
    }

    /// The normal (non-review) primary strip content: own pending choices and
    /// legal actions.
    fn primary_actions(
        &mut self,
        ui: &mut egui::Ui,
        view: &PlayerView,
        owner: Option<PlayerId>,
        human: PlayerId,
    ) {
        if owner == Some(human) && view.pending_kind.is_some() {
            let pending_view = view.clone();
            self.pending_controls(ui, &pending_view);
        }
        if owner != Some(human) {
            if let Some(owner) = owner {
                theme::heading(ui, &format!("WAITING ON P{owner}"));
                if view.priority == Some(owner) {
                    theme::muted(ui, format!("P{owner} holds priority."));
                } else {
                    theme::muted(
                        ui,
                        format!(
                            "P{owner} has a pending decision; details are private to that seat. No player has priority."
                        ),
                    );
                }
                if self.session.seat_is_ai(owner) && ui.button("Let the AI answer (step)").clicked()
                {
                    self.session.request_ai_decision_default();
                }
            }
        } else {
            // A hand choice (CR 514.1 discard, CR 103.5 bottoming) is answered by
            // the selection panel above, where the player picks the exact cards.
            // The engine now offers one action per legal set (up to the documented
            // bound), so re-listing every one of them as a button here would only
            // duplicate that panel and crowd the strip.
            let hand_choice = matches!(
                view.pending_kind,
                Some(aperture_core::view::PendingKind::Discard { .. })
                    | Some(aperture_core::view::PendingKind::BottomCards { .. })
            );
            if view.legal.is_empty() {
                theme::muted(ui, "no legal action for your seat right now");
            }
            let mut chosen: Option<Action> = None;
            if !hand_choice {
                ui.horizontal_wrapped(|ui| {
                    for entry in &view.legal {
                        let label = format!("{}   [CR {}]", entry.label, entry.rules.join(", "));
                        if ui.button(label).clicked() {
                            chosen = Some(entry.action.clone());
                        }
                    }
                });
            }
            if let Some(action) = chosen {
                let _ = self.session.submit_human(action);
            }
            if let Some((card, options)) = self.session.target_pick.clone() {
                let name = self
                    .session
                    .game
                    .state
                    .try_object(card)
                    .map(|o| o.name())
                    .unwrap_or("spell");
                ui.label(
                    RichText::new(format!("Choose a target for {name}:")).color(theme::brass(ui)),
                );
                ui.horizontal_wrapped(|ui| {
                    for action in options {
                        let label = view
                            .legal
                            .iter()
                            .find(|entry| entry.action == action)
                            .map(|entry| entry.label.clone())
                            .unwrap_or_else(|| "cast".to_string());
                        if ui.button(label).clicked() {
                            let _ = self.session.submit_human(action);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.session.target_pick = None;
                    }
                });
            }
        }
    }

    /// Ask the background worker for a fresh move review of the reviewing
    /// seat's current decision. A refusal to start (one is already running) is
    /// reported as an error state, never as a silent no-op.
    fn request_move_review(&mut self) {
        if self.session.request_move_review() {
            self.session.message =
                "move review requested: the ranked alternatives are computed on a background worker and appear in the MOVE REVIEW panel when they are ready".to_string();
        } else {
            self.session.message =
                "refused: a move review is already running; wait for it to finish or change the table".to_string();
        }
    }

    /// One labelled block of the move-review panel. The title and the source
    /// label are per-section, so a policy ordering can never be read as a rule.
    fn review_block(
        ui: &mut egui::Ui,
        section: &crate::review::ReviewSection,
        color: egui::Color32,
    ) {
        theme::heading(ui, section.title);
        ui.label(
            RichText::new(section.source)
                .small()
                .color(theme::muted_color(ui)),
        );
        egui::ScrollArea::vertical()
            .id_salt(format!("move-review-{}", section.title))
            .max_height(240.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for line in &section.lines {
                    // Wrap instead of clipping: a review line is a sentence, and
                    // §5.6 requires it to stay legible at the enforced minimum
                    // window size and at 70–200% text scale.
                    ui.add(
                        egui::Label::new(RichText::new(line).small().monospace().color(color))
                            .wrap(),
                    );
                }
            });
    }

    /// The move-review surface. Everything expensive already happened on the
    /// worker thread; this only formats its finished report. The panel is
    /// read-only: it carries no action, submits nothing, grants no priority and
    /// unlocks no sandbox or undo control.
    fn move_review_panel(&mut self, ui: &mut egui::Ui, human: PlayerId, view: &PlayerView) {
        // Rules come from the engine trace path, live, and are always current —
        // rule availability does not depend on the analysis (§5.1).
        let explanations = coach::explain(&self.session.game, human, &view.legal);
        let busy = self.session.review_busy;
        let stale = self.session.review_fresh(view);
        let report = self.session.review.clone();
        let rules = match &report {
            Some(report) => report.sections(&explanations).rules,
            None => crate::review::rules_section(&explanations),
        };

        if busy {
            theme::heading(ui, crate::review::POLICY_TITLE);
            theme::muted(
                ui,
                "computing ranked alternatives on the background worker — the frame loop is not blocked and the table is unchanged until you act",
            );
            ui.ctx().request_repaint_after(Duration::from_millis(120));
        } else {
            match (&stale, &report) {
                (Some(code), Some(report)) => {
                    theme::heading(
                        ui,
                        "MOVE REVIEW — the stored review no longer matches the table",
                    );
                    ui.label(
                        RichText::new(format!(
                            "[{code}] that review was computed for engine decision {} of revision {} (seat P{}); the table has moved on, so none of its ranking is shown",
                            report.decision(),
                            report.revision(),
                            report.seat()
                        ))
                        .color(theme::warn(ui)),
                    );
                }
                (None, Some(report)) => {
                    Self::review_block(ui, report.policy_section(), theme::text_color(ui));
                }
                (_, None) => {
                    theme::heading(ui, crate::review::POLICY_TITLE);
                    theme::muted(
                        ui,
                        "no review has been computed for this decision yet. \"Move review\" (V) asks the background worker to rank the actions the engine is offering you right now; nothing is ranked until it answers.",
                    );
                }
            }
        }
        ui.separator();
        Self::review_block(ui, &rules, theme::brass(ui));
        if ui
            .add_enabled(
                !busy,
                egui::Button::new("Move review (V) — rank this decision"),
            )
            .clicked()
        {
            self.request_move_review();
        }
    }

    /// The study panel: branch from a recorded line (a session save or a bare
    /// replay file) at an explicitly chosen action boundary, and show the
    /// public comparison of accepted actions between the base line and the
    /// live branch.
    fn study_panel(&mut self, ui: &mut egui::Ui) {
        theme::heading(ui, "STUDY — branch from a recorded line");
        theme::muted(
            ui,
            "Accepts a session save (its embedded replay) or a bare replay file; the branch keeps the same seed and decks, replays the recorded prefix deterministically, then diverges at the chosen action index. Nothing after that point on the base line is shown to the live table.",
        );
        egui::Grid::new("study-inputs")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("study file");
                ui.text_edit_singleline(&mut self.session.branch_path);
                ui.label("branch after action #");
                ui.add(egui::DragValue::new(&mut self.session.branch_at).range(0..=10_000_000));
            });
        let path = PathBuf::from(self.session.branch_path.clone());
        let keep = self.session.branch_at;
        if ui
            .add_enabled(
                !self.session.io_busy,
                egui::Button::new("Branch from replay"),
            )
            .clicked()
        {
            self.ask_branch(path, keep);
        }
        theme::muted(
            ui,
            "G asks the same question using the study file and action index above.",
        );
        if self.session.branch_source.is_some() {
            match self.session.branch_comparison() {
                Some(diff) => {
                    let text = diff.render();
                    egui::ScrollArea::vertical()
                        .id_salt("branch-compare")
                        .max_height(160.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new(&text).monospace());
                        });
                    if self.session.branches_agree() {
                        theme::muted(
                            ui,
                            "no accepted action differs yet: the branch has still played only base-line moves (the base tail after the branch point is never revealed).",
                        );
                    } else {
                        theme::muted(
                            ui,
                            "the comparison stops at the branch's current recorded history — the base continuation past it is shown as an unrevealed note, never as action kinds.",
                        );
                    }
                }
                None => {
                    theme::muted(
                        ui,
                        "the live branch and its base line cannot be compared (they are not from the same setup).",
                    );
                }
            }
        }
    }

    fn render_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let perf_started = self
            .perf
            .as_mut()
            .and_then(|probe| probe.callback_started(Instant::now()));
        let had_input = ctx.input(|input| !input.events.is_empty());
        self.frame_count += 1;
        // eframe applies its own style when the window is created, so the
        // palette is (re)asserted over the first frames and whenever the
        // accessibility panel changes contrast or motion.
        self.apply_theme_if_needed(&ctx);
        // Finished background work is applied here, on the main thread only.
        let applied = self.session.poll_workers();
        if !applied.is_empty() {
            self.session.busy_note = Some(applied.join("; "));
        }
        // Workers keep running after a game ends (import, health, save, load),
        // so keep the frame loop ticking while any is outstanding.
        if self.session.worker_busy() {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
        self.tick_ai(&ctx);
        self.keyboard_shortcuts(&ctx);
        // Follow table changes that came from outside the setup controls
        // (load, imported match) without overwriting a selection the user is
        // about to start.
        let live_seats = self.session.seats.len().min(4) as u8;
        let live_profile = self.session.profile;
        if self.observed_seats != live_seats {
            self.setup_seats = live_seats;
            self.observed_seats = live_seats;
        }
        if self.observed_profile != live_profile {
            self.setup_profile = live_profile;
            self.observed_profile = live_profile;
        }

        let human = self.session.human_seat();
        let view = self.session.view(human);
        let owner = self.session.owner();

        egui::Panel::top("primary-strip").show(ui, |ui| {
            let viewport_height = ui.available_height();
            let header_cap = (viewport_height * 0.38).max(72.0);
            let action_cap = (viewport_height * 0.30).clamp(96.0, 210.0);
            // Fixed row: title, zoom, accessibility and help are never inside
            // the scrollable header, so a 200% zoom or a high-DPI window can
            // never hide them.
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("THE APERTURE").heading().color(theme::brass(ui)));
                theme::muted(ui, crate::session::profile_line());
                ui.separator();
                if ui
                    .small_button("A−")
                    .on_hover_text("Text zoom out (also the − key)")
                    .clicked()
                {
                    let zoom = self.zoom - 0.1;
                    self.set_zoom(&ctx, zoom);
                }
                ui.label(format!("text {:.0}%", self.zoom * 100.0));
                if ui
                    .small_button("A+")
                    .on_hover_text("Text zoom in (also the + key), up to 200%")
                    .clicked()
                {
                    let zoom = self.zoom + 0.1;
                    self.set_zoom(&ctx, zoom);
                }
                if ui
                    .small_button("100%")
                    .on_hover_text("Reset text zoom to 100% (Ctrl+0)")
                    .clicked()
                {
                    self.set_zoom(&ctx, 1.0);
                }
                if ui
                    .selectable_label(self.show_accessibility, "Accessibility (A)")
                    .on_hover_text(
                        "High-contrast and reduced-motion settings. A opens/closes this panel; Esc closes it first.",
                    )
                    .clicked()
                {
                    self.show_accessibility = !self.show_accessibility;
                }
                if ui.small_button("Help (H)").clicked() {
                    self.show_help = !self.show_help;
                }
                if let Some(probe) = &self.perf {
                    let status = probe.status().unwrap_or_else(|| {
                        format!(
                            "collecting {:.1}s of 10s",
                            probe.elapsed().as_secs_f32()
                        )
                    });
                    ui.label(
                        RichText::new(format!("Performance report: {status}"))
                            .small()
                            .color(theme::muted_color(ui)),
                    );
                }
            });
            // A pending destructive confirmation stays fixed too: it can never
            // scroll out of sight at high zoom.
            self.confirm_bar(ui);
            egui::ScrollArea::vertical()
                .id_salt("primary-header")
                .max_height(header_cap)
                .auto_shrink([false, true])
                .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("TABLE:").color(theme::brass(ui)).small());
                for count in 2..=4u8 {
                    if ui
                        .selectable_label(self.setup_seats == count, format!("{count} seats"))
                        .clicked()
                    {
                        self.setup_seats = count;
                    }
                }
                ui.separator();
                ui.label(RichText::new("PROFILE:").color(theme::brass(ui)).small());
                if ui
                    .selectable_label(
                        self.setup_profile == Profile::Practice20,
                        "ordinary 20 life",
                    )
                    .clicked()
                {
                    self.setup_profile = Profile::Practice20;
                }
                if ui
                    .selectable_label(
                        self.setup_profile == Profile::CommanderCombat40,
                        "bounded 40 life · Isamaru each seat",
                    )
                    .clicked()
                {
                    self.setup_profile = Profile::CommanderCombat40;
                }
                if ui.button("Start new table with these options").clicked() {
                    let seed = self.session.seed.wrapping_add(1);
                    self.confirm = Some(PendingAction::NewTable {
                        seed,
                        seats: self.setup_seats as usize,
                        profile: self.setup_profile,
                    });
                }
                theme::muted(
                    ui,
                    "bounded 40 life gives every seat the command-zone library so Isamaru is castable — practice, not Commander",
                );
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("New game").clicked() {
                    let seed = self.session.seed.wrapping_add(1);
                    self.confirm = Some(PendingAction::NewGame { seed });
                }
                if ui.button("Pass (P)").clicked() {
                    let _ = self.session.submit_human(Action::Pass);
                }
                if ui.button("AI step").clicked() {
                    self.session.request_ai_decision_default();
                }
                ui.checkbox(&mut self.session.ai.autoplay, "AI autoplay");
                if ui
                    .checkbox(
                        &mut self.session.human_plays_ai_seat,
                        "Let the AI play my seat too",
                    )
                    .changed()
                {
                    self.session.invalidate_pending_work();
                }
                let decider_toggle = ui.checkbox(
                    &mut self.session.ai.use_decider,
                    "Use local Decider-2B",
                );
                if decider_toggle.changed() {
                    let on = self.session.ai.use_decider;
                    self.session.set_use_decider(on);
                }
                if ui.button("Save (S)").clicked() {
                    self.session
                        .request_save(PathBuf::from(self.session.save_path.clone()));
                }
                if ui.button("Load (L)").clicked() {
                    self.confirm = Some(PendingAction::Load {
                        path: PathBuf::from(self.session.save_path.clone()),
                    });
                }
                if ui.button("Set branch").clicked() {
                    self.session.set_branch();
                }
                if ui.button("Return to branch").clicked()
                    && let Err(err) = self.session.return_to_branch()
                {
                    self.session.message = err;
                }
                if ui
                    .add_enabled(
                        self.session.can_undo(),
                        egui::Button::new("Undo (U)"),
                    )
                    .on_hover_text(
                        "Undo the last accepted action by rebuilding the recorded prefix exactly",
                    )
                    .clicked()
                    && let Err(err) = self.session.undo_last_action()
                {
                    self.session.message = err;
                }
                if ui.button("Study branches (B)").clicked() {
                    self.show_study = !self.show_study;
                }
                if ui
                    .selectable_label(self.show_review, "Move review (V)")
                    .on_hover_text(
                        "Rank the actions the engine is offering you right now, with the analysis's own counts, its compute budget and its stated uncertainty. Runs on a background worker; read-only.",
                    )
                    .clicked()
                {
                    self.show_review = !self.show_review;
                    if self.show_review {
                        self.request_move_review();
                    }
                }
                let review_label = if self.session.review_mode {
                    "Leave review (R)"
                } else {
                    "Postgame review (R)"
                };
                if ui
                    .add_enabled(
                        self.session.review_mode || self.session.conclude().is_some(),
                        egui::Button::new(review_label),
                    )
                    .clicked()
                {
                    if self.session.review_mode {
                        self.session.leave_review();
                    } else if let Err(err) = self.session.enter_review() {
                        self.session.message = err;
                    }
                }
                ui.checkbox(&mut self.show_coach, "Coaching (C)");
                ui.checkbox(&mut self.show_sandbox, "Sandbox (K)");
                ui.checkbox(&mut self.show_import, "Import (I)");
                ui.checkbox(&mut self.show_zones, "Zones (Z)");
            });
            if self.show_help {
                ui.label(
                    RichText::new(
                        "Keyboard: Tab / Shift+Tab move focus · Enter activates the focused control (a 2 px outline marks focus) · Ctrl+0 resets text zoom to 100% · +/− text zoom (up to 200%) · A accessibility panel (high contrast, reduced motion) · P pass · U undo · N new game (keeps seats/profile, asks first) · S save · L load (asks first) · I import · B study-branch panel · G branch from the prefilled study inputs (asks first) · R postgame review (game over only) · V move review (ranks the engine's offers for your current decision on a background worker; read-only) · Z zone inspector · K sandbox · C coaching · H help · Esc closes the accessibility panel first, then cancels a target pick, a confirmation or the inspector; it never performs a destructive action. Single-letter shortcuts are ignored while a text field has focus.",
                    )
                    .small()
                    .color(theme::muted_color(ui)),
                );
            }
            }); // primary-header scroll
            ui.horizontal_wrapped(|ui| {
                if let Some((label, _)) = &self.session.branch {
                    ui.label(
                        RichText::new(format!("branch: {label}"))
                            .color(theme::brass(ui))
                            .small(),
                    );
                }
                if let Some(pending) = &view.pending {
                    ui.label(RichText::new(format!("WAITING: {pending}")).color(theme::warn(ui)));
                } else if let Some(owner) = owner {
                    if owner == human || view.priority == Some(owner) {
                        ui.label(format!("P{owner} holds priority"));
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "Waiting on P{owner} (no player has priority)"
                            ))
                            .color(theme::warn(ui)),
                        );
                    }
                } else if let Some(conclusion) = self.session.conclude() {
                    ui.label(RichText::new(conclusion).color(theme::good(ui)).strong());
                }
            });
            ui.horizontal_wrapped(|ui| {
                let lives = (0..view.seat_count as PlayerId)
                    .map(|seat| {
                        let life = if seat == human {
                            view.own_life
                        } else {
                            view.opponent_view(seat).map(|o| o.life).unwrap_or(0)
                        };
                        let out = if seat == human {
                            view.own_eliminated
                        } else {
                            view.opponent_view(seat)
                                .map(|o| o.eliminated)
                                .unwrap_or(true)
                        };
                        format!("P{seat} {life}{}", if out { " (out)" } else { "" })
                    })
                    .collect::<Vec<_>>()
                    .join(" / ");
                ui.label(lives);
                ui.label(format!(
                    "· {} seats · profile {} ({} life) · turn {} · {} · active P{} · stack {} · digest {}",
                    self.session.seats.len(),
                    self.session.profile.label(),
                    self.session.profile.starting_life(),
                    view.turn_number,
                    view.step.name(),
                    view.active,
                    view.stack.len(),
                    view.digest
                ));
                if let Some(pending_owner) = view.pending_owner
                    && view.pending.is_some()
                {
                    ui.label(
                        RichText::new(format!("pending owner P{pending_owner}"))
                            .color(theme::warn(ui))
                            .small(),
                    );
                }
                if self.session.tainted() {
                    ui.label(
                        RichText::new(format!(
                            "SANDBOX-MODIFIED — NOT A LEGAL MATCH RESULT ({})",
                            self.session.sandbox_record.summary()
                        ))
                        .color(theme::bad(ui))
                        .strong(),
                    );
                }
                if self.session.ai.use_decider {
                    ui.label(RichText::new("decider-2b opt-in").color(theme::brass(ui)));
                }
                if self.session.worker_busy() {
                    self.working_indicator(ui);
                    ui.label(RichText::new("background work running").color(theme::brass(ui)));
                }
            });
            if let Some(note) = &self.session.busy_note {
                ui.label(RichText::new(note).color(theme::muted_color(ui)).small());
            }
            ui.separator();
            // The persistent primary strip: own pending choices and legal
            // actions never scroll away with the table. In postgame review
            // mode it becomes the omniscient review readout and offers NO
            // action, policy or sandbox control — review is a read-only view.
            if self.session.review_mode {
                theme::heading(
                    ui,
                    "REVIEW — omniscient postgame view · game over · acting is disabled",
                );
                egui::ScrollArea::vertical()
                    .id_salt("review-strip")
                    .max_height(action_cap)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if let Some(review) = &self.session.review_cache {
                            ui.label(RichText::new(review).monospace());
                        }
                    });
                if ui.button("Leave review (R)").clicked() {
                    self.session.leave_review();
                }
            } else {
                theme::heading(ui, "YOUR ACTIONS — only engine-offered decisions are accepted");
                egui::ScrollArea::vertical()
                    .id_salt("primary-actions")
                    .max_height(action_cap)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.primary_actions(ui, &view, owner, human);
                    });
                if let Some(review_ready) = self.session.conclude() {
                    theme::muted(
                        ui,
                        format!("game over — {review_ready}. Postgame review (R) is enabled."),
                    );
                }
            }
            if let Some(note) = &self.session.note {
                ui.label(RichText::new(note).color(theme::brass(ui)).small());
            }
            if !self.session.message.is_empty() {
                // A refusal or a failed load/branch is an error state, not a
                // hint: it is shown in the error colour so a silent no-op can
                // never be mistaken for success.
                let failed = message_is_error(&self.session.message);
                ui.label(
                    RichText::new(&self.session.message)
                        .color(if failed { theme::bad(ui) } else { theme::warn(ui) })
                        .small(),
                );
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            // Scroll the real application content. The primary strip above is
            // never inside this scroll area, so choices stay reachable.
            egui::ScrollArea::vertical()
                .id_salt("tabletop-content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(refusal) = self.session.deck_refusal.clone() {
                        ui.label(
                            RichText::new(format!("DECK REFUSED — {refusal}"))
                                .color(theme::bad(ui))
                                .strong(),
                        );
                        theme::muted(
                            ui,
                            "Use the Import panel to supply a valid list for each seat, then start the imported match.",
                        );
                    }

                    // Compact 2x2 seat panels: every battlefield, status block
                    // and public zone control is visible without a huge scroll.
                    let seats: Vec<PlayerId> = (0..view.seat_count as PlayerId)
                        .filter(|seat| *seat == human)
                        .chain(
                            (0..view.seat_count as PlayerId)
                                .filter(|seat| *seat != human)
                                .take(3),
                        )
                        .take(4)
                        .collect();
                    let gap = 10.0;
                    let cell_width = ((ui.available_width() - gap) / 2.0).max(320.0);
                    for row in seats.chunks(2) {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for seat in row {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(cell_width, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.set_width(cell_width);
                                        let data = SeatPanelData::from_view(
                                            &view,
                                            &self.session.seats,
                                            *seat,
                                        );
                                        self.seat_panel(ui, &data, &view, *seat == human);
                                    },
                                );
                            }
                        });
                    }

                    if self.show_review {
                        ui.separator();
                        self.move_review_panel(ui, human, &view);
                    }

                    ui.separator();
                    theme::heading(ui, "STACK");
                    if view.stack.is_empty() {
                        theme::muted(ui, "(stack empty)");
                    }
                    for (index, item) in view.stack.iter().rev().enumerate() {
                        let targets = if item.targets.is_empty() {
                            String::new()
                        } else {
                            format!(" → {}", item.targets.join(", "))
                        };
                        ui.label(format!(
                            "{}. {}{}  (P{})",
                            index + 1,
                            item.name,
                            targets,
                            item.controller
                        ));
                    }

                    if self.show_coach {
                        ui.separator();
                        theme::heading(ui, "COACHING (derived from the engine trace)");
                        let explanations =
                            coach::explain(&self.session.game, human, &view.legal);
                        egui::ScrollArea::vertical()
                            .id_salt("coach")
                            .max_height(220.0)
                            .show(ui, |ui| {
                                for explanation in explanations {
                                    let badge = match explanation.kind {
                                        coach::ExplanationKind::Rule => "[RULE]",
                                        coach::ExplanationKind::Simplification => {
                                            "[SIMPLIFICATION]"
                                        }
                                        coach::ExplanationKind::Strategy => "[STRATEGY]",
                                    };
                                    let color = match explanation.kind {
                                        coach::ExplanationKind::Rule
                                            if explanation.topic.contains("unavailable") =>
                                        {
                                            theme::muted_color(ui)
                                        }
                                        coach::ExplanationKind::Rule => theme::text_color(ui),
                                        coach::ExplanationKind::Simplification => theme::warn(ui),
                                        coach::ExplanationKind::Strategy => theme::brass(ui),
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "{badge} [{}] {}",
                                            explanation.topic, explanation.text
                                        ))
                                        .color(color),
                                    );
                                    for rule in &explanation.rules {
                                        if let Some(note) = coach::rule_note(rule) {
                                            ui.label(
                                                RichText::new(format!("    CR {rule}: {note}"))
                                                    .small()
                                                    .color(theme::muted_color(ui)),
                                            );
                                        }
                                    }
                                }
                            });
                    }

                    ui.separator();
                    theme::heading(ui, "LOG (viewer-masked)");
                    egui::ScrollArea::vertical()
                        .id_salt("log")
                        .max_height(190.0)
                        .show(ui, |ui| {
                            for line in view.log.iter().rev().take(80) {
                                ui.label(RichText::new(line).small());
                            }
                        });

                    if self.show_sandbox {
                        ui.separator();
                        theme::heading(ui, crate::session::sandbox_label());
                        ui.label(
                            RichText::new(
                                "LEGAL MATCH ACTIONS stay in YOUR ACTIONS at the top of the window. Every control below is a manual table override: it bypasses the rules, it is written into this session's taint record, and the table can then never be presented as a legal match result.",
                            )
                            .small()
                            .color(theme::warn(ui)),
                        );
                        if self.session.tainted() {
                            ui.label(
                                RichText::new(format!(
                                    "TAINTED — {}",
                                    self.session.sandbox_record.summary()
                                ))
                                .color(theme::bad(ui))
                                .strong(),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!("on your seat (P{human}):"))
                                    .color(theme::brass(ui))
                                    .small(),
                            );
                            for (label, op) in [
                                ("Draw 1", SandboxOp::Draw(1)),
                                ("Mill 1", SandboxOp::Mill(1)),
                                ("Untap all", SandboxOp::Untap),
                                ("Scry 1 · keep on top", SandboxOp::Scry(1)),
                                ("Scry 1 · to bottom", SandboxOp::ScryBottom(1)),
                                ("Surveil 1 · keep on top", SandboxOp::Surveil(1)),
                                ("Surveil 1 · to graveyard", SandboxOp::SurveilGraveyard(1)),
                                ("Reveal top 3", SandboxOp::RevealTop(3)),
                            ] {
                                if ui.button(format!("{label} · sandbox")).clicked() {
                                    let _ = self.session.sandbox(op);
                                }
                            }
                        });
                        theme::muted(
                            ui,
                            "A scry or surveil look is shown to your seat only; a reveal is public by construction. A control is refused while a rules choice is pending.",
                        );
                        theme::heading(ui, "Counter editor — your permanents");
                        let own = view.own_battlefield.clone();
                        if own.is_empty() {
                            theme::muted(ui, "(no permanents on your battlefield)");
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                for card in &own {
                                    let selected = self.counter_object == Some(card.id);
                                    if ui
                                        .selectable_label(
                                            selected,
                                            format!(
                                                "{} [{} +1/+1]",
                                                card.name, card.plus_one_counters
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.counter_object = Some(card.id);
                                    }
                                }
                            });
                            if let Some(card) = own
                                .iter()
                                .find(|card| Some(card.id) == self.counter_object)
                                .cloned()
                            {
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .add_enabled(
                                            card.plus_one_counters > 0,
                                            egui::Button::new("−1 · sandbox"),
                                        )
                                        .clicked()
                                    {
                                        let _ =
                                            self.session.sandbox(SandboxOp::Counter(card.id, -1));
                                    }
                                    if ui
                                        .add_enabled(
                                            card.plus_one_counters < 99,
                                            egui::Button::new("+1 · sandbox"),
                                        )
                                        .clicked()
                                    {
                                        let _ =
                                            self.session.sandbox(SandboxOp::Counter(card.id, 1));
                                    }
                                    if ui.button("Clear · sandbox").clicked() {
                                        let _ = self
                                            .session
                                            .sandbox(SandboxOp::SetCounters(card.id, 0));
                                    }
                                    ui.label(format!(
                                        "{}: {} +1/+1 counter(s)",
                                        card.name, card.plus_one_counters
                                    ));
                                });
                                theme::muted(
                                    ui,
                                    "Counter edits are stored and displayed only: this engine's combat math does not apply them.",
                                );
                            }
                        }
                        theme::heading(ui, "Life editor");
                        ui.horizontal_wrapped(|ui| {
                            for seat in 0..view.seat_count as PlayerId {
                                let life = if seat == human {
                                    view.own_life
                                } else {
                                    view.opponent_view(seat).map(|other| other.life).unwrap_or(0)
                                };
                                if ui
                                    .selectable_label(self.life_seat == seat, format!("P{seat} {life}"))
                                    .clicked()
                                {
                                    self.life_seat = seat;
                                    self.life_value = life;
                                }
                            }
                            ui.add(egui::DragValue::new(&mut self.life_value).range(1..=999));
                            if ui.button("Set life · sandbox").clicked() {
                                let _ = self.session.sandbox(SandboxOp::SetLife(
                                    self.life_seat,
                                    self.life_value,
                                ));
                            }
                        });
                        theme::muted(
                            ui,
                            "A sandbox life edit stays between 1 and 999: ending the game is a legal-match action, not an override.",
                        );
                        if !self.session.revealed.is_empty() {
                            ui.label(format!(
                                "revealed (sandbox): {}",
                                self.session.revealed.join(", ")
                            ));
                        }
                        if ui.button("Reset this game · sandbox").clicked() {
                            let seed = self.session.seed;
                            self.confirm = Some(PendingAction::ResetSandboxGame { seed });
                        }
                    }

                    if self.show_import {
                        ui.separator();
                        self.import_panel(ui);
                    }

                    if self.show_study {
                        ui.separator();
                        self.study_panel(ui);
                    }

                    if self.frame_count == 1 {
                        ui.label(
                            RichText::new(format!(
                                "rules revision {} · pool {} cards · deterministic replay + digest · {}",
                                aperture_core::RULES_REVISION,
                                catalog::pool_size(),
                                if self.session.ai.use_decider {
                                    "model opt-in ON"
                                } else {
                                    "model off (heuristic)"
                                }
                            ))
                            .small()
                            .color(theme::muted_color(ui)),
                        );
                    }
                });
        });

        // ------------------------------ floating windows
        if let Some(text) = self.session.inspect.clone() {
            let mut open = true;
            egui::Window::new("Inspect")
                .open(&mut open)
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.label(RichText::new(text).monospace());
                    theme::muted(
                        ui,
                        "Inspect is read-only. Use the table controls for decisions.",
                    );
                });
            if !open {
                self.session.inspect = None;
            }
        }

        if self.show_zones {
            let mut open = true;
            let mut inspect_card: Option<CardView> = None;
            egui::Window::new("Zone inspector — searchable, viewer-masked")
                .open(&mut open)
                .resizable(true)
                .default_width(660.0)
                .default_height(460.0)
                .show(&ctx, |ui| {
                    theme::muted(
                        ui,
                        "One inspector for every zone this seat may legally see: your own hand plus every public zone of every seat. Opponent hands and every library stay counts only, and nothing here queries the engine.",
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label("search");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zone_query)
                                .hint_text("name, rules text or mana cost")
                                .desired_width(300.0),
                        );
                        if ui.button("Clear").clicked() {
                            self.zone_query.clear();
                            self.zone_seat_filter = None;
                            self.zone_zone_filter = None;
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("seat:").color(theme::brass(ui)).small());
                        if ui
                            .selectable_label(self.zone_seat_filter.is_none(), "all")
                            .clicked()
                        {
                            self.zone_seat_filter = None;
                        }
                        for seat in 0..view.seat_count as PlayerId {
                            if ui
                                .selectable_label(
                                    self.zone_seat_filter == Some(seat),
                                    format!("P{seat}"),
                                )
                                .clicked()
                            {
                                self.zone_seat_filter = Some(seat);
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("zone:").color(theme::brass(ui)).small());
                        if ui
                            .selectable_label(self.zone_zone_filter.is_none(), "all")
                            .clicked()
                        {
                            self.zone_zone_filter = None;
                        }
                        for zone in ZONE_ORDER {
                            if ui
                                .selectable_label(
                                    self.zone_zone_filter == Some(zone),
                                    zone_label(zone),
                                )
                                .clicked()
                            {
                                self.zone_zone_filter = Some(zone);
                            }
                        }
                    });
                    for seat in 0..view.seat_count as PlayerId {
                        let data = SeatPanelData::from_view(&view, &self.session.seats, seat);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "P{seat} '{}' · life {} · hand {} · library {}",
                                    data.name, data.life, data.hand_count, data.library_count
                                ))
                                .color(theme::brass(ui))
                                .small(),
                            );
                            for (zone, count) in [
                                (Zone::Hand, data.hand_count),
                                (Zone::Battlefield, data.battlefield.len()),
                                (Zone::Graveyard, data.graveyard.len()),
                                (Zone::Exile, data.exile.len()),
                                (Zone::Command, data.command_zone.len()),
                            ] {
                                if ui
                                    .small_button(format!("{} {count}", zone_label(zone)))
                                    .clicked()
                                {
                                    self.zone_seat_filter = Some(seat);
                                    self.zone_zone_filter = Some(zone);
                                }
                            }
                        });
                    }
                    let seats: Vec<PlayerId> = self.zone_seat_filter.into_iter().collect();
                    let zones: Vec<Zone> = self.zone_zone_filter.into_iter().collect();
                    let hits = view.search(&self.zone_query, &seats, &zones);
                    ui.separator();
                    ui.label(RichText::new(format!("{} card(s) match", hits.len())).strong());
                    if hits.is_empty() {
                        theme::muted(
                            ui,
                            "Nothing matches. A hidden hand or a library never matches: its contents are not projected to this seat.",
                        );
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("zone-hits")
                        .max_height(240.0)
                        .show(ui, |ui| {
                            for hit in &hits {
                                let label = format!(
                                    "P{} · {} · {}",
                                    hit.seat,
                                    zone_label(hit.zone),
                                    Self::card_label(&hit.card)
                                );
                                if ui.small_button(label).clicked() {
                                    inspect_card = Some(hit.card.clone());
                                }
                            }
                        });
                });
            if let Some(card) = inspect_card {
                self.inspect_card(&card);
            }
            if !open {
                self.show_zones = false;
            }
        }

        // ------------------------------ accessibility shell
        self.accessibility_panel(&ctx);

        // ------------------------------ opt-in performance probe
        let callback_end = Instant::now();
        if let Some(started) = perf_started
            && let Some(probe) = self.perf.as_mut()
            && probe.callback_finished(started, callback_end, had_input)
        {
            // Serialization and the filesystem write happen on a background
            // thread after this frame; no IO runs here.
            probe.finish_and_write();
        }
        if let Some(probe) = &self.perf {
            if probe.collecting() {
                // Forced ~60 Hz sampling only while the bounded window runs.
                ctx.request_repaint_after(Duration::from_millis(16));
            } else if !probe.write_done() {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }

        // ------------------------------ smoke mode
        if let Some(path) = self.startup.smoke.clone() {
            let elapsed = self.smoke_started.elapsed();
            if !self.smoke_requested && elapsed > Duration::from_millis(700) {
                self.smoke_requested = true;
                let written = self.smoke_written.clone();
                ctx.request_screenshot(move |image| {
                    if let Err(err) = write_png(&path, &image) {
                        eprintln!("smoke screenshot failed: {err}");
                    }
                    written.store(true, Ordering::SeqCst);
                });
            }
            if self.smoke_requested && self.smoke_written.load(Ordering::SeqCst) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }
}

fn write_png(path: &std::path::Path, image: &egui::ColorImage) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut buffer = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        let [r, g, b, a] = pixel.to_array();
        if a == 0 || a == 255 {
            buffer.extend_from_slice(&[r, g, b, a]);
        } else {
            // The framebuffer is premultiplied; store straight alpha.
            let unpremultiply = |channel: u8| ((channel as u32 * 255) / a as u32).min(255) as u8;
            buffer.extend_from_slice(&[unpremultiply(r), unpremultiply(g), unpremultiply(b), a]);
        }
    }
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        image.size[0] as u32,
        image.size[1] as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    writer
        .write_image_data(&buffer)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}

impl eframe::App for ApertureApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}

#[cfg(test)]
mod viewport_tests {
    use super::*;
    use crate::session::{BUNDLED_A, BUNDLED_B};
    use aperture_ai::Policy;
    use aperture_core::{CardId, DeckList, Game, ObjectId, Step, Zone};

    fn startup() -> Startup {
        Startup {
            smoke: None,
            seed: 1,
            deck_a: PathBuf::new(),
            deck_b: PathBuf::new(),
            deck_a_explicit: false,
            deck_b_explicit: false,
            ai_seat: 1,
            human_autoplay: false,
            autoplay: false,
            use_decider: false,
            seats: 2,
            profile: Profile::Practice20,
            window: None,
            study_file: None,
            branch_at: 0,
            performance_report: None,
            performance_conditions: Vec::new(),
            initial_high_contrast: false,
            initial_reduced_motion: false,
        }
    }

    fn imported_app() -> ApertureApp {
        let startup = startup();
        let (a, b, _) = startup_decks(&startup);
        let mut session =
            UiSession::new(1, vec![a, b], AiSettings::default(), false, false).expect("session");
        session.seat_import[0] =
            Some(crate::session::import_seat_text(P0, "P0 imported", BUNDLED_A).unwrap());
        session.seat_import[1] =
            Some(crate::session::import_seat_text(1, "P1 imported", BUNDLED_B).unwrap());
        ApertureApp {
            session,
            startup,
            smoke_started: Instant::now(),
            smoke_requested: false,
            smoke_written: Arc::new(AtomicBool::new(false)),
            last_ai_step: Instant::now(),
            show_sandbox: false,
            show_import: true,
            show_zones: false,
            show_review: false,
            show_coach: true,
            show_study: false,
            show_help: false,
            show_accessibility: false,
            high_contrast: false,
            reduced_motion: false,
            observed_theme: (false, false),
            perf: None,
            setup_seats: 2,
            setup_profile: Profile::Practice20,
            observed_seats: 2,
            observed_profile: Profile::Practice20,
            zone_query: String::new(),
            zone_seat_filter: None,
            zone_zone_filter: None,
            confirm: None,
            counter_object: None,
            life_seat: P0,
            life_value: 20,
            zoom: 1.0,
            frame_count: 0,
        }
    }

    fn four_seat_app() -> ApertureApp {
        let startup = Startup {
            seats: 4,
            ..startup()
        };
        let seats = UiSession::bundled_seats(4).expect("four seats");
        let ai = AiSettings {
            human_seat: P0,
            ai_seats: vec![1, 2, 3],
            ..AiSettings::default()
        };
        let mut session = UiSession::new(1, seats, ai, false, false).expect("session");
        session.profile = Profile::CommanderCombat40;
        session.new_game(1);
        ApertureApp {
            session,
            startup,
            smoke_started: Instant::now(),
            smoke_requested: false,
            smoke_written: Arc::new(AtomicBool::new(false)),
            last_ai_step: Instant::now(),
            show_sandbox: false,
            show_import: false,
            show_zones: false,
            show_review: false,
            show_coach: true,
            show_study: false,
            show_help: false,
            show_accessibility: false,
            high_contrast: false,
            reduced_motion: false,
            observed_theme: (false, false),
            perf: None,
            setup_seats: 4,
            setup_profile: Profile::CommanderCombat40,
            observed_seats: 4,
            observed_profile: Profile::CommanderCombat40,
            zone_query: String::new(),
            zone_seat_filter: None,
            zone_zone_filter: None,
            confirm: None,
            counter_object: None,
            life_seat: P0,
            life_value: 20,
            zoom: 1.0,
            frame_count: 0,
        }
    }

    #[test]
    fn a_capture_launch_can_start_in_high_contrast_and_reduced_motion() {
        // `--a11y` only seeds the panel's two settings; rendering must keep
        // them and install the reduced-motion style (animation_time 0).
        let mut app = four_seat_app();
        app.high_contrast = true;
        app.reduced_motion = true;
        app.observed_theme = (true, true);
        app.session.ai_busy = true;
        let labels = run_frame(&mut app, 1280.0, 720.0, 6);
        assert!(
            labels.iter().any(|label| label == "Working..."),
            "reduced motion must show static Working... while a worker runs: {labels:?}"
        );
        assert!(app.high_contrast && app.reduced_motion);
    }

    #[test]
    fn the_startup_accessibility_flags_seed_the_app_settings() {
        let startup = Startup {
            initial_high_contrast: true,
            initial_reduced_motion: true,
            ..startup()
        };
        assert!(startup.initial_high_contrast && startup.initial_reduced_motion);
    }

    #[test]
    fn explicit_startup_decks_override_the_bounded_default() {
        let path = std::env::temp_dir().join(format!(
            "aperture-explicit-startup-{}.txt",
            std::process::id()
        ));
        std::fs::write(&path, "1 Grizzly Bears\n39 Forest\n").expect("temp deck");
        let mut startup = startup();
        startup.seats = 4;
        startup.profile = Profile::CommanderCombat40;
        startup.deck_a = path.clone();
        startup.deck_a_explicit = true;
        let (deck_a, deck_b, refusal) = startup_decks(&startup);
        assert!(refusal.is_none(), "{refusal:?}");
        assert_eq!(
            deck_a.count_of(CardId::GrizzlyBears),
            1,
            "an explicit --deck-a must be applied exactly"
        );
        assert_eq!(deck_a.size(), 40);
        assert_eq!(
            deck_b.count_of(CardId::Plains),
            12,
            "an unmodified bounded P1 still receives the command-zone default"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn command_zone_inspection_does_not_invent_a_battlefield_location() {
        let mut app = four_seat_app();
        let view = app.session.view(P0);
        let commander = view.own_command_zone.first().expect("public commander");
        app.inspect_card(commander);
        let text = app.session.inspect.as_deref().expect("inspection text");
        assert!(text.contains("Isamaru, Hound of Konda"));
        assert!(
            !text.contains("zone: battlefield"),
            "a command-zone card was mislabeled: {text}"
        );
    }

    fn run_frame(app: &mut ApertureApp, width: f32, height: f32, frame: usize) -> Vec<String> {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            time: Some(frame as f64 / 30.0),
            ..Default::default()
        };
        // Panels settle their remembered sizes on the first frame; assert the
        // second frame so the capture matches a real running window.
        let mut warm = ctx.run_ui(input.clone(), |ui| app.render_ui(ui));
        warm.textures_delta.clear();
        let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
        output.textures_delta.clear();
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    // ------------------------------------------------------- move review (#28)

    /// A synthetic but structurally real review payload, used only by the layout
    /// tests: the renderer must show exactly what a finished analysis contains.
    /// Its labels are taken from the live engine offer when one exists, and its
    /// line is a recorded-style sequence of engine-accepted actions — the panel
    /// only ever renders what the payload carries.
    fn canned_payload(app: &ApertureApp, mean: Option<f32>) -> aperture_ai::review::RankedReview {
        use aperture_ai::review::{
            BudgetRecord, CandidateLine, LineScope, LineStep, Preference, RankedCandidate,
            RankedReview, ReviewBinding, ReviewText, TextKind, Uncertainty, review_config,
        };
        let seat = app.session.human_seat();
        let view = app.session.view(seat);
        let offers: Vec<String> = view.legal.iter().map(|legal| legal.label.clone()).collect();
        let first = offers.first().cloned().unwrap_or("Pass".to_string());
        let second = offers.get(1).cloned().unwrap_or("Pass".to_string());
        let config = review_config();
        let binding = ReviewBinding::for_view(
            &view,
            app.session.generation,
            app.session.game.revision(),
            app.session.game.decision(),
        );
        let budget = BudgetRecord {
            seeds: config.seeds,
            node_budget: config.node_budget,
            max_rollout_nodes: config.max_rollout_nodes,
            max_millis: config.max_millis,
            max_candidates: config.max_candidates,
            decisions_used: 1_234,
            stop: aperture_ai::StopReason::DeterminizationsExhausted,
            cancelled: false,
            reproducible: true,
            fidelity: "opponents: neutral belief deck (1 opponent(s)); no real hand or library order read; own future draws sampled from declared deck".to_string(),
        };
        let line = |label: &str, seed_index: u64| CandidateLine {
            steps: vec![
                LineStep {
                    decision: 0,
                    seat,
                    label: label.to_string(),
                },
                LineStep {
                    decision: 1,
                    seat: (seat + 1) % 4,
                    label: "pass priority".to_string(),
                },
            ],
            truncated: false,
            seed_index,
            outcome: 1,
            scope: LineScope::SearchGame,
        };
        let candidate = |offer_index: usize,
                         label: String,
                         chosen: bool,
                         score: Option<f32>,
                         line: Option<CandidateLine>|
         -> RankedCandidate {
            RankedCandidate {
                offer_index,
                move_label: label,
                offer_rules: Vec::new(),
                score,
                scored_samples: 2,
                without_result: 0,
                refused_times: 0,
                rollouts_attempted: 2,
                line,
                reasons: if chosen {
                    vec![aperture_ai::review::ReasonTag::HighestPracticeOutcome]
                } else {
                    Vec::new()
                },
                uncertainty: Uncertainty::Ordered,
                chosen,
            }
        };
        let mut notes = vec![
            ReviewText {
                kind: TextKind::Provenance,
                body: budget.rendered(),
                evidence: vec![aperture_ai::review::EvidenceRef::Budget {
                    field: "decisions_used".into(),
                }],
            },
            ReviewText {
                kind: TextKind::Provenance,
                body: format!("determinization fidelity (verbatim): {}", budget.fidelity),
                evidence: vec![aperture_ai::review::EvidenceRef::Budget {
                    field: "fidelity".into(),
                }],
            },
        ];
        notes.push(ReviewText {
            kind: TextKind::Policy,
            body: format!("the search's own choice is offer 0 (\"{first}\")"),
            evidence: vec![aperture_ai::review::EvidenceRef::EngineOffer { index: 0 }],
        });
        RankedReview {
            spec: "move-review/1.2".into(),
            binding,
            budget,
            candidates: vec![
                candidate(0, first, true, mean, Some(line("engine offer", 0))),
                candidate(1, second, false, mean.map(|value| -value), None),
            ],
            preference: mean.map(|_| Preference::Unique { offer_index: 0 }),
            refusal: None,
            notes,
        }
    }

    fn canned_review(app: &ApertureApp, mean: Option<f32>) -> crate::review::MoveReview {
        crate::review::MoveReview::from_ranked(canned_payload(app, mean))
    }

    /// The review panel must be reachable, fully inside a clip rect and inside
    /// the window at the size eframe actually enforces (1024x640), not merely
    /// present somewhere in the widget tree.
    #[test]
    fn the_move_review_panel_is_readable_at_the_minimum_1024x640() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.show_coach = false;
        app.show_review = true;
        app.session.review = Some(canned_review(&app, Some(0.5)));
        for needle in [
            "MOVE REVIEW — ranked practice outcomes",
            "source: aperture_ai::review over engine-offered actions",
            "ordering: highest mean practice outcome first",
            "policy preference",
            "RULES FOR THESE OFFERED ACTIONS",
            "source: engine rule/trace explanations",
        ] {
            assert!(
                scroll_until_visible(&mut app, needle, 1024.0, 640.0),
                "{needle:?} is not fully visible inside a clip rect at the enforced minimum 1024x640"
            );
        }
        // The candidate row itself (label + mean + sample count) must be
        // readable at that size, not just the headings.
        let review = app.session.review.clone().expect("the canned review");
        let row = review
            .policy_section()
            .lines
            .iter()
            .find(|line| line.contains("mean +0.50"))
            .cloned()
            .expect("a scored candidate row");
        assert!(
            scroll_until_visible(&mut app, &row, 1024.0, 640.0),
            "the scored candidate row is not readable at 1024x640: {row}"
        );
    }

    /// A refusal and a stale panel each show their own reason and never a
    /// ranking, at the enforced minimum size.
    #[test]
    fn the_move_review_refusal_and_stale_states_show_no_ranking_at_the_minimum() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.show_coach = false;
        app.show_review = true;
        let seat = app.session.human_seat();
        let reason =
            "non-priority decision (pending kind mulligan); search not enabled there".to_string();
        let view = app.session.view(seat);
        app.session.review = Some(crate::review::refused(
            &view,
            &aperture_ai::review::review_config(),
            aperture_ai::review::ReviewBinding::for_view(
                &view,
                app.session.generation,
                app.session.game.revision(),
                app.session.game.decision(),
            ),
            reason.clone(),
        ));
        assert!(
            scroll_until_visible(&mut app, "no alternative is ranked", 1024.0, 640.0),
            "a refusal must say plainly that nothing is ranked"
        );

        let mut payload = canned_payload(&app, Some(0.5));
        // Same seat/revision/decision, superseded decision serial: the binding
        // must reject it and the panel must say so instead of showing a ranking.
        payload.binding.decision = payload.binding.decision.wrapping_add(9);
        app.session.review = Some(crate::review::MoveReview::from_ranked(payload));
        assert!(
            scroll_until_visible(
                &mut app,
                "the stored review no longer matches the table",
                1024.0,
                640.0
            ),
            "a stale review must say so instead of showing its ranking"
        );
        assert!(
            scroll_until_visible(&mut app, "[stale_decision]", 1024.0, 640.0),
            "a stale review must carry its stale code"
        );
    }

    /// V opens the panel and asks the worker; it never acts on the table.
    #[test]
    fn the_v_shortcut_opens_the_move_review_without_acting() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let digest = app.session.game.digest();
        let revision = app.session.game.revision();
        assert!(!app.show_review, "the panel starts closed");
        press_key(&mut app, 1024.0, 640.0, egui::Key::V);
        assert!(app.show_review, "V must open the move review panel");
        assert!(
            app.session.review_busy || app.session.review.is_some(),
            "V must request a review (or receive an immediate typed refusal)"
        );
        assert_eq!(
            app.session.game.digest(),
            digest,
            "opening the review must not change the table"
        );
        assert_eq!(
            app.session.game.revision(),
            revision,
            "opening the review must not move the engine revision"
        );
    }

    /// The one end-to-end journey: real click on the panel control, the real
    /// background search, and the ranking read back out of the rendered
    /// interface. Driven headlessly through the same widget tree the window
    /// draws (egui frame loop, no GPU surface), so this is interaction
    /// evidence, not a screenshot.
    #[test]
    fn the_move_review_journey_reads_its_ranking_back_from_the_interface() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let human = app.session.human_seat();
        let mut guard = 0;
        while app.session.owner() != Some(human) {
            assert!(
                app.session.step_ai().is_some(),
                "the table stopped before the human seat held priority"
            );
            guard += 1;
            assert!(
                guard < 400,
                "the journey never reached a human priority decision"
            );
        }
        // Walk the legal trace forward until the reviewing seat faces a real
        // choice (several engine offers, no pending sub-decision), so the
        // journey reviews a decision with something to rank. Every step is an
        // engine-offered action; nothing here invents a move.
        loop {
            let view = app.session.view(human);
            if view.pending_kind.is_none() && view.legal.len() >= 3 {
                break;
            }
            while app.session.owner() != Some(human) {
                assert!(
                    app.session.step_ai().is_some(),
                    "the table stopped before the human seat held priority"
                );
                guard += 1;
                assert!(guard < 800, "the journey never reached a real choice");
            }
            let view = app.session.view(human);
            if view.pending_kind.is_none() && view.legal.len() >= 3 {
                break;
            }
            let pass = view
                .legal
                .iter()
                .find(|legal| legal.action == Action::Pass)
                .map(|legal| legal.action.clone())
                .or_else(|| view.legal.first().map(|legal| legal.action.clone()));
            let Some(action) = pass else { break };
            app.session
                .submit_human(action)
                .expect("the engine-offered step of the walk must be legal");
            guard += 1;
            assert!(guard < 800, "the journey never reached a real choice");
        }
        println!(
            "journey: reached the reviewing seat's decision (turn {}, step {}, {} engine offers)",
            app.session.view(human).turn_number,
            app.session.view(human).step.name(),
            app.session.view(human).legal.len()
        );
        assert!(!app.show_review, "the panel starts closed");
        click_text(&mut app, 1280.0, 720.0, "Move review (V)");
        assert!(app.show_review, "the click must open the review panel");
        assert!(
            app.session.review_busy || app.session.review.is_some(),
            "the click must request a review"
        );
        // The frame loop is the only thing that applies worker results, so pump
        // real frames until the review lands.
        let mut frames = 0;
        while app.session.review_busy {
            let _ = run_frame(&mut app, 1280.0, 720.0, frames);
            frames += 1;
            assert!(
                frames < 4_000,
                "the background review never produced a result"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let review = app
            .session
            .review
            .clone()
            .expect("the journey must produce a review panel");
        println!(
            "journey: review applied after {frames} frame(s) · refused={:?} · {} candidate(s) · {} engine decisions · budget {:?}",
            review.refusal(),
            review.ranked().candidates.len(),
            review.ranked().budget.decisions_used,
            review.ranked().budget
        );
        assert!(
            review.refusal().is_none(),
            "the priority decision must not be refused: {:?}",
            review.refusal()
        );
        let offers: Vec<String> = app
            .session
            .view(human)
            .legal
            .iter()
            .map(|legal| legal.label.clone())
            .collect();
        for candidate in &review.ranked().candidates {
            assert!(
                offers.contains(&candidate.move_label),
                "the panel ranked an action the engine never offered: {candidate:?}"
            );
        }
        // A recorded line, when the search recorded one, must reach the panel
        // step by step and be scoped to the search world (AC-15).
        let recorded: Vec<String> = review
            .policy_section()
            .lines
            .iter()
            .filter(|line| line.trim_start().starts_with("line: decision"))
            .cloned()
            .collect();
        for candidate in &review.ranked().candidates {
            match &candidate.line {
                Some(line) if !line.steps.is_empty() => {
                    assert!(
                        recorded.iter().any(|shown| {
                            line.steps
                                .iter()
                                .all(|step| shown.contains(step.label.as_str()))
                        }),
                        "the journey must render this candidate's recorded line: {recorded:?}"
                    );
                }
                _ => {}
            }
        }
        if !recorded.is_empty() {
            let shown = review.policy_section().lines.join("\n");
            assert!(
                shown.contains("search world only"),
                "a rendered line must state that it belongs to the search world: {shown}"
            );
        }
        // Read the panel back out of the interface, through a real scroll.
        assert!(
            scroll_until_visible(&mut app, "ordering:", 1280.0, 720.0),
            "the ranking is not readable in the rendered frame"
        );
        let shown = app
            .session
            .review
            .as_ref()
            .unwrap()
            .policy_section()
            .lines
            .clone();
        let visible: Vec<String> = run_frame(&mut app, 1280.0, 720.0, 0);
        let shown_rows = shown
            .iter()
            .filter(|line| visible.iter().any(|label| label.contains(line.as_str())))
            .count();
        println!("journey: policy section lines read back from the panel:");
        for line in &shown {
            println!("  {line}");
        }
        assert!(
            shown_rows >= 2,
            "the panel's own lines (not just its heading) must be rendered: {shown_rows} of {} matched",
            shown.len()
        );
        assert!(
            shown
                .iter()
                .any(|line| line.contains("no real hand or library order read")),
            "the journey must show the analysis' fidelity line"
        );
        println!(
            "journey: {} of {} policy lines were present in the rendered frame at 1280x720",
            shown_rows,
            shown.len()
        );
    }

    #[test]
    fn absolute_life_labels_remain_correct_when_the_human_is_p1() {
        let mut app = imported_app();
        app.show_import = false;
        app.session.ai.human_seat = 1;
        app.session.ai.ai_seats = vec![P0];
        app.session.game.state.players[0].life = 3;
        app.session.game.state.players[1].life = 17;
        let labels = run_frame(&mut app, 1400.0, 940.0, 0);
        assert!(
            labels.iter().any(|text| text.contains("P0 3 / P1 17")),
            "absolute life labels were swapped for the P1 viewer: {labels:?}"
        );
    }

    #[test]
    fn four_seat_panels_are_visible_at_the_minimum_viewport() {
        let mut app = four_seat_app();
        let labels = run_frame(&mut app, 1280.0, 720.0, 0);
        for seat in 0..4 {
            assert!(
                labels.iter().any(|text| text.contains(&format!("P{seat}"))),
                "P{seat} panel/status missing at 1280x720: {labels:?}"
            );
        }
        assert!(
            labels.iter().any(|text| text.contains("Commander")),
            "bounded 40-life commander data missing: {labels:?}"
        );
        assert!(
            labels.iter().any(|text| text.contains("PROFILE:")),
            "profile setup controls missing: {labels:?}"
        );
        assert!(
            labels.iter().any(|text| text.contains("text 100%")),
            "text zoom control missing: {labels:?}"
        );
    }

    fn run_frame_with_clip(
        app: &mut ApertureApp,
        width: f32,
        height: f32,
    ) -> Vec<(String, egui::Rect, egui::Rect)> {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            ..Default::default()
        };
        let mut warm = ctx.run_ui(input.clone(), |ui| app.render_ui(ui));
        warm.textures_delta.clear();
        let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
        output.textures_delta.clear();
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => {
                    let bounds = text.visual_bounding_rect();
                    Some((text.galley.text().to_owned(), clipped.clip_rect, bounds))
                }
                _ => None,
            })
            .collect()
    }

    fn click_text(app: &mut ApertureApp, width: f32, height: f32, needle: &str) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            ..Default::default()
        };
        let mut warm = ctx.run_ui(input.clone(), |ui| app.render_ui(ui));
        warm.textures_delta.clear();
        let mut warm = ctx.run_ui(input.clone(), |ui| app.render_ui(ui));
        warm.textures_delta.clear();
        let target = warm
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                let labels: Vec<&str> = warm
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect();
                panic!("no clickable label containing {needle:?}: {labels:?}")
            });
        let mut click_input = input;
        click_input.events = vec![
            egui::Event::PointerMoved(target),
            egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let mut clicked = ctx.run_ui(click_input, |ui| app.render_ui(ui));
        clicked.textures_delta.clear();
    }

    #[test]
    fn real_clicks_start_a_four_seat_bounded_table() {
        let mut app = imported_app();
        assert_eq!(app.session.seats.len(), 2);
        assert_eq!(app.session.profile, Profile::Practice20);
        click_text(&mut app, 1280.0, 720.0, "4 seats");
        click_text(
            &mut app,
            1280.0,
            720.0,
            "bounded 40 life · Isamaru each seat",
        );
        click_text(
            &mut app,
            1280.0,
            720.0,
            "Start new table with these options",
        );
        assert_eq!(
            app.session.seats.len(),
            2,
            "a table start that discards the live game must ask before it runs"
        );
        assert!(
            app.confirm.is_some(),
            "the confirmation question is offered in the primary strip"
        );
        click_text(&mut app, 1280.0, 720.0, "Confirm new table");
        assert_eq!(
            app.session.seats.len(),
            4,
            "the confirmed setup did not start four seats"
        );
        assert_eq!(app.session.profile, Profile::CommanderCombat40);
        assert_eq!(app.session.game.state.player(P0).life, 40);
        assert_eq!(app.session.game.state.commanders.len(), 4);
        assert_eq!(app.session.ai.ai_seats, vec![1, 2, 3]);
    }

    #[test]
    fn a_pending_table_selection_is_not_overwritten_by_the_live_table() {
        let mut app = imported_app();
        app.setup_seats = 4;
        app.setup_profile = Profile::CommanderCombat40;
        let _ = run_frame(&mut app, 1280.0, 720.0, 0);
        assert_eq!(
            app.setup_seats, 4,
            "the frame loop reset the user's seat selection"
        );
        assert_eq!(
            app.setup_profile,
            Profile::CommanderCombat40,
            "the frame loop reset the user's profile selection"
        );
    }

    #[test]
    fn four_seat_panels_and_primary_actions_are_actually_visible_at_1280x720() {
        let mut app = four_seat_app();
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        let visible = |needle: &str| {
            shapes.iter().any(|(text, clip, bounds)| {
                text.contains(needle) && clip.intersects(*bounds) && bounds.min.y < 720.0
            })
        };
        for seat in 0..4 {
            assert!(
                visible(&format!("P{seat} ·")),
                "P{seat} seat panel header is not visible at 1280x720"
            );
        }
        assert!(
            visible("YOUR ACTIONS"),
            "the persistent primary action strip is not visible at 1280x720"
        );
        assert!(
            visible("Pass priority")
                || visible("Keep this hand")
                || visible("Mulligan")
                || visible("Let the AI answer")
                || visible("WAITING ON"),
            "no primary action control is visible at 1280x720"
        );
    }

    #[test]
    fn the_postgame_review_replaces_the_action_strip_at_1280x720() {
        let mut app = four_seat_app();
        // A finished game is the only state in which review is offered.
        app.session.game.state.outcome = Some(aperture_core::Outcome::Win {
            winner: 0,
            reason: "scripted finish for the review-mode control".to_string(),
        });
        app.session.enter_review().expect("review opens");
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        let visible = |needle: &str| {
            shapes.iter().any(|(text, clip, bounds)| {
                text.contains(needle) && clip.intersects(*bounds) && bounds.min.y < 720.0
            })
        };
        assert!(
            visible("REVIEW — omniscient postgame view"),
            "the review heading is not visible at 1280x720"
        );
        assert!(
            visible("POSTGAME OMNISCIENT REVIEW"),
            "the omniscient review body is not visible at 1280x720"
        );
        assert!(
            visible("Leave review"),
            "the review has no visible way back to the table"
        );
        assert!(
            !visible("YOUR ACTIONS"),
            "the live action strip must be replaced while review is shown"
        );
        app.session.leave_review();
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        assert!(
            shapes
                .iter()
                .any(|(text, clip, bounds)| text.contains("YOUR ACTIONS")
                    && clip.intersects(*bounds)
                    && bounds.min.y < 720.0),
            "leaving review must restore the live action strip"
        );
    }

    #[test]
    fn four_seat_panels_are_visible_at_a_large_viewport() {
        let mut app = four_seat_app();
        let labels = run_frame(&mut app, 1920.0, 1080.0, 0);
        for seat in 0..4 {
            assert!(
                labels.iter().any(|text| text.contains(&format!("P{seat}"))),
                "P{seat} panel/status missing at 1920x1080: {labels:?}"
            );
        }
    }

    #[test]
    fn imported_match_controls_are_reachable_in_the_actual_minimum_sized_frame() {
        let mut app = imported_app();
        let ctx = egui::Context::default();
        let mut reachable = false;
        let mut visible_before_scroll = false;
        for frame in 0..40 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 760.0),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            };
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(20.0, 640.0)));
            if frame > 1 {
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -90.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
            // This renderer-only test intentionally has no GPU texture backend.
            output.textures_delta.clear();
            for clipped in output.shapes {
                if let egui::Shape::Text(text) = clipped.shape
                    && text.galley.text().contains("Start imported match")
                    && clipped.clip_rect.contains_rect(text.visual_bounding_rect())
                {
                    if frame <= 1 {
                        visible_before_scroll = true;
                    } else {
                        reachable = true;
                    }
                }
            }
        }
        assert!(
            !visible_before_scroll,
            "the regression fixture must require scrolling"
        );
        assert!(
            reachable,
            "the two certified decks' Start button never becomes visible in the actual 1100x760 application frame"
        );
    }

    fn partial_apnap_app() -> ApertureApp {
        let deck = DeckList::new(
            "Native APNAP partial",
            vec![(CardId::Mountain, 20), (CardId::CrackTheEarth, 20)],
        );
        let decks = vec![deck.clone(), deck];
        let seed = (0..1_000u64)
            .find(|seed| {
                let game = Game::new(*seed, &decks).expect("game");
                game.state.first_player == P0
                    && game
                        .state
                        .player(P0)
                        .hand
                        .iter()
                        .any(|id| game.state.object(*id).card == CardId::CrackTheEarth)
                    && game
                        .state
                        .player(P0)
                        .hand
                        .iter()
                        .any(|id| game.state.object(*id).card == CardId::Mountain)
            })
            .expect("seed with Mountain and Crack");
        let ai = AiSettings {
            enabled: true,
            autoplay: false,
            human_seat: P0,
            ai_seats: vec![1],
            ..AiSettings::default()
        };
        let mut session = UiSession::new(seed, decks, ai, false, false).expect("session");
        while matches!(
            session.game.state.pending,
            Some(aperture_core::Pending::Mulligan { .. })
        ) {
            session
                .submit(Action::Mulligan { keep: true })
                .expect("keep");
        }
        let mut guard = 0;
        while !(session.game.state.turn.step == Step::PrecombatMain
            && session.game.state.turn.priority == Some(P0)
            && session.game.state.stack.is_empty())
        {
            session.submit(Action::Pass).expect("pass");
            guard += 1;
            assert!(guard < 64);
        }
        let mountain = *session
            .game
            .state
            .player(P0)
            .hand
            .iter()
            .find(|id| session.game.state.object(**id).card == CardId::Mountain)
            .expect("Mountain");
        session
            .submit(Action::PlayLand { card: mountain })
            .expect("land");
        let crack = *session
            .game
            .state
            .player(P0)
            .hand
            .iter()
            .find(|id| session.game.state.object(**id).card == CardId::CrackTheEarth)
            .expect("Crack");
        session
            .submit(Action::CastSpell {
                card: crack,
                targets: Vec::new(),
            })
            .expect("cast");
        while !matches!(
            session.game.state.pending,
            Some(
                aperture_core::Pending::ApnapSacrifice { .. }
                    | aperture_core::Pending::CommanderReturn { .. },
            )
        ) {
            session.submit(Action::Pass).expect("pass");
        }
        let choice = session
            .game
            .legal_actions()
            .into_iter()
            .find(|legal| matches!(legal.action, Action::ChooseSacrifice { .. }))
            .expect("sacrifice choice")
            .action;
        session.submit(choice).expect("answer P0");
        let startup = startup();
        ApertureApp {
            session,
            startup,
            smoke_started: Instant::now(),
            smoke_requested: false,
            smoke_written: Arc::new(AtomicBool::new(false)),
            last_ai_step: Instant::now(),
            show_sandbox: false,
            show_import: false,
            show_zones: false,
            show_review: false,
            show_coach: true,
            show_study: false,
            show_help: false,
            show_accessibility: false,
            high_contrast: false,
            reduced_motion: false,
            observed_theme: (false, false),
            perf: None,
            setup_seats: 2,
            setup_profile: Profile::Practice20,
            observed_seats: 2,
            observed_profile: Profile::Practice20,
            zone_query: String::new(),
            zone_seat_filter: None,
            zone_zone_filter: None,
            confirm: None,
            counter_object: None,
            life_seat: P0,
            life_value: 20,
            zoom: 1.0,
            frame_count: 0,
        }
    }

    #[test]
    fn partial_apnap_shows_generic_waiting_without_the_owner_label() {
        let mut app = partial_apnap_app();
        assert_eq!(app.session.owner(), Some(1));
        assert_eq!(app.session.game.state.turn.priority, None);
        let labels = run_frame(&mut app, 1100.0, 760.0, 0);
        assert!(
            labels
                .iter()
                .any(|text| text.contains("Waiting on P1 (no player has priority)")),
            "generic waiting status missing: {labels:?}"
        );
        assert!(
            !labels.iter().any(|text| text.contains("P1 holds priority")),
            "false priority claim shown: {labels:?}"
        );
        assert!(
            !labels
                .iter()
                .any(|text| text.contains("choose a permanent to sacrifice")),
            "unentitled APNAP label shown: {labels:?}"
        );
        assert!(
            labels.iter().any(|text| text.contains("Let the AI answer")),
            "AI step control missing: {labels:?}"
        );
    }

    /// Finish the opening mulligan choices so the table reaches a real priority.
    fn finish_opening(app: &mut ApertureApp) {
        let mut guard = 0;
        while app.session.game.state.pending.is_some() {
            app.session
                .submit(Action::Mulligan { keep: true })
                .unwrap_or_else(|err| panic!("opening keep refused: {err}"));
            guard += 1;
            assert!(guard < 40, "the opening procedure did not finish");
        }
    }

    #[test]
    fn the_zone_inspector_is_searchable_and_usable_at_1280x720() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.show_zones = true;
        app.show_coach = false;
        let own = app
            .session
            .view(P0)
            .own_hand
            .first()
            .cloned()
            .expect("hand");
        let hidden = app
            .session
            .view(P0)
            .opponent_view(1)
            .expect("P1")
            .hand_count;
        app.zone_query = own.name.to_string();
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        let visible = |needle: &str| {
            shapes.iter().any(|(text, clip, bounds)| {
                text.contains(needle) && clip.intersects(*bounds) && bounds.min.y < 720.0
            })
        };
        assert!(
            visible("Zone inspector"),
            "the inspector window is not visible"
        );
        assert!(visible("search"), "the search field is missing");
        assert!(
            visible("card(s) match"),
            "the inspector does not report its result count"
        );
        assert!(
            visible(&format!("P0 · hand · {}", own.name)),
            "the searched card is not listed with its real zone"
        );
        assert!(
            !visible(&format!("P1 · hand · {}", own.name)),
            "an opponent hand must never be projected"
        );
        assert_eq!(hidden, 7, "the opponent hand is still a count");
        assert!(
            visible("YOUR ACTIONS"),
            "the persistent primary strip must stay visible with the inspector open"
        );
    }

    #[test]
    fn the_zone_inspector_reports_hidden_zones_as_counts_only() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.show_zones = true;
        app.zone_seat_filter = Some(1);
        app.zone_zone_filter = Some(Zone::Hand);
        app.zone_query = String::new();
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        let visible = |needle: &str| {
            shapes.iter().any(|(text, clip, bounds)| {
                text.contains(needle) && clip.intersects(*bounds) && bounds.min.y < 720.0
            })
        };
        assert!(
            visible("0 card(s) match"),
            "an opponent hand filter must match nothing"
        );
        assert!(
            visible("not projected to this seat"),
            "the inspector must explain why a hidden zone is empty"
        );
    }

    /// Press one key in a real frame, the way the native window receives it.
    fn press_key(app: &mut ApertureApp, width: f32, height: f32, key: egui::Key) {
        press_key_with(app, width, height, key, egui::Modifiers::NONE);
    }

    fn press_key_with(
        app: &mut ApertureApp,
        width: f32,
        height: f32,
        key: egui::Key,
        modifiers: egui::Modifiers,
    ) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            ..Default::default()
        };
        let mut warm = ctx.run_ui(input.clone(), |ui| app.render_ui(ui));
        warm.textures_delta.clear();
        let mut pressed = input;
        pressed.events = vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }];
        let mut frame = ctx.run_ui(pressed, |ui| app.render_ui(ui));
        frame.textures_delta.clear();
    }

    /// Poll the session's background channel until the branch worker is done,
    /// exactly like the real frame loop does.
    fn pump_worker(app: &mut ApertureApp) -> Vec<String> {
        let mut applied = Vec::new();
        for _ in 0..400 {
            applied.extend(app.session.poll_workers());
            if !app.session.io_busy {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(!app.session.io_busy, "the background worker never finished");
        applied
    }

    /// The G shortcut used to replace the live position on the first press
    /// while the Study panel button for the same action asked first. Both now
    /// go through one helper, and this pins the shortcut to the question.
    #[test]
    fn the_g_shortcut_asks_before_it_replaces_the_live_position() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let path =
            std::env::temp_dir().join(format!("aperture-g-branch-{}.json", std::process::id()));
        app.session.save(&path).expect("the study line is written");
        let recorded = app.session.replay.steps.len();
        assert!(recorded >= 2, "a usable recorded line, got {recorded}");
        let keep = recorded - 1;
        app.session.branch_path = path.display().to_string();
        app.session.branch_at = keep;
        let digest = app.session.game.digest();
        let log_len = app.session.log.len();

        press_key(&mut app, 1280.0, 720.0, egui::Key::G);
        assert_eq!(
            app.confirm,
            Some(PendingAction::Branch {
                path: path.clone(),
                keep
            }),
            "G must ask before it replaces the live position"
        );
        assert!(
            app.session.branch_source.is_none(),
            "the shortcut branched without asking"
        );
        assert_eq!(
            app.session.game.digest(),
            digest,
            "the first press must not touch the table"
        );
        assert_eq!(app.session.log.len(), log_len);
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        assert!(
            shapes
                .iter()
                .any(|(text, clip, bounds)| text.contains("CONFIRM: branch from")
                    && clip.intersects(*bounds)
                    && bounds.min.y < 720.0),
            "the branch question is not visible in the fixed primary strip"
        );

        // Esc withdraws the question and changes nothing.
        press_key(&mut app, 1280.0, 720.0, egui::Key::Escape);
        assert!(app.confirm.is_none(), "Esc withdraws the branch question");
        assert_eq!(app.session.game.digest(), digest);

        // The confirmed branch runs exactly what the Study panel button runs.
        press_key(&mut app, 1280.0, 720.0, egui::Key::G);
        click_text(&mut app, 1280.0, 720.0, "Confirm branch");
        assert!(app.confirm.is_none());
        let applied = pump_worker(&mut app);
        assert!(
            applied.iter().any(|line| line.starts_with("branched from")),
            "{applied:?}"
        );
        assert_eq!(
            app.session.replay.steps.len(),
            keep,
            "the branch is cut at the prefilled action index"
        );
        assert_eq!(app.session.branch_at, keep);
        assert!(app.session.branch_source.is_some(), "the base line is kept");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_g_shortcut_refuses_an_empty_or_busy_study_request() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.session.branch_path = String::new();
        press_key(&mut app, 1280.0, 720.0, egui::Key::G);
        assert!(
            app.confirm.is_none(),
            "an empty study path must not offer a branch"
        );
        assert!(
            app.session.message.contains("study file"),
            "{}",
            app.session.message
        );
        assert!(message_is_error(&app.session.message));

        app.session.branch_path = "recorded-line.json".to_string();
        app.session.io_busy = true;
        press_key(&mut app, 1280.0, 720.0, egui::Key::G);
        assert!(
            app.confirm.is_none(),
            "a busy table must not offer a branch"
        );
        assert!(
            app.session.message.contains("still running"),
            "{}",
            app.session.message
        );
        assert!(message_is_error(&app.session.message));
    }

    #[test]
    fn a_failed_destructive_action_is_an_error_state_not_a_hint() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let digest = app.session.game.digest();

        // A table the engine cannot build.
        app.confirm = Some(PendingAction::NewTable {
            seed: 7,
            seats: 5,
            profile: Profile::Practice20,
        });
        click_text(&mut app, 1280.0, 720.0, "Confirm new table");
        assert!(app.confirm.is_none());
        assert_eq!(
            app.session.seats.len(),
            4,
            "a refused start resized the table"
        );
        assert_eq!(
            app.session.game.digest(),
            digest,
            "a refused start replaced the position"
        );
        assert!(
            app.session.message.starts_with("refused:"),
            "{}",
            app.session.message
        );
        assert!(message_is_error(&app.session.message));

        // An imported match with uncertified seats.
        app.session.seat_import = vec![None; app.session.seats.len()];
        app.confirm = Some(PendingAction::StartImported { seed: 8 });
        click_text(&mut app, 1280.0, 720.0, "Confirm imported match");
        assert!(
            app.session.message.starts_with("refused:"),
            "{}",
            app.session.message
        );
        assert!(
            app.session.message.contains("certified"),
            "{}",
            app.session.message
        );
        assert!(message_is_error(&app.session.message));
        assert_eq!(app.session.game.digest(), digest);

        // Success notes and ordinary hints keep the hint colour.
        for ok in [
            "started a new game with seed 3",
            "saved to table.json (digest 3b252264abcd)",
            "loaded table.json — digest 3b252264abcd (stored 3b252264abcd), 3 action(s) in the replay",
        ] {
            assert!(!message_is_error(ok), "{ok:?} was painted as an error");
        }
    }

    #[test]
    fn the_new_controls_are_reachable_at_the_programmatic_minimum_1024x640() {
        // The window minimum eframe enforces is 1024x640 (crates/ui/src/main.rs);
        // 1280x720 is the acceptance viewport, not the smallest real client.
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.show_sandbox = true;
        app.show_coach = false;
        app.session
            .sandbox(SandboxOp::Draw(1))
            .expect("draw override");
        for needle in [
            "YOUR ACTIONS",
            "SANDBOX-MODIFIED",
            "Draw 1 · sandbox",
            "Reveal top 3 · sandbox",
            "Counter editor",
            "Life editor",
        ] {
            assert!(
                scroll_until_visible(&mut app, needle, 1024.0, 640.0),
                "{needle:?} never became visible at the programmatic minimum 1024x640"
            );
        }
        app.show_zones = true;
        assert!(
            scroll_until_visible(&mut app, "Zone inspector", 1024.0, 640.0),
            "the zone inspector is unreachable at 1024x640"
        );
    }

    /// Scroll the real application content with wheel input until `needle` is
    /// fully inside a clip rect (the same interaction the packaged app gets).
    fn scroll_until_visible(app: &mut ApertureApp, needle: &str, width: f32, height: f32) -> bool {
        let ctx = egui::Context::default();
        for frame in 0..60 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, height),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            };
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(20.0, height - 80.0)));
            if frame > 1 {
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -90.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
            output.textures_delta.clear();
            for clipped in output.shapes {
                if let egui::Shape::Text(text) = clipped.shape
                    && text.galley.text().contains(needle)
                    && clipped.clip_rect.contains_rect(text.visual_bounding_rect())
                {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn the_sandbox_panel_separates_overrides_and_shows_the_taint_at_1280x720() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        app.show_sandbox = true;
        app.show_coach = false;
        assert!(
            !app.session.tainted(),
            "the fixture starts with an untouched record"
        );
        app.session
            .sandbox(SandboxOp::Draw(1))
            .expect("draw override");
        assert!(app.session.tainted());

        // The taint is in the fixed primary strip, so it is visible without any
        // scrolling; the controls themselves live in the scrollable table.
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        let visible = |needle: &str| {
            shapes.iter().any(|(text, clip, bounds)| {
                text.contains(needle) && clip.intersects(*bounds) && bounds.min.y < 720.0
            })
        };
        assert!(visible("YOUR ACTIONS"), "legal actions stay on screen");
        assert!(
            visible("NOT A LEGAL MATCH RESULT"),
            "the permanent taint banner is missing from the strip"
        );
        assert!(
            visible("SANDBOX-MODIFIED"),
            "the strip does not mark the session as sandbox-modified"
        );

        // Every override surface must be reachable at the enforced minimum size.
        for needle in [
            "SANDBOX OVERRIDES",
            "LEGAL MATCH ACTIONS stay in YOUR ACTIONS",
            "Draw 1 · sandbox",
            "Surveil 1 · to graveyard · sandbox",
            "Counter editor",
            "Life editor",
        ] {
            assert!(
                scroll_until_visible(&mut app, needle, 1280.0, 720.0),
                "{needle:?} never became visible in the real 1280x720 frame"
            );
        }
        // Scrolling the table must not move the persistent primary strip.
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        assert!(
            shapes
                .iter()
                .any(|(text, clip, bounds)| text.contains("YOUR ACTIONS")
                    && clip.intersects(*bounds)
                    && bounds.min.y < 720.0),
            "the primary strip scrolled away with the table"
        );
    }

    #[test]
    fn a_destructive_action_waits_for_an_explicit_confirmation() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let digest = app.session.game.digest();
        click_text(&mut app, 1280.0, 720.0, "New game");
        assert!(
            app.confirm.is_some(),
            "a new game must ask before it discards the table"
        );
        assert_eq!(
            app.session.game.digest(),
            digest,
            "the live position must survive the first click"
        );
        click_text(&mut app, 1280.0, 720.0, "Cancel");
        assert!(app.confirm.is_none(), "cancel clears the question");
        assert_eq!(app.session.game.digest(), digest, "cancel changes nothing");
        assert!(
            app.session.message.contains("cancelled"),
            "{}",
            app.session.message
        );

        click_text(&mut app, 1280.0, 720.0, "New game");
        click_text(&mut app, 1280.0, 720.0, "Confirm new game");
        assert!(app.confirm.is_none());
        assert_ne!(
            app.session.game.digest(),
            digest,
            "the confirmed action must actually run"
        );
        assert_eq!(app.session.seats.len(), 4, "the table size is retained");
    }

    #[test]
    fn undo_is_only_offered_on_an_untainted_recorded_line() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        assert!(
            app.session.can_undo(),
            "the opening keep is a recorded action, so undo is offered"
        );
        let shapes = run_frame_with_clip(&mut app, 1280.0, 720.0);
        assert!(
            shapes
                .iter()
                .any(|(text, clip, bounds)| text.contains("Undo (U)")
                    && clip.intersects(*bounds)
                    && bounds.min.y < 720.0),
            "the undo control is not reachable in the primary strip"
        );
        app.session
            .sandbox(SandboxOp::Draw(1))
            .expect("draw override");
        let digest = app.session.game.digest();
        assert!(
            !app.session.can_undo(),
            "undo must not pretend a sandbox line is replayable"
        );
        assert!(app.session.undo_last_action().is_err());
        let after = app.session.game.digest();
        assert_eq!(digest, after, "the refused undo changed nothing");
    }

    /// Issue21: the exact command-seat list used by the panel journey.
    const IMPORT_JOURNEY_COMMAND_SEAT: &str = "Commander
1 Isamaru, Hound of Konda
Deck
12 Plains
8 Forest
8 Island
4 Grizzly Bears
4 Centaur Courser
2 Unsummon
2 Swords to Plowshares
";

    #[test]
    fn the_import_panel_reports_both_axes_and_every_unsupported_card_before_start() {
        let mut app = imported_app();
        app.session
            .start_table(61, 2, Profile::CommanderCombat40)
            .expect("bounded command table");
        let refused = IMPORT_JOURNEY_COMMAND_SEAT.replace("2 Unsummon", "2 Darksteel Ingot");
        let import =
            crate::session::import_seat_text_for(0, "P0 refused", &refused, app.session.profile)
                .expect("import runs");
        assert!(!import.certified());
        app.session.seat_import[0] = Some(import);
        app.show_import = true;
        let labels = run_frame(&mut app, 1600.0, 1600.0, 4);
        let joined = labels.join(
            "
",
        );
        assert!(
            joined.contains("format legality (Commander construction)"),
            "the format axis is not shown: {joined}"
        );
        assert!(
            joined.contains("engine support: unsupported"),
            "the support axis is not shown: {joined}"
        );
        assert!(
            joined.contains("Darksteel Ingot"),
            "the exact unsupported card is not listed: {joined}"
        );
        assert!(
            joined.contains("refused:"),
            "the refusal reason is not shown: {joined}"
        );
        assert!(
            joined.contains("command-unfriendly")
                || joined.contains("commander-combat-practice-40"),
            "the panel must name the profile it certifies against: {joined}"
        );
        assert!(
            !app.session.can_start_imported(),
            "an unsupported deck must not enable start"
        );
    }

    #[test]
    fn the_import_panel_starts_a_command_seat_from_certified_lists() {
        let mut app = imported_app();
        app.session
            .start_table(62, 2, Profile::CommandZone20)
            .expect("bounded command table");
        for seat in 0..2u8 {
            let import = crate::session::import_seat_text_for(
                seat,
                &format!("P{seat} imported command"),
                IMPORT_JOURNEY_COMMAND_SEAT,
                app.session.profile,
            )
            .expect("import runs");
            assert!(import.certified(), "refusal: {:?}", import.refusal);
            app.session.seat_import[seat as usize] = Some(import);
        }
        app.show_import = true;
        assert!(app.session.can_start_imported());
        click_text(&mut app, 1280.0, 2400.0, "Start imported match");
        assert!(
            matches!(app.confirm, Some(PendingAction::StartImported { .. })),
            "the start button did not ask for confirmation"
        );
        click_text(&mut app, 1280.0, 2400.0, "Confirm imported match");
        assert_eq!(app.session.game.state.profile, Profile::CommandZone20);
        assert_eq!(app.session.game.state.commanders.len(), 2);
        assert!(app.session.seats.iter().all(|deck| deck.size() == 40));
        assert!(
            app.session
                .seats
                .iter()
                .all(|deck| deck.name.contains("imported command")),
            "the certified lists were applied verbatim"
        );
    }

    fn frame_shapes_at(
        ctx: &egui::Context,
        app: &mut ApertureApp,
        width: f32,
        height: f32,
        zoom: f32,
        frames: usize,
    ) -> Vec<(String, egui::Rect, egui::Rect)> {
        ctx.set_zoom_factor(zoom);
        let mut output = None;
        for frame in 0..frames.max(1) {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, height),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            };
            let mut run = ctx.run_ui(input, |ui| app.render_ui(ui));
            run.textures_delta.clear();
            output = Some(run);
        }
        output
            .expect("at least one frame")
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => {
                    let bounds = text.visual_bounding_rect();
                    Some((text.galley.text().to_owned(), clipped.clip_rect, bounds))
                }
                _ => None,
            })
            .collect()
    }

    fn visible_at(
        shapes: &[(String, egui::Rect, egui::Rect)],
        height: f32,
    ) -> impl Fn(&str) -> bool {
        move |needle| {
            shapes.iter().any(|(text, clip, bounds)| {
                text.contains(needle) && clip.intersects(*bounds) && bounds.min.y < height
            })
        }
    }

    #[test]
    fn zoom_is_repeatable_caps_at_200_percent_and_has_a_100_percent_reset() {
        let mut app = four_seat_app();
        let ctx = egui::Context::default();
        app.set_zoom(&ctx, 5.0);
        assert_eq!(app.zoom, 2.0, "200% must be the usable ceiling");
        app.set_zoom(&ctx, 0.1);
        assert_eq!(app.zoom, 0.7, "the minimum remains 70%");
        app.set_zoom(&ctx, 1.0);

        // Bare + repeats, as the existing shortcut did.
        for expected in [1.1_f32, 1.2, 1.3] {
            press_key(&mut app, 1280.0, 720.0, egui::Key::Plus);
            assert!(
                (app.zoom - expected).abs() < 0.001,
                "repeatable zoom broke: {} != {expected}",
                app.zoom
            );
        }
        // Ctrl+0 is the documented reset and does not fight egui's built-in
        // zoom (which this app disables).
        press_key_with(
            &mut app,
            1280.0,
            720.0,
            egui::Key::Num0,
            egui::Modifiers {
                command: true,
                ctrl: true,
                ..Default::default()
            },
        );
        assert_eq!(app.zoom, 1.0);

        // The visible 100% control performs the same reset.
        app.set_zoom(&ctx, 2.0);
        click_text(&mut app, 1280.0, 720.0, "100%");
        assert_eq!(app.zoom, 1.0);
    }

    #[test]
    fn accessibility_opens_only_without_text_focus_and_escape_closes_it_first() {
        let mut app = four_seat_app();
        press_key(&mut app, 1280.0, 720.0, egui::Key::A);
        assert!(
            app.show_accessibility,
            "A must open the accessibility panel"
        );
        app.confirm = Some(PendingAction::NewGame { seed: 9 });
        press_key(&mut app, 1280.0, 720.0, egui::Key::A);
        assert!(!app.show_accessibility);
        assert!(
            app.confirm.is_some(),
            "A must not disturb a pending confirmation"
        );
        press_key(&mut app, 1280.0, 720.0, egui::Key::A);
        press_key(&mut app, 1280.0, 720.0, egui::Key::Escape);
        assert!(
            !app.show_accessibility,
            "Esc must close the accessibility panel first"
        );
        assert!(
            app.confirm.is_some(),
            "the same Esc press must not also cancel the confirmation"
        );
        press_key(&mut app, 1280.0, 720.0, egui::Key::Escape);
        assert!(
            app.confirm.is_none(),
            "a second Esc cancels the confirmation"
        );

        // With a real focused text field, A is ignored; Esc still closes the
        // panel because it is handled before the text-focus guard.
        let mut text = String::new();
        let focus_id = egui::Id::unique("issue22-focus-probe");
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let mut edit = ctx.run_ui(input.clone(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(focus_id));
        });
        edit.textures_delta.clear();
        ctx.memory_mut(|memory| memory.request_focus(focus_id));
        // Keep the focused TextEdit in the same passes as the app so egui does
        // not drop focus for a widget that was not shown.
        let mut warm = ctx.run_ui(input.clone(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(focus_id));
            app.render_ui(ui);
        });
        warm.textures_delta.clear();
        assert!(ctx.text_edit_focused());
        let mut pressed = input;
        pressed.events = vec![egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        let mut frame = ctx.run_ui(pressed.clone(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(focus_id));
            app.render_ui(ui);
        });
        frame.textures_delta.clear();
        assert!(
            !app.show_accessibility,
            "a single-letter shortcut must not fire while a text field is focused"
        );
        app.show_accessibility = true;
        pressed.events = vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        let mut frame = ctx.run_ui(pressed, |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(focus_id));
            app.render_ui(ui);
        });
        frame.textures_delta.clear();
        assert!(
            !app.show_accessibility,
            "Esc must close the accessibility panel even while text has focus"
        );
    }

    #[test]
    fn the_accessibility_panel_states_are_words_and_install_high_contrast_richtext() {
        let mut app = four_seat_app();
        app.show_accessibility = true;
        let ctx = egui::Context::default();
        let shapes = frame_shapes_at(&ctx, &mut app, 1280.0, 720.0, 1.0, 2);
        let visible = visible_at(&shapes, 720.0);
        assert!(visible("Accessibility (A)"), "the panel did not open");
        assert!(visible("High contrast: OFF"), "the state is not a word");
        assert!(visible("Reduced motion: OFF"), "the state is not a word");

        app.high_contrast = true;
        app.reduced_motion = true;
        let ctx = egui::Context::default();
        let shapes = frame_shapes_at(&ctx, &mut app, 1280.0, 720.0, 1.0, 3);
        let visible = visible_at(&shapes, 720.0);
        assert!(visible("High contrast: ON"), "high contrast state missing");
        assert!(
            visible("Reduced motion: ON"),
            "reduced motion state missing"
        );
        let style = ctx.global_style();
        assert_eq!(
            style.animation_time, 0.0,
            "reduced motion must set egui animation_time to zero"
        );
        assert_eq!(
            style.visuals.override_text_color,
            Some(theme::HC_TEXT),
            "high contrast must use explicit light body text"
        );
        assert!(
            (style.visuals.widgets.active.bg_stroke.width - 2.0).abs() < f32::EPSILON,
            "keyboard focus must have a 2 px outline"
        );

        // Rich text is palette-aware, not merely a background change: the
        // heading keeps a bright accent and card/body text is the HC text.
        let mut heading_colors = Vec::new();
        let ctx = egui::Context::default();
        for frame in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
            output.textures_delta.clear();
            heading_colors.extend(output.shapes.into_iter().filter_map(|clipped| {
                if let egui::Shape::Text(text) = clipped.shape
                    && text.galley.text().contains("THE APERTURE")
                {
                    return text
                        .galley
                        .job
                        .sections
                        .first()
                        .map(|section| section.format.color);
                }
                None
            }));
        }
        assert!(
            heading_colors.contains(&theme::HC_BRASS),
            "the heading richtext was not recoloured for high contrast: {heading_colors:?}"
        );
    }

    #[test]
    fn reduced_motion_uses_static_working_text_without_blocking_worker_polling() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let mut guard = 0;
        while app.session.owner() != Some(1) {
            app.session.submit(Action::Pass).expect("pass to P1");
            guard += 1;
            assert!(guard < 32, "never reached the AI seat");
        }
        // A warm renderer pass before the synthetic work so the measurement is
        // not dominated by first-frame font/theme initialisation.
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };
        let mut warm = ctx.run_ui(input.clone(), |ui| app.render_ui(ui));
        warm.textures_delta.clear();

        // SYNTHETIC delay, explicitly test-only: no model or network service is
        // contacted. The heuristic computes the decision after the sleep.
        let requested = app.session.request_ai_decision(|view| {
            std::thread::sleep(Duration::from_millis(500));
            aperture_ai::HeuristicPolicy::new().decide(&view)
        });
        assert!(requested, "the AI seat should accept the injected work");
        app.reduced_motion = true;

        let mut durations = Vec::new();
        let mut labels = Vec::new();
        for frame in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            };
            let started = Instant::now();
            let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
            durations.push(started.elapsed());
            output.textures_delta.clear();
            labels.extend(output.shapes.into_iter().filter_map(|clipped| {
                if let egui::Shape::Text(text) = clipped.shape {
                    Some(text.galley.text().to_owned())
                } else {
                    None
                }
            }));
            assert!(
                app.session.ai_busy,
                "the frame must report the worker as pending, not block on it"
            );
        }
        assert!(
            labels.iter().any(|text| text.contains("Working...")),
            "reduced motion must show static 'Working...' text: {labels:?}"
        );
        for (index, duration) in durations.iter().enumerate() {
            assert!(
                *duration < Duration::from_millis(250),
                "frame {index} blocked on the synthetic worker delay for {duration:?}"
            );
        }
        for _ in 0..400 {
            app.session.poll_workers();
            if !app.session.ai_busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!app.session.ai_busy, "the injected worker never finished");
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
        output.textures_delta.clear();
        assert!(
            !app.session.ai_busy,
            "polling did not apply the worker result"
        );
    }

    #[test]
    fn a_performance_note_is_a_declared_condition_never_a_synthetic_delay() {
        let mut startup = startup();
        startup.performance_conditions = vec!["shared-desktop".to_string(), "   ".to_string()];
        let conditions = performance_conditions(&startup);
        assert!(
            conditions.synthetic_delays.is_empty(),
            "an ordinary collection note must never be reported as a synthetic delay"
        );
        assert_eq!(conditions.notes, vec!["shared-desktop".to_string()]);
    }

    #[test]
    fn the_written_report_keeps_a_note_out_of_synthetic_delays() {
        let path = std::env::temp_dir().join(format!(
            "aperture-issue22-perf-note-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut startup = startup();
        startup.performance_conditions = vec!["shared-desktop".to_string()];
        let mut probe = PerformanceProbe::with_window(path.clone(), Duration::from_millis(120))
            .with_conditions(performance_conditions(&startup));
        for _ in 0..200 {
            let started = Instant::now();
            let done =
                probe.callback_finished(started, started + Duration::from_micros(400), false);
            if done {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(probe.finished(), "the bounded window never finished");
        probe.finish_and_write();
        for _ in 0..400 {
            if probe.write_done() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(probe.write_done(), "the report writer never finished");
        let report = std::fs::read_to_string(&path).expect("report exists");
        let _ = std::fs::remove_file(&path);
        let json: serde_json::Value = serde_json::from_str(&report).expect("report is json");
        assert_eq!(json["synthetic_delays"], serde_json::json!([]));
        assert_eq!(
            json["declared_conditions"],
            serde_json::json!(["shared-desktop"])
        );
    }

    #[test]
    fn the_opt_in_performance_probe_reports_bounded_native_samples() {
        let path =
            std::env::temp_dir().join(format!("aperture-issue22-perf-{}.json", std::process::id()));
        let mut app = four_seat_app();
        app.perf = Some(PerformanceProbe::with_window(
            path.clone(),
            Duration::from_millis(600),
        ));
        let ctx = egui::Context::default();
        for frame in 0..80 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                time: Some(frame as f64 / 60.0),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.render_ui(ui));
            output.textures_delta.clear();
            if app.perf.as_ref().is_some_and(|probe| probe.finished()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(6));
        }
        let probe = app.perf.as_ref().expect("probe is installed");
        assert!(probe.finished(), "the bounded window never finished");
        for _ in 0..400 {
            if probe.write_done() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            probe.write_done(),
            "the background report write never finished"
        );
        let text = std::fs::read_to_string(&path).expect("report written");
        let report: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(report["schema"], "aperture-ui-performance/1");
        assert_eq!(report["forced_repaint_hz"], 60);
        assert!(
            report["callback_duration_us"]["samples"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );
        assert!(report["callback_duration_us"]["p95"].as_u64().is_some());
        assert!(
            report["inter_callback_interval_us"]["samples"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );
        assert_eq!(report["input_frame_render_us"]["limited"], true);
        assert_eq!(
            report["callback_duration_us"]["method"],
            "nearest-rank over bounded in-memory samples"
        );
        assert!(
            report["synthetic_delays"]
                .as_array()
                .is_some_and(|delays| delays.is_empty())
        );
        let _ = std::fs::remove_file(&path);
    }

    fn inject_large_board(app: &mut ApertureApp, per_seat: usize) {
        use aperture_core::state::Object;
        let state = &mut app.session.game.state;
        for seat in 0..state.players.len() as PlayerId {
            for index in 0..per_seat {
                let id = ObjectId(state.next_object);
                state.next_object += 1;
                state.objects.insert(
                    id,
                    Object {
                        id,
                        card: if index % 2 == 0 {
                            CardId::GrizzlyBears
                        } else {
                            CardId::Forest
                        },
                        owner: seat,
                        controller: seat,
                        zone: Zone::Battlefield,
                        commander: None,
                        tapped: false,
                        damage: 0,
                        summoning_sick: false,
                        pump_power: 0,
                        pump_toughness: 0,
                        plus_one_counters: 0,
                        // A synthetic board-filling permanent starts with no
                        // typed counters; this fixture only needs board bulk.
                        counters: aperture_core::counters::Counters::new(),
                        paired_with: None,
                    },
                );
            }
        }
    }

    #[test]
    fn fixed_controls_stay_visible_at_200_percent_and_high_dpi_minimum_windows() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        // 1024x640 physical at 200% is 512x320 logical points.
        let ctx = egui::Context::default();
        let shapes = frame_shapes_at(&ctx, &mut app, 512.0, 320.0, 2.0, 3);
        let visible = visible_at(&shapes, 320.0);
        for needle in [
            "THE APERTURE",
            "100%",
            "Accessibility (A)",
            "A+",
            "YOUR ACTIONS",
        ] {
            assert!(
                visible(needle),
                "{needle:?} was hidden at the 200%/1024x640 minimum"
            );
        }

        // The acceptance 1280x720 viewport at 200%.
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let ctx = egui::Context::default();
        let shapes = frame_shapes_at(&ctx, &mut app, 640.0, 360.0, 2.0, 3);
        let visible = visible_at(&shapes, 360.0);
        for needle in ["YOUR ACTIONS", "Accessibility (A)", "100%"] {
            assert!(visible(needle), "{needle:?} was hidden at 1280x720/200%");
        }

        // High DPI without text zoom: 512x320 points at 2 physical px/point.
        let mut app = four_seat_app();
        finish_opening(&mut app);
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(2.0);
        let shapes = frame_shapes_at(&ctx, &mut app, 512.0, 320.0, 1.0, 3);
        let visible = visible_at(&shapes, 320.0);
        assert!(
            visible("YOUR ACTIONS"),
            "the primary action strip was hidden by high DPI"
        );
        assert!(visible("Accessibility (A)"));
    }

    #[test]
    fn a_large_board_and_long_per_seat_reports_stay_renderable_and_reachable() {
        let mut app = four_seat_app();
        finish_opening(&mut app);
        inject_large_board(&mut app, 12);
        app.show_coach = false;
        let shapes = frame_shapes_at(&egui::Context::default(), &mut app, 1024.0, 640.0, 1.0, 2);
        let visible = visible_at(&shapes, 640.0);
        assert!(
            visible("Grizzly Bears"),
            "injected permanents did not render"
        );
        assert!(
            visible("YOUR ACTIONS"),
            "the fixed action strip vanished on a large board"
        );
        assert!(
            scroll_until_visible(&mut app, "STACK", 1024.0, 640.0),
            "the stack area is unreachable on a large board"
        );

        let mut app = imported_app();
        let mut list = String::from("40 Forest\n");
        for index in 0..28 {
            list.push_str(&format!("1 Unsupported Card {index}\n"));
        }
        let import =
            crate::session::import_seat_text_for(0, "P0 long report", &list, app.session.profile)
                .expect("the import path reports unsupported entries");
        assert!(!import.certified());
        assert_eq!(import.unsupported.len(), 28);
        app.session.seat_import[0] = Some(import);
        app.show_import = true;
        assert!(
            scroll_until_visible(&mut app, "Start imported match", 1024.0, 640.0),
            "a long per-seat report hid the start control"
        );
    }
}
