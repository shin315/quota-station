// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Ported from Cockpit Tools `src-tauri/src/modules/wakeup_gateway.rs`
// (`official_antigravity_root_for_version`, `official_antigravity_app_version`,
// the product.json and Info.plist lookups, `parse_version_from_plutil_output`)
// and `src-tauri/src/modules/quota.rs` (the user agents, `x-goog-api-client`,
// the platform name, the google-auth-library lookup and the defaults)
// (1.3.70, commit 244b957, CC BY-NC-SA 4.0); changes: macOS only; no
// environment overrides and no configured app path, so Antigravity is looked
// for where Cockpit looks with its default settings; the bundle is read
// through the folder guard's read-only `Bundle`; the version is worked out
// once per round of reads instead of once per request.

//! Who the quota calls say they are: Antigravity, at the version installed
//! on this Mac, exactly as Cockpit Tools says it. Nothing here names Quota
//! Station: SECURITY.md explains why the user agent must not change.

use std::path::Path;
use std::time::Duration;

use crate::app_dir::Bundle;
use crate::runner;

/// Where Cockpit looks for Antigravity when no app path is set.
pub const DEFAULT_ROOT: &str = "/Applications/Antigravity IDE.app";
/// Cockpit's version when no Antigravity is installed.
pub const DEFAULT_IDE_VERSION: &str = "1.20.5";
/// Cockpit's google-auth-library version when the bundle has none.
pub const DEFAULT_GOOGLE_API_NODEJS_CLIENT_VERSION: &str = "10.3.0";
/// Cockpit's `x-goog-api-client` for `loadCodeAssist`.
pub const X_GOOG_API_CLIENT: &str = "gl-node/22.21.1";

const PLUTIL: &str = "/usr/bin/plutil";

/// The Antigravity app on this Mac, where Cockpit looks for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntigravityApp(Bundle);

impl AntigravityApp {
    pub fn installed() -> Self {
        AntigravityApp(Bundle::new(DEFAULT_ROOT))
    }

    /// A fake bundle for the tests.
    #[cfg(feature = "test-support")]
    pub fn at(root: impl Into<std::path::PathBuf>) -> Self {
        AntigravityApp(Bundle::new(root))
    }

    /// The versions the calls carry, read from the bundle now.
    pub fn identity(&self) -> Identity {
        let root = self.0.present().then_some(&self.0);
        Identity {
            ide_version: version_for_cloud_code(root),
            google_client_version: google_api_nodejs_client_version(root),
        }
    }
}

/// The versions the user agents and the `loadCodeAssist` metadata carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub ide_version: String,
    pub google_client_version: String,
}

impl Identity {
    /// For `fetchAvailableModels` and `retrieveUserQuotaSummary`:
    /// `antigravity/<version> darwin/<arch>`.
    pub fn cloud_code_user_agent(&self) -> String {
        format!("antigravity/{} {}/{}", self.ide_version, os(), arch())
    }

    /// For `loadCodeAssist`: the same, then `google-api-nodejs-client/<version>`.
    pub fn load_code_assist_user_agent(&self) -> String {
        format!(
            "antigravity/{} {}/{} google-api-nodejs-client/{}",
            self.ide_version,
            os(),
            arch(),
            self.google_client_version
        )
    }
}

/// Cockpit's names for this system, as Antigravity writes them.
pub fn os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        "linux" => "linux",
        _ => "windows",
    }
}

pub fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => "amd64",
    }
}

/// The `platform` of the `loadCodeAssist` metadata.
pub fn platform() -> &'static str {
    match (os(), arch()) {
        ("darwin", "amd64") => "DARWIN_AMD64",
        ("darwin", "arm64") => "DARWIN_ARM64",
        ("linux", "amd64") => "LINUX_AMD64",
        ("linux", "arm64") => "LINUX_ARM64",
        ("windows", "amd64") => "WINDOWS_AMD64",
        _ => "PLATFORM_UNSPECIFIED",
    }
}

fn version_for_cloud_code(root: Option<&Bundle>) -> String {
    let version = root.map(app_version).unwrap_or_default();
    let trimmed = version.trim();
    if trimmed.is_empty() {
        return DEFAULT_IDE_VERSION.to_string();
    }
    trimmed.to_string()
}

/// `product.json`'s `ideVersion` or `version`, else Info.plist's
/// `CFBundleShortVersionString`, else nothing.
fn app_version(root: &Bundle) -> String {
    let candidates: [&[&str]; 3] = [
        &["Contents", "Resources", "app", "product.json"],
        &["resources", "app", "product.json"],
        &["app", "product.json"],
    ];
    // The first product.json that is there answers, or none does.
    if let Some(product) = candidates.into_iter().find(|parts| root.has(parts)) {
        if let Some(version) = read_json_string_field(root, product, "ideVersion")
            .or_else(|| read_json_string_field(root, product, "version"))
        {
            return version;
        }
    }

    let plist = root.path().join("Contents").join("Info.plist");
    if let Some(version) = plutil_version(&plist) {
        return version;
    }
    String::new()
}

fn plutil_version(plist: &Path) -> Option<String> {
    let args = ["-p".to_string(), plist.to_string_lossy().into_owned()];
    let out = runner::run(Path::new(PLUTIL), &args, Duration::from_secs(15)).ok()?;
    if out.code != Some(0) {
        return None;
    }
    parse_version_from_plutil_output(&String::from_utf8_lossy(&out.stdout))
}

fn read_json_string_field(root: &Bundle, parts: &[&str], key: &str) -> Option<String> {
    let content = root.read_to_string(parts).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn google_api_nodejs_client_version(root: Option<&Bundle>) -> String {
    if let Some(root) = root {
        let candidates: [&[&str]; 3] = [
            &[
                "Contents",
                "Resources",
                "app",
                "node_modules",
                "google-auth-library",
                "package.json",
            ],
            &["resources", "app", "node_modules", "google-auth-library", "package.json"],
            &["node_modules", "google-auth-library", "package.json"],
        ];
        for candidate in candidates {
            if let Some(version) = read_json_string_field(root, candidate, "version") {
                return version;
            }
        }
    }

    DEFAULT_GOOGLE_API_NODEJS_CLIENT_VERSION.to_string()
}

fn parse_version_from_plutil_output(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("\"CFBundleShortVersionString\"") {
            continue;
        }
        let Some(version) = line.split("=>").nth(1) else {
            continue;
        };
        let version = version.trim().trim_matches('"');
        if !version.is_empty() {
            return Some(version.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_short_version_is_read_from_plutils_output() {
        let text = "{\n  \"CFBundleName\" => \"Antigravity\"\n  \"CFBundleShortVersionString\" => \"2.24.0\"\n}\n";
        assert_eq!(parse_version_from_plutil_output(text), Some("2.24.0".into()));
        assert_eq!(parse_version_from_plutil_output("{\n}\n"), None);
        assert_eq!(
            parse_version_from_plutil_output("\"CFBundleShortVersionString\" => \"\""),
            None
        );
    }
}
