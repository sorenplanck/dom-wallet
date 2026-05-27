// Mining — deterministic proof-of-work. The miner takes the current tip,
// drains the mempool up to a cap, and searches nonces until the block hash
// has the required number of leading zero bits.
//
// Mining is optional. It runs independently of wallet unlock state — the
// node operates as monetary infrastructure even when the UI is locked.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::chain::block::{Block, BlockHeader};
use crate::chain::state::ChainState;
use crate::persist::snapshot::{SnapshotEvent, SnapshotLog};

pub struct Miner {
    chain: Arc<ChainState>,
    miner_address: String,
    enabled: Arc<AtomicBool>,
    hashrate: Arc<AtomicU64>,
    snapshots: Arc<SnapshotLog>,
}

impl Miner {
    pub fn new(
        chain: Arc<ChainState>,
        miner_address: String,
        snapshots: Arc<SnapshotLog>,
    ) -> Arc<Self> {
        Arc::new(Self {
            chain,
            miner_address,
            enabled: Arc::new(AtomicBool::new(false)),
            hashrate: Arc::new(AtomicU64::new(0)),
            snapshots,
        })
    }

    pub fn set_enabled(&self, on: bool) {
        let was = self.enabled.swap(on, Ordering::SeqCst);
        if was != on {
            self.snapshots.record(if on {
                SnapshotEvent::MiningStarted
            } else {
                SnapshotEvent::MiningStopped
            });
        }
        if !on {
            self.hashrate.store(0, Ordering::Relaxed);
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn hashrate(&self) -> u64 {
        self.hashrate.load(Ordering::Relaxed)
    }

    /// Long-running miner loop. Honors the enabled flag and a shutdown signal.
    pub async fn run(self: Arc<Self>, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        loop {
            if *shutdown.borrow() {
                return;
            }
            if !self.is_enabled() {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                    _ = shutdown.changed() => return,
                }
                continue;
            }

            // Try to mine one block.
            let tip_height = self.chain.height();
            let tip_hash = self.chain.tip_hash_hex();
            let tip_hash_bytes = decode_hex_to_32(&tip_hash);

            let txs = self.chain.take_mempool_for_mining(64);
            let merkle = Block::compute_merkle_root(&txs);

            let mut header = BlockHeader {
                height: tip_height + 1,
                prev_hash: tip_hash_bytes,
                merkle_root: merkle,
                timestamp: chrono::Utc::now().timestamp(),
                nonce: 0,
                miner: self.miner_address.clone(),
                difficulty: 16, // bits of leading zeros required
            };

            let difficulty_bits = header.difficulty as u32;
            let start = Instant::now();
            let mut hashes = 0u64;

            loop {
                if !self.is_enabled() || *shutdown.borrow() {
                    break;
                }
                header.nonce = header.nonce.wrapping_add(1);
                let bytes = bincode::serialize(&header).unwrap_or_default();
                let digest = Sha256::digest(&bytes);
                hashes += 1;
                if leading_zero_bits(&digest) >= difficulty_bits {
                    let block = Block { header: header.clone(), txs: txs.clone() };
                    let new_height = block.header.height;
                    let new_hash = hex::encode(block.hash());
                    if self.chain.append_block(block).is_ok() {
                        self.snapshots.record(SnapshotEvent::CanonicalTipAdvanced {
                            from: tip_height,
                            to: new_height,
                            block_hash: new_hash,
                        });
                        let _ = self.chain.persist_blocks();
                    }
                    break;
                }
                if hashes % 4096 == 0 {
                    let elapsed = start.elapsed().as_secs_f64().max(0.001);
                    let rate = (hashes as f64 / elapsed) as u64;
                    self.hashrate.store(rate, Ordering::Relaxed);
                    // Yield so we don't starve the runtime.
                    tokio::task::yield_now().await;
                    if !self.is_enabled() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    }
}

fn decode_hex_to_32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    if let Ok(b) = hex::decode(s) {
        let n = b.len().min(32);
        out[..n].copy_from_slice(&b[..n]);
    }
    out
}

fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut count = 0u32;
    for b in bytes {
        if *b == 0 {
            count += 8;
        } else {
            count += b.leading_zeros();
            break;
        }
    }
    count
}
