//! Writing `config.toml` safely, its backups, and the app's record of its
//! own last write.
//!
//! A write goes through [`crate::agents::write`]: the text the switch was
//! planned on and its SHA-256, a temporary file, a backup of the planned-on
//! text with its keys left out in the app's `backups/codex/` (the newest 10
//! stay), a second comparison, the rename and a read of the result. The MCP
//! tab writes the same file the same way and keeps its backups here too.
//! The record keeps the hash just written, so a later read can tell the
//! file changed outside the app.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub use crate::agents::write::{sha256, WriteError, BACKUPS_KEPT};
use crate::agents::{redact, write};
use crate::antigravity::fs::AppDir;

/// In the app's folder.
pub const BACKUP_DIR: &str = "backups/codex";
pub const BACKUP_PREFIX: &str = "config.toml.";
/// In the app's folder: the record of the last write.
pub const RECORD: &str = "codex/state.json";

/// The app's last write, and what the owner kept since.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    /// SHA-256 of the file as the app last wrote it.
    #[serde(default)]
    pub written_hash: Option<String>,
    /// Unix seconds.
    #[serde(default)]
    pub written_at: Option<i64>,
    /// `model_provider` as written; `None` for the ChatGPT login.
    #[serde(default)]
    pub written_provider: Option<String>,
    /// The MCP list's fingerprint when the app wrote.
    #[serde(default)]
    pub written_mcp: Option<String>,
    /// A file changed outside the app that the owner chose to keep.
    #[serde(default)]
    pub kept_hash: Option<String>,
    /// The custom provider last seen in use, so the switch goes back to it.
    #[serde(default)]
    pub last_custom: Option<String>,
    /// What "Hoàn tác" after "Dán khoá" puts back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_undo: Option<KeyUndo>,
}

/// The custom provider's table before "Dán khoá", as far as it held no key:
/// a key written over another key cannot be undone, since the app never
/// keeps one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyUndo {
    /// SHA-256 of the file the key write left: the undo runs only on it.
    pub hash: String,
    pub provider: String,
    /// `experimental_bearer_token` before: CC Switch's placeholder, empty,
    /// or `None` when there was none.
    pub token: Option<String>,
    /// `base_url` before, when the write changed it.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Unix seconds.
    pub at: i64,
}

pub fn record_path(app: &AppDir) -> PathBuf {
    app.path().join(RECORD)
}

pub fn load(app: &AppDir) -> Record {
    app.read_to_string(&record_path(app))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppDir, record: &Record) -> io::Result<()> {
    let text = serde_json::to_string_pretty(record).map_err(io::Error::other)?;
    app.write_private(&record_path(app), &text)
}

impl Record {
    /// The file on disk now is not the one the app wrote, nor one the owner
    /// kept. `false` before the app's first write: there is nothing to compare.
    pub fn changed_outside(&self, hash: &str) -> bool {
        self.written_hash.as_deref().is_some_and(|w| w != hash)
            && self.kept_hash.as_deref() != Some(hash)
    }

    /// The MCP tab rewrote the file from `before` to `after`: a file that
    /// was the app's own write, or one the owner kept, stays so, and the
    /// Codex tab does not report the app's own change as made outside.
    pub fn follow(&mut self, before: &str, after: &str, mcp: String) {
        // The MCP list does not touch the provider's table: the undo still fits.
        if let Some(u) = self.key_undo.as_mut().filter(|u| u.hash == before) {
            u.hash = after.to_string();
        }
        if self.written_hash.as_deref() == Some(before) {
            self.written_hash = Some(after.to_string());
            self.written_mcp = Some(mcp);
        } else if self.kept_hash.as_deref() == Some(before) {
            self.kept_hash = Some(after.to_string());
        }
    }
}

/// Replaces `file` with `text`, when the file still hashes to `base_hash`
/// (the text the switch was planned on, `base`). The backup leaves the keys
/// out ([`redact::toml`]). Returns the backup made.
pub fn write(
    file: &Path,
    base: &[u8],
    base_hash: &str,
    text: &str,
    backups: &Path,
    now: OffsetDateTime,
) -> Result<PathBuf, WriteError> {
    if sha256(base) != base_hash {
        return Err(WriteError::Changed);
    }
    let clean =
        std::str::from_utf8(base).ok().and_then(|t| redact::toml(t).ok()).ok_or_else(|| {
            WriteError::Io { detail: "the backup could not leave the keys out".into() }
        })?;
    let backup = write::Backup { dir: backups, prefix: BACKUP_PREFIX, bytes: clean.as_bytes() };
    write::write(file, Some(base), text, Some(backup), now)?.ok_or(WriteError::Verify)
}

/// Keeps `bytes` as `config.toml.<local time>` (mode 600, folder 700), and
/// only the newest [`BACKUPS_KEPT`]. The caller leaves the keys out.
pub fn backup(bytes: &[u8], dir: &Path, now: OffsetDateTime) -> io::Result<PathBuf> {
    write::keep(bytes, dir, BACKUP_PREFIX, now)
}

/// Backups by name, newest first: the name is the time it was taken.
pub fn backups(dir: &Path) -> Vec<PathBuf> {
    write::list(dir, BACKUP_PREFIX)
}

/// Leaves the keys out of backups the switch kept whole before M9.
pub fn scrub_backups(dir: &Path) -> usize {
    write::scrub(dir, BACKUP_PREFIX, redact::toml)
}
