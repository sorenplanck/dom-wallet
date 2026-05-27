// Theme. Strict palette per spec:
//   #06070A primary background
//   #0D1117 secondary panels
//   #1A1F29 soft borders
//   #D6A85F primary accent
//   #F0C674 bright accent
//   #F3F4F6 text primary
//   #8B949E text secondary
//   #3FB950 success
//   #F85149 error

use egui::{Color32, Stroke};

pub const BG_PRIMARY: Color32 = Color32::from_rgb(0x06, 0x07, 0x0A);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x0D, 0x11, 0x17);
pub const BG_PANEL_RAISED: Color32 = Color32::from_rgb(0x11, 0x16, 0x1E);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(0x1A, 0x1F, 0x29);
pub const BORDER_DIM: Color32 = Color32::from_rgb(0x12, 0x16, 0x1D);

pub const AMBER: Color32 = Color32::from_rgb(0xD6, 0xA8, 0x5F);
pub const AMBER_BRIGHT: Color32 = Color32::from_rgb(0xF0, 0xC6, 0x74);
pub const AMBER_DIM: Color32 = Color32::from_rgb(0x6E, 0x55, 0x30);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x8B, 0x94, 0x9E);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x55, 0x5B, 0x66);

pub const SUCCESS: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x50);
pub const ERROR: Color32 = Color32::from_rgb(0xF8, 0x51, 0x49);

/// Install our dark/restrained visuals onto egui.
pub fn install(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_PRIMARY;
    visuals.window_fill = BG_PANEL;
    visuals.window_stroke = Stroke::new(1.0, BORDER_SOFT);
    visuals.extreme_bg_color = BG_PRIMARY;
    visuals.faint_bg_color = BG_PANEL;
    visuals.code_bg_color = BG_PANEL_RAISED;
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 12.0),
        blur: 32.0,
        spread: 0.0,
        color: Color32::from_black_alpha(160),
    };
    visuals.popup_shadow = visuals.window_shadow;

    // Selection / accent.
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, 40);
    visuals.selection.stroke = Stroke::new(1.0, AMBER);

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SOFT);

    visuals.widgets.inactive.bg_fill = BG_PANEL_RAISED;
    visuals.widgets.inactive.weak_bg_fill = BG_PANEL_RAISED;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SOFT);
    visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x18, 0x1E, 0x28);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x18, 0x1E, 0x28);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, AMBER_BRIGHT);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, AMBER_DIM);
    visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);

    visuals.widgets.active.bg_fill = Color32::from_rgb(0x1F, 0x26, 0x32);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, AMBER_BRIGHT);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, AMBER);
    visuals.widgets.active.rounding = egui::Rounding::same(6.0);

    ctx.set_visuals(visuals);

    // Typography: slightly looser line spacing, generous heading sizes,
    // restrained body weight to feel calm and editorial.
    let mut style = (*ctx.style()).clone();
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(28.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    ctx.set_style(style);
}
