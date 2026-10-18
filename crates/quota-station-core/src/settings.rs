//! The app's own settings file, `settings.json` in its Application Support
//! folder. M1 keeps only the quota-axi path.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Set by the user; `None` means find quota-axi in the usual places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_axi_path: Option<PathBuf>,
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
