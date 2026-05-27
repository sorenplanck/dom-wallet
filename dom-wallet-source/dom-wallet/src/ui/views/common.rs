// Common UI primitives — restrained panels, section headings, status pills.

use egui::{Color32, Rounding, Sense, Stroke, Vec2};

use crate::ui::theme::*;

pub fn section_title(ui: &mut egui::Ui, title: &str, subtitle: Option<&str>) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .color(TEXT_PRIMARY)
            .size(22.0),
    );
    if let Some(s) = subtitle {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(s)
                .color(TEXT_SECONDARY)
                .size(12.0)
                .extra_letter_spacing(1.5),
        );
    }
    ui.add_space(16.0);
    thin_rule(ui);
    ui.add_space(20.0);
}

pub fn thin_rule(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, BORDER_SOFT),
    );
}

/// A restrained panel with matte background, thin border, generous padding.
pub fn panel(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, add_contents);
}

pub fn metric(ui: &mut egui::Ui, label: &str, value: &str, accent: bool) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(TEXT_SECONDARY)
                .size(10.5)
                .extra_letter_spacing(1.8),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(value)
                .color(if accent { AMBER_BRIGHT } else { TEXT_PRIMARY })
                .size(18.0),
        );
    });
}

pub fn status_pill(ui: &mut egui::Ui, label: &str, color: Color32) {
    let text = egui::RichText::new(label).color(color).size(11.0).extra_letter_spacing(1.2);
    egui::Frame::none()
        .fill(Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 24))
        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90)))
        .rounding(Rounding::same(999.0))
        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
        .show(ui, |ui| {
            ui.label(text);
        });
}

/// Tactile dark button with amber hover glow. Returns whether clicked.
pub fn tactile_button(ui: &mut egui::Ui, label: &str, primary: bool) -> bool {
    let text = egui::RichText::new(label)
        .color(if primary { TEXT_PRIMARY } else { TEXT_PRIMARY })
        .size(14.0);
    let desired = Vec2::new(140.0, 44.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    let hovered = response.hovered();
    let painter = ui.painter();

    let fill = if hovered {
        Color32::from_rgb(0x1F, 0x18, 0x0C)
    } else {
        Color32::from_rgb(0x12, 0x0E, 0x09)
    };
    let stroke_color = if hovered { AMBER } else { AMBER_DIM };

    painter.rect(
        rect,
        Rounding::same(8.0),
        fill,
        Stroke::new(1.0, stroke_color),
    );

    // Hover glow — a soft outer amber halo.
    if hovered {
        for i in 0..6 {
            let inflate = i as f32 * 1.5 + 1.0;
            let alpha = (24 - i * 3).max(0) as u8;
            let r = rect.expand(inflate);
            painter.rect_stroke(
                r,
                Rounding::same(8.0 + inflate),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha)),
            );
        }
    }

    let label_color = if primary { AMBER_BRIGHT } else { TEXT_PRIMARY };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text.text(),
        egui::FontId::proportional(14.0),
        label_color,
    );

    response.clicked()
}

pub fn amber_progress(ui: &mut egui::Ui, fraction: f32, label: &str) {
    let f = fraction.clamp(0.0, 1.0);
    ui.label(
        egui::RichText::new(label)
            .color(TEXT_SECONDARY)
            .size(10.5)
            .extra_letter_spacing(1.8),
    );
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(3.0), BG_PANEL_RAISED);
    let filled = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * f, rect.height()));
    painter.rect_filled(filled, Rounding::same(3.0), AMBER);
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(format!("{:.1}%", f * 100.0))
            .color(TEXT_PRIMARY)
            .size(11.5),
    );
}
