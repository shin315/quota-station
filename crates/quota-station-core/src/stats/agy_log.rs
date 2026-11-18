// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Antigravity's conversations: the `agy` command line keeps one SQLite file
//! per conversation, `~/.gemini/antigravity-cli/conversations/<id>.db`, and
//! in its `steps` table one row per step of the agent's work. A step the
//! model answered carries, in its `metadata` (a protocol buffer), the time it
//! was made, the model and the tokens that call used:
//!
//! ```text
//! metadata
//!   1  { 1: seconds }                     the step's time
//!   9  {                                  the model call's usage
//!        1: model, 2: input, 3: output,   output counts the reasoning
//!        4: cache write, 5: cache read,
//!        7: the call's id }
//!   11 model                              the same model, as a number
//! ```
//!
//! The folder a conversation ran in is in `conversation_summaries.db`
//! beside the folder of conversations (`workspace_uris`).
//!
//! Antigravity writes no schema for these files; the numbers above are what
//! its own files hold, found by reading them, and a step that does not fit
//! (a usage with no time, a file with no `steps` table, bytes that are not a
//! protocol buffer) is skipped and counted, never guessed. The model is
//! only a number there (`MODEL_PLACEHOLDER_M528`), so a model is named `M528`.
//!
//! Only the time, the model, the four counts and the call's id are taken
//! from a step. The rest of its `metadata` (a tool call's arguments, say) is
//! walked past by its length and never copied. The files are opened read
//! only, and a conversation being written right now is left for the next
//! read when SQLite says it is busy.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, OpenFlags};
use serde::{Deserialize, Serialize};

use super::{key, Tokens, Usage};

/// Antigravity's folder under the user's home.
pub const DIR: &str = ".gemini/antigravity-cli";
/// The conversations, under [`DIR`].
pub const CONVERSATIONS: &str = "conversations";
/// The summaries (title, folder, step count of each conversation), under [`DIR`].
pub const SUMMARIES: &str = "conversation_summaries.db";

/// A read starts this many steps before where the last one stopped: a step
/// the agent finished writing after the last read is then still found, and
/// the call's key keeps one that was read already from counting twice.
const BACK: i64 = 64;
/// How long a conversation being written is waited for.
const BUSY: Duration = Duration::from_millis(250);

/// What a conversation has told so far, kept between reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    /// The first step not read yet.
    #[serde(default)]
    pub next: i64,
    /// The file's size (the write-ahead log's with it) at the last read.
    #[serde(default)]
    pub len: u64,
    /// Its newest change then, in nanoseconds since the epoch.
    #[serde(default)]
    pub modified: i64,
}

/// Why a conversation gave nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fail {
    /// Locked, gone or not readable right now: read again next time.
    Busy,
    /// Not the format the app knows: skipped and counted.
    Unreadable,
}

/// What one read of a conversation found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub usages: Vec<Usage>,
    /// Steps that held a usage the app could not read, from the first step
    /// not read before.
    pub unknown: u32,
    /// Where the next read starts.
    pub next: i64,
}

/// The steps of the conversation in `db` from `from` on (less [`BACK`]).
/// `conversation` is its id, which with the call's id makes the key that
/// counts a call once; `project` the folder it ran in.
pub fn scan(db: &Path, conversation: &str, project: Option<&str>, from: i64) -> Result<Scan, Fail> {
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(fail)?;
    conn.busy_timeout(BUSY).map_err(fail)?;
    let mut statement = conn
        .prepare("SELECT idx, metadata FROM steps WHERE idx >= ?1 ORDER BY idx")
        .map_err(fail)?;
    let mut rows = statement.query([(from - BACK).max(0)]).map_err(fail)?;
    let mut out = Scan { next: from, ..Scan::default() };
    while let Some(row) = rows.next().map_err(fail)? {
        let idx: i64 = row.get(0).map_err(|_| Fail::Unreadable)?;
        out.next = out.next.max(idx + 1);
        let rusqlite::types::ValueRef::Blob(bytes) = row.get_ref(1).map_err(fail)? else {
            continue;
        };
        match step(bytes) {
            Step::Usage(u) => {
                let call = u.call.map_or_else(|| format!("#{idx}"), |c| c.to_string());
                out.usages.push(Usage {
                    at: u.at,
                    model: u.model,
                    project: project.map(str::to_string),
                    tokens: u.tokens,
                    key: Some(key(&["agy", conversation, &call])),
                });
            }
            Step::Quiet => {}
            Step::Unknown if idx >= from => out.unknown += 1,
            Step::Unknown => {}
        }
    }
    Ok(out)
}

fn fail(e: rusqlite::Error) -> Fail {
    match e.sqlite_error_code() {
        // The file is not a database, or SQLite names no table or column it
        // asked for (a plain `SQLITE_ERROR`): a shape the app does not know.
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt | ErrorCode::Unknown) => {
            Fail::Unreadable
        }
        // Busy, locked, gone or cut off: read again next time.
        _ => Fail::Busy,
    }
}

/// The folder of each conversation, by the conversation's id, from the
/// summaries in `db`; empty when they cannot be read.
pub fn folders(db: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(conn) = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return out;
    };
    let _ = conn.busy_timeout(BUSY);
    let Ok(mut statement) =
        conn.prepare("SELECT conversation_id, workspace_uris FROM conversation_summaries")
    else {
        return out;
    };
    let Ok(rows) =
        statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    else {
        return out;
    };
    for (id, uris) in rows.flatten() {
        let first = serde_json::from_str::<Vec<String>>(&uris)
            .ok()
            .and_then(|l| l.into_iter().next())
            .and_then(|u| uri_path(&u));
        if let Some(path) = first {
            out.insert(id, path);
        }
    }
    out
}

/// `file:///Users/me/My%20App` → `/Users/me/My App`; on Windows
/// `file:///C:/Users/me` → `C:\Users\me`. Only a `file:` address names a
/// folder.
fn uri_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let mut bytes = Vec::with_capacity(rest.len());
    let raw = rest.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'%' if raw.len() >= i + 3 => {
                let hex = std::str::from_utf8(&raw[i + 1..i + 3]).ok()?;
                bytes.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b => {
                bytes.push(b);
                i += 1;
            }
        }
    }
    let path = String::from_utf8(bytes).ok()?;
    if path.is_empty() {
        return None;
    }
    let b = path.as_bytes();
    let drive = b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':';
    Some(if drive { path[1..].replace('/', "\\") } else { path })
}

/// What a step held.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    Usage(Call),
    /// A step without a model call: a command, a file read, a prompt.
    Quiet,
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
struct Call {
    at: i64,
    model: String,
    tokens: Tokens,
    /// The id of the model call, as its bytes say.
    call: Option<String>,
}

/// A step's `metadata`.
fn step(bytes: &[u8]) -> Step {
    let mut at = None;
    let mut usage: Option<&[u8]> = None;
    let mut model = 0u64;
    let ok = fields(bytes, &mut |n, v| match (n, v) {
        (1, Value::Bytes(t)) => {
            let mut secs = None;
            if fields(t, &mut |n, v| {
                if let (1, Value::Varint(s)) = (n, v) {
                    secs = i64::try_from(s).ok();
                }
            }) {
                at = secs;
            }
        }
        (9, Value::Bytes(u)) => usage = Some(u),
        (11, Value::Varint(m)) => model = m,
        _ => {}
    });
    let Some(usage) = usage else {
        return if ok { Step::Quiet } else { Step::Unknown };
    };
    let mut tokens = Tokens::default();
    let mut call = None;
    let mut used_model = 0u64;
    let usage_ok = fields(usage, &mut |n, v| match (n, v) {
        (1, Value::Varint(m)) => used_model = m,
        (2, Value::Varint(x)) => tokens.input = x,
        (3, Value::Varint(x)) => tokens.output = x,
        (4, Value::Varint(x)) => tokens.cache_write = x,
        (5, Value::Varint(x)) => tokens.cache_read = x,
        (7, Value::Bytes(id)) => call = std::str::from_utf8(id).ok().map(str::to_string),
        _ => {}
    });
    let Some(at) = at.filter(|_| usage_ok) else { return Step::Unknown };
    if tokens.is_empty() {
        return Step::Quiet;
    }
    Step::Usage(Call {
        at,
        model: model_name(if used_model != 0 { used_model } else { model }),
        tokens,
        call,
    })
}

/// `1528` → `M528`: the model as Antigravity's files name it
/// (`MODEL_PLACEHOLDER_M528`); a number outside that range as `model-<n>`;
/// none as nothing.
fn model_name(n: u64) -> String {
    match n {
        0 => String::new(),
        1001..=1999 => format!("M{}", n - 1000),
        _ => format!("model-{n}"),
    }
}

/// A field's value in a protocol buffer.
enum Value<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    /// A fixed-size number: walked past.
    Fixed,
}

/// Walks the fields of one protocol buffer message, calling `each` with
/// each field's number and value; false when the bytes are not one.
fn fields<'a>(mut buf: &'a [u8], each: &mut dyn FnMut(u64, Value<'a>)) -> bool {
    while !buf.is_empty() {
        let Some(tag) = varint(&mut buf) else { return false };
        let value = match tag & 7 {
            0 => match varint(&mut buf) {
                Some(v) => Value::Varint(v),
                None => return false,
            },
            1 => match skip(&mut buf, 8) {
                true => Value::Fixed,
                false => return false,
            },
            2 => {
                let Some(len) = varint(&mut buf).and_then(|l| usize::try_from(l).ok()) else {
                    return false;
                };
                if len > buf.len() {
                    return false;
                }
                let (head, tail) = buf.split_at(len);
                buf = tail;
                Value::Bytes(head)
            }
            5 => match skip(&mut buf, 4) {
                true => Value::Fixed,
                false => return false,
            },
            _ => return false,
        };
        each(tag >> 3, value);
    }
    true
}

fn skip(buf: &mut &[u8], n: usize) -> bool {
    if buf.len() < n {
        return false;
    }
    *buf = &buf[n..];
    true
}

fn varint(buf: &mut &[u8]) -> Option<u64> {
    let mut out = 0u64;
    for (i, b) in buf.iter().enumerate().take(10) {
        out |= u64::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            *buf = &buf[i + 1..];
            return Some(out);
        }
    }
    None
}

/// Protocol buffer bytes for the tests: what Antigravity's steps look like,
/// made up.
#[doc(hidden)]
pub mod wire {
    pub fn varint(mut n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let b = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                out.push(b);
                return out;
            }
            out.push(b | 0x80);
        }
    }

    pub fn number(field: u64, n: u64) -> Vec<u8> {
        [varint(field << 3), varint(n)].concat()
    }

    pub fn bytes(field: u64, b: &[u8]) -> Vec<u8> {
        [varint(field << 3 | 2), varint(b.len() as u64), b.to_vec()].concat()
    }

    /// A step's `metadata`: made at `at`, by model `model`, with a usage of
    /// `tokens` (input, output, cache write, cache read) and call id `call`,
    /// and `text` as a field the app must never keep.
    pub fn metadata(at: i64, model: u64, tokens: [u64; 4], call: &str, text: &str) -> Vec<u8> {
        let usage = [
            number(1, model),
            number(2, tokens[0]),
            number(3, tokens[1]),
            number(4, tokens[2]),
            number(5, tokens[3]),
            number(6, 24),
            bytes(7, call.as_bytes()),
        ]
        .concat();
        [
            bytes(1, &[number(1, at as u64), number(2, 75_742_000)].concat()),
            number(3, 2),
            bytes(4, text.as_bytes()),
            bytes(9, &usage),
            number(11, model),
        ]
        .concat()
    }

    /// A step's `metadata` with no model call.
    pub fn quiet(at: i64, text: &str) -> Vec<u8> {
        [bytes(1, &[number(1, at as u64)].concat()), number(3, 2), bytes(4, text.as_bytes())]
            .concat()
    }
}
