//! System tray icon.
//!
//! Closing the main window hides it instead of quitting, which is what office
//! users expect from a background collaboration client. The tray is the only
//! way to actually exit, and its labels follow the selected language.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::i18n;

pub const TRAY_ID: &str = "main-tray";

/// Builds the tray icon on first launch.
pub fn create(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let menu = build_menu(app, language)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("LibreOffice Collab")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Double-click is the Windows convention for "restore from tray".
            if let TrayIconEvent::DoubleClick { .. } = event {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Re-labels the menu after the user switches language, with no restart.
pub fn refresh_language(app: &AppHandle, language: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = build_menu(app, language) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn build_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let strings = i18n::strings(language);
    let show = MenuItem::with_id(app, "show", strings.tray_show, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", strings.tray_quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &separator, &quit])
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
