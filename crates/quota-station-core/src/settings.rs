//! The app's own settings file, `settings.json` in its Application Support
//! folder: the quota-axi path, the name this Mac goes by, the other Macs
//! the rotation file is copied to, and whether emails show.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
    /// Show the emails of the Antigravity accounts logged in here. Off by
    /// default: accounts go by number.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub show_email: bool,
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
    show_email: bool,
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
            show_email: s.show_email,
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
