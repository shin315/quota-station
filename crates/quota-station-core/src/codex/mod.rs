//! Switching Codex between the ChatGPT login and a custom API provider (M8),
//! which CC Switch did before, while keeping the ChatGPT login so Codex App's
//! plugins and remote control keep working through the custom provider.
//!
//! The app writes one file, `<Codex home>/config.toml`, and in it only the
//! keys the switch owns ([`config`]): `model_provider`, and
//! `requires_openai_auth` in the custom provider's table. It backs the file
//! up first, with the keys left out of the backup, and checks it was not
//! changed meanwhile ([`files`]). It never writes `auth.json` (only its
//! shape is looked at, [`login`]); the switch never writes the
//! `[mcp_servers]` list, which only the MCP tab edits
//! ([`crate::agents::codex_mcp`]); and the custom provider's settings come
//! from the file itself: no host, URL or key is built in. It runs `ps` and, when the
//! owner confirms, `codex app-server daemon restart` ([`procs`]); it asks the
//! provider's `/models` with no key to see that it answers ([`probe`]).

pub mod config;
pub mod files;
pub mod log;
pub mod login;
pub mod probe;
pub mod procs;
pub mod resume;

use std::path::{Path, PathBuf};

use serde::Serialize;
use time::OffsetDateTime;

use crate::antigravity::fs::AppDir;
use config::{Parsed, Refusal, Target};
use files::{Record, WriteError};
use login::Login;

/// Codex's folder under the home folder, unless the owner sets another.
pub const HOME: &str = ".codex";
pub const CONFIG: &str = "config.toml";
pub const AUTH: &str = "auth.json";
/// `config.toml` is a few kilobytes; one far larger is not read.
const MAX_CONFIG: u64 = 4 * 1024 * 1024;

pub fn home(user_home: &Path, setting: Option<&Path>) -> PathBuf {
    setting.map(Path::to_path_buf).unwrap_or_else(|| user_home.join(HOME))
}

/// `config.toml` as it is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Missing,
    Unreadable(String),
    /// Not TOML Codex can read: nothing is written.
    Invalid(String),
    Ok(Box<Parsed>),
}

/// Codex's files as the app last read them. Holds no secret: the file's
/// text is dropped once parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The Codex folder is there.
    pub present: bool,
    pub file: FileState,
    /// SHA-256 of the file's bytes, sent back with a switch.
    pub hash: Option<String>,
    /// When the file last changed, Unix seconds.
    pub modified_at: Option<i64>,
    pub login: Login,
}

impl Reading {
    pub fn parsed(&self) -> Option<&Parsed> {
        match &self.file {
            FileState::Ok(p) => Some(p),
            _ => None,
        }
    }
}

fn read_bytes(file: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(file)?.take(MAX_CONFIG).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Reads `config.toml` and the shape of `auth.json`.
pub fn read(codex_home: &Path, record: &Record) -> Reading {
    let file = codex_home.join(CONFIG);
    let present = codex_home.is_dir();
    let modified_at = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);
    let (state, hash) = match read_bytes(&file) {
        Ok(bytes) => {
            let hash = files::sha256(&bytes);
            let state = match String::from_utf8(bytes) {
                Ok(text) => match config::parse(&text, record.last_custom.as_deref()) {
                    Ok(parsed) => FileState::Ok(Box::new(parsed)),
                    Err(reason) => FileState::Invalid(reason),
                },
                Err(_) => FileState::Invalid("not UTF-8".into()),
            };
            (state, Some(hash))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (FileState::Missing, None),
        Err(e) => (FileState::Unreadable(e.to_string()), None),
    };
    let store = match &state {
        FileState::Ok(p) => p.store,
        _ => config::AuthStore::File,
    };
    Reading { present, file: state, hash, modified_at, login: login::check(codex_home, store) }
}

/// The provider id a session or the log names for each side.
pub fn provider_id(selected: Option<&str>) -> String {
    selected.unwrap_or(config::DEFAULT_ID).to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Outcome {
    /// Written; `backup` is the backup's file name.
    Switched {
        target: Target,
        from: String,
        to: String,
        backup: String,
    },
    /// The file already says so: nothing written.
    Already {
        target: Target,
    },
    Refused {
        target: Target,
        refusal: Refusal,
    },
    /// The file could not be read or edited: nothing written.
    Invalid {
        target: Target,
        detail: String,
    },
    Failed {
        target: Target,
        error: WriteError,
    },
}

/// Switches to `target`, when `config.toml` still hashes to `base_hash`
/// (the file the owner saw on the sheet). Keeps the switch in the log and
/// the write in the record.
pub fn switch(
    codex_home: &Path,
    app: &AppDir,
    target: Target,
    base_hash: Option<&str>,
    now: OffsetDateTime,
) -> Outcome {
    let file = codex_home.join(CONFIG);
    let bytes = match read_bytes(&file) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return match target {
                Target::ChatGpt => Outcome::Already { target },
                Target::Custom => Outcome::Refused { target, refusal: Refusal::NoCustom },
            };
        }
        Err(e) => return Outcome::Invalid { target, detail: e.to_string() },
    };
    let hash = files::sha256(&bytes);
    if base_hash.is_some_and(|b| b != hash) {
        return Outcome::Failed { target, error: WriteError::Changed };
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Outcome::Invalid { target, detail: "not UTF-8".into() };
    };
    let mut record = files::load(app);
    let parsed = match config::parse(text, record.last_custom.as_deref()) {
        Ok(p) => p,
        Err(detail) => return Outcome::Invalid { target, detail },
    };
    let store = parsed.store;
    let login = login::check(codex_home, store);
    let plan = match config::plan(&parsed, target, login.on_disk()) {
        Ok(p) => p,
        Err(refusal) => return Outcome::Refused { target, refusal },
    };
    let new_text = match config::apply(text, &plan) {
        Ok(t) => t,
        Err(config::PatchError::Parse(detail)) => return Outcome::Invalid { target, detail },
        Err(config::PatchError::Check(what)) => {
            return Outcome::Invalid { target, detail: format!("check: {what}") }
        }
    };
    let from = provider_id(parsed.selected.as_deref());
    let to = provider_id(plan.selector.as_deref());
    if new_text == text {
        return Outcome::Already { target };
    }
    let backups = app.path().join(files::BACKUP_DIR);
    let at = now.unix_timestamp();
    match files::write(&file, &bytes, &hash, &new_text, &backups, now) {
        Ok(backup) => {
            let mcp = config::parse(&new_text, None).map(|p| config::mcp_fingerprint(&p.mcp)).ok();
            record.written_hash = Some(files::sha256(new_text.as_bytes()));
            record.written_at = Some(at);
            record.written_provider = plan.selector.clone();
            record.written_mcp = mcp;
            record.kept_hash = None;
            if let Some(c) = &parsed.custom {
                record.last_custom = Some(c.id.clone());
            }
            let _ = files::save(app, &record);
            let _ = log::append(
                app,
                &log::Line {
                    at,
                    from: from.clone(),
                    to: to.clone(),
                    result: "switched".into(),
                    restarted: None,
                },
            );
            let backup =
                backup.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            Outcome::Switched { target, from, to, backup }
        }
        Err(error) => {
            let _ = log::append(
                app,
                &log::Line { at, from, to, result: "failed".into(), restarted: None },
            );
            Outcome::Failed { target, error }
        }
    }
}

/// "Giữ như bây giờ": the file changed outside the app is the one to keep.
pub fn keep(app: &AppDir, hash: &str, parsed: Option<&Parsed>) -> std::io::Result<()> {
    let mut record = files::load(app);
    record.kept_hash = Some(hash.to_string());
    if let Some(c) = parsed.and_then(|p| p.custom.as_ref()) {
        record.last_custom = Some(c.id.clone());
    }
    files::save(app, &record)
}
