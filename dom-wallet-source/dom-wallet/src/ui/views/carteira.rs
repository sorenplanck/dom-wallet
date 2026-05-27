// Carteira — wallet identity, lock state, password management, seed reveal.
//
// Password rules per spec:
//   - protects: signing, seed reveal, key export, sensitive controls
//   - does NOT protect: node, sync, mining, peer relay

use egui::{Layout, RichText, Sense, Vec2};

use crate::ui::app::DomApp;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    section_title(
        ui,
        "Carteira",
        Some("IDENTIDADE MONETÁRIA DETERMINÍSTICA"),
    );

    ui.horizontal_top(|ui| {
        // Left: identity card
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width() * 0.55, ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(
                        RichText::new("ENDEREÇO PERSISTENTE")
                            .color(TEXT_SECONDARY)
                            .size(10.5)
                            .extra_letter_spacing(2.0),
                    );
                    ui.add_space(8.0);
                    let addr = app.wallet.address();
                    ui.label(
                        RichText::new(&addr)
                            .color(AMBER_BRIGHT)
                            .size(15.0)
                            .monospace(),
                    );
                    ui.add_space(8.0);
                    if tactile_button(ui, "Copiar", false) {
                        ui.ctx().copy_text(addr);
                    }
                    ui.add_space(20.0);
                    thin_rule(ui);
                    ui.add_space(16.0);

                    ui.label(
                        RichText::new("ESTADO")
                            .color(TEXT_SECONDARY)
                            .size(10.5)
                            .extra_letter_spacing(2.0),
                    );
                    ui.add_space(6.0);
                    let unlocked = app.wallet.is_unlocked();
                    ui.horizontal(|ui| {
                        if unlocked {
                            status_pill(ui, "DESBLOQUEADA", SUCCESS);
                        } else {
                            status_pill(ui, "BLOQUEADA", AMBER);
                        }
                    });
                });
            },
        );

        ui.add_space(20.0);

        // Right: actions
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(
                        RichText::new("OPERAÇÕES SENSÍVEIS")
                            .color(TEXT_SECONDARY)
                            .size(10.5)
                            .extra_letter_spacing(2.0),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "O nó, a sincronização, a mineração e a conectividade \
                            continuam ativos independentemente do estado de bloqueio. \
                            A senha protege somente operações da carteira.",
                        )
                        .color(TEXT_SECONDARY)
                        .size(12.0),
                    );
                    ui.add_space(18.0);

                    if app.wallet.is_unlocked() {
                        if tactile_button(ui, "Bloquear", true) {
                            app.wallet.lock();
                            app.notify("Carteira bloqueada");
                        }
                        ui.add_space(10.0);
                        if tactile_button(ui, "Ver semente", false) {
                            app.open_reveal_seed();
                        }
                        ui.add_space(10.0);
                        if tactile_button(ui, "Mudar senha", false) {
                            app.open_change_password();
                        }
                    } else {
                        if tactile_button(ui, "Desbloquear", true) {
                            app.open_unlock_modal();
                        }
                    }
                });
            },
        );
    });
}
