// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2025 Jason Young (CC Switch)
// SPDX-License-Identifier: MIT

//! Whether Codex has a ChatGPT login: the shape of `auth.json`, never its
//! values. The file is read into memory, looked at and dropped; nothing in
//! it is kept, printed or sent anywhere, and it is never written.
//!
//! Which fields count as a login is ported from CC Switch (MIT, © 2025
//! Jason Young), commit 79e754ab, `src-tauri/src/codex_config.rs`
//! (`codex_auth_has_credential_login_material`) and the store modes of
//! `codex_config_auth_store_mode`. See `NOTICE`.

use std::io::Read;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use super::config::AuthStore;

/// `auth.json` is a few kilobytes; anything far larger is not Codex's.
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Login {
    /// ChatGPT's tokens: plugins and remote control can run.
    ChatGpt,
    /// Another login Codex signs in with (an access token, an agent identity).
    Other,
    /// Only an `OPENAI_API_KEY`: no ChatGPT login.
    ApiKey,
    Missing,
    Unreadable,
    /// Kept in the system's store (`keyring`, `auto`): the Keychain on a
    /// Mac; on Windows Credential Manager holds the key of
    /// `secrets\codex_auth.age`, and the UI says Credential Manager. The app
    /// does not look.
    Keychain,
    /// `ephemeral`: never kept on disk.
    Ephemeral,
}

impl Login {
    /// Codex will find a login: the input to the `requires_openai_auth`
    /// rule. A login the app cannot see in the Keychain counts, as CC Switch
    /// counts it when it keeps the login.
    pub fn on_disk(self) -> bool {
        matches!(self, Login::ChatGpt | Login::Other | Login::Keychain)
    }
}

fn present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    }
}

/// What kind of login the parsed file holds.
pub fn classify(auth: &Value) -> Login {
    let Some(obj) = auth.as_object() else { return Login::Unreadable };
    let tokens = obj.get("tokens").and_then(Value::as_object).is_some_and(|tokens| {
        ["id_token", "access_token", "refresh_token"]
            .iter()
            .any(|key| tokens.get(*key).is_some_and(present))
    });
    if tokens {
        return Login::ChatGpt;
    }
    if ["personal_access_token", "agent_identity", "bedrock_api_key"]
        .iter()
        .any(|key| obj.get(*key).is_some_and(present))
    {
        return Login::Other;
    }
    if obj.get("OPENAI_API_KEY").is_some_and(present) {
        return Login::ApiKey;
    }
    Login::Missing
}

/// Looks at `<codex home>/auth.json`, unless Codex keeps the login elsewhere.
pub fn check(codex_home: &Path, store: AuthStore) -> Login {
    match store {
        AuthStore::Keyring | AuthStore::Auto | AuthStore::Unknown => return Login::Keychain,
        AuthStore::Ephemeral => return Login::Ephemeral,
        AuthStore::File => {}
    }
    let file = codex_home.join(super::AUTH);
    let mut bytes = Vec::new();
    match std::fs::File::open(&file) {
        Ok(f) => {
            if f.take(MAX_BYTES).read_to_end(&mut bytes).is_err() {
                return Login::Unreadable;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Login::Missing,
        Err(_) => return Login::Unreadable,
    }
    let login =
        serde_json::from_slice::<Value>(&bytes).map(|v| classify(&v)).unwrap_or(Login::Unreadable);
    // The values go no further than this function.
    bytes.iter_mut().for_each(|b| *b = 0);
    login
}
