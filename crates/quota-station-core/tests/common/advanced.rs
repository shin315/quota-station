// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's test helpers: a fake scheduler for the rotation's
//! job, and a Mac whose store keeps the advanced part's keys on the
//! accounts, with that scheduler.

use std::collections::BTreeSet;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use quota_station_core::advanced::held::HeldHooks;
use quota_station_core::claude::agent::{self, AgentError};
use quota_station_core::ext::Hooks;
use quota_station_core::os::Scheduler;

/// A scheduler that keeps which jobs are loaded, by launchd label; one
/// that cannot tell answers `None` for every label, as `launchctl` does
/// when it fails.
#[derive(Debug, Default)]
pub struct FakeScheduler {
    loaded: Mutex<BTreeSet<String>>,
    pub cannot_tell: bool,
    /// The `exe` of the last install.
    installed: Mutex<Option<PathBuf>>,
}

impl FakeScheduler {
    pub fn new() -> Arc<FakeScheduler> {
        Arc::new(FakeScheduler::default())
    }

    /// One whose system cannot tell what is loaded.
    pub fn cannot_tell() -> Arc<FakeScheduler> {
        Arc::new(FakeScheduler { cannot_tell: true, ..FakeScheduler::default() })
    }

    pub fn load(&self, label: &str) {
        self.loaded.lock().unwrap().insert(label.into());
    }

    pub fn is_loaded(&self, label: &str) -> bool {
        self.loaded.lock().unwrap().contains(label)
    }

    /// The program the last install runs.
    pub fn installed(&self) -> Option<PathBuf> {
        self.installed.lock().unwrap().clone()
    }

    fn label_loaded(&self, label: &str) -> Option<bool> {
        (!self.cannot_tell).then(|| self.is_loaded(label))
    }
}

impl Scheduler for FakeScheduler {
    fn loaded(&self) -> Option<bool> {
        self.label_loaded(agent::LABEL)
    }

    fn legacy_loaded(&self) -> Option<&'static str> {
        agent::LEGACY.into_iter().find(|label| self.label_loaded(label) != Some(false))
    }

    fn install(&self, _home: &Path, exe: &Path) -> Result<(), AgentError> {
        if let Some(label) = self.legacy_loaded() {
            return Err(AgentError::Legacy(label.to_string()));
        }
        self.load(agent::LABEL);
        *self.installed.lock().unwrap() = Some(exe.to_path_buf());
        Ok(())
    }

    fn remove(&self, _home: &Path) -> Result<(), AgentError> {
        self.loaded.lock().unwrap().remove(agent::LABEL);
        *self.installed.lock().unwrap() = None;
        Ok(())
    }
}

/// A Mac in a temporary folder (the shared helpers' [`super::accounts::Mac`]),
/// whose store keeps the advanced part's keys on the accounts
/// ([`HeldHooks`]), with a fake scheduler.
pub struct Mac {
    mac: super::accounts::Mac,
    pub scheduler: Arc<FakeScheduler>,
}

impl Deref for Mac {
    type Target = super::accounts::Mac;

    fn deref(&self) -> &Self::Target {
        &self.mac
    }
}

impl Mac {
    pub fn new(tag: &str) -> Mac {
        Mac::with_hooks(tag, &HeldHooks)
    }

    /// The same, whose store runs with other hooks than the advanced
    /// part's.
    pub fn with_hooks(tag: &str, hooks: &dyn Hooks) -> Mac {
        let mac = super::accounts::Mac::with_hooks(tag, hooks);
        Mac { mac, scheduler: FakeScheduler::new() }
    }

    pub fn load_label(&self, label: &str) {
        self.scheduler.load(label);
    }

    pub fn loaded(&self, label: &str) -> bool {
        self.scheduler.is_loaded(label)
    }
}
