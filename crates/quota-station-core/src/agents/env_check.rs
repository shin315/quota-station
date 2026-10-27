// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Environment variables that override an agent's login (M14, I): an
//! `ANTHROPIC_API_KEY` in `~/.zshrc` makes Claude Code in Terminal use that
//! key instead of the claude-swap account in use, whatever the app switches.
//!
//! [`check`] looks for the eleven names in [`VARS`] in the five shell files
//! and launchd's environment ([`Place`]). It learns only whether a name is
//! set, and where: the file and its line, or launchd. It never learns a
//! value:
//!
//! - A shell file is never run or `source`d. Its bytes are read, each line
//!   is matched by the name at its start (`export NAME=` or `NAME=`), only
//!   the line number is kept, and the bytes are zeroed before they go.
//! - `launchctl getenv NAME` prints the value itself. Its output is only
//!   asked whether it is empty, then zeroed at once: never kept, logged,
//!   stored or shown.
//!
//! On Windows the places are Git Bash's three files, the two PowerShell
//! profiles (`$env:NAME = …`, matched without regard to case, as PowerShell
//! reads names) and the registry's environment, the user's
//! (`HKCU\Environment`) and the computer's, which every app started from
//! Start gets. For the registry only the value's size is asked for, never
//! its text ([`crate::win::env_set`]).
//!
//! The app never edits a shell file and never unsets anything. "Mở file"
//! opens the file in an editor at the line ([`open_args`]); "Bỏ qua" is
//! kept in the app's own folder ([`load_ignored`]).

use std::collections::BTreeSet;
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
        "dùng khoá này thay cho tài khoản claude-swap đang chọn.",
    ),
    var(
        "ANTHROPIC_AUTH_TOKEN",
        Agent::Claude,
        Weight::Bad,
        "gửi token này thay cho đăng nhập của tài khoản claude-swap đang chọn.",
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
        "dùng token này thay cho tài khoản claude-swap đang chọn.",
    ),
    var(
        "CLAUDE_CONFIG_DIR",
        Agent::Claude,
        Weight::Warn,
        "đọc đăng nhập ở thư mục này, không phải chỗ claude-swap chuyển tài khoản.",
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

/// Where a variable can be set: five shell files in the home folder, and
/// launchd's environment, which apps opened from the Dock get. On Windows:
/// Git Bash's files, the PowerShell profiles, and the registry's
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
    PowerShell,
    WindowsPowerShell,
    UserEnv,
    MachineEnv,
}

impl Place {
    /// The places on a Mac.
    pub const ALL: [Place; 6] = [
        Place::Zshenv,
        Place::Zprofile,
        Place::Zshrc,
        Place::BashProfile,
        Place::Profile,
        Place::Launchctl,
    ];

    /// The places on Windows.
    pub const WINDOWS: [Place; 7] = [
        Place::BashProfile,
        Place::Bashrc,
        Place::Profile,
        Place::PowerShell,
        Place::WindowsPowerShell,
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
            Place::PowerShell => Some("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"),
            Place::WindowsPowerShell => {
                Some("Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1")
            }
            Place::Launchctl | Place::UserEnv | Place::MachineEnv => None,
        }
    }

    /// `~/.zshrc`, `launchctl`, or the registry's environment.
    pub fn label(self) -> String {
        match self {
            Place::Launchctl => "launchctl".into(),
            Place::UserEnv => "biến môi trường của người dùng".into(),
            Place::MachineEnv => "biến môi trường của máy".into(),
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
            Place::PowerShell => "powerShell",
            Place::WindowsPowerShell => "windowsPowerShell",
            Place::UserEnv => "userEnv",
            Place::MachineEnv => "machineEnv",
        }
    }

    pub fn from_word(word: &str) -> Option<Place> {
        Place::ALL.into_iter().chain(Place::WINDOWS).find(|p| p.word() == word)
    }

    /// A PowerShell profile: `$env:NAME = …`.
    fn powershell(self) -> bool {
        matches!(self, Place::PowerShell | Place::WindowsPowerShell)
    }

    /// Where an app started by the owner gets it, before anything else.
    fn opened(self) -> &'static str {
        match self {
            Place::Launchctl => "Trong các app mở từ Dock",
            Place::UserEnv | Place::MachineEnv => "Trong các app mở từ Start",
            Place::PowerShell | Place::WindowsPowerShell => "Trong PowerShell",
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
    /// "Trong Terminal, Claude Code dùng khoá này thay cho …".
    pub effect: String,
}

impl Found {
    fn new(v: &Var, place: Place, line: Option<u32>) -> Found {
        let opened = place.opened();
        Found {
            id: id_of(v.name, place),
            name: v.name.into(),
            agent: v.agent,
            weight: v.weight,
            place,
            line,
            effect: format!("{opened}, {} {}", v.agent.label(), v.effect),
        }
    }

    /// `~/.zshrc dòng 42`, or `launchctl`.
    pub fn at(&self) -> String {
        match self.line {
            Some(n) => format!("{} dòng {n}", self.place.label()),
            None => self.place.label(),
        }
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
        match place.file() {
            Some(file) => match read_lines(&home.join(file), place.powershell()) {
                Ok(Some(lines)) => {
                    for v in &VARS {
                        if let Some(&n) = lines.get(v.name) {
                            found.push(Found::new(v, place, Some(n)));
                        }
                    }
                }
                Ok(None) => {}
                Err(()) => out.unread.push(place),
            },
            None => {
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

/// For each of [`VARS`] the file sets, the last line that does (in
/// PowerShell's words when `powershell`). `None` when there is no file; an
/// error when it is there but cannot be read.
fn read_lines(
    path: &Path,
    powershell: bool,
) -> Result<Option<std::collections::BTreeMap<&'static str, u32>>, ()> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(()),
    };
    let meta = file.metadata().map_err(|_| ())?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    let read = file.by_ref().take(MAX_BYTES + 1).read_to_end(&mut bytes);
    let lines = match read {
        Ok(_) if bytes.len() as u64 <= MAX_BYTES => {
            Ok(Some(if powershell { scan_powershell(&bytes) } else { scan(&bytes) }))
        }
        _ => Err(()),
    };
    // The values are in these bytes: they go now, zeroed first.
    bytes.fill(0);
    lines
}

/// The last line (from 1) that sets each name, from a file's bytes.
pub fn scan(bytes: &[u8]) -> std::collections::BTreeMap<&'static str, u32> {
    let mut at = std::collections::BTreeMap::new();
    for (i, line) in bytes.split(|b| *b == b'\n').enumerate() {
        for v in &VARS {
            if assigns(line, v.name.as_bytes()) {
                at.insert(v.name, i as u32 + 1);
            }
        }
    }
    at
}

/// [`scan`] of a PowerShell profile.
pub fn scan_powershell(bytes: &[u8]) -> std::collections::BTreeMap<&'static str, u32> {
    let mut at = std::collections::BTreeMap::new();
    for (i, line) in bytes.split(|b| *b == b'\n').enumerate() {
        for v in &VARS {
            if assigns_env(line, v.name.as_bytes()) {
                at.insert(v.name, i as u32 + 1);
            }
        }
    }
    at
}

/// `$env:NAME = …` at the start of the line, after spaces, in any case. A
/// comment, `$env:NAME -eq …` or `$env:OTHER_NAME =` is not.
fn assigns_env(line: &[u8], name: &[u8]) -> bool {
    let blank = |b: &u8| *b == b' ' || *b == b'\t';
    let rest = &line[line.iter().take_while(|b| blank(b)).count()..];
    let Some(after) =
        rest.get(..5).filter(|p| p.eq_ignore_ascii_case(b"$env:")).map(|_| &rest[5..])
    else {
        return false;
    };
    let Some(after) = after
        .get(..name.len())
        .filter(|n| n.eq_ignore_ascii_case(name))
        .map(|_| &after[name.len()..])
    else {
        return false;
    };
    let after = &after[after.iter().take_while(|b| blank(b)).count()..];
    after.first() == Some(&b'=') && after.get(1) != Some(&b'=')
}

/// `NAME=…` or `export NAME=…` at the start of the line, after spaces. A
/// comment, `unset NAME` or `OTHER_NAME=` is not.
fn assigns(line: &[u8], name: &[u8]) -> bool {
    let blank = |b: &u8| *b == b' ' || *b == b'\t';
    let skip = |s: &[u8]| -> usize { s.iter().take_while(|b| blank(b)).count() };
    let mut rest = &line[skip(line)..];
    if let Some(r) = rest.strip_prefix(b"export") {
        if r.first().is_some_and(blank) {
            rest = &r[skip(r)..];
        }
    }
    rest.strip_prefix(name).is_some_and(|r| r.first() == Some(&b'='))
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
            home.join(Place::PowerShell.file().unwrap()),
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
                ("CODEX_HOME", Place::PowerShell, Some(2)),
            ]
        );
        assert_eq!(c.unread, [Place::MachineEnv]);
        assert_eq!(c.found[0].effect.split(',').next(), Some("Trong các app mở từ Start"));
        assert_eq!(c.found[2].effect.split(',').next(), Some("Trong PowerShell"));
        assert_eq!(c.found[0].at(), "biến môi trường của người dùng");
        assert_eq!(c.found[1].at(), "~/.bashrc dòng 1");
        // Each name asked once of each registry place, and of nothing else.
        let asked = asked.into_inner().unwrap();
        assert_eq!(asked.len(), 2 * VARS.len());
        assert!(asked.iter().all(|(p, _)| matches!(p, Place::UserEnv | Place::MachineEnv)));
        for p in Place::WINDOWS {
            assert_eq!(Place::from_word(p.word()), Some(p));
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
