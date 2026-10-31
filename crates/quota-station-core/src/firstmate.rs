// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What Firstmate sees (F10): the one line its quota-pool watch writes
//! after every sweep, `quota-pool.last`:
//!
//! `fork=ok auto=off cs=ok active=3 pool=ok detail=s1=98%,s2=100%,s3=27%,next-reset=-`
//!
//! `pool=dry` means Firstmate holds new Claude work until the pool has room
//! again. The file's path is set in the app's settings; nothing in the code
//! names it, and only a file called `quota-pool.last` is read. Only the keys
//! above are kept, each checked, so nothing else in a file ever shows.

use std::io::Read;
use std::path::Path;

use serde::Serialize;

use crate::time_util::parse_ts;

pub const FILE_NAME: &str = "quota-pool.last";
/// The line is short; a bigger file is not Firstmate's.
const MAX_BYTES: u64 = 16 * 1024;
/// Firstmate sweeps every few minutes: a file older than this says it is
/// not running, and its `pool=` is not acted on.
pub const STALE_AFTER_SECS: i64 = 30 * 60;
/// A file written this far ahead of this Mac's clock still counts (the
/// clocks may differ a little); one further ahead does not.
const AHEAD_SECS: i64 = 60;

/// The parts of the line the app reads.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolLine {
    /// quota-axi still has its `+cswap` fork (`fork=ok`).
    pub fork: Option<bool>,
    /// claude-swap's own automatic switcher is loaded (`auto=on`).
    pub auto_switcher: Option<bool>,
    /// Firstmate could read claude-swap's accounts (`cs=ok`).
    pub claude_swap: Option<bool>,
    pub active: Option<u32>,
    /// `pool=dry`: Firstmate holds new Claude work. `None` for `pool=-`.
    pub dry: Option<bool>,
    pub slots: Vec<SlotLeft>,
    /// The first reset among the accounts under 15 %.
    pub next_reset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotLeft {
    pub slot: u32,
    /// Percent left; `None` when Firstmate wrote `?`.
    pub percent: Option<f64>,
    /// Past its reset, waiting for a new reading.
    pub reset: bool,
}

/// Reads one `quota-pool.last` line. Unknown keys and values that are not
/// what Firstmate writes are left out.
pub fn parse(text: &str) -> PoolLine {
    let mut line = PoolLine::default();
    let first = text.lines().next().unwrap_or("");
    for token in first.split_whitespace() {
        let Some((key, value)) = token.split_once('=') else { continue };
        match key {
            "fork" => line.fork = word(value, "ok", "missing"),
            "auto" => line.auto_switcher = word(value, "on", "off"),
            "cs" => line.claude_swap = word(value, "ok", "unavailable"),
            "active" => line.active = value.parse().ok().filter(|n| *n > 0),
            "pool" => line.dry = word(value, "dry", "ok"),
            "detail" => detail(value, &mut line),
            _ => {}
        }
    }
    line
}

fn word(value: &str, yes: &str, no: &str) -> Option<bool> {
    if value == yes {
        Some(true)
    } else if value == no {
        Some(false)
    } else {
        None
    }
}

/// `s1=98%,s2=reset,s3=?%,next-reset=2026-01-16T10:29:00.000Z`
fn detail(value: &str, line: &mut PoolLine) {
    for part in value.split(',') {
        let Some((key, v)) = part.split_once('=') else { continue };
        if key == "next-reset" {
            line.next_reset = parse_ts(v).map(|_| v.to_string());
            continue;
        }
        let Some(slot) = key.strip_prefix('s').and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        let left = if v == "reset" {
            Some(SlotLeft { slot, percent: None, reset: true })
        } else {
            v.strip_suffix('%').map(|p| SlotLeft {
                slot,
                percent: p.parse::<f64>().ok().filter(|p| (0.0..=100.0).contains(p)),
                reset: false,
            })
        };
        line.slots.extend(left);
    }
}

/// The file as last read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub line: PoolLine,
    /// When Firstmate last wrote it, Unix seconds.
    pub modified_at: i64,
}

impl Reading {
    /// Firstmate wrote it within [`STALE_AFTER_SECS`], and not later than now.
    pub fn fresh(&self, now: i64) -> bool {
        (-AHEAD_SECS..=STALE_AFTER_SECS).contains(&(now - self.modified_at))
    }

    /// Firstmate holds new Claude work now, by a file fresh enough to trust.
    pub fn holding(&self, now: i64) -> Option<bool> {
        self.line.dry.filter(|_| self.fresh(now))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The path does not end in `quota-pool.last`.
    NotPoolFile,
    Missing,
    /// Not a plain file, or larger than Firstmate writes.
    NotPlain,
    Unreadable(String),
}

impl ReadError {
    pub fn kind(&self) -> &'static str {
        match self {
            ReadError::NotPoolFile => "notPoolFile",
            ReadError::Missing => "missing",
            ReadError::NotPlain => "notPlain",
            ReadError::Unreadable(_) => "unreadable",
        }
    }
}

/// Whether `path` names a Firstmate state file at all.
pub fn is_pool_file(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == FILE_NAME)
}

/// Reads the state file at `path`: a plain file, not a symlink (it is
/// opened without following one) and not a pipe or a device.
pub fn read(path: &Path) -> Result<Reading, ReadError> {
    if !is_pool_file(path) {
        return Err(ReadError::NotPoolFile);
    }
    let file = open_plain(path)?;
    let meta = file.metadata().map_err(|e| ReadError::Unreadable(e.to_string()))?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(ReadError::NotPlain);
    }
    let modified_at = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0);
    let mut text = String::new();
    file.take(MAX_BYTES)
        .read_to_string(&mut text)
        .map_err(|e| ReadError::Unreadable(e.to_string()))?;
    Ok(Reading { line: parse(&text), modified_at })
}

/// Opens `path` without following a symlink at its end, and without
/// waiting on a pipe.
#[cfg(unix)]
fn open_plain(path: &Path) -> Result<std::fs::File, ReadError> {
    use std::os::unix::fs::OpenOptionsExt;
    match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(f) => Ok(f),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ReadError::Missing),
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => Err(ReadError::NotPlain),
        Err(e) => Err(ReadError::Unreadable(e.to_string())),
    }
}

/// Opens `path` as itself when it is a link (a symlink or a junction, both
/// reparse points), and refuses it then; a folder is refused before
/// opening, which Windows would refuse as access denied.
#[cfg(windows)]
fn open_plain(path: &Path) -> Result<std::fs::File, ReadError> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    };
    let not_found = |e: &std::io::Error| e.kind() == std::io::ErrorKind::NotFound;
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => return Err(ReadError::NotPlain),
        Ok(_) => {}
        Err(e) if not_found(&e) => return Err(ReadError::Missing),
        Err(e) => return Err(ReadError::Unreadable(e.to_string())),
    }
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
    {
        Ok(f) => f,
        Err(e) if not_found(&e) => return Err(ReadError::Missing),
        Err(e) => return Err(ReadError::Unreadable(e.to_string())),
    };
    let meta = file.metadata().map_err(|e| ReadError::Unreadable(e.to_string()))?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(ReadError::NotPlain);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_firstmate_writes_reads_back_in_parts() {
        let line = parse(
            "fork=ok auto=off cs=ok active=3 pool=dry detail=s1=98%,s2=reset,s3=4%,s4=?%,next-reset=2026-01-16T10:29:00.000Z\n",
        );
        assert_eq!(line.fork, Some(true));
        assert_eq!(line.auto_switcher, Some(false));
        assert_eq!(line.claude_swap, Some(true));
        assert_eq!(line.active, Some(3));
        assert_eq!(line.dry, Some(true));
        assert_eq!(line.next_reset.as_deref(), Some("2026-01-16T10:29:00.000Z"));
        assert_eq!(
            line.slots,
            vec![
                SlotLeft { slot: 1, percent: Some(98.0), reset: false },
                SlotLeft { slot: 2, percent: None, reset: true },
                SlotLeft { slot: 3, percent: Some(4.0), reset: false },
                SlotLeft { slot: 4, percent: None, reset: false },
            ]
        );
    }

    #[test]
    fn an_unavailable_sweep_and_unknown_text_say_nothing() {
        let line = parse("fork=ok auto=off cs=unavailable active=- pool=- detail=-");
        assert_eq!(line.claude_swap, Some(false));
        assert_eq!((line.active, line.dry, line.next_reset.clone()), (None, None, None));
        assert!(line.slots.is_empty());
        let line = parse("token=abc pool=maybe active=x next-reset=soon secret");
        assert_eq!(line, PoolLine::default());
    }

    #[test]
    fn only_a_file_called_quota_pool_last_is_read() {
        let dir = std::env::temp_dir().join(format!("qs-firstmate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let other = dir.join("settings.json");
        std::fs::write(&other, "pool=dry").unwrap();
        assert_eq!(read(&other), Err(ReadError::NotPoolFile));
        let file = dir.join(FILE_NAME);
        assert_eq!(read(&file), Err(ReadError::Missing));
        std::fs::write(
            &file,
            "fork=ok auto=off cs=ok active=2 pool=ok detail=s2=40%,next-reset=-\n",
        )
        .unwrap();
        let reading = read(&file).unwrap();
        assert_eq!(reading.line.dry, Some(false));
        let now = reading.modified_at + 60;
        assert_eq!(reading.holding(now), Some(false));
        assert_eq!(reading.holding(reading.modified_at + STALE_AFTER_SECS + 1), None);
        // Written well ahead of this Mac's clock: not trusted either.
        assert_eq!(reading.holding(reading.modified_at - 2 * AHEAD_SECS), None);
        assert_eq!(reading.holding(reading.modified_at - 5), Some(false));

        // A symlink by that name is not followed, nor is a folder read.
        // (Making a symlink on Windows needs Developer Mode or an admin.)
        #[cfg(unix)]
        {
            let linked = dir.join("linked");
            std::fs::create_dir_all(&linked).unwrap();
            std::os::unix::fs::symlink(&other, linked.join(FILE_NAME)).unwrap();
            assert_eq!(read(&linked.join(FILE_NAME)), Err(ReadError::NotPlain));
        }
        let folder = dir.join("folder").join(FILE_NAME);
        std::fs::create_dir_all(&folder).unwrap();
        assert_eq!(read(&folder), Err(ReadError::NotPlain));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
