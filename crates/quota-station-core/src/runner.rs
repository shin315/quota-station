// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Running the programs the app calls: `agy`, `claude`, `codex`, `security`,
//! `ps` and the rest. Each run has a time limit, and a program started from
//! an app opened in Finder still finds the tools of the owner's shell. On
//! Windows no run opens a console window, and a command is found under any
//! of the `PATHEXT` extensions (`claude.cmd`, `agy.exe`).

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// Nothing at the path.
    NotFound(PathBuf),
    Spawn(String),
    Timeout(Duration),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NotFound(p) => write!(f, "no file at {}", p.display()),
            RunError::Spawn(e) => write!(f, "could not start it: {e}"),
            RunError::Timeout(d) => write!(f, "it did not answer within {}s", d.as_secs()),
        }
    }
}

impl std::error::Error for RunError {}

pub(crate) struct Output {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Runs `path` with `args`, stopping it at `timeout`.
pub(crate) fn run(path: &Path, args: &[String], timeout: Duration) -> Result<Output, RunError> {
    run_with(path, args, &[], None, timeout)
}

/// [`run`] with extra environment variables and, when `input` is given, that
/// text on stdin (closed once written).
pub(crate) fn run_with(
    path: &Path,
    args: &[String],
    env: &[(&str, &std::ffi::OsStr)],
    input: Option<&str>,
    timeout: Duration,
) -> Result<Output, RunError> {
    run_in(path, args, env, input, None, timeout)
}

/// [`run_with`] in the folder `cwd`, when given.
pub(crate) fn run_in(
    path: &Path,
    args: &[String],
    env: &[(&str, &std::ffi::OsStr)],
    input: Option<&str>,
    cwd: Option<&Path>,
    timeout: Duration,
) -> Result<Output, RunError> {
    if !path.is_file() {
        return Err(RunError::NotFound(path.to_path_buf()));
    }
    let mut command = Command::new(path);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    command
        .args(args)
        .env("PATH", child_path(path, std::env::var_os("PATH")))
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: an app in the tray has no console to lend.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|e| RunError::Spawn(e.to_string()))?;
    // Written from its own thread, so a child that does not read cannot
    // block the timeout below.
    let feed = input.zip(child.stdin.take()).map(|(text, mut pipe)| {
        let text = text.to_string();
        thread::spawn(move || {
            use std::io::Write;
            let _ = pipe.write_all(text.as_bytes());
        })
    });
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    };
    let stdout = drain(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let stderr = drain(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Timeout(timeout));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(RunError::Spawn(e.to_string())),
        }
    };
    if let Some(feed) = feed {
        let _ = feed.join();
    }
    Ok(Output {
        code: status.code(),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

pub(crate) fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).lines().next().unwrap_or("").trim().to_string()
}

/// An app started from Finder gets launchd's short PATH without nvm or
/// Homebrew, and a Node script (`#!/usr/bin/env node`) finds the `node` it
/// was installed with next to itself or its link target.
pub fn child_path(path: &Path, inherited: Option<OsString>) -> OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push = |d: Option<&Path>| {
        if let Some(d) = d {
            if !dirs.iter().any(|x| x == d) {
                dirs.push(d.to_path_buf());
            }
        }
    };
    push(path.parent());
    if let Ok(real) = std::fs::canonicalize(path) {
        push(real.parent());
    }
    for dir in system_dirs() {
        push(Some(&dir));
    }
    let inherited = inherited.unwrap_or_else(|| {
        OsString::from(if cfg!(windows) { "" } else { "/usr/bin:/bin:/usr/sbin:/sbin" })
    });
    for d in std::env::split_paths(&inherited) {
        push(Some(&d));
    }
    std::env::join_paths(dirs).unwrap_or(inherited)
}

/// The folders a command the owner installed is usually in, in the order
/// the app tries them: `~/.local/bin`, each nvm Node's newest first, then
/// Homebrew's.
pub fn bin_dirs(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![home.join(".local/bin")];
    let mut nvm: Vec<(Vec<u64>, PathBuf)> = std::fs::read_dir(home.join(".nvm/versions/node"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let version: Vec<u64> = name
                .trim_start_matches('v')
                .split('.')
                .map(|p| p.parse().ok())
                .collect::<Option<_>>()?;
            Some((version, e.path().join("bin")))
        })
        .collect();
    nvm.sort_by(|a, b| b.0.cmp(&a.0));
    out.extend(nvm.into_iter().map(|(_, p)| p));
    if cfg!(windows) {
        out.extend(windows_dirs(home));
    }
    out.extend(system_dirs());
    out
}

/// Homebrew's folders on macOS; none on Windows, whose own come from
/// [`windows_dirs`].
fn system_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        Vec::new()
    } else {
        vec![PathBuf::from("/opt/homebrew/bin"), PathBuf::from("/usr/local/bin")]
    }
}

/// Where Windows keeps the commands the owner installed: npm's global
/// folder, Node itself, nvm-windows, Volta, scoop, WinGet's links and the
/// agy installer's folder, from the variables that name them.
fn windows_dirs(home: &Path) -> Vec<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let roaming = var("APPDATA").unwrap_or_else(|| home.join("AppData").join("Roaming"));
    let local = var("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData").join("Local"));
    let mut out = vec![roaming.join("npm")];
    out.extend(var("NVM_SYMLINK"));
    out.extend(var("ProgramFiles").map(|p| p.join("nodejs")));
    out.push(local.join("Volta").join("bin"));
    out.push(home.join("scoop").join("shims"));
    out.push(local.join("Microsoft").join("WinGet").join("Links"));
    out.push(local.join("agy").join("bin"));
    out
}

/// The files a command `name` may be in `dir`, in the order they are
/// tried: `dir/name`; on Windows a native program and npm's shim,
/// `name.exe` and `name.cmd`. A `.cmd` runs through `cmd.exe`, which
/// `std::process::Command` does itself.
pub fn program_files(dir: &Path, name: &str) -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![dir.join(format!("{name}.exe")), dir.join(format!("{name}.cmd"))]
    } else {
        vec![dir.join(name)]
    }
}

/// `dir/name` when it is a file, or on Windows the first of `name` with a
/// `PATHEXT` extension (`.COM;.EXE;.BAT;.CMD` when the variable is unset).
/// On Windows a file of the bare name is no program: npm puts a `sh`
/// script for Git Bash there, beside `name.cmd`.
pub fn program_in(dir: &Path, name: &str) -> Option<PathBuf> {
    let plain = dir.join(name);
    if !cfg!(windows) || Path::new(name).extension().is_some() {
        return plain.is_file().then_some(plain);
    }
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    exts.split(';')
        .filter(|e| !e.is_empty())
        .map(|e| dir.join(format!("{name}{}", e.to_ascii_lowercase())))
        .find(|p| p.is_file())
}

/// The first `name` on this process's `PATH`, then in Homebrew's folders:
/// where a program the app runs looks for it.
pub fn on_path(name: &str) -> Option<PathBuf> {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&inherited)
        .chain(system_dirs())
        .filter(|d| d.is_absolute())
        .find_map(|d| program_in(&d, name))
}

/// `~/…` for paths under `home`, so the UI never shows the user name. The
/// parts are joined by `/` on every system, as the UI splits them.
pub fn display_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => {
            let parts: Vec<String> =
                rest.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            format!("~/{}", parts.join("/"))
        }
        Err(_) => path.display().to_string(),
    }
}

/// Expands a leading `~/` typed in a settings field (or `~\` on Windows).
pub fn expand_path(text: &str, home: &Path) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        home.to_path_buf()
    } else if let Some(rest) =
        text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\").filter(|_| cfg!(windows)))
    {
        home.join(rest)
    } else {
        PathBuf::from(text)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_hung_program_is_stopped_at_the_timeout() {
        let dir = std::env::temp_dir().join(format!("qs-runner-timeout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hangs");
        std::fs::write(&path, "#!/bin/sh\nsleep 5\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let started = Instant::now();
        let err = run(&path, &[], Duration::from_millis(200)).err().unwrap();
        assert_eq!(err, RunError::Timeout(Duration::from_millis(200)));
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
