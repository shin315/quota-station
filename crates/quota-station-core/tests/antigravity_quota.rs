//! Reading Antigravity accounts' quota end to end, against a local fake
//! Google and Cloud Code, a fake `security` and a fake Antigravity bundle in
//! a temporary home: the calls and headers Cockpit Tools makes, the cache it
//! writes, the 60-second TTL, one account at a time, and what 401, 403, 429,
//! 5xx and network failures do. These tests never reach Google and never
//! touch the Keychain. The last one runs the real quota-axi fork on the
//! files written here, and only when asked (`--ignored`).

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::google::{
    cloud_code, token_body, FakeBrowser, FakeGoogle, Home, Script, ACCESS, ACCOUNT_ID, EMAIL,
    INVALID_GRANT, LOAD_CODE_ASSIST, MODELS, SUMMARY,
};
use common::{fixture_text, script, temp_dir};
use quota_station_core::antigravity::export;
use quota_station_core::antigravity::index::{self, Standing};
use quota_station_core::antigravity::login;
use quota_station_core::antigravity::pace::{Finish, Pace};
use quota_station_core::antigravity::quota::{self, Outcome, ReadError};
use quota_station_core::antigravity::token::TokenData;
use quota_station_core::antigravity::version::{AntigravityApp, Identity};

/// 2026-01-16 07:20 UTC, the fixtures' clock, and the cache fixture's `updatedAt`.
const NOW_MS: i64 = 1_768_548_000_000;
const MIN: i64 = 60_000;
const WAIT: Duration = Duration::from_secs(20);
const ARCH: &str = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
const PROJECT: &str = "fixture-project-1001";

fn no_antigravity() -> Identity {
    AntigravityApp::at(temp_dir("agy-none").join("Antigravity IDE.app")).identity()
}

/// Logs an account in through the fake Google at `now_ms`; returns its id.
fn log_in(home: &Home, google: &FakeGoogle, now_ms: i64) -> String {
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let now = move || now_ms / 1000;
    let flow = login::prepare(&deps).unwrap();
    login::complete(&deps, flow, &FakeBrowser::following(), WAIT, &now).unwrap().id
}

fn read_with(
    home: &Home,
    google: &FakeGoogle,
    identity: &Identity,
    id: &str,
    now_ms: i64,
) -> Result<Outcome, ReadError> {
    let (endpoints, hosts) = (google.endpoints(), google.hosts());
    let login = home.deps(&endpoints);
    let deps = quota::Deps { login: &login, hosts: &hosts, identity };
    quota::read_account(&deps, id, now_ms)
}

fn read(home: &Home, google: &FakeGoogle, id: &str, now_ms: i64) -> Result<Outcome, ReadError> {
    read_with(home, google, &no_antigravity(), id, now_ms)
}

fn cache_text(home: &Home, email: &str) -> Option<String> {
    std::fs::read_to_string(export::path(&home.app, email)).ok()
}

fn stored_token(home: &Home, id: &str) -> TokenData {
    serde_json::from_str(&home.item(id).expect("the app's item")).unwrap()
}

fn standing(home: &Home, id: &str) -> Option<Standing> {
    index::load_statuses(&home.app).accounts.remove(id)
}

fn ends(requests: &[common::google::Request]) -> Vec<&str> {
    requests.iter().map(|r| r.path.rsplit('/').next().unwrap()).collect()
}

/// A Google error body asking for a verification, with its link.
fn verify_body(status: u16) -> String {
    format!(
        r#"{{"error":{{"code":{status},"message":"Verify your account to continue.","status":"PERMISSION_DENIED","details":[{{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"VALIDATION_REQUIRED","metadata":{{"validation_url":"https://accounts.google.com/signin/continue?authuser=0&continue=https%3A%2F%2Fdevelopers.google.com"}}}}]}}}}"#
    )
}

#[test]
fn a_read_makes_cockpits_calls_and_writes_cockpits_cache() {
    let home = Home::new("agy-quota-read");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    google.clear();

    assert_eq!(read(&home, &google, &id, NOW_MS), Ok(Outcome::Read));

    // loadCodeAssist first (the token has no project yet), then the two
    // quota calls, all on the daily host.
    let calls = google.cloud_code();
    assert_eq!(ends(&calls), [LOAD_CODE_ASSIST, MODELS, SUMMARY]);
    assert!(calls.iter().all(|r| r.host() == "daily" && r.method == "POST"));

    let lca = &calls[0];
    let header = |r: &common::google::Request, k: &str| r.header(k).unwrap_or_default();
    assert_eq!(
        header(lca, "user-agent"),
        format!("antigravity/1.20.5 darwin/{ARCH} google-api-nodejs-client/10.3.0")
    );
    assert_eq!(header(lca, "x-goog-api-client"), "gl-node/22.21.1");
    assert_eq!(header(lca, "accept"), "*/*");
    assert_eq!(header(lca, "accept-encoding"), "gzip, deflate, br");
    assert_eq!(header(lca, "content-type"), "application/json");
    assert_eq!(header(lca, "authorization"), format!("Bearer {ACCESS}"));
    let platform = if ARCH == "arm64" { "DARWIN_ARM64" } else { "DARWIN_AMD64" };
    assert_eq!(
        lca.json(),
        serde_json::json!({
            "metadata": {
                "ideName": "antigravity",
                "ideType": "ANTIGRAVITY",
                "ideVersion": "1.20.5",
                "pluginVersion": env!("CARGO_PKG_VERSION"),
                "platform": platform,
                "updateChannel": "stable",
                "pluginType": "GEMINI",
            },
            "mode": "FULL_ELIGIBILITY_CHECK",
        })
    );

    for r in &calls[1..] {
        assert_eq!(header(r, "user-agent"), format!("antigravity/1.20.5 darwin/{ARCH}"));
        assert_eq!(header(r, "accept-encoding"), "gzip");
        assert_eq!(header(r, "content-type"), "application/json");
        assert_eq!(header(r, "authorization"), format!("Bearer {ACCESS}"));
        assert_eq!(r.header("x-goog-api-client"), None);
        assert_eq!(r.json(), serde_json::json!({ "project": PROJECT }));
    }

    // The cache file is the record Cockpit writes, byte for byte.
    let text = cache_text(&home, EMAIL).expect("the reading is cached");
    assert_eq!(text.trim_end(), fixture_text("cloud-code/cockpit-cache-record.json").trim_end());
    let file = export::path(&home.app, EMAIL);
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);

    // The project and the GCP ToS answer are kept with the token.
    let token = stored_token(&home, &id);
    assert_eq!(token.project_id.as_deref(), Some(PROJECT));
    assert_eq!(token.is_gcp_tos, Some(false));
    assert_eq!(token.access_token, ACCESS, "no refresh was due");
}

#[test]
fn the_user_agent_is_cockpits_with_and_without_antigravity_on_disk() {
    // No Antigravity: Cockpit's defaults.
    let none = no_antigravity();
    assert_eq!(none.cloud_code_user_agent(), format!("antigravity/1.20.5 darwin/{ARCH}"));
    assert_eq!(
        none.load_code_assist_user_agent(),
        format!("antigravity/1.20.5 darwin/{ARCH} google-api-nodejs-client/10.3.0")
    );

    // A bundle with product.json and google-auth-library.
    let root = temp_dir("agy-bundle").join("Antigravity IDE.app");
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "Contents/Resources/app/product.json",
        r#"{"nameShort":"Antigravity","ideVersion":"2.24.0","version":"1.107.0"}"#,
    );
    write(
        "Contents/Resources/app/node_modules/google-auth-library/package.json",
        r#"{"name":"google-auth-library","version":"9.15.1"}"#,
    );
    let installed = AntigravityApp::at(&root).identity();
    assert_eq!(installed.cloud_code_user_agent(), format!("antigravity/2.24.0 darwin/{ARCH}"));
    assert_eq!(
        installed.load_code_assist_user_agent(),
        format!("antigravity/2.24.0 darwin/{ARCH} google-api-nodejs-client/9.15.1")
    );

    // The strings reach the wire as they are.
    let home = Home::new("agy-quota-ua");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    google.clear();
    read_with(&home, &google, &installed, &id, NOW_MS).unwrap();
    let calls = google.cloud_code();
    assert_eq!(calls[0].header("user-agent").unwrap(), installed.load_code_assist_user_agent());
    assert_eq!(calls[0].json()["metadata"]["ideVersion"], "2.24.0");
    assert_eq!(calls[1].header("user-agent").unwrap(), installed.cloud_code_user_agent());

    // A product.json without a version falls through to Info.plist, read
    // with plutil as Cockpit reads it.
    write("Contents/Resources/app/product.json", r#"{"nameShort":"Antigravity"}"#);
    write(
        "Contents/Info.plist",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.example.fake</string>
<key>CFBundleShortVersionString</key><string>2.19.3</string>
</dict></plist>
"#,
    );
    let plist = AntigravityApp::at(&root).identity();
    assert_eq!(plist.cloud_code_user_agent(), format!("antigravity/2.19.3 darwin/{ARCH}"));
    assert_eq!(plist.google_client_version, "9.15.1");

    // The second place Cockpit looks, with `version` only.
    std::fs::remove_dir_all(root.join("Contents")).unwrap();
    write("resources/app/product.json", r#"{"version":"1.22.0"}"#);
    let other = AntigravityApp::at(&root).identity();
    assert_eq!(other.cloud_code_user_agent(), format!("antigravity/1.22.0 darwin/{ARCH}"));
    assert_eq!(other.google_client_version, "10.3.0");
}

#[test]
fn a_reading_younger_than_a_minute_is_used_without_asking_google() {
    let home = Home::new("agy-quota-ttl");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    assert_eq!(read(&home, &google, &id, NOW_MS), Ok(Outcome::Read));
    google.clear();

    assert_eq!(read(&home, &google, &id, NOW_MS + MIN - 1), Ok(Outcome::Cached));
    assert!(google.requests().is_empty(), "no call at all, not even a refresh");

    assert_eq!(read(&home, &google, &id, NOW_MS + MIN), Ok(Outcome::Read));
    // The token now has its project: loadCodeAssist is not asked again.
    assert_eq!(ends(&google.cloud_code()), [MODELS, SUMMARY]);
    let record = export::read(&home.app, EMAIL).unwrap();
    assert_eq!(record.updated_at, NOW_MS + MIN);
}

#[test]
fn a_gcp_tos_account_with_a_project_reads_from_prod() {
    let home = Home::new("agy-quota-prod");
    let google = FakeGoogle::start(Script {
        load_code_assist: (200, cloud_code("load-code-assist-gcp")),
        ..Script::default()
    });
    let id = log_in(&home, &google, NOW_MS);
    google.clear();
    read(&home, &google, &id, NOW_MS).unwrap();
    let calls = google.cloud_code();
    assert_eq!(calls.iter().map(|r| r.host()).collect::<Vec<_>>(), ["daily", "prod", "prod"]);
    assert_eq!(calls[1].json(), serde_json::json!({ "project": "fixture-gcp-project-2002" }));
    let token = stored_token(&home, &id);
    assert_eq!(token.is_gcp_tos, Some(true));
    assert_eq!(token.project_id.as_deref(), Some("fixture-gcp-project-2002"));
}

#[test]
fn a_401_keeps_the_last_reading_and_is_not_retried() {
    let home = Home::new("agy-quota-401");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    read(&home, &google, &id, NOW_MS).unwrap();
    let before = cache_text(&home, EMAIL).unwrap();
    google.clear();
    google.set(|s| s.models = (401, r#"{"error":{"code":401,"status":"UNAUTHENTICATED"}}"#.into()));

    let err = read(&home, &google, &id, NOW_MS + 2 * MIN).unwrap_err();
    assert_eq!(err.kind(), "unauthorized");
    assert_eq!(quota::finish(&Err(err)), Finish::Failed);
    assert_eq!(ends(&google.cloud_code()), [MODELS]);
    assert_eq!(cache_text(&home, EMAIL).unwrap(), before, "the last reading stays, with its age");
    assert_eq!(standing(&home, &id), None);
}

#[test]
fn a_403_stops_the_reads_until_a_new_login() {
    let home = Home::new("agy-quota-403");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    read(&home, &google, &id, NOW_MS).unwrap();
    google.clear();
    google.set(|s| s.models = (403, verify_body(403)));

    assert_eq!(read(&home, &google, &id, NOW_MS + 2 * MIN), Err(ReadError::Forbidden));
    assert_eq!(ends(&google.cloud_code()), [MODELS], "no retry");
    let Some(Standing::Forbidden { url: Some(url), .. }) = standing(&home, &id) else {
        panic!("forbidden with Google's link")
    };
    let url = url::Url::parse(&url).unwrap();
    assert_eq!(url.host_str(), Some("accounts.google.com"));
    assert!(url.query_pairs().any(|(k, v)| k == "authuser" && v == EMAIL));

    // Not read again, and the pace skips it.
    google.clear();
    assert_eq!(read(&home, &google, &id, NOW_MS + 30 * MIN), Err(ReadError::Forbidden));
    assert!(google.requests().is_empty());
    let (_, candidates) = quota::candidates(&home.app).unwrap();
    assert!(candidates[0].skip);

    // A new login answers it.
    google.set(|s| s.models = (200, cloud_code("fetch-available-models")));
    log_in(&home, &google, NOW_MS + 31 * MIN);
    assert_eq!(standing(&home, &id), None);
    assert_eq!(read(&home, &google, &id, NOW_MS + 31 * MIN), Ok(Outcome::Read));
}

#[test]
fn a_429_or_5xx_is_not_retried_keeps_the_last_reading_and_backs_off() {
    let home = Home::new("agy-quota-429");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    read(&home, &google, &id, NOW_MS).unwrap();
    let before = cache_text(&home, EMAIL).unwrap();

    for (status, kind) in [(429, "rateLimited"), (503, "server"), (500, "server")] {
        google.clear();
        google.set(|s| {
            s.models = (status, r#"{"error":{"code":429,"status":"RESOURCE_EXHAUSTED"}}"#.into())
        });
        let result = read(&home, &google, &id, NOW_MS + 2 * MIN);
        let err = result.clone().unwrap_err();
        assert_eq!(err.kind(), kind);
        assert!(err.throttled());
        assert_eq!(quota::finish(&result), Finish::Throttled);
        assert_eq!(ends(&google.cloud_code()), [MODELS], "{status}: one try");
        assert_eq!(cache_text(&home, EMAIL).unwrap(), before);
    }

    // The same on the summary call.
    google.clear();
    google.set(|s| {
        s.models = (200, cloud_code("fetch-available-models"));
        s.summary = (429, "{}".into());
    });
    let err = read(&home, &google, &id, NOW_MS + 2 * MIN).unwrap_err();
    assert!(err.throttled());
    assert_eq!(ends(&google.cloud_code()), [MODELS, SUMMARY]);
    assert_eq!(cache_text(&home, EMAIL).unwrap(), before);

    // The pace then waits 2, 4, 8… minutes before the next try.
    let mut pace = Pace::new();
    pace.started(&id, NOW_MS);
    pace.finished(&id, quota::finish(&Err(err.clone())), NOW_MS);
    assert_eq!(pace.waiting_until(&id, NOW_MS), Some(NOW_MS + 2 * MIN));
    pace.finished(&id, quota::finish(&Err(err)), NOW_MS);
    assert_eq!(pace.waiting_until(&id, NOW_MS), Some(NOW_MS + 4 * MIN));
}

#[test]
fn network_errors_are_tried_three_times_a_second_apart() {
    let home = Home::new("agy-quota-net");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    read(&home, &google, &id, NOW_MS).unwrap();
    let before = cache_text(&home, EMAIL).unwrap();

    // Two dropped connections, then an answer.
    google.clear();
    google.set(|s| {
        s.queue = vec![(MODELS.into(), 0, String::new()), (MODELS.into(), 0, String::new())]
    });
    let started = Instant::now();
    assert_eq!(read(&home, &google, &id, NOW_MS + 2 * MIN), Ok(Outcome::Read));
    assert!(started.elapsed() >= Duration::from_secs(2), "{:?}", started.elapsed());
    assert_eq!(ends(&google.cloud_code()), [MODELS, MODELS, MODELS, SUMMARY]);

    // Three dropped: the last reading stays.
    let kept = cache_text(&home, EMAIL).unwrap();
    assert_ne!(kept, before);
    google.clear();
    google.set(|s| s.queue = vec![(MODELS.into(), 0, String::new()); 3]);
    let err = read(&home, &google, &id, NOW_MS + 4 * MIN).unwrap_err();
    assert_eq!(err.kind(), "network");
    assert!(!err.throttled());
    assert_eq!(ends(&google.cloud_code()), [MODELS, MODELS, MODELS]);
    assert_eq!(cache_text(&home, EMAIL).unwrap(), kept);
}

#[test]
fn a_summary_asking_for_a_verification_keeps_the_last_reading_and_says_so() {
    let home = Home::new("agy-quota-verify");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    read(&home, &google, &id, NOW_MS).unwrap();
    let before = cache_text(&home, EMAIL).unwrap();
    google.set(|s| s.summary = (403, verify_body(403)));

    let err = read(&home, &google, &id, NOW_MS + 2 * MIN).unwrap_err();
    assert_eq!(err.kind(), "verification");
    assert!(matches!(standing(&home, &id), Some(Standing::NeedsVerification { url: Some(_), .. })));
    assert_eq!(cache_text(&home, EMAIL).unwrap(), before);
    // Still read, as Cockpit still reads it; a full reading clears it.
    let (_, candidates) = quota::candidates(&home.app).unwrap();
    assert!(!candidates[0].skip);
    google.set(|s| s.summary = (200, cloud_code("retrieve-user-quota-summary")));
    assert_eq!(read(&home, &google, &id, NOW_MS + 4 * MIN), Ok(Outcome::Read));
    assert_eq!(standing(&home, &id), None);
}

#[test]
fn an_account_google_logged_out_is_not_read() {
    let home = Home::new("agy-quota-relogin");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    google.set(|s| s.refresh = (400, INVALID_GRANT.into()));
    google.clear();

    // Under five minutes left on the token: a refresh first, refused.
    let late = NOW_MS + 3_400_000;
    assert_eq!(read(&home, &google, &id, late), Err(ReadError::NeedsLogin));
    assert_eq!(google.to("/token").len(), 1);
    assert!(google.cloud_code().is_empty());
    assert!(matches!(standing(&home, &id), Some(Standing::NeedsLogin { .. })));

    google.clear();
    assert_eq!(read(&home, &google, &id, late + 10 * MIN), Err(ReadError::NeedsLogin));
    assert!(google.requests().is_empty(), "skipped without a call");
}

#[test]
fn a_refresh_that_is_due_comes_first_and_is_kept() {
    let home = Home::new("agy-quota-refresh");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    google.clear();
    read(&home, &google, &id, NOW_MS + 3_400_000).unwrap();
    let paths: Vec<String> = google.requests().iter().map(|r| r.path.clone()).collect();
    assert_eq!(paths[0], "/token");
    let calls = google.cloud_code();
    assert_eq!(calls[0].header("authorization").unwrap(), "Bearer fake-access-2");
    assert_eq!(stored_token(&home, &id).access_token, "fake-access-2");
}

#[test]
fn the_client_file_is_needed_only_for_a_refresh() {
    let home = Home::new("agy-quota-client");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    std::fs::remove_file(home.app.path().join("oauth-client.json")).unwrap();
    assert_eq!(read(&home, &google, &id, NOW_MS), Ok(Outcome::Read));
    let err = read(&home, &google, &id, NOW_MS + 3_400_000).unwrap_err();
    assert_eq!(err.kind(), "client");
    assert_eq!(cache_text(&home, EMAIL).map(|t| t.contains("1768548000000")), Some(true));
}

#[test]
fn accounts_are_read_one_at_a_time_the_one_in_use_first() {
    let home = Home::new("agy-quota-turn");
    let google = FakeGoogle::start(Script::default());
    let first = log_in(&home, &google, NOW_MS);
    google.set(|s| {
        s.userinfo.1 = r#"{"id":"1000002","email":"ultra.two@example.com"}"#.into();
        s.exchange.1 = token_body("fake-access-b", Some("fake-refresh-b"));
    });
    let second = log_in(&home, &google, NOW_MS);
    google.clear();
    google.set(|s| {
        s.delay = Duration::from_millis(150);
        // The first summary asked is the second account's.
        s.queue = vec![(SUMMARY.into(), 200, cloud_code("retrieve-user-quota-summary-2"))];
    });

    let (index, candidates) = quota::candidates(&home.app).unwrap();
    assert_eq!(quota::active_id(&index, None), Some(first.clone()), "the index's current account");
    let active = quota::active_id(&index, Some(2));
    assert_eq!(active, Some(second.clone()), "quota-axi's activeAccount 2");
    let mut pace = Pace::new();
    pace.cycle();
    let due = pace.take(&candidates, active.as_deref(), NOW_MS);
    assert_eq!(due, [second.clone(), first.clone()]);

    let identity = no_antigravity();
    let (endpoints, hosts) = (google.endpoints(), google.hosts());
    let login = home.deps(&endpoints);
    let deps = quota::Deps { login: &login, hosts: &hosts, identity: &identity };
    let mut log = Vec::new();
    quota::read_in_turn(&deps, &due, &|| NOW_MS, &mut |id, result| {
        log.push((id.to_string(), result.map(|r| r.clone().map_err(|e| e.kind()))));
    });
    assert_eq!(
        log,
        [
            (second.clone(), None),
            (second.clone(), Some(Ok(Outcome::Read))),
            (first.clone(), None),
            (first.clone(), Some(Ok(Outcome::Read))),
        ]
    );
    assert_eq!(google.most_at_once.load(std::sync::atomic::Ordering::SeqCst), 1);
    let tokens: Vec<String> =
        google.cloud_code().iter().map(|r| r.header("authorization").unwrap()).collect();
    assert!(tokens[..3].iter().all(|t| t == "Bearer fake-access-b"), "{tokens:?}");
    assert!(tokens[3..].iter().all(|t| t == &format!("Bearer {ACCESS}")), "{tokens:?}");
    let two = export::read(&home.app, "ultra.two@example.com").unwrap();
    assert_eq!(two.payload["quota_summary"]["groups"][1]["buckets"][0]["remainingFraction"], 0.05);
}

#[test]
fn logging_out_removes_the_reading() {
    let home = Home::new("agy-quota-logout");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    read(&home, &google, &id, NOW_MS).unwrap();
    assert!(cache_text(&home, EMAIL).is_some());
    let endpoints = google.endpoints();
    login::logout(&home.deps(&endpoints), &id).unwrap();
    assert!(cache_text(&home, EMAIL).is_none());
    assert!(quota::candidates(&home.app).unwrap().1.is_empty());
}

#[test]
fn the_cache_layout_is_the_one_quota_axi_reads() {
    let home = Home::new("agy-quota-layout");
    let google = FakeGoogle::start(Script::default());
    let id = log_in(&home, &google, NOW_MS);
    assert_eq!(id, ACCOUNT_ID);
    read(&home, &google, &id, NOW_MS).unwrap();
    // The three things quota-axi opens, under the folder it is pointed at.
    let dir = home.app.path().join("antigravity");
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("accounts.json")).unwrap()).unwrap();
    assert_eq!(index["version"], "2.0");
    assert_eq!(index["accounts"][0]["email"], EMAIL);
    // `printf '%s' someone@example.com | shasum -a 256`
    let cache = dir.join(
        "cache/quota_api_v1_desktop/authorized/\
         72497f475e4f76d0b28f57c73a084ece576d170874eba3ee2609d9afe4b71aab.json",
    );
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(cache).unwrap()).unwrap();
    assert_eq!(record["version"], 1);
    assert_eq!(record["source"], "authorized");
    assert_eq!(record["email"], EMAIL);
    assert!(record["updatedAt"].is_i64());
    assert!(record["payload"]["quota_summary"]["groups"].is_array());
}

/// Where the built quota-axi fork is (`dist/bin/quota-axi.js`), and node.
fn quota_axi() -> (PathBuf, PathBuf) {
    let script = std::env::var_os("QUOTA_STATION_QUOTA_AXI")
        .map(PathBuf::from)
        .expect("QUOTA_STATION_QUOTA_AXI names the quota-axi fork's dist/bin/quota-axi.js");
    let node = std::env::var_os("QUOTA_STATION_NODE").map(PathBuf::from).unwrap_or_else(|| {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join("node"))
            .find(|p| p.is_file())
            .expect("node on PATH")
    });
    (script, node)
}

/// A PATH with only node and fakes: an `agy` that answers `/quota` with
/// account 1's reading, and `ps` and `lsof` that find nothing, so quota-axi
/// never looks at this Mac's own Antigravity.
fn fake_bin(dir: &Path, node: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(node, bin.join("node")).unwrap();
    let summary: serde_json::Value =
        serde_json::from_str(&cloud_code("retrieve-user-quota-summary")).unwrap();
    let answer = serde_json::json!({ "command": { "name": "/quota", "data": summary } });
    std::fs::write(dir.join("agy-quota.json"), format!("{answer}\n")).unwrap();
    script(
        &bin.join("agy"),
        &format!(
            "[ \"$1\" = -p ] && [ \"$2\" = /quota ] || exit 2\nwhile IFS= read -r line; do printf '%s\\n' \"$line\"; done < '{}'\n",
            dir.join("agy-quota.json").display()
        ),
    );
    script(&bin.join("ps"), "echo '  PID COMMAND'\n");
    script(&bin.join("lsof"), "exit 1\n");
    bin
}

#[test]
#[ignore = "runs the quota-axi fork: set QUOTA_STATION_QUOTA_AXI to its dist/bin/quota-axi.js"]
fn quota_axi_reads_both_accounts_from_the_apps_folder() {
    let (quota_axi, node) = quota_axi();
    let home = Home::new("agy-quota-axi");
    let google = FakeGoogle::start(Script::default());
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
    let first = log_in(&home, &google, now_ms);
    google.set(|s| {
        s.userinfo.1 = r#"{"id":"1000002","email":"ultra.two@example.com"}"#.into();
        s.exchange.1 = token_body("fake-access-b", Some("fake-refresh-b"));
        s.queue = vec![(SUMMARY.into(), 200, cloud_code("retrieve-user-quota-summary-2"))];
    });
    let second = log_in(&home, &google, now_ms);
    assert_eq!(read(&home, &google, &second, now_ms), Ok(Outcome::Read));
    assert_eq!(read(&home, &google, &first, now_ms), Ok(Outcome::Read));

    let bin = fake_bin(&home.home, &node);
    let run = |dir: Option<&Path>| -> serde_json::Value {
        let mut cmd = std::process::Command::new(&node);
        cmd.arg(&quota_axi)
            .args(["--provider", "agy", "--json"])
            .env_clear()
            .env("HOME", &home.home)
            .env("PATH", &bin)
            .env("TMPDIR", home.home.join("tmp"));
        std::fs::create_dir_all(home.home.join("tmp")).unwrap();
        if let Some(dir) = dir {
            cmd.env("QUOTA_AXI_COCKPIT_DIR", dir);
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        eprintln!("quota-axi {}:\n{text}", if dir.is_some() { "(named dir)" } else { "(default)" });
        serde_json::from_str(&text).expect("quota-axi printed JSON")
    };

    for dir in [Some(home.app.path().join("antigravity")), None] {
        let report = run(dir.as_deref());
        let agy = &report["providers"][0];
        assert_eq!(agy["provider"], "agy");
        assert_eq!(agy["state"]["status"], "fresh", "the fake agy answered");
        let cockpit = &agy["cockpit"];
        assert_eq!(cockpit["status"], "ok", "{cockpit}");
        let source = if dir.is_some() { serde_json::Value::Null } else { "quota-station".into() };
        assert_eq!(cockpit["source"], source);
        assert_eq!(cockpit["activeAccount"], 1);
        assert_eq!(cockpit["activeBasis"], "reading");
        let accounts = cockpit["accounts"].as_array().unwrap();
        assert_eq!(accounts.len(), 2);
        let scope = |a: usize, s: &str| {
            accounts[a]["scopes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["scope"] == s)
                .map(|x| x["effectivePercentRemaining"].as_f64().unwrap())
                .unwrap()
        };
        for (a, gemini, claude) in [(0, 82.0, 64.0), (1, 25.0, 5.0)] {
            assert_eq!(accounts[a]["account"], a + 1);
            assert_eq!(accounts[a]["status"], "fresh");
            assert_eq!(scope(a, "gemini"), gemini);
            assert_eq!(scope(a, "claude_gpt"), claude);
        }
        // The default report carries no email.
        assert!(!report.to_string().contains("@example.com"));
    }
}
