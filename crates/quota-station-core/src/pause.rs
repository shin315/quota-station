// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation's pause flag (F07), `claude/pause` in the app's folder, beside
//! the rotation file and the `prime` flag: on macOS `~/Library/Application
//! Support/com.shin315.quota-station/claude/pause`, on Windows
//! `AppData\Roaming\com.shin315.quota-station\claude\pause`. It was under the
//! old switcher's folder ([`OLD_FILE`]); [`file`] carries a flag left there
//! over to the app's folder once.
//!
//! Plain `key = value` lines, like the rotation file: `until = 2026-10-16T18:30`
//! in local time and `reason = …`. No `until` pauses until the file goes away.
//! The rotation's guard compares `until` with its own clock as text and
//! deletes a flag whose time has passed; [`Flag::holds`] applies the same rule,
//! so the app and the rotation agree on whether a pause is on.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// In the app's folder for the Claude accounts ([`crate::claude::DIR`]).
pub const FILE: &str = "pause";
/// Where the flag was before, under the home folder (the old switcher's
/// own path).
pub const OLD_FILE: &str = ".config/cswap-prefer-team/pause";

/// The flag's place in the app's folder, found from the home folder as the
/// command line finds it.
pub fn path(home: &Path) -> PathBuf {
    crate::claude::Paths::for_home(home).dir.join(FILE)
}

/// Where the flag was before it moved to the app's folder.
pub fn old_path(home: &Path) -> PathBuf {
    home.join(OLD_FILE)
}

/// The flag every reader and writer uses: [`path`], once a flag left at
/// [`old_path`] has been carried over (as it is, then the old file removed,
/// so a pause set before the move still holds and is never applied twice).
/// While a carry-over cannot be made (the app's folder cannot be written, or
/// the old file cannot be removed) this is the old file, so the pause still
/// holds; the next call tries again.
pub fn file(home: &Path) -> PathBuf {
    let new = path(home);
    let old = old_path(home);
    if carry_over(&old, &new).is_err() && !exists(&new) && exists(&old) {
        old
    } else {
        new
    }
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Moves the flag at `old` to `new`. A flag already at `new` is the newer
/// one: it stays and the old one goes. The old file is removed before the
/// new one is put in place, never replacing one written meanwhile, so a
/// carry-over that two processes make at once lands one flag, and a flag
/// removed in between is not brought back.
fn carry_over(old: &Path, new: &Path) -> io::Result<()> {
    let text = match std::fs::read(old) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if exists(new) {
        return remove(old);
    }
    let dir = new.parent().ok_or_else(|| io::Error::other("no folder"))?;
    if !dir.is_dir() {
        crate::claude::private_dir(dir)?;
    }
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = new.with_file_name(format!("pause.carry-{}-{n}", std::process::id()));
    std::fs::write(&tmp, &text)?;
    let placed = match std::fs::remove_file(old) {
        // Another process carried it over first.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(()) => match std::fs::hard_link(&tmp, new) {
            Err(e) if e.kind() != io::ErrorKind::AlreadyExists => {
                // Put the flag back where it was, for the next try.
                let _ = std::fs::write(old, &text);
                Err(e)
            }
            _ => Ok(()),
        },
    };
    let _ = std::fs::remove_file(&tmp);
    placed
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

/// Writes through a temporary file and a rename, so the rotation never reads
/// half a flag. A folder made for it is the owner's only (700).
pub fn write(file: &Path, flag: &Flag) -> io::Result<()> {
    if let Some(dir) = file.parent().filter(|dir| !dir.is_dir()) {
        crate::claude::private_dir(dir)?;
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
