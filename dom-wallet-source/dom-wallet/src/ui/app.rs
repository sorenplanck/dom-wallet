// DomApp — central eframe app. Owns the sidebar selection, modal state,
// and references to node + wallet + paths. The actual rendering is delegated
// to the sidebar and views.

use std::sync::Arc;
use std::time::Instant;

use egui::{Color32, Rounding, Stroke, Vec2};

use crate::chain::tx::BASE_UNITS_PER_DOM;
use crate::node::Node;
use crate::persist::paths::Paths;
use crate::ui::sidebar::Section;
use crate::ui::theme;
use crate::ui::views;
use crate::wallet::Wallet;

pub struct DomApp {
    pub node: Arc<Node>,
    pub wallet: Arc<Wallet>,
    pub paths: Paths,
    section: Section,
    started: Instant,
    pub modal: Modal,
    pub notification: Option<(String, Instant)>,
    pub update_status: Option<String>,
}

#[derive(Default, Clone)]
pub enum Modal {
    #[default]
    None,
    Unlock { password: String, error: Option<String> },
    SetInitialPassword { password: String, confirm: String, error: Option<String> },
    ChangePassword { current: String, new_pw: String, confirm: String, error: Option<String> },
    Send { to: String, amount: String, fee: String, memo: String, error: Option<String>, last_tx: Option<String> },
    Receive,
    RevealSeed { seed: Option<String> },
}

impl DomApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        node: Arc<Node>,
        wallet: Arc<Wallet>,
        paths: Paths,
    ) -> Self {
        theme::install(&cc.egui_ctx);
        Self {
            node,
            wallet,
            paths,
            section: Section::Inicio,
            started: Instant::now(),
            modal: Modal::None,
            notification: None,
            update_status: None,
        }
    }

    pub fn time_seconds(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.notification = Some((msg.into(), Instant::now()));
    }

    pub fn open_unlock_modal(&mut self) {
        self.modal = Modal::Unlock { password: String::new(), error: None };
    }
    pub fn open_change_password(&mut self) {
        self.modal = Modal::ChangePassword {
            current: String::new(),
            new_pw: String::new(),
            confirm: String::new(),
            error: None,
        };
    }
    pub fn open_reveal_seed(&mut self) {
        let seed = self.wallet.reveal_mnemonic().ok();
        self.modal = Modal::RevealSeed { seed };
    }
    pub fn open_send_modal(&mut self) {
        if !self.wallet.is_unlocked() {
            self.modal = Modal::Unlock { password: String::new(), error: None };
            return;
        }
        self.modal = Modal::Send {
            to: String::new(),
            amount: String::new(),
            fee: "0.0001".to_string(),
            memo: String::new(),
            error: None,
            last_tx: None,
        };
    }
    pub fn open_receive_modal(&mut self) {
        self.modal = Modal::Receive;
    }

    pub fn trigger_update_check(&mut self) {
        self.update_status = Some("Verificando…".to_string());
        // Fire-and-forget on a blocking thread. egui repaints on next frame.
        let _ = std::thread::Builder::new().name("update-check".into()).spawn(|| {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };
            let _ = rt.block_on(async move {
                let _ = crate::update::check_for_update().await;
            });
        });
    }
}

impl eframe::App for DomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));

        // Left sidebar.
        egui::SidePanel::left("sidebar")
            .exact_width(220.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(0x05, 0x06, 0x09))
                    .stroke(Stroke::new(0.0, Color32::TRANSPARENT)),
            )
            .show(ctx, |ui| {
                // Vertical separator on the right edge.
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [
                        egui::pos2(r.right(), r.top()),
                        egui::pos2(r.right(), r.bottom()),
                    ],
                    Stroke::new(1.0, theme::BORDER_SOFT),
                );
                crate::ui::sidebar::render(ui, &mut self.section);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG_PRIMARY))
            .show(ctx, |ui| match self.section {
                Section::Inicio => views::inicio::render(self, ui),
                Section::Carteira => views::carteira::render(self, ui),
                Section::Transacoes => views::transacoes::render(self, ui),
                Section::Rede => views::rede::render(self, ui),
                Section::Mineracao => views::mineracao::render(self, ui),
                Section::Diagnosticos => views::diagnosticos::render(self, ui),
                Section::Configuracoes => views::configuracoes::render(self, ui),
            });

        self.render_modal(ctx);
        self.render_notification(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.node.shutdown();
    }
}

impl DomApp {
    fn render_modal(&mut self, ctx: &egui::Context) {
        let modal = std::mem::take(&mut self.modal);
        self.modal = match modal {
            Modal::None => Modal::None,
            Modal::Unlock { mut password, mut error } => {
                let mut close = false;
                let mut next = None;
                modal_frame(ctx, "Desbloquear carteira", |ui| {
                    ui.label(
                        egui::RichText::new("Insira a senha da carteira.")
                            .color(theme::TEXT_SECONDARY)
                            .size(12.5),
                    );
                    ui.add_space(10.0);
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut password)
                            .password(true)
                            .hint_text("senha")
                            .desired_width(360.0),
                    );
                    if let Some(err) = &error {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(err).color(theme::ERROR).size(11.5));
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if crate::ui::views::common::tactile_button(ui, "Cancelar", false) {
                            close = true;
                        }
                        ui.add_space(8.0);
                        let do_submit = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if crate::ui::views::common::tactile_button(ui, "Desbloquear", true) || do_submit {
                            match self.wallet.unlock(&password) {
                                Ok(()) => {
                                    self.notify("Carteira desbloqueada");
                                    close = true;
                                }
                                Err(e) => {
                                    error = Some(e.to_string());
                                }
                            }
                        }
                    });
                });
                if close {
                    Modal::None
                } else if let Some(n) = next {
                    n
                } else {
                    Modal::Unlock { password, error }
                }
            }
            Modal::SetInitialPassword { password, confirm, error } => {
                // Placeholder — flow used when no password is set yet.
                let _ = (password, confirm, error);
                Modal::None
            }
            Modal::ChangePassword { mut current, mut new_pw, mut confirm, mut error } => {
                let mut close = false;
                modal_frame(ctx, "Alterar senha", |ui| {
                    ui.label(
                        egui::RichText::new("A nova senha protege apenas operações da carteira.")
                            .color(theme::TEXT_SECONDARY)
                            .size(12.0),
                    );
                    ui.add_space(10.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut current)
                            .password(true)
                            .hint_text("senha atual")
                            .desired_width(360.0),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut new_pw)
                            .password(true)
                            .hint_text("nova senha")
                            .desired_width(360.0),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut confirm)
                            .password(true)
                            .hint_text("confirmar nova senha")
                            .desired_width(360.0),
                    );
                    if let Some(err) = &error {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(err).color(theme::ERROR).size(11.5));
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if crate::ui::views::common::tactile_button(ui, "Cancelar", false) {
                            close = true;
                        }
                        ui.add_space(8.0);
                        if crate::ui::views::common::tactile_button(ui, "Aplicar", true) {
                            if new_pw != confirm {
                                error = Some("Senhas não coincidem.".to_string());
                            } else if self.wallet.unlock(&current).is_err() {
                                error = Some("Senha atual incorreta.".to_string());
                            } else {
                                match self.wallet.change_password(&new_pw) {
                                    Ok(()) => {
                                        self.notify("Senha atualizada");
                                        close = true;
                                    }
                                    Err(e) => error = Some(e.to_string()),
                                }
                            }
                        }
                    });
                });
                if close {
                    Modal::None
                } else {
                    Modal::ChangePassword { current, new_pw, confirm, error }
                }
            }
            Modal::Send { mut to, mut amount, mut fee, mut memo, mut error, mut last_tx } => {
                let mut close = false;
                modal_frame(ctx, "Enviar DOM", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut to)
                            .hint_text("endereço de destino")
                            .desired_width(420.0),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut amount)
                            .hint_text("quantia (ex.: 1,2500)")
                            .desired_width(220.0),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut fee)
                            .hint_text("taxa (DOM)")
                            .desired_width(220.0),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut memo)
                            .hint_text("memo (opcional)")
                            .desired_width(420.0),
                    );
                    if let Some(err) = &error {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(err).color(theme::ERROR).size(11.5));
                    }
                    if let Some(tx) = &last_tx {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("Enviada · txid {}…", &tx[..16.min(tx.len())]))
                                .color(theme::SUCCESS)
                                .size(11.5)
                                .monospace(),
                        );
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if crate::ui::views::common::tactile_button(ui, "Cancelar", false) {
                            close = true;
                        }
                        ui.add_space(8.0);
                        if crate::ui::views::common::tactile_button(ui, "Assinar e enviar", true) {
                            match self.try_send(&to, &amount, &fee, &memo) {
                                Ok(tx_id) => {
                                    last_tx = Some(tx_id);
                                    error = None;
                                    self.notify("Transação na mempool");
                                }
                                Err(e) => error = Some(e.to_string()),
                            }
                        }
                    });
                });
                if close {
                    Modal::None
                } else {
                    Modal::Send { to, amount, fee, memo, error, last_tx }
                }
            }
            Modal::Receive => {
                let mut close = false;
                modal_frame(ctx, "Receber DOM", |ui| {
                    ui.label(
                        egui::RichText::new("Seu endereço persistente")
                            .color(theme::TEXT_SECONDARY)
                            .size(11.5)
                            .extra_letter_spacing(1.5),
                    );
                    ui.add_space(8.0);
                    let addr = self.wallet.address();
                    ui.label(
                        egui::RichText::new(&addr)
                            .color(theme::AMBER_BRIGHT)
                            .size(15.0)
                            .monospace(),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if crate::ui::views::common::tactile_button(ui, "Copiar", true) {
                            ui.ctx().copy_text(addr);
                            self.notify("Endereço copiado");
                        }
                        ui.add_space(8.0);
                        if crate::ui::views::common::tactile_button(ui, "Fechar", false) {
                            close = true;
                        }
                    });
                });
                if close { Modal::None } else { Modal::Receive }
            }
            Modal::RevealSeed { seed } => {
                let mut close = false;
                modal_frame(ctx, "Semente determinística", |ui| {
                    match &seed {
                        Some(phrase) => {
                            ui.label(
                                egui::RichText::new(
                                    "Guarde estas palavras em local seguro. Quem as tiver controla a carteira.",
                                )
                                .color(theme::AMBER)
                                .size(11.5),
                            );
                            ui.add_space(10.0);
                            egui::Frame::none()
                                .fill(theme::BG_PANEL_RAISED)
                                .stroke(Stroke::new(1.0, theme::BORDER_SOFT))
                                .rounding(Rounding::same(6.0))
                                .inner_margin(egui::Margin::same(12.0))
                                .show(ui, |ui| {
                                    ui.set_max_width(440.0);
                                    ui.label(
                                        egui::RichText::new(phrase)
                                            .color(theme::TEXT_PRIMARY)
                                            .size(13.5)
                                            .monospace(),
                                    );
                                });
                        }
                        None => {
                            ui.label(
                                egui::RichText::new("Carteira bloqueada.")
                                    .color(theme::ERROR)
                                    .size(12.0),
                            );
                        }
                    }
                    ui.add_space(14.0);
                    if crate::ui::views::common::tactile_button(ui, "Fechar", true) {
                        close = true;
                    }
                });
                if close { Modal::None } else { Modal::RevealSeed { seed } }
            }
        };
    }

    fn try_send(&self, to: &str, amount: &str, fee: &str, memo: &str) -> anyhow::Result<String> {
        if to.is_empty() || !to.starts_with("dom1") {
            anyhow::bail!("endereço inválido");
        }
        let amount_units = crate::wallet::parse_dom_amount(amount)?;
        let fee_units = crate::wallet::parse_dom_amount(fee)?;
        if amount_units == 0 {
            anyhow::bail!("quantia deve ser maior que zero");
        }
        let nonce = self.node.chain.nonce_of(&self.wallet.address());
        let memo_opt = if memo.trim().is_empty() {
            None
        } else {
            Some(memo.to_string())
        };
        let tx = self.wallet.sign_transaction(
            to,
            amount_units,
            fee_units,
            nonce,
            memo_opt,
            crate::chain::state::DOM_CHAIN_ID,
        )?;
        let tx_id = tx.id_hex();
        self.node.chain.submit_local_tx(tx);
        // Suppress unused warning for the constant in DEVNET v0 where the
        // balance display path uses it.
        let _ = BASE_UNITS_PER_DOM;
        Ok(tx_id)
    }

    fn render_notification(&mut self, ctx: &egui::Context) {
        if let Some((msg, when)) = self.notification.clone() {
            let elapsed = when.elapsed().as_secs_f32();
            if elapsed > 4.0 {
                self.notification = None;
                return;
            }
            let alpha = if elapsed < 0.2 {
                (elapsed / 0.2 * 255.0) as u8
            } else if elapsed > 3.6 {
                (((4.0 - elapsed) / 0.4).clamp(0.0, 1.0) * 255.0) as u8
            } else {
                255
            };
            let area_id = egui::Id::new("dom-notification");
            egui::Area::new(area_id)
                .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -32.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(0x12, 0x16, 0x1D, alpha))
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha),
                        ))
                        .rounding(Rounding::same(999.0))
                        .inner_margin(egui::Margin::symmetric(18.0, 10.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(msg)
                                    .color(Color32::from_rgba_unmultiplied(0xF0, 0xC6, 0x74, alpha))
                                    .size(12.5)
                                    .extra_letter_spacing(1.3),
                            );
                        });
                });
        }
    }
}

fn modal_frame(ctx: &egui::Context, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    // Dim background.
    egui::Area::new(egui::Id::new("dom-modal-dim"))
        .interactable(true)
        .order(egui::Order::Middle)
        .anchor(egui::Align2::LEFT_TOP, Vec2::ZERO)
        .show(ctx, |ui| {
            let screen = ui.ctx().screen_rect();
            ui.painter().rect_filled(
                screen,
                Rounding::ZERO,
                Color32::from_black_alpha(180),
            );
        });

    egui::Area::new(egui::Id::new("dom-modal"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::BG_PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER_SOFT))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(24.0))
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 16.0),
                    blur: 48.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(220),
                })
                .show(ui, |ui| {
                    ui.set_min_width(460.0);
                    ui.label(
                        egui::RichText::new(title)
                            .color(theme::AMBER_BRIGHT)
                            .size(16.0)
                            .extra_letter_spacing(1.5),
                    );
                    ui.add_space(4.0);
                    ui.painter().line_segment(
                        [
                            egui::pos2(ui.cursor().left_top().x, ui.cursor().left_top().y),
                            egui::pos2(ui.cursor().left_top().x + 460.0, ui.cursor().left_top().y),
                        ],
                        Stroke::new(1.0, theme::BORDER_SOFT),
                    );
                    ui.add_space(14.0);
                    body(ui);
                });
        });
}

/// Persist the auto-start preference flag. Actual registry key registration
/// happens on Windows in a follow-up; in v0 we record the intent so the
/// Settings panel reflects user choice across runs.
pub fn write_autostart_flag(paths: &Paths, hidden: bool) {
    let flag = paths.config.join("autostart.flag");
    if hidden {
        let _ = std::fs::write(flag, "1");
    } else {
        let _ = std::fs::remove_file(flag);
    }
}
