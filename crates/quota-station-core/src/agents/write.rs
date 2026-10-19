//! Writing an agent's file safely, and the backups kept before each write.
//! The Codex switch (M8) and the MCP tab (M9) both write through here.
//!
//! A write: the bytes the edit was planned on (or that the file was not
//! there); the new text to a temporary file next to the real one (mode 600,
//! synced); a backup of the planned-on text **with its secrets left out**
//! ([`super::redact`]) in the app's `backups/<agent>/` (the newest 10
//! stay); the real file read again and compared with the planned-on bytes
//! (another program wrote it meanwhile: nothing is replaced); the rename;
//! and a read of the result. As CC Switch notes (`live/engine.rs`), a
//! program outside the app can still write between the comparison and the
//! rename; the hash the app keeps of its write catches that afterwards.

use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::local_time;

pub const BACKUPS_KEPT: usize = 10;

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum WriteError {
    /// The file changed after the edit was planned: nothing replaced.
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

/// Where a write keeps its backup: the folder, the file name's start
/// (`config.toml.`), and the planned-on bytes with the secrets left out.
pub struct Backup<'a> {
    pub dir: &'a Path,
    pub prefix: &'a str,
    pub bytes: &'a [u8],
}

static WRITES: AtomicU64 = AtomicU64::new(0);

/// The real file: a symlinked file is written where it points, so the link
/// stays.
fn real(file: &Path) -> io::Result<PathBuf> {
    match std::fs::symlink_metadata(file) {
        Ok(m) if m.file_type().is_symlink() => std::fs::canonicalize(file),
        _ => Ok(file.to_path_buf()),
    }
}

/// The file's bytes now; `None` when it is not there.
fn current(file: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(file) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Replaces `file` with `text`, when it still holds `base` (`None`: when it
/// is still not there; its folder must be). Returns the backup made, if
/// there was a file to back up.
pub fn write(
    file: &Path,
    base: Option<&[u8]>,
    text: &str,
    backup: Option<Backup<'_>>,
    now: OffsetDateTime,
) -> Result<Option<PathBuf>, WriteError> {
    let target = real(file)?;
    let dir = target.parent().ok_or_else(|| io::Error::other("no folder"))?;
    let name = target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{name}.qs-{}-{n}", std::process::id()));
    let result = (|| {
        let mut f =
            std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        let kept = match (base, backup) {
            (Some(_), Some(b)) => Some(keep(b.bytes, b.dir, b.prefix, now)?),
            _ => None,
        };
        if current(&target)?.as_deref() != base {
            return Err(WriteError::Changed);
        }
        std::fs::rename(&tmp, &target)?;
        if std::fs::read(&target)? != text.as_bytes() {
            return Err(WriteError::Verify);
        }
        Ok(kept)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Keeps `bytes` as `<prefix><local time>` (mode 600, folder 700), and
/// only the newest [`BACKUPS_KEPT`] with that prefix.
pub fn keep(bytes: &[u8], dir: &Path, prefix: &str, now: OffsetDateTime) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    let name = local_time::stamp(now).replace(['-', ':'], "").replace('T', "-");
    let mut path = dir.join(format!("{prefix}{name}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{prefix}{name}-{n}"));
        n += 1;
    }
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    for old in list(dir, prefix).into_iter().skip(BACKUPS_KEPT) {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

/// Backups with `prefix` by name, newest first: the name is the time it
/// was taken.
pub fn list(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .map(|e| e.path())
        .collect();
    found.sort();
    found.reverse();
    found
}

/// Leaves the secrets out of the backups already in `dir`: before M9 the
/// Codex switch kept `config.toml` whole. A backup that changes is
/// rewritten in place with its time kept; one that cannot be read as the
/// file it copies is removed, since it might hold a key. Returns how many
/// changed. Running it again changes nothing.
pub fn scrub(dir: &Path, prefix: &str, redact: fn(&str) -> Result<String, String>) -> usize {
    let mut changed = 0;
    for path in list(dir, prefix) {
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if !meta.file_type().is_file() {
            continue;
        }
        let text = std::fs::read(&path).ok().and_then(|b| String::from_utf8(b).ok());
        match text.as_deref().map(redact) {
            Some(Ok(clean)) if Some(clean.as_str()) == text.as_deref() => {}
            Some(Ok(clean)) => {
                let mut tmp = path.clone().into_os_string();
                tmp.push(".qs-scrub");
                let tmp = PathBuf::from(tmp);
                let written = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&tmp)
                    .and_then(|mut f| {
                        f.write_all(clean.as_bytes())?;
                        f.sync_all()?;
                        if let Ok(at) = meta.modified() {
                            f.set_modified(at)?;
                        }
                        Ok(())
                    })
                    .and_then(|()| std::fs::rename(&tmp, &path));
                if written.is_err() {
                    let _ = std::fs::remove_file(&tmp);
                    let _ = std::fs::remove_file(&path);
                }
                changed += 1;
            }
            _ => {
                let _ = std::fs::remove_file(&path);
                changed += 1;
            }
        }
    }
    changed
}
