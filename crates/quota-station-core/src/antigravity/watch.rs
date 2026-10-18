//! The six-hour watch after an Antigravity switch (the rule of the
//! `agy-prefer-room` switcher draft). A running Antigravity.app writes its
//! own refreshed token back to the shared login about once an hour, which
//! can undo a switch made while it ran. So after each switch the app keeps
//! "from account A to account B at T" in `antigravity/watch.json`, and for
//! six hours looks at every Antigravity reading: once B has been seen in use,
//! a reading that shows A in use again is an overwrite. The popover then
//! offers to switch back, and the overwrites are counted, so automatic
//! switching (later) can stop after two.
//!
//! Only readings that name the account in use count; one that names neither
//! account, or none, changes nothing. The reader and a switch change the file
//! one at a time, so neither undoes the other's change.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::fs::AppDir;
use super::index;

pub const FILE: &str = "watch.json";
/// How long after a switch an overwrite is looked for.
pub const WINDOW_SECS: i64 = 6 * 3600;

/// Held while the file is read, changed and written back.
static LOCK: Mutex<()> = Mutex::new(());

fn locked<T>(f: impl FnOnce() -> T) -> T {
    let _held = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watch {
    /// The account in use before the switch.
    pub from: String,
    /// The account switched to.
    pub to: String,
    /// Unix seconds of the switch.
    pub at: i64,
    /// A reading has shown `to` in use since the switch or the last overwrite.
    #[serde(default)]
    pub seen: bool,
    /// Overwrites of this switch so far, kept across switching back.
    #[serde(default)]
    pub overwrites: u32,
    /// The last overwrite, while the owner has not answered it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overwritten_at: Option<i64>,
    /// The owner chose "Thôi": no more looking, no banner.
    #[serde(default)]
    pub dismissed: bool,
}

impl Watch {
    /// A new watch for a switch from `from` to `to` at `at`. A switch back
    /// to the same account within the six hours, after an overwrite, keeps
    /// the count of `previous`.
    pub fn start(previous: Option<&Watch>, from: &str, to: &str, at: i64) -> Watch {
        let overwrites = previous
            .filter(|p| p.from == from && p.to == to && at - p.at <= WINDOW_SECS)
            .map_or(0, |p| p.overwrites);
        Watch {
            from: from.to_string(),
            to: to.to_string(),
            at,
            seen: false,
            overwrites,
            overwritten_at: None,
            dismissed: false,
        }
    }

    /// Still looking at readings.
    pub fn open(&self, now: i64) -> bool {
        !self.dismissed && now - self.at <= WINDOW_SECS
    }

    /// Takes one reading, `active` being the account it shows in use.
    /// Returns true when it changed the watch.
    pub fn observe(&mut self, active: Option<&str>, now: i64) -> bool {
        if !self.open(now) {
            return false;
        }
        match active {
            // The new account is back: an overwrite not answered yet is over.
            Some(a) if a == self.to && !self.seen => {
                self.seen = true;
                self.overwritten_at = None;
                true
            }
            Some(a) if a == self.from && self.seen => {
                self.seen = false;
                self.overwrites += 1;
                self.overwritten_at = Some(now);
                true
            }
            _ => false,
        }
    }

    /// The overwrite the owner has not answered yet. It stays after the six
    /// hours end, until the owner answers it, switches again, or a reading
    /// inside the six hours shows the new account back in use.
    pub fn alert(&self) -> Option<i64> {
        self.overwritten_at.filter(|_| !self.dismissed)
    }
}

pub fn path(app: &AppDir) -> std::path::PathBuf {
    index::dir(app).join(FILE)
}

/// The watch; none, or an unreadable file, reads as none.
pub fn load(app: &AppDir) -> Option<Watch> {
    let text = app.read_to_string(&path(app)).ok()?;
    serde_json::from_str(&text).ok()
}

fn save(app: &AppDir, watch: &Watch) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(watch).map_err(std::io::Error::other)?;
    app.write_private(&path(app), &text)
}

/// Starts the watch for a switch from `from` to `to` at `at`, in place of
/// the one before (`Watch::start`).
pub fn begin(app: &AppDir, from: &str, to: &str, at: i64) -> std::io::Result<()> {
    locked(|| save(app, &Watch::start(load(app).as_ref(), from, to, at)))
}

/// Feeds one reading to the watch on disk. Returns true when it changed.
pub fn observe(app: &AppDir, active: Option<&str>, now: i64) -> bool {
    locked(|| {
        let Some(mut watch) = load(app) else { return false };
        watch.observe(active, now) && save(app, &watch).is_ok()
    })
}

/// Forgets the watch: a switch whose previous account is not known starts
/// none, and an earlier one must not answer for it.
pub fn clear(app: &AppDir) -> std::io::Result<()> {
    locked(|| app.remove(&path(app)))
}

/// "Thôi" on the banner: stops looking.
pub fn dismiss(app: &AppDir) -> std::io::Result<()> {
    locked(|| match load(app) {
        Some(mut watch) if !watch.dismissed => {
            watch.dismissed = true;
            save(app, &watch)
        }
        _ => Ok(()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = 1_768_548_000;
    const H: i64 = 3600;

    /// Feeds `readings` (seconds after the switch, the account shown) and
    /// returns the watch.
    fn run(readings: &[(i64, Option<&str>)]) -> Watch {
        let mut w = Watch::start(None, "A", "B", T);
        for (after, active) in readings {
            w.observe(*active, T + after);
        }
        w
    }

    #[test]
    fn the_old_account_back_after_the_new_one_is_an_overwrite() {
        let w = run(&[(5, Some("B")), (60, Some("B")), (2 * H, Some("A"))]);
        assert_eq!(w.overwrites, 1);
        assert_eq!(w.alert(), Some(T + 2 * H));
    }

    #[test]
    fn the_old_account_never_left_is_not_counted() {
        // agy never picked the switch up: the switch feedback says so, the
        // watch does not.
        let w = run(&[(5, Some("A")), (60, Some("A")), (H, Some("A"))]);
        assert_eq!(w.overwrites, 0);
        assert_eq!(w.alert(), None);
    }

    #[test]
    fn readings_that_name_no_account_or_another_change_nothing() {
        let w = run(&[(5, None), (60, Some("B")), (120, None), (180, Some("C")), (240, None)]);
        assert!(w.seen);
        assert_eq!((w.overwrites, w.alert()), (0, None));
    }

    #[test]
    fn after_six_hours_nothing_counts() {
        let w = run(&[(5, Some("B")), (6 * H + 1, Some("A"))]);
        assert_eq!((w.overwrites, w.alert()), (0, None));
        // On the edge it still counts.
        let w = run(&[(5, Some("B")), (6 * H, Some("A"))]);
        assert_eq!(w.overwrites, 1);
    }

    #[test]
    fn each_overwrite_counts_once_and_again_after_the_new_account_returns() {
        let w = run(&[
            (5, Some("B")),
            (H, Some("A")),
            (H + 60, Some("A")),
            (H + 120, Some("A")),
            (2 * H, Some("B")),
            (3 * H, Some("A")),
        ]);
        assert_eq!(w.overwrites, 2);
        assert_eq!(w.alert(), Some(T + 3 * H));
    }

    #[test]
    fn switching_back_keeps_the_count_and_a_new_switch_starts_over() {
        let first = run(&[(5, Some("B")), (H, Some("A"))]);
        let again = Watch::start(Some(&first), "A", "B", T + H + 60);
        assert_eq!(again.overwrites, 1);
        assert_eq!(again.alert(), None, "the new switch answers the old overwrite");
        let other = Watch::start(Some(&first), "B", "A", T + H + 60);
        assert_eq!(other.overwrites, 0);
        let late = Watch::start(Some(&first), "A", "B", T + 7 * H);
        assert_eq!(late.overwrites, 0);
    }

    #[test]
    fn thoi_stops_the_watch_and_hides_the_alert() {
        let mut w = run(&[(5, Some("B")), (H, Some("A"))]);
        w.dismissed = true;
        assert_eq!(w.alert(), None);
        assert!(!w.observe(Some("B"), T + H + 60));
        assert!(!w.observe(Some("A"), T + 2 * H));
        assert_eq!(w.overwrites, 1);
    }

    #[test]
    fn the_new_account_back_ends_the_alert_but_keeps_the_count() {
        let w = run(&[(5, Some("B")), (H, Some("A")), (2 * H, Some("B"))]);
        assert_eq!((w.overwrites, w.alert()), (1, None));
    }

    #[test]
    fn an_unanswered_overwrite_stays_after_the_six_hours() {
        let w = run(&[(5, Some("B")), (5 * H, Some("A")), (7 * H, Some("B"))]);
        assert_eq!(w.alert(), Some(T + 5 * H));
    }
}
