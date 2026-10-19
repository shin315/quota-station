//! Writing `config.toml` safely, its backups, and the app's record of its
//! own last write.
//!
//! A write: the text the switch was planned on and its SHA-256; the new
//! text to a temporary file next to the real one (mode 600, synced); a copy
//! of the planned-on text in the app's `backups/codex/` (the newest 10
//! stay); the real file read again and compared with the planned-on hash
//! (another program wrote it meanwhile: nothing is replaced); the rename;
//! and a read of the result. The record keeps the hash just written, so a
//! later read can tell the file changed outside the app. As CC Switch notes
//! (`live/engine.rs`), a program outside the app can still write between the
//! comparison and the rename; the record catches that afterwards.

use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::antigravity::fs::AppDir;
use crate::local_time;

/// In the app's folder.
pub const BACKUP_DIR: &str = "backups/codex";
pub const BACKUPS_KEPT: usize = 10;
const BACKUP_PREFIX: &str = "config.toml.";
/// In the app's folder: the record of the last write.
pub const RECORD: &str = "codex/state.json";

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum WriteError {
    /// The file changed after the switch was planned: nothing replaced.
    Changed,
    /// Reading, the backup, the temporary file or the rename failed.
    Io { detail: String },
    /// The file read after the rename is not what was written.
    Verify,
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::Io { detail: e.to_string() }
    }
}

static WRITES: AtomicU64 = AtomicU64::new(0);

/// The real file: a symlinked `config.toml` is written where it points, so
/// the link stays.
fn real(file: &Path) -> io::Result<PathBuf> {
    match std::fs::symlink_metadata(file) {
        Ok(m) if m.file_type().is_symlink() => std::fs::canonicalize(file),
        _ => Ok(file.to_path_buf()),
    }
}

/// Replaces `file` with `text`, when the file still hashes to `base_hash`
/// (the text the switch was planned on, `base`). Returns the backup made.
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
    let target = real(file)?;
    let dir = target.parent().ok_or_else(|| io::Error::other("no folder"))?;
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".config.toml.qs-{}-{n}", std::process::id()));
    let result = (|| {
        let mut f =
            std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        let backup = backup(base, backups, now)?;
        if sha256(&std::fs::read(&target)?) != base_hash {
            return Err(WriteError::Changed);
        }
        std::fs::rename(&tmp, &target)?;
        if std::fs::read(&target)? != text.as_bytes() {
            return Err(WriteError::Verify);
        }
        Ok(backup)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Keeps `bytes` as `config.toml.<local time>` (mode 600, folder 700), and
/// only the newest [`BACKUPS_KEPT`].
pub fn backup(bytes: &[u8], dir: &Path, now: OffsetDateTime) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    let name = local_time::stamp(now).replace(['-', ':'], "").replace('T', "-");
    let mut path = dir.join(format!("{BACKUP_PREFIX}{name}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{BACKUP_PREFIX}{name}-{n}"));
        n += 1;
    }
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    for old in backups(dir).into_iter().skip(BACKUPS_KEPT) {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

/// Backups by name, newest first: the name is the time it was taken.
pub fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(BACKUP_PREFIX))
        .map(|e| e.path())
        .collect();
    found.sort();
    found.reverse();
    found
}
