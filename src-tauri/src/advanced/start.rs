// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the advanced part runs beside the app once it has started: its own
//! threads (on a Mac the rotation sync to the other Macs, on Windows the
//! rotation timer, which has no launchd) and its look at the switcher at
//! each pass of the app's reader: the pause flag on every pass, and the dry
//! run, log and launchd while a window is open or after the owner acted.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use quota_station_core::claude::rotate;
use quota_station_core::claude::store::Store;
use quota_station_core::claude::Paths;
use quota_station_core::local_time;
use quota_station_core::os::Scheduler;
use quota_station_core::pause;
use quota_station_core::switcher;
use time::OffsetDateTime;

use super::state::{AdvancedState, DryRunRecord, Fields};
use crate::ext::{Host, Kept, ReaderDone, ReaderPass, ReaderWork};

/// Starts the advanced part's threads, once the app's own reader runs.
pub fn threads(host: Host) {
    #[cfg(unix)]
    {
        let own = host.kept_state::<AdvancedState>().unwrap_or_default();
        crate::rules::spawn(host, own);
    }
    // Windows has no launchd: the app runs the rotation itself.
    #[cfg(windows)]
    crate::timer::spawn(host);
}

/// The owner, or the advanced part, asked for a look at the switcher now
/// (`AdvancedState::request_check`).
pub fn asked(kept: &Kept) -> bool {
    kept.get::<AdvancedState>().is_some_and(|own| own.lock().switcher_now)
}

/// After the owner's manual switch: the pause flag read again, since a
/// hold may have written it, and a look at the switcher asked for (the
/// app wakes the reader once the switch is in the history), as
/// `AdvancedState::request_check` asks for one.
pub fn after_switch(kept: &Kept, home: &Path) -> bool {
    let author = kept.get::<AdvancedState>().unwrap_or_default();
    let mut own = author.lock();
    own.reload_pause(home);
    own.switcher_now = true;
    true
}

/// After an action of the Claude tab on the accounts: a look at the
/// switcher asked for, and the reader woken for it (as
/// `AdvancedState::request_check` asks for one).
pub fn after_accounts_change(kept: &Kept) -> bool {
    kept.get::<AdvancedState>().unwrap_or_default().lock().switcher_now = true;
    true
}

/// The look at the switcher for this pass of the reader. Beside a read, in
/// full only while a window is open or after a look was asked for; alone,
/// between reads, always in full.
pub fn pass(pass: &ReaderPass<'_>) -> ReaderWork {
    let author = pass.kept.get::<AdvancedState>().unwrap_or_default();
    let home = pass.home.to_path_buf();
    let claude_paths = Paths::new(pass.home, pass.app_dir.path());
    let mut own = author.lock();
    let pause_generation = own.switcher.pause_generation;
    let full = !pass.read_due || own.switcher_now || pass.window_open;
    own.switcher_now = false;
    drop(own);
    Box::new(move || {
        let check = check_switcher(&home, claude_paths, full, pause_generation);
        let done: ReaderDone = Box::new(move |kept: &Kept| check.store(kept, &author));
        done
    })
}

/// One look at the switcher. Without `full`, only the pause flag.
struct SwitcherCheck {
    pause: Option<pause::Flag>,
    /// The flag's generation when the check started.
    pause_generation: u64,
    full: Option<FullCheck>,
}

struct FullCheck {
    /// The rotation's decision now; `noAccounts` before the app keeps any.
    dry_run: Result<DryRunRecord, &'static str>,
    last_log: Option<switcher::LogLine>,
    service: Option<bool>,
    legacy: Option<String>,
}

fn check_switcher(home: &Path, paths: Paths, full: bool, pause_generation: u64) -> SwitcherCheck {
    let flag_file = pause::file(home);
    let pause = pause::read(&flag_file).ok().flatten();
    let full = full.then(|| {
        let at = OffsetDateTime::now_utc();
        let flag_held = pause.as_ref().is_some_and(|f| f.holds(&local_time::stamp(at)));
        let scheduler = quota_station_core::os::scheduler();
        let dry_run = if paths.in_use() {
            let store = Store::new(paths, quota_station_core::os::secret_store())
                .with_hooks(&super::hooks::AdvancedHooks);
            let decided =
                rotate::dry_run(&store, &rotate::Tools::system(home), &rotate::policy(&store), at);
            Ok(DryRunRecord { run: switcher::decided(&decided.line), at, flag_held })
        } else {
            Err("noAccounts")
        };
        FullCheck {
            dry_run,
            last_log: switcher::last_log_line(&switcher::log_file(home)),
            service: scheduler.loaded(),
            legacy: scheduler.legacy_loaded().map(String::from),
        }
    });
    SwitcherCheck { pause, pause_generation, full }
}

impl SwitcherCheck {
    /// Stores the check in the advanced part's state, under the app's lock
    /// too (the reader hands over what the app keeps, `_kept`, while it
    /// holds it), so a model drawn meanwhile sees it whole.
    fn store(self, _kept: &Kept, author: &Arc<AdvancedState>) {
        self.store_in(&mut author.lock());
    }

    fn store_in(self, own: &mut Fields) {
        let sw = &mut own.switcher;
        // A command changed the flag after this check read it: keep the newer.
        if sw.pause_generation == self.pause_generation {
            sw.pause = self.pause;
        }
        if let Some(full) = self.full {
            match full.dry_run {
                Ok(record) => {
                    sw.dry_run = Some(record);
                    sw.dry_run_error = None;
                }
                Err(kind) => {
                    sw.dry_run = None;
                    sw.dry_run_error = Some(kind);
                }
            }
            sw.last_log = full.last_log;
            sw.service = full.service;
            sw.legacy = full.legacy;
            sw.checked_at = Some(Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use quota_station_core::app_dir::AppDir;

    use super::*;

    /// After the owner's manual switch the pause flag a hold wrote is read
    /// again, and a look at the switcher is asked for, which the reader
    /// sees once woken.
    #[test]
    fn a_manual_switch_reloads_the_pause_and_asks_for_a_look() {
        let home = std::env::temp_dir().join(format!("qs-author-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let flag = pause::Flag { until: None, reason: Some("chuyển tay".into()) };
        pause::write(&pause::path(&home), &flag).unwrap();
        let mut kept = Kept::default();
        let author = Arc::new(AdvancedState::default());
        kept.keep(author.clone());
        let generation = author.lock().switcher.pause_generation;
        assert!(after_switch(&kept, &home));
        assert!(asked(&kept));
        let own = author.lock();
        assert_eq!(own.switcher.pause, Some(flag));
        assert_eq!(own.switcher.pause_generation, generation + 1);
        drop(own);
        // Before the advanced part keeps its state there is nothing to follow
        // the switch, and the reader still has nothing to look at.
        assert!(after_switch(&Kept::default(), &home));
        assert!(!asked(&Kept::default()));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// After an action on the Claude accounts a look at the switcher is
    /// asked for, which the reader sees once woken; before the advanced part
    /// keeps its state the reader is still woken, with nothing to look at.
    #[test]
    fn a_change_of_the_accounts_asks_for_a_look() {
        let mut kept = Kept::default();
        kept.keep(Arc::new(AdvancedState::default()));
        assert!(!asked(&kept));
        assert!(after_accounts_change(&kept));
        assert!(asked(&kept));
        assert!(after_accounts_change(&Kept::default()));
        assert!(!asked(&Kept::default()));
    }

    /// A pause set before the flag moved to the app's folder, at its old
    /// place, is the pause the app shows after the upgrade; the look carries
    /// it over.
    #[test]
    fn a_pause_from_before_the_move_still_shows_after_it() {
        let home = std::env::temp_dir().join(format!("qs-author-moved-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let old = pause::old_path(&home);
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, "reason = tạm dừng\n").unwrap();
        let check = check_switcher(&home, Paths::for_home(&home), false, 0);
        let flag = pause::Flag { until: None, reason: Some("tạm dừng".into()) };
        assert_eq!(check.pause, Some(flag));
        assert!(pause::path(&home).exists() && !old.exists());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_switcher_check_that_read_the_flag_before_a_pause_keeps_the_pause() {
        let author = AdvancedState::default();
        let mut own = author.lock();
        let started = own.switcher.pause_generation;
        // The owner pauses while a check that saw no flag is running.
        let flag = pause::Flag { until: None, reason: Some("tạm dừng".into()) };
        own.switcher.pause = Some(flag.clone());
        own.switcher.pause_generation += 1;
        SwitcherCheck { pause: None, pause_generation: started, full: None }.store_in(&mut own);
        assert_eq!(own.switcher.pause, Some(flag));

        // A check that started after the pause is taken as it is.
        let now = own.switcher.pause_generation;
        SwitcherCheck { pause: None, pause_generation: now, full: None }.store_in(&mut own);
        assert_eq!(own.switcher.pause, None);
    }

    /// A look asked for (`AdvancedState::request_check`) sends the reader at
    /// once, and the pass that answers it takes the ask.
    #[test]
    fn a_look_asked_for_sends_the_reader_and_its_pass_takes_the_ask() {
        let author = Arc::new(AdvancedState::default());
        let mut kept = Kept::default();
        kept.keep(author.clone());
        assert!(!asked(&kept));
        author.lock().switcher_now = true;
        assert!(asked(&kept));

        let home = PathBuf::from("/nonexistent/home");
        let app_dir = AppDir::new(PathBuf::from("/nonexistent/app"));
        let alone = ReaderPass {
            kept: &kept,
            home: &home,
            app_dir: &app_dir,
            read_due: false,
            window_open: false,
        };
        let _work = pass(&alone);
        assert!(!asked(&kept), "the ask is taken by the pass");
    }
}
