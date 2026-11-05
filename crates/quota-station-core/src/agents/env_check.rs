// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Environment variables that override an agent's login (M14, I): an
//! `ANTHROPIC_API_KEY` in `~/.zshrc` makes Claude Code in Terminal use that
//! key instead of the Claude account in use, whatever the app switches.
//!
//! [`check`] looks for the eleven names in [`VARS`] in the five shell files,
//! the `env` of Claude Code's user settings (`~/.claude/settings.json`, and
//! `settings.local.json` beside it), which every run of Claude Code applies,
//! and launchd's environment ([`Place`]). It learns only whether a name is
//! set, and where: the file and its line, or launchd. It never learns a
//! value:
//!
//! - A shell file is never run or `source`d. Its bytes are read, each line
//!   is matched by the name at its start (`export NAME=` or `NAME=`), only
//!   the line number is kept, and the bytes are zeroed before they go. A
//!   file that starts with a byte order mark is read in its encoding
//!   (UTF-8, or UTF-16 as Windows PowerShell's `Out-File` writes).
//! - A settings file is parsed as JSON for its `env` member's keys and
//!   their lines, then zeroed the same way. Only the Claude Code names are
//!   looked for there: the other agents never read it.
//! - `launchctl getenv NAME` prints the value itself. Its output is only
//!   asked whether it is empty, then zeroed at once: never kept, logged,
//!   stored or shown.
//!
//! On Windows the places are Git Bash's three files, the two PowerShell
//! profiles (`$env:NAME = …`, matched without regard to case, as PowerShell
//! reads names; one set inside a function's body counts only when that
//! function is called, and [`Found::only_in`] names it), Claude Code's
//! settings in `%USERPROFILE%\.claude` and the registry's environment, the user's
//! (`HKCU\Environment`) and the computer's, which every app started from
//! Start gets. For the registry only the value's size is asked for, never
//! its text ([`crate::win::env_set`]).
//!
//! The app never unsets anything, and edits a file only when the owner
//! presses "Gỡ" on one place ([`super::env_remove`]), which takes out that
//! one line or key after a preview. "Mở file" opens the file in an editor
//! at the line ([`open_args`]); "Bỏ qua" is kept in the app's own folder
//! ([`load_ignored`]).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::Agent;
use crate::app_dir::AppDir;
use crate::runner::{self, RunError};

/// launchd's own command.
pub const LAUNCHCTL: &str = "/bin/launchctl";
/// One `launchctl getenv` answers in milliseconds.
const LAUNCHCTL_TIMEOUT: Duration = Duration::from_secs(5);
/// A shell file is a few kilobytes; a larger one is not read.
const MAX_BYTES: u64 = 1024 * 1024;
/// "Bỏ qua", in the app's folder.
pub const IGNORED_FILE: &str = "agents/env-ignored.json";

/// How much a variable takes over: a key or token replaces the login
/// (`bad`); an address, folder or project sends it elsewhere (`warn`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Weight {
    Bad,
    Warn,
}

/// One variable the app looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Var {
    pub name: &'static str,
    pub agent: Agent,
    pub weight: Weight,
    /// What the agent does with it, after "Trong Terminal, Claude Code ".
    pub effect: &'static str,
}

const fn var(name: &'static str, agent: Agent, weight: Weight, effect: &'static str) -> Var {
    Var { name, agent, weight, effect }
}

/// The eleven names, in the order the list shows them.
pub const VARS: [Var; 11] = [
    var(
        "ANTHROPIC_API_KEY",
        Agent::Claude,
        Weight::Bad,
        "dùng khoá này thay cho tài khoản Claude đang chọn.",
    ),
    var(
        "ANTHROPIC_AUTH_TOKEN",
        Agent::Claude,
        Weight::Bad,
        "gửi token này thay cho đăng nhập của tài khoản Claude đang chọn.",
    ),
    var(
        "ANTHROPIC_BASE_URL",
        Agent::Claude,
        Weight::Warn,
        "gửi yêu cầu tới địa chỉ này thay cho máy chủ của Anthropic.",
    ),
    var(
        "CLAUDE_CODE_OAUTH_TOKEN",
        Agent::Claude,
        Weight::Bad,
        "dùng token này thay cho tài khoản Claude đang chọn.",
    ),
    var(
        "CLAUDE_CONFIG_DIR",
        Agent::Claude,
        Weight::Warn,
        "đọc đăng nhập ở thư mục này, không phải ở thư mục nơi tài khoản được chuyển.",
    ),
    var(
        "OPENAI_API_KEY",
        Agent::Codex,
        Weight::Bad,
        "có thể dùng khoá này thay cho đăng nhập ChatGPT.",
    ),
    var(
        "OPENAI_BASE_URL",
        Agent::Codex,
        Weight::Warn,
        "gửi yêu cầu tới địa chỉ này thay cho nhà cung cấp trong config.toml.",
    ),
    var(
        "CODEX_HOME",
        Agent::Codex,
        Weight::Warn,
        "đọc cấu hình và đăng nhập ở thư mục này, không phải thư mục app đang theo dõi.",
    ),
    var(
        "GEMINI_API_KEY",
        Agent::Agy,
        Weight::Bad,
        "có thể dùng khoá này thay cho tài khoản Google đang đăng nhập.",
    ),
    var(
        "GOOGLE_API_KEY",
        Agent::Agy,
        Weight::Bad,
        "có thể dùng khoá này thay cho tài khoản Google đang đăng nhập.",
    ),
    var(
        "GOOGLE_CLOUD_PROJECT",
        Agent::Agy,
        Weight::Warn,
        "có thể tính hạn mức vào dự án Google Cloud này thay cho tài khoản đang đăng nhập.",
    ),
];

pub fn var_of(name: &str) -> Option<&'static Var> {
    VARS.iter().find(|v| v.name == name)
}

/// Where a variable can be set: five shell files in the home folder,
/// Claude Code's two user settings files, and launchd's environment, which
/// apps opened from the Dock get. On Windows: Git Bash's files, the
/// PowerShell profiles, Claude Code's settings, and the registry's
/// environment of the user and of the computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    Zshenv,
    Zprofile,
    Zshrc,
    BashProfile,
    Profile,
    Launchctl,
    Bashrc,
    /// Windows PowerShell's `$PROFILE`, the one Windows comes with.
    PsProfile,
    /// PowerShell 7's.
    #[serde(rename = "pwshProfile")]
    Pwsh,
    UserEnv,
    MachineEnv,
    /// The `env` of Claude Code's `~/.claude/settings.json`.
    ClaudeSettings,
    /// The `env` of `~/.claude/settings.local.json`, when there is one.
    #[serde(rename = "claudeLocalSettings")]
    ClaudeLocal,
}

/// How a file place sets a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// `export NAME=…` or `NAME=…`.
    Shell,
    /// `$env:NAME = …`.
    PowerShell,
    /// A key of the `env` object of a JSON settings file.
    Json,
}

impl Place {
    /// The places on a Mac.
    pub const ALL: [Place; 8] = [
        Place::Zshenv,
        Place::Zprofile,
        Place::Zshrc,
        Place::BashProfile,
        Place::Profile,
        Place::ClaudeSettings,
        Place::ClaudeLocal,
        Place::Launchctl,
    ];

    /// The places on Windows.
    pub const WINDOWS: [Place; 9] = [
        Place::BashProfile,
        Place::Bashrc,
        Place::Profile,
        Place::PsProfile,
        Place::Pwsh,
        Place::ClaudeSettings,
        Place::ClaudeLocal,
        Place::UserEnv,
        Place::MachineEnv,
    ];

    /// This system's places.
    pub fn system() -> &'static [Place] {
        if cfg!(windows) {
            &Place::WINDOWS
        } else {
            &Place::ALL
        }
    }

    /// The file's path from the home folder; launchd and the registry have
    /// none.
    pub fn file(self) -> Option<&'static str> {
        match self {
            Place::Zshenv => Some(".zshenv"),
            Place::Zprofile => Some(".zprofile"),
            Place::Zshrc => Some(".zshrc"),
            Place::BashProfile => Some(".bash_profile"),
            Place::Profile => Some(".profile"),
            Place::Bashrc => Some(".bashrc"),
            Place::PsProfile => {
                Some("Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1")
            }
            Place::Pwsh => Some("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"),
            Place::ClaudeSettings => Some(".claude/settings.json"),
            Place::ClaudeLocal => Some(".claude/settings.local.json"),
            Place::Launchctl | Place::UserEnv | Place::MachineEnv => None,
        }
    }

    /// `~/.zshrc`, `launchctl`, `$PROFILE`, or the registry's environment,
    /// as the window names them.
    pub fn label(self) -> String {
        match self {
            Place::Launchctl => "launchctl".into(),
            Place::UserEnv => "biến người dùng".into(),
            Place::MachineEnv => "biến hệ thống".into(),
            Place::PsProfile => "$PROFILE".into(),
            Place::Pwsh => "$PROFILE (PowerShell 7)".into(),
            _ => self.file().map(|f| format!("~/{f}")).unwrap_or_default(),
        }
    }

    /// The word in an id (`ANTHROPIC_API_KEY@zshrc`).
    pub fn word(self) -> &'static str {
        match self {
            Place::Zshenv => "zshenv",
            Place::Zprofile => "zprofile",
            Place::Zshrc => "zshrc",
            Place::BashProfile => "bashProfile",
            Place::Profile => "profile",
            Place::Launchctl => "launchctl",
            Place::Bashrc => "bashrc",
            Place::PsProfile => "psProfile",
            Place::Pwsh => "pwshProfile",
            Place::UserEnv => "userEnv",
            Place::MachineEnv => "machineEnv",
            Place::ClaudeSettings => "claudeSettings",
            Place::ClaudeLocal => "claudeLocalSettings",
        }
    }

    pub fn from_word(word: &str) -> Option<Place> {
        Place::ALL.into_iter().chain(Place::WINDOWS).find(|p| p.word() == word)
    }

    /// How the file sets a variable; `None` for launchd and the registry.
    pub fn syntax(self) -> Option<Syntax> {
        match self {
            Place::PsProfile | Place::Pwsh => Some(Syntax::PowerShell),
            Place::ClaudeSettings | Place::ClaudeLocal => Some(Syntax::Json),
            Place::Launchctl | Place::UserEnv | Place::MachineEnv => None,
            _ => Some(Syntax::Shell),
        }
    }

    /// Whether the place can set `v`: Claude Code's settings set only its
    /// own variables.
    fn sets(self, v: &Var) -> bool {
        self.syntax() != Some(Syntax::Json) || v.agent == Agent::Claude
    }

    /// Where an app started by the owner gets it, before anything else.
    fn opened(self) -> &'static str {
        match self {
            Place::Launchctl => "Trong các app mở từ Dock",
            Place::UserEnv | Place::MachineEnv => "Trong các app mở từ Start",
            Place::PsProfile | Place::Pwsh => "Trong PowerShell",
            Place::ClaudeSettings | Place::ClaudeLocal => "Ở mọi lần chạy",
            _ => "Trong Terminal",
        }
    }
}

/// One variable set in one place. Only its name, place and line: the
/// value is never read into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// `ANTHROPIC_API_KEY@zshrc`: what "Mở file" and "Bỏ qua" name.
    pub id: String,
    pub name: String,
    pub agent: Agent,
    pub weight: Weight,
    pub place: Place,
    /// The last line of the file that sets it (the one that wins), from 1.
    pub line: Option<u32>,
    /// The PowerShell function whose body sets it: it takes effect only
    /// when that function is called. Empty when the function's name is not
    /// a plain word; absent when the line runs as the profile loads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only_in: Option<String>,
    /// "Trong Terminal, Claude Code dùng khoá này thay cho …".
    pub effect: String,
}

impl Found {
    fn new(v: &Var, place: Place, hit: Option<Hit>) -> Found {
        let (line, only_in) = match hit {
            Some(h) => (Some(h.line), h.only_in),
            None => (None, None),
        };
        let opened = match &only_in {
            Some(f) => format!("{}, khi gọi hàm {}", place.opened(), function_name(f)),
            None => place.opened().to_string(),
        };
        Found {
            id: id_of(v.name, place),
            name: v.name.into(),
            agent: v.agent,
            weight: v.weight,
            place,
            line,
            only_in,
            effect: format!("{opened}, {} {}", v.agent.label(), v.effect),
        }
    }

    /// `~/.zshrc dòng 42`, `$PROFILE dòng 4 (trong hàm ccd)`, or
    /// `launchctl`.
    pub fn at(&self) -> String {
        let inside = match &self.only_in {
            Some(f) => format!(" (trong hàm {})", function_name(f)),
            None => String::new(),
        };
        match self.line {
            Some(n) => format!("{} dòng {n}{inside}", self.place.label()),
            None => self.place.label(),
        }
    }
}

/// A function's name as a sentence says it: `ccd`, or "không tên".
fn function_name(f: &str) -> &str {
    if f.is_empty() {
        "không tên"
    } else {
        f
    }
}

pub fn id_of(name: &str, place: Place) -> String {
    format!("{name}@{}", place.word())
}

/// An id's name and place, when it names one of [`VARS`] and a [`Place`].
pub fn parse_id(id: &str) -> Option<(&'static Var, Place)> {
    let (name, word) = id.split_once('@')?;
    Some((var_of(name)?, Place::from_word(word)?))
}

/// What one check found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// In [`VARS`] order, then [`Place`] order.
    pub found: Vec<Found>,
    /// Places that could not be looked at: a file there but unreadable or
    /// too large, or `launchctl` that did not answer. A missing file is not
    /// one: nothing is set there.
    pub unread: Vec<Place>,
}

/// Looks at the five shell files under `home` and asks `launchctl` (the
/// path, so tests pass a fake one) about each name. On Windows: Git Bash's
/// files, the PowerShell profiles and the registry ([`Place::WINDOWS`]).
pub fn check(home: &Path, launchctl: &Path) -> Check {
    #[cfg(windows)]
    {
        let _ = launchctl;
        check_with(home, &Place::WINDOWS, &|place, name| {
            crate::win::env_set(place == Place::MachineEnv, name).map_err(|_| ())
        })
    }
    #[cfg(not(windows))]
    check_with(home, &Place::ALL, &|_, name| launchctl_has(launchctl, name).map_err(|_| ()))
}

/// [`check`] of `places`, asking `set` whether a name is set in a place
/// that is not a file.
pub fn check_with(
    home: &Path,
    places: &[Place],
    set: &dyn Fn(Place, &str) -> Result<bool, ()>,
) -> Check {
    let mut out = Check::default();
    let mut found: Vec<Found> = Vec::new();
    for &place in places {
        match (place.file(), place.syntax()) {
            (Some(file), Some(syntax)) => match read_hits(&home.join(file), syntax) {
                Ok(Some(mut hits)) => {
                    for v in VARS.iter().filter(|v| place.sets(v)) {
                        if let Some(hit) = hits.remove(v.name) {
                            found.push(Found::new(v, place, Some(hit)));
                        }
                    }
                }
                Ok(None) => {}
                Err(Unreadable) => out.unread.push(place),
            },
            _ => {
                let mut failed = false;
                for v in &VARS {
                    match set(place, v.name) {
                        Ok(true) => found.push(Found::new(v, place, None)),
                        Ok(false) => {}
                        Err(_) => failed = true,
                    }
                }
                if failed {
                    out.unread.push(place);
                }
            }
        }
    }
    let order = |f: &Found| (VARS.iter().position(|v| v.name == f.name), f.place);
    found.sort_by_key(order);
    out.found = found;
    out
}

/// A file there that cannot be looked at: unreadable, too large, its
/// UTF-16 not decoding, or a settings file that is not JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

/// Where a file sets one of [`VARS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// From 1.
    pub line: u32,
    /// The PowerShell function whose body holds the line ([`Found::only_in`]).
    pub only_in: Option<String>,
}

/// For each of [`VARS`] the file sets, where it does. `None` when there is
/// no file; an error when it is there but cannot be read, is too large, its
/// encoding does not decode, or a settings file is not JSON.
fn read_hits(
    path: &Path,
    syntax: Syntax,
) -> Result<Option<BTreeMap<&'static str, Hit>>, Unreadable> {
    let Some(mut bytes) = read_file(path)? else { return Ok(None) };
    let hits = match decode(&bytes) {
        Some(mut text) => {
            let hits = scan_text(&text.body, syntax);
            text.body.fill(0);
            hits.map(Some)
        }
        None => Err(Unreadable),
    };
    // The values are in these bytes: they go now, zeroed first.
    bytes.fill(0);
    hits
}

/// A file's bytes, at most [`MAX_BYTES`]: `None` when it is not there, an
/// error when it cannot be read or is larger. The caller zeroes them.
pub fn read_file(path: &Path) -> Result<Option<Vec<u8>>, Unreadable> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Unreadable),
    };
    let meta = file.metadata().map_err(|_| Unreadable)?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(Unreadable);
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    let read = file.by_ref().take(MAX_BYTES + 1).read_to_end(&mut bytes);
    match read {
        Ok(_) if bytes.len() as u64 <= MAX_BYTES => Ok(Some(bytes)),
        _ => {
            bytes.fill(0);
            Err(Unreadable)
        }
    }
}

// ===== Encodings =====

/// How a file's text is stored: as it comes (UTF-8, or a code page whose
/// first 128 bytes are ASCII's), or after a byte order mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Encoding {
    Bytes,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

/// A file's text without its byte order mark: its own bytes, or UTF-16
/// made UTF-8, so the names (ASCII) are matched the same way in each.
#[derive(Debug)]
pub struct Text {
    pub encoding: Encoding,
    pub body: Vec<u8>,
}

/// `bytes` as [`Text`]; `None` when UTF-16 does not decode (an odd length
/// or a lone surrogate), which [`encode`] could not write back the same.
pub fn decode(bytes: &[u8]) -> Option<Text> {
    let utf16 = |rest: &[u8], le: bool| -> Option<Vec<u8>> {
        if rest.len() % 2 != 0 {
            return None;
        }
        let mut units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| {
                if le {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            })
            .collect();
        let text = String::from_utf16(&units).ok().map(String::into_bytes);
        units.fill(0);
        text
    };
    if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        Some(Text { encoding: Encoding::Utf8Bom, body: rest.to_vec() })
    } else if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        Some(Text { encoding: Encoding::Utf16Le, body: utf16(rest, true)? })
    } else if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        Some(Text { encoding: Encoding::Utf16Be, body: utf16(rest, false)? })
    } else {
        Some(Text { encoding: Encoding::Bytes, body: bytes.to_vec() })
    }
}

/// `body` written back in `encoding`, with its byte order mark. `None` when
/// UTF-16's body is not UTF-8, which a decoded body always is.
pub fn encode(encoding: Encoding, body: &[u8]) -> Option<Vec<u8>> {
    let utf16 = |le: bool| -> Option<Vec<u8>> {
        let text = std::str::from_utf8(body).ok()?;
        let mut out = if le { vec![0xFF, 0xFE] } else { vec![0xFE, 0xFF] };
        for u in text.encode_utf16() {
            out.extend(if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        Some(out)
    };
    match encoding {
        Encoding::Bytes => Some(body.to_vec()),
        Encoding::Utf8Bom => Some([b"\xEF\xBB\xBF".as_slice(), body].concat()),
        Encoding::Utf16Le => utf16(true),
        Encoding::Utf16Be => utf16(false),
    }
}

// ===== Finding the names =====

/// Where `body` (a file's text, [`decode`]d) sets each of [`VARS`]. In a
/// shell file or a profile, the last line that sets it; in a profile, the
/// last line that runs as it loads when there is one, else the last line
/// inside a function. In a settings file, the last `env` key of that name.
/// An error when a settings file is not JSON.
pub fn scan_text(body: &[u8], syntax: Syntax) -> Result<BTreeMap<&'static str, Hit>, Unreadable> {
    match syntax {
        Syntax::Shell => Ok(scan_lines(body, None)),
        Syntax::PowerShell => Ok(scan_lines(body, Some(ps_lines(body)))),
        Syntax::Json => scan_json(body),
    }
}

/// [`scan_text`] of a shell file (`ps`: none) or a profile (`ps`: its
/// [`ps_lines`]).
fn scan_lines(body: &[u8], ps: Option<Vec<PsLine>>) -> BTreeMap<&'static str, Hit> {
    let mut always: BTreeMap<&'static str, Hit> = BTreeMap::new();
    let mut inside: BTreeMap<&'static str, Hit> = BTreeMap::new();
    for (i, line) in body.split(|b| *b == b'\n').enumerate() {
        let state = ps.as_ref().and_then(|p| p.get(i));
        if state.is_some_and(|s| s.quoted) {
            continue;
        }
        for v in &VARS {
            let set = match ps {
                Some(_) => assigns_env(line, v.name.as_bytes()),
                None => assigns(line, v.name.as_bytes()),
            };
            if !set {
                continue;
            }
            let only_in = state.and_then(|s| s.function.clone());
            let hit = Hit { line: i as u32 + 1, only_in };
            if hit.only_in.is_some() {
                inside.insert(v.name, hit);
            } else {
                always.insert(v.name, hit);
            }
        }
    }
    for (name, hit) in inside {
        always.entry(name).or_insert(hit);
    }
    always
}

/// The `env` keys of a settings file. Empty text is no settings; text
/// that is not JSON is an error. Names are matched as the system matches
/// them: in any case on Windows.
fn scan_json(body: &[u8]) -> Result<BTreeMap<&'static str, Hit>, Unreadable> {
    let mut at = BTreeMap::new();
    let text = std::str::from_utf8(body).map_err(|_| Unreadable)?;
    if text.trim().is_empty() {
        return Ok(at);
    }
    let root = super::json_text::parse(text).map_err(|_| Unreadable)?;
    let Some(env) = root.get("env").filter(|m| m.value.is_object()) else { return Ok(at) };
    for v in VARS.iter().filter(|v| v.agent == Agent::Claude) {
        if let Some(i) = env_key(&env.value, v.name) {
            let start = env.value.members()[i].key_span.start;
            let line = text[..start].bytes().filter(|b| *b == b'\n').count() as u32 + 1;
            at.insert(v.name, Hit { line, only_in: None });
        }
    }
    Ok(at)
}

/// The index of the last member of `env` named `name`: JSON readers keep
/// the last of a repeated key.
pub fn env_key(env: &super::json_text::Node, name: &str) -> Option<usize> {
    let same = |k: &str| if cfg!(windows) { k.eq_ignore_ascii_case(name) } else { k == name };
    env.members().iter().rposition(|m| same(&m.key))
}

/// What a PowerShell profile's line starts inside.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PsLine {
    /// The innermost function whose body the line is in: its name, or empty
    /// when the name is not a plain word.
    pub function: Option<String>,
    /// The line starts inside a string, a here-string or a `<# … #>`
    /// comment: it sets nothing.
    pub quoted: bool,
}

/// For each line of a profile (split at `\n`), what it starts inside. The
/// text is read as PowerShell's tokens are, only as far as braces, strings
/// and comments go: `function NAME {` (or `filter`) opens a function's
/// body, any other `{` a block, `}` closes the innermost.
pub fn ps_lines(body: &[u8]) -> Vec<PsLine> {
    #[derive(Clone, Copy, PartialEq)]
    enum S {
        Code,
        Single,
        Double,
        Comment,
        Block,
        HereSingle,
        HereDouble,
    }
    let word = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'$' | b':' | b'.');
    let blank = |b: u8| b == b' ' || b == b'\t';
    let mut out = Vec::new();
    let mut s = S::Code;
    let mut stack: Vec<Option<String>> = Vec::new();
    let mut pending: Option<String> = None;
    let mut i = 0;
    let mut start = true;
    loop {
        if start {
            start = false;
            let rest = &body[i..];
            out.push(PsLine {
                function: stack.iter().rev().find_map(|f| f.clone()),
                quoted: !matches!(s, S::Code | S::Comment),
            });
            if (s == S::HereSingle && rest.starts_with(b"'@"))
                || (s == S::HereDouble && rest.starts_with(b"\"@"))
            {
                s = S::Code;
                i += 2;
                continue;
            }
        }
        let Some(&b) = body.get(i) else { break };
        let next = body.get(i + 1).copied();
        if b == b'\n' {
            if s == S::Comment {
                s = S::Code;
            }
            start = true;
            i += 1;
            continue;
        }
        match s {
            S::Code => match b {
                b'#' => s = S::Comment,
                b'<' if next == Some(b'#') => {
                    s = S::Block;
                    i += 1;
                }
                b'@' if matches!(next, Some(b'\'' | b'"')) && {
                    let after = &body[i + 2..];
                    let end = after.iter().position(|c| *c == b'\n').unwrap_or(after.len());
                    after[..end].iter().all(|c| blank(*c) || *c == b'\r')
                } =>
                {
                    s = if next == Some(b'\'') { S::HereSingle } else { S::HereDouble };
                    i += 1;
                }
                b'\'' => s = S::Single,
                b'"' => s = S::Double,
                b'`' if next != Some(b'\n') => i += 1,
                b'$' if next == Some(b'{') => {
                    // `${name}`: a variable, not a block.
                    let close = body[i..].iter().position(|c| *c == b'}' || *c == b'\n');
                    if let Some(n) = close.filter(|n| body[i + n] == b'}') {
                        i += n;
                    }
                }
                b'{' => stack.push(pending.take()),
                b'}' => {
                    stack.pop();
                }
                _ if (i == 0 || !word(body[i - 1])) && keyword(&body[i..]).is_some() => {
                    let len = keyword(&body[i..]).unwrap_or(0);
                    let mut j = i + len;
                    while body.get(j).is_some_and(|c| blank(*c)) {
                        j += 1;
                    }
                    let name_end = body[j..]
                        .iter()
                        .position(|c| c.is_ascii_whitespace() || matches!(c, b'{' | b'('))
                        .map_or(body.len(), |n| j + n);
                    let name = &body[j..name_end];
                    let plain = !name.is_empty()
                        && name.len() <= 64
                        && name.iter().all(|c| {
                            c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b':' | b'.')
                        });
                    pending = Some(if plain {
                        String::from_utf8_lossy(name).into_owned()
                    } else {
                        String::new()
                    });
                    i = name_end;
                    continue;
                }
                _ => {}
            },
            S::Single | S::HereSingle => {
                if s == S::Single && b == b'\'' {
                    if next == Some(b'\'') {
                        i += 1;
                    } else {
                        s = S::Code;
                    }
                }
            }
            S::Double | S::HereDouble => {
                if b == b'`' && next != Some(b'\n') {
                    i += 1;
                } else if s == S::Double && b == b'"' {
                    if next == Some(b'"') {
                        i += 1;
                    } else {
                        s = S::Code;
                    }
                }
            }
            S::Comment => {}
            S::Block => {
                if b == b'#' && next == Some(b'>') {
                    s = S::Code;
                    i += 1;
                }
            }
        }
        i += 1;
    }
    out
}

/// `function` or `filter` and a blank, in any case: the keyword's length.
fn keyword(at: &[u8]) -> Option<usize> {
    ["function", "filter"].iter().find_map(|k| {
        let n = k.len();
        let head = at.get(..n)?;
        let blank = at.get(n).is_some_and(|c| *c == b' ' || *c == b'\t');
        (head.eq_ignore_ascii_case(k.as_bytes()) && blank).then_some(n)
    })
}

/// `$env:NAME = …` at the start of the line, after spaces, in any case. A
/// comment, `$env:NAME -eq …` or `$env:OTHER_NAME =` is not.
fn assigns_env(line: &[u8], name: &[u8]) -> bool {
    assignment(line, name, Syntax::PowerShell).is_some()
}

/// `NAME=…` or `export NAME=…` at the start of the line, after spaces. A
/// comment, `unset NAME` or `OTHER_NAME=` is not.
fn assigns(line: &[u8], name: &[u8]) -> bool {
    assignment(line, name, Syntax::Shell).is_some()
}

/// Where the value starts on a line that sets `name` in a shell file or a
/// profile: just past the `=` and, in a profile, the blanks after it. What
/// comes before holds no value, only blanks, `export` or `$env:`, the name
/// and the `=`.
pub fn assignment(line: &[u8], name: &[u8], syntax: Syntax) -> Option<usize> {
    let blank = |b: &u8| *b == b' ' || *b == b'\t';
    let skip =
        |s: &[u8], at: usize| -> usize { at + s[at..].iter().take_while(|b| blank(b)).count() };
    let mut at = skip(line, 0);
    match syntax {
        Syntax::PowerShell => {
            line.get(at..at + 5).filter(|p| p.eq_ignore_ascii_case(b"$env:"))?;
            at += 5;
            line.get(at..at + name.len()).filter(|n| n.eq_ignore_ascii_case(name))?;
            at = skip(line, at + name.len());
            (line.get(at) == Some(&b'=') && line.get(at + 1) != Some(&b'='))
                .then(|| skip(line, at + 1))
        }
        Syntax::Shell => {
            if line[at..].starts_with(b"export") && line.get(at + 6).is_some_and(blank) {
                at = skip(line, at + 6);
            }
            line[at..].starts_with(name).then_some(())?;
            at += name.len();
            (line.get(at) == Some(&b'=')).then_some(at + 1)
        }
        Syntax::Json => None,
    }
}

/// Whether launchd's environment has `name`. `launchctl getenv` prints the
/// value: it is only asked whether it is empty, then zeroed.
pub fn launchctl_has(launchctl: &Path, name: &str) -> Result<bool, RunError> {
    let args = ["getenv".to_string(), name.to_string()];
    let mut out = runner::run(launchctl, &args, LAUNCHCTL_TIMEOUT)?;
    let set = out.code == Some(0) && out.stdout.iter().any(|b| !b.is_ascii_whitespace());
    out.stdout.fill(0);
    out.stderr.fill(0);
    Ok(set)
}

// ===== "Bỏ qua" =====

#[derive(Debug, Default, Serialize, Deserialize)]
struct Ignored {
    #[serde(default)]
    ignored: BTreeSet<String>,
}

/// The ids "Bỏ qua" was pressed on.
pub fn load_ignored(app: &AppDir) -> BTreeSet<String> {
    super::skills::load_json::<Ignored>(app, IGNORED_FILE).ignored
}

pub fn save_ignored(app: &AppDir, ids: &BTreeSet<String>) -> std::io::Result<()> {
    super::skills::save_json(app, IGNORED_FILE, &Ignored { ignored: ids.clone() })
}

/// What is still to show: the found not ignored. An ignored id that is no
/// longer found is forgotten, so the same place set again is shown again.
pub fn shown(check: &Check, ignored: &mut BTreeSet<String>) -> Vec<Found> {
    ignored.retain(|id| check.found.iter().any(|f| &f.id == id));
    check.found.iter().filter(|f| !ignored.contains(&f.id)).cloned().collect()
}

// ===== "Mở file" =====

/// Editors that open a file at a line from a URL: the app's folder name,
/// the URL scheme, and the name the feedback says.
const EDITORS: [(&str, &str, &str); 3] = [
    ("Visual Studio Code.app", "vscode", "Visual Studio Code"),
    ("Cursor.app", "cursor", "Cursor"),
    ("Zed.app", "zed", "Zed"),
];

/// The folders apps are installed in: `/Applications` and `~/Applications`.
pub fn app_folders(home: &Path) -> Vec<PathBuf> {
    vec![PathBuf::from("/Applications"), home.join("Applications")]
}

/// The words for `/usr/bin/open` that open `path` at `line`, and the editor
/// that does it: the first of [`EDITORS`] installed in `apps`, by its URL;
/// with none, the default text editor (`open -t`), which opens the file
/// without going to the line.
pub fn open_args(
    path: &Path,
    line: Option<u32>,
    apps: &[PathBuf],
) -> (Vec<String>, Option<&'static str>) {
    let editor = EDITORS.iter().find(|(app, _, _)| apps.iter().any(|d| d.join(app).is_dir()));
    if let (Some((_, scheme, name)), Some(line)) = (editor, line) {
        if let Ok(mut url) = url::Url::parse(&format!("{scheme}://file")) {
            url.set_path(&format!("{}:{line}", path.display()));
            return (vec![url.to_string()], Some(*name));
        }
    }
    (vec!["-t".into(), path.display().to_string()], None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_counts_only_at_the_start_of_a_line() {
        let n = b"ANTHROPIC_API_KEY".as_slice();
        assert!(assigns(b"ANTHROPIC_API_KEY=x", n));
        assert!(assigns(b"export ANTHROPIC_API_KEY=\"x\"", n));
        assert!(assigns(b"  export\tANTHROPIC_API_KEY=", n));
        assert!(!assigns(b"# export ANTHROPIC_API_KEY=x", n));
        assert!(!assigns(b"unset ANTHROPIC_API_KEY", n));
        assert!(!assigns(b"export ANTHROPIC_API_KEY_OLD=x", n));
        assert!(!assigns(b"MY_ANTHROPIC_API_KEY=x", n));
        assert!(!assigns(b"exportANTHROPIC_API_KEY=x", n));
        assert!(!assigns(b"echo ANTHROPIC_API_KEY=x", n));
        assert!(!assigns(b"export ANTHROPIC_API_KEY", n));
    }

    #[test]
    fn a_powershell_profile_sets_a_name_with_env() {
        let n = b"ANTHROPIC_API_KEY".as_slice();
        assert!(assigns_env(b"$env:ANTHROPIC_API_KEY = \"x\"", n));
        assert!(assigns_env(b"  $Env:anthropic_api_key='x'", n));
        assert!(!assigns_env(b"# $env:ANTHROPIC_API_KEY = 'x'", n));
        assert!(!assigns_env(b"$env:ANTHROPIC_API_KEY_OLD = 'x'", n));
        assert!(!assigns_env(b"if ($env:ANTHROPIC_API_KEY -eq 'x') {}", n));
        assert!(!assigns_env(b"$env:ANTHROPIC_API_KEY == 'x'", n));
        assert!(!assigns_env(b"ANTHROPIC_API_KEY=x", n));
    }

    #[test]
    fn windows_places_are_git_bash_powershell_and_the_registry() {
        let home = std::env::temp_dir().join(format!("qs-env-win-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("Documents/PowerShell")).unwrap();
        std::fs::write(home.join(".bashrc"), "export OPENAI_API_KEY=sk-made-up\n").unwrap();
        std::fs::write(
            home.join(Place::Pwsh.file().unwrap()),
            "Set-Alias g git\n$env:CODEX_HOME = 'D:\\codex'\n",
        )
        .unwrap();
        let asked = std::sync::Mutex::new(Vec::new());
        let set = |place: Place, name: &str| {
            asked.lock().unwrap().push((place, name.to_string()));
            match (place, name) {
                (Place::UserEnv, "ANTHROPIC_API_KEY") => Ok(true),
                (Place::MachineEnv, _) => Err(()),
                _ => Ok(false),
            }
        };
        let c = check_with(&home, &Place::WINDOWS, &set);
        let got: Vec<(&str, Place, Option<u32>)> =
            c.found.iter().map(|f| (f.name.as_str(), f.place, f.line)).collect();
        assert_eq!(
            got,
            [
                ("ANTHROPIC_API_KEY", Place::UserEnv, None),
                ("OPENAI_API_KEY", Place::Bashrc, Some(1)),
                ("CODEX_HOME", Place::Pwsh, Some(2)),
            ]
        );
        assert_eq!(c.unread, [Place::MachineEnv]);
        assert_eq!(c.found[0].effect.split(',').next(), Some("Trong các app mở từ Start"));
        assert_eq!(c.found[2].effect.split(',').next(), Some("Trong PowerShell"));
        assert_eq!(c.found[0].at(), "biến người dùng");
        assert_eq!(c.found[2].at(), "$PROFILE (PowerShell 7) dòng 2");
        assert_eq!(c.found[1].at(), "~/.bashrc dòng 1");
        // Each name asked once of each registry place, and of nothing else.
        let asked = asked.into_inner().unwrap();
        assert_eq!(asked.len(), 2 * VARS.len());
        assert!(asked.iter().all(|(p, _)| matches!(p, Place::UserEnv | Place::MachineEnv)));
        for p in Place::WINDOWS {
            assert_eq!(Place::from_word(p.word()), Some(p));
            // The window's `EnvPlace` is the same word.
            assert_eq!(serde_json::to_value(p).unwrap(), p.word());
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn ids_name_a_variable_and_a_place() {
        assert_eq!(id_of("CODEX_HOME", Place::BashProfile), "CODEX_HOME@bashProfile");
        let (v, p) = parse_id("CODEX_HOME@bashProfile").unwrap();
        assert_eq!((v.name, p), ("CODEX_HOME", Place::BashProfile));
        assert!(parse_id("PATH@zshrc").is_none());
        assert!(parse_id("CODEX_HOME@fishrc").is_none());
        assert!(parse_id("CODEX_HOME").is_none());
    }
}
