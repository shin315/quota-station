// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The switcher's pause flag, `~/.config/cswap-prefer-team/pause` (F07).
//!
//! Plain `key = value` lines, like the rotation file: `until = 2026-10-16T18:30`
//! in local time and `reason = …`. No `until` pauses until the file goes away.
//! The switcher's guard compares `until` with its own clock as text and
//! deletes a flag whose time has passed; [`Flag::holds`] applies the same rule,
//! so the app and the switcher agree on whether a pause is on.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Under the home folder, next to the switcher's `prime` flag and rotation file.
pub const FILE: &str = ".config/cswap-prefer-team/pause";

pub fn path(home: &Path) -> PathBuf {
    home.join(FILE)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flag {
    /// Local wall-clock time, as written: `2026-10-16T18:30`.
    pub until: Option<String>,
    pub reason: Option<String>,
}

impl Flag {
    /// Reads the flag as the switcher reads its rotation file: `#` lines are
    /// comments, keys and values are trimmed, and the last value of a key wins.
    pub fn parse(text: &str) -> Flag {
        let mut flag = Flag::default();
        for line in text.lines() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else { continue };
            let value = value.trim();
            let value = (!value.is_empty()).then(|| value.to_string());
            match key.trim() {
                "until" => flag.until = value,
                "reason" => flag.reason = value,
                _ => {}
            }
        }
        flag
    }

    pub fn render(&self) -> String {
        let mut text = String::new();
        if let Some(until) = &self.until {
            text.push_str(&format!("until = {until}\n"));
        }
        if let Some(reason) = &self.reason {
            text.push_str(&format!("reason = {}\n", reason.replace(['\n', '\r'], " ")));
        }
        text
    }

    /// The pause is on at local time `now` (`2026-10-16T18:30:05`): no
    /// `until`, an `until` that is not a time (kept paused rather than
    /// guessed), or one still ahead. The guard's rule, compared as text.
    pub fn holds(&self, now: &str) -> bool {
        match self.until.as_deref() {
            Some(until) if well_formed(until) => now < until,
            _ => true,
        }
    }
}

/// Starts with `YYYY-MM-DDTHH:MM`, the guard's test before it compares.
fn well_formed(until: &str) -> bool {
    let b = until.as_bytes();
    b.len() >= 16
        && b[..16].iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 => *c == b':',
            _ => c.is_ascii_digit(),
        })
}

/// The flag, or `None` when there is no file.
pub fn read(file: &Path) -> io::Result<Option<Flag>> {
    match std::fs::read_to_string(file) {
        Ok(text) => Ok(Some(Flag::parse(&text))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Gives each write its own temporary file, so two writes at once (a hold
/// and a pause) never share one.
static WRITES: AtomicU64 = AtomicU64::new(0);

/// Writes through a temporary file and a rename, so the switcher never reads
/// half a flag.
pub fn write(file: &Path, flag: &Flag) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = file.with_file_name(format!("pause.tmp-{}-{n}", std::process::id()));
    std::fs::write(&tmp, flag.render())?;
    std::fs::rename(&tmp, file).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Resumes automatic switching. No flag is not an error.
pub fn remove(file: &Path) -> io::Result<()> {
    match std::fs::remove_file(file) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Why a manual switch paused the switcher; `at` is `HH:MM`.
pub fn hold_reason(slot: u32, at: &str) -> String {
    format!("chuyển tay sang tài khoản {slot} lúc {at}")
}

/// Why the owner paused the switcher from the app; `at` is `HH:MM`.
pub fn pause_reason(at: &str) -> String {
    format!("tạm dừng trong Quota Station lúc {at}")
}
