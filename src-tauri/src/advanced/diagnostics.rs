// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part in the app's diagnostics: whether Nâng cao shows on the
//! system line of what "Chép" copies, the rotation's self-check in a guided
//! test session, and the other Macs (`peers`) hidden in the diagnostic
//! bundle.

use std::path::Path;

use quota_station_core::advanced::settings::Own;
use quota_station_core::app_dir::AppDir;
use quota_station_core::settings::Settings;
use serde_json::{json, Value};

/// The keys the diagnostic bundle hides besides the app's: the other Macs
/// the rules are copied to, by their SSH names.
pub const HIDDEN: &[&str] = &["peers"];

/// The words after the system line: Nâng cao on or off.
pub fn words(settings: &Settings) -> Vec<String> {
    let on = Own::of(settings).advanced_shown();
    vec![format!("Advanced {}", if on { "on" } else { "off" })]
}

/// The advanced part's self-checks: `rotation` only.
pub fn self_check(id: &str, app_dir: &AppDir) -> Option<Value> {
    (id == "rotation").then(|| rotation(app_dir.path()))
}

/// The rotation's timer flag and its log: lines counted, the last one's
/// time, never their text.
fn rotation(app_dir: &Path) -> Value {
    let claude = app_dir.join("claude");
    let log = std::fs::read_to_string(claude.join("rotation.log")).unwrap_or_default();
    let last = log.lines().last().and_then(|l| l.split_whitespace().next());
    let last = last.filter(|w| w.len() <= 32 && w.starts_with(|c: char| c.is_ascii_digit()));
    json!({
        "timerOn": claude.join("rotation-timer").exists(),
        "logLines": log.lines().count(),
        "lastLine": last,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_counts_lines_and_keeps_only_the_last_time() {
        let dir = std::env::temp_dir().join(format!("qs-session-rot-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("claude")).unwrap();
        std::fs::write(
            dir.join("claude").join("rotation.log"),
            "2026-10-28T09:00:00Z kept lan@example.com\n2026-10-28T09:03:00Z switched to 2\n",
        )
        .unwrap();
        let v = rotation(&dir);
        assert_eq!(v["logLines"], 2);
        assert_eq!(v["lastLine"], "2026-10-28T09:03:00Z");
        assert_eq!(v["timerOn"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_the_rotation_is_a_self_check_of_the_private_part() {
        let dir = std::env::temp_dir().join(format!("qs-diag-rot-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("claude")).unwrap();
        std::fs::write(dir.join("claude").join("rotation-timer"), "").unwrap();
        let app_dir = AppDir::new(&dir);
        let v = self_check("rotation", &app_dir).expect("its own");
        assert_eq!(v, json!({ "timerOn": true, "logLines": 0, "lastLine": null }));
        assert_eq!(self_check("tray", &app_dir), None);
        assert_eq!(self_check("nothing", &app_dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_system_line_says_whether_advanced_shows() {
        let mut settings = Settings::default();
        assert_eq!(words(&settings), ["Advanced off"]);
        settings.ext.insert("advanced".into(), Value::Bool(true));
        assert_eq!(words(&settings), ["Advanced on"]);
    }
}
