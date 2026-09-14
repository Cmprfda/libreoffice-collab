//! HTTP client for the `collab-server` host, plus the editor window launcher.
//!
//! Why the editor lives in its own window: Collabora Online is a full web app
//! and it needs the whole viewport. Running it in a separate `WebviewWindow`
//! also means a crashing document cannot take the dashboard down with it.

use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;

use crate::discovery::DiscoveredServer;

/// Shared client: connection pooling matters because the dashboard polls health
/// every five seconds. Also reused for the GitHub Releases API (see `github`).
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            // A LAN server either answers fast or is not there.
            .timeout(Duration::from_secs(4))
            .connect_timeout(Duration::from_secs(2))
            .user_agent(concat!("libreoffice-collab/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client")
    })
}

#[derive(Debug, Deserialize)]
pub struct Health {
    pub ok: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub cool_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDocument {
    pub id: String,
    pub name: String,
    pub extension: String,
    /// text | spreadsheet | presentation | drawing | other
    pub kind: String,
    pub size: u64,
    /// Unix seconds.
    pub modified: i64,
}

#[derive(Debug, Deserialize)]
struct SessionResponse {
    /// Fully-formed Collabora URL, including WOPISrc and the access token.
    editor_url: String,
}

/// Normalises whatever the user typed: adds the scheme, drops a trailing slash.
pub fn normalise_base_url(input: &str) -> String {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

pub async fn health(base_url: &str) -> anyhow::Result<Health> {
    let url = format!("{}/api/health", normalise_base_url(base_url));
    let health = client()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Health>()
        .await?;
    Ok(health)
}

pub async fn documents(base_url: &str) -> anyhow::Result<Vec<SharedDocument>> {
    let url = format!("{}/api/documents", normalise_base_url(base_url));
    let docs = client()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<SharedDocument>>()
        .await?;
    Ok(docs)
}

/// Turns a hand-typed address into a `DiscoveredServer` by asking it who it is.
pub async fn probe(base_url: &str) -> anyhow::Result<DiscoveredServer> {
    let base = normalise_base_url(base_url);
    let health = health(&base).await?;
    let parsed = Url::parse(&base)?;

    Ok(DiscoveredServer {
        id: "manual".into(),
        name: if health.name.is_empty() {
            base.clone()
        } else {
            health.name
        },
        host: parsed.host_str().unwrap_or_default().to_string(),
        port: parsed.port().unwrap_or(7373),
        base_url: base,
        cool_url: health.cool_url,
        version: health.version,
        manual: true,
    })
}

/// Asks the host to mint a WOPI session and opens the returned Collabora URL.
///
/// The access token is created server-side and is short-lived, so it never has
/// to be stored on the client.
pub async fn open_editor(
    app: &AppHandle,
    base_url: &str,
    doc_id: &str,
    doc_name: &str,
    locale: &str,
) -> anyhow::Result<()> {
    let label = window_label(doc_id);

    // Already open? Just bring it forward instead of loading a second session.
    if let Some(existing) = app.get_webview_window(&label) {
        existing.unminimize().ok();
        existing.set_focus().ok();
        return Ok(());
    }

    // The Windows account name becomes the collaborator label Collabora shows
    // next to each cursor — nobody has to create an account anywhere.
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "Utilizador".to_string());

    let session_url = format!(
        "{}/api/session/{}?lang={}&user={}",
        normalise_base_url(base_url),
        urlencode(doc_id),
        urlencode(locale),
        urlencode(&user)
    );

    let session = client()
        .get(session_url)
        .send()
        .await?
        .error_for_status()?
        .json::<SessionResponse>()
        .await?;

    let editor_url = Url::parse(&session.editor_url)?;

    WebviewWindowBuilder::new(app, label, WebviewUrl::External(editor_url))
        .title(format!("{doc_name} — LibreOffice Collab"))
        .inner_size(1360.0, 900.0)
        .min_inner_size(800.0, 560.0)
        .center()
        .maximized(true)
        // Native decorations here: this window is a document, and users expect
        // the standard Windows caption buttons on it.
        .decorations(true)
        .build()?;

    Ok(())
}

/// Window labels may only contain alphanumerics, `-`, `/`, `:` and `_`.
fn window_label(doc_id: &str) -> String {
    let safe: String = doc_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("editor_{safe}")
}

/// Minimal percent-encoding for query values; avoids pulling in another crate.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
