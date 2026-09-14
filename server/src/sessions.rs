//! Short-lived WOPI access tokens.
//!
//! One token is minted per (document, user) when a client asks to open a file.
//! Collabora then presents it on every WOPI call. Tokens are kept in memory
//! only — restarting the server simply invalidates them, which is the correct
//! behaviour for a LAN tool.

use std::collections::HashMap;
use std::sync::Mutex;

use rand::distributions::Alphanumeric;
use rand::Rng;

/// Long enough to cover a working day of editing without a surprise logout.
const TTL_SECONDS: i64 = 12 * 60 * 60;

#[derive(Debug, Clone)]
pub struct Session {
    pub doc_id: String,
    pub user: String,
    pub expires_at: i64,
}

#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, Session>>);

impl Sessions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a token for one document and returns it.
    pub fn create(&self, doc_id: &str, user: &str) -> String {
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect();

        if let Ok(mut map) = self.0.lock() {
            let now = now_seconds();
            // Opportunistic cleanup keeps the map from growing forever.
            map.retain(|_, session| session.expires_at > now);
            map.insert(
                token.clone(),
                Session {
                    doc_id: doc_id.to_string(),
                    user: user.to_string(),
                    expires_at: now + TTL_SECONDS,
                },
            );
        }

        token
    }

    /// Returns the session when the token is valid *for this document*.
    ///
    /// Binding the token to the document matters: without it, a token handed
    /// out for a harmless file would grant write access to every other one.
    pub fn validate(&self, token: &str, doc_id: &str) -> Option<Session> {
        let map = self.0.lock().ok()?;
        let session = map.get(token)?;
        if session.doc_id != doc_id || session.expires_at <= now_seconds() {
            return None;
        }
        Some(session.clone())
    }
}

pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
