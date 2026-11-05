// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The version rule and the Windows branches are adapted from Cockpit Tools
// `src-tauri/src/commands/account.rs`,
// `src-tauri/src/commands/system_config_types.rs`,
// `src-tauri/src/modules/process_path_resolution.rs`,
// `src-tauri/src/modules/process_launch_candidates.rs` and
// `src-tauri/src/modules/process_close_lifecycle.rs` (1.3.70, commit
// 244b957); see NOTICE for the changes.

//! Antigravity, as a switch sees it: is it installed, which version, is it
//! running, quit it politely, open it again.
//!
//! On macOS, Antigravity.app; every question but the first is a short
//! program run, never a file this app opens:
//!
//! - installed: the bundle is there (`fs::Bundle::present`, which opens
//!   nothing);
//! - version: `plutil -extract CFBundleShortVersionString raw` on its
//!   `Info.plist`;
//! - running: `pgrep -f /Applications/Antigravity.app/Contents/MacOS/Antigravity`;
//! - quit: `osascript -e 'tell application id "com.google.antigravity" to quit'`,
//!   the same as choosing Quit in its menu, so the app may save its work or
//!   ask first. Nothing here ever kills it;
//! - open: `open -b com.google.antigravity`.
//!
//! On Windows, `Antigravity.exe` in its install folders ([`install_roots`]):
//!
//! - installed: the first of its folders that holds `Antigravity.exe` (or
//!   `Electron.exe`), asked through `fs::Bundle`, which only reads;
//! - version: `resources\app\product.json` (`ideVersion`, else `version`),
//!   else the executable's VersionInfo, else the Uninstall entry that names
//!   it, both through PowerShell;
//! - running: a process of that executable without `--type=`, from the
//!   process table;
//! - quit: `taskkill /PID <pid> /T` for each, without `/F`: Windows tells
//!   its windows to close, as when the owner closes them, so the app may
//!   save its work or ask first. Nothing here ever kills it;
//! - open: the executable started in its folder with `--reuse-window`,
//!   apart from this app.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::app_dir::Bundle;
use crate::runner;

pub const BUNDLE_ID: &str = "com.google.antigravity";
pub const APP_PATH: &str = "/Applications/Antigravity.app";
/// What `pgrep -f` looks for: the main executable's path on the command line.
pub const PROCESS: &str = "/Applications/Antigravity.app/Contents/MacOS/Antigravity";
/// The first version that keeps its login in the shared Keychain item.
pub const MIN_VERSION: &str = "2.0.0";
/// How long a quit may take before the switch gives up.
pub const QUIT_WAIT: Duration = Duration::from_secs(20);
/// Each `pgrep`, `plutil` or `open`.
const STEP: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(250);

/// What is installed at [`APP_PATH`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Installed {
    /// No bundle at [`APP_PATH`] (on Windows, no `Antigravity.exe` in its
    /// install folders): a switch only changes agy's login.
    Missing,
    /// 2.0 or later.
    Supported { version: String },
    /// Older than 2.0: it keeps its login elsewhere, and the app does not
    /// switch it.
    TooOld { version: String },
    /// The bundle is there, but its version could not be read or holds no
    /// number: the switch stops before anything is written, since the app
    /// cannot tell where this one keeps its login.
    Unreadable { detail: String },
}

/// The version compare: the numbers in each, part by part.
fn parse_version_parts(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left_parts = parse_version_parts(left);
    let right_parts = parse_version_parts(right);
    if left_parts.is_empty() || right_parts.is_empty() {
        return None;
    }
    let max_len = left_parts.len().max(right_parts.len());
    for index in 0..max_len {
        let left_value = left_parts.get(index).copied().unwrap_or(0);
        let right_value = right_parts.get(index).copied().unwrap_or(0);
        match left_value.cmp(&right_value) {
            std::cmp::Ordering::Equal => {}
            ordering => return Some(ordering),
        }
    }
    Some(std::cmp::Ordering::Equal)
}

/// What a version string says about switching.
pub fn classify(version: &str) -> Installed {
    let version = version.trim().to_string();
    match compare_versions(&version, MIN_VERSION) {
        Some(std::cmp::Ordering::Less) => Installed::TooOld { version },
        Some(_) => Installed::Supported { version },
        None => Installed::Unreadable { detail: format!("version {version:?} has no number") },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuitError {
    /// `pgrep` did not say whether it runs, so a quit cannot be confirmed.
    Unknown(String),
    /// Still running when the wait ran out: it may be asking to save, or
    /// refused to quit.
    StillRunning,
}

impl std::fmt::Display for QuitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuitError::Unknown(e) => write!(f, "could not tell whether Antigravity runs: {e}"),
            QuitError::StillRunning => write!(f, "Antigravity.app did not quit"),
        }
    }
}

impl std::error::Error for QuitError {}

/// The programs the questions run, and where the app is.
#[cfg(not(windows))]
#[derive(Debug, Clone)]
pub struct Desktop {
    plutil: PathBuf,
    pgrep: PathBuf,
    osascript: PathBuf,
    open: PathBuf,
    app: PathBuf,
    process: String,
}

#[cfg(not(windows))]
impl Desktop {
    /// The real Antigravity.app, through macOS's own tools.
    pub fn system() -> Self {
        // The tests run fakes; one that reached for the real app stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake Antigravity.app");
        }
        Desktop {
            plutil: PathBuf::from("/usr/bin/plutil"),
            pgrep: PathBuf::from("/usr/bin/pgrep"),
            osascript: PathBuf::from("/usr/bin/osascript"),
            open: PathBuf::from("/usr/bin/open"),
            app: PathBuf::from(APP_PATH),
            process: PROCESS.to_string(),
        }
    }

    /// Fakes for the tests: `plutil`, `pgrep`, `osascript` and `open` in
    /// `bin`, and an app bundle at `app`.
    #[cfg(feature = "test-support")]
    pub fn fake(bin: &Path, app: &Path) -> Self {
        Desktop {
            plutil: bin.join("plutil"),
            pgrep: bin.join("pgrep"),
            osascript: bin.join("osascript"),
            open: bin.join("open"),
            app: app.to_path_buf(),
            process: app.join("Contents/MacOS/Antigravity").display().to_string(),
        }
    }

    fn run(&self, program: &Path, args: &[&str]) -> Result<runner::Output, String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        runner::run(program, &args, STEP).map_err(|e| e.to_string())
    }

    /// Whether Antigravity.app is installed, and which version. `Missing`
    /// only when the bundle is not there; a bundle whose version cannot be
    /// read is `Unreadable`.
    pub fn installed(&self) -> Installed {
        if !Bundle::new(&self.app).present() {
            return Installed::Missing;
        }
        let plist = self.app.join("Contents/Info.plist").display().to_string();
        let read = |key: &str| match self
            .run(&self.plutil, &["-extract", key, "raw", "-o", "-", &plist])
        {
            Ok(out) if out.code == Some(0) => {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if v.is_empty() {
                    Err(format!("{key} is empty"))
                } else {
                    Ok(v)
                }
            }
            Ok(out) => {
                Err(format!("plutil exited {:?}: {}", out.code, runner::first_line(&out.stderr)))
            }
            Err(e) => Err(e),
        };
        match read("CFBundleShortVersionString").or_else(|_| read("CFBundleVersion")) {
            Ok(version) => classify(&version),
            Err(detail) => Installed::Unreadable { detail },
        }
    }

    /// Whether it runs: `pgrep` exits 0 for a match and 1 for none.
    pub fn running(&self) -> Result<bool, String> {
        let out = self.run(&self.pgrep, &["-f", &self.process])?;
        match out.code {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            code => Err(format!("pgrep exited {code:?}: {}", runner::first_line(&out.stderr))),
        }
    }

    /// Asks it to quit and waits up to `wait` for it to be gone. It is never
    /// killed: one that does not quit is left running and reported.
    pub fn quit(&self, wait: Duration) -> Result<(), QuitError> {
        let deadline = Instant::now() + wait;
        let script = format!("tell application id \"{BUNDLE_ID}\" to quit");
        // osascript waits for the app's answer; a dialog in the app can hold
        // it, so it gets only the time left, and its answer is not trusted:
        // the process list decides.
        let args = ["-e".to_string(), script];
        let _ = runner::run(&self.osascript, &args, wait);
        loop {
            match self.running() {
                Ok(false) => return Ok(()),
                Ok(true) if Instant::now() >= deadline => return Err(QuitError::StillRunning),
                Ok(true) => thread::sleep(POLL),
                Err(e) => return Err(QuitError::Unknown(e)),
            }
        }
    }

    /// Opens it again.
    pub fn open(&self) -> Result<(), String> {
        let out = self.run(&self.open, &["-b", BUNDLE_ID])?;
        if out.code == Some(0) {
            Ok(())
        } else {
            Err(runner::first_line(&out.stderr))
        }
    }
}

// ===== Windows =====

/// The folders Antigravity is looked for in on Windows, in this order:
/// `%LOCALAPPDATA%\Programs\Antigravity`, `%PROGRAMFILES%\Antigravity`,
/// `%PROGRAMFILES(X86)%\Antigravity`; one whose variable is not set is
/// left out.
#[cfg(windows)]
pub fn install_roots() -> Vec<PathBuf> {
    let under = |var: &str, sub: &[&str]| {
        let base = PathBuf::from(std::env::var_os(var)?);
        Some(sub.iter().fold(base, |p, s| p.join(s)))
    };
    [
        under("LOCALAPPDATA", &["Programs", "Antigravity"]),
        under("PROGRAMFILES", &["Antigravity"]),
        under("PROGRAMFILES(X86)", &["Antigravity"]),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The executables started, in each folder, in this order.
#[cfg(any(windows, test))]
pub const EXES: [&str; 2] = ["Antigravity.exe", "Electron.exe"];

/// One process as Windows lists it.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub pid: u32,
    /// Its executable, when Windows tells it.
    pub exe: Option<PathBuf>,
    /// Its words joined by spaces.
    pub command: String,
}

/// What a switch asks Windows about Antigravity, behind a seam so the tests
/// give their own ([`Desktop::fake`]).
#[cfg(windows)]
pub trait Host: Send + Sync {
    /// Every process; `None` when they cannot be listed.
    fn processes(&self) -> Option<Vec<Task>>;
    /// Asks `pid` and the processes it started to close: `taskkill /PID
    /// <pid> /T`, never `/F`. Its answer is not trusted: the process list
    /// decides.
    fn close(&self, pid: u32);
    /// Starts `exe` in its folder, apart from this app.
    fn start(&self, exe: &Path) -> Result<(), String>;
    /// `exe`'s version from its VersionInfo, else from the Uninstall entry
    /// that names it.
    fn version_info(&self, exe: &Path) -> Result<String, String>;
}

/// What `taskkill` is given to ask `pid` and the processes it started to
/// close: never `/F`.
#[cfg(any(windows, test))]
pub fn taskkill_args(pid: u32) -> [String; 3] {
    ["/PID".to_string(), pid.to_string(), "/T".to_string()]
}

/// What Antigravity is started with.
#[cfg(any(windows, test))]
pub const START_ARGS: [&str; 1] = ["--reuse-window"];

/// An executable's path normalized: quotes and `\\?\` dropped, `/` as `\`,
/// lower case, so two spellings of one executable compare equal.
#[cfg(any(windows, test))]
pub fn normalize_exe(path: &str) -> String {
    let mut value = path.trim().trim_matches('"').replace('/', "\\");
    if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
        value = format!("\\\\{rest}");
    } else if let Some(rest) = value.strip_prefix("\\\\?\\") {
        value = rest.to_string();
    }
    value.to_lowercase()
}

/// The executable in a command line: the quoted first word, else
/// everything up to the first `.exe`, else up to the first space.
#[cfg(any(windows, test))]
pub fn exe_from_command_line(command: &str) -> Option<String> {
    let value = command.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(rest) = value.strip_prefix('"') {
        if let Some(end) = rest.find('"').filter(|end| *end > 0) {
            return Some(rest[..end].to_string());
        }
    }
    let lower = value.to_ascii_lowercase();
    if let Some(at) = lower.find(".exe") {
        if !value[..at].contains('"') {
            return Some(value[..at + 4].trim().to_string());
        }
    }
    Some(value.split(' ').next().unwrap_or(value).to_string())
}

/// Whether `task` is Antigravity's own process, one of `exes`: its
/// executable (or, when Windows does not tell it, the one its command line
/// names) is one of them, and it is not one of Chromium's helpers
/// (`--type=`).
#[cfg(any(windows, test))]
pub fn is_app(task: &Task, exes: &[String]) -> bool {
    let exe = match &task.exe {
        Some(exe) => normalize_exe(&exe.to_string_lossy()),
        None => match exe_from_command_line(&task.command) {
            Some(exe) => normalize_exe(&exe),
            None => return false,
        },
    };
    exes.contains(&exe) && !task.command.to_ascii_lowercase().contains("--type=")
}

/// The version in product.json's fields, from that file's text.
#[cfg(any(windows, test))]
fn product_json_version(text: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    ["ideVersion", "version"]
        .iter()
        .find_map(|key| {
            value
                .get(*key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
        })
        .map(str::to_string)
}

/// The version in what the PowerShell scripts print: `ProductVersion`,
/// else `FileVersion`, else `DisplayVersion`.
#[cfg(any(windows, test))]
pub fn powershell_version(stdout: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok()?;
    ["ProductVersion", "FileVersion", "DisplayVersion"]
        .iter()
        .find_map(|key| {
            value
                .get(*key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
        })
        .map(str::to_string)
}

/// Antigravity's install folders and the way to ask Windows about it.
#[cfg(windows)]
#[derive(Clone)]
pub struct Desktop {
    roots: Vec<PathBuf>,
    host: std::sync::Arc<dyn Host>,
}

#[cfg(windows)]
impl std::fmt::Debug for Desktop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Desktop").field("roots", &self.roots).finish_non_exhaustive()
    }
}

#[cfg(windows)]
impl Desktop {
    /// The real Antigravity, through Windows' own tools.
    pub fn system() -> Self {
        // The tests give fakes; one that reached for the real app stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake Antigravity");
        }
        Desktop { roots: install_roots(), host: std::sync::Arc::new(SystemHost) }
    }

    /// Antigravity looked for in `roots`, asked about through `host`.
    #[cfg(feature = "test-support")]
    pub fn fake(roots: Vec<PathBuf>, host: impl Host + 'static) -> Self {
        Desktop { roots, host: std::sync::Arc::new(host) }
    }

    /// Every executable that counts as Antigravity, normalized.
    fn exes(&self) -> Vec<String> {
        self.roots
            .iter()
            .flat_map(|root| EXES.iter().map(move |exe| root.join(exe)))
            .map(|path| normalize_exe(&path.to_string_lossy()))
            .collect()
    }

    /// The first install folder that holds one of [`EXES`], and that one.
    fn found(&self) -> Option<(Bundle, PathBuf)> {
        self.roots.iter().find_map(|root| {
            let bundle = Bundle::new(root);
            let exe = EXES.iter().copied().find(|exe| bundle.has(&[*exe]))?;
            Some((bundle, root.join(exe)))
        })
    }

    /// Whether Antigravity is installed, and which version. `Missing` only
    /// when no folder holds its executable; one whose version cannot be
    /// read is `Unreadable`.
    pub fn installed(&self) -> Installed {
        let Some((root, exe)) = self.found() else {
            return Installed::Missing;
        };
        let product = [["resources", "app", "product.json"].as_slice(), &["app", "product.json"]]
            .into_iter()
            .find_map(|parts| product_json_version(&root.read_to_string(parts).ok()?));
        match product.map(Ok).unwrap_or_else(|| self.host.version_info(&exe)) {
            Ok(version) => classify(&version),
            Err(detail) => Installed::Unreadable { detail },
        }
    }

    /// Antigravity's own processes, by pid.
    fn pids(&self) -> Result<Vec<u32>, String> {
        let tasks = self.host.processes().ok_or("could not list the processes")?;
        let exes = self.exes();
        Ok(tasks.iter().filter(|t| is_app(t, &exes)).map(|t| t.pid).collect())
    }

    /// Whether it runs.
    pub fn running(&self) -> Result<bool, String> {
        self.pids().map(|pids| !pids.is_empty())
    }

    /// Asks it to close and waits up to `wait` for it to be gone. It is
    /// never killed: one that does not quit is left running and reported.
    pub fn quit(&self, wait: Duration) -> Result<(), QuitError> {
        let deadline = Instant::now() + wait;
        for pid in self.pids().map_err(QuitError::Unknown)? {
            self.host.close(pid);
        }
        loop {
            match self.running() {
                Ok(false) => return Ok(()),
                Ok(true) if Instant::now() >= deadline => return Err(QuitError::StillRunning),
                Ok(true) => thread::sleep(POLL),
                Err(e) => return Err(QuitError::Unknown(e)),
            }
        }
    }

    /// Opens it again.
    pub fn open(&self) -> Result<(), String> {
        let (_, exe) = self.found().ok_or("Antigravity.exe is not installed")?;
        self.host.start(&exe)
    }
}

/// `%SystemRoot%\System32\<parts>`.
#[cfg(windows)]
fn system32(parts: &[&str]) -> PathBuf {
    let root = std::env::var_os("SystemRoot").map(PathBuf::from);
    let base = root.unwrap_or_else(|| PathBuf::from("C:\\Windows")).join("System32");
    parts.iter().fold(base, |p, part| p.join(part))
}

/// The variable the scripts read the executable's path from, so no
/// path is ever quoted into a script.
#[cfg(windows)]
const EXE_VAR: &str = "QUOTA_STATION_ANTIGRAVITY_EXE";

/// The script that prints the executable's VersionInfo.
#[cfg(windows)]
const VERSION_INFO_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
$p = [Environment]::GetEnvironmentVariable('QUOTA_STATION_ANTIGRAVITY_EXE', 'Process')
if ([string]::IsNullOrWhiteSpace($p)) { exit 3 }
if (-not (Test-Path -LiteralPath $p -PathType Leaf)) { exit 2 }
$v = (Get-Item -LiteralPath $p).VersionInfo
if ([string]::IsNullOrWhiteSpace($v.ProductVersion) -and [string]::IsNullOrWhiteSpace($v.FileVersion)) { exit 4 }
[pscustomobject]@{
  ProductName = $v.ProductName
  ProductVersion = $v.ProductVersion
  FileVersion = $v.FileVersion
} | ConvertTo-Json -Compress
"#;

/// The script that prints the Uninstall entry naming the executable.
#[cfg(windows)]
const UNINSTALL_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

function Normalize-RegistryPath([string]$value) {
  if ([string]::IsNullOrWhiteSpace($value)) { return $null }
  $clean = $value.Trim().Trim('"')
  $clean = $clean -replace ',\d+$',''
  try { return [System.IO.Path]::GetFullPath($clean) } catch { return $clean }
}

$exe = [Environment]::GetEnvironmentVariable('QUOTA_STATION_ANTIGRAVITY_EXE', 'Process')
if ([string]::IsNullOrWhiteSpace($exe)) { exit 3 }
$exe = [System.IO.Path]::GetFullPath($exe)

$roots = @(
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)

$match = Get-ItemProperty -Path $roots -ErrorAction SilentlyContinue |
  Where-Object {
    $_.DisplayName -like 'Antigravity*' -and (
      ((Normalize-RegistryPath $_.DisplayIcon) -ieq $exe) -or
      ($_.InstallLocation -and $exe.StartsWith(
        (Normalize-RegistryPath $_.InstallLocation).TrimEnd('\') + '\',
        [System.StringComparison]::OrdinalIgnoreCase
      ))
    )
  } |
  Select-Object -First 1

if (-not $match) { exit 4 }

[pscustomobject]@{
  DisplayName = $match.DisplayName
  DisplayVersion = $match.DisplayVersion
} | ConvertTo-Json -Compress
"#;

/// Windows' own tools: `sysinfo`'s process table, `taskkill`, PowerShell.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemHost;

#[cfg(windows)]
impl SystemHost {
    fn powershell(&self, script: &str, exe: &Path, wait: Duration) -> Result<String, String> {
        let program = system32(&["WindowsPowerShell", "v1.0", "powershell.exe"]);
        let args =
            ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]
                .map(String::from);
        let out = runner::run_with(&program, &args, &[(EXE_VAR, exe.as_os_str())], None, wait)
            .map_err(|e| format!("PowerShell: {e}"))?;
        if out.code != Some(0) {
            return Err(format!("PowerShell exited {:?}", out.code));
        }
        powershell_version(&String::from_utf8_lossy(&out.stdout))
            .ok_or_else(|| "PowerShell printed no version".to_string())
    }
}

#[cfg(windows)]
impl Host for SystemHost {
    fn processes(&self) -> Option<Vec<Task>> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
        let mut system = System::new();
        let kind =
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_exe(UpdateKind::Always);
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        let tasks = system
            .processes()
            .iter()
            .map(|(pid, p)| Task {
                pid: pid.as_u32(),
                exe: p.exe().map(Path::to_path_buf),
                command: p
                    .cmd()
                    .iter()
                    .map(|w| w.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" "),
            })
            .collect();
        Some(tasks)
    }

    fn close(&self, pid: u32) {
        let _ = runner::run(&system32(&["taskkill.exe"]), &taskkill_args(pid), STEP);
    }

    fn start(&self, exe: &Path) -> Result<(), String> {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = Command::new(exe);
        if let Some(dir) = exe.parent() {
            command.current_dir(dir);
        }
        command
            .args(START_ARGS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    fn version_info(&self, exe: &Path) -> Result<String, String> {
        self.powershell(VERSION_INFO_SCRIPT, exe, STEP).or_else(|first| {
            self.powershell(UNINSTALL_SCRIPT, exe, STEP)
                .map_err(|second| format!("{first}; {second}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_versions_from_2_0_are_switched() {
        assert_eq!(classify("2.24.0"), Installed::Supported { version: "2.24.0".into() });
        assert_eq!(classify("2.0"), Installed::Supported { version: "2.0".into() });
        assert_eq!(classify("v10.1"), Installed::Supported { version: "v10.1".into() });
        assert_eq!(classify("1.20.5"), Installed::TooOld { version: "1.20.5".into() });
        assert_eq!(classify("1.99.99"), Installed::TooOld { version: "1.99.99".into() });
        // A version with no number stops the switch.
        assert!(matches!(classify("dev"), Installed::Unreadable { .. }));
        assert!(matches!(classify(""), Installed::Unreadable { .. }));
    }

    #[test]
    fn a_windows_executable_is_told_from_its_command_line() {
        let exe = r"C:\Users\x\AppData\Local\Programs\Antigravity\Antigravity.exe";
        assert_eq!(
            exe_from_command_line(&format!("\"{exe}\" --reuse-window")).as_deref(),
            Some(exe)
        );
        assert_eq!(exe_from_command_line(&format!("{exe} --flag")).as_deref(), Some(exe));
        let spaced = r"C:\Program Files\Antigravity\Antigravity.exe";
        assert_eq!(exe_from_command_line(&format!("{spaced} --x")).as_deref(), Some(spaced));
        assert_eq!(exe_from_command_line("antigravity --x").as_deref(), Some("antigravity"));
        assert_eq!(exe_from_command_line("  "), None);
        assert_eq!(
            normalize_exe(r"\\?\C:/Users/X/AppData/Local/Programs/Antigravity/Antigravity.EXE"),
            r"c:\users\x\appdata\local\programs\antigravity\antigravity.exe"
        );
        assert_eq!(normalize_exe(r"\\?\UNC\server\share\a.exe"), r"\\server\share\a.exe");
    }

    #[test]
    fn only_antigravitys_own_main_process_counts() {
        let exe = r"C:\Program Files\Antigravity\Antigravity.exe";
        let exes = vec![normalize_exe(exe)];
        let task = |exe: Option<&str>, command: &str| Task {
            pid: 7,
            exe: exe.map(PathBuf::from),
            command: command.to_string(),
        };
        assert!(is_app(&task(Some(exe), exe), &exes));
        assert!(is_app(&task(Some(&exe.to_uppercase()), ""), &exes));
        // Windows did not tell the executable: the command line does.
        assert!(is_app(&task(None, &format!("{exe} --reuse-window")), &exes));
        // Chromium's helpers are not the app.
        assert!(!is_app(&task(Some(exe), &format!("{exe} --type=renderer")), &exes));
        // Another program, or Antigravity elsewhere.
        assert!(!is_app(
            &task(Some(r"C:\Program Files\Antigravity IDE\Antigravity IDE.exe"), ""),
            &exes
        ));
        assert!(!is_app(&task(Some(r"D:\copy\Antigravity.exe"), ""), &exes));
        assert!(!is_app(&task(None, ""), &exes));
    }

    #[test]
    fn the_windows_version_comes_from_product_json_or_what_powershell_prints() {
        assert_eq!(
            product_json_version(r#"{"ideVersion":"2.1.0","version":"1.0"}"#).as_deref(),
            Some("2.1.0")
        );
        assert_eq!(product_json_version(r#"{"version":" 2.0.3 "}"#).as_deref(), Some("2.0.3"));
        assert_eq!(product_json_version(r#"{"version":""}"#), None);
        assert_eq!(product_json_version("not json"), None);
        assert_eq!(
            powershell_version(
                r#"{"ProductName":"Antigravity","ProductVersion":"2.0.3","FileVersion":"2.0.3.0"}"#
            )
            .as_deref(),
            Some("2.0.3")
        );
        assert_eq!(
            powershell_version(r#"{"ProductVersion":"","FileVersion":"2.0.3.0"}"#).as_deref(),
            Some("2.0.3.0")
        );
        assert_eq!(
            powershell_version(
                "{\"DisplayName\":\"Antigravity\",\"DisplayVersion\":\"2.4.1\"}\r\n"
            )
            .as_deref(),
            Some("2.4.1")
        );
        assert_eq!(powershell_version(""), None);
        assert_eq!(EXES, ["Antigravity.exe", "Electron.exe"]);
    }

    #[test]
    fn antigravity_is_asked_to_close_and_started_with_the_expected_words() {
        assert_eq!(taskkill_args(4242), ["/PID", "4242", "/T"]);
        assert!(!taskkill_args(1).iter().any(|a| a.eq_ignore_ascii_case("/F")));
        assert_eq!(START_ARGS, ["--reuse-window"]);
    }

    /// The real VersionInfo script, on a program every Windows has: it reads
    /// only that file's version, never Antigravity's. A cold PowerShell on a
    /// busy CI runner can take longer than the app's ten seconds, so the
    /// script gets a minute here: this checks what it prints, not how fast.
    #[cfg(windows)]
    #[test]
    fn powershell_reads_an_executables_version() {
        let notepad = system32(&["notepad.exe"]);
        let version =
            SystemHost.powershell(VERSION_INFO_SCRIPT, &notepad, Duration::from_secs(60)).unwrap();
        assert!(version.chars().next().is_some_and(|c| c.is_ascii_digit()), "{version}");
    }
}
