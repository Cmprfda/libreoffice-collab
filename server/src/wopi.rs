//! Minimal WOPI host implementation.
//!
//! This is the piece that makes true simultaneous editing possible. Collabora
//! Online is the single writer; it fetches the document from us, keeps every
//! participant's changes in one in-memory model, and pushes the merged result
//! back with PutFile. Clients never open the file themselves, so LibreOffice's
//! file-level lock never comes into play.
//!
//! Implemented operations: CheckFileInfo, GetFile, PutFile. Locks are accepted
//! and ignored on purpose — there is exactly one writer by construction.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::docs;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    #[serde(default)]
    pub access_token: String,
}

/// Validates the token and resolves the document, or fails the WOPI call.
fn authorise(
    state: &Arc<AppState>,
    id: &str,
    token: &str,
) -> Result<(std::path::PathBuf, String, String), StatusCode> {
    let session = state
        .sessions
        .validate(token, id)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let (path, name) = docs::resolve(&state.config.docs_dir, id).ok_or(StatusCode::NOT_FOUND)?;

    Ok((path, name, session.user))
}

/// GET /wopi/files/{id} — tells Collabora what it is about to open.
pub async fn check_file_info(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<TokenQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (path, name, user) = authorise(&state, &id, &query.access_token)?;

    let metadata = std::fs::metadata(&path).map_err(|_| StatusCode::NOT_FOUND)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    Ok(Json(json!({
        "BaseFileName": name,
        "Size": metadata.len(),
        "OwnerId": "collab-server",
        // One WOPI user per Windows account, so Collabora shows real names and
        // gives each person their own cursor colour.
        "UserId": user,
        "UserFriendlyName": user,
        "UserCanWrite": true,
        // We do not implement PutRelativeFile ("Save as"), so say so up front.
        "UserCanNotWriteRelative": true,
        "SupportsUpdate": true,
        "SupportsLocks": false,
        "SupportsRename": false,
        "DisablePrint": false,
        "DisableExport": false,
        "DisableCopy": false,
        "HideUserList": "",
        "LastModifiedTime": rfc3339(modified as i64),
        // Version must change whenever the bytes change, or Collabora will
        // serve a stale document from its cache.
        "Version": format!("{}_{}", modified, metadata.len()),
    })))
}

/// GET /wopi/files/{id}/contents — hands the raw document to Collabora.
pub async fn get_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<TokenQuery>,
) -> Result<Response, StatusCode> {
    // The route is "/wopi/files/{id}/contents"; axum gives us just the id.
    let (path, _, _) = authorise(&state, &id, &query.access_token)?;

    let bytes = std::fs::read(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let version = version_of(&path);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::HeaderName::from_static("x-wopi-itemversion"), version),
        ],
        bytes,
    )
        .into_response())
}

/// POST /wopi/files/{id}/contents — Collabora saving the merged document.
pub async fn put_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<TokenQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, StatusCode> {
    let (path, name, user) = authorise(&state, &id, &query.access_token)?;

    if body.is_empty() {
        // Never truncate a document because of an empty autosave.
        return Err(StatusCode::BAD_REQUEST);
    }

    docs::write_atomic(&path, &body).map_err(|error| {
        eprintln!("[wopi] failed to save {name}: {error}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let reason = headers
        .get("x-lool-wopi-timestamp")
        .or_else(|| headers.get("x-cool-wopi-timestamp"))
        .and_then(|value| value.to_str().ok())
        .unwrap_or("autosave");
    println!("[wopi] saved {name} ({} bytes) by {user} [{reason}]", body.len());

    Ok((
        StatusCode::OK,
        [(
            header::HeaderName::from_static("x-wopi-itemversion"),
            version_of(&path),
        )],
        Json(json!({ "LastModifiedTime": rfc3339(crate::sessions::now_seconds()) })),
    )
        .into_response())
}

/// POST /wopi/files/{id} — LOCK / UNLOCK / REFRESH_LOCK.
///
/// Collabora is the only process that ever writes these files, so there is no
/// second writer to lock out. Acknowledging the operation keeps the protocol
/// happy without inventing state we would then have to expire.
pub async fn lock_noop(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (path, _, _) = authorise(&state, &id, &query.access_token)?;

    let lock = headers
        .get("x-wopi-lock")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    Ok((
        StatusCode::OK,
        [
            (header::HeaderName::from_static("x-wopi-lock"), lock),
            (
                header::HeaderName::from_static("x-wopi-itemversion"),
                version_of(&path),
            ),
        ],
        // Empty body: the tuple needs a final `IntoResponse` element.
        "",
    )
        .into_response())
}

fn version_of(path: &std::path::Path) -> String {
    std::fs::metadata(path)
        .ok()
        .map(|metadata| {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            format!("{}_{}", modified, metadata.len())
        })
        .unwrap_or_else(|| "0_0".to_string())
}

/// WOPI wants ISO-8601 / RFC-3339 timestamps in UTC.
fn rfc3339(unix_seconds: i64) -> String {
    chrono::DateTime::from_timestamp(unix_seconds, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).expect("epoch is valid"))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
