// Replay snapshots. The DEVNET diagnostics surface depends on a durable
// append-only log of operationally significant runtime events: canonical tip
// changes, reorgs, restart recovery, mempool reconciliation, peer rotation.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SnapshotEvent {
    BootStart { version: String },
    BootComplete { wallet_address: String, height: u64 },
    CanonicalTipAdvanced { from: u64, to: u64, block_hash: String },
    Reorg { depth: u64, new_tip: String },
    IbdResumed { from_height: u64 },
    IbdComplete { final_height: u64 },
    PeerRotation { added: usize, removed: usize },
    MempoolReconciled { added: usize, evicted: usize },
    RestartRecovery { last_known_height: u64 },
    MiningStarted,
    MiningStopped,
    Shutdown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub timestamp: DateTime<Utc>,
    pub event: SnapshotEvent,
}

pub struct SnapshotLog {
    file: parking_lot::Mutex<std::fs::File>,
}

impl SnapshotLog {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("events.jsonl");
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file: parking_lot::Mutex::new(file) })
    }

    pub fn record(&self, event: SnapshotEvent) {
        let record = SnapshotRecord { timestamp: Utc::now(), event };
        if let Ok(line) = serde_json::to_string(&record) {
            let mut f = self.file.lock();
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }

    pub fn read_recent(dir: &Path, max: usize) -> Result<Vec<SnapshotRecord>> {
        let path = dir.join("events.jsonl");
        if !path.exists() {
            return Ok(vec![]);
        }
        let text = std::fs::read_to_string(&path)?;
        let mut out: Vec<SnapshotRecord> = Vec::new();
        for line in text.lines().rev().take(max) {
            if let Ok(rec) = serde_json::from_str::<SnapshotRecord>(line) {
                out.push(rec);
            }
        }
        out.reverse();
        Ok(out)
    }
}
