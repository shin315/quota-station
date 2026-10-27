// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Cài đặt › Chung › Cập nhật Agent: Claude Code, the Codex CLI and the
//! Antigravity CLI on this Mac, their versions, the newest for the way each
//! was installed, and the tool's own update when the owner presses
//! "Cập nhật".
//!
//! Finding a tool runs only its `--version`. The newest version is asked
//! where that install method publishes it, the same place the tool's own
//! updater asks: Anthropic's release channel, Homebrew's cask, npm,
//! GitHub's latest release of openai/codex, Google's agy manifest. Nothing
//! about this Mac or its accounts is sent, and no login, account or quota
//! file is read or written.
//!
//! The update is the tool's own command: `claude update`, `brew upgrade
//! --cask <cask>`, `npm install -g @openai/codex@latest`, `codex update`,
//! `agy update`. Claude Code (native and Homebrew) and agy keep a running
//! session on its old files until it is opened again, so they update while
//! sessions run; Codex from Homebrew or npm replaces the files its sessions
//! use, so it waits until none of them is open. An install the app cannot
//! update safely (Claude Code from npm, a copy inside another app, a folder
//! that needs an administrator, a layout it does not know) is left to the
//! owner, with the command to run in Terminal where there is one.

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::app_dir::AppDir;
use crate::os::{Process, ProcessList};
use crate::release::{self, newer};
use crate::runner::{self, RunError};

/// In the app's folder: what the last check found and each tool's last update.
pub const FILE: &str = "cli-tools.json";
/// In the app's folder: each update's output, `<tool>.log`.
pub const LOG_DIR: &str = "cli-updates";
/// Homebrew may download a few hundred megabytes; npm builds nothing.
pub const UPDATE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const VERSION_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
/// A cask's or a package's JSON is a few kilobytes.
const MAX_BODY: u64 = 1024 * 1024;
/// A reason shown under a row is a line, not the log.
const MAX_DETAIL: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tool {
    Claude,
    Codex,
    Agy,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::Claude, Tool::Codex, Tool::Agy];

    /// The command's name.
    pub fn program(self) -> &'static str {
        match self {
            Tool::Claude => "claude",
            Tool::Codex => "codex",
            Tool::Agy => "agy",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tool::Claude => "Claude Code",
            Tool::Codex => "Codex CLI",
            Tool::Agy => "Antigravity CLI",
        }
    }

    pub fn parse(word: &str) -> Option<Tool> {
        Tool::ALL.into_iter().find(|t| t.program() == word)
    }

    /// Whether a session keeps running on its old files through an update:
    /// Claude Code applies an update at the next launch and keeps the old
    /// versions; agy updates itself in the background while it runs. Codex
    /// from Homebrew or npm replaces the folder its sessions call into.
    pub fn safe_while_running(self) -> bool {
        !matches!(self, Tool::Codex)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Install {
    /// Claude Code's own installer (`~/.local/share/claude/versions`).
    Native,
    Homebrew,
    Npm,
    /// Codex's `install.sh` (`~/.codex/packages/standalone`).
    Standalone,
    /// agy's `install.sh`, the only way it is installed.
    Script,
}

/// Why the app leaves a tool's update to the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandsOff {
    /// Claude Code from npm, the old way: an update writes over the files a
    /// running session uses. `claude install` moves it to the installer.
    Npm,
    /// A copy inside another app, which updates it.
    Bundled,
    /// Its folder needs an administrator.
    Unwritable,
    /// A layout the app does not know: it does not guess a command.
    Unknown,
}

/// Where a tool is and how it was installed, from its path alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub install: Option<Install>,
    pub hands_off: Option<HandsOff>,
    /// `Codex.app`, for a copy inside an app.
    pub bundled_by: Option<String>,
    /// What the app runs to update it: the program and its arguments.
    pub update: Option<Vec<String>>,
    /// The update as the owner would type it.
    pub command: Option<String>,
    /// The Homebrew cask, for the newest version.
    pub cask: Option<String>,
    /// The folder the update writes; a process running from inside it is
    /// one of this install's.
    pub root: Option<PathBuf>,
}

impl Layout {
    fn hands_off(kind: HandsOff) -> Layout {
        Layout {
            install: None,
            hands_off: Some(kind),
            bundled_by: None,
            update: None,
            command: None,
            cask: None,
            root: None,
        }
    }

    fn runs(install: Install, update: Vec<String>, command: &str, root: PathBuf) -> Layout {
        Layout {
            install: Some(install),
            hands_off: None,
            bundled_by: None,
            update: Some(update),
            command: Some(command.into()),
            cask: None,
            root: Some(root),
        }
    }
}

fn words(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(w) => Some(w.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// The path up to and including its `n`th part (counting normal parts).
fn upto(path: &Path, n: usize) -> PathBuf {
    let mut out = PathBuf::new();
    let mut seen = 0;
    for c in path.components() {
        out.push(c);
        if matches!(c, Component::Normal(_)) {
            seen += 1;
            if seen > n {
                break;
            }
        }
    }
    out
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// How `tool` was installed, from `path` (where the owner's shell finds it)
/// and `target` (that path with its links followed).
pub fn layout(tool: Tool, path: &Path, target: &Path, home: &Path) -> Layout {
    let parts = words(target);
    if let Some(app) = parts.iter().find(|w| w.ends_with(".app")) {
        return Layout { bundled_by: Some(app.clone()), ..Layout::hands_off(HandsOff::Bundled) };
    }
    if let Some(i) = parts.iter().position(|w| w == "Caskroom") {
        let Some(cask) = parts.get(i + 1) else { return Layout::hands_off(HandsOff::Unknown) };
        let prefix = upto(target, i).parent().map(Path::to_path_buf).unwrap_or_default();
        let brew = prefix.join("bin").join("brew");
        let update = vec![text(&brew), "upgrade".into(), "--cask".into(), cask.clone()];
        let command = format!("brew upgrade --cask {cask}");
        return Layout {
            cask: Some(cask.clone()),
            ..Layout::runs(Install::Homebrew, update, &command, upto(target, i + 1))
        };
    }
    let package = match tool {
        Tool::Claude => ["@anthropic-ai", "claude-code"],
        Tool::Codex => ["@openai", "codex"],
        Tool::Agy => ["", ""],
    };
    if let Some(i) = parts.windows(3).position(|w| w[0] == "node_modules" && w[1..] == package) {
        if tool == Tool::Claude {
            return Layout {
                command: Some("claude install".into()),
                ..Layout::hands_off(HandsOff::Npm)
            };
        }
        // npm's global folder is `<prefix>/lib/node_modules`; bun's and
        // pnpm's are laid out otherwise.
        if i == 0 || parts[i - 1] != "lib" {
            return Layout::hands_off(HandsOff::Unknown);
        }
        let prefix = upto(target, i - 1).parent().map(Path::to_path_buf).unwrap_or_default();
        let npm = prefix.join("bin").join("npm");
        let update = vec![text(&npm), "install".into(), "-g".into(), "@openai/codex@latest".into()];
        return Layout::runs(
            Install::Npm,
            update,
            "npm install -g @openai/codex@latest",
            upto(target, i + 2),
        );
    }
    let own = |args: &[&str]| {
        let mut v = vec![text(path)];
        v.extend(args.iter().map(|a| a.to_string()));
        v
    };
    match tool {
        Tool::Claude if target.starts_with(home.join(".local/share/claude/versions")) => {
            Layout::runs(
                Install::Native,
                own(&["update"]),
                "claude update",
                home.join(".local/share/claude"),
            )
        }
        // The old local npm install (`~/.claude/local`).
        Tool::Claude if target.starts_with(home.join(".claude/local")) => {
            Layout { command: Some("claude install".into()), ..Layout::hands_off(HandsOff::Npm) }
        }
        Tool::Codex if target.starts_with(home.join(".codex/packages/standalone")) => Layout::runs(
            Install::Standalone,
            own(&["update"]),
            "codex update",
            home.join(".codex/packages/standalone"),
        ),
        // agy has one installer, which puts a single file wherever it was told.
        Tool::Agy => Layout::runs(
            Install::Script,
            own(&["update"]),
            "agy update",
            target.parent().map(Path::to_path_buf).unwrap_or_default(),
        ),
        _ => Layout::hands_off(HandsOff::Unknown),
    }
}

/// The layout once its folder turned out to need an administrator: the
/// owner runs the command (npm's with `sudo`) in Terminal.
pub fn unwritable(mut l: Layout) -> Layout {
    if l.install == Some(Install::Npm) {
        l.command = l.command.map(|c| format!("sudo {c}"));
    }
    l.hands_off = Some(HandsOff::Unwritable);
    l.update = None;
    l
}

/// The version in a tool's `--version` (`2.1.311 (Claude Code)`,
/// `codex-cli 0.163.0`, `1.4.7`) or in a release tag (`rust-v0.164.0`).
pub fn parse_version(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',').find_map(|w| {
        let w = w.trim_start_matches("rust-").trim_start_matches(['v', 'V']);
        let core = w.split(['-', '+']).next()?;
        let parts: Vec<&str> = core.split('.').collect();
        (parts.len() >= 2
            && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())))
        .then(|| w.to_string())
    })
}

/// `<path> --version`, its version.
pub fn version(path: &Path) -> Option<String> {
    let out = runner::run(path, &["--version".into()], VERSION_TIMEOUT).ok()?;
    if out.code != Some(0) {
        return None;
    }
    parse_version(&String::from_utf8_lossy(&out.stdout))
}

/// One tool as found on this Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub tool: Tool,
    /// Where the owner's shell finds it.
    pub path: PathBuf,
    /// The same with its links followed.
    pub target: PathBuf,
    pub version: Option<String>,
    pub layout: Layout,
}

/// Where `tool` is: `~/.local/bin`, nvm's newest Node, Homebrew's folders,
/// then `/usr/local/bin`, as the app finds the other commands it runs.
pub fn locate(tool: Tool, home: &Path) -> Option<PathBuf> {
    runner::bin_dirs(home).into_iter().find_map(|d| runner::program_in(&d, tool.program()))
}

/// What `tool` at `path` is, running only its `--version`.
pub fn inspect(tool: Tool, path: &Path, home: &Path) -> Found {
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    // The target has its links followed, so the home folder is compared so too.
    let real_home = std::fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    let mut l = layout(tool, path, &target, &real_home);
    if l.hands_off.is_none() && l.root.as_deref().is_some_and(|r| !crate::os::writable(r)) {
        l = unwritable(l);
    }
    Found { tool, path: path.to_path_buf(), version: version(path), target, layout: l }
}

/// Every tool that is installed.
pub fn find_all(home: &Path) -> Vec<Found> {
    Tool::ALL.into_iter().filter_map(|t| locate(t, home).map(|p| inspect(t, &p, home))).collect()
}

// ===== Sessions =====

fn is_node(word: &str) -> bool {
    word == "node" || word.ends_with("/node")
}

/// Whether `command` (a `ps` line) runs `found`: its program is the tool's
/// name as the shell finds it (`claude --resume`), the tool's own path, or
/// a file inside its install (Codex's native binary under npm). A copy
/// elsewhere (inside an app, another program's `node_modules`) is not this
/// one, and neither is the `node` that starts an npm install's binary.
pub fn runs(found: &Found, command: &str) -> bool {
    let Some(first) = command.split(' ').next() else { return false };
    if is_node(first) {
        return false;
    }
    if first == found.tool.program() {
        return true;
    }
    let exe = Path::new(first);
    if !exe.is_absolute() {
        return false;
    }
    exe == found.path
        || exe == found.target
        || found.layout.root.as_deref().is_some_and(|r| exe.starts_with(r))
}

/// How many of `found`'s processes run.
pub fn running(found: &Found, processes: &[Process]) -> u32 {
    processes.iter().filter(|p| runs(found, &p.command)).count() as u32
}

// ===== The newest version =====

/// Where the newest versions are asked. The app always uses [`Feeds::live`].
#[derive(Debug, Clone)]
pub struct Feeds {
    claude: String,
    brew: String,
    npm: String,
    github: String,
    agy: String,
}

impl Feeds {
    pub fn live() -> Feeds {
        // The tests run a local fake; one that reached for the real ones stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use fake feeds");
        }
        Feeds {
            claude: "https://downloads.claude.ai".into(),
            brew: "https://formulae.brew.sh".into(),
            npm: "https://registry.npmjs.org".into(),
            github: "https://api.github.com".into(),
            agy: "https://antigravity-cli-auto-updater-974169037036.us-central1.run.app".into(),
        }
    }

    /// Every feed under `base`: `/claude`, `/brew`, `/npm`, `/github`, `/agy`.
    #[cfg(feature = "test-support")]
    pub fn at(base: &str) -> Feeds {
        let base = base.trim_end_matches('/');
        Feeds {
            claude: format!("{base}/claude"),
            brew: format!("{base}/brew"),
            npm: format!("{base}/npm"),
            github: format!("{base}/github"),
            agy: format!("{base}/agy"),
        }
    }
}

/// What a feed answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Newest {
    Version(String),
    /// Nowhere to ask for this install.
    NoFeed,
    /// The feed was not reached, or did not answer with a version.
    Failed,
}

/// Claude Code's release channel: `autoUpdatesChannel` in its settings,
/// `latest` unless it says `stable`. Only that key is read.
pub fn claude_channel(home: &Path) -> &'static str {
    let set = std::fs::read_to_string(home.join(".claude/settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("autoUpdatesChannel").and_then(|c| c.as_str()).map(String::from));
    if set.as_deref() == Some("stable") {
        "stable"
    } else {
        "latest"
    }
}

/// agy's manifest for this Mac: `darwin_arm64` or `darwin_amd64`.
fn agy_platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => other,
    };
    format!("{os}_{arch}")
}

/// The address to ask for `found`'s newest version, and the JSON key that
/// holds it (`None`: the answer is the version as text).
pub fn feed(feeds: &Feeds, found: &Found, home: &Path) -> Option<(String, Option<&'static str>)> {
    let l = &found.layout;
    match (found.tool, l.install?) {
        (Tool::Claude, Install::Native) => {
            Some((format!("{}/claude-code-releases/{}", feeds.claude, claude_channel(home)), None))
        }
        (_, Install::Homebrew) => {
            Some((format!("{}/api/cask/{}.json", feeds.brew, l.cask.as_deref()?), Some("version")))
        }
        (Tool::Codex, Install::Npm) => {
            Some((format!("{}/@openai/codex/latest", feeds.npm), Some("version")))
        }
        (Tool::Codex, Install::Standalone) => {
            Some((format!("{}/repos/openai/codex/releases/latest", feeds.github), Some("tag_name")))
        }
        (Tool::Agy, Install::Script) => {
            Some((format!("{}/manifests/{}.json", feeds.agy, agy_platform()), Some("version")))
        }
        _ => None,
    }
}

/// The version in a feed's answer.
pub fn read_feed(body: &str, key: Option<&str>) -> Option<String> {
    let raw = match key {
        None => body.trim().to_string(),
        Some(k) => {
            serde_json::from_str::<serde_json::Value>(body).ok()?.get(k)?.as_str()?.to_string()
        }
    };
    // A cask's version may carry a build after a comma.
    parse_version(raw.split(',').next()?)
}

/// Asks `found`'s feed for its newest version.
pub fn newest(feeds: &Feeds, found: &Found, home: &Path) -> Newest {
    let Some((url, key)) = feed(feeds, found, home) else { return Newest::NoFeed };
    let Ok(client) = reqwest::blocking::Client::builder().timeout(HTTP_TIMEOUT).build() else {
        return Newest::Failed;
    };
    let resp = client
        .get(&url)
        .header("Accept", "application/json, text/plain")
        .header("User-Agent", "Quota-Station")
        .send();
    let Ok(resp) = resp else { return Newest::Failed };
    if !resp.status().is_success() {
        return Newest::Failed;
    }
    let mut body = Vec::new();
    if resp.take(MAX_BODY).read_to_end(&mut body).is_err() {
        return Newest::Failed;
    }
    read_feed(&String::from_utf8_lossy(&body), key).map_or(Newest::Failed, Newest::Version)
}

// ===== What the app keeps =====

/// Why an update did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Failure {
    /// The download did not reach its server.
    Offline,
    /// A folder needs an administrator.
    Permission,
    /// Homebrew or the tool's own updater was already running.
    Busy,
    /// The command failed for another reason; the detail is its last line.
    Exit,
    Timeout,
    /// The command succeeded and the version stayed.
    Unchanged,
    /// A session of a tool that must wait opened in the meantime.
    Started,
    /// The processes could not be listed, so a tool that must wait did not
    /// update.
    Sessions,
}

/// A tool's last update from the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Last {
    pub ok: bool,
    /// Unix seconds.
    pub at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The update's output, `~/…`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}

impl Last {
    pub fn done(from: &str, to: &str, at: i64) -> Last {
        Last {
            ok: true,
            at,
            from: Some(from.into()),
            to: Some(to.into()),
            failure: None,
            detail: None,
            log: None,
        }
    }

    pub fn failed(failure: Failure, detail: impl Into<String>, at: i64) -> Last {
        Last {
            ok: false,
            at,
            from: None,
            to: None,
            failure: Some(failure),
            detail: Some(detail.into()),
            log: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Kept {
    /// Unix seconds of the last check that ended.
    pub checked_at: Option<i64>,
    /// No feed answered the last check.
    pub offline: bool,
    /// The newest version of each tool, as its feed said.
    pub latest: BTreeMap<Tool, String>,
    pub last: BTreeMap<Tool, Last>,
}

impl Kept {
    /// Whether the daily check is due at `now`: a day after one that
    /// reached the feeds, an hour after one that did not.
    pub fn due(&self, now: i64) -> bool {
        match self.checked_at {
            None => true,
            Some(at) => now - at >= if self.offline { release::RETRY } else { release::DAILY },
        }
    }
}

pub fn load(app: &AppDir) -> Kept {
    app.read_to_string(&app.path().join(FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppDir, kept: &Kept) -> io::Result<()> {
    let text = serde_json::to_string_pretty(kept).map_err(io::Error::other)?;
    app.write_private(&app.path().join(FILE), &text)
}

/// Asks each installed tool's feed and returns what the app keeps after,
/// carrying `before`'s updates and, for a feed that did not answer, its
/// version from before.
pub fn check(feeds: &Feeds, found: &[Found], home: &Path, before: &Kept, now: i64) -> Kept {
    let mut after = Kept { checked_at: Some(now), last: before.last.clone(), ..Kept::default() };
    let (mut asked, mut failed) = (0, 0);
    for f in found {
        match newest(feeds, f, home) {
            Newest::Version(v) => {
                asked += 1;
                after.latest.insert(f.tool, v);
            }
            Newest::NoFeed => {}
            Newest::Failed => {
                asked += 1;
                failed += 1;
                if let Some(v) = before.latest.get(&f.tool) {
                    after.latest.insert(f.tool, v.clone());
                }
            }
        }
    }
    after.offline = asked > 0 && failed == asked;
    after
}

// ===== The row =====

/// One tool's row in Cài đặt › Chung › Cập nhật Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub tool: Tool,
    pub version: Option<String>,
    pub latest: Option<String>,
    pub newer: bool,
    pub install: Option<Install>,
    /// `~/.local/bin/claude → ~/.local/share/claude/versions/2.1.311`.
    pub path: Option<String>,
    pub command: Option<String>,
    pub hands_off: Option<HandsOff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundled_by: Option<String>,
    pub running: u32,
    pub safe_while_running: bool,
    pub updating: bool,
    pub last: Option<Last>,
}

/// The row for `tool`; `found` is `None` when it is not installed.
pub fn row(
    tool: Tool,
    found: Option<&Found>,
    kept: &Kept,
    running: u32,
    updating: bool,
    home: &Path,
) -> ToolView {
    let Some(f) = found else {
        return ToolView {
            tool,
            version: None,
            latest: None,
            newer: false,
            install: None,
            path: None,
            command: None,
            hands_off: None,
            bundled_by: None,
            running: 0,
            safe_while_running: tool.safe_while_running(),
            updating: false,
            last: kept.last.get(&tool).cloned(),
        };
    };
    let latest = kept.latest.get(&tool).cloned();
    let path = runner::display_path(&f.path, home);
    let path = if f.target != f.path {
        format!("{path} → {}", runner::display_path(&f.target, home))
    } else {
        path
    };
    ToolView {
        tool,
        newer: matches!((&latest, &f.version), (Some(l), Some(v)) if newer(l, v)),
        version: f.version.clone(),
        latest,
        install: f.layout.install,
        path: Some(path),
        command: f.layout.command.clone(),
        hands_off: f.layout.hands_off,
        bundled_by: f.layout.bundled_by.clone(),
        running,
        safe_while_running: tool.safe_while_running(),
        updating,
        last: kept.last.get(&tool).cloned(),
    }
}

// ===== The update =====

/// Why the app did not start an update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The owner updates this one; the app has no command for it.
    HandsOff,
}

/// What an update's output says went wrong, and the line that says it.
pub fn diagnose(output: &str, home: &Path) -> (Failure, String) {
    let lower = output.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
    let line_with = |needles: &[&str]| {
        output
            .lines()
            .find(|l| needles.iter().any(|n| l.to_lowercase().contains(n)))
            .unwrap_or("")
            .to_string()
    };
    const BUSY: &[&str] = &[
        "already locked",
        "another active homebrew",
        "has already locked",
        "update.lock",
        "already in progress",
        "another background updater",
    ];
    const PERMISSION: &[&str] = &["permission denied", "eacces", "operation not permitted"];
    const OFFLINE: &[&str] = &[
        "could not resolve host",
        "enotfound",
        "econnrefused",
        "etimedout",
        "eai_again",
        "network is unreachable",
        "failed to connect",
        "could not connect",
        "curl: (6)",
        "curl: (7)",
    ];
    if has(BUSY) {
        return (Failure::Busy, short(&line_with(BUSY), home));
    }
    if has(PERMISSION) {
        // The folder a line names: `'/usr/local/lib/node_modules'` or a bare path.
        let path = output
            .lines()
            .filter(|l| PERMISSION.iter().any(|n| l.to_lowercase().contains(n)))
            .find_map(|l| {
                l.split(|c: char| c.is_whitespace() || c == '\'' || c == '"' || c == ',')
                    .find(|w| w.starts_with('/'))
                    .map(|p| runner::display_path(Path::new(p.trim_end_matches(':')), home))
            });
        return (Failure::Permission, path.unwrap_or_else(|| short(&line_with(PERMISSION), home)));
    }
    if has(OFFLINE) {
        return (Failure::Offline, short(&line_with(OFFLINE), home));
    }
    let last = output.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    (Failure::Exit, short(last, home))
}

/// A line for the row: the home folder as `~`, no longer than a sentence.
fn short(line: &str, home: &Path) -> String {
    let home = home.to_string_lossy();
    let line = if home.len() > 1 { line.replace(home.as_ref(), "~") } else { line.to_string() };
    line.trim().chars().take(MAX_DETAIL).collect()
}

/// Updates `found` with its own command, unless it is a tool that waits
/// and one of its sessions runs (or the processes cannot be listed). The
/// command's output goes to `log`. The version is read again after: an
/// update that changed nothing says so.
pub fn update(
    found: &Found,
    processes: &dyn ProcessList,
    log: &Path,
    home: &Path,
    timeout: Duration,
    now: i64,
) -> Result<Last, Refused> {
    let Some(argv) = found.layout.update.as_ref().filter(|_| found.layout.hands_off.is_none())
    else {
        return Err(Refused::HandsOff);
    };
    if !found.tool.safe_while_running() {
        match processes.all() {
            None => return Ok(Last::failed(Failure::Sessions, "", now)),
            Some(all) => {
                let n = running(found, &all);
                if n > 0 {
                    return Ok(Last::failed(Failure::Started, n.to_string(), now));
                }
            }
        }
    }
    let before = found.version.clone().unwrap_or_default();
    let (program, args) = argv.split_first().ok_or(Refused::HandsOff)?;
    let env: [(&str, &std::ffi::OsStr); 3] = [
        ("NONINTERACTIVE", "1".as_ref()),
        ("HOMEBREW_NO_ENV_HINTS", "1".as_ref()),
        ("CODEX_NON_INTERACTIVE", "1".as_ref()),
    ];
    let ran = runner::run_with(Path::new(program), args, &env, None, timeout);
    let shown = runner::display_path(log, home);
    let mut text = format!("$ {}\n", found.layout.command.as_deref().unwrap_or(program));
    let outcome = match ran {
        Err(RunError::Timeout(_)) => {
            text += "(stopped: time limit)\n";
            Last::failed(Failure::Timeout, "", now)
        }
        Err(e) => {
            text += &format!("{e}\n");
            Last::failed(Failure::Exit, short(&e.to_string(), home), now)
        }
        Ok(out) => {
            let output = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            text += &output;
            text += &format!("\n(exit {})\n", out.code.map_or("signal".into(), |c| c.to_string()));
            if out.code != Some(0) {
                let (failure, detail) = diagnose(&output, home);
                Last::failed(failure, detail, now)
            } else {
                match version(&found.path) {
                    Some(after) if newer(&after, &before) || before.is_empty() => {
                        Last::done(&before, &after, now)
                    }
                    _ => Last::failed(Failure::Unchanged, "", now),
                }
            }
        }
    };
    let _ = log.parent().map(std::fs::create_dir_all);
    let _ = std::fs::write(log, text);
    Ok(if outcome.ok { outcome } else { Last { log: Some(shown), ..outcome } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read_from_each_tools_words() {
        assert_eq!(parse_version("2.1.311 (Claude Code)\n").as_deref(), Some("2.1.311"));
        assert_eq!(parse_version("codex-cli 0.163.0").as_deref(), Some("0.163.0"));
        assert_eq!(parse_version("1.4.7").as_deref(), Some("1.4.7"));
        assert_eq!(parse_version("rust-v0.164.0").as_deref(), Some("0.164.0"));
        assert_eq!(parse_version("codex-cli").as_deref(), None);
        assert_eq!(parse_version("").as_deref(), None);
    }

    #[test]
    fn a_feed_answers_with_text_or_a_key() {
        assert_eq!(read_feed("2.1.312\n", None).as_deref(), Some("2.1.312"));
        assert_eq!(
            read_feed(r#"{"version":"0.164.0"}"#, Some("version")).as_deref(),
            Some("0.164.0")
        );
        assert_eq!(
            read_feed(r#"{"version":"1.2.3,abc"}"#, Some("version")).as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            read_feed(r#"{"tag_name":"rust-v0.164.0"}"#, Some("tag_name")).as_deref(),
            Some("0.164.0")
        );
        assert_eq!(read_feed("<html>", None), None);
        assert_eq!(read_feed(r#"{"other":1}"#, Some("version")), None);
    }
}
