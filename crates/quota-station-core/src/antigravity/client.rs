// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Adapted from Cockpit Tools `src-tauri/src/modules/oauth.rs` (1.3.70,
// commit 244b957, CC BY-NC-SA 4.0): the client's key name and the one
// client it logs in with; changes: the id and the secret come from
// `oauth-client.json` instead of constants in the source, and the file is
// checked before use.

//! The OAuth client the app logs in with: Antigravity's own, the one Cockpit
//! Tools uses, so Google sees the same client either way. Its id and secret
//! never go into this public repository. The owner puts them in
//! `oauth-client.json` at the root of the app's Application Support folder,
//! readable only by them (mode 600):
//!
//! ```json
//! { "client_id": "<id>.apps.googleusercontent.com", "client_secret": "<secret>" }
//! ```
//!
//! Nothing here prints the id or the secret: `Debug` hides them and the UI
//! only learns the file's [`ClientStatus`].

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app_dir::AppDir;

pub const FILE: &str = "oauth-client.json";
/// Cockpit's name for this client, kept with every token it gets
/// (`TokenData.oauth_client_key`).
pub const CLIENT_KEY: &str = "antigravity_enterprise";
const ID_SUFFIX: &str = ".apps.googleusercontent.com";

#[derive(Clone, PartialEq, Eq)]
pub struct OAuthClient {
    id: String,
    secret: String,
}

impl std::fmt::Debug for OAuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OAuthClient(…)")
    }
}

impl OAuthClient {
    /// `None` unless both look like a Google OAuth client: an id ending in
    /// `.apps.googleusercontent.com` and a secret, neither with spaces. The
    /// README's placeholders do not pass.
    pub fn new(id: &str, secret: &str) -> Option<Self> {
        let plain = |s: &str| !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '<');
        let id = id.trim();
        let secret = secret.trim();
        let ok =
            plain(id) && id.len() > ID_SUFFIX.len() && id.ends_with(ID_SUFFIX) && plain(secret);
        ok.then(|| OAuthClient { id: id.to_string(), secret: secret.to_string() })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn secret(&self) -> &str {
        &self.secret
    }
}

/// What the app found in `oauth-client.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientStatus {
    Ok,
    Missing,
    /// Not a plain file the app can read.
    Unreadable,
    /// Group or others may read it: the app refuses it until it is 600.
    TooOpen,
    /// Not JSON, or the id or the secret is missing or still a placeholder.
    Invalid,
}

#[derive(Deserialize)]
struct Fields {
    client_id: String,
    client_secret: String,
}

pub fn path(dir: &AppDir) -> PathBuf {
    dir.path().join(FILE)
}

/// Reads the client, or says why it cannot.
pub fn load(dir: &AppDir) -> Result<OAuthClient, ClientStatus> {
    let file = path(dir);
    let meta = match dir.metadata(&file) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(ClientStatus::Missing),
        Err(_) => return Err(ClientStatus::Unreadable),
    };
    if !meta.file_type().is_file() {
        return Err(ClientStatus::Unreadable);
    }
    if meta.permissions().mode() & 0o077 != 0 {
        return Err(ClientStatus::TooOpen);
    }
    let text = dir.read_to_string(&file).map_err(|_| ClientStatus::Unreadable)?;
    let fields: Fields = serde_json::from_str(&text).map_err(|_| ClientStatus::Invalid)?;
    OAuthClient::new(&fields.client_id, &fields.client_secret).ok_or(ClientStatus::Invalid)
}

pub fn status(dir: &AppDir) -> ClientStatus {
    load(dir).err().unwrap_or(ClientStatus::Ok)
}
