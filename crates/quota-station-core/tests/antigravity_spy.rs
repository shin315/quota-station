//! The spy test of SECURITY.md: every Antigravity flow runs in a temporary
//! home that has a decoy, holding a sentinel secret, at every path the app
//! must never touch, and a decoy for the `gemini`/`antigravity` Keychain item
//! that Antigravity.app and agy share. Afterwards every file the module
//! opened must be inside the app's folder or the (fake) Antigravity bundle it
//! reads the version from, every `security` run must name the app's own
//! service, the decoys must be as they were, and the sentinel must appear
//! nowhere the app wrote or sent anything.
//!
//! In a file of its own: the list of opened files is global to the test
//! binary, so no other test that opens files may run beside it.

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::google::{
    token_body, FakeBrowser, FakeGoogle, Home, Script, ACCOUNT_ID, INVALID_GRANT, MODELS,
};
use quota_station_core::antigravity::store::SERVICE;
use quota_station_core::antigravity::version::AntigravityApp;
use quota_station_core::antigravity::{client, fs, index, login, pace, quota};

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

    google.set(|s| s.refresh = (400, INVALID_GRANT.into()));
    // The read above refreshed this token; this refresh is due an hour on.
    assert!(login::fresh_token(&deps, &second.id, now() + 7100).is_err());
    login::logout(&deps, ACCOUNT_ID).unwrap();
    let _ = index::load(&home.app).unwrap();
    let _ = index::load_statuses(&home.app);

    // 1. Every file opened is in the app's folder, or in the bundle.
    let opened = fs::take_opened();
    assert!(!opened.is_empty());
    assert!(opened.iter().any(|p| p.starts_with(&bundle)), "the bundle was read");
    for path in &opened {
        let allowed = path.starts_with(home.app.path()) || path.starts_with(&bundle);
        assert!(allowed, "opened {}", path.display());
    }

    // 2. Every `security` run is for the app's own service.
    let calls = home.security_calls();
    assert!(calls.len() >= 6, "{calls:?}");
    for call in &calls {
        let svc = call.iter().position(|a| a == "-s").map(|i| call[i + 1].as_str());
        assert_eq!(svc, Some(SERVICE), "{call:?}");
        assert!(!call.iter().any(|a| a == "gemini" || a == "antigravity"), "{call:?}");
        assert!(!call.iter().any(|a| a == "-A"), "{call:?}");
    }

    // 3. The decoys are untouched.
    assert_eq!(snapshot(&decoys), before);

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
/// another tool's secret files or the shared Keychain item.
#[test]
fn only_the_folder_guard_opens_files() {
    let sources: [(&str, &str); 16] = [
        ("client.rs", include_str!("../src/antigravity/client.rs")),
        ("cloud_code.rs", include_str!("../src/antigravity/cloud_code.rs")),
        ("export.rs", include_str!("../src/antigravity/export.rs")),
        ("fs.rs", include_str!("../src/antigravity/fs.rs")),
        ("index.rs", include_str!("../src/antigravity/index.rs")),
        ("login.rs", include_str!("../src/antigravity/login.rs")),
        ("mod.rs", include_str!("../src/antigravity/mod.rs")),
        ("oauth.rs", include_str!("../src/antigravity/oauth.rs")),
        ("oauth_server.rs", include_str!("../src/antigravity/oauth_server.rs")),
        ("pace.rs", include_str!("../src/antigravity/pace.rs")),
        ("quota.rs", include_str!("../src/antigravity/quota.rs")),
        ("store.rs", include_str!("../src/antigravity/store.rs")),
        ("token.rs", include_str!("../src/antigravity/token.rs")),
        ("version.rs", include_str!("../src/antigravity/version.rs")),
        ("src-tauri antigravity.rs", include_str!("../../../src-tauri/src/antigravity.rs")),
        ("src-tauri agy_quota.rs", include_str!("../../../src-tauri/src/agy_quota.rs")),
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
            if file != "fs.rs" {
                // The guard's own module and the trait for a file's mode are
                // the only `fs` paths allowed; any other names the filesystem.
                let code = code
                    .replace("super::fs::", "")
                    .replace("antigravity::fs::", "")
                    .replace("unix::fs::PermissionsExt", "");
                for call in FILE_CALLS {
                    assert!(!code.contains(call), "{file}:{}: {line}", n + 1);
                }
                let std_group = code.starts_with("use std::{") && code.contains("fs");
                assert!(!std_group, "{file}:{}: {line}", n + 1);
            }
            for name in names {
                assert!(!code.contains(name), "{file}:{}: {line}", n + 1);
            }
        }
    }
}
