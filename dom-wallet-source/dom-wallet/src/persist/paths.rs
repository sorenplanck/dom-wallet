// Portable paths. Everything lives beside the executable — no AppData,
// no admin, no installer state.

use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub chain: PathBuf,
    pub wallet: PathBuf,
    pub config: PathBuf,
    pub peers: PathBuf,
    pub logs: PathBuf,
    pub snapshots: PathBuf,
    pub runtime: PathBuf,
    pub updates: PathBuf,
}

impl Paths {
    pub fn portable_beside_executable() -> Result<Self> {
        let exe = std::env::current_exe().context("locating current executable")?;
        let root = exe
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Self::with_root(root))
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self {
            chain: root.join("chain"),
            wallet: root.join("wallet"),
            config: root.join("config"),
            peers: root.join("peers"),
            logs: root.join("logs"),
            snapshots: root.join("snapshots"),
            runtime: root.join("runtime"),
            updates: root.join("updates"),
            root,
        }
    }

    pub fn ensure_all(&self) -> Result<()> {
        for d in [
            &self.chain,
            &self.wallet,
            &self.config,
            &self.peers,
            &self.logs,
            &self.snapshots,
            &self.runtime,
            &self.updates,
        ] {
            std::fs::create_dir_all(d).with_context(|| format!("creating {d:?}"))?;
        }
        Ok(())
    }
}
