//! Dark charcoal / brass theme. Applied once at start-up through the egui
//! style so every widget stays readable.

use eframe::egui::{self, Color32};

pub const CHARCOAL: Color32 = Color32::from_rgb(0x14, 0x15, 0x18);
pub const PANEL: Color32 = Color32::from_rgb(0x1c, 0x1d, 0x21);
pub const CARD_FACE: Color32 = Color32::from_rgb(0x24, 0x25, 0x2a);
pub const BRASS: Color32 = Color32::from_rgb(0xb0, 0x8d, 0x57);
pub const BRASS_DIM: Color32 = Color32::from_rgb(0x7a, 0x62, 0x3d);
pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe2, 0xd4);
pub const MUTED: Color32 = Color32::from_rgb(0x9a, 0x95, 0x8a);
pub const GOOD: Color32 = Color32::from_rgb(0x7f, 0xbf, 0x7f);
pub const WARN: Color32 = Color32::from_rgb(0xd0, 0x8a, 0x4a);
pub const BAD: Color32 = Color32::from_rgb(0xc8, 0x6a, 0x5a);

pub fn apply(ctx: &egui::Context) {
    // Ask for the dark theme explicitly and write the palette into every style,
    // so the window cannot fall back to the light style on a fresh installation.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.panel_fill = CHARCOAL;
        visuals.window_fill = PANEL;
        visuals.extreme_bg_color = Color32::from_rgb(0x0e, 0x0f, 0x11);
        visuals.faint_bg_color = PANEL;
        visuals.override_text_color = Some(TEXT);
        visuals.widgets.noninteractive.bg_fill = PANEL;
        visuals.widgets.noninteractive.fg_stroke.color = MUTED;
        visuals.widgets.inactive.bg_fill = CARD_FACE;
        visuals.widgets.inactive.weak_bg_fill = CARD_FACE;
        visuals.widgets.inactive.fg_stroke.color = TEXT;
        visuals.widgets.hovered.bg_fill = BRASS_DIM;
        visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(0x1a, 0x14, 0x0a);
        visuals.widgets.active.bg_fill = BRASS;
        visuals.widgets.active.fg_stroke.color = Color32::from_rgb(0x1a, 0x14, 0x0a);
        visuals.selection.bg_fill = BRASS_DIM;
        visuals.window_stroke.color = BRASS_DIM;
    });
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(BRASS).strong());
}

pub fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(egui::RichText::new(text.into()).color(MUTED).small());
}
