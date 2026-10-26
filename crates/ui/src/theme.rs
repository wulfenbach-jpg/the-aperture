//! Dark charcoal / brass theme. Applied once at start-up through the egui
//! style so every widget stays readable, and re-asserted over the first frames
//! because eframe installs its own style when the native window is created.
//!
//! The normal palette is the original issue20-era charcoal/brass. The
//! accessibility panel can switch to an explicit high-contrast palette (white
//! text, near-black surfaces, strong borders and a 2 px focus ring) and can
//! disable animation for reduced-motion users. Rich text colours are read from
//! the active palette through [`text_color`], [`muted_color`], [`brass`],
//! [`good`], [`warn`] and [`bad`]; the constants remain the source of the
//! normal palette.

use eframe::egui::{self, Color32, Stroke};

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

/// Explicit high-contrast surfaces and accents. Pure white body text on
/// near-black surfaces, bright accents that keep their meaning (green/yellow/
/// red) and a light widget border that is visible without relying on fill.
pub const HC_CHARCOAL: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const HC_PANEL: Color32 = Color32::from_rgb(0x07, 0x09, 0x0e);
pub const HC_CARD_FACE: Color32 = Color32::from_rgb(0x11, 0x14, 0x1c);
pub const HC_TEXT: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
pub const HC_MUTED: Color32 = Color32::from_rgb(0xd8, 0xdd, 0xe6);
pub const HC_BRASS: Color32 = Color32::from_rgb(0xff, 0xd7, 0x5e);
pub const HC_BRASS_DIM: Color32 = Color32::from_rgb(0xc9, 0xa2, 0x3c);
pub const HC_GOOD: Color32 = Color32::from_rgb(0x77, 0xff, 0x96);
pub const HC_WARN: Color32 = Color32::from_rgb(0xff, 0xc4, 0x4d);
pub const HC_BAD: Color32 = Color32::from_rgb(0xff, 0x8e, 0x7e);
pub const HC_BORDER: Color32 = Color32::from_rgb(0xc8, 0xd1, 0xdd);
pub const HC_FOCUS: Color32 = Color32::from_rgb(0xff, 0xff, 0x00);

/// Colours consumed by the renderer. `Palette::NORMAL` is the original style;
/// `Palette::HIGH_CONTRAST` is the accessibility alternative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub charcoal: Color32,
    pub panel: Color32,
    pub card_face: Color32,
    pub extreme_bg: Color32,
    pub brass: Color32,
    pub brass_dim: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,
    pub border: Color32,
    pub focus: Color32,
    pub hovered_fg: Color32,
    pub active_fg: Color32,
    pub high_contrast: bool,
}

impl Palette {
    pub const NORMAL: Palette = Palette {
        charcoal: CHARCOAL,
        panel: PANEL,
        card_face: CARD_FACE,
        extreme_bg: Color32::from_rgb(0x0e, 0x0f, 0x11),
        brass: BRASS,
        brass_dim: BRASS_DIM,
        text: TEXT,
        muted: MUTED,
        good: GOOD,
        warn: WARN,
        bad: BAD,
        border: Color32::from_rgb(0x3a, 0x3b, 0x42),
        focus: Color32::from_rgb(0xf2, 0xd0, 0x7c),
        hovered_fg: Color32::from_rgb(0x1a, 0x14, 0x0a),
        active_fg: Color32::from_rgb(0x1a, 0x14, 0x0a),
        high_contrast: false,
    };

    pub const HIGH_CONTRAST: Palette = Palette {
        charcoal: HC_CHARCOAL,
        panel: HC_PANEL,
        card_face: HC_CARD_FACE,
        extreme_bg: Color32::from_rgb(0x00, 0x00, 0x00),
        brass: HC_BRASS,
        brass_dim: HC_BRASS_DIM,
        text: HC_TEXT,
        muted: HC_MUTED,
        good: HC_GOOD,
        warn: HC_WARN,
        bad: HC_BAD,
        border: HC_BORDER,
        focus: HC_FOCUS,
        hovered_fg: Color32::from_rgb(0x00, 0x00, 0x00),
        active_fg: Color32::from_rgb(0x00, 0x00, 0x00),
        high_contrast: true,
    };

    pub fn for_mode(high_contrast: bool) -> Self {
        if high_contrast {
            Self::HIGH_CONTRAST
        } else {
            Self::NORMAL
        }
    }
}

/// Palette storage is per-egui-`Context` (not process-global config), so two
/// windows/tests cannot leak an accessibility choice into each other.
fn palette_id() -> egui::Id {
    egui::Id::unique("aperture-active-palette")
}

/// The palette currently published on this UI's context.
pub fn current(ui: &egui::Ui) -> Palette {
    ui.ctx()
        .data(|data| data.get_temp::<Palette>(palette_id()))
        .unwrap_or(Palette::NORMAL)
}

/// Publish a palette and install the widget/richtext style for it.
pub fn apply_mode(ctx: &egui::Context, high_contrast: bool, reduced_motion: bool) {
    let palette = Palette::for_mode(high_contrast);
    publish_palette(ctx, high_contrast);
    // Own the zoom shortcuts: the app supports 70%-200% and a 100% reset
    // (Ctrl+0). egui's built-in 20%-500% keyboard zoom would fight that range.
    ctx.options_mut(|options| options.zoom_with_keyboard = false);
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(|style| {
        style.animation_time = if reduced_motion { 0.0 } else { 0.2 };
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = palette.charcoal;
        style.visuals.window_fill = palette.panel;
        style.visuals.extreme_bg_color = palette.extreme_bg;
        style.visuals.faint_bg_color = palette.panel;
        style.visuals.override_text_color = Some(palette.text);
        style.visuals.widgets.noninteractive.bg_fill = palette.panel;
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.muted);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
        style.visuals.widgets.inactive.bg_fill = palette.card_face;
        style.visuals.widgets.inactive.weak_bg_fill = palette.card_face;
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
        style.visuals.widgets.hovered.bg_fill = palette.brass_dim;
        style.visuals.widgets.hovered.weak_bg_fill = palette.brass_dim;
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.hovered_fg);
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.focus);
        style.visuals.widgets.active.bg_fill = palette.brass;
        style.visuals.widgets.active.weak_bg_fill = palette.brass;
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.active_fg);
        // egui renders keyboard focus with the `active` widget style. A 2 px
        // focus-coloured frame makes Tab/Enter focus unmistakable.
        style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, palette.focus);
        style.visuals.selection.bg_fill = palette.brass_dim;
        style.visuals.selection.stroke = Stroke::new(2.0, palette.focus);
        style.visuals.window_stroke = Stroke::new(
            if high_contrast { 2.0 } else { 1.0 },
            if high_contrast {
                palette.border
            } else {
                palette.brass_dim
            },
        );
    });
}

/// Store the active palette on this context. Rich text helpers read it, so
/// every render pass must publish it (a new egui context starts with none).
pub fn publish_palette(ctx: &egui::Context, high_contrast: bool) {
    let palette = Palette::for_mode(high_contrast);
    ctx.data_mut(|data| data.insert_temp(palette_id(), palette));
}

/// Original normal-contrast start-up theme.
pub fn apply(ctx: &egui::Context) {
    apply_mode(ctx, false, false);
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(brass(ui)).strong());
}

pub fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(
        egui::RichText::new(text.into())
            .color(muted_color(ui))
            .small(),
    );
}

pub fn text_color(ui: &egui::Ui) -> Color32 {
    current(ui).text
}

pub fn muted_color(ui: &egui::Ui) -> Color32 {
    current(ui).muted
}

pub fn brass(ui: &egui::Ui) -> Color32 {
    current(ui).brass
}

pub fn good(ui: &egui::Ui) -> Color32 {
    current(ui).good
}

pub fn warn(ui: &egui::Ui) -> Color32 {
    current(ui).warn
}

pub fn bad(ui: &egui::Ui) -> Color32 {
    current(ui).bad
}
