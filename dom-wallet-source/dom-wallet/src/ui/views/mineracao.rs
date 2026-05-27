// Mineração — toggle, hashrate, auto-start on boot.

use egui::{Layout, RichText, Vec2};

use crate::persist::runtime_state::RuntimeState;
use crate::ui::app::DomApp;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    section_title(ui, "Mineração", Some("PRODUÇÃO DETERMINÍSTICA DE BLOCOS"));

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width() * 0.5, ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    let on = app.node.miner.is_enabled();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("ESTADO")
                                .color(TEXT_SECONDARY)
                                .size(10.5)
                                .extra_letter_spacing(2.0),
                        );
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if on {
                                status_pill(ui, "ATIVA", AMBER_BRIGHT);
                            } else {
                                status_pill(ui, "INATIVA", TEXT_DIM);
                            }
                        });
                    });
                    ui.add_space(18.0);
                    let label = if on { "Parar mineração" } else { "Iniciar mineração" };
                    if tactile_button(ui, label, true) {
                        app.node.set_mining(!on);
                    }
                    ui.add_space(20.0);
                    thin_rule(ui);
                    ui.add_space(16.0);

                    // Auto-mine toggle (persisted in runtime state).
                    let mut rs = RuntimeState::load(&app.paths.runtime).unwrap_or_default();
                    let mut auto = rs.auto_mine;
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut auto, "Iniciar mineração com o sistema").changed() {
                            rs.auto_mine = auto;
                            let _ = rs.save(&app.paths.runtime);
                        }
                    });
                    ui.label(
                        RichText::new(
                            "A mineração roda como serviço de infraestrutura — \
                            independente do estado de bloqueio da carteira.",
                        )
                        .color(TEXT_DIM)
                        .size(11.5),
                    );
                });
            },
        );

        ui.add_space(20.0);

        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(RichText::new("HASHRATE").color(TEXT_SECONDARY).size(10.5).extra_letter_spacing(2.0));
                    ui.add_space(8.0);
                    let rate = app.node.miner.hashrate();
                    let label = if rate == 0 {
                        "—".to_string()
                    } else if rate < 1_000 {
                        format!("{rate}  H/s")
                    } else if rate < 1_000_000 {
                        format!("{:.2}  kH/s", rate as f64 / 1_000.0)
                    } else {
                        format!("{:.2}  MH/s", rate as f64 / 1_000_000.0)
                    };
                    ui.label(RichText::new(label).color(AMBER_BRIGHT).size(34.0));

                    ui.add_space(18.0);
                    metric(ui, "RECOMPENSAS  →", &app.wallet.address(), false);
                });
            },
        );
    });
}
