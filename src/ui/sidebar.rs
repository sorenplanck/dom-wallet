// Left sidebar — fixed vertical navigation with minimal monochrome icons,
// subtle gold highlight on active state, very thin separators.

use egui::{Align, Color32, Layout, RichText, Rounding, Sense, Stroke, Vec2};

use super::theme::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Inicio,
    Carteira,
    Transacoes,
    Rede,
    Mineracao,
    Diagnosticos,
    Configuracoes,
}

impl Section {
    pub fn label(&self) -> &'static str {
        match self {
            Section::Inicio => "Início",
            Section::Carteira => "Carteira",
            Section::Transacoes => "Transações",
            Section::Rede => "Rede",
            Section::Mineracao => "Mineração",
            Section::Diagnosticos => "Diagnósticos",
            Section::Configuracoes => "Configurações",
        }
    }

    pub fn glyph(&self) -> &'static str {
        // Monochrome geometric glyphs — restrained, no emoji color.
        match self {
            Section::Inicio => "◇",
            Section::Carteira => "◈",
            Section::Transacoes => "↯",
            Section::Rede => "◯",
            Section::Mineracao => "◆",
            Section::Diagnosticos => "≡",
            Section::Configuracoes => "⚙",
        }
    }

    pub const ALL: [Section; 7] = [
        Section::Inicio,
        Section::Carteira,
        Section::Transacoes,
        Section::Rede,
        Section::Mineracao,
        Section::Diagnosticos,
        Section::Configuracoes,
    ];
}

pub fn render(ui: &mut egui::Ui, current: &mut Section) {
    ui.vertical(|ui| {
        ui.add_space(28.0);

        // Brand mark — small geometric DOM glyph + wordmark.
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(
                RichText::new("◉")
                    .color(AMBER)
                    .size(22.0),
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("DOM")
                        .color(TEXT_PRIMARY)
                        .size(15.0)
                        .strong(),
                );
                ui.label(
                    RichText::new("WALLET")
                        .color(TEXT_SECONDARY)
                        .size(10.0)
                        .extra_letter_spacing(2.0),
                );
            });
        });

        ui.add_space(36.0);

        // Thin separator
        thin_sep(ui);
        ui.add_space(20.0);

        for section in Section::ALL {
            let active = *current == section;
            if nav_item(ui, section, active) {
                *current = section;
            }
            ui.add_space(2.0);
        }

        // Fill remainder
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("DEVNET")
                        .color(TEXT_DIM)
                        .size(10.0)
                        .extra_letter_spacing(2.5),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .color(TEXT_DIM)
                        .size(10.0),
                );
            });
            ui.add_space(12.0);
            thin_sep(ui);
        });
    });
}

fn nav_item(ui: &mut egui::Ui, section: Section, active: bool) -> bool {
    let desired = Vec2::new(ui.available_width(), 40.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    let hovered = response.hovered();
    let painter = ui.painter();

    // Active state: very subtle amber wash + a thin left rail.
    if active {
        painter.rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, 16),
        );
        let rail = egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.left() + 2.0, rect.bottom()),
        );
        painter.rect_filled(rail, Rounding::ZERO, AMBER);
    } else if hovered {
        painter.rect_filled(
            rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 6),
        );
    }

    let glyph_color = if active {
        AMBER_BRIGHT
    } else if hovered {
        AMBER
    } else {
        TEXT_SECONDARY
    };
    let label_color = if active {
        TEXT_PRIMARY
    } else if hovered {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    painter.text(
        egui::pos2(rect.left() + 28.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        section.glyph(),
        egui::FontId::proportional(15.0),
        glyph_color,
    );
    painter.text(
        egui::pos2(rect.left() + 52.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        section.label(),
        egui::FontId::proportional(13.5),
        label_color,
    );

    response.clicked()
}

fn thin_sep(ui: &mut egui::Ui) {
    let w = ui.available_width() - 32.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w + 32.0, 1.0), Sense::hover());
    let r = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::pos2(rect.right() - 20.0, rect.center().y),
    );
    ui.painter().line_segment(
        [r.left_center(), r.right_center()],
        Stroke::new(1.0, BORDER_SOFT),
    );
}
