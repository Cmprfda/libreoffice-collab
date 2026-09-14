/**
 * Build-time constants.
 *
 * ┌──────────────────────────────────────────────────────────────────────────┐
 * │ CHANGE THESE TWO VALUES WHEN YOU FORK THE PROJECT.                       │
 * │ They must match the same values in:                                      │
 * │   - src-tauri/tauri.conf.json  (plugins.updater.endpoints)               │
 * │   - scripts/instalar.bat       (OWNER / REPO)                            │
 * │   - scripts/install.ps1        ($Owner / $Repo)                          │
 * └──────────────────────────────────────────────────────────────────────────┘
 */
export const GITHUB_OWNER = "your-org";
export const GITHUB_REPO = "libreoffice-collab";

export const GITHUB_REPO_URL = `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}`;

/**
 * The Releases API (`api.github.com/repos/{owner}/{repo}/releases/latest`) is
 * queried from Rust instead — see `src-tauri/src/github.rs`. Keeping it there
 * means the startup check still runs while the window is hidden in the tray,
 * and the call sits next to the signature verification it belongs with.
 */

/** How often the dashboard re-checks that the selected server is alive (ms). */
export const HEALTH_POLL_INTERVAL_MS = 5_000;

/** How often the document list refreshes while the dashboard is visible (ms). */
export const DOCUMENTS_POLL_INTERVAL_MS = 15_000;

/**
 * Silent background update check cadence (ms), also run once shortly after
 * startup. The timer itself lives in Rust (`updater::spawn_background_checks`)
 * so it keeps running while the window is hidden in the tray; this constant
 * documents the same interval for the UI side.
 */
export const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
