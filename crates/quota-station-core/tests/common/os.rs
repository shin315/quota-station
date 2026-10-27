// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The operating system as the tests give it, kept in memory: a secret
//! store, a scheduler and a process list. Nothing here runs a program or
//! reaches the Keychain, launchd or the real process table, so the tests
//! that use them run on every system.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use quota_station_core::claude::agent::{self, AgentError};
use quota_station_core::claude::keychain::KeychainError;
use quota_station_core::os::{Process, ProcessList, Scheduler, SecretStore};

/// A secret store that keeps its items in memory and records every call
/// (never a value) as `command -a <account> -s <service>`, in the words of
/// macOS's `security`.
#[derive(Debug, Clone, Default)]
pub struct FakeKeychain(Arc<Mutex<Keys>>);

#[derive(Debug, Default)]
struct Keys {
    items: BTreeMap<(String, String), String>,
    calls: Vec<String>,
    fail_add: bool,
    fail_find: bool,
    fail_delete: bool,
    /// Each write notes whether this folder existed.
    watch: Option<PathBuf>,
    watched: Vec<String>,
    /// Writes to this service report success and keep nothing.
    drop_add: Option<String>,
    /// Items another program writes: read from the file once it is there.
    files: BTreeMap<(String, String), PathBuf>,
}

fn refused() -> KeychainError {
    KeychainError::Failed { code: Some(36), message: "User interaction is not allowed.".into() }
}

impl FakeKeychain {
    pub fn new() -> FakeKeychain {
        FakeKeychain::default()
    }

    fn keys(&self) -> std::sync::MutexGuard<'_, Keys> {
        self.0.lock().unwrap()
    }

    pub fn item(&self, service: &str, account: &str) -> Option<String> {
        let key = (service.to_string(), account.to_string());
        let keys = self.keys();
        if let Some(text) = keys.files.get(&key).and_then(|f| std::fs::read_to_string(f).ok()) {
            return Some(text);
        }
        keys.items.get(&key).cloned()
    }

    /// The item as another program (a fake `claude`) leaves it in `file`:
    /// once the file is there, it is the item's value.
    pub fn written_by_file(&self, service: &str, account: &str, file: &Path) {
        self.keys().files.insert((service.into(), account.into()), file.to_path_buf());
    }

    pub fn put(&self, service: &str, account: &str, value: &str) {
        self.keys().items.insert((service.into(), account.into()), value.into());
    }

    pub fn remove(&self, service: &str, account: &str) {
        self.keys().items.remove(&(service.to_string(), account.to_string()));
    }

    /// Every item, as `(service, account)` and value.
    pub fn items(&self) -> BTreeMap<(String, String), String> {
        self.keys().items.clone()
    }

    /// Every call so far, one per line.
    pub fn calls(&self) -> String {
        self.keys().calls.iter().map(|c| format!("{c}\n")).collect()
    }

    pub fn clear_calls(&self) {
        self.keys().calls.clear();
    }

    pub fn fail_add(&self, fail: bool) {
        self.keys().fail_add = fail;
    }

    pub fn fail_find(&self, fail: bool) {
        self.keys().fail_find = fail;
    }

    pub fn fail_delete(&self, fail: bool) {
        self.keys().fail_delete = fail;
    }

    pub fn watch(&self, dir: &Path) {
        self.keys().watch = Some(dir.to_path_buf());
    }

    /// `held <service>` or `free <service>` for each write while watching.
    pub fn watched(&self) -> Vec<String> {
        self.keys().watched.clone()
    }

    pub fn drop_add(&self, service: &str) {
        self.keys().drop_add = Some(service.into());
    }

    fn call(&self, command: &str, service: &str, account: &str) {
        self.keys().calls.push(format!("{command} -a {account} -s {service}"));
    }
}

impl SecretStore for FakeKeychain {
    fn get_within(
        &self,
        service: &str,
        account: &str,
        _wait: Duration,
    ) -> Result<Option<String>, KeychainError> {
        self.call("find-generic-password", service, account);
        if self.keys().fail_find {
            return Err(refused());
        }
        Ok(self.item(service, account))
    }

    fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError> {
        self.get_within(service, account, Duration::ZERO)
    }

    fn exists(&self, service: &str, account: &str) -> Result<bool, KeychainError> {
        self.call("find-generic-password", service, account);
        if self.keys().fail_find {
            return Err(refused());
        }
        Ok(self.item(service, account).is_some())
    }

    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), KeychainError> {
        self.call("add-generic-password", service, account);
        let mut keys = self.keys();
        if keys.fail_add {
            return Err(refused());
        }
        if let Some(dir) = keys.watch.clone() {
            let note = if dir.is_dir() { "held" } else { "free" };
            keys.watched.push(format!("{note} {service}"));
        }
        if keys.drop_add.as_deref() == Some(service) {
            return Ok(());
        }
        let key = (service.to_string(), account.to_string());
        if let Some(file) = keys.files.get(&key) {
            // The app's write replaces what the other program left.
            let _ = std::fs::remove_file(file);
        }
        keys.items.insert(key, value.into());
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError> {
        self.call("delete-generic-password", service, account);
        let mut keys = self.keys();
        if keys.fail_delete {
            return Err(refused());
        }
        let key = (service.to_string(), account.to_string());
        if let Some(file) = keys.files.get(&key) {
            let _ = std::fs::remove_file(file);
        }
        keys.items.remove(&key);
        Ok(())
    }
}

/// A scheduler that keeps which jobs are loaded, by launchd label; one that
/// cannot tell answers `None` for every label, as `launchctl` does when it
/// fails.
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

/// A process table: `None` where the system could not list processes or
/// ports.
#[derive(Debug, Clone, Default)]
pub struct FakeProcesses {
    pub alive: BTreeSet<i32>,
    pub all: Option<Vec<Process>>,
    pub own: Option<Vec<Process>>,
    /// The ports each pid listens on; a pid not here cannot be told.
    pub ports: BTreeMap<u32, Vec<u16>>,
}

impl FakeProcesses {
    /// Nothing runs.
    pub fn none() -> FakeProcesses {
        FakeProcesses { all: Some(Vec::new()), own: Some(Vec::new()), ..Default::default() }
    }

    /// This user runs `command` as `pid`, listening on `ports`.
    pub fn with_own(mut self, pid: u32, command: &str, ports: &[u16]) -> FakeProcesses {
        let process = Process { pid, tty: None, command: command.into() };
        self.own.get_or_insert_with(Vec::new).push(process);
        self.ports.insert(pid, ports.to_vec());
        self
    }
}

impl ProcessList for FakeProcesses {
    fn alive(&self, pid: i32) -> bool {
        self.alive.contains(&pid)
    }

    fn all(&self) -> Option<Vec<Process>> {
        self.all.clone()
    }

    fn own(&self, _wait: Duration) -> Option<Vec<Process>> {
        self.own.clone()
    }

    fn listening(&self, pid: u32, _wait: Duration) -> Option<Vec<u16>> {
        self.ports.get(&pid).cloned()
    }
}
