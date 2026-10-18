//! Finding and running quota-axi.
//!
//! The app only ever runs quota-axi reads. Every read passes
//! `--no-credential-refresh` and reuses quota-axi's shared cache with
//! `--max-age`, so a refresh adds no vendor call while the cache is young.
//! The one exception is [`grant_keychain`], which the user starts from a
//! button: it lets quota-axi ask macOS for Keychain access once.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::axi::{self, ParseError, Response};

/// How long a read may run. The first `agy` read of a cold cache runs
/// `agy -p /quota` and took about 6 seconds when measured.
pub const READ_TIMEOUT: Duration = Duration::from_secs(45);
/// The Keychain prompt waits for the user.
pub const GRANT_TIMEOUT: Duration = Duration::from_secs(180);
/// quota-axi's shared cache may answer while younger than this.
pub const MAX_AGE: &str = "120s";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    Claude,
    Agy,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Claude => "claude",
            ProviderId::Agy => "agy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// No path is set and none of the usual places has quota-axi.
    NotConfigured,
    /// Nothing at the configured path.
    NotFound(PathBuf),
    Spawn(String),
    Timeout(Duration),
    /// Exited without a report on stdout.
    Exit {
        code: Option<i32>,
        stderr: String,
    },
    Parse(ParseError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NotConfigured => write!(f, "quota-axi not found"),
            RunError::NotFound(p) => write!(f, "no file at {}", p.display()),
            RunError::Spawn(e) => write!(f, "could not start quota-axi: {e}"),
            RunError::Timeout(d) => write!(f, "quota-axi did not answer within {}s", d.as_secs()),
            RunError::Exit { code, stderr } => match code {
                Some(c) => write!(f, "quota-axi exited {c}: {stderr}"),
                None => write!(f, "quota-axi was stopped: {stderr}"),
            },
            RunError::Parse(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RunError {}

/// The read the app makes for one provider.
pub fn read_args(provider: ProviderId) -> Vec<String> {
    ["--provider", provider.as_str(), "--json", "--max-age", MAX_AGE, "--no-credential-refresh"]
        .map(String::from)
        .to_vec()
}

/// One Claude read that may show the macOS Keychain prompt, the remedy
/// quota-axi names for `keychain_prompt_required`.
pub fn grant_args() -> Vec<String> {
    ["--provider", "claude", "--json", "--allow-keychain-prompt", "--no-credential-refresh"]
        .map(String::from)
        .to_vec()
}

pub fn read(path: &Path, provider: ProviderId) -> Result<Response, RunError> {
    report(path, &read_args(provider), READ_TIMEOUT)
}

pub fn grant_keychain(path: &Path) -> Result<Response, RunError> {
    report(path, &grant_args(), GRANT_TIMEOUT)
}

/// `quota-axi --version`, for example `0.1.61+cswap.4`.
pub fn version(path: &Path) -> Result<String, RunError> {
    let out = run(path, &["--version".to_string()], Duration::from_secs(15))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.code == Some(0) && !text.is_empty() {
        Ok(text)
    } else {
        Err(RunError::Exit { code: out.code, stderr: first_line(&out.stderr) })
    }
}

/// The local fork adds the claude-swap slots, the rotation pool and the
/// Cockpit accounts; upstream quota-axi has none of them.
pub fn is_fork(version: &str) -> bool {
    version.contains("+cswap")
}

/// A report is read from stdout whatever the exit code: quota-axi exits
/// non-zero when a provider needs attention and still prints the report.
fn report(path: &Path, args: &[String], timeout: Duration) -> Result<Response, RunError> {
    let out = run(path, args, timeout)?;
    let text = String::from_utf8_lossy(&out.stdout);
    match axi::parse(&text) {
        Ok(r) => Ok(r),
        Err(ParseError::Malformed(_)) if out.code != Some(0) => {
            Err(RunError::Exit { code: out.code, stderr: first_line(&out.stderr) })
        }
        Err(e) => Err(RunError::Parse(e)),
    }
}

struct Output {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(path: &Path, args: &[String], timeout: Duration) -> Result<Output, RunError> {
    if !path.is_file() {
        return Err(RunError::NotFound(path.to_path_buf()));
    }
    let mut child = Command::new(path)
        .args(args)
        .env("PATH", child_path(path, std::env::var_os("PATH")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RunError::Spawn(e.to_string()))?;
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
    Ok(Output {
        code: status.code(),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).lines().next().unwrap_or("").trim().to_string()
}

/// quota-axi is a Node script (`#!/usr/bin/env node`), and an app started
/// from Finder gets launchd's short PATH without nvm or Homebrew. The
/// script's own directory, and its link target's, hold the `node` it was
/// installed with.
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
    push(Some(Path::new("/opt/homebrew/bin")));
    push(Some(Path::new("/usr/local/bin")));
    let inherited = inherited.unwrap_or_else(|| OsString::from("/usr/bin:/bin:/usr/sbin:/sbin"));
    for d in std::env::split_paths(&inherited) {
        push(Some(&d));
    }
    std::env::join_paths(dirs).unwrap_or(inherited)
}

/// Where quota-axi is usually installed, in the order the app tries them.
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![home.join(".local/bin/quota-axi")];
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
            Some((version, e.path().join("bin/quota-axi")))
        })
        .collect();
    nvm.sort_by(|a, b| b.0.cmp(&a.0));
    out.extend(nvm.into_iter().map(|(_, p)| p));
    out.push(PathBuf::from("/opt/homebrew/bin/quota-axi"));
    out.push(PathBuf::from("/usr/local/bin/quota-axi"));
    out
}

/// The first usual place that has quota-axi.
pub fn discover(home: &Path) -> Option<PathBuf> {
    candidates(home).into_iter().find(|p| p.is_file())
}

/// `~/…` for paths under `home`, so the UI never shows the user name.
pub fn display_path(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// Expands a leading `~/` typed in the settings field.
pub fn expand_path(text: &str, home: &Path) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        home.to_path_buf()
    } else if let Some(rest) = text.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_hung_quota_axi_is_stopped_at_the_timeout() {
        let dir = std::env::temp_dir().join(format!("qs-runner-timeout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quota-axi");
        std::fs::write(&path, "#!/bin/sh\nsleep 5\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let started = Instant::now();
        let err =
            report(&path, &read_args(ProviderId::Claude), Duration::from_millis(200)).unwrap_err();
        assert_eq!(err, RunError::Timeout(Duration::from_millis(200)));
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
