//! collab-server — the host half of LibreOffice Collab.
//!
//! Runs on one machine in the office and does three things:
//!   1. advertises itself on the LAN over mDNS so clients find it with no setup,
//!   2. lists the documents in a shared folder over a tiny REST API,
//!   3. speaks WOPI to Collabora Online so those documents can be co-authored.
//!
//! Collabora Online itself runs next to this process (see docker/docker-compose.yml).

mod api;
mod config;
mod docs;
mod mdns;
mod sessions;
mod wopi;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::get;
use axum::Router;

use config::Config;
use sessions::Sessions;

/// Shared, immutable-ish state handed to every request.
pub struct AppState {
    pub config: Config,
    pub sessions: Sessions,
}

/// Collabora uploads the whole document on every save; 256 MB is generous
/// enough for a presentation full of images and still bounded.
const MAX_UPLOAD_BYTES: usize = 256 * 1024 * 1024;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load();
    docs::ensure_dir(&config.docs_dir)?;

    banner(&config);

    // Keep the daemon alive for the whole process, otherwise the advert stops.
    let _mdns_daemon = match mdns::advertise(&config) {
        Ok(daemon) => Some(daemon),
        Err(error) => {
            eprintln!(
                "[aviso/warning] mDNS indisponivel ({error}). \
                 Os clientes terao de indicar o endereco manualmente. / \
                 mDNS unavailable; clients must enter the address manually."
            );
            None
        }
    };

    let port = config.port;
    let state = Arc::new(AppState {
        config,
        sessions: Sessions::new(),
    });

    let app = Router::new()
        // --- client-facing API -------------------------------------------
        .route("/api/health", get(api::health))
        .route("/api/documents", get(api::documents))
        .route("/api/session/:id", get(api::session))
        // --- WOPI, called by Collabora Online ----------------------------
        // The POST on the file endpoint carries lock operations. We have a
        // single writer by construction, so acknowledging them is enough.
        .route(
            "/wopi/files/:id",
            get(wopi::check_file_info).post(wopi::lock_noop),
        )
        .route(
            "/wopi/files/:id/contents",
            get(wopi::get_file).post(wopi::put_file),
        )
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    println!("[ok] A escutar em 0.0.0.0:{port} / Listening on 0.0.0.0:{port}");
    axum::serve(listener, app).await?;

    Ok(())
}

fn banner(config: &Config) {
    println!("=====================================================");
    println!(
        " LibreOffice Collab — Servidor / Server v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("=====================================================");
    println!(" Nome / Name        : {}", mdns::describe(config));
    println!(" Pasta / Folder     : {}", config.docs_dir.display());
    println!(" Collabora Online   : {}", config.cool_url);
    println!(" Endereco / Address : {}", config.base_url());
    println!("-----------------------------------------------------");
    println!(" Deixe esta janela aberta. Feche-a para parar o servidor.");
    println!(" Keep this window open. Closing it stops the server.");
    println!("=====================================================");
}
