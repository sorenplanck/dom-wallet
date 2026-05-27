// Configurações — auto-start, hidden launch, update check, advanced.

use egui::{Layout, RichText, Vec2};

use crate::persist::runtime_state::RuntimeState;
use crate::ui::app::DomApp;
use crate::ui::theme::*;
use crate::ui::views::common::*;

pub fn render(app: &mut DomApp, ui: &mut egui::Ui) {
    section_title(ui, "Configurações", Some("EXECUÇÃO  ·  ATUALIZAÇÕES  ·  AVANÇADO"));

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width() * 0.5, ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(
                        RichText::new("EXECUÇÃO")
                            .color(TEXT_SECONDARY)
                            .size(10.5)
                            .extra_letter_spacing(2.0),
                    );
                    ui.add_space(10.0);

                    let mut rs = RuntimeState::load(&app.paths.runtime).unwrap_or_default();
                    let mut hidden = rs.auto_start_hidden;
                    if ui.checkbox(&mut hidden, "Iniciar oculto com o Windows").changed() {
                        rs.auto_start_hidden = hidden;
                        let _ = rs.save(&app.paths.runtime);
                        crate::ui::app::write_autostart_flag(&app.paths, hidden);
                    }
                    ui.label(
                        RichText::new(
                            "O nó executa em segundo plano com bandeja do sistema. \
                            A carteira permanece bloqueada até autenticação manual.",
                        )
                        .color(TEXT_DIM)
                        .size(11.5),
                    );

                    ui.add_space(14.0);
                    let mut auto_mine = rs.auto_mine;
                    if ui.checkbox(&mut auto_mine, "Iniciar mineração automaticamente").changed() {
                        rs.auto_mine = auto_mine;
                        let _ = rs.save(&app.paths.runtime);
                        app.node.set_mining(auto_mine);
                    }
                });
            },
        );

        ui.add_space(20.0);

        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), ui.available_height()),
            Layout::top_down(egui::Align::LEFT),
            |ui| {
                panel(ui, |ui| {
                    ui.label(
                        RichText::new("ATUALIZAÇÕES")
                            .color(TEXT_SECONDARY)
                            .size(10.5)
                            .extra_letter_spacing(2.0),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("Versão atual  ·  v{}", env!("CARGO_PKG_VERSION")))
                            .color(TEXT_PRIMARY)
                            .size(13.0),
                    );
                    ui.add_space(10.0);
                    if tactile_button(ui, "Verificar agora", false) {
                        app.trigger_update_check();
                    }
                    if let Some(status) = &app.update_status {
                        ui.add_space(8.0);
                        ui.label(RichText::new(status).color(TEXT_SECONDARY).size(11.5));
                    }
                });
            },
        );
    });

    ui.add_space(16.0);

    panel(ui, |ui| {
        ui.label(
            RichText::new("DIRETÓRIO PORTÁVEL")
                .color(TEXT_SECONDARY)
                .size(10.5)
                .extra_letter_spacing(2.0),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(app.paths.root.display().to_string())
                .color(AMBER)
                .size(12.0)
                .monospace(),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Todo o estado do runtime — cadeia, carteira, pares, instantâneos — \
                vive ao lado do executável. Não há dependência de AppData.",
            )
            .color(TEXT_DIM)
            .size(11.5),
        );
    });
}
