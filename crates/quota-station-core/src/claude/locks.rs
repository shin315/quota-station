// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Onur Cetinkol (claude-swap)
// SPDX-License-Identifier: MIT

// Claude Code's lock folders and their order are claude-swap's
// (`claude_locks.py`, MIT, https://github.com/realiti4/claude-swap), which
// follows Claude Code's own: the `proper-lockfile` protocol, a folder made
// with `mkdir`, judged stale by its age and touched while held.

//! The locks a change of the Claude accounts takes, in this order:
//!
//! 1. [`AppLock`]: the app's own `flock` on `claude/.lock`, so the app, the
//!    rotation's LaunchAgent and the command line never change the accounts
//!    or Claude Code's login at the same time;
//! 2. [`credentials`]: Claude Code's `~/.claude/.oauth_refresh.lock`, then
//!    `~/.claude.lock`, which Claude Code holds while it refreshes or writes
//!    its login;
//! 3. [`config`]: Claude Code's `~/.claude.json.lock`, which it holds while
//!    it writes `~/.claude.json`.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::Paths;

/// How long the app waits for its own lock.
pub const APP_TIMEOUT: Duration = Duration::from_secs(30);
/// How long it waits for each of Claude Code's lock folders
/// ([`Paths::lock_wait`]).
pub const CC_TIMEOUT: Duration = Duration::from_secs(9);
/// Claude Code's login locks count as stale after this, its config lock
/// after [`CONFIG_STALE`].
pub const CREDENTIALS_STALE: Duration = Duration::from_secs(60);
pub const CONFIG_STALE: Duration = Duration::from_secs(10);
/// How often a held lock folder is touched, so it never looks stale.
const TOUCH_EVERY: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub enum LockError {
    /// Another holder kept it past the timeout: which lock.
    Busy(String),
    Io(io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Busy(which) => write!(f, "{which} is held by another process"),
            LockError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LockError {}

impl From<io::Error> for LockError {
    fn from(e: io::Error) -> Self {
        LockError::Io(e)
    }
}

/// The app's own lock, released when dropped.
pub struct AppLock {
    _file: std::fs::File,
}

impl AppLock {
    pub fn take(paths: &Paths) -> Result<AppLock, LockError> {
        AppLock::take_within(paths, APP_TIMEOUT)
    }

    pub fn take_within(paths: &Paths, timeout: Duration) -> Result<AppLock, LockError> {
        super::private_dir(&paths.dir)?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(paths.lock())?;
        let deadline = Instant::now() + timeout;
        loop {
            // `flock` on Unix, `LockFileEx` on Windows; named in full, as
            // newer Rust has a `File::try_lock` of its own.
            match fs4::FileExt::try_lock(&file) {
                Ok(()) => return Ok(AppLock { _file: file }),
                Err(fs4::TryLockError::WouldBlock) => {}
                Err(fs4::TryLockError::Error(e)) => return Err(e.into()),
            }
            if Instant::now() >= deadline {
                return Err(LockError::Busy("Quota Station's account lock".into()));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

/// One of Claude Code's lock folders, held: removed when dropped.
pub struct DirLock {
    dir: PathBuf,
    stop: Arc<AtomicBool>,
    toucher: Option<thread::JoinHandle<()>>,
}

impl DirLock {
    /// Makes `dir`, waiting up to `timeout` while another holds it; a folder
    /// older than `stale` was left behind and is taken over.
    pub fn take(dir: &Path, stale: Duration, timeout: Duration) -> Result<DirLock, LockError> {
        let deadline = Instant::now() + timeout;
        loop {
            match std::fs::create_dir(dir) {
                Ok(()) => break,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    let age = std::fs::metadata(dir)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok());
                    if age.is_some_and(|a| a > stale) {
                        let _ = std::fs::remove_dir(dir);
                        continue;
                    }
                    if Instant::now() >= deadline {
                        return Err(LockError::Busy(dir.display().to_string()));
                    }
                    thread::sleep(Duration::from_millis(250 + jitter()));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    // Claude Code's folder is not there yet: nothing holds it.
                    if let Some(parent) = dir.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        let stop = Arc::new(AtomicBool::new(false));
        let toucher = {
            let (dir, stop) = (dir.to_path_buf(), stop.clone());
            thread::spawn(move || {
                let mut since = Instant::now();
                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(100));
                    if since.elapsed() >= TOUCH_EVERY {
                        touch(&dir);
                        since = Instant::now();
                    }
                }
            })
        };
        Ok(DirLock { dir: dir.to_path_buf(), stop, toucher: Some(toucher) })
    }
}

impl Drop for DirLock {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.toucher.take() {
            let _ = t.join();
        }
        let _ = std::fs::remove_dir(&self.dir);
    }
}

/// Both of the folder's times to now, as `utimes(dir, NULL)` sets them.
fn touch(dir: &Path) {
    let now = filetime::FileTime::now();
    let _ = filetime::set_file_times(dir, now, now);
}

/// 0 to 250 milliseconds, so two waiters do not retry in step.
fn jitter() -> u64 {
    let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    u64::from(nanos.subsec_nanos() % 250_000_000) / 1_000_000
}

/// Claude Code's two login locks, in its order.
pub fn credentials(paths: &Paths) -> Result<(DirLock, DirLock), LockError> {
    let refresh = DirLock::take(
        &paths.claude_home().join(".oauth_refresh.lock"),
        CREDENTIALS_STALE,
        paths.lock_wait,
    )?;
    let login =
        DirLock::take(&paths.home.join(".claude.lock"), CREDENTIALS_STALE, paths.lock_wait)?;
    Ok((refresh, login))
}

/// Claude Code's lock on `~/.claude.json`.
pub fn config(paths: &Paths) -> Result<DirLock, LockError> {
    let mut lock = paths.claude_json().into_os_string();
    lock.push(".lock");
    DirLock::take(Path::new(&lock), CONFIG_STALE, paths.lock_wait)
}
