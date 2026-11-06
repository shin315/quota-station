// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Tải và cài" for a release build: Tauri's updater plugin
//! (`tauri-plugin-updater`), registered only when this copy may update
//! itself ([`Way::Install`]), so no other copy can download a release, let
//! alone install one over a build from source. The windows drive it through
//! the plugin's own commands (`@tauri-apps/plugin-updater`): it reads
//! `latest.json` from the latest release, downloads this system's updater
//! file, checks its signature against [`PUBKEY`] and installs it over the
//! running copy. On macOS the app then offers [`release_relaunch`]; on
//! Windows the installer (NSIS, `installMode` passive) closes the app and
//! opens the new one itself.

#[cfg(not(windows))]
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use quota_station_core::self_update::{self, Way};
use tauri::AppHandle;

/// The update key's public half (`src-tauri/updater.pub`). A copy built
/// without a key there opens the download instead of installing.
pub const PUBKEY: &str = include_str!("../updater.pub");

/// The `.app` this copy runs from (macOS); `None` outside one.
#[cfg(not(windows))]
fn bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    bundle_of(&exe)
}

/// The `.app` that holds `exe` (`…/Quota Station.app/Contents/MacOS/…`).
#[cfg(not(windows))]
fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let is_app = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension().is_some_and(|e| e == "app");
    is_app.then(|| app.to_path_buf())
}

/// How this copy takes a newer release, worked out once.
pub fn way() -> Way {
    static WAY: OnceLock<Way> = OnceLock::new();
    *WAY.get_or_init(|| {
        let release = crate::install::release_build();
        let key = self_update::key_ready(PUBKEY);
        #[cfg(not(windows))]
        {
            let bundle = bundle();
            let caskrooms = self_update::CASKROOMS.map(Path::new);
            let brew = self_update::homebrew(bundle.as_deref(), &caskrooms);
            self_update::way(release, brew, key, bundle.is_some())
        }
        // The installer puts the app in place on Windows; Homebrew has no cask there.
        #[cfg(windows)]
        self_update::way(release, false, key, true)
    })
}

/// The plugin, with the public key from [`PUBKEY`]; `None` unless this copy
/// installs releases itself.
pub fn plugin() -> Option<tauri::plugin::TauriPlugin<tauri::Wry, tauri_plugin_updater::Config>> {
    (way() == Way::Install)
        .then(|| tauri_plugin_updater::Builder::new().pubkey(PUBKEY.trim()).build())
}

/// "Mở lại" once the new copy is installed (macOS only): a small shell waits
/// for this process to end, then starts the new copy's executable from the
/// same bundle with this copy's environment. Rejects with `notInstall` on a
/// copy that does not install releases itself, `unbundled` outside a `.app`,
/// and `macosOnly` on Windows, whose installer reopens the app itself.
#[tauri::command]
pub fn release_relaunch(app: AppHandle) -> Result<(), String> {
    if way() != Way::Install {
        return Err("notInstall".into());
    }
    relaunch(app)
}

#[cfg(not(windows))]
fn relaunch(app: AppHandle) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let bundle = bundle().ok_or("unbundled")?;
    let exe = new_executable(&bundle).ok_or("unbundled")?;
    let wait = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec "$2""#;
    Command::new("/bin/sh")
        .args(["-c", wait, "sh"])
        .arg(std::process::id().to_string())
        .arg(&exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[cfg(windows)]
fn relaunch(_app: AppHandle) -> Result<(), String> {
    Err("macosOnly".into())
}

/// The executable the bundle's `Info.plist` names now: the update may have
/// renamed it.
#[cfg(not(windows))]
fn new_executable(bundle: &Path) -> Option<PathBuf> {
    let plist = std::fs::read_to_string(bundle.join("Contents/Info.plist")).ok()?;
    let name = quota_station_core::update::plist_string(&plist, "CFBundleExecutable")?;
    let exe = bundle.join("Contents/MacOS").join(name);
    exe.is_file().then_some(exe)
}

#[cfg(test)]
mod tests {
    #[cfg(not(windows))]
    #[test]
    fn the_bundle_is_the_app_that_holds_the_executable() {
        use std::path::{Path, PathBuf};

        use super::bundle_of;
        assert_eq!(
            bundle_of(Path::new("/Applications/Quota Station.app/Contents/MacOS/quota-station")),
            Some(PathBuf::from("/Applications/Quota Station.app"))
        );
        assert_eq!(bundle_of(Path::new("/srv/qs/target/release/quota-station")), None);
        assert_eq!(bundle_of(Path::new("/x/Contents/MacOS/quota-station")), None);
    }

    /// The committed key is a minisign public key: a broken file would
    /// quietly turn self-update off in every release.
    #[test]
    fn the_committed_key_is_a_public_key() {
        assert!(
            quota_station_core::self_update::key_ready(super::PUBKEY),
            "src-tauri/updater.pub holds no minisign public key"
        );
    }
}
