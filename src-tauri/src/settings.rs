//! Durable user preferences.
//!
//! Stored as plain JSON in `%APPDATA%\com.libreoffice-collab.app\settings.json`
//! so a support person can inspect or hand-edit it without any tooling.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// "pt" or "en". Portuguese is the product default.
    pub language: String,
    /// "system" | "light" | "dark".
    pub theme: String,
    /// When true, the mDNS browser picks the server; when false `server_url` wins.
    pub auto_discover: bool,
    /// Manual fallback address, e.g. "http://192.168.1.50:7373".
    pub server_url: String,
    pub auto_install_updates: bool,
    pub start_minimized: bool,
    /// Unix seconds of the last successful update check; 0 means never.
    pub last_update_check: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: "system".into(),
            auto_discover: true,
            server_url: String::new(),
            // Off by default: office IT usually wants to approve rollouts, and
            // the user still gets a one-click prompt.
            auto_install_updates: false,
            start_minimized: false,
            last_update_check: 0,
        }
    }
}

/// Picks the initial language from the Windows display language.
/// Anything Portuguese (pt-PT, pt-BR, …) stays Portuguese; the rest get English.
fn default_language() -> String {
    let locale = sys_locale().unwrap_or_default().to_lowercase();
    if locale.is_empty() || locale.starts_with("pt") {
        "pt".into()
    } else {
        "en".into()
    }
}

/// Reads the user's UI language from Windows without pulling in a crate.
#[cfg(target_os = "windows")]
fn sys_locale() -> Option<String> {
    // `Get-Culture` is avoided on purpose (spawning PowerShell at startup is
    // slow); the LANG/LC_ALL vars set by some corporate images are checked
    // first, then we fall back to the Win32 user default UI language.
    if let Ok(lang) = std::env::var("LANG") {
        if !lang.is_empty() {
            return Some(lang);
        }
    }
    // GetUserDefaultLocaleName via the `kernel32` stub that Tauri already links.
    extern "system" {
        fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
    }
    // LOCALE_NAME_MAX_LENGTH is 85 wide chars.
    let mut buffer = [0u16; 85];
    let written = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    if written <= 1 {
        return None;
    }
    // `written` includes the trailing NUL.
    Some(String::from_utf16_lossy(&buffer[..(written as usize - 1)]))
}

#[cfg(not(target_os = "windows"))]
fn sys_locale() -> Option<String> {
    std::env::var("LANG").ok()
}

/// In-memory copy shared with every command, wrapped so writes are serialised.
pub struct SettingsState(pub Mutex<AppSettings>);

fn settings_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_config_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(FILE_NAME))
}

/// Loads settings, falling back to defaults on a missing or corrupt file.
/// A corrupt file is never fatal — the user would have no way to recover.
pub fn load(app: &AppHandle) -> AppSettings {
    let Ok(path) = settings_path(app) else {
        return AppSettings::default();
    };
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|error| {
            log::warn!("settings.json unreadable ({error}); using defaults");
            AppSettings::default()
        }),
        Err(_) => AppSettings::default(),
    }
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> anyhow::Result<()> {
    let path = settings_path(app)?;
    fs::write(&path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

/// Convenience read of the current in-memory settings.
pub fn current(app: &AppHandle) -> AppSettings {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}
