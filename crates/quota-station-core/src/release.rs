// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A newer Quota Station on GitHub Releases (the community edition's B1).
//!
//! Once a day, while "Tự kiểm tra bản mới của app" is on, or when the owner
//! presses "Kiểm tra bản mới", the app asks GitHub one thing: this repository's
//! latest release (`GET /repos/shin315/quota-station/releases/latest`). No
//! token and nothing about this Mac is sent. The app never downloads or
//! installs anything: "Tải và cài" opens the release's disk image (on
//! Windows its `-setup.exe` for this processor) or its page in the browser,
//! and the owner installs it as the first time.
//!
//! "Để sau" quiets one version's banner and popover line; "Bỏ qua bản này"
//! also leaves it out of Cài đặt until a later version comes. Both are kept
//! by version, so the next release is told again.

use std::io::{self, Read};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::app_dir::AppDir;

/// In the app's folder.
pub const FILE: &str = "release.json";
/// The repository the releases come from.
pub const REPO: &str = "shin315/quota-station";
/// The releases page, for a Mac that cannot use the disk image.
pub const RELEASES_PAGE: &str = "https://github.com/shin315/quota-station/releases";
/// The bug form, where the diagnostic info is pasted.
pub const ISSUE_FORM: &str = "https://github.com/shin315/quota-station/issues/new";

/// How long the question may take.
const TIMEOUT: Duration = Duration::from_secs(20);
/// A release as GitHub sends it is a few kilobytes; its notes are text.
const MAX_BODY: u64 = 2 * 1024 * 1024;
/// A successful check holds for a day.
pub const DAILY: i64 = 24 * 60 * 60;
/// One that did not reach GitHub is tried again after an hour.
pub const RETRY: i64 = 60 * 60;
/// The notes' first lines the dialog lists.
const MAX_NOTES: usize = 6;

/// The latest release, as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    /// `0.3.0`, without the tag's `v`.
    pub version: String,
    /// RFC 3339.
    pub published_at: Option<String>,
    /// The notes' points, in plain words.
    pub notes: Vec<String>,
    /// The release's page.
    pub url: String,
    /// The installer for this system, when the release has one: the disk
    /// image on macOS, the NSIS `-setup.exe` for this processor on Windows.
    pub dmg: Option<String>,
    /// Its size in bytes.
    pub size: Option<u64>,
}

/// What the app keeps between checks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Releases {
    /// Unix seconds of the last check that ended, either way.
    #[serde(default)]
    pub checked_at: Option<i64>,
    /// Why it did not reach the release: `offline`, `rateLimited`, or GitHub's status.
    #[serde(default)]
    pub error: Option<String>,
    /// The repository has no release yet.
    #[serde(default)]
    pub none: bool,
    #[serde(default)]
    pub latest: Option<Latest>,
    /// "Bỏ qua bản này".
    #[serde(default)]
    pub skipped: Option<String>,
    /// "Để sau".
    #[serde(default)]
    pub later: Option<String>,
}

impl Releases {
    /// Whether the daily check is due at `now`: a day after a check that
    /// reached GitHub, an hour after one that did not.
    pub fn due(&self, now: i64) -> bool {
        match self.checked_at {
            None => true,
            Some(at) => now - at >= if self.error.is_some() { RETRY } else { DAILY },
        }
    }
}

pub fn load(app: &AppDir) -> Releases {
    app.read_to_string(&app.path().join(FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppDir, r: &Releases) -> io::Result<()> {
    let text = serde_json::to_string_pretty(r).map_err(io::Error::other)?;
    app.write_private(&app.path().join(FILE), &text)
}

// ===== Versions =====

/// `v0.3.0`, `0.3.0-beta.1` → `[0, 3, 0]`; None when it is not one.
fn parts(v: &str) -> Option<Vec<u64>> {
    let core = v.trim().trim_start_matches(['v', 'V']).split(['-', '+']).next()?;
    core.split('.').map(|p| p.parse().ok()).collect()
}

/// `latest` is newer than `current`. A pre-release never counts.
pub fn newer(latest: &str, current: &str) -> bool {
    if latest.contains('-') {
        return false;
    }
    match (parts(latest), parts(current)) {
        (Some(a), Some(b)) => {
            let n = a.len().max(b.len());
            let pad =
                |v: &Vec<u64>| (0..n).map(|i| v.get(i).copied().unwrap_or(0)).collect::<Vec<_>>();
            pad(&a) > pad(&b)
        }
        _ => false,
    }
}

// ===== The release =====

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Deserialize)]
struct Body {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// Only GitHub's own addresses for this repository are kept: the app opens them.
fn ours(url: &str) -> bool {
    url.starts_with(&format!("https://github.com/{REPO}/"))
}

/// The notes' points: the list items, in order, without their markdown;
/// the first plain lines when there is no list.
pub fn notes(body: &str) -> Vec<String> {
    let clean = |s: &str| {
        let s = s.replace("**", "").replace('`', "");
        // `[text](url)` keeps its text.
        let mut out = String::new();
        let mut rest = s.as_str();
        while let Some(open) = rest.find('[') {
            match (rest[open..].find("]("), rest[open..].find(')')) {
                (Some(mid), Some(end)) if mid < end => {
                    out.push_str(&rest[..open]);
                    out.push_str(&rest[open + 1..open + mid]);
                    rest = &rest[open + end + 1..];
                }
                _ => break,
            }
        }
        out.push_str(rest);
        out.trim().to_string()
    };
    let lines: Vec<&str> = body.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let items: Vec<String> = lines
        .iter()
        .filter_map(|l| l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")))
        .map(clean)
        .filter(|l| !l.is_empty())
        .take(MAX_NOTES)
        .collect();
    if !items.is_empty() {
        return items;
    }
    lines
        .iter()
        .filter(|l| !l.starts_with('#'))
        .map(|l| clean(l))
        .filter(|l| !l.is_empty())
        .take(MAX_NOTES)
        .collect()
}

/// Whether the asset `name` installs the app on `os` (`std::env::consts::OS`)
/// and `arch`: the disk image on macOS; on Windows the NSIS installer for the
/// processor, as tauri-action names it (`Quota.Station_0.3.0_x64-setup.exe`,
/// `…_arm64-setup.exe`), never the `.msi` nor the updater's `.sig`.
pub fn installs(name: &str, os: &str, arch: &str) -> bool {
    match (os, arch) {
        ("macos", _) => name.ends_with(".dmg"),
        ("windows", "x86_64") => name.ends_with("_x64-setup.exe"),
        ("windows", "aarch64") => name.ends_with("_arm64-setup.exe"),
        _ => false,
    }
}

/// The release GitHub calls latest; None for a draft or a pre-release, or
/// one whose page is not this repository's.
pub fn parse(json: &[u8]) -> Option<Latest> {
    parse_for(json, std::env::consts::OS, std::env::consts::ARCH)
}

/// [`parse`], with the installer chosen for `os` and `arch`.
pub fn parse_for(json: &[u8], os: &str, arch: &str) -> Option<Latest> {
    let b: Body = serde_json::from_slice(json).ok()?;
    if b.draft || b.prerelease || !ours(&b.html_url) {
        return None;
    }
    let version = b.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    parts(&version)?;
    let dmg =
        b.assets.iter().find(|a| installs(&a.name, os, arch) && ours(&a.browser_download_url));
    Some(Latest {
        version,
        published_at: b.published_at,
        notes: notes(b.body.as_deref().unwrap_or("")),
        url: b.html_url,
        dmg: dmg.map(|a| a.browser_download_url.clone()),
        size: dmg.and_then(|a| a.size),
    })
}

// ===== Asking GitHub =====

/// Where the check asks. The app always uses [`GitHub::api`].
#[derive(Debug, Clone)]
pub struct GitHub {
    base: String,
}

impl GitHub {
    pub fn api() -> Self {
        // The tests run a local fake; one that reached for GitHub stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake GitHub");
        }
        GitHub { base: "https://api.github.com".into() }
    }

    #[cfg(feature = "test-support")]
    pub fn at(base: &str) -> Self {
        GitHub { base: base.trim_end_matches('/').into() }
    }
}

/// Asks for the latest release and returns what the app keeps after it,
/// carrying `before`'s skipped and put-off versions.
pub fn check(gh: &GitHub, before: &Releases, now: i64) -> Releases {
    let mut after = Releases {
        checked_at: Some(now),
        skipped: before.skipped.clone(),
        later: before.later.clone(),
        ..Default::default()
    };
    let client = match reqwest::blocking::Client::builder().timeout(TIMEOUT).build() {
        Ok(c) => c,
        Err(_) => {
            after.error = Some("offline".into());
            after.latest = before.latest.clone();
            return after;
        }
    };
    let resp = client
        .get(format!("{}/repos/{REPO}/releases/latest", gh.base))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Quota-Station")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send();
    let failed = |after: &mut Releases, why: String| {
        after.error = Some(why);
        // What the last check found still stands.
        after.latest = before.latest.clone();
        after.none = before.none;
    };
    let resp = match resp {
        Ok(r) => r,
        Err(_) => {
            failed(&mut after, "offline".into());
            return after;
        }
    };
    match resp.status().as_u16() {
        200 => {}
        404 => {
            after.none = true;
            return after;
        }
        403 | 429 => {
            failed(&mut after, "rateLimited".into());
            return after;
        }
        s => {
            failed(&mut after, s.to_string());
            return after;
        }
    }
    let mut body = Vec::new();
    if resp.take(MAX_BODY).read_to_end(&mut body).is_err() {
        failed(&mut after, "offline".into());
        return after;
    }
    match parse(&body) {
        Some(l) => after.latest = Some(l),
        None => after.none = true,
    }
    after
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.3.0", "0.2.0"));
        assert!(newer("v0.10.0", "0.9.9"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.2.0", "0.2.0"));
        assert!(!newer("v0.2.0", "0.2.1"));
        // A pre-release is never offered; a version that is not one neither.
        assert!(!newer("0.3.0-beta.1", "0.2.0"));
        assert!(!newer("nightly", "0.2.0"));
    }

    #[test]
    fn the_notes_are_the_list_in_plain_words() {
        let body = "## What's new\n\n- **Faster** reads\n* A [guide](https://example.com/g) for `claude`\n\nThanks!";
        assert_eq!(notes(body), vec!["Faster reads".to_string(), "A guide for claude".to_string()]);
        assert_eq!(
            notes("## Title\nFirst line\nSecond line"),
            vec!["First line".to_string(), "Second line".to_string()]
        );
        let many = (0..10).map(|i| format!("- point {i}")).collect::<Vec<_>>().join("\n");
        assert_eq!(notes(&many).len(), MAX_NOTES);
    }

    #[test]
    fn a_release_keeps_its_disk_image_from_this_repository_only() {
        let json = br#"{
            "tag_name": "v0.3.0",
            "html_url": "https://github.com/shin315/quota-station/releases/tag/v0.3.0",
            "body": "- One",
            "published_at": "2026-10-23T08:00:00Z",
            "assets": [
                {"name": "notes.txt", "browser_download_url": "https://github.com/shin315/quota-station/releases/download/v0.3.0/notes.txt", "size": 10},
                {"name": "Quota.Station_0.3.0_aarch64.dmg", "browser_download_url": "https://github.com/shin315/quota-station/releases/download/v0.3.0/Quota.Station_0.3.0_aarch64.dmg", "size": 14000000}
            ]
        }"#;
        let l = parse_for(json, "macos", "aarch64").unwrap();
        assert_eq!(l.version, "0.3.0");
        assert_eq!(l.notes, vec!["One".to_string()]);
        assert_eq!(l.size, Some(14_000_000));
        assert!(l.dmg.unwrap().ends_with("_aarch64.dmg"));

        let elsewhere =
            br#"{"tag_name": "v0.3.0", "html_url": "https://evil.example/r", "assets": []}"#;
        assert_eq!(parse(elsewhere), None);
        let draft = br#"{"tag_name": "v0.3.0", "html_url": "https://github.com/shin315/quota-station/releases/tag/v0.3.0", "draft": true}"#;
        assert_eq!(parse(draft), None);
        let foreign_dmg = br#"{"tag_name": "v0.3.0", "html_url": "https://github.com/shin315/quota-station/releases/tag/v0.3.0",
            "assets": [{"name": "x.dmg", "browser_download_url": "https://evil.example/x.dmg"}]}"#;
        assert_eq!(parse_for(foreign_dmg, "macos", "aarch64").unwrap().dmg, None);
    }

    #[test]
    fn windows_takes_the_setup_exe_for_its_processor() {
        let base = "https://github.com/shin315/quota-station/releases/download/v0.3.0/";
        let assets = [
            "Quota.Station_0.3.0_aarch64.dmg",
            "Quota.Station_0.3.0_x64_en-US.msi",
            "Quota.Station_0.3.0_x64-setup.exe.sig",
            "Quota.Station_0.3.0_arm64-setup.exe",
            "Quota.Station_0.3.0_x64-setup.exe",
            "latest.json",
        ]
        .map(|name| {
            format!(r#"{{"name": "{name}", "browser_download_url": "{base}{name}", "size": 9}}"#)
        })
        .join(",");
        let json = format!(
            r#"{{"tag_name": "v0.3.0", "html_url": "https://github.com/shin315/quota-station/releases/tag/v0.3.0", "assets": [{assets}]}}"#
        );
        let pick = |os, arch| parse_for(json.as_bytes(), os, arch).unwrap().dmg;
        assert_eq!(
            pick("windows", "x86_64"),
            Some(format!("{base}Quota.Station_0.3.0_x64-setup.exe"))
        );
        assert_eq!(
            pick("windows", "aarch64"),
            Some(format!("{base}Quota.Station_0.3.0_arm64-setup.exe"))
        );
        assert_eq!(
            pick("macos", "aarch64"),
            Some(format!("{base}Quota.Station_0.3.0_aarch64.dmg"))
        );
        // Another system gets the release's page, as a release without one.
        assert_eq!(pick("linux", "x86_64"), None);
    }

    /// The Mac mini M6 at ca353c5: `release.json` sits in the app's folder
    /// itself and was refused, so Phiên bản kept saying it had never checked.
    #[test]
    fn a_check_is_kept_in_the_apps_folder_and_read_back() {
        let root = std::env::temp_dir().join(format!("qs-release-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = AppDir::new(root.join("app"));
        let r = Releases { checked_at: Some(1000), none: true, ..Default::default() };
        save(&app, &r).unwrap();
        assert_eq!(load(&app), r);
        assert!(!load(&app).due(1000 + 60));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_check_is_due_a_day_after_one_that_worked_and_an_hour_after_one_that_did_not() {
        let mut r = Releases::default();
        assert!(r.due(0));
        r.checked_at = Some(1000);
        assert!(!r.due(1000 + DAILY - 1));
        assert!(r.due(1000 + DAILY));
        r.error = Some("offline".into());
        assert!(!r.due(1000 + RETRY - 1));
        assert!(r.due(1000 + RETRY));
    }
}
