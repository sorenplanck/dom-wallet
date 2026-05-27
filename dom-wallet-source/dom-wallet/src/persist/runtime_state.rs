// Runtime state — durable record of last known operational state. Loaded on
// boot to restore canonical tip references, sync progress markers, and
// optional flags (auto-mine, hidden start) across restarts.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeState {
    pub last_height: u64,
    pub last_tip_hash: String,
    pub auto_mine: bool,
    pub auto_start_hidden: bool,
    pub last_clean_shutdown: bool,
}

impl RuntimeState {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("runtime.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text).unwrap_or_default())
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("runtime.json");
        let tmp = dir.join("runtime.json.tmp");
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}
