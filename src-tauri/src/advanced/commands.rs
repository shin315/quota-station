// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's commands that change its settings and its state
//! through the host ([`Host`]): the Nâng cao switch, the pause (the
//! rotation's pause flag) and the quit guard for unsaved rules. Each keeps the name, the arguments and the answer it had
//! among the app's own commands.

use std::sync::Arc;

use quota_station_core::advanced::settings::Own;
use quota_station_core::local_time;
use quota_station_core::pause;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::advanced::state::AdvancedState;
use crate::ext::{Host, Model};

/// Cài đặt › Nâng cao: shows or hides what was made for the author's own
/// Macs (the rotation, the pause and the switcher, the other Macs, the update
/// from source). Hiding changes no file but the settings: a
/// rotation already running keeps running.
#[tauri::command]
pub fn set_advanced(app: AppHandle, on: bool) -> Result<Model, String> {
    let host = Host::of(&app);
    host.update_settings(|s| Own::change(s, |own| own.advanced = on))?;
    host.publish();
    Ok(host.model())
}

/// Pauses automatic switching until `until` (RFC 3339), or until the owner
/// resumes when it is `None`.
#[tauri::command]
pub fn pause_switching(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    until: Option<String>,
) -> Result<Model, String> {
    let flag = pause_flag(until.as_deref(), OffsetDateTime::now_utc())?;
    write_flag(&Host::of(&app), &own, Some(flag))
}

/// Moves the end of a timed pause `minutes` later, keeping why it started.
#[tauri::command]
pub fn extend_pause(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    minutes: u32,
) -> Result<Model, String> {
    let host = Host::of(&app);
    let file = pause::file(&host.home());
    let flag = pause::read(&file).map_err(|e| e.to_string())?.ok_or("notPaused")?;
    let flag = extended(flag, minutes, OffsetDateTime::now_utc())?;
    write_flag(&host, &own, Some(flag))
}

/// Lets the switcher switch again: removes the pause flag.
#[tauri::command]
pub fn resume_switching(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
) -> Result<Model, String> {
    write_flag(&Host::of(&app), &own, None)
}

/// The flag a pause until `until` (RFC 3339; `None` until the owner resumes)
/// writes at `now`: its end on the minute, and why it started. A time that
/// is not RFC 3339 is `badTime`, one not after `now` is `past`.
fn pause_flag(until: Option<&str>, now: OffsetDateTime) -> Result<pause::Flag, String> {
    let until = match until {
        Some(text) => {
            let t = OffsetDateTime::parse(text, &Rfc3339).map_err(|_| "badTime")?;
            if t <= now {
                return Err("past".into());
            }
            Some(local_time::minute(t))
        }
        None => None,
    };
    Ok(pause::Flag { until, reason: Some(pause::pause_reason(&local_time::clock(now))) })
}

/// `flag` with its end `minutes` later, counted from its end or from `now`
/// when that has passed. A pause with no end is `noEnd`.
fn extended(
    mut flag: pause::Flag,
    minutes: u32,
    now: OffsetDateTime,
) -> Result<pause::Flag, String> {
    let until = flag.until.as_deref().and_then(local_time::instant).ok_or("noEnd")?;
    let from = until.max(now);
    flag.until = Some(local_time::minute(from + Duration::minutes(i64::from(minutes))));
    Ok(flag)
}

/// Writes the pause flag (`None` removes it) and reads it again into the
/// private part's state, under the app's lock, so a model drawn meanwhile
/// never sees half of it; then asks the reader to look at the switcher and
/// sends the windows the model.
fn write_flag(
    host: &Host,
    own: &AdvancedState,
    flag: Option<pause::Flag>,
) -> Result<Model, String> {
    host.locked(|app| {
        let mut fields = own.lock();
        let file = pause::file(app.home());
        match &flag {
            Some(flag) => pause::write(&file, flag),
            None => pause::remove(&file),
        }
        .map_err(|e| e.to_string())?;
        fields.reload_pause(app.home());
        Ok::<(), String>(())
    })?;
    own.request_check(host);
    host.publish();
    Ok(host.model())
}

/// The editor has changes not saved: closing the main window asks first.
#[tauri::command]
pub fn rules_dirty(own: State<'_, Arc<AdvancedState>>, dirty: bool) {
    own.lock().rules_dirty = dirty;
}

/// Takes a pending `Thoát Quota Station` request: true once per request, for
/// the page to ask about unsaved rules (or quit when there are none).
#[tauri::command]
pub fn take_quit(own: State<'_, Arc<AdvancedState>>) -> bool {
    std::mem::take(&mut own.lock().quit_pending)
}

/// Closes the main window after the owner answered the unsaved-changes
/// dialog, or quits the app when the dialog came from `Thoát Quota Station`.
#[tauri::command]
pub fn close_main(app: AppHandle, own: State<'_, Arc<AdvancedState>>, quit: bool) {
    {
        let mut fields = own.lock();
        fields.rules_dirty = false;
        fields.quit_pending = false;
    }
    if quit {
        app.exit(0);
    } else {
        Host::of(&app).close_main_window();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> OffsetDateTime {
        OffsetDateTime::parse(text, &Rfc3339).unwrap()
    }

    /// A pause to a time after now ends on that time's minute and says
    /// when it started; one with no end holds until the owner resumes.
    #[test]
    fn a_pause_writes_its_end_on_the_minute_and_why_it_started() {
        let now = at("2026-10-16T10:00:00Z");
        let flag = pause_flag(Some("2026-10-16T12:30:45Z"), now).unwrap();
        assert_eq!(flag.until, Some(local_time::minute(at("2026-10-16T12:30:45Z"))));
        assert_eq!(flag.reason, Some(pause::pause_reason(&local_time::clock(now))));
        let open = pause_flag(None, now).unwrap();
        assert_eq!(open.until, None);
        assert_eq!(open.reason, flag.reason);
    }

    #[test]
    fn a_pause_to_a_past_or_unreadable_time_is_refused() {
        let now = at("2026-10-16T10:00:00Z");
        assert_eq!(pause_flag(Some("2026-10-16T10:00:00Z"), now).unwrap_err(), "past");
        assert_eq!(pause_flag(Some("2026-10-16T09:59:00Z"), now).unwrap_err(), "past");
        assert_eq!(pause_flag(Some("tomorrow"), now).unwrap_err(), "badTime");
    }

    /// Extending a pause keeps why it started and moves its end later:
    /// from its end while that is ahead, from now once it passed.
    #[test]
    fn extending_a_pause_moves_its_end_and_keeps_why() {
        let end = at("2026-10-16T12:00:00Z");
        let flag = pause::Flag {
            until: Some(local_time::minute(end)),
            reason: Some("why it started".into()),
        };
        let early = extended(flag.clone(), 30, at("2026-10-16T10:00:00Z")).unwrap();
        assert_eq!(early.until, Some(local_time::minute(end + Duration::minutes(30))));
        assert_eq!(early.reason.as_deref(), Some("why it started"));
        let late_now = at("2026-10-16T13:00:00Z");
        let late = extended(flag, 15, late_now).unwrap();
        assert_eq!(late.until, Some(local_time::minute(late_now + Duration::minutes(15))));
    }

    #[test]
    fn a_pause_with_no_end_cannot_be_extended() {
        let flag = pause::Flag { until: None, reason: None };
        assert_eq!(extended(flag, 30, at("2026-10-16T10:00:00Z")).unwrap_err(), "noEnd");
    }
}
