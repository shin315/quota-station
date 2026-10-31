// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A newer Quota Station on GitHub Releases, and the diagnostic info for a
//! bug report (the community edition's B1). The check only reads the
//! latest release; "Tải và cài" opens its disk image or page in the browser.
//! Nothing here downloads, installs or sends anything else.

use std::time::{Duration, Instant};

use quota_station_core::release::{self, GitHub, Latest, Releases};
use quota_station_core::snapshot::Section;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::open;
use crate::reader;
use crate::state::{AppState, Inner};

/// A check that did not start (or ran) is not tried again before this.
const RETRY: Duration = Duration::from_secs(60 * 60);

/// The check running now, and when the daily one last started.
#[derive(Debug, Default)]
pub struct Live {
    pub checking: bool,
    pub tried: Option<Instant>,
}

/// Cài đặt › Cập nhật, the banner on Tổng quan and the popover's line.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseView {
    /// "Tự kiểm tra bản mới của app".
    pub auto: bool,
    pub checking: bool,
    /// Unix seconds of the last check that ended.
    pub checked_at: Option<i64>,
    /// Why the last check did not reach GitHub: `offline`, `rateLimited`, a status.
    pub error: Option<String>,
    /// The repository has no release yet.
    pub none: bool,
    pub latest: Option<Latest>,
    /// The latest release is newer than this copy.
    pub newer: bool,
    pub skipped: Option<String>,
    pub later: Option<String>,
}

pub fn view(inner: &Inner) -> ReleaseView {
    let r = release::load(&inner.app_dir());
    ReleaseView {
        auto: !inner.settings.release_checks_off,
        checking: inner.release.checking,
        checked_at: r.checked_at,
        error: r.error,
        none: r.none,
        newer: r.latest.as_ref().is_some_and(|l| release::newer(&l.version, &inner.app_version)),
        latest: r.latest,
        skipped: r.skipped,
        later: r.later,
    }
}

fn check(app: &AppHandle, state: &AppState) {
    let dir = state.lock().app_dir();
    let before = release::load(&dir);
    let after = release::check(&GitHub::api(), &before, OffsetDateTime::now_utc().unix_timestamp());
    let _ = release::save(&dir, &after);
    reader::publish(app, state);
}

fn start(app: &AppHandle, state: &AppState) -> bool {
    {
        let mut inner = state.lock();
        if inner.release.checking {
            return false;
        }
        inner.release.checking = true;
        inner.release.tried = Some(Instant::now());
    }
    reader::publish(app, state);
    true
}

fn end(app: &AppHandle, state: &AppState) {
    state.lock().release.checking = false;
    reader::publish(app, state);
}

/// "Kiểm tra bản mới".
#[tauri::command]
pub async fn release_check(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    if !start(&app, &shared) {
        return Ok(());
    }
    let (a, s) = (app.clone(), shared.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || check(&a, &s)).await;
    end(&app, &shared);
    Ok(())
}

/// After each read: once a day while "Tự kiểm tra bản mới của app" is on, an
/// hour after a check that did not reach GitHub.
pub fn daily(app: &AppHandle, state: &AppState) {
    {
        let inner = state.lock();
        if inner.settings.release_checks_off
            || inner.release.checking
            || inner.release.tried.is_some_and(|t| t.elapsed() < RETRY)
        {
            return;
        }
        if !release::load(&inner.app_dir()).due(OffsetDateTime::now_utc().unix_timestamp()) {
            return;
        }
    }
    if !start(app, state) {
        return;
    }
    let (a, s) = (app.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        let (a2, s2) = (a.clone(), s.clone());
        let _ = tauri::async_runtime::spawn_blocking(move || check(&a2, &s2)).await;
        end(&a, &s);
    });
}

/// Changes what the app keeps about the releases, then shows it.
fn keep(app: &AppHandle, state: &AppState, f: impl FnOnce(&mut Releases)) -> Result<(), String> {
    let dir = state.lock().app_dir();
    let mut r = release::load(&dir);
    f(&mut r);
    release::save(&dir, &r).map_err(|e| e.to_string())?;
    reader::publish(app, state);
    Ok(())
}

/// "Bỏ qua bản này" (a version), or its "Hoàn tác" (None).
#[tauri::command]
pub fn release_skip(
    app: AppHandle,
    state: State<'_, AppState>,
    version: Option<String>,
) -> Result<(), String> {
    keep(&app, &state, |r| r.skipped = version)
}

/// "Để sau": this version's banner and popover line go quiet.
#[tauri::command]
pub fn release_later(
    app: AppHandle,
    state: State<'_, AppState>,
    version: String,
) -> Result<(), String> {
    keep(&app, &state, |r| r.later = Some(version))
}

/// Cài đặt › Cập nhật: "Tự kiểm tra bản mới của app".
#[tauri::command]
pub fn set_release_checks(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.release_checks_off = !on;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(())
}

/// Opens one of the repository's pages: `download` (the latest disk image,
/// else its page), `notes` (the latest release's page), `releases`, or
/// `issue` (the bug form). Only this repository's addresses on github.com.
#[tauri::command]
pub fn release_open(
    app: AppHandle,
    state: State<'_, AppState>,
    what: String,
) -> Result<(), String> {
    let latest = release::load(&state.lock().app_dir()).latest;
    let url = match what.as_str() {
        "download" => latest.as_ref().and_then(|l| l.dmg.clone().or_else(|| Some(l.url.clone()))),
        "notes" => latest.as_ref().map(|l| l.url.clone()),
        "releases" => Some(release::RELEASES_PAGE.to_string()),
        "issue" => Some(release::ISSUE_FORM.to_string()),
        _ => None,
    }
    .unwrap_or_else(|| release::RELEASES_PAGE.to_string());
    if !url.starts_with(&format!("https://github.com/{}/", release::REPO))
        && url != release::RELEASES_PAGE
    {
        return Err("notOurs".into());
    }
    open::url(&app, &url)
}

// ===== The diagnostic info =====

/// macOS's version, as `sw_vers` gives it.
#[cfg(not(windows))]
fn macos() -> String {
    std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "?".into())
}

/// The system and its version: `macOS 15.1`.
#[cfg(not(windows))]
fn system() -> String {
    format!("macOS {}", macos())
}

/// The system and its version, as Settings › System › About shows it:
/// `Windows 11 24H2 (build 26100.2033)`. Windows 11 still names itself
/// Windows 10 in the registry; its builds start at 22000.
#[cfg(windows)]
fn system() -> String {
    let Ok(key) =
        windows_registry::LOCAL_MACHINE.open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
    else {
        return "Windows ?".into();
    };
    let build = key.get_string("CurrentBuildNumber").unwrap_or_default();
    let name = match build.parse::<u32>() {
        Ok(b) if b >= 22000 => "Windows 11",
        Ok(_) => "Windows 10",
        Err(_) => "Windows",
    };
    let release = key.get_string("DisplayVersion").unwrap_or_default();
    let patch = key.get_u32("UBR").map(|u| format!(".{u}")).unwrap_or_default();
    format!("{name} {release} (build {build}{patch})").replace("  ", " ")
}

/// `text` with every `home` in it as `~`; on Windows, where `HOME` is
/// rarely set and a path may come in any case, the profile folder
/// (`C:\Users\<name>`) in any case.
pub(crate) fn tilde(text: &str, home: &str) -> String {
    if home.len() <= 1 {
        return text.to_string();
    }
    if !windows_path(home) {
        return text.replace(home, "~");
    }
    // ASCII lowercasing keeps every byte where it was.
    let (lower, needle) = (text.to_ascii_lowercase(), home.to_ascii_lowercase());
    let mut out = String::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(&needle) {
        out.push_str(&text[from..from + at]);
        out.push('~');
        from += at + needle.len();
    }
    out.push_str(&text[from..]);
    out
}

/// A Windows path: a drive (`C:\`, `c:/`) or a share (`\\server`).
pub(crate) fn windows_path(word: &str) -> bool {
    let b = word.as_bytes();
    word.starts_with("\\\\")
        || (b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && matches!(b[2], b'\\' | b'/'))
}

/// A line with no email, no path outside the home folder (the home folder
/// itself becomes `~`), and nothing longer than a short reason. A Windows
/// path counts as a path too, so `C:\Users\<name>` never shows.
pub fn scrub(text: &str, home: &str) -> String {
    let text = tilde(text, home);
    text.split_whitespace()
        .map(|w| {
            let bare = w.trim_start_matches(['(', '"', '\'', '`']);
            if w.contains('@') {
                "[email]"
            } else if w.starts_with('/') || windows_path(bare) {
                "[path]"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

fn section_line(name: &str, s: &Section, home: &str) -> String {
    let status = serde_json::to_value(s.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let mut line = format!("{name}: {status}, {} account(s)", s.accounts.len());
    if let Some(r) = &s.reason {
        line += &format!(" · {}", scrub(r, home));
    }
    line
}

/// What "Chép" copies, exactly as the dialog shows it: the version, the
/// build, macOS and the chip, the language, what was found and each
/// service's last read. No email, account name, key or path outside ~.
pub fn diagnostics(inner: &Inner) -> String {
    // Windows rarely sets `HOME`: its profile folder is the app's home.
    let home = if cfg!(windows) {
        inner.home.display().to_string()
    } else {
        std::env::var("HOME").unwrap_or_default()
    };
    let mut features = Vec::new();
    if cfg!(feature = "antigravity-accounts") {
        features.push("antigravity-accounts");
    }
    #[cfg(feature = "author-tools")]
    features.push("author-tools");
    let build = if features.is_empty() { "MIT build".to_string() } else { features.join(", ") };
    let commit = crate::install::COMMIT.unwrap_or("no commit");
    let snap = inner.detailed_snapshot();
    let keychain = serde_json::to_value(snap.keychain)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let language = serde_json::to_value(inner.settings.language)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let platform = format!("{} · {} · language {language}", system(), std::env::consts::ARCH);
    // And Nâng cao.
    #[cfg(feature = "author-tools")]
    let platform = format!(
        "{platform} · Advanced {}",
        if inner.settings.advanced_shown() { "on" } else { "off" }
    );
    let lines = [
        format!("Quota Station {} ({commit}) · {build}", inner.app_version),
        platform,
        format!(
            "Claude Code: {} · {} account(s) kept · Keychain {keychain}",
            if inner.claude_program.is_some() { "found" } else { "not found" },
            inner.claude_accounts.len()
        ),
        section_line("Claude quota", &snap.claude, &home),
        section_line("Antigravity quota", &snap.agy, &home),
        section_line("Codex quota", &snap.codex, &home),
    ];
    lines.join("\n")
}

/// The dialog's text; "Chép" puts it on the clipboard from the window.
#[tauri::command]
pub fn release_diagnostics(state: State<'_, AppState>) -> String {
    diagnostics(&state.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_info_keeps_no_email_and_no_path_outside_home() {
        assert_eq!(
            scrub(
                "read failed for lan@example.com at /Users/lan/.claude/x and /opt/tool",
                "/Users/lan"
            ),
            "read failed for [email] at ~/.claude/x and [path]"
        );
        assert_eq!(scrub(&"x".repeat(400), "/Users/lan").len(), 160);
    }

    #[test]
    fn a_windows_profile_folder_never_shows_whatever_its_case() {
        assert_eq!(
            scrub(
                r"read failed at C:\Users\Lan\.claude\x and c:\users\lan\AppData (D:\work\y) \\nas\share",
                r"C:\Users\Lan"
            ),
            r"read failed at ~\.claude\x and ~\AppData [path] [path]"
        );
        // Another user's folder, or one under a home that is not set.
        assert_eq!(scrub(r"at C:\Users\other\x", ""), "at [path]");
        assert_eq!(scrub(r"at C:/Users/other/x", r"C:\Users\Lan"), "at [path]");
    }
}
