//! The app's own settings file, `settings.json` in its Application Support
//! folder: the quota-axi path and the name this Mac goes by.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Set by the user; `None` means find quota-axi in the usual places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_axi_path: Option<PathBuf>,
    /// Set by the user; `None` means the name macOS gives this Mac.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_name: Option<String>,
}

/// The longest machine name kept, in characters: it shares a line with the
/// freshness text at the top of the popover.
pub const MACHINE_NAME_MAX: usize = 40;

/// Tidies a machine name typed by the user: trimmed, cut to
/// [`MACHINE_NAME_MAX`] characters, and `None` when nothing is left.
pub fn machine_name(input: &str) -> Option<String> {
    let name: String = input.trim().chars().take(MACHINE_NAME_MAX).collect();
    let name = name.trim_end();
    (!name.is_empty()).then(|| name.to_string())
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
