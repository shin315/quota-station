// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Shared helpers for the fixture tests; each test file uses a subset.
//! Every fixture is synthetic: made-up slot numbers, percentages and
//! times, generated at 2026-01-16 07:20 UTC.
#![allow(dead_code)]

pub mod accounts;
pub mod claude;
#[cfg(feature = "antigravity-accounts")]
pub mod desktop;
#[cfg(feature = "antigravity-accounts")]
pub mod google;
pub mod http;
pub mod keychain;

use std::path::PathBuf;

use quota_station_core::quota::Response;
use quota_station_core::rotation::Rules;

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).expect("fixture exists")
}

pub fn fixture(name: &str) -> Response {
    serde_json::from_str(&fixture_text(name)).expect("fixture parses")
}

static NEXT_DIR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A fresh, empty directory for one test, under the system's temp folder.
pub fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "qs-{tag}-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes an executable `#!/bin/sh` script at `path`.
pub fn script(path: &std::path::Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_path_buf()
}

/// The Claude accounts' details the app shows, from a fixture in the shape
/// of claude-swap's `cswap list --json`, as the fixtures were made.
pub fn claude_details(name: &str) -> Vec<quota_station_core::detail::ClaudeDetail> {
    use quota_station_core::detail::{ClaudeDetail, Usage};
    use serde_json::Value;
    let data: Value = serde_json::from_str(&fixture_text(name)).expect("fixture parses");
    let text = |v: &Value| v.as_str().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
    let usage = |v: &Value| {
        Some(Usage { used: v.get("pct")?.as_f64()?, resets_at: v.get("resetsAt").and_then(text) })
    };
    data["accounts"]
        .as_array()
        .expect("accounts")
        .iter()
        .map(|a| ClaudeDetail {
            number: a["number"].as_u64().unwrap() as u32,
            email: text(&a["email"]),
            organization: text(&a["organizationName"]),
            five_hour: usage(&a["usage"]["fiveHour"]),
            seven_day: usage(&a["usage"]["sevenDay"]),
            fable: a["usage"]["scoped"]
                .as_array()
                .and_then(|s| {
                    s.iter().find(|w| {
                        w["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("fable"))
                    })
                })
                .and_then(usage),
        })
        .collect()
}

/// A rotation with every key set: Team 2 3 4, personal 1 from 8 to 23, full
/// at 90 %, Fable full at 95 %, no end times. The app has no built-in
/// rotation; this is one a test writes.
pub fn sample_rules() -> Rules {
    Rules {
        team: vec![2, 3, 4],
        personal: Some(1),
        personal_from: 8,
        personal_to: 23,
        full: 90.0,
        fable_full: 95.0,
        expire: Vec::new(),
    }
}
