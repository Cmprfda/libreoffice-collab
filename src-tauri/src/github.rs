//! GitHub Releases API client.
//!
//! The updater plugin decides *whether* an update exists (it is the only path
//! that verifies the minisign signature, so it must stay authoritative). This
//! module answers the other half of the question — *what changed* — by reading
//! the release GitHub actually published.
//!
//! Keeping the two separate matters: `latest.json` carries a machine-readable
//! version and signature, while the release body is the human changelog the
//! "Novidades desta versão" panel shows.

use serde::Deserialize;

/// ┌────────────────────────────────────────────────────────────────────────┐
/// │ CHANGE THESE WHEN YOU FORK THE PROJECT — they must match src/lib/config │
/// │ .ts, tauri.conf.json and the installer scripts.                        │
/// └────────────────────────────────────────────────────────────────────────┘
pub const OWNER: &str = "your-org";
pub const REPO: &str = "libreoffice-collab";

/// Only the fields we actually use; GitHub sends a lot more.
#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub published_at: String,
}

fn latest_release_url() -> String {
    format!("https://api.github.com/repos/{OWNER}/{REPO}/releases/latest")
}

/// Reads the newest published release.
///
/// Unauthenticated calls are rate-limited to 60 per hour per IP, which is far
/// more than our "once at startup, then every six hours" cadence needs.
pub async fn latest_release() -> anyhow::Result<Release> {
    let release = crate::server::client()
        .get(latest_release_url())
        // GitHub rejects API calls without a User-Agent; the shared client
        // already sets one. This header pins the response schema version.
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;
    Ok(release)
}

/// Strips the leading `v` so "v1.2.3" can be compared with a package version.
pub fn version_of(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}
