// Block primitive. Deterministic hashing using SHA-256 over a canonical
// serialization. Header fields cover: previous hash, merkle root over
// transactions, height, timestamp, nonce, miner address.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::tx::Transaction;

pub type Hash = [u8; 32];

pub fn hash_hex(h: &Hash) -> String {
    hex::encode(h)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub prev_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: i64,
    pub nonce: u64,
    pub miner: String,
    pub difficulty: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Transaction>,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }
}

impl Block {
    pub fn genesis() -> Self {
        let header = BlockHeader {
            height: 0,
            prev_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            timestamp: 0,
            nonce: 0,
            miner: "DOM_GENESIS".to_string(),
            difficulty: 16,
        };
        Self { header, txs: vec![] }
    }

    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    pub fn compute_merkle_root(txs: &[Transaction]) -> Hash {
        if txs.is_empty() {
            return [0u8; 32];
        }
        let mut layer: Vec<Hash> = txs.iter().map(|t| t.id()).collect();
        while layer.len() > 1 {
            let mut next = Vec::with_capacity(layer.len() / 2 + 1);
            for pair in layer.chunks(2) {
                let mut h = Sha256::new();
                h.update(pair[0]);
                h.update(pair.get(1).unwrap_or(&pair[0]));
                let mut out = [0u8; 32];
                out.copy_from_slice(&h.finalize());
                next.push(out);
            }
            layer = next;
        }
        layer[0]
    }
}
