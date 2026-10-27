// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the core asks of the operating system, behind four seams, beside
//! [`crate::login_item::LoginItems`] and [`crate::alerts::Notifier`]:
//!
//! - [`SecretStore`]: generic passwords by service and account (macOS:
//!   the login Keychain through `/usr/bin/security`,
//!   [`crate::claude::keychain::Security`]);
//! - [`Scheduler`]: the rotation run every few minutes while the app is
//!   closed, and the older switchers that must not run beside it (macOS:
//!   launchd through `/bin/launchctl`, [`crate::claude::agent::Launchd`]);
//! - [`ProcessList`]: which processes run, their command lines and the
//!   ports they listen on (macOS: `kill(pid, 0)`, `ps` and `lsof`, [`Ps`]);
//! - [`Opener`]: a URL or a file handed to the program the owner chose for
//!   it (macOS: `/usr/bin/open`, [`SystemOpener`]).
//!
//! The app gets each from [`secret_store`], [`scheduler`], [`processes`]
//! and [`opener`]; the tests give their own, kept in memory, so none of
//! them runs a system tool. On Windows the process list answers whether a
//! pid runs; the secret store, the scheduler and the opener answer
//! [`UNSUPPORTED`] until the Windows lanes build them, and nothing there
//! counts as an older switcher.
//!
//! The file steps that differ between the two systems are here too: modes
//! (700 and 600 on Unix; on Windows the profile's ACL keeps other users
//! out), a replace that waits for another program to let go of the file,
//! links to folders (junctions on Windows), whether a file runs, and a
//! file's identity.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::claude::agent::AgentError;
use crate::claude::keychain::KeychainError;
#[cfg(unix)]
use crate::runner;

/// Generic passwords: a value per service and account.
pub trait SecretStore: Send + Sync + std::fmt::Debug {
    /// The item's value, or `None` when there is no such item, waiting up
    /// to `wait` for the system (macOS may ask the owner first).
    fn get_within(
        &self,
        service: &str,
        account: &str,
        wait: Duration,
    ) -> Result<Option<String>, KeychainError>;
    /// The item's value within the store's own time limit.
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError>;
    /// Whether the item is there, without reading its value.
    fn exists(&self, service: &str, account: &str) -> Result<bool, KeychainError>;
    /// Adds or replaces the item. On a failure the item that was there
    /// stays as it was.
    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), KeychainError>;
    /// Removes the item; one that is not there counts as removed.
    fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError>;
}

/// The job that runs `<exe> claude rotate` every few minutes, and the
/// older switchers that must never run beside it.
pub trait Scheduler: Send + Sync + std::fmt::Debug {
    /// Whether the job is installed and loaded; `None` when that cannot be
    /// told.
    fn loaded(&self) -> Option<bool>;
    /// The older switcher still running, if any, or of which the system
    /// cannot tell: nothing switches or refreshes beside one.
    fn legacy_loaded(&self) -> Option<&'static str>;
    /// Installs and starts the job for `exe`. Refused while an older
    /// switcher runs.
    fn install(&self, home: &Path, exe: &Path) -> Result<(), AgentError>;
    /// Stops the job and removes it.
    fn remove(&self, home: &Path) -> Result<(), AgentError>;
}

/// One running process, as [`ProcessList`] lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    /// Its terminal; `None` for one started without (an app, a daemon).
    pub tty: Option<String>,
    /// The command line, its words joined by spaces.
    pub command: String,
}

/// The processes that run.
pub trait ProcessList: Send + Sync + std::fmt::Debug {
    /// Whether `pid` runs, even when it belongs to another user.
    fn alive(&self, pid: i32) -> bool;
    /// Every process, with its terminal; `None` when they cannot be listed.
    fn all(&self) -> Option<Vec<Process>>;
    /// This user's processes, within `wait`; `None` when they cannot be
    /// listed.
    fn own(&self, wait: Duration) -> Option<Vec<Process>>;
    /// The TCP ports `pid` listens on, within `wait`; `None` when that
    /// cannot be told.
    fn listening(&self, pid: u32, wait: Duration) -> Option<Vec<u16>>;
}

/// Hands a URL (or a file) to the program the owner chose for it.
pub trait Opener {
    fn open(&self, target: &str) -> Result<(), String>;
}

/// What a seam answers on a system it is not built for yet.
pub const UNSUPPORTED: &str = "not available on this system yet";

/// This system's secret store.
#[cfg(unix)]
pub fn secret_store() -> crate::claude::keychain::Security {
    crate::claude::keychain::Security::system()
}

/// This system's secret store.
#[cfg(windows)]
pub fn secret_store() -> NoSecretStore {
    NoSecretStore
}

/// This system's scheduler.
#[cfg(unix)]
pub fn scheduler() -> crate::claude::agent::Launchd {
    crate::claude::agent::Launchd::system()
}

/// This system's scheduler.
#[cfg(windows)]
pub fn scheduler() -> NoScheduler {
    NoScheduler
}

/// This system's process list.
#[cfg(unix)]
pub fn processes() -> Ps {
    Ps::system()
}

/// This system's process list.
#[cfg(windows)]
pub fn processes() -> Tasks {
    Tasks
}

/// This system's opener.
pub fn opener() -> SystemOpener {
    SystemOpener
}

/// macOS's `ps`, `lsof` and `kill(pid, 0)`.
#[cfg(unix)]
#[derive(Debug, Clone)]
pub struct Ps {
    pub ps: PathBuf,
    pub lsof: PathBuf,
}

#[cfg(unix)]
impl Ps {
    pub fn system() -> Ps {
        Ps { ps: PathBuf::from("/bin/ps"), lsof: PathBuf::from("/usr/sbin/lsof") }
    }
}

#[cfg(unix)]
impl ProcessList for Ps {
    fn alive(&self, pid: i32) -> bool {
        if pid <= 1 {
            return false;
        }
        // SAFETY: signal 0 sends nothing; it only checks that the pid exists.
        let r = unsafe { libc::kill(pid, 0) };
        r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    /// `ps -ww -Ao tty=,command=`, which shows `??` for no terminal.
    fn all(&self) -> Option<Vec<Process>> {
        let out = std::process::Command::new(&self.ps)
            .args(["-ww", "-Ao", "tty=,command="])
            .env("LC_ALL", "C")
            .stdin(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        Some(tty_table(&String::from_utf8_lossy(&out.stdout)))
    }

    /// `ps -x -u <euid> -o pid=,command=`.
    fn own(&self, wait: Duration) -> Option<Vec<Process>> {
        // SAFETY: `geteuid` only reads the process's user id.
        let uid = unsafe { libc::geteuid() }.to_string();
        let args: Vec<String> =
            ["-x", "-u", &uid, "-o", "pid=,command="].iter().map(|s| s.to_string()).collect();
        let out = runner::run(&self.ps, &args, wait).ok().filter(|o| o.code == Some(0))?;
        Some(pid_table(&String::from_utf8_lossy(&out.stdout)))
    }

    /// `lsof -nP -a -p PID -iTCP -sTCP:LISTEN`.
    fn listening(&self, pid: u32, wait: Duration) -> Option<Vec<u16>> {
        let args: Vec<String> = ["-nP", "-a", "-p", &pid.to_string(), "-iTCP", "-sTCP:LISTEN"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let out = runner::run(&self.lsof, &args, wait).ok().filter(|o| o.code == Some(0))?;
        Some(listening_ports(&String::from_utf8_lossy(&out.stdout)))
    }
}

/// `ps`'s `tty=,command=` lines; `??` and `-` are no terminal.
pub fn tty_table(table: &str) -> Vec<Process> {
    table
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (tty, command) = line.split_once(char::is_whitespace)?;
            let tty = (tty != "??" && tty != "-").then(|| tty.to_string());
            Some(Process { pid: 0, tty, command: command.trim_start().to_string() })
        })
        .collect()
}

/// `ps`'s `pid=,command=` lines; a line that does not start with a pid
/// (a header) or has no command is left out.
pub fn pid_table(table: &str) -> Vec<Process> {
    table
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, command) = line.split_once(char::is_whitespace)?;
            let pid: u32 = pid.parse().ok()?;
            let command = command.trim();
            (!command.is_empty()).then(|| Process { pid, tty: None, command: command.into() })
        })
        .collect()
}

/// The ports `lsof` shows listening.
pub fn listening_ports(lsof: &str) -> Vec<u16> {
    let mut out = Vec::new();
    for line in lsof.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = words.iter().position(|w| *w == "TCP") else { continue };
        let (Some(addr), Some(state)) = (words.get(at + 1), words.get(at + 2)) else { continue };
        if *state != "(LISTEN)" {
            continue;
        }
        let port = addr.rsplit(':').next().and_then(|p| p.parse::<u16>().ok());
        if let Some(p) = port.filter(|p| *p > 0) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// The default browser (or the app for a file), through `open` on macOS.
pub struct SystemOpener;

impl Opener for SystemOpener {
    #[cfg(windows)]
    fn open(&self, _target: &str) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }

    #[cfg(unix)]
    fn open(&self, target: &str) -> Result<(), String> {
        let out =
            runner::run(Path::new("/usr/bin/open"), &[target.to_string()], Duration::from_secs(15))
                .map_err(|e| e.to_string())?;
        if out.code == Some(0) {
            Ok(())
        } else {
            Err(runner::first_line(&out.stderr))
        }
    }
}

/// Windows has no secret store in this build yet (the Windows logins
/// bring one): every call fails as one that could not reach it, and the
/// readers go on with the files they also read.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSecretStore;

#[cfg(windows)]
impl SecretStore for NoSecretStore {
    fn get_within(&self, _: &str, _: &str, _: Duration) -> Result<Option<String>, KeychainError> {
        Err(KeychainError::Spawn(UNSUPPORTED.into()))
    }

    fn get(&self, _: &str, _: &str) -> Result<Option<String>, KeychainError> {
        Err(KeychainError::Spawn(UNSUPPORTED.into()))
    }

    fn exists(&self, _: &str, _: &str) -> Result<bool, KeychainError> {
        Err(KeychainError::Spawn(UNSUPPORTED.into()))
    }

    fn set(&self, _: &str, _: &str, _: &str) -> Result<(), KeychainError> {
        Err(KeychainError::Spawn(UNSUPPORTED.into()))
    }

    fn delete(&self, _: &str, _: &str) -> Result<(), KeychainError> {
        Err(KeychainError::Spawn(UNSUPPORTED.into()))
    }
}

/// Windows has no rotation job in this build yet (the app's own timer comes
/// with the Windows services): none is loaded, none can be installed, and
/// the older switchers, which are macOS's, never run.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NoScheduler;

#[cfg(windows)]
impl Scheduler for NoScheduler {
    fn loaded(&self) -> Option<bool> {
        Some(false)
    }

    fn legacy_loaded(&self) -> Option<&'static str> {
        None
    }

    fn install(&self, _: &Path, _: &Path) -> Result<(), AgentError> {
        Err(AgentError::Io(UNSUPPORTED.into()))
    }

    fn remove(&self, _: &Path) -> Result<(), AgentError> {
        Ok(())
    }
}

/// Windows's processes: whether a pid runs, through `OpenProcess`. The
/// process table and the ports come with the Windows logins; until then
/// they cannot be listed.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Tasks;

#[cfg(windows)]
impl ProcessList for Tasks {
    fn alive(&self, pid: i32) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let Ok(pid) = u32::try_from(pid) else { return false };
        if pid <= 4 {
            // The idle process and System, never a session.
            return false;
        }
        // SAFETY: the handle is checked, read once and closed.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                // Denied means it runs, as another user's.
                return std::io::Error::last_os_error().raw_os_error() == Some(5);
            }
            let mut code = 0u32;
            let read = GetExitCodeProcess(handle, &mut code);
            CloseHandle(handle);
            read != 0 && code == STILL_ACTIVE as u32
        }
    }

    fn all(&self) -> Option<Vec<Process>> {
        None
    }

    fn own(&self, _wait: Duration) -> Option<Vec<Process>> {
        None
    }

    fn listening(&self, _pid: u32, _wait: Duration) -> Option<Vec<u16>> {
        None
    }
}

// ===== Files =====

/// `options` creating a file of `mode` (Unix; nothing on Windows).
pub(crate) fn with_mode(
    options: &mut std::fs::OpenOptions,
    mode: u32,
) -> &mut std::fs::OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode)
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        options
    }
}

/// Sets `path`'s mode (Unix; nothing on Windows).
pub(crate) fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(())
    }
}

/// Makes the folder `dir` of `mode`, its parents too when `recursive`.
pub(crate) fn create_dir(dir: &Path, mode: u32, recursive: bool) -> io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(recursive);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    builder.create(dir)
}

/// The file's permission bits: `0o644` on Windows, which has none.
pub(crate) fn mode(meta: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0o644
    }
}

/// Whether the file runs: its execute bits on Unix; on Windows, which has
/// none, whether it starts with `#!`, as a script someone made executable
/// on a Mac or Linux does.
pub(crate) fn runs(meta: &std::fs::Metadata, head: &[u8]) -> bool {
    if cfg!(unix) {
        mode(meta) & 0o111 != 0
    } else {
        head.starts_with(b"#!")
    }
}

/// `rename` over `to`. On Windows a file another program holds open
/// (Claude Code, an indexer, an antivirus) cannot be replaced for that
/// moment: tried again for up to two seconds.
pub(crate) fn replace(from: &Path, to: &Path) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match std::fs::rename(from, to) {
            // ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION.
            Err(e) if cfg!(windows) && matches!(e.raw_os_error(), Some(5 | 32 | 33)) => {
                tries += 1;
                if tries > 20 {
                    return Err(e);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            other => return other,
        }
    }
}

/// A link at `link` to the folder `target` (relative to the link's folder,
/// or absolute): a symlink on Unix, a junction on Windows, which needs no
/// Developer Mode and takes an absolute target.
pub(crate) fn link_dir(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        junction::create(beside(target, link), link)
    }
}

/// `target` as a link at `link` reads it: relative to the link's folder.
#[cfg(windows)]
fn beside(target: &Path, link: &Path) -> PathBuf {
    match link.parent() {
        Some(dir) if target.is_relative() => dir.join(target),
        _ => target.to_path_buf(),
    }
}

/// A link at `link` to the file `target`: a symlink on Unix; on Windows a
/// hard link, else a copy.
pub(crate) fn link_file(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        let target = beside(target, link);
        std::fs::hard_link(&target, link).or_else(|_| std::fs::copy(&target, link).map(|_| ()))
    }
}

/// A copy at `to` of the link `from`, to the same target.
pub(crate) fn copy_link(from: &Path, to: &Path) -> io::Result<()> {
    let target = std::fs::read_link(from)?;
    if std::fs::metadata(from).is_ok_and(|m| m.is_dir()) {
        link_dir(&target, to)
    } else {
        link_file(&target, to)
    }
}

/// Removes the link `path` (never what it points to): on Windows a link to
/// a folder goes with `remove_dir`.
pub(crate) fn remove_link(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_DIRECTORY on the link itself.
        if std::fs::symlink_metadata(path)?.file_attributes() & 0x10 != 0 {
            return std::fs::remove_dir(path);
        }
    }
    std::fs::remove_file(path)
}

/// A file's identity while it lives, whatever its name: device and inode
/// on Unix, volume and file index on Windows.
pub(crate) fn file_id(path: &Path, meta: &std::fs::Metadata) -> io::Result<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = path;
        Ok((meta.dev(), meta.ino()))
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };
        let _ = meta;
        let file = std::fs::File::open(path)?;
        // SAFETY: the handle is the open file's; the struct is plain data.
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
        Ok((u64::from(info.dwVolumeSerialNumber), index))
    }
}

/// When the file last changed, in Unix seconds: its status change time on
/// Unix, its last write on Windows.
pub(crate) fn changed(meta: &std::fs::Metadata) -> i64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.ctime()
    }
    #[cfg(not(unix))]
    {
        meta.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_tables_split_into_processes() {
        let tty = tty_table("ttys001 codex --resume\n??       /Applications/Codex.app/x\n");
        assert_eq!(tty[0].tty.as_deref(), Some("ttys001"));
        assert_eq!(tty[0].command, "codex --resume");
        assert_eq!(tty[1].tty, None);
        let pid = pid_table("  PID COMMAND\n 4242 /usr/local/bin/agy --resume\n 7 \n");
        assert_eq!(
            pid,
            [Process { pid: 4242, tty: None, command: "/usr/local/bin/agy --resume".into() }]
        );
    }

    #[test]
    fn lsof_lines_give_the_listening_ports() {
        let out = "COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME\n\
                   language 202 me 10u IPv4 0x1 0t0 TCP 127.0.0.1:53210 (LISTEN)\n\
                   language 202 me 11u IPv6 0x2 0t0 TCP [::1]:53211 (LISTEN)\n\
                   language 202 me 12u IPv4 0x3 0t0 TCP 127.0.0.1:53212->127.0.0.1:1 (ESTABLISHED)\n";
        assert_eq!(listening_ports(out), [53210, 53211]);
    }
}
