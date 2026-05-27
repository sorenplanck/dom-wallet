// Diagnósticos — replay snapshots and runtime logs.

use egui::{RichText, ScrollArea};

use crate::persist::snapshot::{SnapshotEvent, SnapshotLog};
use crate::ui::app::DomApp;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    section_title(
        ui,
        "Diagnósticos",
        Some("EVENTOS DE REPLAY · TIP CANÔNICO · REORGS · RETOMADA"),
    );

    panel(ui, |ui| {
        ui.label(
            RichText::new("REGISTRO DE EVENTOS")
                .color(TEXT_SECONDARY)
                .size(10.5)
                .extra_letter_spacing(2.0),
        );
        ui.add_space(8.0);
        let events = SnapshotLog::read_recent(&app.paths.snapshots, 200).unwrap_or_default();
        ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            for rec in events.iter().rev() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(rec.timestamp.format("%H:%M:%S").to_string())
                            .color(TEXT_DIM)
                            .size(11.0)
                            .monospace(),
                    );
                    ui.add_space(10.0);
                    let (label, color) = event_pretty(&rec.event);
                    ui.label(RichText::new(label).color(color).size(12.0));
                });
            }
        });
    });

    ui.add_space(14.0);

    panel(ui, |ui| {
        ui.label(
            RichText::new("LOGS DO NÓ")
                .color(TEXT_SECONDARY)
                .size(10.5)
                .extra_letter_spacing(2.0),
        );
        ui.add_space(8.0);
        let logs = app.node.recent_logs(200);
        ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
            for line in logs {
                ui.label(RichText::new(line).color(TEXT_SECONDARY).size(11.5).monospace());
            }
        });
    });
}

fn event_pretty(ev: &SnapshotEvent) -> (String, egui::Color32) {
    match ev {
        SnapshotEvent::BootStart { version } => (format!("BOOT START — v{version}"), AMBER),
        SnapshotEvent::BootComplete { wallet_address, height } => {
            (format!("BOOT COMPLETE — h={height}  addr={}", short(wallet_address)), SUCCESS)
        }
        SnapshotEvent::CanonicalTipAdvanced { from, to, block_hash } => {
            (format!("TIP {from} → {to}  hash={}", &block_hash[..16.min(block_hash.len())]), AMBER_BRIGHT)
        }
        SnapshotEvent::Reorg { depth, new_tip } => {
            (format!("REORG depth={depth}  tip={}", &new_tip[..16.min(new_tip.len())]), ERROR)
        }
        SnapshotEvent::IbdResumed { from_height } => (format!("IBD RESUMED from h={from_height}"), AMBER),
        SnapshotEvent::IbdComplete { final_height } => (format!("IBD COMPLETE h={final_height}"), SUCCESS),
        SnapshotEvent::PeerRotation { added, removed } => {
            (format!("PEER ROTATION  +{added}  -{removed}"), TEXT_SECONDARY)
        }
        SnapshotEvent::MempoolReconciled { added, evicted } => {
            (format!("MEMPOOL RECONCILED  +{added}  -{evicted}"), TEXT_SECONDARY)
        }
        SnapshotEvent::RestartRecovery { last_known_height } => {
            (format!("RESTART RECOVERY  last_h={last_known_height}"), AMBER)
        }
        SnapshotEvent::MiningStarted => ("MINING STARTED".to_string(), AMBER_BRIGHT),
        SnapshotEvent::MiningStopped => ("MINING STOPPED".to_string(), TEXT_DIM),
        SnapshotEvent::Shutdown => ("SHUTDOWN".to_string(), TEXT_DIM),
    }
}

fn short(s: &str) -> String {
    if s.len() > 14 {
        format!("{}…", &s[..14])
    } else {
        s.to_string()
    }
}
