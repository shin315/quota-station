// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Adapted from Cockpit Tools `src-tauri/src/modules/oauth.rs` (1.3.70,
// commit 244b957); modified for this app (NOTICE).

//! The OAuth client the app logs in with: Antigravity's own, so Google sees
//! the same client either way. Its id and secret never go into this public
//! repository. The app finds them in the Antigravity installed on this
//! computer ([`super::discover`]), so adding an account needs no setup.
//!
//! An `oauth-client.json` at the root of the app's Application Support
//! folder, when there, comes first: readable only by its owner (mode 600);
//! on Windows that folder is `%APPDATA%\com.shin315.quota-station`, which
//! the user profile's access list keeps to its owner:
//!
//! ```json
//! { "client_id": "<id>.apps.googleusercontent.com", "client_secret": "<secret>" }
//! ```
//!
//! The file is checked before use; a UTF-8 byte order mark, as Windows
//! editors write one, is skipped. Nothing here prints the id or the secret:
//! `Debug` hides them and the UI only learns the [`ClientStatus`].

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::discover::{Miss, Sources};
use crate::app_dir::AppDir;

pub const FILE: &str = "oauth-client.json";
/// The key name of this client, kept with every token it gets
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

/// Whether the app has a client to log in with, and why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientStatus {
    Ok,
    /// No `oauth-client.json`, and no Antigravity on this computer.
    Missing,
    /// No `oauth-client.json`, and the Antigravity on this computer is a
    /// build whose client the app does not know.
    Unrecognized,
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

static INSTALLED: OnceLock<Sources> = OnceLock::new();

/// Lets [`load`] look in `sources` when there is no `oauth-client.json`.
/// The app calls it once as it starts, with [`Sources::system`]; a test
/// gives only fake places, so no test reads the Antigravity of the computer
/// it runs on.
pub fn use_installed(sources: Sources) {
    let _ = INSTALLED.set(sources);
}

/// Reads the client: `oauth-client.json` when it is there, else the one in
/// the installed Antigravity; or says why there is none.
pub fn load(dir: &AppDir) -> Result<OAuthClient, ClientStatus> {
    load_with(dir, INSTALLED.get())
}

/// [`load`] with the given places to look in, or none.
pub fn load_with(dir: &AppDir, installed: Option<&Sources>) -> Result<OAuthClient, ClientStatus> {
    match (load_file(dir), installed) {
        (Err(ClientStatus::Missing), Some(sources)) => sources.find().map_err(|miss| match miss {
            Miss::NotInstalled => ClientStatus::Missing,
            Miss::Unrecognized => ClientStatus::Unrecognized,
        }),
        (file, _) => file,
    }
}

/// Reads `oauth-client.json`, or says why it cannot.
fn load_file(dir: &AppDir) -> Result<OAuthClient, ClientStatus> {
    let file = path(dir);
    let meta = match dir.metadata(&file) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(ClientStatus::Missing),
        Err(_) => return Err(ClientStatus::Unreadable),
    };
    if !meta.file_type().is_file() {
        return Err(ClientStatus::Unreadable);
    }
    // Windows keeps no mode: the profile's ACL keeps others out there.
    if cfg!(unix) && crate::os::mode(&meta) & 0o077 != 0 {
        return Err(ClientStatus::TooOpen);
    }
    let text = dir.read_to_string(&file).map_err(|_| ClientStatus::Unreadable)?;
    // Notepad and PowerShell 5's UTF-8 may start the file with a byte order mark.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let fields: Fields = serde_json::from_str(text).map_err(|_| ClientStatus::Invalid)?;
    OAuthClient::new(&fields.client_id, &fields.client_secret).ok_or(ClientStatus::Invalid)
}

pub fn status(dir: &AppDir) -> ClientStatus {
    load(dir).err().unwrap_or(ClientStatus::Ok)
}
