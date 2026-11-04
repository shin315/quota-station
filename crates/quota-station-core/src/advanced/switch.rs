// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part around the owner's manual switch of the Claude
//! account, which the app knows only through the hooks
//! ([`crate::ext::Hooks::manual_switch`],
//! [`crate::ext::Hooks::may_refresh_logins`]):
//!
//! - while an older switcher (`com.cswap.prefer-team`, `com.cswap.auto`) is
//!   loaded nothing is switched, by the app or by `quota-station claude
//!   switch`, and no token is refreshed: it keeps its own copies of the same
//!   logins;
//! - when the owner chose to hold the account (`holdUntil`, RFC 3339, among
//!   the arguments of the window's `switch_claude`), the rotation is paused
//!   until then once the switch landed, so it does not switch straight back
//!   ([`Hold`], in the outcome's `hold`), and the hold is kept on the
//!   switch's line in the history journal ([`HOLD_UNTIL`]);
//! - the second look matters because the rotation may run right after: the
//!   app's account lock keeps the two switches apart, so the later one wins;
//!   the rotation's log says whether it was the rotation (the outcome's
//!   `bySwitcherAt`).
//!
//! [`SwitchHooks`] gives the core this; [`Around`] is one switch.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::claude::store::Store;
use crate::ext::{Fields, Hooks};
use crate::local_time;
use crate::os::Scheduler;
use crate::pause::{self, Flag};
use crate::switcher::{self, LogEvent};
use crate::switching::{AroundSwitch, Refusal};

/// The argument of the window's `switch_claude` that asks for a hold, and
/// the key of the switch's line in the history journal that keeps it.
pub const HOLD_UNTIL: &str = "holdUntil";

/// The advanced part's hooks around a manual switch: the rotation's job and
/// the older switchers (`scheduler`), and the pause flag, by default the
/// one under the home of the accounts switched ([`pause::file`]).
#[derive(Debug, Clone)]
pub struct SwitchHooks {
    pub scheduler: Arc<dyn Scheduler>,
    pub pause_file: Option<PathBuf>,
}

impl SwitchHooks {
    /// The hooks of this Mac: its own launchd (or Task Scheduler) and pause
    /// flag.
    pub fn system() -> SwitchHooks {
        SwitchHooks { scheduler: Arc::new(crate::os::scheduler()), pause_file: None }
    }
}

impl Hooks for SwitchHooks {
    fn manual_switch(
        &self,
        asked: &Map<String, Value>,
    ) -> Result<Option<Box<dyn AroundSwitch>>, String> {
        let hold_until = match asked.get(HOLD_UNTIL) {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => {
                Some(OffsetDateTime::parse(text, &Rfc3339).map_err(|_| "badTime")?)
            }
            Some(_) => return Err("badTime".into()),
        };
        Ok(Some(Box::new(Around::new(self.clone(), hold_until))))
    }

    /// No token is refreshed while an older switcher runs.
    fn may_refresh_logins(&self) -> bool {
        self.scheduler.legacy_loaded().is_none()
    }
}

/// The hold a switch set: when the rotation may switch again, RFC 3339, and
/// whether the pause flag could not be written (the rotation is not held
/// back then).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hold {
    pub until: String,
    pub failed: bool,
}

/// One manual switch, from the owner's ask: the hold asked for, when the
/// switch started and the hold it set.
pub struct Around {
    hooks: SwitchHooks,
    hold_until: Option<OffsetDateTime>,
    started: OffsetDateTime,
    hold: Option<Hold>,
}

impl Around {
    pub fn new(hooks: SwitchHooks, hold_until: Option<OffsetDateTime>) -> Around {
        Around { hooks, hold_until, started: OffsetDateTime::now_utc(), hold: None }
    }

    fn pause_file(&self, store: &Store) -> PathBuf {
        self.hooks.pause_file.clone().unwrap_or_else(|| pause::file(&store.paths.home))
    }
}

impl AroundSwitch for Around {
    /// Nothing is switched while an older switcher is loaded (`legacy`, its
    /// label as the detail).
    fn before(&mut self, store: &Store) -> Option<Refusal> {
        let _ = store;
        if let Some(label) = self.hooks.scheduler.legacy_loaded() {
            return Some(Refusal {
                error: "legacy".into(),
                detail: label.to_string(),
                message: format!("{label} is loaded; stop it first (see the README's cutover)"),
            });
        }
        self.started = OffsetDateTime::now_utc();
        None
    }

    fn landed(&mut self, store: &Store, slot: u32) {
        self.hold = hold(&self.pause_file(store), slot, self.hold_until, self.started);
    }

    /// `bySwitcherAt` when another account won, then `hold`.
    fn looked(&mut self, store: &Store, slot: u32, active: Option<u32>) -> Fields {
        let mut ext = Fields::new();
        if let Some(active) = active.filter(|a| *a != slot) {
            let at = switcher_switch(&store.paths.log(), active, self.started);
            ext.insert("bySwitcherAt", serde_json::to_value(at).unwrap_or(Value::Null));
        }
        // Written as `Hold` writes it: `until`, then `failed`.
        if ext.insert_written("hold", &self.hold).is_err() {
            ext.insert("hold", Value::Null);
        }
        ext
    }

    /// The hold the switch wrote; none when writing it failed.
    fn journal(&self) -> Fields {
        let mut ext = Fields::new();
        if let Some(hold) = self.hold.as_ref().filter(|h| !h.failed) {
            ext.insert(HOLD_UNTIL, Value::String(hold.until.clone()));
        }
        ext
    }
}

/// The History tab's `holdUntil` on the line of a manual switch: the hold
/// the journal kept for it, `null` for none.
pub fn manual_line(kept: &Fields) -> Fields {
    let until = match kept.get(HOLD_UNTIL) {
        Some(Value::String(until)) => Value::String(until.clone()),
        _ => Value::Null,
    };
    let mut ext = Fields::new();
    ext.insert(HOLD_UNTIL, until);
    ext
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
    pause_file: &Path,
    slot: u32,
    until: Option<OffsetDateTime>,
    started: OffsetDateTime,
) -> Option<Hold> {
    let now = OffsetDateTime::now_utc();
    let until = until.filter(|until| *until > now)?;
    let minute = local_time::minute(until);
    if let Ok(Some(current)) = pause::read(pause_file) {
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
        failed: pause::write(pause_file, &flag).is_err(),
    })
}

/// When the rotation's last log line is its switch to `slot`, written after
/// `since`: the time of that switch.
fn switcher_switch(log: &Path, slot: u32, since: OffsetDateTime) -> Option<String> {
    let line = switcher::last_log_line(log)?;
    let at = line.at?;
    let when = OffsetDateTime::parse(&at, &Rfc3339).ok()?;
    // The log is written to the second; allow the switch's own second.
    let fresh = when >= since.replace_nanosecond(0).unwrap_or(since);
    (line.event == LogEvent::Switch { slot } && fresh).then_some(at)
}
