// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Ported from Cockpit Tools `src-tauri/src/modules/quota.rs` (1.3.70,
// commit 244b957, CC BY-NC-SA 4.0): `QuotaApiCacheRecord`, `hash_email`,
// `api_cache_path`, `read_api_cache`, `is_api_cache_valid`,
// `write_api_cache` and the 60-second TTL; changes: the folder is the app's
// `antigravity/` folder, files go through the folder guard (mode 600 in a
// mode 700 folder, written through a temporary file), a write failure is
// returned, and the clock is passed in.

//! The quota cache, in Cockpit's layout, and what quota-axi reads: one file
//! per account at `antigravity/cache/quota_api_v1_desktop/authorized/`,
//! named by the SHA-256 of the email (trimmed, lower case), holding Google's
//! `fetchAvailableModels` answer with the `retrieveUserQuotaSummary` answer
//! under `quota_summary`. With the account index beside it, quota-axi
//! reports these accounts as it reports Cockpit's, so the windows and
//! Firstmate read the same numbers.
//!
//! The same files are the app's cache: a reading younger than 60 seconds is
//! used instead of asking Google again.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::index;
use crate::app_dir::AppDir;

/// Under the app's `antigravity/` folder, as under Cockpit's data folder.
pub const CACHE_DIR: [&str; 3] = ["cache", "quota_api_v1_desktop", "authorized"];
pub const SOURCE: &str = "authorized";
pub const CUSTOM_SOURCE: &str = "desktop";
pub const VERSION: u8 = 1;
/// A reading younger than this is used as it is.
pub const TTL_MS: i64 = 60_000;

/// One account's last reading, as Cockpit writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub version: u8,
    pub source: String,
    pub custom_source: String,
    pub email: String,
    pub project_id: Option<String>,
    /// Unix milliseconds.
    pub updated_at: i64,
    pub payload: serde_json::Value,
}

impl Record {
    /// Younger than [`TTL_MS`].
    pub fn fresh(&self, now_ms: i64) -> bool {
        now_ms - self.updated_at < TTL_MS
    }

    /// Written for a project the calls no longer name, or for the consumer
    /// placeholder: not used, as Cockpit does not use it.
    pub fn usable_for(&self, project_id: Option<&str>) -> bool {
        let dirty = self.project_id.as_deref().is_some_and(|p| p.trim() == "aicode-consumers");
        !dirty && self.project_id.as_deref() == project_id
    }
}

fn hash_email(email: &str) -> String {
    let normalized = email.trim().to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn path(app: &AppDir, email: &str) -> PathBuf {
    CACHE_DIR
        .iter()
        .fold(index::dir(app), |p, part| p.join(part))
        .join(format!("{}.json", hash_email(email)))
}

/// The account's reading; none, or one in another layout, reads as none.
pub fn read(app: &AppDir, email: &str) -> Option<Record> {
    let content = app.read_to_string(&path(app, email)).ok()?;
    let record = serde_json::from_str::<Record>(&content).ok()?;
    if record.version != VERSION || record.source != SOURCE {
        return None;
    }
    Some(record)
}

/// Keeps a new reading of the account.
pub fn write(
    app: &AppDir,
    email: &str,
    project_id: Option<String>,
    payload: serde_json::Value,
    now_ms: i64,
) -> std::io::Result<()> {
    let record = Record {
        version: VERSION,
        source: SOURCE.to_string(),
        custom_source: CUSTOM_SOURCE.to_string(),
        email: email.to_string(),
        project_id,
        updated_at: now_ms,
        payload,
    };
    let content = serde_json::to_string_pretty(&record).map_err(std::io::Error::other)?;
    app.write_private(&path(app, email), &content)
}

/// Forgets the account's reading, at logout.
pub fn remove(app: &AppDir, email: &str) -> std::io::Result<()> {
    app.remove(&path(app, email))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_named_by_the_digest_of_the_email_in_lower_case() {
        let app = AppDir::new("/tmp/qs-app");
        // `printf '%s' someone@example.com | shasum -a 256`
        let expected = "/tmp/qs-app/antigravity/cache/quota_api_v1_desktop/authorized/\
             72497f475e4f76d0b28f57c73a084ece576d170874eba3ee2609d9afe4b71aab.json";
        assert_eq!(path(&app, " Someone@Example.com ").to_string_lossy(), expected);
    }

    #[test]
    fn a_reading_is_fresh_for_a_minute_and_only_for_its_project() {
        let record = Record {
            version: VERSION,
            source: SOURCE.into(),
            custom_source: CUSTOM_SOURCE.into(),
            email: "someone@example.com".into(),
            project_id: Some("p-1".into()),
            updated_at: 1_000_000,
            payload: serde_json::json!({}),
        };
        assert!(record.fresh(1_000_000 + 59_999));
        assert!(!record.fresh(1_000_000 + 60_000));
        assert!(record.usable_for(Some("p-1")));
        assert!(!record.usable_for(None));
        assert!(!record.usable_for(Some("p-2")));
        let dirty = Record { project_id: Some("aicode-consumers".into()), ..record };
        assert!(!dirty.usable_for(Some("aicode-consumers")));
    }
}
