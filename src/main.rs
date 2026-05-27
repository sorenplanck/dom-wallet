// dom-wallet — Deterministic Monetary Network desktop client.
//
// Entry point. Boots the embedded node, wallet, and UI in a single process.
// On Windows release builds the console is hidden via the windows subsystem
// linker attribute below.

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod chain;
mod mining;
mod net;
mod node;
mod persist;
mod ui;
mod update;
mod wallet;

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::node::Node;
use crate::persist::paths::Paths;
use crate::ui::app::DomApp;
use crate::wallet::Wallet;

fn install_tracing(paths: &Paths) {
    // File logger lives in <portable>/logs/dom-wallet.log
    let log_dir = paths.logs.clone();
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender_minimal::daily(&log_dir, "dom-wallet.log");

    let env = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,dom_wallet=info"));

    let stdout_layer = fmt::layer().with_target(false).with_ansi(false).compact();
    let file_layer = fmt::layer().with_target(false).with_ansi(false).with_writer(file_appender);

    tracing_subscriber::registry()
        .with(env)
        .with(stdout_layer)
        .with(file_layer)
        .init();
}

// Tiny stand-in for tracing-appender so we don't pull another crate just for
// daily rotation. Writes append-only to <dir>/<base>.
mod tracing_appender_minimal {
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;

    pub fn daily(dir: &PathBuf, base: &str) -> FileWriter {
        let path = dir.join(base);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        FileWriter { file: Mutex::new(file) }
    }

    pub struct FileWriter {
        file: Mutex<Option<File>>,
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileWriter {
        type Writer = WriterGuard<'a>;
        fn make_writer(&'a self) -> Self::Writer {
            WriterGuard { inner: self.file.lock().unwrap() }
        }
    }

    pub struct WriterGuard<'a> {
        inner: std::sync::MutexGuard<'a, Option<File>>,
    }

    impl<'a> Write for WriterGuard<'a> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match self.inner.as_mut() {
                Some(f) => f.write(buf),
                None => Ok(buf.len()),
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            match self.inner.as_mut() {
                Some(f) => f.flush(),
                None => Ok(()),
            }
        }
    }
}

fn main() -> Result<()> {
    // Portable paths: everything lives beside the executable.
    let paths = Paths::portable_beside_executable()?;
    paths.ensure_all()?;

    install_tracing(&paths);
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "dom-wallet starting");

    // Parse CLI flags (hidden, tray, mine, etc.)
    let args: Vec<String> = std::env::args().collect();
    let hidden = args.iter().any(|a| a == "--hidden");
    let auto_mine = args.iter().any(|a| a == "--mine");

    // Build wallet (loads existing or creates a fresh encrypted container with
    // a deterministic seed). The wallet starts LOCKED — the password is needed
    // for signing and seed reveal, but not for the node runtime.
    let wallet = Arc::new(Wallet::open_or_create(&paths)?);

    // Build and start the embedded node. The node runs regardless of wallet
    // unlock state — see "Critical operational behavior" in the spec.
    let node = Arc::new(Node::new(&paths, wallet.clone())?);
    node.spawn_runtime();

    if auto_mine {
        node.set_mining(true);
    }

    // Run UI (or stay headless until the user opens the tray icon).
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1024.0, 700.0])
            .with_title("DOM Wallet")
            .with_decorations(true)
            .with_visible(!hidden),
        ..Default::default()
    };

    let node_for_ui = node.clone();
    let wallet_for_ui = wallet.clone();
    let paths_for_ui = paths.clone();

    eframe::run_native(
        "DOM Wallet",
        native_options,
        Box::new(move |cc| Box::new(DomApp::new(cc, node_for_ui, wallet_for_ui, paths_for_ui))),
    )
    .map_err(|e| anyhow::anyhow!("eframe failed: {e}"))?;

    // UI closed — stop the node cleanly.
    node.shutdown();
    tracing::info!("dom-wallet exited cleanly");
    Ok(())
}
