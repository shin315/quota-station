// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Shared helpers for the fixture tests; each test file uses a subset.
//! Every fixture is synthetic: made-up slot numbers, percentages and
//! times, generated at 2026-01-16 07:20 UTC.
#![allow(dead_code)]

pub mod accounts;
pub mod advanced;
#[cfg(unix)]
pub mod claude;
// The Antigravity fakes and the fake `security` are shell scripts.
#[cfg(all(feature = "antigravity-accounts", unix))]
pub mod desktop;
pub mod ext;
#[cfg(all(feature = "antigravity-accounts", unix))]
pub mod google;
pub mod http;
#[cfg(unix)]
pub mod keychain;
pub mod os;
pub mod ui;

use std::path::{Path, PathBuf};

use quota_station_core::quota::Response;
use quota_station_core::rotation::Rules;

pub fn fixture_path(name: &str) -> PathBuf {
    let core = Path::new(quota_station_core::TEST_DIR);
    let here = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from);
    if let Some(why) = built_elsewhere(here.as_deref(), Path::new(env!("CARGO_MANIFEST_DIR")), core)
    {
        panic!("{why}");
    }
    core.join("tests/fixtures").join(name)
}

/// Why this test binary cannot read this checkout's fixtures, if it cannot:
/// `here` is the folder of the crate whose tests run (Cargo sets it as the
/// test runs), `built` that crate's folder and `core` the core's folder
/// ([`quota_station_core::TEST_DIR`]) where they were compiled. Cargo names
/// a workspace's builds by their paths inside it, so a target folder shared
/// with another checkout of the workspace (a community export, say) can
/// hand this checkout the binary or the core built there: its fixtures are
/// then the other checkout's, gone with that folder.
pub fn built_elsewhere(here: Option<&Path>, built: &Path, core: &Path) -> Option<String> {
    let same = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    // The core is `<checkout>/crates/quota-station-core`; the tests that
    // read its fixtures are in that checkout too (the core's, ext/author's).
    let checkout = core.parent().and_then(Path::parent).unwrap_or(core);
    let ok = built.starts_with(checkout) && here.is_none_or(|here| same(here, built));
    (!ok).then(|| {
        format!(
            "this test binary reads the fixtures of the core built in {}, not of this checkout \
             ({}): a target folder shared with that checkout let Cargo reuse its build; give \
             each checkout its own CARGO_TARGET_DIR",
            core.display(),
            here.unwrap_or(built).display()
        )
    })
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

/// Writes an executable `#!/bin/sh` script at `path`. Unix only: a test
/// that runs one runs there.
#[cfg(unix)]
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
