//! REST API consumed by the desktop client.
//!
//! Three endpoints, all unauthenticated because the server is meant to live on
//! a trusted office LAN behind the mDNS advert. Put it behind a reverse proxy
//! with auth if you ever expose it beyond that.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::docs;
use crate::AppState;

/// GET /api/health — liveness probe and identity, also used by mDNS fallback.
pub async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "name": state.config.name,
        "version": env!("CARGO_PKG_VERSION"),
        "cool_url": state.config.cool_url,
        "documents": docs::list(&state.config.docs_dir).len(),
    }))
}

/// GET /api/documents — everything editable in the shared folder.
pub async fn documents(State(state): State<Arc<AppState>>) -> Json<Vec<docs::DocumentDto>> {
    Json(docs::list(&state.config.docs_dir))
}

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    /// BCP-47 tag that localises the Collabora toolbars, e.g. "pt-PT".
    pub lang: Option<String>,
    /// Display name shown on the collaborator's cursor.
    pub user: Option<String>,
}

/// GET /api/session/{id} — mints a WOPI token and returns the editor URL.
///
/// The client never builds this URL itself: keeping it here means the token,
/// the WOPISrc address and the Collabora location can all change server-side
/// without shipping a new client.
pub async fn session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Fail fast if the document vanished between listing and opening.
    docs::resolve(&state.config.docs_dir, &id).ok_or(StatusCode::NOT_FOUND)?;

    let user = query
        .user
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Utilizador".to_string());
    let lang = query.lang.unwrap_or_else(|| "pt-PT".to_string());

    let token = state.sessions.create(&id, &user);
    let wopi_src = format!("{}/wopi/files/{}", state.config.base_url(), id);

    let editor_url = format!(
        "{}/browser/dist/cool.html?WOPISrc={}&access_token={}&lang={}",
        state.config.cool_url,
        urlencode(&wopi_src),
        urlencode(&token),
        urlencode(&lang),
    );

    Ok(Json(json!({
        "editor_url": editor_url,
        "wopi_src": wopi_src,
    })))
}

/// Percent-encodes a query-string value (RFC 3986 unreserved set).
pub fn urlencode(value: &str) -> String {
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
