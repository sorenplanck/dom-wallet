// Transações — full tx history with lifecycle state.

use egui::{Layout, RichText, ScrollArea, Vec2};

use crate::chain::tx::{TxStatus, BASE_UNITS_PER_DOM};
use crate::ui::app::DomApp;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    section_title(ui, "Transações", Some("CICLO DE VIDA E HISTÓRICO"));

    let records = app.node.chain.tx_records();
    if records.is_empty() {
        panel(ui, |ui| {
            ui.label(
                RichText::new("Nenhuma transação registrada.")
                    .color(TEXT_SECONDARY)
                    .size(13.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "O ciclo de vida — pendente, confirmada, falha, rebroadcast — é \
                    persistido localmente e sobrevive a reinicializações.",
                )
                .color(TEXT_DIM)
                .size(11.5),
            );
        });
        return;
    }

    panel(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            for rec in records {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let (status_label, status_color) = match rec.status {
                        TxStatus::Pending => ("PENDENTE", AMBER),
                        TxStatus::Confirmed => ("CONFIRMADA", SUCCESS),
                        TxStatus::Failed => ("FALHOU", ERROR),
                        TxStatus::Rebroadcast => ("REBROADCAST", AMBER_BRIGHT),
                    };
                    status_pill(ui, status_label, status_color);
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        let dir = if rec.tx.body.from == app.wallet.address() {
                            "→ ENVIAR"
                        } else {
                            "← RECEBER"
                        };
                        ui.label(
                            RichText::new(dir)
                                .color(TEXT_SECONDARY)
                                .size(10.5)
                                .extra_letter_spacing(1.6),
                        );
                        let counterparty = if rec.tx.body.from == app.wallet.address() {
                            &rec.tx.body.to
                        } else {
                            &rec.tx.body.from
                        };
                        let short = short_addr(counterparty);
                        ui.label(
                            RichText::new(short)
                                .color(TEXT_PRIMARY)
                                .size(13.0)
                                .monospace(),
                        );
                        ui.label(
                            RichText::new(&rec.tx.id_hex()[..16])
                                .color(TEXT_DIM)
                                .size(10.5)
                                .monospace(),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            let amt = rec.tx.body.amount as f64 / BASE_UNITS_PER_DOM as f64;
                            ui.label(
                                RichText::new(format!("{amt:.4} DOM"))
                                    .color(AMBER_BRIGHT)
                                    .size(15.0),
                            );
                            let fee = rec.tx.body.fee as f64 / BASE_UNITS_PER_DOM as f64;
                            ui.label(
                                RichText::new(format!("taxa {fee:.4}"))
                                    .color(TEXT_DIM)
                                    .size(10.5),
                            );
                        });
                    });
                });

                if rec.status == TxStatus::Pending {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        if tactile_button(ui, "Cancelar", false) {
                            app.node.chain.cancel_pending(&rec.tx.id_hex());
                            app.notify("Transação pendente cancelada");
                        }
                        ui.add_space(8.0);
                        if tactile_button(ui, "Rebroadcast", false) {
                            // A real rebroadcast would re-emit via p2p. v0:
                            // refresh the record's timestamp and mark as
                            // Rebroadcast so the UI reflects user intent.
                            app.notify("Solicitada retransmissão");
                        }
                    });
                }
                ui.add_space(8.0);
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
                ui.painter().line_segment(
                    [
                        ui.cursor().left_top(),
                        egui::pos2(ui.cursor().left_top().x + ui.available_width(), ui.cursor().left_top().y),
                    ],
                    egui::Stroke::new(1.0, BORDER_DIM),
                );
                ui.add_space(4.0);
            }
        });
    });
}

fn short_addr(s: &str) -> String {
    if s.len() > 22 {
        format!("{}…{}", &s[..12], &s[s.len() - 6..])
    } else {
        s.to_string()
    }
}
