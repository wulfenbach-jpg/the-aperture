//! The card table: header, both battlefields, stack, hand, legal actions,
//! coaching, log, pending-choice controls, sandbox, per-seat deck import and
//! save/load. Every model/network/file operation runs on a background worker;
//! the frame callback never blocks.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use aperture_ai::{DeciderClient, DeciderConfig};
use aperture_core::view::{CardView, PlayerView};
use aperture_core::{Action, AiSettings, ObjectId, P0, P1, PlayerId, catalog, coach};
use eframe::egui::{self, RichText};

use crate::session::{
    BUNDLED_A, BUNDLED_B, SandboxOp, UiSession, block_list, bundled_practice, import_seat_file,
    import_seat_text, import_seat_url, load_explicit_deck,
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
    frame_count: u64,
}

fn seat_label(seat: PlayerId) -> String {
    format!("P{seat}")
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
    let (deck_a, err_a) = if startup.deck_a_explicit {
        load_explicit_deck(&startup.deck_a, "Temur Beats")
    } else {
        (bundled_practice("Temur Beats", BUNDLED_A), None)
    };
    let (deck_b, err_b) = if startup.deck_b_explicit {
        load_explicit_deck(&startup.deck_b, "Izzet Control")
    } else {
        (bundled_practice("Izzet Control", BUNDLED_B), None)
    };
    let refusal = match (err_a, err_b) {
        (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    (deck_a, deck_b, refusal)
}

impl ApertureApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        theme::apply(&cc.egui_ctx);
        let (deck_a, deck_b, deck_refusal) = startup_decks(&startup);
        // A smoke run never spends its frames on model calls: the decider is
        // disabled for the whole capture even when --decider was passed, and a
        // note is printed so the capture is not mistaken for an opt-in run.
        let use_decider = startup.use_decider && startup.smoke.is_none();
        if startup.use_decider && startup.smoke.is_some() {
            eprintln!("smoke mode: Decider-2B calls are disabled for the capture");
        }
        let ai = AiSettings {
            human_seat: if startup.ai_seat == P0 { P1 } else { P0 },
            ai_seats: vec![startup.ai_seat.min(1)],
            autoplay: startup.autoplay || startup.smoke.is_some(),
            use_decider,
            ..AiSettings::default()
        };
        let seats = vec![deck_a, deck_b];
        let mut session = UiSession::new(
            startup.seed,
            seats,
            ai,
            use_decider,
            startup.human_autoplay || startup.smoke.is_some(),
        )
        .expect("the native table always starts with two valid seat lists");
        session.deck_refusal = deck_refusal.clone();
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
            frame_count: 0,
        }
    }

    fn tick_ai(&mut self, ctx: &egui::Context) {
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
                    let _ = self
                        .session
                        .submit_human(Action::DeclareAttackers {
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
        if card.plus_one_counters > 0 || card.pump_power != 0 {
            label.push_str(&format!(
                "  [{}{}]",
                if card.plus_one_counters > 0 {
                    format!("{} +1/+1", card.plus_one_counters)
                } else {
                    String::new()
                },
                if card.pump_power != 0 {
                    format!(" pump {:+}/{:+}", card.pump_power, card.pump_toughness)
                } else {
                    String::new()
                }
            ));
        }
        label
    }

    fn battlefield_ui(
        &mut self,
        ui: &mut egui::Ui,
        seat: PlayerId,
        cards: &[CardView],
        interactive: bool,
    ) {
        if cards.is_empty() {
            theme::muted(ui, "(no permanents)");
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt(("bf", seat))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut sandbox_click: Option<ObjectId> = None;
                    for card in cards {
                        let label = Self::card_label(card);
                        let text = RichText::new(label).color(if card.card.def().is_land() {
                            theme::MUTED
                        } else {
                            theme::TEXT
                        });
                        let response =
                            ui.add(egui::Button::new(text).min_size(egui::vec2(150.0, 54.0)));
                        if response.clicked() {
                            if interactive && self.show_sandbox {
                                sandbox_click = Some(card.id);
                            } else {
                                self.session.inspect = Some(format!(
                                    "{} — {}\n{}\ncontroller P{}",
                                    card.name,
                                    card.oracle,
                                    card.cost.unwrap_or("(no cost)"),
                                    card.controller
                                ));
                            }
                        }
                        response.on_hover_text(format!("{} {}", card.name, card.oracle));
                    }
                    if let Some(id) = sandbox_click {
                        self.session.sandbox(SandboxOp::Counter(id, 1));
                    }
                });
            });
    }

    /// Per-seat import: URL, pasted text or a text file. Each seat is certified
    /// on its own; nothing is substituted and nothing is fetched in this frame.
    fn import_panel(&mut self, ui: &mut egui::Ui) {
        self.import_panel_contents(ui);
    }

    fn import_panel_contents(&mut self, ui: &mut egui::Ui) {
        theme::heading(ui, "Deck import — two seats, each certified on its own");
        theme::muted(
            ui,
            "URL form: https://archidekt.com/decks/<id>[/slug] (host-allowlisted, canonical API built from the id). \
             Text fallback: paste a list or point at a file. A seat becomes usable only after its exact list passes \
             the practice profile; unsupported input is refused, never replaced.",
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
                    .desired_rows(4)
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
                    "disabled until BOTH seats are certified (an unsupported deck cannot start)",
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

impl ApertureApp {
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

        let human = self.session.human_seat();
        let view = self.session.view(human);
        let owner = self.session.owner();

        egui::CentralPanel::default().show(ui, |ui| {
            // Scroll the real application content, not only a final panel that
            // may already start beyond the viewport after the board and log.
            egui::ScrollArea::vertical()
                .id_salt("tabletop-content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("THE APERTURE").heading().color(theme::BRASS));
                theme::muted(ui, crate::session::profile_line());
            });
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
            ui.horizontal_wrapped(|ui| {
                if ui.button("New game").clicked() {
                    let seed = self.session.seed.wrapping_add(1);
                    self.session.new_game(seed);
                }
                if ui.button("Pass / next action").clicked() {
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
                if ui.button("Save").clicked() {
                    self.session
                        .request_save(PathBuf::from(self.session.save_path.clone()));
                }
                if ui.button("Load").clicked() {
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
                ui.checkbox(&mut self.show_coach, "Coaching");
                ui.checkbox(&mut self.show_sandbox, "Sandbox");
                ui.checkbox(&mut self.show_import, "Import");
                ui.checkbox(&mut self.show_zones, "Zones");
            });
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
                    ui.label(format!("P{owner} holds priority"));
                } else if let Some(conclusion) = self.session.conclude() {
                    ui.label(RichText::new(conclusion).color(theme::GOOD).strong());
                }
                ui.label(format!(
                    "turn {} · {} · active P{} · stack {} · {} · digest {}",
                    view.turn_number,
                    view.step.name(),
                    view.active,
                    view.stack.len(),
                    {
                        let mut life = vec![format!("P{} {}", view.viewer, view.own_life)];
                        life.extend(
                            view.opponents
                                .iter()
                                .map(|o| format!("P{} {}{}", o.seat, o.life, if o.eliminated { " (out)" } else { "" })),
                        );
                        life.join(" / ")
                    },
                    view.digest
                ));
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

            ui.columns(2, |columns| {
                // ---------------- left: the table
                let left = &mut columns[0];
                let opponents = view.opponents.clone();
                for opponent in &opponents {
                    theme::heading(
                        left,
                        &format!(
                            "OPPONENT  P{}  life {}{}  hand {}  library {}  graveyard {}  exile {}",
                            opponent.seat,
                            opponent.life,
                            if opponent.eliminated { "  [LEFT THE GAME]" } else { "" },
                            opponent.hand_count,
                            opponent.library_count,
                            opponent.graveyard.len(),
                            opponent.exile.len()
                        ),
                    );
                    theme::muted(
                        left,
                        "opponent hand contents are hidden; graveyard and exile are public",
                    );
                    self.battlefield_ui(left, opponent.seat, &opponent.battlefield, false);
                }

                theme::heading(left, "STACK");
                if view.stack.is_empty() {
                    theme::muted(left, "(stack empty)");
                }
                for (index, item) in view.stack.iter().rev().enumerate() {
                    let targets = if item.targets.is_empty() {
                        String::new()
                    } else {
                        format!(" → {}", item.targets.join(", "))
                    };
                    left.label(format!(
                        "{}. {}{}  (P{})",
                        index + 1,
                        item.name,
                        targets,
                        item.controller
                    ));
                }

                theme::heading(
                    left,
                    &format!(
                        "YOU  P{human}  life {}  hand {}  library {}  mana {}",
                        view.own_life,
                        view.own_hand.len(),
                        view.own_library_count,
                        view.own_mana
                    ),
                );
                theme::muted(
                    left,
                    format!(
                        "graveyard {} · exile {} · lands played this turn {} · mulligans {}",
                        view.own_graveyard.len(),
                        view.own_exile.len(),
                        self.session.game.state.turn.lands_played,
                        view.own_mulligans
                    ),
                );
                let own_cards = view.own_battlefield.clone();
                self.battlefield_ui(left, human, &own_cards, true);

                theme::heading(left, "YOUR HAND");
                let hand = view.own_hand.clone();
                let legal = view.legal.clone();
                let mut play: Option<Action> = None;
                for card in &hand {
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
                    let response = left.add_enabled(
                        enabled,
                        egui::Button::new(RichText::new(label).color(if enabled {
                            theme::TEXT
                        } else {
                            theme::MUTED
                        })),
                    );
                    if response.clicked() {
                        match matching.len() {
                            0 => {
                                self.session.inspect = Some(format!(
                                    "{}\n{}\n{}",
                                    card.name,
                                    card.cost.unwrap_or("(no cost)"),
                                    card.oracle
                                ));
                            }
                            1 => play = Some(matching[0].clone()),
                            _ => self.session.target_pick = Some((card.id, matching)),
                        }
                    }
                    response.on_hover_text(card.oracle);
                }
                if let Some(action) = play {
                    let _ = self.session.submit_human(action);
                }
                if self.session.target_pick.is_some() {
                    let (card, options) = self.session.target_pick.clone().unwrap();
                    let name = self
                        .session
                        .game
                        .state
                        .try_object(card)
                        .map(|o| o.name())
                        .unwrap_or("spell");
                    left.label(RichText::new(format!("Choose a target for {name}:")).color(theme::BRASS));
                    for action in options {
                        let label = legal
                            .iter()
                            .find(|entry| entry.action == action)
                            .map(|entry| entry.label.clone())
                            .unwrap_or_else(|| "cast".to_string());
                        if left.button(label).clicked() {
                            let _ = self.session.submit_human(action);
                        }
                    }
                    if left.button("Cancel").clicked() {
                        self.session.target_pick = None;
                    }
                }

                // ---------------- right: state, actions, coaching, log
                let right = &mut columns[1];
                theme::heading(right, "LEGAL ACTIONS — only these are accepted");
                if view.legal.is_empty() {
                    theme::muted(right, "no legal action for your seat right now");
                }
                let mut chosen: Option<Action> = None;
                for entry in &view.legal {
                    let label = format!("{}   [CR {}]", entry.label, entry.rules.join(", "));
                    if right.button(label).clicked() {
                        chosen = Some(entry.action.clone());
                    }
                }
                if let Some(action) = chosen {
                    let _ = self.session.submit_human(action);
                }

                if view.pending_kind.is_some() {
                    right.separator();
                    let pending_seat = owner.unwrap_or(human);
                    if pending_seat == human {
                        let pending_view = view.clone();
                        self.pending_controls(right, &pending_view);
                    } else {
                        theme::heading(right, "WAITING ON THE OTHER SEAT");
                        theme::muted(
                            right,
                            format!(
                                "P{pending_seat} must answer: {}. Your controls are disabled for this decision point.",
                                view.pending.clone().unwrap_or_default()
                            ),
                        );
                        if self.session.seat_is_ai(pending_seat)
                            && right.button("Let the AI answer (step)").clicked()
                        {
                            self.session.request_ai_decision_default();
                        }
                    }
                }

                if self.show_coach {
                    right.separator();
                    theme::heading(right, "COACHING (derived from the engine trace)");
                    let explanations = coach::explain(&self.session.game, human, &view.legal);
                    egui::ScrollArea::vertical()
                        .id_salt("coach")
                        .max_height(240.0)
                        .show(right, |ui| {
                            for explanation in explanations {
                                ui.label(
                                    RichText::new(format!("[{}] {}", explanation.topic, explanation.text))
                                        .color(if explanation.topic.contains("unavailable") {
                                            theme::MUTED
                                        } else {
                                            theme::TEXT
                                        }),
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

                right.separator();
                theme::heading(right, "LOG (viewer-masked)");
                egui::ScrollArea::vertical()
                    .id_salt("log")
                    .max_height(200.0)
                    .show(right, |ui| {
                        for line in view.log.iter().rev().take(60) {
                            ui.label(RichText::new(line).small());
                        }
                    });

                if let Some(note) = &self.session.note {
                    right.label(RichText::new(note).color(theme::BRASS).small());
                }
                if !self.session.message.is_empty() {
                    right.label(RichText::new(&self.session.message).color(theme::WARN).small());
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
                        "click a permanent while the sandbox is open to add a +1/+1 counter (storage only)",
                    );
                });
                if !self.session.revealed.is_empty() {
                    ui.label(format!("revealed (sandbox): {}", self.session.revealed.join(", ")));
                }
            }

            if self.show_import {
                ui.separator();
                self.import_panel(ui);
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
                });
            if !open {
                self.session.inspect = None;
            }
        }

        if self.show_zones {
            let mut open = true;
            let own_graveyard = view.own_graveyard.clone();
            let own_exile = view.own_exile.clone();
            let opponent_graveyards: Vec<Vec<CardView>> = view
                .opponents
                .iter()
                .map(|o| o.graveyard.clone())
                .collect();
            let opponent_exiles: Vec<Vec<CardView>> = view
                .opponents
                .iter()
                .map(|o| o.exile.clone())
                .collect();
            egui::Window::new("Public zones")
                .open(&mut open)
                .resizable(true)
                .show(&ctx, |ui| {
                    ui.label(RichText::new("Graveyard (public)").color(theme::BRASS));
                    for card in own_graveyard.iter().chain(opponent_graveyards.iter().flatten()) {
                        ui.label(Self::card_label(card));
                    }
                    ui.separator();
                    ui.label(RichText::new("Exile (public)").color(theme::BRASS));
                    for card in own_exile.iter().chain(opponent_exiles.iter().flatten()) {
                        ui.label(Self::card_label(card));
                    }
                });
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

impl eframe::App for ApertureApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}

#[cfg(test)]
mod viewport_tests {
    use super::*;

    fn imported_app() -> ApertureApp {
        let startup = Startup {
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
        };
        let (a, b, _) = startup_decks(&startup);
        let mut session = UiSession::new(1, a, b, AiSettings::default(), false, false);
        session.seat_import[0] = Some(import_seat_text(P0, "P0 imported", BUNDLED_A).unwrap());
        session.seat_import[1] = Some(import_seat_text(P1, "P1 imported", BUNDLED_B).unwrap());
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
            frame_count: 0,
        }
    }

    #[test]
    fn absolute_life_labels_remain_correct_when_the_human_is_p1() {
        let mut app = imported_app();
        app.show_import = false;
        app.session.ai.ai_seat = P0;
        app.session.game.state.players[0].life = 3;
        app.session.game.state.players[1].life = 17;
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 940.0),
                )),
                ..Default::default()
            },
            |ui| app.render_ui(ui),
        );
        output.textures_delta.clear();
        let labels: Vec<String> = output
            .shapes
            .into_iter()
            .filter_map(|clipped| {
                if let egui::Shape::Text(text) = clipped.shape {
                    Some(text.galley.text().to_owned())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            labels.iter().any(|text| text.contains("life P0 3 / P1 17")),
            "absolute life labels were swapped for the P1 viewer: {labels:?}"
        );
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
                .push(egui::Event::PointerMoved(egui::pos2(20.0, 200.0)));
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
}
