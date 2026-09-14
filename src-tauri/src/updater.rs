//! Auto-update against GitHub Releases.
//!
//! Flow: `latest.json` (published by the release workflow) → semver comparison
//! → minisign signature verification by the updater plugin → silent NSIS
//! install → relaunch. The public key lives in `tauri.conf.json`; a release
//! signed with anything else is rejected before a single byte is executed.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

use crate::github;
use crate::i18n;
use crate::settings;

pub const EVENT_UPDATE_AVAILABLE: &str = "update://available";
pub const EVENT_UPDATE_PROGRESS: &str = "update://progress";

/// Silent re-check cadence. Mirrors UPDATE_CHECK_INTERVAL_MS on the TS side.
const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// Delay before the first check so startup stays snappy on slow machines.
const STARTUP_DELAY: std::time::Duration = std::time::Duration::from_secs(8);

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: u64,
    percent: f64,
}

/// Queries the update endpoint. Returns `available: false` when up to date.
///
/// Two sources are consulted, on purpose:
///   * the signed `latest.json` behind the updater plugin decides *whether* an
///     update exists — it is the only path that verifies the signature;
///   * `api.github.com/repos/{owner}/{repo}/releases/latest` supplies the human
///     changelog and publication date shown in the update dialog.
pub async fn check(app: &AppHandle) -> anyhow::Result<UpdateInfo> {
    let current_version = app.package_info().version.to_string();

    let updater = app.updater()?;
    let Some(update) = updater.check().await? else {
        return Ok(UpdateInfo {
            available: false,
            current_version: current_version.clone(),
            version: current_version,
            notes: String::new(),
            date: String::new(),
        });
    };

    let mut info = UpdateInfo {
        available: true,
        current_version,
        version: update.version.clone(),
        notes: update.body.clone().unwrap_or_default(),
        date: update.date.map(|date| date.to_string()).unwrap_or_default(),
    };

    // Enrich from GitHub. A failure here is never fatal: the update is still
    // installable, the dialog just shows no release notes.
    match github::latest_release().await {
        Ok(release) => {
            // Only trust the notes when GitHub is describing the same version
            // the signed manifest offered, otherwise we would show a changelog
            // for a release the user is not about to install.
            if github::version_of(&release.tag_name) == info.version {
                if !release.body.trim().is_empty() {
                    info.notes = release.body;
                }
                if info.date.is_empty() {
                    info.date = release.published_at;
                }
            }
        }
        Err(error) => log::debug!("GitHub release notes unavailable: {error}"),
    }

    Ok(info)
}

/// Downloads, verifies, installs and relaunches. Never returns on success.
pub async fn install(app: &AppHandle) -> anyhow::Result<()> {
    let updater = app.updater()?;
    let Some(update) = updater.check().await? else {
        // Nothing to do — another instance may have updated us already.
        return Ok(());
    };

    let downloaded = Arc::new(AtomicU64::new(0));
    let progress_app = app.clone();
    let progress_counter = Arc::clone(&downloaded);

    update
        .download_and_install(
            move |chunk_length, content_length| {
                let total = content_length.unwrap_or(0);
                let so_far =
                    progress_counter.fetch_add(chunk_length as u64, Ordering::Relaxed)
                        + chunk_length as u64;
                let percent = if total > 0 {
                    (so_far as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let _ = progress_app.emit(
                    EVENT_UPDATE_PROGRESS,
                    Progress {
                        downloaded: so_far,
                        total,
                        percent: percent.min(100.0),
                    },
                );
            },
            || log::info!("update downloaded; handing over to the installer"),
        )
        .await?;

    let strings = i18n::strings(&settings::current(app).language);
    let _ = app
        .notification()
        .builder()
        .title(strings.update_installed_title)
        .body(strings.update_installed_body)
        .show();

    // Replaces the running process with the freshly installed one.
    app.restart();
}

/// Background task: one check shortly after launch, then every six hours.
///
/// When "install updates automatically" is on we go straight to installing;
/// otherwise the UI is asked to show the one-click prompt, backed by a native
/// Windows toast so the user sees it even with the window hidden in the tray.
pub fn spawn_background_checks(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        loop {
            match check(&app).await {
                Ok(info) if info.available => {
                    let config = settings::current(&app);

                    // Record the successful check so Settings can show it.
                    if let Ok(mut guard) = app.state::<settings::SettingsState>().0.lock() {
                        guard.last_update_check = now_seconds();
                        let _ = settings::save(&app, &guard);
                    }

                    if config.auto_install_updates {
                        if let Err(error) = install(&app).await {
                            log::error!("automatic update failed: {error}");
                        }
                    } else {
                        notify_available(&app, &info, &config.language);
                        let _ = app.emit(EVENT_UPDATE_AVAILABLE, info);
                    }
                }
                Ok(_) => {
                    if let Ok(mut guard) = app.state::<settings::SettingsState>().0.lock() {
                        guard.last_update_check = now_seconds();
                        let _ = settings::save(&app, &guard);
                    }
                }
                Err(error) => log::warn!("update check failed: {error}"),
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

fn notify_available(app: &AppHandle, info: &UpdateInfo, language: &str) {
    let strings = i18n::strings(language);
    let body = i18n::fill(strings.update_body, "version", &info.version);
    let _ = app
        .notification()
        .builder()
        .title(strings.update_title)
        .body(body)
        .show();
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
