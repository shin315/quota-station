//! The app's own settings file, `settings.json` in its Application Support
//! folder: the quota-axi path, the name this Mac goes by, the other Macs
//! the rotation file is copied to, whether account names show, the light or
//! dark look, which alerts are on,
//! where Firstmate's state file is, the clone "Cập nhật từ mã nguồn"
//! builds from and Codex's folder.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::alerts::Switches;
use crate::peers;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "Stored")]
pub struct Settings {
    /// Set by the user; `None` means find quota-axi in the usual places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_axi_path: Option<PathBuf>,
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
/// one other Mac became the list of [`Peer`]s. The list is read loosely: an
/// entry that is not a Mac with a usable alias is dropped, not the file.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    #[serde(default)]
    quota_axi_path: Option<PathBuf>,
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
    #[serde(default)]
    alerts: Switches,
    #[serde(default)]
    firstmate_file: Option<PathBuf>,
    #[serde(default)]
    source_dir: Option<PathBuf>,
    #[serde(default)]
    codex_home: Option<PathBuf>,
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
            quota_axi_path: s.quota_axi_path,
            machine_name: s.machine_name,
            peers,
            hide_names: s.hide_names,
            theme: serde_json::from_value(s.theme).unwrap_or_default(),
            alerts: s.alerts,
            firstmate_file: s.firstmate_file,
            source_dir: s.source_dir,
            codex_home: s.codex_home,
        }
    }
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
