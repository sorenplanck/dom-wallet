// Embedded node. Orchestrates:
//   - chain state and persistence
//   - peer registry and backbone connection
//   - mining (optional)
//   - replay snapshot log
//
// The node owns a dedicated tokio runtime so the UI thread (eframe/egui)
// remains responsive. The node's lifecycle is independent of the wallet
// unlock state — it runs as monetary infrastructure.

use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::chain::state::{ChainState, SyncPhase, SyncProgress};
use crate::mining::Miner;
use crate::net::p2p::P2pClient;
use crate::net::peer::PeerRegistry;
use crate::persist::paths::Paths;
use crate::persist::runtime_state::RuntimeState;
use crate::persist::snapshot::{SnapshotEvent, SnapshotLog};
use crate::wallet::Wallet;

pub struct Node {
    pub paths: Paths,
    pub chain: Arc<ChainState>,
    pub peers: Arc<PeerRegistry>,
    pub miner: Arc<Miner>,
    pub snapshots: Arc<SnapshotLog>,
    pub wallet: Arc<Wallet>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    rt_handle: Mutex<Option<tokio::runtime::Runtime>>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    log_buffer: Arc<parking_lot::RwLock<std::collections::VecDeque<String>>>,
}

impl Node {
    pub fn new(paths: &Paths, wallet: Arc<Wallet>) -> Result<Self> {
        let chain = ChainState::open(paths.chain.clone())?;
        let peers = PeerRegistry::open(&paths.peers)?;
        let snapshots = Arc::new(SnapshotLog::open(&paths.snapshots)?);

        snapshots.record(SnapshotEvent::BootStart {
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let runtime_state = RuntimeState::load(&paths.runtime).unwrap_or_default();
        if !runtime_state.last_clean_shutdown && runtime_state.last_height > 0 {
            snapshots.record(SnapshotEvent::RestartRecovery {
                last_known_height: runtime_state.last_height,
            });
        }

        let miner = Miner::new(chain.clone(), wallet.address(), snapshots.clone());

        // Honor persisted auto-mine flag.
        if runtime_state.auto_mine {
            miner.set_enabled(true);
        }

        let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);

        // Initial sync state.
        chain.set_sync(SyncProgress {
            current_height: chain.height(),
            target_height: chain.height(),
            phase: SyncPhase::Connecting,
            peer_source: None,
        });

        snapshots.record(SnapshotEvent::BootComplete {
            wallet_address: wallet.address(),
            height: chain.height(),
        });

        Ok(Self {
            paths: paths.clone(),
            chain,
            peers,
            miner,
            snapshots,
            wallet,
            started_at: chrono::Utc::now(),
            rt_handle: Mutex::new(None),
            shutdown_tx,
            log_buffer: Arc::new(parking_lot::RwLock::new(std::collections::VecDeque::with_capacity(500))),
        })
    }

    /// Spawn the long-running tokio runtime that drives the node.
    pub fn spawn_runtime(self: &Arc<Self>) {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("dom-node")
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!(error = %e, "failed to build tokio runtime");
                return;
            }
        };

        let chain = self.chain.clone();
        let peers = self.peers.clone();
        let miner = self.miner.clone();
        let wallet_addr = self.wallet.address();
        let snapshots = self.snapshots.clone();
        let log_buffer = self.log_buffer.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        let shutdown_rx_for_miner = self.shutdown_tx.subscribe();
        let shutdown_rx_for_sync = self.shutdown_tx.subscribe();

        rt.spawn(async move {
            push_log(&log_buffer, "Node runtime starting");
            let client = Arc::new(P2pClient::new(chain.clone(), peers.clone(), wallet_addr));
            let _ = client.supervisor(shutdown_rx).await;
        });

        let miner_clone = miner.clone();
        rt.spawn(async move {
            miner_clone.run(shutdown_rx_for_miner).await;
        });

        // Sync simulator — updates phase based on peer activity. In v0 this
        // reflects reachability of the backbone; a real header-sync engine
        // replaces it.
        let chain_for_sync = self.chain.clone();
        let peers_for_sync = self.peers.clone();
        let snapshots_for_sync = snapshots.clone();
        let log_buffer_for_sync = log_buffer.clone();
        rt.spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            let mut last_phase = SyncPhase::Connecting;
            loop {
                interval.tick().await;
                if *shutdown_rx_for_sync.borrow() {
                    return;
                }
                let alive = peers_for_sync.count_alive();
                let height = chain_for_sync.height();
                let target = chain_for_sync.sync_progress().target_height.max(height);
                let phase = if alive == 0 {
                    SyncPhase::Connecting
                } else if height < target {
                    SyncPhase::CatchingUp
                } else {
                    SyncPhase::Live
                };
                if phase != last_phase {
                    push_log(
                        &log_buffer_for_sync,
                        &format!("sync phase → {}", phase.label_pt()),
                    );
                    if matches!(phase, SyncPhase::Live) && matches!(last_phase, SyncPhase::CatchingUp | SyncPhase::InitialBlockDownload) {
                        snapshots_for_sync.record(SnapshotEvent::IbdComplete { final_height: height });
                    }
                    last_phase = phase;
                }
                chain_for_sync.set_sync(SyncProgress {
                    current_height: height,
                    target_height: target,
                    phase,
                    peer_source: Some(crate::net::BACKBONE_PEER.to_string()),
                });
            }
        });

        *self.rt_handle.lock() = Some(rt);
    }

    pub fn set_mining(&self, on: bool) {
        self.miner.set_enabled(on);
        // Persist preference.
        let mut rs = RuntimeState::load(&self.paths.runtime).unwrap_or_default();
        rs.auto_mine = on;
        let _ = rs.save(&self.paths.runtime);
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        // Mark clean shutdown.
        let mut rs = RuntimeState::load(&self.paths.runtime).unwrap_or_default();
        rs.last_height = self.chain.height();
        rs.last_tip_hash = self.chain.tip_hash_hex();
        rs.last_clean_shutdown = true;
        let _ = rs.save(&self.paths.runtime);
        let _ = self.peers.save(&self.paths.peers);
        let _ = self.chain.persist_blocks();
        self.snapshots.record(SnapshotEvent::Shutdown);

        // Drop the runtime so background tasks exit.
        if let Some(rt) = self.rt_handle.lock().take() {
            rt.shutdown_background();
        }
    }

    pub fn recent_logs(&self, max: usize) -> Vec<String> {
        let g = self.log_buffer.read();
        g.iter().rev().take(max).cloned().collect::<Vec<_>>()
    }
}

fn push_log(buf: &parking_lot::RwLock<std::collections::VecDeque<String>>, line: &str) {
    let stamp = chrono::Utc::now().format("%H:%M:%S");
    let entry = format!("{stamp}  {line}");
    let mut g = buf.write();
    if g.len() >= 500 {
        g.pop_front();
    }
    g.push_back(entry);
}
