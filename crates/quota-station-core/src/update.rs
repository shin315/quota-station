// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Cập nhật từ mã nguồn" (M7): `git pull --ff-only` in the clone set in
//! settings, then that clone's `scripts/install.sh`, which builds the app and
//! puts it where the running copy is. Nothing runs unless the owner presses
//! the button, and nothing is pulled from anywhere but the clone's own
//! upstream.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::runner::{self, RunError};

/// `git pull` waits for the network, never for a password (below).
pub const PULL_TIMEOUT: Duration = Duration::from_secs(120);
/// A cold release build with LTO took a few minutes when measured.
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// The script, relative to the clone.
pub const INSTALL_SCRIPT: &str = "scripts/install.sh";

/// Why a folder typed in settings is not a clone to update from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceError {
    /// No folder at that path.
    Missing,
    /// A folder without `.git` or without `scripts/install.sh`.
    NotSource,
}

impl SourceError {
    pub fn code(self) -> &'static str {
        match self {
            SourceError::Missing => "missing",
            SourceError::NotSource => "notSource",
        }
    }
}

/// A clone of Quota Station: a git work tree (`.git` is a folder, or a file
/// in a linked work tree) with the install script.
pub fn check_source(dir: &Path) -> Result<(), SourceError> {
    if !dir.is_dir() {
        return Err(SourceError::Missing);
    }
    if !dir.join(".git").exists() || !dir.join(INSTALL_SCRIPT).is_file() {
        return Err(SourceError::NotSource);
    }
    Ok(())
}

/// What an update runs. [`Tools::system`] for the app; the tests point
/// these at temporary repositories and a fake script.
#[derive(Debug, Clone)]
pub struct Tools {
    pub git: PathBuf,
    /// Runs the install script: macOS's own bash.
    pub bash: PathBuf,
    /// Added to the environment of both, after the app's own.
    pub env: Vec<(String, OsString)>,
}

impl Tools {
    pub fn system() -> Tools {
        let git = ["/opt/homebrew/bin/git", "/usr/local/bin/git", "/usr/bin/git"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file())
            .unwrap_or_else(|| PathBuf::from("/usr/bin/git"));
        Tools { git, bash: PathBuf::from("/bin/bash"), env: Vec::new() }
    }
}

/// Where an update is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    Pulling,
    Installing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    /// The clone had nothing new and the running copy was built from it.
    UpToDate { commit: String },
    /// The new copy is in place; it runs once the app opens again.
    Installed { commit: String, version: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateError {
    Source {
        reason: SourceError,
    },
    /// `git pull --ff-only` refused or failed: its last line.
    Pull {
        detail: String,
    },
    /// The install script failed: its own error line, or the last line it
    /// printed.
    Install {
        detail: String,
    },
    Timeout {
        step: Step,
    },
    Spawn {
        detail: String,
    },
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Source { reason } => write!(f, "not a source folder: {}", reason.code()),
            UpdateError::Pull { detail } => write!(f, "git pull failed: {detail}"),
            UpdateError::Install { detail } => write!(f, "install.sh failed: {detail}"),
            UpdateError::Timeout { step } => write!(f, "timed out while {step:?}"),
            UpdateError::Spawn { detail } => write!(f, "could not start: {detail}"),
        }
    }
}

impl std::error::Error for UpdateError {}

/// The environment of `git pull`: it never asks for a password or a host
/// key, since nobody can answer.
fn pull_env() -> Vec<(&'static str, OsString)> {
    vec![
        ("GIT_TERMINAL_PROMPT", OsString::from("0")),
        ("GIT_SSH_COMMAND", OsString::from("ssh -o BatchMode=yes")),
    ]
}

/// The install script's arguments: into `dest`, the folder that holds the
/// running app, and without opening it (the app opens again on its own).
pub fn install_args(script: &Path, dest: &Path) -> Vec<String> {
    vec![
        script.display().to_string(),
        "--dest".into(),
        dest.display().to_string(),
        "--no-open".into(),
    ]
}

/// Pulls, then installs into `dest` unless `running` (the commit the
/// running copy was built from) is the clone's commit already. `on_step`
/// hears each step as it starts; `log` gets the install script's output.
pub fn update(
    tools: &Tools,
    source: &Path,
    dest: &Path,
    running: Option<&str>,
    log: Option<&Path>,
    on_step: &dyn Fn(Step),
) -> Result<Outcome, UpdateError> {
    check_source(source).map_err(|reason| UpdateError::Source { reason })?;
    let env: Vec<(&str, OsString)> = pull_env()
        .into_iter()
        .chain(tools.env.iter().map(|(k, v)| (k.as_str(), v.clone())))
        .collect();
    let env_refs: Vec<(&str, &std::ffi::OsStr)> =
        env.iter().map(|(k, v)| (*k, v.as_os_str())).collect();

    on_step(Step::Pulling);
    let dir = source.display().to_string();
    let pull = ["-C", &dir, "pull", "--ff-only"].map(String::from);
    let out = runner::run_with(&tools.git, &pull, &env_refs, None, PULL_TIMEOUT)
        .map_err(|e| failed(e, Step::Pulling))?;
    if out.code != Some(0) {
        return Err(UpdateError::Pull { detail: last_line(&out.stderr, &out.stdout) });
    }
    let head = ["-C", &dir, "rev-parse", "HEAD"].map(String::from);
    let out = runner::run_with(&tools.git, &head, &env_refs, None, PULL_TIMEOUT)
        .map_err(|e| failed(e, Step::Pulling))?;
    let commit = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.code != Some(0) || commit.is_empty() {
        return Err(UpdateError::Pull { detail: last_line(&out.stderr, &out.stdout) });
    }
    if running == Some(commit.as_str()) {
        return Ok(Outcome::UpToDate { commit });
    }

    on_step(Step::Installing);
    let args = install_args(&source.join(INSTALL_SCRIPT), dest);
    let tool_env: Vec<(&str, &std::ffi::OsStr)> =
        tools.env.iter().map(|(k, v)| (k.as_str(), v.as_os_str())).collect();
    let out = runner::run_with(&tools.bash, &args, &tool_env, None, INSTALL_TIMEOUT)
        .map_err(|e| failed(e, Step::Installing))?;
    if let Some(log) = log {
        write_log(log, &out.stdout, &out.stderr);
    }
    if out.code != Some(0) {
        return Err(UpdateError::Install { detail: script_error(&out.stderr, &out.stdout) });
    }
    Ok(Outcome::Installed { commit, version: version(source) })
}

fn failed(e: RunError, step: Step) -> UpdateError {
    match e {
        RunError::Timeout(_) => UpdateError::Timeout { step },
        other => UpdateError::Spawn { detail: other.to_string() },
    }
}

/// The version the clone builds, from `src-tauri/tauri.conf.json`.
pub fn version(source: &Path) -> Option<String> {
    let text = std::fs::read_to_string(source.join("src-tauri/tauri.conf.json")).ok()?;
    let conf: serde_json::Value = serde_json::from_str(&text).ok()?;
    conf.get("version")?.as_str().map(String::from)
}

/// The last line that says something, from `first` or else `second`.
fn last_line(first: &[u8], second: &[u8]) -> String {
    [first, second]
        .iter()
        .find_map(|b| {
            String::from_utf8_lossy(b)
                .lines()
                .rev()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(truncate)
        })
        .unwrap_or_default()
}

/// The script's own `install.sh: error: …` line, else the last line.
fn script_error(stderr: &[u8], stdout: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    match text.lines().rev().find_map(|l| l.trim().strip_prefix("install.sh: error: ")) {
        Some(e) => truncate(e),
        None => last_line(stderr, stdout),
    }
}

fn truncate(line: &str) -> String {
    const MAX: usize = 300;
    if line.chars().count() <= MAX {
        line.to_string()
    } else {
        let cut: String = line.chars().take(MAX).collect();
        format!("{cut}…")
    }
}

/// Keeps the last install's output, readable only by the owner.
fn write_log(file: &Path, stdout: &[u8], stderr: &[u8]) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::remove_file(file);
    if let Ok(mut f) =
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(file)
    {
        let _ = f.write_all(stdout);
        let _ = f.write_all(b"\n--- stderr ---\n");
        let _ = f.write_all(stderr);
    }
}

/// The folder that holds the `.app` this executable runs from:
/// `/Applications` for `/Applications/Quota Station.app/Contents/MacOS/…`.
/// `None` for a binary outside a bundle, as `tauri dev` runs it.
pub fn install_dest(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let is_app = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension().is_some_and(|e| e == "app");
    is_app.then(|| app.parent().map(Path::to_path_buf)).flatten()
}
