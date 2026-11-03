// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's own state in the running app: what it last saw of
//! the rotation and the switcher, the other Macs' copies of the rotation
//! file, the rules editor's save and unsaved changes, the copy to another
//! Mac and the update from source. [`super::Advanced::start`]
//! makes it and keeps it through the host (`ext::Host::keep_state`), so the
//! commands take it as Tauri state (`State<'_, Arc<AdvancedState>>`).
//!
//! The app's own lock (`Inner`) may be held while this lock is taken (the
//! model, the menu bar icon and the reader read it under theirs), never the
//! other way round: code holding [`AdvancedState::lock`] never takes the app's.

#[cfg(unix)]
use std::collections::BTreeSet;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::Condvar;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use quota_station_core::app_dir::AppDir;
use quota_station_core::local_time;
use quota_station_core::pause;
#[cfg(unix)]
use quota_station_core::peers::SyncLog;
use quota_station_core::switcher::{Decision, DryRun, LogLine};
use time::OffsetDateTime;

use crate::ext::Host;

/// The advanced part's state, with its lock and the private threads' wakes.
pub struct AdvancedState {
    fields: Mutex<Fields>,
    /// Wakes the rotation sync thread: a save, or the list of Macs changed.
    #[cfg(unix)]
    pub sync_wake: Condvar,
    /// Held while copying the rotation file to the other Macs, so two copies
    /// to the same Mac never run at once.
    #[cfg(unix)]
    pub push_lock: Mutex<()>,
}

/// What [`AdvancedState::lock`] guards.
#[derive(Debug)]
pub struct Fields {
    pub switcher: SwitcherState,
    /// The reader looks at the switcher at once (`AdvancedState::request_check`).
    pub switcher_now: bool,
    /// Where each other Mac stands with this Mac's rotation file.
    #[cfg(unix)]
    pub sync: SyncLog,
    #[cfg(unix)]
    pub sync_file: PathBuf,
    #[cfg(unix)]
    pub sync_now: bool,
    /// Aliases of the Macs a copy is going to now.
    #[cfg(unix)]
    pub pushing: BTreeSet<String>,
    /// A rotation save is running.
    pub rotation_saving: bool,
    /// The rules editor holds changes not saved yet: closing the main window asks first.
    pub rules_dirty: bool,
    /// `Thoát Quota Station` waits for the main window to ask about unsaved
    /// rules. Kept here, not only sent as an event, because a window opened
    /// for it may not listen yet; the page takes it with `take_quit`.
    pub quit_pending: bool,
    /// The copy to another Mac: what "So sánh" read, and a copy running.
    #[cfg(unix)]
    pub copy: crate::copy::Live,
    /// Where an update from source stands.
    #[cfg(unix)]
    pub update: super::update::Live,
}

impl AdvancedState {
    /// The state at start: the other Macs' records from `rotation-sync.json`
    /// in `app_dir`.
    pub fn new(app_dir: &AppDir) -> AdvancedState {
        #[cfg(unix)]
        let sync_file = app_dir.path().join("rotation-sync.json");
        #[cfg(not(unix))]
        let _ = app_dir;
        AdvancedState::with(Fields {
            #[cfg(unix)]
            sync: SyncLog::load(&sync_file),
            #[cfg(unix)]
            sync_file,
            // The first loop of the reader looks at the switcher in full,
            // and the sync thread at the other Macs.
            switcher_now: true,
            #[cfg(unix)]
            sync_now: true,
            ..Fields::empty()
        })
    }

    fn with(fields: Fields) -> AdvancedState {
        AdvancedState {
            fields: Mutex::new(fields),
            #[cfg(unix)]
            sync_wake: Condvar::new(),
            #[cfg(unix)]
            push_lock: Mutex::new(()),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Fields> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.fields.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Asks the sync thread to look at the other Macs now.
    #[cfg(unix)]
    pub fn request_sync(&self) {
        self.lock().sync_now = true;
        self.sync_wake.notify_all();
    }

    /// Asks the reader thread to look at the switcher again: its dry run,
    /// its log and launchd.
    pub fn request_check(&self, host: &Host) {
        self.lock().switcher_now = true;
        host.wake_reader();
    }
}

/// Nothing read and nothing asked: what the app draws before the private
/// part started (a test's model).
impl Default for AdvancedState {
    fn default() -> AdvancedState {
        AdvancedState::with(Fields::empty())
    }
}

impl Fields {
    fn empty() -> Fields {
        Fields {
            switcher: SwitcherState::default(),
            switcher_now: false,
            #[cfg(unix)]
            sync: SyncLog::default(),
            #[cfg(unix)]
            sync_file: PathBuf::new(),
            #[cfg(unix)]
            sync_now: false,
            #[cfg(unix)]
            pushing: BTreeSet::new(),
            rotation_saving: false,
            rules_dirty: false,
            quit_pending: false,
            #[cfg(unix)]
            copy: crate::copy::Live::default(),
            #[cfg(unix)]
            update: super::update::Live::default(),
        }
    }

    /// Reads the pause flag again after a command changed it; an unreadable
    /// flag counts as none.
    pub fn reload_pause(&mut self, home: &Path) {
        self.switcher.pause = pause::read(&pause::path(home)).ok().flatten();
        self.switcher.pause_generation += 1;
    }
}

/// What the app last saw of the rotation.
#[derive(Debug, Default)]
pub struct SwitcherState {
    /// The pause flag; `None` when there is none.
    pub pause: Option<pause::Flag>,
    /// Bumped when a command writes or removes the flag, so a reader check
    /// that read the flag before cannot put the older one back.
    pub pause_generation: u64,
    pub dry_run: Option<DryRunRecord>,
    pub dry_run_error: Option<&'static str>,
    pub last_log: Option<LogLine>,
    /// The app's rotation LaunchAgent is loaded.
    pub service: Option<bool>,
    /// An older switcher still loaded (`com.cswap.prefer-team` or
    /// `com.cswap.auto`): nothing switches while it is.
    pub legacy: Option<String>,
    pub checked_at: Option<Instant>,
}

#[derive(Debug)]
pub struct DryRunRecord {
    pub run: DryRun,
    pub at: OffsetDateTime,
    /// The pause flag held when the dry run started.
    pub flag_held: bool,
}

impl SwitcherState {
    /// The pause is on now: the flag holds by the guard's rule, and the
    /// switcher has not shown that it ignores the flag.
    pub fn paused(&self, now: OffsetDateTime) -> bool {
        let now = local_time::stamp(now);
        self.pause.as_ref().is_some_and(|f| f.holds(&now)) && self.guard() != Some(false)
    }

    /// Whether the switcher's guard reads the pause flag: known only from a
    /// dry run made while the flag held.
    pub fn guard(&self) -> Option<bool> {
        let record = self.dry_run.as_ref().filter(|r| r.flag_held)?;
        Some(matches!(record.run.decision, Decision::Paused { .. }))
    }
}
