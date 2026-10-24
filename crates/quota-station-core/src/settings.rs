// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's own settings file, `settings.json` in its Application Support
//! folder: the name this Mac goes by, the other Macs
//! the rotation file is copied to, whether account names show, the light or
//! dark look, which service the menu bar follows and which of its windows
//! the title shows, which alerts are on,
//! where Firstmate's state file is, the clone "Cập nhật từ mã nguồn"
//! builds from, Codex's folder and whether Codex keeps the ChatGPT login
//! for plugins under a custom API, when the agents' backup packs are made,
//! where they go, how the app looks for newer versions of the skills, the
//! two thresholds of alert 1, and the switches of Cài đặt › Nâng cao.
//!
//! Nâng cao (`advanced`) holds what was made for the author's own Macs: the
//! rotation, the pause and the switcher, the other Macs and the copy to them,
//! the update from source, and Firstmate (its own switch inside). A new
//! install starts with it off; a Mac that already had the app keeps it on, so
//! nothing there changes ([`Settings::open`]).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::alerts::Switches;
use crate::peers;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "Stored")]
pub struct Settings {
    /// Set by the user; `None` means the name macOS gives this Mac.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_name: Option<String>,
    /// The other Macs, each reached by an SSH alias.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peers: Vec<Peer>,
    /// Account names (emails, organizations) are hidden, for screenshots:
    /// accounts go by their place (Team A, Ultra 1). Off by default. It
    /// replaces `showEmail`, which only Antigravity used and which was off
    /// by default, so an older file shows names as a new one does.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_names: bool,
    /// Light, dark, or as macOS is.
    #[serde(default, skip_serializing_if = "Theme::is_auto")]
    pub theme: Theme,
    /// The service whose account and percent used the menu bar shows.
    /// Each Mac keeps its own.
    #[serde(default, skip_serializing_if = "Menubar::is_claude")]
    pub menubar: Menubar,
    /// The window whose share used the menu bar title shows. Each Mac keeps
    /// its own.
    #[serde(default, skip_serializing_if = "MenubarWindow::is_five_hours")]
    pub menubar_window: MenubarWindow,
    /// Which of the five alerts are on.
    #[serde(default, skip_serializing_if = "Switches::is_default")]
    pub alerts: Switches,
    /// Firstmate's `quota-pool.last`, set by the user; `None` on a Mac
    /// without Firstmate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firstmate_file: Option<PathBuf>,
    /// The clone of this repository the app updates itself from; `None` on
    /// a Mac that installs a copy built elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<PathBuf>,
    /// Codex's folder, set by the user; `None` is `~/.codex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_home: Option<PathBuf>,
    /// How Codex runs on the custom provider.
    #[serde(default, skip_serializing_if = "CodexOptions::is_default")]
    pub codex: CodexOptions,
    /// When a backup pack of the agents is made on its own (M10).
    #[serde(default, skip_serializing_if = "AutoBackup::is_default")]
    pub auto_backup: AutoBackup,
    /// Looking for newer versions of the skills (M11).
    #[serde(default, skip_serializing_if = "SkillChecks::is_default")]
    pub skill_checks: SkillChecks,
    /// Where the backup packs go; `None` is iCloud Drive's
    /// `Quota Station Agents`, as before there was a choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_dir: Option<PathBuf>,
    /// Alert 1's two thresholds.
    #[serde(default, skip_serializing_if = "PoolAlert::is_default")]
    pub pool_alert: PoolAlert,
    /// Cài đặt › Nâng cao: the tabs and switches made for the author's own
    /// Macs show. Always written, so a new install stays new.
    pub advanced: bool,
    /// The Firstmate tab and alert 5, inside Nâng cao. Always written.
    pub firstmate: bool,
}

/// Alert 1: the account in use has `dry_at_most` percent left or less, and
/// no other account has `room_at_least` percent or more. 10 and 15 by
/// default, Firstmate's own rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolAlert {
    #[serde(default = "dry_at_most")]
    pub dry_at_most: u8,
    #[serde(default = "room_at_least")]
    pub room_at_least: u8,
}

fn dry_at_most() -> u8 {
    10
}

fn room_at_least() -> u8 {
    15
}

impl Default for PoolAlert {
    fn default() -> Self {
        PoolAlert { dry_at_most: dry_at_most(), room_at_least: room_at_least() }
    }
}

impl PoolAlert {
    pub fn is_default(&self) -> bool {
        *self == PoolAlert::default()
    }

    /// Both percents, 1 to 99; the room asked of another account is more
    /// than what counts as dry.
    pub fn check(&self) -> Result<(), &'static str> {
        let ok = |v: u8| (1..=99).contains(&v);
        if !ok(self.dry_at_most) || !ok(self.room_at_least) {
            return Err("range");
        }
        if self.room_at_least <= self.dry_at_most {
            return Err("order");
        }
        Ok(())
    }
}

/// "Dùng plugin khi chạy Custom API (giữ đăng nhập ChatGPT)": on by
/// default, as the switch did before there was a choice. Off, a switch to
/// the custom provider writes `requires_openai_auth = false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexOptions {
    #[serde(default = "yes")]
    pub plugins: bool,
}

impl Default for CodexOptions {
    fn default() -> Self {
        CodexOptions { plugins: true }
    }
}

impl CodexOptions {
    pub fn is_default(&self) -> bool {
        *self == CodexOptions::default()
    }
}

/// "Kiểm bản mới" (once a day, or never) and "Báo bằng thông báo macOS"
/// (off by default: the alert shows in the app only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillChecks {
    #[serde(default = "yes")]
    pub daily: bool,
    #[serde(default)]
    pub notify: bool,
}

impl Default for SkillChecks {
    fn default() -> Self {
        SkillChecks { daily: true, notify: false }
    }
}

impl SkillChecks {
    pub fn is_default(&self) -> bool {
        *self == SkillChecks::default()
    }
}

/// "Tự sao lưu": after each change the app writes to an agent, and once a
/// day when an agent's list changed outside the app. Both on by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackup {
    #[serde(default = "yes")]
    pub after_change: bool,
    #[serde(default = "yes")]
    pub daily: bool,
}

fn yes() -> bool {
    true
}

impl Default for AutoBackup {
    fn default() -> Self {
        AutoBackup { after_change: true, daily: true }
    }
}

impl AutoBackup {
    pub fn is_default(&self) -> bool {
        *self == AutoBackup::default()
    }
}

/// How the windows look.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// As macOS is.
    #[default]
    Auto,
    Light,
    Dark,
}

impl Theme {
    pub fn is_auto(&self) -> bool {
        *self == Theme::Auto
    }
}

/// The service the menu bar title follows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Menubar {
    #[default]
    Claude,
    /// Antigravity's Gemini models.
    Gemini,
    Codex,
}

impl Menubar {
    pub fn is_claude(&self) -> bool {
        *self == Menubar::Claude
    }
}

/// The window the menu bar title shows. A service without that window shows
/// its five-hour window instead, and the tightest one without either.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MenubarWindow {
    #[default]
    FiveHours,
    Week,
    /// Claude's Fable weekly window.
    FableWeek,
    /// The window used the most, as the title showed before there was a
    /// choice.
    Tightest,
}

impl MenubarWindow {
    pub fn is_five_hours(&self) -> bool {
        *self == MenubarWindow::FiveHours
    }
}

/// Another of the owner's Macs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    /// What the app calls it: `Mac B`.
    pub name: String,
    /// The SSH alias or host name: `mac-b`.
    pub alias: String,
}

/// The file as written by any version: the plan's single `sshAlias` for the
/// one other Mac became the list of [`Peer`]s, and the quota-axi path of
/// versions that ran it (`quotaAxiPath`) is left behind. The list is read loosely: an
/// entry that is not a Mac with a usable alias is dropped, not the file.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    #[serde(default)]
    machine_name: Option<String>,
    #[serde(default)]
    peers: Vec<serde_json::Value>,
    #[serde(default)]
    ssh_alias: Option<String>,
    #[serde(default)]
    hide_names: bool,
    /// An unknown value reads as automatic.
    #[serde(default)]
    theme: serde_json::Value,
    /// An unknown value reads as Claude.
    #[serde(default)]
    menubar: serde_json::Value,
    /// An unknown value reads as five hours.
    #[serde(default)]
    menubar_window: serde_json::Value,
    #[serde(default)]
    alerts: Switches,
    #[serde(default)]
    firstmate_file: Option<PathBuf>,
    #[serde(default)]
    source_dir: Option<PathBuf>,
    #[serde(default)]
    codex_home: Option<PathBuf>,
    /// An unknown value reads as plugins on.
    #[serde(default)]
    codex: serde_json::Value,
    /// An unknown value reads as both on.
    #[serde(default)]
    auto_backup: serde_json::Value,
    /// An unknown value reads as daily, no macOS card.
    #[serde(default)]
    skill_checks: serde_json::Value,
    #[serde(default)]
    backup_dir: Option<PathBuf>,
    /// An unknown or refused value reads as 10 and 15.
    #[serde(default)]
    pool_alert: serde_json::Value,
    /// Missing in a file an older version wrote: that Mac had the app, so on.
    #[serde(default)]
    advanced: Option<bool>,
    #[serde(default)]
    firstmate: Option<bool>,
}

impl From<Stored> for Settings {
    fn from(s: Stored) -> Settings {
        let text = |v: &serde_json::Value, key: &str| v.get(key)?.as_str().map(String::from);
        let mut found: Vec<Peer> = s
            .peers
            .iter()
            .filter_map(|v| {
                Some(Peer { name: text(v, "name").unwrap_or_default(), alias: text(v, "alias")? })
            })
            .collect();
        if found.is_empty() {
            found.extend(s.ssh_alias.map(|alias| Peer { name: String::new(), alias }));
        }
        // The same tidying as settings typed in the app, one entry at a time.
        let mut peers: Vec<Peer> = Vec::new();
        for p in found {
            if peers.len() == PEERS_MAX {
                break;
            }
            if let Ok(one) = self::peers(&[p]) {
                if !peers.iter().any(|q| q.alias.eq_ignore_ascii_case(&one[0].alias)) {
                    peers.extend(one);
                }
            }
        }
        Settings {
            machine_name: s.machine_name,
            peers,
            hide_names: s.hide_names,
            theme: serde_json::from_value(s.theme).unwrap_or_default(),
            menubar: serde_json::from_value(s.menubar).unwrap_or_default(),
            menubar_window: serde_json::from_value(s.menubar_window).unwrap_or_default(),
            alerts: s.alerts,
            firstmate_file: s.firstmate_file,
            source_dir: s.source_dir,
            codex_home: s.codex_home,
            codex: serde_json::from_value(s.codex).unwrap_or_default(),
            auto_backup: serde_json::from_value(s.auto_backup).unwrap_or_default(),
            skill_checks: serde_json::from_value(s.skill_checks).unwrap_or_default(),
            backup_dir: s.backup_dir,
            pool_alert: serde_json::from_value::<PoolAlert>(s.pool_alert)
                .ok()
                .filter(|p| p.check().is_ok())
                .unwrap_or_default(),
            advanced: s.advanced.unwrap_or(true),
            firstmate: s.firstmate.unwrap_or(true),
        }
    }
}

/// Anything but hidden files in `dir`, `file` aside.
fn has_files(dir: &Path, file: &Path) -> bool {
    std::fs::read_dir(dir).into_iter().flatten().flatten().any(|e| {
        let name = e.file_name();
        e.path() != file && !name.to_string_lossy().starts_with('.')
    })
}

/// The longest machine name kept, in characters: it shares a line with the
/// freshness text at the top of the popover.
pub const MACHINE_NAME_MAX: usize = 40;
/// The most other Macs the settings take.
pub const PEERS_MAX: usize = 8;

/// Tidies a machine name typed by the user: trimmed, cut to
/// [`MACHINE_NAME_MAX`] characters, and `None` when nothing is left.
pub fn machine_name(input: &str) -> Option<String> {
    let name: String = input.trim().chars().take(MACHINE_NAME_MAX).collect();
    let name = name.trim_end();
    (!name.is_empty()).then(|| name.to_string())
}

/// Why a list of other Macs typed in settings is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeersError {
    TooMany,
    /// The alias at this index is empty or not one `ssh` takes safely.
    BadAlias(usize),
    /// The alias at this index is already in the list.
    SameAlias(usize),
}

impl PeersError {
    pub fn code(&self) -> String {
        match self {
            PeersError::TooMany => "tooMany".into(),
            PeersError::BadAlias(i) => format!("badAlias:{i}"),
            PeersError::SameAlias(i) => format!("sameAlias:{i}"),
        }
    }
}

/// Tidies the other Macs typed in settings: names trimmed like
/// [`machine_name`] (an empty name takes the alias), aliases trimmed and
/// checked, no alias twice.
pub fn peers(input: &[Peer]) -> Result<Vec<Peer>, PeersError> {
    if input.len() > PEERS_MAX {
        return Err(PeersError::TooMany);
    }
    let mut out: Vec<Peer> = Vec::new();
    for (i, p) in input.iter().enumerate() {
        let alias = p.alias.trim();
        if !peers::alias_ok(alias) {
            return Err(PeersError::BadAlias(i));
        }
        if out.iter().any(|q| q.alias.eq_ignore_ascii_case(alias)) {
            return Err(PeersError::SameAlias(i));
        }
        let name = machine_name(&p.name).unwrap_or_else(|| alias.to_string());
        out.push(Peer { name, alias: alias.to_string() });
    }
    Ok(out)
}

impl Settings {
    /// The settings at start. The first time this version runs it settles
    /// whether the Mac is new to the app: one that already has this file, or
    /// anything else in the app's folder, had the app before and keeps it as
    /// it was, Nâng cao and Firstmate on; a new one starts with both off.
    /// The answer is written at once, so what the app puts in its folder
    /// afterwards never makes a new install look like an older one.
    pub fn open(file: &Path) -> Self {
        let json = std::fs::read_to_string(file)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
        if json.as_ref().is_some_and(|v| v.get("advanced").is_some_and(|a| a.is_boolean())) {
            return Settings::load(file);
        }
        let had_app = file.exists() || file.parent().is_some_and(|dir| has_files(dir, file));
        let mut settings = Settings::load(file);
        settings.advanced = had_app;
        settings.firstmate = had_app;
        // A file there that cannot be read is never written over. Unsaved,
        // the next start asks again and finds the same answer.
        if json.is_some() || !file.exists() {
            let _ = settings.save(file);
        }
        settings
    }

    /// Which alerts run: the switches, less the ones Nâng cao holds while
    /// it is off (2, the rotation's; 5, Firstmate's, also behind its own switch).
    pub fn alert_switches(&self) -> Switches {
        let mut on = self.alerts;
        on.leaving &= self.advanced;
        on.firstmate &= self.advanced && self.firstmate;
        on
    }

    /// The Firstmate tab and alert 5 show.
    pub fn firstmate_shown(&self) -> bool {
        self.advanced && self.firstmate
    }

    /// A missing or unreadable file gives the defaults: the app still runs.
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes through a temporary file and a rename, so a crash never
    /// leaves half a file.
    pub fn save(&self, file: &Path) -> io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, file)
    }
}
