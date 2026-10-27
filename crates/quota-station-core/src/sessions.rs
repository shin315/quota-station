// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! How many Claude Code sessions run on this Mac (F04), counted as
//! claude-swap counts them: one record per session in `~/.claude/sessions`,
//! live while its pid is alive.
//!
//! Only `pid` and `kind` are read from a record; serde drops every other
//! field. Sessions started with `quota-station claude run` (or `cswap run`)
//! keep their own profile and record elsewhere, and a switch does not touch
//! them.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::os::ProcessList;

pub const DIR: &str = ".claude/sessions";

pub fn dir(home: &Path) -> PathBuf {
    home.join(DIR)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sessions {
    /// Records whose process is running.
    pub live: u32,
    /// Records that could not be read: each may be a live session.
    pub unreadable: u32,
}

#[derive(Deserialize)]
struct Record {
    pid: i64,
    #[serde(default)]
    kind: Option<String>,
}

/// Counts the live sessions in `dir`. A missing folder is no session.
pub fn count(dir: &Path) -> io::Result<Sessions> {
    let processes = crate::os::processes();
    count_with(dir, |pid| processes.alive(pid))
}

/// [`count`] with the liveness check passed in, for tests.
pub fn count_with(dir: &Path, alive: impl Fn(i32) -> bool) -> io::Result<Sessions> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Sessions::default()),
        Err(e) => return Err(e),
    };
    let mut out = Sessions::default();
    for entry in entries {
        // An entry that cannot be listed may be a live session's record.
        let Ok(entry) = entry else {
            out.unreadable += 1;
            continue;
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let record = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Record>(&bytes).ok());
        let Some(record) = record else {
            out.unreadable += 1;
            continue;
        };
        // A `daemon` record is the host of background sessions, and each of
        // those writes its own record: counting the host would count twice.
        if record.kind.as_deref() == Some("daemon") {
            continue;
        }
        match i32::try_from(record.pid) {
            Ok(pid) if alive(pid) => out.live += 1,
            Ok(_) => {}
            Err(_) => out.unreadable += 1,
        }
    }
    Ok(out)
}
