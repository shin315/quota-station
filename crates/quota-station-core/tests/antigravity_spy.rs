// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The spy test of SECURITY.md: every Antigravity flow runs in a temporary
//! home that has a decoy, holding a sentinel secret, at every path the app
//! must never touch, and a decoy for the `gemini`/`antigravity` Keychain item
//! that Antigravity.app and agy share. Afterwards every file the module
//! opened must be inside the app's folder or the (fake) Antigravity bundle it
//! reads the version from, every `security` run must name the app's own
//! service, except the one write of a switch, which must be Cockpit's exact
//! command, the decoys must be as they were (the shared item replaced by the
//! switch, never read), and the sentinel must appear nowhere the app wrote or
//! sent anything.
//!
//! In a file of its own: the list of opened files is global to the test
//! binary, so no other test that opens files may run beside it.
#![cfg(feature = "antigravity-accounts")]

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use common::desktop::{agy_report, fake_quota_axi, FakeApp};
use common::google::{
    token_body, FakeBrowser, FakeGoogle, Home, Script, ACCOUNT_ID, INVALID_GRANT, MODELS,
};
use quota_station_core::antigravity::credential::SharedLogin;
use quota_station_core::antigravity::store::SERVICE;
use quota_station_core::antigravity::switch::{
    self, CloudCodeProbe, How, Limits, Outcome, Request,
};
use quota_station_core::antigravity::version::AntigravityApp;
use quota_station_core::antigravity::{client, index, login, pace, quota, watch};
use quota_station_core::app_dir as fs;

const SENTINEL: &str = "SENTINEL-must-never-leave-0f3c";

/// The paths SECURITY.md lists, under `home`.
const FORBIDDEN: [&str; 11] = [
    ".claude-swap-backup/credentials/1.json",
    ".claude-swap-backup/configs/1.json",
    ".claude-swap-backup/sessions/1/.credentials.json",
    ".claude-swap-backup/sequence.json",
    ".cockpit_tools/accounts/antigravity_x.json",
    ".cockpit_tools/secure-account-storage.key",
    ".cockpit_tools/server.json",
    ".cockpit_tools/config.json",
    ".cockpit_tools/accounts.json",
    ".antigravity_cockpit/accounts.json",
    ".claude/.credentials.json",
];

fn snapshot(files: &[PathBuf]) -> Vec<(PathBuf, String, SystemTime)> {
    files
        .iter()
        .map(|f| {
            let meta = std::fs::metadata(f).unwrap();
            (f.clone(), std::fs::read_to_string(f).unwrap(), meta.modified().unwrap())
        })
        .collect()
}

fn all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            all_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn the_antigravity_flows_never_touch_another_tools_secrets() {
    let home = Home::new("agy-spy");
    let mut decoys: Vec<PathBuf> = FORBIDDEN.iter().map(|p| home.home.join(p)).collect();
    // The shared item, as the fake `security` would keep it.
    decoys.push(home.sec.join("items/gemini--antigravity"));
    for d in &decoys {
        std::fs::create_dir_all(d.parent().unwrap()).unwrap();
        std::fs::write(d, format!("{{\"token\":\"{SENTINEL}\"}}")).unwrap();
    }
    let before = snapshot(&decoys);
    std::thread::sleep(Duration::from_millis(20));
    let _ = fs::take_opened();

    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let now = || 1_768_548_000;

    // Every flow there is: the client check, two logins (one through a
    // pasted callback), a new login of the first, a use with a refresh, a
    // switch that quits and reopens Antigravity.app, the overwrite watch, a
    // refusal, and a logout.
    assert_eq!(client::status(&home.app), client::ClientStatus::Ok);
    let browser = FakeBrowser::following();
    let flow = login::prepare(&deps).unwrap();
    login::complete(&deps, flow, &browser, Duration::from_secs(20), &now).unwrap();

    google.set(|s| {
        s.userinfo.1 = r#"{"id":"1000002","email":"ultra.two@example.com"}"#.into();
        s.exchange.1 = token_body("fake-access-b", Some("fake-refresh-b"));
    });
    let flow = login::prepare(&deps).unwrap();
    let handle = flow.handle();
    let paster = std::thread::spawn(move || {
        let callback = common::google::redirect_of(handle.auth_url());
        handle.submit_callback_url(&callback).unwrap();
    });
    let second =
        login::complete(&deps, flow, &FakeBrowser::default(), Duration::from_secs(20), &now)
            .unwrap();
    paster.join().unwrap();

    google.set(|s| s.userinfo.1 = r#"{"id":"1000001","email":"someone@example.com"}"#.into());
    let flow = login::prepare(&deps).unwrap();
    login::complete(&deps, flow, &FakeBrowser::following(), Duration::from_secs(20), &now).unwrap();

    login::fresh_token(&deps, ACCOUNT_ID, now() + 3500).unwrap();

    // Quota reads of both accounts, with the version from a fake bundle:
    // in turn, again from the cache, and once refused.
    let bundle = home.home.join("Applications/Antigravity IDE.app");
    std::fs::create_dir_all(bundle.join("Contents/Resources/app")).unwrap();
    std::fs::write(
        bundle.join("Contents/Resources/app/product.json"),
        r#"{"ideVersion":"2.24.0"}"#,
    )
    .unwrap();
    let identity = AntigravityApp::at(&bundle).identity();
    let hosts = google.hosts();
    let reads = quota::Deps { login: &deps, hosts: &hosts, identity: &identity };
    let (index_now, candidates) = quota::candidates(&home.app).unwrap();
    let due = pace::Pace::new().take(&candidates, quota::active_id(&index_now, None).as_deref(), 0);
    assert_eq!(due.len(), 2);
    let ms = (now() + 3500) * 1000;
    quota::read_in_turn(&reads, &due, &|| ms, &mut |_, r| assert!(r.is_none_or(|r| r.is_ok())));
    quota::read_account(&reads, ACCOUNT_ID, ms + 1_000).unwrap();
    google.set(|s| s.models = (403, r#"{"error":{"code":403}}"#.into()));
    assert!(quota::read_account(&reads, &second.id, ms + 120_000).is_err());
    assert!(!google.calls(MODELS).is_empty());

    // A switch that quits and reopens a fake Antigravity.app, then the watch.
    let shared_item = home.sec.join("items/gemini--antigravity");
    let app = FakeApp::new(&home.home.join("desktop"), Some("2.24.0"));
    app.start();
    let axi = fake_quota_axi(&home.home);
    std::fs::write(home.home.join("agy.json"), agy_report(2, Some(2), "reading", "quota-station"))
        .unwrap();
    let desktop = app.desktop();
    let shared = SharedLogin::fake(home.sec.join("bin/security"), Duration::from_secs(5));
    let switch_deps = switch::Deps {
        app: &home.app,
        endpoints: &endpoints,
        keychain: &home.keychain,
        probe: Arc::new(CloudCodeProbe { hosts: hosts.clone(), identity: identity.clone() }),
        shared: &shared,
        desktop: &desktop,
        quota_axi: Some(&axi),
    };
    let request = Request { id: second.id.clone(), how: How::QuitFirst, from: None };
    let limits = Limits { settle: Duration::ZERO, ..Limits::default() };
    let done = switch::switch_antigravity(&switch_deps, &request, limits, &|| now() + 3600);
    assert!(matches!(done.outcome, Outcome::Switched { .. }), "{:?}", done.outcome);
    let report =
        quota_station_core::axi::parse(&agy_report(2, Some(1), "reading", "quota-station"));
    let active = switch::shown_active_id(&report.unwrap(), &index::load(&home.app).unwrap());
    watch::observe(&home.app, active.as_deref(), now() + 7200);
    watch::dismiss(&home.app).unwrap();
    let _ = switch::read_log(&home.app);

    google.set(|s| s.refresh = (400, INVALID_GRANT.into()));
    // The read above refreshed this token; this refresh is due an hour on.
    assert!(login::fresh_token(&deps, &second.id, now() + 7100).is_err());
    login::logout(&deps, ACCOUNT_ID).unwrap();
    let _ = index::load(&home.app).unwrap();
    let _ = index::load_statuses(&home.app);

    // 1. Every file opened is in the app's folder, or in a bundle: the one
    // the version is read from, and the (fake) Antigravity.app the switch
    // only looks for.
    let opened = fs::take_opened();
    assert!(!opened.is_empty());
    assert!(opened.iter().any(|p| p.starts_with(&bundle)), "the bundle was read");
    for path in &opened {
        let allowed =
            path.starts_with(home.app.path()) || path.starts_with(&bundle) || path == &app.app;
        assert!(allowed, "opened {}", path.display());
    }

    // 2. Every `security` run is for the app's own service, but the one
    // write of the switch, which is Cockpit's command and nothing else.
    let calls = home.security_calls();
    assert!(calls.len() >= 6, "{calls:?}");
    let mut shared_writes = 0;
    for call in &calls {
        if call.iter().any(|a| a == "gemini" || a == "antigravity") {
            assert_eq!(call.len(), 9, "{call:?}");
            assert_eq!(
                call[..7],
                ["add-generic-password", "-U", "-s", "gemini", "-a", "antigravity", "-w"]
            );
            assert!(call[7].starts_with("go-keyring-base64:") && call[8] == "-A", "{call:?}");
            shared_writes += 1;
            continue;
        }
        let svc = call.iter().position(|a| a == "-s").map(|i| call[i + 1].as_str());
        assert_eq!(svc, Some(SERVICE), "{call:?}");
        assert!(!call.iter().any(|a| a == "-A"), "{call:?}");
    }
    assert_eq!(shared_writes, 1);

    // 3. The decoys are untouched, but the shared item, which the switch
    // replaced without reading it.
    let unchanged: Vec<_> =
        snapshot(&decoys).into_iter().filter(|(f, ..)| *f != shared_item).collect();
    let was: Vec<_> = before.into_iter().filter(|(f, ..)| *f != shared_item).collect();
    assert_eq!(unchanged, was);
    let item = std::fs::read_to_string(&shared_item).unwrap();
    assert!(item.starts_with("go-keyring-base64:") && !item.contains(SENTINEL));
    assert!(app.wait_running(true), "the switch opened Antigravity.app again");

    // 4. The sentinel is nowhere the app wrote or sent anything.
    let mut written = Vec::new();
    all_files(home.app.path(), &mut written);
    for file in written {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        assert!(!text.contains(SENTINEL), "{}", file.display());
    }
    for call in &calls {
        assert!(!call.iter().any(|a| a.contains(SENTINEL)));
    }
    for request in google.requests() {
        assert!(!format!("{request:?}").contains(SENTINEL));
    }
    for url in browser.opened() {
        assert!(!url.contains(SENTINEL));
    }
}

/// What opens, makes, moves or removes files, or asks the filesystem about
/// a path, outside the folder guard.
const FILE_CALLS: [&str; 11] = [
    "fs::",
    "std::fs",
    "File::",
    "OpenOptions",
    "DirBuilder",
    ".exists()",
    ".is_dir()",
    ".read_dir(",
    ".read_link(",
    ".canonicalize(",
    "symlink_metadata",
];

/// Every file of the Antigravity module and the app's commands for it: only
/// the folder guard touches files, and no code (comments aside) names
/// another tool's secret files, nor the shared Keychain item outside the one
/// module that writes it.
#[test]
fn only_the_folder_guard_opens_files() {
    let sources: [(&str, &str); 21] = [
        ("client.rs", include_str!("../src/antigravity/client.rs")),
        ("cloud_code.rs", include_str!("../src/antigravity/cloud_code.rs")),
        ("credential.rs", include_str!("../src/antigravity/credential.rs")),
        ("desktop.rs", include_str!("../src/antigravity/desktop.rs")),
        ("export.rs", include_str!("../src/antigravity/export.rs")),
        ("app_dir.rs", include_str!("../src/app_dir.rs")),
        ("index.rs", include_str!("../src/antigravity/index.rs")),
        ("login.rs", include_str!("../src/antigravity/login.rs")),
        ("mod.rs", include_str!("../src/antigravity/mod.rs")),
        ("oauth.rs", include_str!("../src/antigravity/oauth.rs")),
        ("oauth_server.rs", include_str!("../src/antigravity/oauth_server.rs")),
        ("pace.rs", include_str!("../src/antigravity/pace.rs")),
        ("quota.rs", include_str!("../src/antigravity/quota.rs")),
        ("store.rs", include_str!("../src/antigravity/store.rs")),
        ("switch.rs", include_str!("../src/antigravity/switch.rs")),
        ("token.rs", include_str!("../src/antigravity/token.rs")),
        ("version.rs", include_str!("../src/antigravity/version.rs")),
        ("watch.rs", include_str!("../src/antigravity/watch.rs")),
        ("src-tauri antigravity.rs", include_str!("../../../src-tauri/src/antigravity.rs")),
        ("src-tauri agy_quota.rs", include_str!("../../../src-tauri/src/agy_quota.rs")),
        ("src-tauri agy_switch.rs", include_str!("../../../src-tauri/src/agy_switch.rs")),
    ];
    let names = [
        ".cockpit_tools",
        ".antigravity_cockpit",
        ".claude-swap-backup",
        ".credentials.json",
        "\"gemini\"",
        "secure-account-storage",
    ];
    for (file, text) in sources {
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if file != "app_dir.rs" {
                // The guard's own module and the trait for a file's mode are
                // the only `fs` paths allowed; any other names the filesystem.
                let code =
                    code.replace("crate::app_dir::", "").replace("unix::fs::PermissionsExt", "");
                for call in FILE_CALLS {
                    assert!(!code.contains(call), "{file}:{}: {line}", n + 1);
                }
                let std_group = code.starts_with("use std::{") && code.contains("fs");
                assert!(!std_group, "{file}:{}: {line}", n + 1);
            }
            for name in names {
                // Only the module that writes the shared login names it.
                if file == "credential.rs" && name == "\"gemini\"" {
                    continue;
                }
                assert!(!code.contains(name), "{file}:{}: {line}", n + 1);
            }
        }
    }
}
