// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Switching Codex between the ChatGPT login and a custom API provider (M8),
//! which CC Switch did before, while keeping the ChatGPT login so Codex App's
//! plugins and remote control keep working through the custom provider.
//!
//! The app writes one file, `<Codex home>/config.toml`, and in it only the
//! keys the switch owns ([`config`]): `model_provider`, and
//! `requires_openai_auth` in the custom provider's table; and, when the
//! owner pastes one ("Dán khoá", [`set_key`]), the provider's own key and
//! the `base_url` that pointed at CC Switch's proxy. It backs the file
//! up first, with the keys left out of the backup, and checks it was not
//! changed meanwhile ([`files`]). It never writes `auth.json` (only its
//! shape is looked at, [`login`]); the switch never writes the
//! `[mcp_servers]` list, which only the MCP tab edits
//! ([`crate::agents::codex_mcp`]); and the custom provider's settings come
//! from the file itself: no host, URL or key is built in. It runs `ps` and, when the
//! owner confirms, `codex app-server daemon restart` ([`procs`]); it asks the
//! provider's `/models` with no key to see that it answers ([`probe`]).

pub mod accounts;
pub mod config;
pub mod files;
pub mod log;
pub mod login;
pub mod platform;
pub mod probe;
pub mod procs;
pub mod resume;

use std::path::{Path, PathBuf};

use serde::Serialize;
use time::OffsetDateTime;

use crate::agents::batch::Line as PreviewLine;
use crate::agents::def::MASK;
use crate::app_dir::AppDir;
use config::{KeyKind, KeyPlan, KeyRefusal, Parsed, Refusal, Target};
use files::{KeyUndo, Record, WriteError};
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
/// (the file the owner saw on the sheet). `plugins` is the owner's "Dùng
/// plugin khi chạy Custom API" ([`config::requires_openai_auth`]). On the
/// custom provider already, a switch to it only applies `plugins`. Keeps
/// the switch in the log and the write in the record.
pub fn switch(
    codex_home: &Path,
    app: &AppDir,
    target: Target,
    base_hash: Option<&str>,
    plugins: bool,
    now: OffsetDateTime,
) -> Outcome {
    switch_to(codex_home, app, target, None, base_hash, plugins, now)
}

/// [`switch`], to the provider table `provider` when given, else to the
/// one the file and the record choose.
fn switch_to(
    codex_home: &Path,
    app: &AppDir,
    target: Target,
    provider: Option<&str>,
    base_hash: Option<&str>,
    plugins: bool,
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
    let mut parsed = match config::parse(text, record.last_custom.as_deref()) {
        Ok(p) => p,
        Err(detail) => return Outcome::Invalid { target, detail },
    };
    if let Some(id) = provider {
        parsed.custom = parsed.providers.iter().find(|c| c.id == id).cloned();
    }
    let store = parsed.store;
    let login = login::check(codex_home, store);
    let plan = match config::plan(&parsed, target, login.on_disk(), plugins) {
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
    let result = if from == to { "plugins" } else { "switched" };
    let requires = plan.requires.as_ref().map(|(_, r)| *r);
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
            record.key_undo = None;
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
                    result: result.into(),
                    restarted: None,
                    requires,
                },
            );
            let backup =
                backup.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            Outcome::Switched { target, from, to, backup }
        }
        Err(error) => {
            let _ = log::append(
                app,
                &log::Line { at, from, to, result: "failed".into(), restarted: None, requires },
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

// ===== "Dán khoá" =====

/// What the key sheet shows before anything is written. Holds no key: only
/// [`config::hint`] of the new one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPreview {
    /// Sent back with the write, so a file changed since is not overwritten.
    pub hash: Option<String>,
    /// The lines that change, in the provider's table.
    pub lines: Vec<PreviewLine>,
    pub refusal: Option<KeyRefusal>,
    /// What the parser or the check said.
    pub detail: Option<String>,
    /// A key already sits in the table: it is replaced, and cannot be put back.
    pub replaces: bool,
    /// What "Hoàn tác" puts back: `placeholder`, `empty` or `none` (the
    /// line goes); `None` when there is no undo.
    pub undo: Option<&'static str>,
}

/// What a key write, or its undo, did. Holds no key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum KeyOutcome {
    /// `undo`: "Hoàn tác" can put the table back.
    Written {
        provider: String,
        backup: String,
        undo: bool,
    },
    Undone {
        provider: String,
        backup: String,
    },
    /// The table already holds this key and address: nothing written.
    Already,
    Refused {
        refusal: KeyRefusal,
    },
    Invalid {
        detail: String,
    },
    Failed {
        error: WriteError,
    },
}

/// The file, its hash, its text and what it says, for a key write.
struct Loaded {
    bytes: Vec<u8>,
    hash: String,
    parsed: Parsed,
}

fn load(codex_home: &Path, record: &Record) -> Result<Loaded, KeyOutcome> {
    let bytes = match read_bytes(&codex_home.join(CONFIG)) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(KeyOutcome::Refused { refusal: KeyRefusal::NoCustom })
        }
        Err(e) => return Err(KeyOutcome::Invalid { detail: e.to_string() }),
    };
    let hash = files::sha256(&bytes);
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| KeyOutcome::Invalid { detail: "not UTF-8".into() })?;
    let parsed = config::parse(text, record.last_custom.as_deref())
        .map_err(|detail| KeyOutcome::Invalid { detail })?;
    Ok(Loaded { bytes, hash, parsed })
}

impl Loaded {
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes).unwrap_or_default()
    }
}

/// The plan for the owner's `key` and `base_url`, and the edited text.
fn key_edit(
    loaded: &Loaded,
    key: &str,
    base_url: Option<&str>,
) -> Result<(KeyPlan, String), KeyOutcome> {
    let refused = |refusal| KeyOutcome::Refused { refusal };
    let key = config::clean_key(key).map_err(refused)?;
    let base_url = base_url
        .filter(|u| !u.trim().is_empty())
        .map(config::clean_url)
        .transpose()
        .map_err(refused)?;
    let plan = config::key_plan(&loaded.parsed, key, base_url).map_err(refused)?;
    let text = config::apply_key(loaded.text(), &plan).map_err(|e| match e {
        config::PatchError::Parse(detail) => KeyOutcome::Invalid { detail },
        config::PatchError::Check(what) => KeyOutcome::Invalid { detail: format!("check: {what}") },
    })?;
    Ok((plan, text))
}

fn undo_word(token: &Option<String>) -> &'static str {
    match token.as_deref() {
        None => "none",
        Some(config::PROXY_TOKEN_PLACEHOLDER) => "placeholder",
        Some(_) => "empty",
    }
}

/// The preview of [`set_key`]: the lines that change, the new key shown
/// only as its [`config::hint`].
pub fn key_preview(
    codex_home: &Path,
    app: &AppDir,
    key: &str,
    base_url: Option<&str>,
) -> KeyPreview {
    let record = files::load(app);
    let loaded = match load(codex_home, &record) {
        Ok(l) => l,
        Err(out) => return preview_refused(None, out),
    };
    let hash = Some(loaded.hash.clone());
    let (plan, _) = match key_edit(&loaded, key, base_url) {
        Ok(p) => p,
        Err(out) => return preview_refused(hash, out),
    };
    let custom = loaded.parsed.custom.as_ref();
    let old = config::old_token(loaded.text(), &plan.id);
    let line = |op: &'static str, text: String, secret: bool| PreviewLine { op, text, secret };
    let quoted = |v: &str| toml_edit::Value::from(v).to_string().trim().to_string();
    let mut lines = vec![line("ctx", format!("[model_providers.{}]", plan.id), false)];
    if let Some(old) = custom.and_then(|c| c.base_url.as_deref()) {
        if let Some(new) = plan.base_url.as_deref().filter(|n| *n != old) {
            let shown = if crate::agents::def::secret_url(old) { MASK } else { old };
            lines.push(line("del", format!("base_url = {}", quoted(shown)), false));
            lines.push(line("add", format!("base_url = {}", quoted(new)), false));
        }
    } else if let Some(new) = plan.base_url.as_deref() {
        lines.push(line("add", format!("base_url = {}", quoted(new)), false));
    }
    let token = plan.token.as_deref().unwrap_or_default();
    match &old {
        config::OldToken::NotAKey(v) => {
            lines.push(line("del", format!("{} = {}", config::BEARER_KEY, quoted(v)), false))
        }
        config::OldToken::Absent => {}
        config::OldToken::Key => {
            lines.push(line("del", format!("{} = {}", config::BEARER_KEY, quoted(MASK)), true))
        }
    }
    lines.push(line(
        "add",
        format!("{} = {}", config::BEARER_KEY, quoted(&config::hint(token))),
        true,
    ));
    KeyPreview {
        hash,
        lines,
        refusal: None,
        detail: None,
        replaces: old == config::OldToken::Key,
        undo: old.restore().map(|t| undo_word(&t)),
    }
}

fn preview_refused(hash: Option<String>, out: KeyOutcome) -> KeyPreview {
    let mut p = KeyPreview { hash, ..KeyPreview::default() };
    match out {
        KeyOutcome::Refused { refusal } => p.refusal = Some(refusal),
        KeyOutcome::Invalid { detail } => p.detail = Some(detail),
        _ => {}
    }
    p
}

/// "Dán khoá": writes `key` as the custom provider's
/// `experimental_bearer_token`, and `base_url` when given, when
/// `config.toml` still hashes to `base_hash` (the file the preview showed).
/// Backs the file up with its keys left out, keeps what an undo needs
/// (never a key), and logs it by the provider's id only. `auth.json`, the
/// provider in use and `requires_openai_auth` are not touched.
pub fn set_key(
    codex_home: &Path,
    app: &AppDir,
    key: &str,
    base_url: Option<&str>,
    base_hash: &str,
    now: OffsetDateTime,
) -> KeyOutcome {
    let mut record = files::load(app);
    let loaded = match load(codex_home, &record) {
        Ok(l) => l,
        Err(out) => return out,
    };
    if loaded.hash != base_hash {
        return KeyOutcome::Failed { error: WriteError::Changed };
    }
    let (plan, new_text) = match key_edit(&loaded, key, base_url) {
        Ok(p) => p,
        Err(out) => return out,
    };
    if new_text == loaded.text() {
        return KeyOutcome::Already;
    }
    let restore = config::old_token(loaded.text(), &plan.id).restore();
    let old_url = loaded.parsed.custom.as_ref().and_then(|c| c.base_url.clone());
    let undo = restore.map(|token| KeyUndo {
        hash: files::sha256(new_text.as_bytes()),
        provider: plan.id.clone(),
        token,
        base_url: plan.base_url.as_ref().and(old_url),
        at: now.unix_timestamp(),
    });
    let can_undo = undo.is_some();
    match write_key(codex_home, app, &mut record, &loaded, &new_text, &plan.id, "key", now) {
        Ok(backup) => {
            record.key_undo = undo;
            let _ = files::save(app, &record);
            KeyOutcome::Written { provider: plan.id, backup, undo: can_undo }
        }
        Err(error) => KeyOutcome::Failed { error },
    }
}

/// "Hoàn tác" after [`set_key`]: puts back the table's placeholder (or no
/// key) and its old `base_url`, only while the file is still the one the
/// key write left.
pub fn undo_key(
    codex_home: &Path,
    app: &AppDir,
    base_hash: &str,
    now: OffsetDateTime,
) -> KeyOutcome {
    let mut record = files::load(app);
    let Some(undo) = record.key_undo.clone() else {
        return KeyOutcome::Refused { refusal: KeyRefusal::NoUndo };
    };
    let loaded = match load(codex_home, &record) {
        Ok(l) => l,
        Err(out) => return out,
    };
    if loaded.hash != base_hash || loaded.hash != undo.hash {
        return KeyOutcome::Failed { error: WriteError::Changed };
    }
    let plan = KeyPlan { id: undo.provider.clone(), token: undo.token, base_url: undo.base_url };
    let new_text = match config::apply_key(loaded.text(), &plan) {
        Ok(t) => t,
        Err(config::PatchError::Parse(detail)) => return KeyOutcome::Invalid { detail },
        Err(config::PatchError::Check(what)) => {
            return KeyOutcome::Invalid { detail: format!("check: {what}") }
        }
    };
    match write_key(codex_home, app, &mut record, &loaded, &new_text, &plan.id, "keyUndo", now) {
        Ok(backup) => {
            record.key_undo = None;
            let _ = files::save(app, &record);
            KeyOutcome::Undone { provider: plan.id, backup }
        }
        Err(error) => KeyOutcome::Failed { error },
    }
}

/// Writes a key edit like a switch: the backup without keys, the record
/// following the file when it was the app's, and a log line with no key.
#[allow(clippy::too_many_arguments)]
fn write_key(
    codex_home: &Path,
    app: &AppDir,
    record: &mut Record,
    loaded: &Loaded,
    new_text: &str,
    id: &str,
    result: &str,
    now: OffsetDateTime,
) -> Result<String, WriteError> {
    let backups = app.path().join(files::BACKUP_DIR);
    let at = now.unix_timestamp();
    let line = |result: &str| log::Line {
        at,
        from: id.to_string(),
        to: id.to_string(),
        result: result.into(),
        restarted: None,
        requires: None,
    };
    match files::write(
        &codex_home.join(CONFIG),
        &loaded.bytes,
        &loaded.hash,
        new_text,
        &backups,
        now,
    ) {
        Ok(backup) => {
            let after = files::sha256(new_text.as_bytes());
            let mcp = config::mcp_fingerprint(&loaded.parsed.mcp);
            record.follow(&loaded.hash, &after, mcp);
            let _ = log::append(app, &line(result));
            Ok(backup.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        }
        Err(error) => {
            let _ = log::append(app, &line("failed"));
            Err(error)
        }
    }
}

/// Whether the custom provider's table takes a pasted key: CC Switch's
/// placeholder, no key, or a key to replace, in a plain table.
pub fn takes_key(parsed: &Parsed) -> bool {
    parsed.custom.as_ref().is_some_and(|c| {
        c.plain && matches!(c.key, KeyKind::Bearer | KeyKind::Placeholder | KeyKind::None)
    })
}

// ===== Several custom providers =====

/// What "Xem trước…" of a new provider shows. Holds no key: only its hint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreview {
    /// Sent back with the add, so a file changed since is not overwritten.
    pub hash: Option<String>,
    /// The table's id: the name, made fit for one.
    pub id: Option<String>,
    /// The lines the add appends.
    pub lines: Vec<PreviewLine>,
    pub refusal: Option<KeyRefusal>,
    pub detail: Option<String>,
}

/// What adding or removing a provider did. Holds no key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ProviderOutcome {
    Added { id: String, backup: String },
    Removed { id: String, backup: String },
    Refused { refusal: KeyRefusal },
    Invalid { detail: String },
    Failed { error: WriteError },
}

/// A new provider's id, its table's lines (each with whether it holds the
/// key) and the text with it.
type ProviderEdit = (String, Vec<(String, bool)>, String);

/// The new provider's table and the text with it, or why not.
fn provider_edit(
    loaded: &Loaded,
    name: &str,
    base_url: &str,
    key: &str,
) -> Result<ProviderEdit, KeyOutcome> {
    let refused = |refusal| KeyOutcome::Refused { refusal };
    let id = config::new_provider_id(&loaded.parsed, name).map_err(refused)?;
    let url = config::clean_url(base_url).map_err(refused)?;
    let key = config::clean_key(key).map_err(refused)?;
    let lines = config::provider_lines(&id, name, &url, &key);
    let text = config::apply_add(loaded.text(), &id, &lines).map_err(|e| match e {
        config::PatchError::Parse(detail) => KeyOutcome::Invalid { detail },
        config::PatchError::Check(what) => KeyOutcome::Invalid { detail: format!("check: {what}") },
    })?;
    Ok((id, lines, text))
}

/// `config.toml` for an edit that may start a file of its own: a missing
/// file reads as empty.
fn load_or_empty(codex_home: &Path, record: &Record) -> Result<Loaded, KeyOutcome> {
    match load(codex_home, record) {
        Err(KeyOutcome::Refused { refusal: KeyRefusal::NoCustom })
            if !codex_home.join(CONFIG).exists() =>
        {
            Ok(Loaded {
                hash: files::sha256(b""),
                parsed: config::parse("", None).map_err(|detail| KeyOutcome::Invalid { detail })?,
                bytes: Vec::new(),
            })
        }
        other => other,
    }
}

fn provider_outcome(out: KeyOutcome) -> ProviderOutcome {
    match out {
        KeyOutcome::Refused { refusal } => ProviderOutcome::Refused { refusal },
        KeyOutcome::Invalid { detail } => ProviderOutcome::Invalid { detail },
        KeyOutcome::Failed { error } => ProviderOutcome::Failed { error },
        _ => ProviderOutcome::Invalid { detail: "unexpected".into() },
    }
}

/// The table a new provider `name` at `base_url` with `key` would add,
/// the key shown only as its hint. Writes nothing.
pub fn provider_preview(
    codex_home: &Path,
    app: &AppDir,
    name: &str,
    base_url: &str,
    key: &str,
) -> ProviderPreview {
    let record = files::load(app);
    let loaded = match load_or_empty(codex_home, &record) {
        Ok(l) => l,
        Err(out) => {
            let p = preview_refused(None, out);
            return ProviderPreview {
                refusal: p.refusal,
                detail: p.detail,
                ..ProviderPreview::default()
            };
        }
    };
    let hash = Some(loaded.hash.clone());
    match provider_edit(&loaded, name, base_url, key) {
        Ok((id, lines, _)) => {
            let hint = config::hint(&config::clean_key(key).unwrap_or_default());
            let lines = lines
                .into_iter()
                .map(|(text, secret)| PreviewLine {
                    op: "add",
                    text: if secret {
                        format!(
                            "{} = {}",
                            config::BEARER_KEY,
                            toml_edit::Value::from(hint.as_str())
                        )
                    } else {
                        text
                    },
                    secret,
                })
                .collect();
            ProviderPreview { hash, id: Some(id), lines, refusal: None, detail: None }
        }
        Err(out) => {
            let p = preview_refused(hash, out);
            ProviderPreview {
                hash: p.hash,
                refusal: p.refusal,
                detail: p.detail,
                ..ProviderPreview::default()
            }
        }
    }
}

/// Adds the provider as a new table at the end of `config.toml`, when the
/// file still hashes to `base_hash` (the one the preview showed). Codex is
/// not switched to it. Backed up first with its keys left out; logged by id.
pub fn add_provider(
    codex_home: &Path,
    app: &AppDir,
    name: &str,
    base_url: &str,
    key: &str,
    base_hash: &str,
    now: OffsetDateTime,
) -> ProviderOutcome {
    let mut record = files::load(app);
    let loaded = match load_or_empty(codex_home, &record) {
        Ok(l) => l,
        Err(out) => return provider_outcome(out),
    };
    if loaded.hash != base_hash {
        return ProviderOutcome::Failed { error: WriteError::Changed };
    }
    let (id, _, new_text) = match provider_edit(&loaded, name, base_url, key) {
        Ok(e) => e,
        Err(out) => return provider_outcome(out),
    };
    match write_key(codex_home, app, &mut record, &loaded, &new_text, &id, "added", now) {
        Ok(backup) => {
            let _ = files::save(app, &record);
            ProviderOutcome::Added { id, backup }
        }
        Err(error) => ProviderOutcome::Failed { error },
    }
}

/// Removes the provider table `id`; never the one Codex uses.
pub fn remove_provider(
    codex_home: &Path,
    app: &AppDir,
    id: &str,
    base_hash: &str,
    now: OffsetDateTime,
) -> ProviderOutcome {
    let mut record = files::load(app);
    let loaded = match load(codex_home, &record) {
        Ok(l) => l,
        Err(out) => return provider_outcome(out),
    };
    if loaded.hash != base_hash {
        return ProviderOutcome::Failed { error: WriteError::Changed };
    }
    let Some(table) = loaded.parsed.providers.iter().find(|c| c.id == id) else {
        return ProviderOutcome::Refused { refusal: KeyRefusal::Unknown };
    };
    if loaded.parsed.selected.as_deref() == Some(id) {
        return ProviderOutcome::Refused { refusal: KeyRefusal::InUse };
    }
    if !table.plain {
        return ProviderOutcome::Refused { refusal: KeyRefusal::Shape };
    }
    let new_text = match config::apply_remove(loaded.text(), id) {
        Ok(t) => t,
        Err(config::PatchError::Parse(detail)) => return ProviderOutcome::Invalid { detail },
        Err(config::PatchError::Check(what)) => {
            return ProviderOutcome::Invalid { detail: format!("check: {what}") }
        }
    };
    match write_key(codex_home, app, &mut record, &loaded, &new_text, id, "removed", now) {
        Ok(backup) => {
            if record.last_custom.as_deref() == Some(id) {
                record.last_custom = None;
            }
            if record.key_undo.as_ref().is_some_and(|u| u.provider == id) {
                record.key_undo = None;
            }
            let _ = files::save(app, &record);
            ProviderOutcome::Removed { id: id.to_string(), backup }
        }
        Err(error) => ProviderOutcome::Failed { error },
    }
}

/// Switches Codex to the provider table `id`, as [`switch`] does to the
/// custom provider.
pub fn switch_provider(
    codex_home: &Path,
    app: &AppDir,
    id: &str,
    base_hash: Option<&str>,
    plugins: bool,
    now: OffsetDateTime,
) -> Outcome {
    switch_to(codex_home, app, Target::Custom, Some(id), base_hash, plugins, now)
}
