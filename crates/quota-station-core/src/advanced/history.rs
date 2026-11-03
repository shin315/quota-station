// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's lines of the switch history, which the app knows
//! only through the hooks ([`crate::ext::Hooks::history_lines`]): the
//! switcher's switches and primes at the end of its log
//! ([`switcher::log_file`]), each switch with the reason the switcher gave
//! for it, read the way its dry-run decision is. Their source is
//! [`SOURCE`] (`switcher`), and a switch, not a prime, changes the account
//! in use (the stats' timeline counts it); either explains a change of the
//! account in use the readings show soon after.
//!
//! The History tab's line of the owner's manual switch also shows the hold
//! it set (`holdUntil`, `null` for none), from what the switch kept in the
//! journal ([`crate::advanced::switch::manual_line`]).
//!
//! [`HistoryHooks`] gives the core this; [`lines`] reads them.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::Serialize;

use crate::app_dir::AppDir;
use crate::ext::{Fields, Hooks};
use crate::history::{App, ExtCause, ExtLine};
use crate::switcher::{self, LogEvent, Why};

/// The lines' source, as the windows read it.
pub const SOURCE: &str = "switcher";
/// How much of the end of the switcher's log is read.
const SWITCHER_TAIL: u64 = 64 * 1024;

/// The advanced part's hooks for the history: [`lines`] of the switcher's
/// log under the home. The app's extension gives
/// the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct HistoryHooks;

impl Hooks for HistoryHooks {
    fn history_lines(&self, home: &Path, app: &AppDir) -> Vec<ExtLine> {
        let _ = app;
        lines(&switcher::log_file(home))
    }

    fn manual_line(&self, kept: &Fields) -> Fields {
        crate::advanced::switch::manual_line(kept)
    }
}

/// Why a line of the switcher's log is in the history, as the windows read
/// it (the cause's `kind`, then its keys).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Reason {
    /// `switch to slot N (…)` in the switcher's log.
    Switcher { why: Why, raw: String },
    /// `primed slot N (exit X)`: a five-hour window started early.
    Primed { raw: String },
}

/// The history's lines of the switcher's `log`, oldest first.
pub fn lines(log: &Path) -> Vec<ExtLine> {
    switcher_lines(log)
        .into_iter()
        .filter_map(|line| {
            let at = line.at.as_deref().and_then(unix)?;
            let (to, reason, switch) = match line.event {
                LogEvent::Switch { slot } => {
                    (slot, Reason::Switcher { why: switch_why(&line.raw), raw: line.raw }, true)
                }
                LogEvent::Primed { slot } => (slot, Reason::Primed { raw: line.raw }, false),
                _ => return None,
            };
            let cause = ExtCause::new(&reason, switch).ok()?;
            Some(ExtLine { at, app: App::Claude, source: SOURCE, from: None, to: Some(to), cause })
        })
        .collect()
}

fn unix(at: &str) -> Option<i64> {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::parse(at, &Rfc3339).ok().map(|t| t.unix_timestamp())
}

/// The switcher's switches and primes in the end of its log, oldest first.
pub fn switcher_lines(log: &Path) -> Vec<switcher::LogLine> {
    let Ok(mut f) = std::fs::File::open(log) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    // One byte before the tail too, to tell whether it starts a line.
    let start = len.saturating_sub(SWITCHER_TAIL + 1);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if f.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    // A tail that starts mid-line drops that line; one that starts a line keeps it.
    let tail = if start == 0 {
        &bytes[..]
    } else {
        match bytes.iter().position(|b| *b == b'\n') {
            Some(i) => &bytes[i + 1..],
            None => &[][..],
        }
    };
    let text = String::from_utf8_lossy(tail);
    text.lines()
        .map(switcher::parse_log_line)
        .filter(|l| {
            l.at.is_some() && matches!(l.event, LogEvent::Switch { .. } | LogEvent::Primed { .. })
        })
        .collect()
}

/// `switch to slot 4 (4.0%, ends 15:00)` → the switcher's reason, read the
/// way its dry-run decision is.
fn switch_why(raw: &str) -> Why {
    let text = raw.get(20..).unwrap_or("");
    let rest = text.strip_prefix("switch to slot ").unwrap_or("");
    match switcher::parse_decision(&format!("switch {rest}")) {
        switcher::Decision::Switch { why, .. } => why,
        _ => Why::Other,
    }
}
