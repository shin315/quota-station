// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// `AccountIndex`, `AccountSummary` and `account_id` are ported from Cockpit
// Tools `src-tauri/src/models/account.rs` and `modules/account.rs`
// (`build_account_storage_id`) (1.3.70, commit 244b957, CC BY-NC-SA 4.0);
// changes: the clock is passed in, the index is read and written through the
// app's folder guard, and an unreadable index is reported instead of
// rebuilt from account files. The status file is the app's own.

//! The non-secret files in the app's `antigravity/` folder:
//!
//! - `accounts.json`, the account index in Cockpit's layout (version `2.0`:
//!   id, email, name, created and last-used times, current account), so
//!   quota-axi can read it as it reads Cockpit's;
//! - `status.json`, the accounts that need the owner: a new login after
//!   Google refused the refresh token, a verification Google asks for, or
//!   an account the quota API refuses.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app_dir::AppDir;

pub const DIR: &str = "antigravity";
pub const INDEX_FILE: &str = "accounts.json";
pub const INDEX_VERSION: &str = "2.0";
pub const STATUS_FILE: &str = "status.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIndex {
    pub version: String,
    pub accounts: Vec<AccountSummary>,
    pub current_account_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSummary {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub created_at: i64,
    pub last_used: i64,
}

impl AccountIndex {
    pub fn new() -> Self {
        Self { version: INDEX_VERSION.to_string(), accounts: Vec::new(), current_account_id: None }
    }

    /// The account for `email`, compared as Cockpit does: trimmed, any case.
    pub fn find_email(&self, email: &str) -> Option<&AccountSummary> {
        let id = account_id(email);
        let email = email.trim();
        self.accounts.iter().find(|s| s.id == id || s.email.trim().eq_ignore_ascii_case(email))
    }
}

impl Default for AccountIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Cockpit's id for an account: `antigravity_` and the MD5 of the email,
/// trimmed and in lower case. The same email always gets the same id.
pub fn account_id(email: &str) -> String {
    let seed = email.trim().to_lowercase();
    format!("antigravity_{:x}", md5::compute(seed.as_bytes()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    /// The file is there and is not one this app can read. It is left alone
    /// rather than replaced. Carries the file's name (`accounts.json` or
    /// `status.json`) and why.
    Unreadable(&'static str, String),
    /// The file could not be written: its name and why.
    Write(&'static str, String),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Unreadable(file, e) => write!(f, "cannot read {file}: {e}"),
            IndexError::Write(file, e) => write!(f, "cannot write {file}: {e}"),
        }
    }
}

impl std::error::Error for IndexError {}

pub fn dir(app: &AppDir) -> PathBuf {
    app.path().join(DIR)
}

pub fn index_path(app: &AppDir) -> PathBuf {
    dir(app).join(INDEX_FILE)
}

pub fn status_path(app: &AppDir) -> PathBuf {
    dir(app).join(STATUS_FILE)
}

/// The index; none yet reads as an empty one.
pub fn load(app: &AppDir) -> Result<AccountIndex, IndexError> {
    let text = match app.read_to_string(&index_path(app)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(AccountIndex::new()),
        Err(e) => return Err(IndexError::Unreadable(INDEX_FILE, e.to_string())),
    };
    if text.trim().is_empty() {
        return Ok(AccountIndex::new());
    }
    let index: AccountIndex = serde_json::from_str(&text)
        .map_err(|e| IndexError::Unreadable(INDEX_FILE, e.to_string()))?;
    if index.version != INDEX_VERSION {
        return Err(IndexError::Unreadable(INDEX_FILE, format!("version {}", index.version)));
    }
    Ok(index)
}

pub fn save(app: &AppDir, index: &AccountIndex) -> Result<(), IndexError> {
    let text = serde_json::to_string_pretty(index)
        .map_err(|e| IndexError::Write(INDEX_FILE, e.to_string()))?;
    app.write_private(&index_path(app), &text)
        .map_err(|e| IndexError::Write(INDEX_FILE, e.to_string()))
}

/// Why an account needs the owner. Accounts that need nothing are not listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Standing {
    /// Google refused the refresh token (`invalid_grant`): only a new login
    /// helps, and nothing uses the account until then.
    NeedsLogin { since: i64 },
    /// The quota API asked for a verification (`VALIDATION_REQUIRED`) or a
    /// subscription (`SUBSCRIPTION_REQUIRED`), with Google's link when it
    /// sent one.
    NeedsVerification {
        since: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    /// The quota API answered 403 (Cockpit's `is_forbidden`): the account is
    /// not read again until a new login, with Google's verification link
    /// when it sent one.
    Forbidden {
        since: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statuses {
    #[serde(default)]
    pub accounts: BTreeMap<String, Standing>,
}

/// The statuses; none yet reads as none.
pub fn read_statuses(app: &AppDir) -> Result<Statuses, IndexError> {
    let text = match app.read_to_string(&status_path(app)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Statuses::default()),
        Err(e) => return Err(IndexError::Unreadable(STATUS_FILE, e.to_string())),
    };
    if text.trim().is_empty() {
        return Ok(Statuses::default());
    }
    serde_json::from_str(&text).map_err(|e| IndexError::Unreadable(STATUS_FILE, e.to_string()))
}

/// The statuses for the windows; an unreadable file shows as none.
pub fn load_statuses(app: &AppDir) -> Statuses {
    read_statuses(app).unwrap_or_default()
}

/// Sets or clears (`None`) one account's status. An unreadable file is left
/// as it is, so no other account loses its status.
pub fn set_standing(app: &AppDir, id: &str, standing: Option<Standing>) -> Result<(), IndexError> {
    let mut statuses = read_statuses(app)?;
    let changed = match standing {
        Some(s) => statuses.accounts.insert(id.to_string(), s.clone()) != Some(s),
        None => statuses.accounts.remove(id).is_some(),
    };
    if !changed {
        return Ok(());
    }
    let text = serde_json::to_string_pretty(&statuses)
        .map_err(|e| IndexError::Write(STATUS_FILE, e.to_string()))?;
    app.write_private(&status_path(app), &text)
        .map_err(|e| IndexError::Write(STATUS_FILE, e.to_string()))
}
