// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Web pages, settings pages, folders and files, handed to the program the
//! owner chose for them through `tauri-plugin-opener`, on macOS (which runs
//! `/usr/bin/open` as the app did before) and on Windows alike.

use std::path::Path;

#[cfg(feature = "antigravity-accounts")]
use quota_station_core::os::Opener;
use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;

/// A web page in the default browser, or a settings page by its scheme.
pub fn url<R: Runtime>(app: &AppHandle<R>, url: &str) -> Result<(), String> {
    let opened = app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string());
    let scheme = url.split(':').next().unwrap_or_default();
    noted("url", scheme, &opened);
    opened
}

/// A folder in Finder or File Explorer.
pub fn folder<R: Runtime>(app: &AppHandle<R>, dir: &Path) -> Result<(), String> {
    let opened =
        app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string());
    noted("folder", "", &opened);
    opened
}

/// The test build's record of what was opened (`crate::session`): the
/// kind, a URL's scheme or a file's extension, and whether it opened;
/// never the address or the path.
fn noted(kind: &str, what: &str, opened: &Result<(), String>) {
    crate::session::note(
        "open",
        serde_json::json!({ "kind": kind, "what": what, "ok": opened.is_ok() }),
    );
}

/// A text file in the default text editor. The plugin has no word for
/// that on macOS, so `open -t` stays; on Windows, Notepad, which every
/// Windows has (a `.md` file may have no app of its own).
pub fn text<R: Runtime>(app: &AppHandle<R>, file: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        std::process::Command::new("/usr/bin/open")
            .arg("-t")
            .arg(file)
            .status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let opened = app
            .opener()
            .open_path(file.to_string_lossy(), Some("notepad"))
            .map_err(|e| e.to_string());
        let ext = file.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        noted("text", &ext, &opened);
        opened
    }
}

/// The browser for the core's sign-ins, through the plugin.
#[cfg(feature = "antigravity-accounts")]
pub struct Browser<R: Runtime>(pub AppHandle<R>);

#[cfg(feature = "antigravity-accounts")]
impl<R: Runtime> Opener for Browser<R> {
    fn open(&self, target: &str) -> Result<(), String> {
        url(&self.0, target)
    }
}
