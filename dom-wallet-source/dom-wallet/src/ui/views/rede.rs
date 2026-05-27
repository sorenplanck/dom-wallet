// Rede — peer registry, backbone status, sync detail.

use egui::{Layout, RichText, ScrollArea, Vec2};

use crate::ui::app::DomApp;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    section_title(ui, "Rede", Some("BACKBONE  ·  PARES  ·  SINCRONIZAÇÃO"));

    let sync = app.node.chain.sync_progress();
    let peers = app.node.peers.all();
    let alive = app.node.peers.count_alive();

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width() * 0.42, ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(RichText::new("BACKBONE").color(TEXT_SECONDARY).size(10.5).extra_letter_spacing(2.0));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(crate::net::BACKBONE_PEER)
                            .color(AMBER_BRIGHT)
                            .size(14.0)
                            .monospace(),
                    );
                    ui.add_space(18.0);
                    metric(ui, "FASE", sync.phase.label_pt(), true);
                    ui.add_space(14.0);
                    metric(ui, "ALTURA LOCAL", &format!("{}", sync.current_height), false);
                    ui.add_space(14.0);
                    metric(ui, "ALTURA ALVO", &format!("{}", sync.target_height), false);
                    ui.add_space(14.0);
                    metric(ui, "PARES ATIVOS", &format!("{alive}"), false);
                });
            },
        );

        ui.add_space(20.0);

        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(RichText::new("REGISTRO DE PARES").color(TEXT_SECONDARY).size(10.5).extra_letter_spacing(2.0));
                    ui.add_space(8.0);
                    ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        for p in peers {
                            ui.horizontal(|ui| {
                                if p.is_backbone {
                                    status_pill(ui, "BACKBONE", AMBER_BRIGHT);
                                } else {
                                    status_pill(ui, "PEER", TEXT_SECONDARY);
                                }
                                ui.add_space(8.0);
                                ui.label(RichText::new(&p.address).color(TEXT_PRIMARY).size(12.5).monospace());
                                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("h={}", p.last_height))
                                            .color(TEXT_DIM)
                                            .size(11.0)
                                            .monospace(),
                                    );
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });
                });
            },
        );
    });
}
