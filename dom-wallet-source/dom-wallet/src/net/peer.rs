// Peer registry. Persistent list of known peers with simple health tracking.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub address: String,
    pub first_seen: i64,
    pub last_seen: i64,
    pub last_height: u64,
    pub successful_connections: u64,
    pub failed_attempts: u64,
    pub is_backbone: bool,
}

#[derive(Default)]
pub struct PeerRegistry {
    inner: RwLock<PeerRegistryInner>,
}

#[derive(Default)]
struct PeerRegistryInner {
    peers: HashMap<String, PeerInfo>,
}

impl PeerRegistry {
    pub fn open(dir: &Path) -> Result<Arc<Self>> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("peers.json");
        let peers: HashMap<String, PeerInfo> = if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Always ensure the backbone is present.
        let mut peers = peers;
        peers.entry(crate::net::BACKBONE_PEER.to_string())
            .or_insert(PeerInfo {
                address: crate::net::BACKBONE_PEER.to_string(),
                first_seen: chrono::Utc::now().timestamp(),
                last_seen: 0,
                last_height: 0,
                successful_connections: 0,
                failed_attempts: 0,
                is_backbone: true,
            });

        Ok(Arc::new(Self {
            inner: RwLock::new(PeerRegistryInner { peers }),
        }))
    }

    pub fn all(&self) -> Vec<PeerInfo> {
        self.inner.read().peers.values().cloned().collect()
    }

    pub fn count_alive(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        self.inner
            .read()
            .peers
            .values()
            .filter(|p| now - p.last_seen < 120)
            .count()
    }

    pub fn note_success(&self, address: &str, height: u64) {
        let mut g = self.inner.write();
        let entry = g.peers.entry(address.to_string()).or_insert(PeerInfo {
            address: address.to_string(),
            first_seen: chrono::Utc::now().timestamp(),
            last_seen: 0,
            last_height: 0,
            successful_connections: 0,
            failed_attempts: 0,
            is_backbone: address == crate::net::BACKBONE_PEER,
        });
        entry.last_seen = chrono::Utc::now().timestamp();
        entry.last_height = height;
        entry.successful_connections += 1;
    }

    pub fn note_failure(&self, address: &str) {
        let mut g = self.inner.write();
        if let Some(e) = g.peers.get_mut(address) {
            e.failed_attempts += 1;
        }
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let path = dir.join("peers.json");
        let tmp = dir.join("peers.json.tmp");
        let g = self.inner.read();
        let text = serde_json::to_string_pretty(&g.peers)?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}
