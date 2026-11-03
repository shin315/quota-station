// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's keys of the settings file, which the app keeps in
//! [`Settings::ext`] without knowing them: the other Macs the rotation file
//! is copied to (`peers`), the clone "Cập nhật từ mã nguồn" builds from
//! (`sourceDir`), and the switch of Cài đặt › Nâng cao (`advanced`).
//! [`Own`] is their typed view.
//!
//! Nâng cao (`advanced`) holds the advanced part: the rotation, the pause
//! and the switcher, the other Macs and the copy to them, and the update
//! from source. A new install starts with it off; a Mac that already had
//! the app keeps it on, so nothing there changes ([`SettingsHooks`],
//! through [`Settings::open`]). An app without the advanced part's
//! extension keeps these keys as they were in the file it writes.
//!
//! With it they are written as they always were: the list of other Macs
//! only when there is one, the path only when set, and the switch always. A change of these settings leaves every other key of the
//! file as it was; reading the file ([`SettingsHooks`]'s tidy) leaves
//! behind every key neither the app nor this module knows.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::alerts;
use crate::alerts::{ExtKind, Switches};
use crate::ext::Hooks;
use crate::settings::{machine_name, Settings};

/// Another of the owner's Macs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    /// What the app calls it: `Mac B`.
    pub name: String,
    /// The SSH alias or host name: `mac-b`.
    pub alias: String,
}

/// The advanced part's settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Own {
    /// The other Macs, each reached by an SSH alias.
    pub peers: Vec<Peer>,
    /// The clone of this repository the app updates itself from; `None` on
    /// a Mac that installs a copy built elsewhere.
    pub source_dir: Option<PathBuf>,
    /// Cài đặt › Nâng cao: the advanced part's tabs and switches show.
    /// Always written, so a new install stays new.
    pub advanced: bool,
}

impl Own {
    /// The advanced part's settings in `settings`; a switch it does not
    /// hold is off.
    pub fn of(settings: &Settings) -> Own {
        Own::read(&settings.ext, false)
    }

    /// The keys as a file of any version holds them. A switch missing reads
    /// as `missing`. The plan's single `sshAlias` for the one other Mac
    /// became the list of [`Peer`]s. The list is read loosely: an entry that
    /// is not a Mac with a usable alias is dropped, not the file, and the
    /// rest is tidied as typed ones are, one entry at a time.
    fn read(ext: &Map<String, Value>, missing: bool) -> Own {
        let text = |v: &Value, key: &str| v.get(key)?.as_str().map(String::from);
        let mut found: Vec<Peer> = ext
            .get("peers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| {
                Some(Peer { name: text(v, "name").unwrap_or_default(), alias: text(v, "alias")? })
            })
            .collect();
        if found.is_empty() {
            let alias = ext.get("sshAlias").and_then(Value::as_str).map(String::from);
            found.extend(alias.map(|alias| Peer { name: String::new(), alias }));
        }
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
        let path = |key: &str| ext.get(key).and_then(Value::as_str).map(PathBuf::from);
        let switch = |key: &str| ext.get(key).and_then(Value::as_bool).unwrap_or(missing);
        Own { peers, source_dir: path("sourceDir"), advanced: switch("advanced") }
    }

    /// Writes these settings into `settings`, in place of the keys of
    /// theirs there; every other key stays as it was.
    pub fn write(&self, settings: &mut Settings) {
        self.put(&mut settings.ext);
    }

    fn put(&self, ext: &mut Map<String, Value>) {
        for key in KEYS {
            ext.remove(key);
        }
        if !self.peers.is_empty() {
            ext.insert("peers".into(), serde_json::json!(self.peers));
        }
        if let Some(v) = self.source_dir.as_ref().and_then(|p| serde_json::to_value(p).ok()) {
            ext.insert("sourceDir".into(), v);
        }
        ext.insert("advanced".into(), Value::Bool(self.advanced));
    }

    /// Changes the advanced part's settings in `settings`.
    pub fn change(settings: &mut Settings, f: impl FnOnce(&mut Own)) {
        let mut own = Own::of(settings);
        f(&mut own);
        own.write(settings);
    }

    /// Nâng cao is on.
    pub fn advanced_shown(&self) -> bool {
        self.advanced
    }
}

/// The keys [`Own`] reads, `sshAlias` (the plan's single other Mac) among
/// them: a write puts its own in their place.
const KEYS: [&str; 4] = ["peers", "sshAlias", "sourceDir", "advanced"];

/// The settings file as a build with the advanced part reads it, for a
/// reader that does not open it ([`Settings::open`]): a missing or
/// unreadable file gives the defaults, Nâng cao off.
pub fn load(file: &Path) -> Settings {
    let read =
        std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str::<Settings>(&t).ok());
    let found = read.is_some();
    let mut settings = read.unwrap_or_default();
    SettingsHooks.tidy_settings(&mut settings.ext, found);
    settings
}

/// The advanced part's settings hooks: the app's extension
/// gives the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct SettingsHooks;

impl Hooks for SettingsHooks {
    /// Nâng cao: a file without it was written before it, by a version
    /// that had no first run either.
    fn settled_key(&self) -> Option<&'static str> {
        Some("advanced")
    }

    /// The keys as they are written, and no other key: one neither the app
    /// nor this module knows is left behind. Missing in a file an older
    /// version wrote, the switch is on: that Mac had the app.
    fn tidy_settings(&self, ext: &mut Map<String, Value>, found: bool) {
        let own = Own::read(ext, found);
        ext.clear();
        own.put(ext);
    }

    /// Nâng cao starts on where the Mac had the app, off on a new one.
    fn settle_settings(&self, ext: &mut Map<String, Value>, had_app: bool) {
        let mut own = Own::read(ext, false);
        own.advanced = had_app;
        own.put(ext);
    }

    /// The switches, less the alert Nâng cao holds while it is off (2, the
    /// rotation's).
    fn alert_switches(&self, settings: &Settings, mut on: Switches) -> Switches {
        let kind = alerts::leaving();
        if !Own::of(settings).advanced_shown() && on.on(&kind, self) {
            on.ext.insert(kind.name().to_string(), Value::Bool(false));
        }
        on
    }

    /// Alert 2, whose switch is in the settings file's `alerts`.
    fn alert_kinds(&self) -> &'static [ExtKind] {
        &alerts::KINDS
    }
}

/// The most other Macs the settings take.
pub const PEERS_MAX: usize = 8;

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

/// The longest SSH alias kept.
pub const ALIAS_MAX: usize = 64;

/// An SSH alias as typed in settings: what `~/.ssh/config` or a host name
/// allows, and never something `ssh` would read as an option. Here rather
/// than in `peers`, so the list of other Macs stays in the settings on a
/// system that copies to none (Windows).
pub fn alias_ok(alias: &str) -> bool {
    let mut chars = alias.chars();
    alias.len() <= ALIAS_MAX
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'))
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
        if !alias_ok(alias) {
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
