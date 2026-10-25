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
use aperture_core::view::{CardView, PlayerView};
use aperture_core::{Action, AiSettings, ObjectId, P0, PlayerId, Profile, Zone, catalog, coach};
use eframe::egui::{self, RichText};

use crate::session::{
    SandboxOp, UiSession, block_list, import_seat_file, import_seat_text, import_seat_url,
    load_explicit_deck,
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
    show_coach: bool,
    show_study: bool,
    show_help: bool,
    /// Table setup controls; kept in sync with the live table.
    setup_seats: u8,
    setup_profile: Profile,
    /// Last live settings the controls were synced from. Comparing these
    /// (instead of overwriting the controls every frame) preserves a user's
    /// pending selection until they start the new table.
    observed_seats: u8,
    observed_profile: Profile,
    /// Which public zone the user asked to inspect.
    zone_inspect: Option<(PlayerId, Zone)>,
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

fn zone_cards_for(view: &PlayerView, seat: PlayerId, zone: Zone) -> Vec<CardView> {
    let (own, opponent) = if seat == view.viewer {
        (true, None)
    } else {
        (false, view.opponent_view(seat))
    };
    if own {
        match zone {
            Zone::Battlefield => view.own_battlefield.clone(),
            Zone::Graveyard => view.own_graveyard.clone(),
            Zone::Exile => view.own_exile.clone(),
            Zone::Command => view.own_command_zone.clone(),
            Zone::Hand => view.own_hand.clone(),
            Zone::Library | Zone::Stack | Zone::Removed => Vec::new(),
        }
    } else if let Some(opponent) = opponent {
        match zone {
            Zone::Battlefield => opponent.battlefield.clone(),
            Zone::Graveyard => opponent.graveyard.clone(),
            Zone::Exile => opponent.exile.clone(),
            Zone::Command => opponent.command_zone.clone(),
            // Counts are public; contents are intentionally absent.
            Zone::Hand | Zone::Library | Zone::Stack | Zone::Removed => Vec::new(),
        }
    } else {
        Vec::new()
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
            show_coach: true,
            show_study: false,
            show_help: false,
            setup_seats: seat_count as u8,
            setup_profile: initial_profile,
            observed_seats: seat_count as u8,
            observed_profile: initial_profile,
            zone_inspect: None,
            zoom: 1.0,
            frame_count: 0,
        }
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
        self.zoom = zoom.clamp(0.7, 1.9);
        ctx.set_zoom_factor(self.zoom);
    }

    /// Keyboard controls that do not conflict with a focused text field. Tab
    /// and Shift+Tab move egui focus; Enter activates the focused control.
    fn keyboard_shortcuts(&mut self, ctx: &egui::Context) {
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
            plus,
            minus,
            esc,
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
                input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
                input.key_pressed(egui::Key::Minus),
                input.key_pressed(egui::Key::Escape),
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
        if pass && self.session.owner() == Some(self.session.human_seat()) {
            let _ = self.session.submit_human(Action::Pass);
        }
        if new_game {
            let seed = self.session.seed.wrapping_add(1);
            self.session.new_game(seed);
        }
        if save {
            self.session
                .request_save(PathBuf::from(self.session.save_path.clone()));
        }
        if load {
            self.session
                .request_load(PathBuf::from(self.session.save_path.clone()));
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
        if help {
            self.show_help = !self.show_help;
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
            self.session.target_pick = None;
            self.zone_inspect = None;
        }
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
        self.session.inspect = Some(format!(
            "{} — {}\n{} (mana cost)\ncontroller P{}{}{}\n\n{}",
            card.name,
            if card.card.def().is_land() {
                "land"
            } else {
                "permanent / spell"
            },
            card.cost.unwrap_or("(no cost)"),
            card.controller,
            if card.tapped { "\ntapped" } else { "" },
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
            ui.label(RichText::new(description).color(theme::WARN));
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

    fn battlefield_ui(&mut self, ui: &mut egui::Ui, cards: &[CardView], interactive: bool) {
        if cards.is_empty() {
            theme::muted(ui, "(no permanents)");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            let mut sandbox_click: Option<ObjectId> = None;
            let mut inspect_card: Option<CardView> = None;
            for card in cards {
                let label = Self::card_label(card);
                let text = RichText::new(label).color(if card.card.def().is_land() {
                    theme::MUTED
                } else {
                    theme::TEXT
                });
                let response = ui.add(egui::Button::new(text).min_size(egui::vec2(104.0, 32.0)));
                if response.clicked() {
                    if interactive && self.show_sandbox {
                        sandbox_click = Some(card.id);
                    } else {
                        inspect_card = Some(card.clone());
                    }
                }
                if response.secondary_clicked() {
                    inspect_card = Some(card.clone());
                }
                response.on_hover_text(format!("{} {}", card.name, card.oracle));
            }
            if let Some(card) = inspect_card {
                self.inspect_card(&card);
            }
            if let Some(id) = sandbox_click {
                self.session.sandbox(SandboxOp::Counter(id, 1));
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
                        .color(theme::BRASS)
                        .strong(),
                );
                if active {
                    ui.label(RichText::new("ACTIVE").color(theme::GOOD).small());
                }
                if priority {
                    ui.label(RichText::new("PRIORITY").color(theme::BRASS).small());
                }
                if pending_owner {
                    ui.label(RichText::new("PENDING").color(theme::WARN).small());
                }
                if data.eliminated {
                    ui.label(RichText::new("ELIMINATED").color(theme::BAD).strong());
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
                        self.zone_inspect = Some((seat, zone));
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
                    .color(theme::BRASS),
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
                        .color(theme::WARN),
                );
            }
            self.battlefield_ui(ui, &data.battlefield, is_own);
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
                                    theme::TEXT
                                } else {
                                    theme::MUTED
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
        theme::heading(ui, "Deck import — one certified list per seat (2-4 seats)");
        theme::muted(
            ui,
            "URL form: https://archidekt.com/decks/<id>[/slug] (host-allowlisted, canonical API built from the id). \
             Text fallback: paste a list or point at a file. A seat becomes usable only after its exact list passes \
             the practice profile; unsupported input is refused, never replaced. The old two-seat import still \
             labels its seats honestly.",
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
                    .color(theme::BRASS)
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
                    self.session
                        .request_import(seat, move || import_seat_url(seat, &url, &cache));
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
                    self.session
                        .request_import(seat, move || import_seat_text(seat, &name, &text));
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
                        .request_import(seat, move || import_seat_file(seat, &path));
                }
                if busy {
                    ui.spinner();
                    ui.label("working…");
                }
            });
            if let Some(err) = &self.session.seat_error[index] {
                ui.label(RichText::new(format!("ERROR: {err}")).color(theme::BAD));
            }
            if let Some(import) = &self.session.seat_import[index] {
                let (status, color) = if import.certified() {
                    ("CERTIFIED", theme::GOOD)
                } else {
                    ("REFUSED", theme::BAD)
                };
                ui.label(
                    RichText::new(format!(
                        "{status} — '{}' ({} / {} cards)",
                        import.name, import.supported, import.total
                    ))
                    .color(color)
                    .strong(),
                );
                if let Some(list) = &import.list {
                    let profile = aperture_core::catalog::pool_size();
                    ui.label(RichText::new(format!(
                        "planned list applied verbatim: {} card(s) across {} distinct cards; every card is in the {profile}-card pool",
                        list.size(),
                        list.cards.len()
                    )));
                }
                ui.label(RichText::new(import.summary.clone()).monospace().small());
            }
        }
        ui.separator();
        let ready = self.session.can_start_imported();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(ready, egui::Button::new("Start imported match"))
                .clicked()
            {
                let seed = self.session.seed.wrapping_add(1);
                if let Err(err) = self.session.start_imported_match(seed) {
                    self.session.message = err;
                }
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
                ui.spinner();
            }
        });
        if let Some(status) = &self.session.decider_status {
            ui.label(RichText::new(status).color(theme::BRASS).small());
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
            if view.legal.is_empty() {
                theme::muted(ui, "no legal action for your seat right now");
            }
            let mut chosen: Option<Action> = None;
            ui.horizontal_wrapped(|ui| {
                for entry in &view.legal {
                    let label = format!("{}   [CR {}]", entry.label, entry.rules.join(", "));
                    if ui.button(label).clicked() {
                        chosen = Some(entry.action.clone());
                    }
                }
            });
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
                ui.label(RichText::new(format!("Choose a target for {name}:")).color(theme::BRASS));
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

    /// Study panel: branch a recorded line (from a session save or a bare
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
            self.session.request_replay_branch(path, keep);
        }
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
        self.frame_count += 1;
        if self.frame_count <= 3 {
            // eframe applies its own style when the window is created, so the
            // palette is (re)asserted over the first frames.
            theme::apply(&ctx);
        }
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
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("THE APERTURE").heading().color(theme::BRASS));
                theme::muted(ui, crate::session::profile_line());
                ui.separator();
                if ui.small_button("A−").clicked() {
                    let zoom = self.zoom - 0.1;
                    self.set_zoom(&ctx, zoom);
                }
                ui.label(format!("text {:.0}%", self.zoom * 100.0));
                if ui.small_button("A+").clicked() {
                    let zoom = self.zoom + 0.1;
                    self.set_zoom(&ctx, zoom);
                }
                if ui.small_button("Help (H)").clicked() {
                    self.show_help = !self.show_help;
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("TABLE:").color(theme::BRASS).small());
                for count in 2..=4u8 {
                    if ui
                        .selectable_label(self.setup_seats == count, format!("{count} seats"))
                        .clicked()
                    {
                        self.setup_seats = count;
                    }
                }
                ui.separator();
                ui.label(RichText::new("PROFILE:").color(theme::BRASS).small());
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
                    match self
                        .session
                        .start_table(seed, self.setup_seats as usize, self.setup_profile)
                    {
                        Ok(()) => {
                            self.session.deck_refusal = None;
                            self.session.message = format!(
                                "started a {}-seat table, profile {} ({} life); human P0, other seats heuristic AI",
                                self.session.seats.len(),
                                self.session.profile.label(),
                                self.session.profile.starting_life()
                            );
                        }
                        Err(err) => self.session.message = err,
                    }
                }
                theme::muted(
                    ui,
                    "bounded 40 life gives every seat the command-zone library so Isamaru is castable — practice, not Commander",
                );
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("New game").clicked() {
                    let seed = self.session.seed.wrapping_add(1);
                    self.session.new_game(seed);
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
                    self.session
                        .request_load(PathBuf::from(self.session.save_path.clone()));
                }
                if ui.button("Set branch").clicked() {
                    self.session.set_branch();
                }
                if ui.button("Return to branch").clicked()
                    && let Err(err) = self.session.return_to_branch()
                {
                    self.session.message = err;
                }
                if ui.button("Study branches (B)").clicked() {
                    self.show_study = !self.show_study;
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
                        "Keyboard: Tab / Shift+Tab move focus · Enter activates the focused control · P pass · N new game (keeps seats/profile) · S save · L load · I import · B study-branch panel · R postgame review (game over only) · Z zones · K sandbox · C coaching · H help · +/− text zoom · Esc cancels a target pick. Shortcuts are ignored while a text field has focus.",
                    )
                    .small()
                    .color(theme::MUTED),
                );
            }
            ui.horizontal_wrapped(|ui| {
                if let Some((label, _)) = &self.session.branch {
                    ui.label(
                        RichText::new(format!("branch: {label}"))
                            .color(theme::BRASS)
                            .small(),
                    );
                }
                if let Some(pending) = &view.pending {
                    ui.label(RichText::new(format!("WAITING: {pending}")).color(theme::WARN));
                } else if let Some(owner) = owner {
                    if owner == human || view.priority == Some(owner) {
                        ui.label(format!("P{owner} holds priority"));
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "Waiting on P{owner} (no player has priority)"
                            ))
                            .color(theme::WARN),
                        );
                    }
                } else if let Some(conclusion) = self.session.conclude() {
                    ui.label(RichText::new(conclusion).color(theme::GOOD).strong());
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
                            .color(theme::WARN)
                            .small(),
                    );
                }
                if self.session.sandbox_modified {
                    ui.label(RichText::new("SANDBOX-MODIFIED").color(theme::BAD).strong());
                }
                if self.session.ai.use_decider {
                    ui.label(RichText::new("decider-2b opt-in").color(theme::BRASS));
                }
                if self.session.worker_busy() {
                    ui.spinner();
                    ui.label(RichText::new("background work running").color(theme::BRASS));
                }
            });
            if let Some(note) = &self.session.busy_note {
                ui.label(RichText::new(note).color(theme::MUTED).small());
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
                    .max_height(210.0)
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
                    .max_height(210.0)
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
                ui.label(RichText::new(note).color(theme::BRASS).small());
            }
            if !self.session.message.is_empty() {
                ui.label(RichText::new(&self.session.message).color(theme::WARN).small());
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
                                .color(theme::BAD)
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
                                            theme::MUTED
                                        }
                                        coach::ExplanationKind::Rule => theme::TEXT,
                                        coach::ExplanationKind::Simplification => theme::WARN,
                                        coach::ExplanationKind::Strategy => theme::BRASS,
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
                                                    .color(theme::MUTED),
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
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Draw 1").clicked() {
                                self.session.sandbox(SandboxOp::Draw(1));
                            }
                            if ui.button("Mill 1").clicked() {
                                self.session.sandbox(SandboxOp::Mill(1));
                            }
                            if ui.button("Reveal top 3").clicked() {
                                self.session.sandbox(SandboxOp::RevealTop(3));
                            }
                            if ui.button("Reset game").clicked() {
                                self.session.sandbox_modified = false;
                                let seed = self.session.seed;
                                self.session.new_game(seed);
                            }
                            theme::muted(
                                ui,
                                "click one of YOUR permanents while the sandbox is open to add a +1/+1 counter",
                            );
                        });
                        theme::muted(
                            ui,
                            "Sandbox +1/+1 counters are stored and displayed on the card, but they are NOT a rules-supported counter system; combat math ignores them.",
                        );
                        if !self.session.revealed.is_empty() {
                            ui.label(format!(
                                "revealed (sandbox): {}",
                                self.session.revealed.join(", ")
                            ));
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
                            .color(theme::MUTED),
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
            let inspect = self.zone_inspect;
            let all_seats: Vec<PlayerId> = (0..view.seat_count as PlayerId).collect();
            let mut inspect_card: Option<CardView> = None;
            egui::Window::new("Public zones")
                .open(&mut open)
                .resizable(true)
                .default_width(520.0)
                .show(&ctx, |ui| {
                    theme::muted(
                        ui,
                        "Battlefield, graveyard, exile and command zone are public. Opponent hand and library are counts only.",
                    );
                    for seat in &all_seats {
                        ui.separator();
                        let data = SeatPanelData::from_view(&view, &self.session.seats, *seat);
                        ui.label(
                            RichText::new(format!(
                                "P{seat} · life {} · hand {} · library {}{}",
                                data.life,
                                data.hand_count,
                                data.library_count,
                                if data.eliminated { " · ELIMINATED" } else { "" }
                            ))
                            .color(theme::BRASS)
                            .strong(),
                        );
                        let zones: Vec<(Zone, Vec<CardView>)> = vec![
                            (Zone::Graveyard, data.graveyard.clone()),
                            (Zone::Exile, data.exile.clone()),
                            (Zone::Command, data.command_zone.clone()),
                        ];
                        for (zone, cards) in zones {
                            ui.label(
                                RichText::new(format!("{} ({})", zone_label(zone), cards.len()))
                                    .small()
                                    .color(theme::MUTED),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for card in &cards {
                                    if ui.small_button(Self::card_label(card)).clicked() {
                                        inspect_card = Some(card.clone());
                                    }
                                }
                                if cards.is_empty() {
                                    theme::muted(ui, "(empty)");
                                }
                            });
                        }
                    }
                    if let Some((seat, zone)) = inspect {
                        ui.separator();
                        let cards = zone_cards_for(&view, seat, zone);
                        ui.label(
                            RichText::new(format!(
                                "P{seat} {} — {} card(s)",
                                zone_label(zone),
                                cards.len()
                            ))
                            .color(theme::BRASS),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for card in &cards {
                                if ui.small_button(Self::card_label(card)).clicked() {
                                    inspect_card = Some(card.clone());
                                }
                            }
                        });
                    }
                });
            if let Some(card) = inspect_card {
                self.inspect_card(&card);
            }
            if !open {
                self.show_zones = false;
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
    use aperture_core::{CardId, DeckList, Game, Step};

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
        }
    }

    fn imported_app() -> ApertureApp {
        let startup = startup();
        let (a, b, _) = startup_decks(&startup);
        let mut session =
            UiSession::new(1, vec![a, b], AiSettings::default(), false, false).expect("session");
        session.seat_import[0] = Some(import_seat_text(P0, "P0 imported", BUNDLED_A).unwrap());
        session.seat_import[1] = Some(import_seat_text(1, "P1 imported", BUNDLED_B).unwrap());
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
            show_coach: true,
            show_study: false,
            show_help: false,
            setup_seats: 2,
            setup_profile: Profile::Practice20,
            observed_seats: 2,
            observed_profile: Profile::Practice20,
            zone_inspect: None,
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
            show_coach: true,
            show_study: false,
            show_help: false,
            setup_seats: 4,
            setup_profile: Profile::CommanderCombat40,
            observed_seats: 4,
            observed_profile: Profile::CommanderCombat40,
            zone_inspect: None,
            zoom: 1.0,
            frame_count: 0,
        }
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
            4,
            "the clicked setup did not start four seats"
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
            show_coach: true,
            show_study: false,
            show_help: false,
            setup_seats: 2,
            setup_profile: Profile::Practice20,
            observed_seats: 2,
            observed_profile: Profile::Practice20,
            zone_inspect: None,
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
}
