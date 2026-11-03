// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Gỡ" on one variable [`super::env_check`] found in a file: the line that
//! sets it in a shell file or a PowerShell profile, or its key in the `env`
//! of Claude Code's settings, taken out after a preview, with "Hoàn tác".
//!
//! - [`plan`] reads the file again and finds the same line or key the check
//!   shows. Its [`Preview`] says what goes, the value masked: only what
//!   comes before the value (`export NAME=`, `$env:NAME = `, `"NAME":`)
//!   is kept, then [`MASK`]. No value is ever put in a preview, an error,
//!   a log or anything the window gets.
//! - A line is taken out only when it is a plain assignment that ends on
//!   that line: no quote, bracket or `$(` left open, no `\`, backtick or
//!   `|` that carries it on (or carries the line before into it), no
//!   here-string, and nothing after the value but blanks, a `;` and a
//!   comment. Anything else is refused ([`Refusal`]) rather than guessed
//!   at; the owner can open the file instead.
//! - [`remove`] cuts exactly those bytes: the line with its line ending
//!   (for a last line with none, the line ending before it), or the key's
//!   member with its comma ([`super::json_text::remove_member`]), checked
//!   to still parse. Every other byte stays: the byte order mark, UTF-16,
//!   `\r\n` or `\n`. The file is written through a temporary file beside it
//!   and a rename, only when it still holds the bytes the preview was made
//!   from, keeping its mode on Unix; a symlinked file is written where it
//!   points.
//! - For "Hoàn tác" the cut bytes, where they were, and the file's hashes
//!   before and after are kept in the app's own folder (`agents/env-undo/`,
//!   on Windows in the local twin a roaming profile never carries), the
//!   way the Codex logins are kept: a folder for the owner alone (mode 700,
//!   or an ACL that grants the current user alone) and a file of mode 600.
//!   Only the cut span is kept, never the whole file, and nothing is ever
//!   written beside the owner's file but the temporary file the rename
//!   consumes. [`undo`] puts the span back only when the file still hashes
//!   to what the removal wrote. There is one record per place, the last
//!   removal's; it goes after its undo, and any record older than
//!   [`KEEP_FOR`] goes at the next removal.

use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use base64::Engine;
use serde::{Deserialize, Serialize};

use super::env_check::{self, Encoding, Place, Syntax, Text};
use super::json_text;
use super::write::sha256;
use crate::app_dir::AppDir;
use crate::os;

/// Where "Hoàn tác" records are kept, in the app's (local) folder.
pub const UNDO_DIR: &str = "agents/env-undo";
/// A record older than this goes at the next removal: a week.
pub const KEEP_FOR: i64 = 7 * 24 * 60 * 60;
/// What a preview shows for the value.
pub const MASK: &str = "••••••";

/// Why a place cannot be taken out by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Refusal {
    /// launchd or the registry: no file.
    NoFile,
    /// The file no longer sets it: check again.
    Gone,
    /// The file cannot be read, is too large, or its UTF-16 does not decode.
    Unreadable,
    /// The value goes on past the line: a quote, bracket or `$(` left open.
    Multiline,
    /// A `\`, a backtick or a `|` carries the line on, or the line before
    /// into it.
    Continued,
    /// A PowerShell here-string (`@'…'@`, `@"…"@`).
    HereString,
    /// The line holds more than the one assignment.
    Several,
    /// The settings file is not JSON, or would not be after the cut.
    Json,
}

/// What "Gỡ" will take out, as the window shows it before it is pressed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub name: String,
    pub place: Place,
    pub line: u32,
    /// [`env_check::Found::only_in`].
    pub only_in: Option<String>,
    /// The line or member, the value masked: `export NAME=••••••`.
    pub shown: String,
    /// Set when the app will not take it out.
    pub refusal: Option<Refusal>,
}

/// A removal planned on the file's bytes as they are now. It holds those
/// bytes, which hold the values: they are zeroed when it goes.
pub struct Plan {
    pub preview: Preview,
    file: PathBuf,
    raw: Vec<u8>,
    text: Text,
    /// The bytes of [`Text::body`] to cut; none when refused.
    cut: Option<Range<usize>>,
}

impl Drop for Plan {
    fn drop(&mut self) {
        self.raw.fill(0);
        self.text.body.fill(0);
    }
}

impl std::fmt::Debug for Plan {
    // Never the bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plan").field("preview", &self.preview).finish_non_exhaustive()
    }
}

impl Plan {
    /// The hash of the file the plan was made on, which [`remove`] wants
    /// to find again.
    pub fn hash(&self) -> String {
        sha256(&self.raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RemoveError {
    Refused {
        refusal: Refusal,
    },
    /// The file changed after the preview: nothing written.
    Changed,
    Io {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UndoError {
    /// No record for that place (already undone, or too old).
    Missing,
    /// The file changed after the removal: nothing written.
    Changed,
    Io {
        detail: String,
    },
}

impl From<io::Error> for RemoveError {
    fn from(e: io::Error) -> Self {
        RemoveError::Io { detail: e.to_string() }
    }
}

impl From<io::Error> for UndoError {
    fn from(e: io::Error) -> Self {
        UndoError::Io { detail: e.to_string() }
    }
}

/// The removal of `id` (`ANTHROPIC_AUTH_TOKEN@psProfile`) from the file under
/// `home`, planned on the file as it is now. An error when there is no
/// line or key to show; a plan with [`Preview::refusal`] when there is one
/// the app will not take out.
pub fn plan(home: &Path, id: &str) -> Result<Plan, Refusal> {
    let (var, place) = env_check::parse_id(id).ok_or(Refusal::Gone)?;
    let (Some(file), Some(syntax)) = (place.file(), place.syntax()) else {
        return Err(Refusal::NoFile);
    };
    let file = home.join(file);
    let raw = env_check::read_file(&file).map_err(|_| Refusal::Unreadable)?.ok_or(Refusal::Gone)?;
    let Some(text) = env_check::decode(&raw) else {
        let mut raw = raw;
        raw.fill(0);
        return Err(Refusal::Unreadable);
    };
    let mut plan = Plan {
        preview: Preview {
            id: id.to_string(),
            name: var.name.to_string(),
            place,
            line: 0,
            only_in: None,
            shown: String::new(),
            refusal: None,
        },
        file,
        raw,
        text,
        cut: None,
    };
    let body = &plan.text.body;
    let found = match syntax {
        Syntax::Json => json_cut(body, var.name),
        _ => line_cut(body, var.name, syntax),
    }?;
    plan.preview.line = found.line;
    plan.preview.only_in = found.only_in;
    plan.preview.shown = found.shown;
    match found.cut {
        Ok(cut) => plan.cut = Some(cut),
        Err(r) => plan.preview.refusal = Some(r),
    }
    Ok(plan)
}

/// What a plan found in the text.
struct Found {
    line: u32,
    only_in: Option<String>,
    shown: String,
    cut: Result<Range<usize>, Refusal>,
}

/// The line the check shows for `name` in a shell file or a profile.
fn line_cut(body: &[u8], name: &str, syntax: Syntax) -> Result<Found, Refusal> {
    let hits = env_check::scan_text(body, syntax).map_err(|_| Refusal::Unreadable)?;
    let hit = hits.get(name).ok_or(Refusal::Gone)?;
    let (start, end) = line_span(body, hit.line).ok_or(Refusal::Gone)?;
    let line = body[start..end].strip_suffix(b"\r").unwrap_or(&body[start..end]);
    let value = env_check::assignment(line, name.as_bytes(), syntax).ok_or(Refusal::Gone)?;
    let lead = line.iter().take_while(|b| **b == b' ' || **b == b'\t').count();
    // Only blanks, `export` or `$env:`, the name, `=` and blanks: ASCII.
    let shown = format!("{}{MASK}", String::from_utf8_lossy(&line[lead..value]));
    let before = start.checked_sub(1).map(|nl| {
        let s = body[..nl].iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        &body[s..nl]
    });
    let cut = plain(line, value, syntax)
        .and_then(|()| match before {
            Some(b) if carries_on(b, syntax) => Err(Refusal::Continued),
            _ => Ok(()),
        })
        .map(|()| {
            if end < body.len() {
                start..end + 1
            } else if start == 0 {
                0..end
            } else if start >= 2 && body[start - 2] == b'\r' {
                start - 2..end
            } else {
                start - 1..end
            }
        });
    Ok(Found { line: hit.line, only_in: hit.only_in.clone(), shown, cut })
}

/// The key the check shows for `name` in a settings file's `env`.
fn json_cut(body: &[u8], name: &str) -> Result<Found, Refusal> {
    let text = std::str::from_utf8(body).map_err(|_| Refusal::Json)?;
    let root = json_text::parse(text).map_err(|_| Refusal::Json)?;
    let env = root.get("env").filter(|m| m.value.is_object()).ok_or(Refusal::Gone)?;
    let index = env_check::env_key(&env.value, name).ok_or(Refusal::Gone)?;
    let member = &env.value.members()[index];
    let line = text[..member.key_span.start].bytes().filter(|b| *b == b'\n').count() as u32 + 1;
    let shown = format!("{}: {MASK}", &text[member.key_span.clone()]);
    let edit = json_text::remove_member(&env.value, index);
    let after = json_text::apply(text, vec![edit.clone()]);
    let parses = serde_json::from_str::<serde_json::Value>(&after).is_ok();
    after.into_bytes().fill(0);
    let cut = if parses { Ok(edit.at) } else { Err(Refusal::Json) };
    Ok(Found { line, only_in: None, shown, cut })
}

/// Line `n` (from 1) of `body`: from its first byte to its `\n`, or to the
/// end.
fn line_span(body: &[u8], n: u32) -> Option<(usize, usize)> {
    let mut start = 0;
    for _ in 1..n {
        start += body[start..].iter().position(|b| *b == b'\n')? + 1;
    }
    let end = body[start..].iter().position(|b| *b == b'\n').map_or(body.len(), |i| start + i);
    Some((start, end))
}

/// Whether `line` (without its line ending) carries on into the next one:
/// a `\` at its end in a shell file; a backtick or a `|` in a profile.
fn carries_on(line: &[u8], syntax: Syntax) -> bool {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    match syntax {
        Syntax::Shell => line.ends_with(b"\\"),
        _ => {
            let end = line.iter().rposition(|b| !matches!(b, b' ' | b'\t'));
            end.is_some_and(|i| matches!(line[i], b'`' | b'|'))
        }
    }
}

/// Whether the value from `value` on ends on `line` with nothing after it
/// but blanks, a `;` and a comment.
fn plain(line: &[u8], value: usize, syntax: Syntax) -> Result<(), Refusal> {
    match syntax {
        Syntax::Shell => plain_shell(line, value),
        _ => plain_powershell(line, value),
    }
}

/// What may follow a value: blanks, then nothing, a comment, or a `;` and
/// then nothing or a comment.
fn tail_ok(rest: &[u8]) -> bool {
    let trim = |s: &[u8]| -> usize { s.iter().take_while(|b| matches!(b, b' ' | b'\t')).count() };
    let rest = &rest[trim(rest)..];
    let rest = match rest.strip_prefix(b";") {
        Some(r) => &r[trim(r)..],
        None => rest,
    };
    rest.is_empty() || rest.starts_with(b"#")
}

fn plain_shell(line: &[u8], value: usize) -> Result<(), Refusal> {
    let escapes = line.iter().rev().take_while(|b| **b == b'\\').count();
    if escapes % 2 == 1 {
        return Err(Refusal::Continued);
    }
    let mut quote: Option<u8> = None;
    let mut depth = 0u32;
    let mut tick = false;
    let mut i = value;
    while let Some(&b) = line.get(i) {
        if let Some(q) = quote {
            if q == b'"' && b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\\' if i + 1 >= line.len() => return Err(Refusal::Continued),
            b'\\' => {
                i += 2;
                continue;
            }
            b'\'' | b'"' => quote = Some(b),
            b'`' => tick = !tick,
            b'$' if line.get(i + 1) == Some(&b'(') => {
                depth += 1;
                i += 1;
            }
            b'(' if depth > 0 => depth += 1,
            b')' if depth > 0 => depth -= 1,
            b' ' | b'\t' | b';' | b'&' | b'|' | b'<' | b'>' if depth == 0 && !tick => break,
            _ => {}
        }
        i += 1;
    }
    if quote.is_some() || depth > 0 || tick {
        return Err(Refusal::Multiline);
    }
    if tail_ok(line.get(i..).unwrap_or_default()) {
        Ok(())
    } else {
        Err(Refusal::Several)
    }
}

fn plain_powershell(line: &[u8], value: usize) -> Result<(), Refusal> {
    let rest = &line[value..];
    if rest.starts_with(b"@'") || rest.starts_with(b"@\"") {
        return Err(Refusal::HereString);
    }
    let blank = |b: u8| b == b' ' || b == b'\t';
    let mut quote: Option<u8> = None;
    let mut depth = 0i32;
    // The last byte of code, outside strings and comments.
    let mut last: Option<u8> = Some(b'=');
    let mut i = value;
    let mut tail = line.len();
    while let Some(&b) = line.get(i) {
        let next = line.get(i + 1).copied();
        if let Some(q) = quote {
            if q == b'"' && b == b'`' {
                i += 2;
                continue;
            }
            if b == q {
                if next == Some(q) {
                    i += 2;
                    continue;
                }
                quote = None;
                last = Some(b);
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'`' if line[i + 1..].iter().all(|c| blank(*c)) => return Err(Refusal::Continued),
            b'`' => {
                last = next;
                i += 2;
                continue;
            }
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth < 0 {
                    // It closes a block opened before the line.
                    return Err(Refusal::Several);
                }
            }
            b'<' if next == Some(b'#') => match line[i + 2..].windows(2).position(|w| w == b"#>") {
                Some(n) => {
                    i += n + 4;
                    continue;
                }
                None => return Err(Refusal::Multiline),
            },
            b'#' if depth == 0 && blank(line[i - 1]) => {
                tail = i;
                break;
            }
            b';' if depth == 0 => {
                tail = i;
                break;
            }
            _ => {}
        }
        if !blank(b) {
            last = Some(b);
        }
        i += 1;
    }
    if quote.is_some() || depth > 0 {
        return Err(Refusal::Multiline);
    }
    if matches!(last, Some(b'|' | b',' | b'+' | b'=' | b'-' | b'*' | b'/' | b'%')) {
        // An operator at the end: the value is on the next line.
        return Err(Refusal::Continued);
    }
    if tail_ok(&line[tail..]) {
        Ok(())
    } else {
        Err(Refusal::Several)
    }
}

// ===== The removal and its undo =====

/// What "Hoàn tác" needs, kept in the app's folder: the cut bytes of the
/// decoded text (base64), where they were, and the file's hashes.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    id: String,
    /// The file, for whoever reads the folder; [`undo`] finds it from the
    /// id.
    file: String,
    at: i64,
    encoding: Encoding,
    offset: usize,
    removed: String,
    /// SHA-256 of the file before the removal, and after it.
    before: String,
    after: String,
}

/// The folder the records go in: the app's folder, or on Windows its local
/// twin, as for the Codex accounts.
fn kept_in(app: &AppDir) -> AppDir {
    if cfg!(windows) {
        app.local()
    } else {
        app.clone()
    }
}

fn record_path(kept: &AppDir, id: &str) -> PathBuf {
    kept.path().join(UNDO_DIR).join(format!("{}.json", crate::sealed::name(id)))
}

/// Whether "Hoàn tác" has a record for `id`.
pub fn can_undo(app: &AppDir, id: &str) -> bool {
    record_path(&kept_in(app), id).is_file()
}

/// Takes out what `plan` found, when the file still holds the bytes it was
/// planned on, and keeps the record for [`undo`]. `now` is a Unix time.
pub fn remove(app: &AppDir, plan: Plan, now: i64) -> Result<(), RemoveError> {
    if let Some(refusal) = plan.preview.refusal {
        return Err(RemoveError::Refused { refusal });
    }
    let cut = plan.cut.clone().ok_or(RemoveError::Refused { refusal: Refusal::Gone })?;
    let body = &plan.text.body;
    let mut new_body = [&body[..cut.start], &body[cut.end..]].concat();
    let new_raw = env_check::encode(plan.text.encoding, &new_body);
    new_body.fill(0);
    let mut new_raw = new_raw.ok_or_else(|| RemoveError::Io { detail: "encoding".into() })?;
    let record = Record {
        id: plan.preview.id.clone(),
        file: plan.file.display().to_string(),
        at: now,
        encoding: plan.text.encoding,
        offset: cut.start,
        removed: base64::engine::general_purpose::STANDARD.encode(&body[cut.clone()]),
        before: sha256(&plan.raw),
        after: sha256(&new_raw),
    };
    let kept = kept_in(app);
    prune(&kept, now);
    let path = record_path(&kept, &plan.preview.id);
    let saved = save_record(&kept, &path, record);
    let written = saved.and_then(|()| replace(&plan.file, &plan.raw, &new_raw));
    new_raw.fill(0);
    if written.is_err() {
        let _ = kept.remove(&path);
    }
    written
}

/// Puts back what the last removal of `id` took out of the file under
/// `home`, when the file is still as that removal left it.
pub fn undo(home: &Path, app: &AppDir, id: &str) -> Result<(), UndoError> {
    let (_, place) = env_check::parse_id(id).ok_or(UndoError::Missing)?;
    let file = home.join(place.file().ok_or(UndoError::Missing)?);
    let kept = kept_in(app);
    let path = record_path(&kept, id);
    let mut json = match kept.read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(UndoError::Missing),
        Err(e) => return Err(e.into()),
    };
    let record = serde_json::from_str::<Record>(&json);
    std::mem::take(&mut json).into_bytes().fill(0);
    let mut record = record.map_err(|_| UndoError::Missing)?;
    let result = (|| {
        let mut raw = env_check::read_file(&file)
            .map_err(|_| UndoError::Io { detail: "read".into() })?
            .ok_or(UndoError::Changed)?;
        let restored = (|| {
            if sha256(&raw) != record.after {
                return Err(UndoError::Changed);
            }
            let mut text = env_check::decode(&raw).ok_or(UndoError::Changed)?;
            if text.encoding != record.encoding || record.offset > text.body.len() {
                text.body.fill(0);
                return Err(UndoError::Changed);
            }
            let mut removed = base64::engine::general_purpose::STANDARD
                .decode(&record.removed)
                .map_err(|_| UndoError::Missing)?;
            let at = record.offset;
            let mut body = [&text.body[..at], &removed, &text.body[at..]].concat();
            removed.fill(0);
            text.body.fill(0);
            let bytes = env_check::encode(record.encoding, &body);
            body.fill(0);
            let mut bytes = bytes.ok_or(UndoError::Changed)?;
            let result = if sha256(&bytes) == record.before {
                replace(&file, &raw, &bytes).map_err(|e| match e {
                    RemoveError::Changed => UndoError::Changed,
                    RemoveError::Io { detail } => UndoError::Io { detail },
                    RemoveError::Refused { .. } => UndoError::Changed,
                })
            } else {
                Err(UndoError::Changed)
            };
            bytes.fill(0);
            result
        })();
        raw.fill(0);
        restored
    })();
    std::mem::take(&mut record.removed).into_bytes().fill(0);
    if result.is_ok() {
        kept.remove(&path)?;
    }
    result
}

fn save_record(kept: &AppDir, path: &Path, mut record: Record) -> Result<(), RemoveError> {
    let dir = path.parent().ok_or_else(|| RemoveError::Io { detail: "no folder".into() })?;
    os::owner_dir(dir)?;
    let json =
        serde_json::to_string(&record).map_err(|e| RemoveError::Io { detail: e.to_string() });
    std::mem::take(&mut record.removed).into_bytes().fill(0);
    let json = json?;
    let saved = kept.write_private(path, &json);
    json.into_bytes().fill(0);
    Ok(saved?)
}

/// Removes the records older than [`KEEP_FOR`].
fn prune(kept: &AppDir, now: i64) {
    #[derive(Deserialize)]
    struct At {
        at: i64,
    }
    let Ok(entries) = std::fs::read_dir(kept.path().join(UNDO_DIR)) else { return };
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let Ok(text) = kept.read_to_string(&path) else { continue };
        let at = serde_json::from_str::<At>(&text).map(|a| a.at);
        text.into_bytes().fill(0);
        if at.is_ok_and(|at| now - at > KEEP_FOR) {
            let _ = kept.remove(&path);
        }
    }
}

static WRITES: AtomicU64 = AtomicU64::new(0);

/// Replaces `file` with `bytes` when it still holds `base`: through a
/// temporary file beside it (mode 600 while written, then the file's own
/// mode on Unix) and a rename; a symlinked file is written where it points.
fn replace(file: &Path, base: &[u8], bytes: &[u8]) -> Result<(), RemoveError> {
    use std::io::Write;
    let target = super::write::real(file)?;
    let dir = target.parent().ok_or_else(|| io::Error::other("no folder"))?;
    let name = target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{name}.qs-{}-{n}", std::process::id()));
    let result = (|| {
        let mut f = os::with_mode(std::fs::OpenOptions::new().write(true).create_new(true), 0o600)
            .open(&tmp)?;
        f.write_all(bytes)?;
        os::sync_all(&f)?;
        drop(f);
        #[cfg(unix)]
        if let Ok(meta) = std::fs::metadata(&target) {
            std::fs::set_permissions(&tmp, meta.permissions())?;
        }
        let mut now = std::fs::read(&target)?;
        let same = now == base;
        now.fill(0);
        if !same {
            return Err(RemoveError::Changed);
        }
        os::replace(&tmp, &target)?;
        let mut written = std::fs::read(&target)?;
        let same = written == bytes;
        written.fill(0);
        if !same {
            return Err(RemoveError::Io { detail: "the file read back differs".into() });
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(line: &str) -> Result<(), Refusal> {
        let v = env_check::assignment(line.as_bytes(), b"X", Syntax::Shell).unwrap();
        plain(line.as_bytes(), v, Syntax::Shell)
    }

    fn ps(line: &str) -> Result<(), Refusal> {
        let v = env_check::assignment(line.as_bytes(), b"X", Syntax::PowerShell).unwrap();
        plain(line.as_bytes(), v, Syntax::PowerShell)
    }

    #[test]
    fn a_plain_shell_assignment_is_one_line() {
        assert_eq!(shell("export X=sk-1"), Ok(()));
        assert_eq!(shell("X=\"a b\" # mine"), Ok(()));
        assert_eq!(shell("  export X='a\"b';"), Ok(()));
        assert_eq!(shell("X=\"$(security find-generic-password -w -s x)\""), Ok(()));
        assert_eq!(shell("export X="), Ok(()));
        assert_eq!(shell("X=\"a"), Err(Refusal::Multiline));
        assert_eq!(shell("X=$(cat <<EOF"), Err(Refusal::Multiline));
        assert_eq!(shell("X=a \\"), Err(Refusal::Continued));
        assert_eq!(shell("export X=a Y=b"), Err(Refusal::Several));
        assert_eq!(shell("X=a; export Y=b"), Err(Refusal::Several));
        assert_eq!(shell("X=a && echo hi"), Err(Refusal::Several));
    }

    #[test]
    fn a_plain_powershell_assignment_is_one_line() {
        assert_eq!(ps("$env:X = 'sk-1'"), Ok(()));
        assert_eq!(ps("    $env:X = \"a `\" b\"  # mine"), Ok(()));
        assert_eq!(ps("$env:X = 'it''s';"), Ok(()));
        assert_eq!(ps("$env:X = (Get-Content C:\\k.txt)"), Ok(()));
        assert_eq!(ps("$env:X = 'a' <# note #>"), Ok(()));
        assert_eq!(ps("$env:X = @'"), Err(Refusal::HereString));
        assert_eq!(ps("$env:X = @\""), Err(Refusal::HereString));
        assert_eq!(ps("$env:X = 'a"), Err(Refusal::Multiline));
        assert_eq!(ps("$env:X = (Get-X"), Err(Refusal::Multiline));
        assert_eq!(ps("$env:X = 'a' + `"), Err(Refusal::Continued));
        assert_eq!(ps("$env:X = 'a' +"), Err(Refusal::Continued));
        assert_eq!(ps("$env:X ="), Err(Refusal::Continued));
        assert_eq!(ps("$env:X = 'a'; $env:Y = 'b'"), Err(Refusal::Several));
        assert_eq!(ps("$env:X = 'a' }"), Err(Refusal::Several));
    }

    #[test]
    fn a_line_carried_on_from_the_one_before_is_refused() {
        assert!(carries_on(b"echo a \\", Syntax::Shell));
        assert!(carries_on(b"Write-Host a `\r", Syntax::PowerShell));
        assert!(carries_on(b"Get-X |  ", Syntax::PowerShell));
        assert!(!carries_on(b"echo a", Syntax::Shell));
        assert!(!carries_on(b"function ccd {", Syntax::PowerShell));
    }
}
