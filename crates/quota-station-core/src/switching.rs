// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! One manual switch of the Claude account, start to finish (F04): switch
//! Claude Code's login with the app's own [`crate::claude::switch`], pause
//! the rotation when the owner chose to hold the account, wait for things to
//! settle, then look again at which account is active and say what really
//! happened.
//!
//! The second look matters because the rotation may run right after: the
//! app's account lock keeps the two switches apart, so the later one wins;
//! the rotation's log says whether it was the rotation. While an older
//! switcher (`com.cswap.prefer-team`, `com.cswap.auto`) is loaded nothing is
//! switched: it keeps its own copies of the same logins.

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::claude::locks::LockError;
use crate::claude::store::Store;
use crate::claude::switch::{self, SwitchError};
use crate::claude::{agent, state};
use crate::local_time;
use crate::pause::{self, Flag};
use crate::quota::{claude as quota_claude, Response};
use crate::switcher::{self, LogEvent};

/// How long after the switch the active account is looked at again.
pub const SETTLE: Duration = Duration::from_secs(5);

/// What one switch uses.
#[derive(Debug, Clone)]
pub struct Tools {
    pub store: Store,
    pub launchctl: PathBuf,
    pub pause_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hold {
    /// When the rotation may switch again, RFC 3339.
    pub until: String,
    /// The pause flag could not be written: the rotation is not held back.
    pub failed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Outcome {
    /// The second look shows `slot` active.
    Switched { slot: u32, hold: Option<Hold> },
    /// The switch landed, but the second look shows `active`: a later switch
    /// won. `by_switcher_at` is set when the rotation's log has that switch.
    Raced { slot: u32, active: u32, by_switcher_at: Option<String>, hold: Option<Hold> },
    /// The switch landed, and the active account could not be told after.
    Unconfirmed { slot: u32, hold: Option<Hold> },
    /// The switch did not land: [`SwitchError::kind`], or `legacy` while an
    /// older switcher is loaded (its label in `detail`). `active` is the
    /// account in use when the login could not be put back as it was.
    Failed { slot: u32, error: String, detail: String, active: Option<u32> },
}

#[derive(Debug, Clone)]
pub struct Done {
    pub outcome: Outcome,
    /// The Claude report after settling, from the app's own readings, to
    /// show at once.
    pub report: Option<Response>,
}

/// Switches to `slot`. With `hold_until`, the rotation is paused until then
/// after the switch, so it does not switch straight back. `settle` runs
/// between the switch and the second look: [`settle`] waits [`SETTLE`].
pub fn switch_claude(
    tools: &Tools,
    slot: u32,
    hold_until: Option<OffsetDateTime>,
    settle: &dyn Fn(),
) -> Done {
    let failed = |error: &str, detail: String, active: Option<u32>| Done {
        outcome: Outcome::Failed { slot, error: error.to_string(), detail, active },
        report: None,
    };
    if let Some(label) = agent::legacy_loaded(&tools.launchctl) {
        return failed("legacy", label.to_string(), None);
    }
    let started = OffsetDateTime::now_utc();
    if let Err(e) = switch::switch(&tools.store, slot) {
        let active = matches!(e, SwitchError::NotConfirmed { restored: false })
            .then(|| switch::active_slot(&tools.store))
            .flatten();
        return failed(e.kind(), detail(&e), active);
    }
    let _ = state::write(&tools.store, Some(slot), OffsetDateTime::now_utc());

    let hold = hold(tools, slot, hold_until, started);
    settle();
    let outcome = match switch::active_slot(&tools.store) {
        Some(a) if a == slot => Outcome::Switched { slot, hold },
        Some(active) => Outcome::Raced {
            slot,
            active,
            by_switcher_at: switcher_switch(&tools.store.paths.log(), active, started),
            hold,
        },
        None => Outcome::Unconfirmed { slot, hold },
    };
    Done { outcome, report: quota_claude::own_now(&tools.store, OffsetDateTime::now_utc()) }
}

/// Waits [`SETTLE`].
pub fn settle() {
    thread::sleep(SETTLE);
}

/// The error's words for the sheet: none for a held lock, whose message
/// names a path on this Mac.
fn detail(e: &SwitchError) -> String {
    match e {
        SwitchError::Lock(LockError::Busy(_)) => String::new(),
        other => other.to_string(),
    }
}

/// Pauses the rotation until `until` for a manual switch to `slot` that
/// started at `started`. Nothing is written for a hold already over, or when
/// a pause that lasts at least as long is on: a hold never shortens the
/// owner's pause.
///
/// The flag keeps local time to the minute, and the sheet shows the end to
/// the minute (`đến 15:20`), so the hold ends at the minute the owner saw:
/// up to 59 seconds before a duration counted from the second.
fn hold(
    tools: &Tools,
    slot: u32,
    until: Option<OffsetDateTime>,
    started: OffsetDateTime,
) -> Option<Hold> {
    let now = OffsetDateTime::now_utc();
    let until = until.filter(|until| *until > now)?;
    let minute = local_time::minute(until);
    if let Ok(Some(current)) = pause::read(&tools.pause_file) {
        if current.holds(&local_time::stamp(now)) && current.holds(&minute) {
            return None;
        }
    }
    let flag = Flag {
        until: Some(minute.clone()),
        reason: Some(pause::hold_reason(slot, &local_time::clock(started))),
    };
    Some(Hold {
        until: local_time::instant(&minute).unwrap_or(until).format(&Rfc3339).unwrap_or_default(),
        failed: pause::write(&tools.pause_file, &flag).is_err(),
    })
}

/// When the rotation's last log line is its switch to `slot`, written after
/// `since`: the time of that switch.
fn switcher_switch(log: &std::path::Path, slot: u32, since: OffsetDateTime) -> Option<String> {
    let line = switcher::last_log_line(log)?;
    let at = line.at?;
    let when = OffsetDateTime::parse(&at, &Rfc3339).ok()?;
    // The log is written to the second; allow the switch's own second.
    let fresh = when >= since.replace_nanosecond(0).unwrap_or(since);
    (line.event == LogEvent::Switch { slot } && fresh).then_some(at)
}
