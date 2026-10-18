//! The spy test of SECURITY.md: every Antigravity flow runs in a temporary
//! home that has a decoy, holding a sentinel secret, at every path the app
//! must never touch, and a decoy for the `gemini`/`antigravity` Keychain item
//! that Antigravity.app and agy share. Afterwards every file the module
//! opened must be inside the app's folder, every `security` run must name the
//! app's own service, the decoys must be as they were, and the sentinel must
//! appear nowhere the app wrote or sent anything.
//!
//! In a file of its own: the list of opened files is global to the test
//! binary, so no other test that opens files may run beside it.

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::google::{
    token_body, FakeBrowser, FakeGoogle, Home, Script, ACCOUNT_ID, INVALID_GRANT,
};
use quota_station_core::antigravity::store::SERVICE;
use quota_station_core::antigravity::{client, fs, index, login};

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
    google.set(|s| s.refresh = (400, INVALID_GRANT.into()));
    assert!(login::fresh_token(&deps, &second.id, now() + 3500).is_err());
    login::logout(&deps, ACCOUNT_ID).unwrap();
    let _ = index::load(&home.app).unwrap();
    let _ = index::load_statuses(&home.app);

    // 1. Every file opened is in the app's folder.
    let opened = fs::take_opened();
    assert!(!opened.is_empty());
    for path in &opened {
        assert!(path.starts_with(home.app.path()), "opened {}", path.display());
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

/// Every file of the Antigravity module and the app's commands for it: only
/// the folder guard touches files, and no code (comments aside) names
/// another tool's secret files or the shared Keychain item.
#[test]
fn only_the_folder_guard_opens_files() {
    let sources: [(&str, &str); 10] = [
        ("client.rs", include_str!("../src/antigravity/client.rs")),
        ("fs.rs", include_str!("../src/antigravity/fs.rs")),
        ("index.rs", include_str!("../src/antigravity/index.rs")),
        ("login.rs", include_str!("../src/antigravity/login.rs")),
        ("mod.rs", include_str!("../src/antigravity/mod.rs")),
        ("oauth.rs", include_str!("../src/antigravity/oauth.rs")),
        ("oauth_server.rs", include_str!("../src/antigravity/oauth_server.rs")),
        ("store.rs", include_str!("../src/antigravity/store.rs")),
        ("token.rs", include_str!("../src/antigravity/token.rs")),
        ("src-tauri antigravity.rs", include_str!("../../../src-tauri/src/antigravity.rs")),
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
                for call in ["std::fs::", "File::open", "fs::read", "fs::write", "OpenOptions"] {
                    assert!(!code.contains(call), "{file}:{}: {line}", n + 1);
                }
            }
            for name in names {
                assert!(!code.contains(name), "{file}:{}: {line}", n + 1);
            }
        }
    }
}
