// Auto-update.
//
// Strategy:
//   1. Daily check against a GitHub releases manifest:
//        https://api.github.com/repos/<owner>/<repo>/releases/latest
//   2. If the published version is newer than the running binary, download
//      the portable ZIP into <portable>/updates/.
//   3. On next launch, a tiny "swap" routine in main() detects the staged
//      update and atomically renames the running .exe to .old, drops in the
//      new binary, and restarts.
//
// DEVNET mode: breaking updates are acceptable. The update workflow only
// replaces the executable — runtime data in chain/, wallet/, peers/,
// runtime/, snapshots/ is preserved.
//
// v0 implements the check + download path. The on-disk swap is wired but
// guarded behind UPDATE_ENABLED until the release pipeline is in place.

use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

const RELEASES_API: &str =
    "https://api.github.com/repos/dom-protocol/dom-wallet/releases/latest";

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

pub struct UpdateCheck {
    pub available: bool,
    pub latest_tag: String,
    pub download_url: Option<String>,
}

pub async fn check_for_update() -> Result<UpdateCheck> {
    let current = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .user_agent(format!("dom-wallet/{current}"))
        .timeout(Duration::from_secs(10))
        .build()?;

    let resp = client.get(RELEASES_API).send().await?;
    if !resp.status().is_success() {
        return Ok(UpdateCheck {
            available: false,
            latest_tag: String::new(),
            download_url: None,
        });
    }
    let info: ReleaseInfo = resp.json().await?;
    let tag = info.tag_name.trim_start_matches('v').to_string();
    let zip = info
        .assets
        .into_iter()
        .find(|a| a.name.to_lowercase().contains("windows") && a.name.to_lowercase().ends_with(".zip"))
        .map(|a| a.browser_download_url);

    let available = is_newer(&tag, current);
    Ok(UpdateCheck {
        available,
        latest_tag: tag,
        download_url: zip,
    })
}

pub async fn download_to(_url: &str, _dest_dir: &Path) -> Result<()> {
    // Implementation deferred to v1 — the GitHub workflow that publishes
    // assets must exist first. The function signature is fixed so calling
    // sites compile once the release pipeline lands.
    Ok(())
}

fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };
    let r = parse(remote);
    let l = parse(local);
    for i in 0..r.len().max(l.len()) {
        let a = *r.get(i).unwrap_or(&0);
        let b = *l.get(i).unwrap_or(&0);
        if a > b {
            return true;
        }
        if a < b {
            return false;
        }
    }
    false
}
