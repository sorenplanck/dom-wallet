// Transaction primitive. Ed25519-signed transfer with deterministic txid
// computed over the signed payload.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type TxId = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxBody {
    pub from: String,
    pub to: String,
    pub amount: u64,     // in base units (1 DOM = 10_000_000 base units)
    pub fee: u64,
    pub nonce: u64,
    pub memo: Option<String>,
    pub chain_id: String,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub body: TxBody,
    pub pubkey: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    Pending,
    Confirmed,
    Failed,
    Rebroadcast,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx: Transaction,
    pub status: TxStatus,
    pub seen_at: i64,
    pub confirmed_height: Option<u64>,
}

impl Transaction {
    pub fn id(&self) -> TxId {
        let bytes = bincode::serialize(&self.body).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }

    pub fn id_hex(&self) -> String {
        hex::encode(self.id())
    }
}

pub const BASE_UNITS_PER_DOM: u64 = 10_000_000;

pub fn format_dom(units: u64) -> String {
    let whole = units / BASE_UNITS_PER_DOM;
    let frac = units % BASE_UNITS_PER_DOM;
    // Render with thousands separator and 4 fractional digits like the spec
    // example "3.482,2456 DOM".
    let whole_str = format_thousands(whole);
    // 4 fractional digits — scale 1e7 base units down to 1e4 display digits.
    let four = (frac as f64 / 1_000.0).round() as u64; // 7 -> 4 digits
    format!("{whole_str},{:04}", four.min(9999))
}

fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(*b as char);
    }
    out
}
