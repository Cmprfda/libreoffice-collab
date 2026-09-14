//! LibreOffice Collab — Windows 11 client.
//!
//! Responsibilities of the Rust side:
//!   * own the settings file,
//!   * browse the LAN for collaboration hosts over mDNS,
//!   * talk to the host's REST API and open Collabora editor windows,
//!   * run the signed auto-updater and the system tray.
//!
//! Everything user-visible lives in the WebView; nothing here renders UI except
//! the tray menu and native Windows toasts.

mod discovery;
mod github;
mod i18n;
mod server;
mod settings;
mod tray;
mod updater;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, State, WindowEvent};

use discovery::{DiscoveredServer, Discovery};
use server::SharedDocument;
use settings::{AppSettings, SettingsState};
use updater::UpdateInfo;

/// Records whether the Mica backdrop was actually applied, so the UI can fall
/// back to an opaque background on Windows 10 or in a VM.
struct MicaState(AtomicBool);

/// Commands return plain strings on failure; the UI only ever displays them.
type CmdResult<T> = Result<T, String>;

fn to_string_err<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

/* ------------------------------------------------------------------ commands */

#[tauri::command]
fn get_settings(state: State<'_, SettingsState>) -> CmdResult<AppSettings> {
    state
        .0
        .lock()
        .map(|guard| guard.clone())
        .map_err(|_| "settings lock poisoned".to_string())
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    settings: AppSettings,
) -> CmdResult<AppSettings> {
    let language_changed = {
        let mut guard = state
            .0
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        let changed = guard.language != settings.language;
        *guard = settings.clone();
        changed
    };

    settings::save(&app, &settings).map_err(to_string_err)?;

    // The tray menu is the only Rust-rendered UI, so it has to be re-labelled
    // explicitly when the language changes.
    if language_changed {
        tray::refresh_language(&app, &settings.language);
    }

    Ok(settings)
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn mica_supported(state: State<'_, MicaState>) -> bool {
    state.0.load(Ordering::Relaxed)
}

#[tauri::command]
fn list_servers(discovery: State<'_, Discovery>) -> Vec<DiscoveredServer> {
    discovery.snapshot()
}

#[tauri::command]
fn restart_discovery(app: AppHandle, discovery: State<'_, Discovery>) -> CmdResult<()> {
    discovery.start(app).map_err(to_string_err)
}

#[tauri::command]
async fn check_server(base_url: String) -> bool {
    matches!(server::health(&base_url).await, Ok(health) if health.ok)
}

#[tauri::command]
async fn probe_manual_server(base_url: String) -> CmdResult<DiscoveredServer> {
    server::probe(&base_url).await.map_err(to_string_err)
}

#[tauri::command]
async fn list_documents(base_url: String) -> CmdResult<Vec<SharedDocument>> {
    server::documents(&base_url).await.map_err(to_string_err)
}

#[tauri::command]
async fn open_document(
    app: AppHandle,
    base_url: String,
    doc_id: String,
    doc_name: String,
    locale: String,
) -> CmdResult<()> {
    server::open_editor(&app, &base_url, &doc_id, &doc_name, &locale)
        .await
        .map_err(to_string_err)
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> CmdResult<UpdateInfo> {
    updater::check(&app).await.map_err(to_string_err)
}

#[tauri::command]
async fn install_update(app: AppHandle) -> CmdResult<()> {
    updater::install(&app).await.map_err(to_string_err)
}

/* ---------------------------------------------------------------------- run */

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Launching the app a second time (desktop shortcut, Start menu) just
        // brings the existing window forward instead of starting a new client.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // 1. Settings first: everything else reads the language from here.
            let loaded = settings::load(&handle);
            let language = loaded.language.clone();
            let start_minimized = loaded.start_minimized;
            app.manage(SettingsState(std::sync::Mutex::new(loaded)));

            // 2. Window chrome. Mica is a Windows 11 feature; failing to apply
            //    it is not an error, the UI just paints an opaque background.
            let mica_ok = apply_mica(&handle);
            app.manage(MicaState(AtomicBool::new(mica_ok)));

            if start_minimized {
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // 3. Tray, discovery, updates.
            if let Err(error) = tray::create(&handle, &language) {
                log::error!("could not create the tray icon: {error}");
            }

            let discovery = Discovery::new();
            if let Err(error) = discovery.start(handle.clone()) {
                log::error!("mDNS discovery failed to start: {error}");
            }
            app.manage(discovery);

            updater::spawn_background_checks(handle);

            Ok(())
        })
        .on_window_event(|window, event| {
            // The main window hides to the tray; document windows close for real.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            app_version,
            mica_supported,
            list_servers,
            restart_discovery,
            check_server,
            probe_manual_server,
            list_documents,
            open_document,
            check_for_updates,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LibreOffice Collab");
}

/// Applies the Windows 11 Mica material to the main window.
/// Returns false on Windows 10 and on machines where DWM refuses the effect.
fn apply_mica(app: &AppHandle) -> bool {
    #[cfg(target_os = "windows")]
    {
        use tauri::window::{Effect, EffectsBuilder};
        if let Some(window) = app.get_webview_window("main") {
            return window
                .set_effects(EffectsBuilder::new().effect(Effect::Mica).build())
                .is_ok();
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        false
    }
}
