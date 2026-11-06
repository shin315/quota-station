// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! How this copy takes a newer release: "Tải và cài" installs it in place,
//! or the copy is pointed elsewhere.
//!
//! Only a release build (the release workflow's, `QS_RELEASE=1`) updates
//! itself, with Tauri's updater: it downloads the release's updater file,
//! checks its signature against the update key's public half built into the
//! app, installs it over the running copy and offers to reopen. A build from
//! source keeps "Cập nhật từ mã nguồn" and is never replaced by a download; a
//! copy Homebrew installed is updated by Homebrew, so the app shows the
//! command; a release build without the public key (the placeholder in
//! `src-tauri/updater.pub`) opens the download as before.

use std::path::Path;

use base64::Engine;
use serde::Serialize;

/// What the app shows a copy Homebrew installed.
pub const BREW_UPGRADE: &str = "brew upgrade --cask shin315/tap/quota-station";

/// Where Homebrew keeps the cask's record: Apple silicon's prefix, then
/// Intel's.
pub const CASKROOMS: [&str; 2] =
    ["/opt/homebrew/Caskroom/quota-station", "/usr/local/Caskroom/quota-station"];

/// How "Tải và cài" goes for this copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Way {
    /// Downloads, checks the signature, installs and offers to reopen.
    Install,
    /// Homebrew installed it: the app shows [`BREW_UPGRADE`].
    Brew,
    /// Opens the installer or the release page in the browser, as before.
    Download,
    /// A build from source: rebuilt from its clone, never replaced.
    Source,
}

/// The way for a copy that is a release build or not, Homebrew's or not,
/// built with a real public key or not, and running from an app bundle (on
/// macOS a `.app`; always on Windows) or not.
pub fn way(release_build: bool, homebrew: bool, key_ready: bool, bundled: bool) -> Way {
    if !release_build {
        Way::Source
    } else if homebrew {
        Way::Brew
    } else if key_ready && bundled {
        Way::Install
    } else {
        Way::Download
    }
}

/// Whether `text` is a minisign public key as `tauri signer generate` writes
/// it: base64 of an `untrusted comment:` line and the key's own base64 line,
/// `Ed`, its 8-byte id and its 32 bytes. The placeholder is not.
pub fn key_ready(text: &str) -> bool {
    key_id(text).is_some()
}

/// The key's 8-byte id, as a signature names the key that made it.
pub fn key_id(text: &str) -> Option<[u8; 8]> {
    let b64 = base64::engine::general_purpose::STANDARD;
    let outer = b64.decode(text.trim()).ok()?;
    let outer = String::from_utf8(outer).ok()?;
    let mut lines = outer.lines();
    lines.next()?.strip_prefix("untrusted comment:")?;
    let key = b64.decode(lines.next()?.trim()).ok()?;
    if key.len() != 42 || &key[..2] != b"Ed" {
        return None;
    }
    key[2..10].try_into().ok()
}

/// Whether Homebrew installed the app `bundle` (the `.app` this copy runs
/// from): the cask is recorded in one of `caskrooms` and the bundle sits in
/// an `Applications` folder, where the cask puts it. A copy run from
/// anywhere else is not Homebrew's, even with the cask installed.
pub fn homebrew(bundle: Option<&Path>, caskrooms: &[&Path]) -> bool {
    let Some(bundle) = bundle else { return false };
    let in_applications =
        bundle.parent().and_then(Path::file_name).is_some_and(|name| name == "Applications");
    in_applications && caskrooms.iter().any(|dir| dir.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A public key `tauri signer generate` made for these tests only; its
    /// private half was thrown away.
    const KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEE2MDhFNjBBNzY0QTVEMTAKUldRUVhVcDJDdVlJcHNTQ2xtd1kxRktFVS9WZjgxQ2xjM3lLTzdKQVA3dmhrKzRFMkpqNG9ia3gK";

    #[test]
    fn only_a_minisign_public_key_counts() {
        assert!(key_ready(KEY));
        assert!(key_ready(&format!("\n{KEY}\n")));
        assert_eq!(key_id(KEY), Some([0x10, 0x5D, 0x4A, 0x76, 0x0A, 0xE6, 0x08, 0xA6]));
        assert!(!key_ready("placeholder: no update key yet"));
        assert!(!key_ready(""));
        let b64 = base64::engine::general_purpose::STANDARD;
        // Two lines, but the second is not a key.
        assert!(!key_ready(&b64.encode("untrusted comment: x\nnotakey\n")));
        // A key line without its comment line.
        assert!(!key_ready(
            &b64.encode("RWQQXUp2CuYIpsSClmwY1FKEU/Vf81Clc3yKO7JAP7vhk+4E2Jj4obkx\n")
        ));
    }

    #[test]
    fn only_a_release_build_installs_and_homebrew_comes_first() {
        assert_eq!(way(false, false, true, true), Way::Source);
        assert_eq!(
            way(false, true, true, true),
            Way::Source,
            "a build from source is never replaced"
        );
        assert_eq!(way(true, true, true, true), Way::Brew);
        assert_eq!(way(true, false, true, true), Way::Install);
        assert_eq!(way(true, false, false, true), Way::Download, "no public key yet");
        assert_eq!(way(true, false, true, false), Way::Download, "not in an app bundle");
    }

    #[test]
    fn homebrew_needs_the_cask_and_the_applications_folder() {
        let root = std::env::temp_dir().join(format!("qs-brew-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cask = root.join("Caskroom/quota-station");
        let app = Path::new("/Applications/Quota Station.app");
        let user_app = Path::new("/Users/me/Applications/Quota Station.app");
        let elsewhere = Path::new("/Users/me/Downloads/Quota Station.app");
        assert!(!homebrew(Some(app), &[&cask]), "no cask recorded");
        std::fs::create_dir_all(&cask).unwrap();
        assert!(homebrew(Some(app), &[Path::new("/nonexistent/Caskroom/x"), &cask]));
        assert!(homebrew(Some(user_app), &[&cask]));
        assert!(!homebrew(Some(elsewhere), &[&cask]));
        assert!(!homebrew(None, &[&cask]));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
