// Chain state. The in-memory canonical chain, mempool, and account balances
// derived deterministically from applied blocks. Persistence is delegated to
// the chain/ directory (block files) and runtime state.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use super::block::{Block, Hash};
use super::tx::{Transaction, TxRecord, TxStatus};

pub const DOM_CHAIN_ID: &str = "dom-devnet-1";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SyncProgress {
    pub current_height: u64,
    pub target_height: u64,
    pub phase: SyncPhase,
    pub peer_source: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncPhase {
    #[default]
    Idle,
    Connecting,
    InitialBlockDownload,
    CatchingUp,
    Live,
    Resumed,
}

impl SyncPhase {
    pub fn label_pt(&self) -> &'static str {
        match self {
            SyncPhase::Idle => "Inativo",
            SyncPhase::Connecting => "Conectando",
            SyncPhase::InitialBlockDownload => "Sincronização inicial",
            SyncPhase::CatchingUp => "Atualizando",
            SyncPhase::Live => "Operacional",
            SyncPhase::Resumed => "Retomado",
        }
    }
}

pub struct ChainState {
    inner: RwLock<ChainStateInner>,
    dir: PathBuf,
}

struct ChainStateInner {
    blocks: Vec<Block>,
    block_index: HashMap<Hash, u64>,
    balances: HashMap<String, u64>,
    nonces: HashMap<String, u64>,
    mempool: VecDeque<Transaction>,
    sync: SyncProgress,
    tx_records: HashMap<String, TxRecord>,
}

impl ChainState {
    pub fn open(dir: PathBuf) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&dir)?;
        // Load persisted blocks if present. For DEVNET v0 we keep a single
        // append-only file; a proper LMDB-backed store will replace this.
        let mut blocks = vec![Block::genesis()];
        let blocks_file = dir.join("blocks.bin");
        if blocks_file.exists() {
            if let Ok(bytes) = std::fs::read(&blocks_file) {
                if let Ok(loaded) = bincode::deserialize::<Vec<Block>>(&bytes) {
                    if !loaded.is_empty() && loaded[0].header.height == 0 {
                        blocks = loaded;
                    }
                }
            }
        }

        let mut inner = ChainStateInner {
            block_index: HashMap::new(),
            balances: HashMap::new(),
            nonces: HashMap::new(),
            mempool: VecDeque::new(),
            sync: SyncProgress::default(),
            tx_records: HashMap::new(),
            blocks,
        };

        for b in &inner.blocks {
            inner.block_index.insert(b.hash(), b.header.height);
            // Reapply balances deterministically.
            for tx in &b.txs {
                Self::apply_tx_to_balances(&mut inner.balances, &mut inner.nonces, tx);
            }
        }

        let state = Arc::new(Self {
            inner: RwLock::new(inner),
            dir,
        });
        Ok(state)
    }

    fn apply_tx_to_balances(
        balances: &mut HashMap<String, u64>,
        nonces: &mut HashMap<String, u64>,
        tx: &Transaction,
    ) {
        let from_bal = balances.entry(tx.body.from.clone()).or_insert(0);
        let total = tx.body.amount.saturating_add(tx.body.fee);
        *from_bal = from_bal.saturating_sub(total);
        let to_bal = balances.entry(tx.body.to.clone()).or_insert(0);
        *to_bal = to_bal.saturating_add(tx.body.amount);
        let n = nonces.entry(tx.body.from.clone()).or_insert(0);
        *n = (*n).max(tx.body.nonce + 1);
    }

    pub fn height(&self) -> u64 {
        let g = self.inner.read();
        g.blocks.last().map(|b| b.header.height).unwrap_or(0)
    }

    pub fn tip_hash_hex(&self) -> String {
        let g = self.inner.read();
        match g.blocks.last() {
            Some(b) => hex::encode(b.hash()),
            None => "0".repeat(64),
        }
    }

    pub fn mempool_size(&self) -> usize {
        self.inner.read().mempool.len()
    }

    pub fn balance_of(&self, addr: &str) -> u64 {
        *self.inner.read().balances.get(addr).unwrap_or(&0)
    }

    pub fn nonce_of(&self, addr: &str) -> u64 {
        *self.inner.read().nonces.get(addr).unwrap_or(&0)
    }

    pub fn sync_progress(&self) -> SyncProgress {
        self.inner.read().sync.clone()
    }

    pub fn set_sync(&self, sp: SyncProgress) {
        self.inner.write().sync = sp;
    }

    pub fn submit_local_tx(&self, tx: Transaction) {
        let mut g = self.inner.write();
        let txid_hex = tx.id_hex();
        let record = TxRecord {
            tx: tx.clone(),
            status: TxStatus::Pending,
            seen_at: chrono::Utc::now().timestamp(),
            confirmed_height: None,
        };
        g.tx_records.insert(txid_hex, record);
        g.mempool.push_back(tx);
    }

    pub fn cancel_pending(&self, txid_hex: &str) -> bool {
        let mut g = self.inner.write();
        let mut removed = false;
        g.mempool.retain(|t| {
            if t.id_hex() == txid_hex {
                removed = true;
                false
            } else {
                true
            }
        });
        if let Some(rec) = g.tx_records.get_mut(txid_hex) {
            if rec.status == TxStatus::Pending {
                rec.status = TxStatus::Failed;
            }
        }
        removed
    }

    pub fn tx_records(&self) -> Vec<TxRecord> {
        let g = self.inner.read();
        let mut v: Vec<TxRecord> = g.tx_records.values().cloned().collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.seen_at));
        v
    }

    /// Append a block produced locally (mining) or received from the network.
    /// Performs minimal validation: height continuity + prev_hash linkage.
    pub fn append_block(&self, block: Block) -> Result<()> {
        let mut g = self.inner.write();
        let tip = g.blocks.last().cloned().unwrap_or_else(Block::genesis);
        if block.header.height != tip.header.height + 1 {
            anyhow::bail!("non-contiguous block height");
        }
        if block.header.prev_hash != tip.hash() {
            anyhow::bail!("prev_hash mismatch");
        }
        // Apply transactions deterministically.
        for tx in &block.txs {
            Self::apply_tx_to_balances(&mut g.balances, &mut g.nonces, tx);
            let txid_hex = tx.id_hex();
            if let Some(rec) = g.tx_records.get_mut(&txid_hex) {
                rec.status = TxStatus::Confirmed;
                rec.confirmed_height = Some(block.header.height);
            }
            // Evict from mempool if present.
            g.mempool.retain(|t| t.id_hex() != txid_hex);
        }
        g.block_index.insert(block.hash(), block.header.height);
        g.blocks.push(block);
        Ok(())
    }

    pub fn persist_blocks(&self) -> Result<()> {
        let g = self.inner.read();
        let bytes = bincode::serialize(&g.blocks)?;
        let tmp = self.dir.join("blocks.bin.tmp");
        let final_path = self.dir.join("blocks.bin");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(tmp, final_path)?;
        Ok(())
    }

    pub fn take_mempool_for_mining(&self, max: usize) -> Vec<Transaction> {
        let g = self.inner.read();
        g.mempool.iter().take(max).cloned().collect()
    }
}
