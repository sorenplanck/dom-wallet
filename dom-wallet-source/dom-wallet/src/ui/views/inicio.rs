// Início — the home screen. Composition per spec:
//
//   ┌──────────────────────────────────────────────┬──────────────┐
//   │  HERO  (water, halo, coin, philosophy text)  │  STATUS COL  │
//   │                                              │              │
//   │                                              │              │
//   │              [ balance display ]             │              │
//   │              [ Enviar ] [ Receber ]          │              │
//   └──────────────────────────────────────────────┴──────────────┘

use egui::{Align, Color32, Layout, RichText, Sense, Stroke, Vec2};

use crate::chain::tx::{format_dom, BASE_UNITS_PER_DOM};
use crate::ui::app::DomApp;
use crate::ui::hero;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    let total = ui.available_size();
    let status_width = 280.0_f32.min(total.x * 0.28);
    let hero_width = total.x - status_width;

    ui.horizontal_top(|ui| {
        // Hero column (left).
        ui.allocate_ui_with_layout(
            Vec2::new(hero_width, total.y),
            Layout::top_down(Align::LEFT),
            |ui| {
                render_hero_column(app, ui);
            },
        );

        // Status column (right).
        ui.allocate_ui_with_layout(
            Vec2::new(status_width, total.y),
            Layout::top_down(Align::LEFT),
            |ui| {
                render_status_column(app, ui);
            },
        );
    });
}

fn render_hero_column(app: &mut DomApp, ui: &mut egui::Ui) {
    let total = ui.available_size();
    let hero_h = total.y * 0.66;
    let bottom_h = total.y - hero_h;

    // Hero artwork.
    ui.allocate_ui_with_layout(
        Vec2::new(total.x, hero_h),
        Layout::top_down(Align::LEFT),
        |ui| {
            hero::render(ui, app.time_seconds());
        },
    );

    // Bottom: balance + buttons inside a subtle panel that bleeds into the
    // hero atmosphere.
    ui.allocate_ui_with_layout(
        Vec2::new(total.x, bottom_h),
        Layout::top_down(Align::Center),
        |ui| {
            render_balance_and_actions(app, ui);
        },
    );
}

fn render_balance_and_actions(app: &mut DomApp, ui: &mut egui::Ui) {
    ui.add_space(20.0);
    let balance_units = app.node.chain.balance_of(&app.wallet.address());
    // Display: real balance if non-zero, otherwise the placeholder spec
    // figure so the design intent is visible on a fresh DEVNET install.
    let display_units = if balance_units == 0 {
        // 3.482,2456 DOM → 3482 * 1e7 + (2456 * 1e3)
        3_482u64 * BASE_UNITS_PER_DOM + 2_456_000
    } else {
        balance_units
    };
    let formatted = format_dom(display_units);

    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(format!("{formatted} DOM"))
                .color(TEXT_PRIMARY)
                .size(44.0),
        );
        ui.add_space(4.0);
        // Approximate fiat — DEVNET placeholder; a real oracle replaces this.
        let dom_value = display_units as f64 / BASE_UNITS_PER_DOM as f64;
        let usd = dom_value * 12.40;
        ui.label(
            RichText::new(format!("≈ US$ {usd:.2}"))
                .color(TEXT_SECONDARY)
                .size(13.0),
        );
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            let spacer = (ui.available_width() - 140.0 * 2.0 - 14.0).max(0.0) / 2.0;
            ui.add_space(spacer);
            if tactile_button(ui, "Enviar", true) {
                app.open_send_modal();
            }
            ui.add_space(14.0);
            if tactile_button(ui, "Receber", false) {
                app.open_receive_modal();
            }
        });
    });
}

fn render_status_column(app: &mut DomApp, ui: &mut egui::Ui) {
    // Background fill the full column with a soft side panel color and a
    // single thin left border separating it from the hero.
    let column_rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(
        column_rect,
        egui::Rounding::ZERO,
        Color32::from_rgb(0x08, 0x0A, 0x0F),
    );
    ui.painter().line_segment(
        [
            column_rect.left_top(),
            column_rect.left_bottom(),
        ],
        Stroke::new(1.0, BORDER_SOFT),
    );

    ui.add_space(28.0);
    ui.indent("status_col", |ui| {
        ui.set_max_width(ui.available_width() - 24.0);

        // BACKBONE state
        let peer_count = app.node.peers.count_alive();
        let backbone_connected = peer_count > 0;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("BACKBONE")
                    .color(TEXT_SECONDARY)
                    .size(10.5)
                    .extra_letter_spacing(2.0),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if backbone_connected {
                    status_pill(ui, "CONNECTED", SUCCESS);
                } else {
                    status_pill(ui, "RECONNECTING", AMBER);
                }
            });
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(crate::net::BACKBONE_PEER)
                .color(TEXT_DIM)
                .size(11.0)
                .monospace(),
        );

        ui.add_space(24.0);
        thin_rule(ui);
        ui.add_space(20.0);

        let sync = app.node.chain.sync_progress();
        let height = app.node.chain.height();

        metric(ui, "ALTURA DA CADEIA", &format!("{height}"), false);
        ui.add_space(18.0);

        let frac = if sync.target_height == 0 || sync.target_height == sync.current_height {
            1.0
        } else {
            sync.current_height as f32 / sync.target_height as f32
        };
        amber_progress(ui, frac, "SINCRONIZAÇÃO");
        ui.add_space(6.0);
        ui.label(
            RichText::new(sync.phase.label_pt())
                .color(TEXT_SECONDARY)
                .size(11.0),
        );

        ui.add_space(22.0);
        metric(ui, "PARES", &format!("{peer_count}"), false);

        ui.add_space(22.0);
        let mining_on = app.node.miner.is_enabled();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("MINERAÇÃO")
                    .color(TEXT_SECONDARY)
                    .size(10.5)
                    .extra_letter_spacing(2.0),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mining_on {
                    status_pill(ui, "ATIVA", AMBER_BRIGHT);
                } else {
                    status_pill(ui, "INATIVA", TEXT_DIM);
                }
            });
        });
        ui.add_space(8.0);
        let rate = app.node.miner.hashrate();
        let rate_label = format_rate(rate);
        ui.label(
            RichText::new(rate_label)
                .color(TEXT_PRIMARY)
                .size(15.0),
        );

        ui.add_space(28.0);
        thin_rule(ui);
        ui.add_space(16.0);
        ui.label(
            RichText::new("ENDEREÇO")
                .color(TEXT_SECONDARY)
                .size(10.5)
                .extra_letter_spacing(2.0),
        );
        ui.add_space(4.0);
        let addr = app.wallet.address();
        let short = if addr.len() > 22 {
            format!("{}…{}", &addr[..12], &addr[addr.len() - 6..])
        } else {
            addr.clone()
        };
        ui.label(
            RichText::new(short)
                .color(AMBER)
                .size(12.0)
                .monospace(),
        );
        ui.add_space(6.0);
        let (_, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
        if r.hovered() {
            ui.painter().rect_filled(
                r,
                egui::Rounding::same(4.0),
                Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, 18),
            );
        }
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            "copiar endereço",
            egui::FontId::proportional(11.0),
            if r.hovered() { AMBER_BRIGHT } else { TEXT_SECONDARY },
        );
        if r.clicked() {
            ui.ctx().copy_text(addr);
        }
    });
}

fn format_rate(h: u64) -> String {
    if h == 0 {
        "—".to_string()
    } else if h < 1_000 {
        format!("{h} H/s")
    } else if h < 1_000_000 {
        format!("{:.2} kH/s", h as f64 / 1_000.0)
    } else {
        format!("{:.2} MH/s", h as f64 / 1_000_000.0)
    }
}
