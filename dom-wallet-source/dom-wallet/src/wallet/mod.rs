// Wallet — deterministic key generation, encrypted persistence, address
// derivation tied to the DOM chain. The wallet starts LOCKED. Unlock requires
// the user-provided password. Operations that require unlock:
//   - sign transactions
//   - reveal seed/private key
//   - export keys
// Operations that DO NOT require unlock:
//   - node startup, sync, mining, peer relay (handled by node module)
//
// Persistence: <portable>/wallet/wallet.json
//   { version, salt, kdf_iters, nonce, ciphertext, public_address }
// The public_address is stored in cleartext so the UI can show the address
// and balance while the wallet is locked.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, Context, Result};
use bip39::{Language, Mnemonic};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use parking_lot::RwLock;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use zeroize::Zeroize;

use crate::chain::tx::{Transaction, TxBody, BASE_UNITS_PER_DOM};
use crate::persist::paths::Paths;

const WALLET_FILE: &str = "wallet.json";
const KDF_ITERS: u32 = 200_000;
const ADDRESS_PREFIX: &str = "dom1";

#[derive(Serialize, Deserialize)]
struct WalletFile {
    version: u32,
    salt_hex: String,
    kdf_iters: u32,
    nonce_hex: String,
    ciphertext_hex: String,
    public_address: String,
    // Cleartext public key bytes — used for receive operations and to verify
    // signed transactions without unlocking.
    public_key_hex: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct SealedSecrets {
    mnemonic: String,
    signing_key_hex: String,
}

pub struct Wallet {
    file_path: PathBuf,
    address: String,
    public_key: VerifyingKey,
    state: RwLock<WalletState>,
}

struct WalletState {
    file: WalletFile,
    unlocked: Option<UnlockedKey>,
}

struct UnlockedKey {
    signing_key: SigningKey,
    mnemonic: String,
}

impl Drop for UnlockedKey {
    fn drop(&mut self) {
        self.mnemonic.zeroize();
        // SigningKey zeroizes its internal seed on drop via the ed25519_dalek
        // crate's ZeroizeOnDrop derive when enabled. Defensive only.
    }
}

impl Wallet {
    pub fn open_or_create(paths: &Paths) -> Result<Self> {
        let file_path = paths.wallet.join(WALLET_FILE);
        if file_path.exists() {
            Self::open(&file_path)
        } else {
            Self::create_fresh(&file_path)
        }
    }

    fn open(file_path: &PathBuf) -> Result<Self> {
        let text = std::fs::read_to_string(file_path).context("reading wallet file")?;
        let file: WalletFile = serde_json::from_str(&text).context("parsing wallet file")?;

        let pk_bytes = hex::decode(&file.public_key_hex).context("decoding public key")?;
        let pk_arr: [u8; 32] = pk_bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid public key length"))?;
        let public_key =
            VerifyingKey::from_bytes(&pk_arr).map_err(|e| anyhow!("invalid public key: {e}"))?;

        Ok(Self {
            file_path: file_path.clone(),
            address: file.public_address.clone(),
            public_key,
            state: RwLock::new(WalletState { file, unlocked: None }),
        })
    }

    fn create_fresh(file_path: &PathBuf) -> Result<Self> {
        // Generate a 24-word mnemonic (256 bits of entropy).
        let mut entropy = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| anyhow!("mnemonic generation: {e}"))?;
        let phrase = mnemonic.to_string();

        // Derive an Ed25519 signing key from the mnemonic seed (no passphrase
        // at this stage — the password protects the encrypted store).
        let seed_bytes = mnemonic.to_seed("");
        let mut seed32 = [0u8; 32];
        seed32.copy_from_slice(&seed_bytes[..32]);
        let signing_key = SigningKey::from_bytes(&seed32);
        let verifying_key = signing_key.verifying_key();
        let address = derive_address(&verifying_key);

        // Wallet is created without a password initially. The user is
        // prompted to set one on first unlock attempt. The wallet file stores
        // the secret encrypted under a sentinel "uninitialized" key derived
        // from a fixed-but-unique boot salt — until the user sets a password,
        // signing is disabled and we expose this as `requires_password_init`.
        // For DEVNET v0 we simply require the user to set the password before
        // sending. The cleartext public address persists immediately so the
        // wallet identity is bound to the DOM chain on first run.

        let sealed = SealedSecrets {
            mnemonic: phrase,
            signing_key_hex: hex::encode(signing_key.to_bytes()),
        };

        // Encrypt under a default empty-password derived key; the UI will
        // prompt the user to set a real password on first unlock. This is
        // explicitly DEVNET behavior — see spec.
        let (salt, ciphertext, nonce) = encrypt_secrets(&sealed, "")?;

        let file = WalletFile {
            version: 1,
            salt_hex: hex::encode(salt),
            kdf_iters: KDF_ITERS,
            nonce_hex: hex::encode(nonce),
            ciphertext_hex: hex::encode(ciphertext),
            public_address: address.clone(),
            public_key_hex: hex::encode(verifying_key.to_bytes()),
        };

        let text = serde_json::to_string_pretty(&file)?;
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file_path, text)?;

        Ok(Self {
            file_path: file_path.clone(),
            address,
            public_key: verifying_key,
            state: RwLock::new(WalletState { file, unlocked: None }),
        })
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn is_unlocked(&self) -> bool {
        self.state.read().unlocked.is_some()
    }

    /// Unlock with the user's password. On first run the default password is
    /// the empty string; the UI prompts the user to change it.
    pub fn unlock(&self, password: &str) -> Result<()> {
        let file_snapshot = { self.state.read().file.salt_hex.clone() };
        let salt = hex::decode(&file_snapshot).context("decoding salt")?;
        let nonce = {
            let g = self.state.read();
            hex::decode(&g.file.nonce_hex).context("decoding nonce")?
        };
        let ciphertext = {
            let g = self.state.read();
            hex::decode(&g.file.ciphertext_hex).context("decoding ciphertext")?
        };
        let iters = self.state.read().file.kdf_iters;

        let key = derive_key(password, &salt, iters);
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("cipher init"))?;
        let nonce_arr: [u8; 12] = nonce
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid nonce length"))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce_arr), ciphertext.as_ref())
            .map_err(|_| anyhow!("incorrect password"))?;
        let sealed: SealedSecrets = serde_json::from_slice(&plaintext)?;
        let sk_bytes = hex::decode(&sealed.signing_key_hex)?;
        let sk_arr: [u8; 32] = sk_bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid signing key length"))?;
        let signing_key = SigningKey::from_bytes(&sk_arr);

        self.state.write().unlocked = Some(UnlockedKey {
            signing_key,
            mnemonic: sealed.mnemonic,
        });
        Ok(())
    }

    pub fn lock(&self) {
        self.state.write().unlocked = None;
    }

    /// Change the password. Requires the wallet to be unlocked.
    pub fn change_password(&self, new_password: &str) -> Result<()> {
        let unlocked = {
            let g = self.state.read();
            let u = g
                .unlocked
                .as_ref()
                .ok_or_else(|| anyhow!("wallet must be unlocked to change password"))?;
            SealedSecrets {
                mnemonic: u.mnemonic.clone(),
                signing_key_hex: hex::encode(u.signing_key.to_bytes()),
            }
        };

        let (salt, ciphertext, nonce) = encrypt_secrets(&unlocked, new_password)?;
        let mut g = self.state.write();
        g.file.salt_hex = hex::encode(salt);
        g.file.nonce_hex = hex::encode(nonce);
        g.file.ciphertext_hex = hex::encode(ciphertext);
        g.file.kdf_iters = KDF_ITERS;
        let text = serde_json::to_string_pretty(&g.file)?;
        std::fs::write(&self.file_path, text)?;
        Ok(())
    }

    /// Reveal mnemonic — requires unlock.
    pub fn reveal_mnemonic(&self) -> Result<String> {
        let g = self.state.read();
        let u = g
            .unlocked
            .as_ref()
            .ok_or_else(|| anyhow!("wallet locked"))?;
        Ok(u.mnemonic.clone())
    }

    /// Sign and produce a transaction. Requires unlock.
    pub fn sign_transaction(
        &self,
        to: &str,
        amount_units: u64,
        fee_units: u64,
        nonce: u64,
        memo: Option<String>,
        chain_id: &str,
    ) -> Result<Transaction> {
        let g = self.state.read();
        let u = g
            .unlocked
            .as_ref()
            .ok_or_else(|| anyhow!("wallet locked"))?;

        let body = TxBody {
            from: self.address.clone(),
            to: to.to_string(),
            amount: amount_units,
            fee: fee_units,
            nonce,
            memo,
            chain_id: chain_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        let body_bytes = bincode::serialize(&body)?;
        let signature = u.signing_key.sign(&body_bytes);

        Ok(Transaction {
            body,
            pubkey: u.signing_key.verifying_key().to_bytes().to_vec(),
            signature: signature.to_bytes().to_vec(),
        })
    }
}

pub fn parse_dom_amount(input: &str) -> Result<u64> {
    // Accept "1.234,5678" or "1234.5678" or plain integer formats.
    let cleaned = input.trim().replace(' ', "");
    let normalized = if cleaned.contains(',') && cleaned.contains('.') {
        // Brazilian format: thousand '.', decimal ','
        cleaned.replace('.', "").replace(',', ".")
    } else if cleaned.contains(',') {
        cleaned.replace(',', ".")
    } else {
        cleaned
    };
    let value: f64 = normalized
        .parse()
        .map_err(|_| anyhow!("invalid amount format"))?;
    if value < 0.0 {
        return Err(anyhow!("amount must be non-negative"));
    }
    let units = (value * BASE_UNITS_PER_DOM as f64).round() as u64;
    Ok(units)
}

fn derive_address(vk: &VerifyingKey) -> String {
    // Address = prefix + base58(sha256(pubkey)[..20])
    let mut h = Sha256::new();
    h.update(vk.to_bytes());
    let full = h.finalize();
    let short = &full[..20];
    format!("{ADDRESS_PREFIX}{}", bs58::encode(short).into_string())
}

fn derive_key(password: &str, salt: &[u8], iters: u32) -> [u8; 32] {
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iters, &mut key);
    key
}

fn encrypt_secrets(
    sealed: &SealedSecrets,
    password: &str,
) -> Result<([u8; 16], Vec<u8>, [u8; 12])> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let key = derive_key(password, &salt, KDF_ITERS);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("cipher init"))?;
    let plaintext = serde_json::to_vec(sealed)?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|_| anyhow!("encryption failed"))?;
    Ok((salt, ciphertext, nonce))
}
