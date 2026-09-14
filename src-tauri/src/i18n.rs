//! Translations for the strings Rust owns: the tray menu, native Windows
//! toasts and editor window titles. The web UI has its own dictionaries in
//! `src/i18n`; these keys are deliberately kept in sync with them by name.

pub struct Strings {
    pub tray_show: &'static str,
    pub tray_quit: &'static str,
    pub update_title: &'static str,
    /// Contains a single `{version}` placeholder.
    pub update_body: &'static str,
    pub update_installed_title: &'static str,
    pub update_installed_body: &'static str,
}

const PT: Strings = Strings {
    tray_show: "Abrir LibreOffice Collab",
    tray_quit: "Sair",
    update_title: "Nova versão disponível",
    update_body: "A versão {version} já está disponível. Atualizar e reiniciar?",
    update_installed_title: "Atualização instalada",
    update_installed_body: "O LibreOffice Collab vai reiniciar para concluir.",
};

const EN: Strings = Strings {
    tray_show: "Open LibreOffice Collab",
    tray_quit: "Quit",
    update_title: "New version available",
    update_body: "Version {version} is available. Update and restart?",
    update_installed_title: "Update installed",
    update_installed_body: "LibreOffice Collab will restart to finish.",
};

/// Portuguese is the fallback for anything that is not explicitly English.
pub fn strings(language: &str) -> &'static Strings {
    match language {
        "en" => &EN,
        _ => &PT,
    }
}

/// Replaces a single `{name}` placeholder.
pub fn fill(template: &str, name: &str, value: &str) -> String {
    template.replace(&format!("{{{name}}}"), value)
}
