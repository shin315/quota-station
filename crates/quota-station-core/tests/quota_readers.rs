// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's own quota readers end to end: Codex against a local fake
//! ChatGPT, Antigravity against a fake `agy` and a fake language server,
//! Claude Code against a fake Keychain and a fake Anthropic, each in a
//! temporary home. Every login is made up; nothing here reaches a real
//! service, the Keychain or this Mac's own Antigravity, and no login is
//! ever refreshed or rewritten.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use common::accounts::{credential, in_hours, oauth, FakeAnthropic, Mac};
use common::http::FakeHttp;
use common::os::FakeProcesses;
#[cfg(unix)]
use common::script;
use common::temp_dir;
use quota_station_core::app_dir::AppDir;
use quota_station_core::quota::cache::Cache;
use quota_station_core::quota::claude::{self as live, Keychain, Live};
use quota_station_core::quota::{agy, codex, ProviderId, Response};
use quota_station_core::snapshot::{Keychain as Granted, SectionStatus};
use quota_station_core::Snapshot;
use serde_json::json;
use time::OffsetDateTime;

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn row<'a>(report: &'a Response, provider: &str) -> &'a quota_station_core::quota::Provider {
    report.primary(provider).expect("a row")
}

fn left(report: &Response, provider: &str, id: &str) -> Option<f64> {
    row(report, provider).windows.iter().find(|w| w.id == id)?.percent_remaining
}

// ===== Codex =====

const WHAM: &str = "/backend-api/wham/usage";
const CODEX_USAGE: &str = "/backend-api/codex/usage";

/// A made-up ChatGPT access token whose own time ends at `exp`.
fn jwt(exp: i64) -> String {
    let claims = json!({ "exp": exp, "https://api.openai.com/auth/account_id": "acct-fake" });
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
    format!("fake-header.{payload}.fake-signature")
}

/// Codex's folder with a ChatGPT login of `access`.
fn codex_home(dir: &Path, access: &str) -> PathBuf {
    let home = dir.join(".codex");
    std::fs::create_dir_all(&home).unwrap();
    let auth = json!({
        "OPENAI_API_KEY": null,
        "tokens": { "access_token": access, "refresh_token": "fake-refresh", "account_id": "acct-fake" },
        "last_refresh": "2030-01-01T00:00:00Z"
    });
    std::fs::write(home.join("auth.json"), auth.to_string()).unwrap();
    home
}

fn chatgpt() -> FakeHttp {
    let server = FakeHttp::start();
    let usage = json!({
        "plan_type": "pro",
        "rate_limit": {
            "primary_window": { "used_percent": 25, "limit_window_seconds": 18000, "reset_at": 1_900_000_000 },
            "secondary_window": { "used_percent": 40, "limit_window_seconds": 604800, "reset_at": 1_900_500_000 }
        }
    });
    server.answer(WHAM, 200, &usage.to_string());
    server
}

fn read_codex(home: &Path, server: &FakeHttp, app: &AppDir, max_age: i64) -> Response {
    let cache = Cache::new(app, ProviderId::Codex);
    codex::read(home, &codex::Endpoints::fake(&server.base), &cache, max_age, now())
}

#[test]
fn codex_reads_fresh_then_shows_the_last_reading_as_stale() {
    let dir = temp_dir("readers-codex");
    let app = AppDir::new(dir.join("app"));
    let access = jwt(now().unix_timestamp() + 3600);
    let home = codex_home(&dir, &access);
    let auth = std::fs::read(home.join("auth.json")).unwrap();
    let server = chatgpt();

    let fresh = read_codex(&home, &server, &app, 120);
    assert_eq!(row(&fresh, "codex").state.status, "fresh");
    assert_eq!(left(&fresh, "codex", "five_hour"), Some(75.0));
    assert_eq!(left(&fresh, "codex", "weekly"), Some(60.0));
    let asked = &server.requests()[0];
    assert_eq!(asked.header("authorization"), Some(format!("Bearer {access}")));
    assert_eq!(asked.header("chatgpt-account-id").as_deref(), Some("acct-fake"));

    // Within two minutes the same login's reading answers without asking.
    read_codex(&home, &server, &app, 120);
    assert_eq!(server.requests().len(), 1);

    // ChatGPT fails: the last numbers stay, marked stale.
    server.answer(WHAM, 500, "{}");
    server.answer(CODEX_USAGE, 503, "{}");
    let stale = read_codex(&home, &server, &app, 0);
    assert_eq!(row(&stale, "codex").state.status, "stale");
    assert_eq!(left(&stale, "codex", "five_hour"), Some(75.0));

    // Only the usage was asked, and the login is as it was.
    assert!(server.requests().iter().all(|r| r.path == WHAM || r.path == CODEX_USAGE));
    assert_eq!(std::fs::read(home.join("auth.json")).unwrap(), auth);
}

/// A made-up ChatGPT sign-in of `user`: an `id_token` naming them, an
/// access token whose own time ends at `exp`.
fn signed_in(user: &str, exp: i64) -> String {
    let b64 = |v: serde_json::Value| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v.to_string())
    };
    let id = format!(
        "h.{}.s",
        b64(json!({ "email": format!("{user}@example.com"),
            "https://api.openai.com/auth": { "chatgpt_user_id": format!("user-{user}"), "chatgpt_account_id": format!("ws-{user}") } }))
    );
    let access = format!("h.{}.s", b64(json!({ "exp": exp, "who": user })));
    json!({ "tokens": { "id_token": id, "access_token": access, "refresh_token": "fake-refresh", "account_id": format!("ws-{user}") } })
        .to_string()
}

#[test]
fn every_kept_codex_account_is_read_in_its_own_folder_and_a_waiting_expired_one_is_not_asked() {
    use quota_station_core::codex::accounts::Store;
    use quota_station_core::codex::platform::Unix;
    let dir = temp_dir("readers-codex-accounts");
    let app = AppDir::new(dir.join("app"));
    let home = dir.join(".codex");
    std::fs::create_dir_all(&home).unwrap();
    let soon = now().unix_timestamp() + 3600;
    std::fs::write(home.join("auth.json"), signed_in("ann", soon)).unwrap();
    let platform = Unix::default();
    let store = Store::new(&app, &home, &platform);
    store.list(now()).unwrap();
    for (user, exp) in [("bob", now().unix_timestamp() - 60), ("cat", soon)] {
        let pending = store.begin_login().unwrap();
        std::fs::write(pending.join("auth.json"), signed_in(user, exp)).unwrap();
        store.finish_login(&pending, None, now()).unwrap();
    }
    let server = chatgpt();

    let report = codex::read_accounts(
        &app,
        &home,
        &platform,
        &codex::Endpoints::fake(&server.base),
        0,
        now(),
    );
    let keys: Vec<Vec<String>> = report.providers.iter().map(|p| p.account_keys.clone()).collect();
    assert_eq!(keys, [vec!["codex-home", "codex:1"], vec!["codex:2"], vec!["codex:3"]]);
    let status: Vec<&str> = report.providers.iter().map(|p| p.state.status.as_str()).collect();
    assert_eq!(status, ["fresh", "unavailable", "fresh"]);
    assert_eq!(report.providers[1].state.error.as_deref(), Some(codex::TOKEN_EXPIRED));
    // Two requests, one per account asked; bob's ran-out token was not sent.
    let sent: Vec<String> =
        server.requests().iter().filter_map(|r| r.header("chatgpt-account-id")).collect();
    assert_eq!(sent.len(), 2);
    assert!(!sent.contains(&"ws-bob".to_string()), "{sent:?}");

    let snapshot = Snapshot::build(None, None).with_codex(Some(&report));
    let numbers: Vec<(u32, bool)> =
        snapshot.codex.accounts.iter().map(|a| (a.number, a.active)).collect();
    assert_eq!(numbers, [(1, true), (3, false)]);
    assert_eq!(snapshot.codex.status, SectionStatus::Ok);
}

#[test]
fn an_expired_or_refused_codex_login_is_never_refreshed() {
    let dir = temp_dir("readers-codex-expired");
    let app = AppDir::new(dir.join("app"));
    let server = chatgpt();
    let home = codex_home(&dir, &jwt(now().unix_timestamp() + 3600));
    assert_eq!(row(&read_codex(&home, &server, &app, 0), "codex").state.status, "fresh");

    // The token's own time ran out: Codex renews it when it runs; the app
    // shows the last reading as stale meanwhile.
    let expired = codex_home(&dir, &jwt(now().unix_timestamp() - 60));
    let auth = std::fs::read(expired.join("auth.json")).unwrap();
    server.answer(WHAM, 401, "{}");
    server.answer(CODEX_USAGE, 401, "{}");
    let report = read_codex(&expired, &server, &app, 0);
    let r = row(&report, "codex");
    assert_eq!(r.state.status, "stale");
    assert_eq!(r.state.error.as_deref(), Some(codex::TOKEN_EXPIRED));
    assert_eq!(std::fs::read(expired.join("auth.json")).unwrap(), auth, "never rewritten");

    // A token still in its time refused: a sign-in, and the old reading goes.
    let refused = codex_home(&dir, &jwt(now().unix_timestamp() + 3600));
    let report = read_codex(&refused, &server, &app, 0);
    let r = row(&report, "codex");
    assert_eq!((r.state.status.as_str(), r.windows.len()), ("auth_required", 0));
    assert_eq!(r.state.error.as_deref(), Some(codex::SIGN_IN_REQUIRED));
    assert!(Cache::new(&app, ProviderId::Codex).last().is_none());

    assert!(server.requests().iter().all(|r| r.path == WHAM || r.path == CODEX_USAGE));
}

#[test]
fn a_mac_without_a_codex_login_has_no_codex_row() {
    let dir = temp_dir("readers-codex-missing");
    let app = AppDir::new(dir.join("app"));
    let server = chatgpt();
    let report = read_codex(&dir.join(".codex"), &server, &app, 0);
    assert!(row(&report, "codex").not_set_up);
    assert!(server.requests().is_empty());
    let s = Snapshot::build(None, None).with_codex(Some(&report));
    assert_eq!(s.codex.status, SectionStatus::NotSetUp);
}

// ===== Antigravity =====

const SUMMARY: &str = "/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary";
const USER_STATUS: &str = "/exa.language_server_pb.LanguageServerService/GetUserStatus";

fn summary() -> serde_json::Value {
    json!({ "groups": [
        { "displayName": "Gemini", "buckets": [
            { "bucketId": "gemini-5h", "remainingFraction": 0.8, "resetTime": "2030-01-15T14:00:00Z" },
            { "bucketId": "gemini-weekly", "remainingFraction": 0.6, "resetTime": "2030-01-20T00:00:00Z" }
        ]},
        { "displayName": "Claude and GPT", "buckets": [
            { "bucketId": "3p-5h", "remainingFraction": 1, "resetTime": "2030-01-15T15:00:00Z" },
            { "bucketId": "3p-weekly", "remainingFraction": 0.9, "resetTime": "2030-01-21T00:00:00Z" }
        ]}
    ]})
}

/// An `agy` that records its arguments, tries to open a browser, and prints
/// `dir/agy.json` (or fails when there is none); no process runs.
#[cfg(unix)]
fn agy_cli(dir: &Path) -> agy::Tools {
    let bin = dir.join("bin");
    let d = dir.display();
    agy::Tools {
        agy: Some(script(
            &bin.join("agy"),
            &format!(
                "printf '%s\\n' \"$@\" > '{d}/agy-args'\nopen 'https://example.invalid/login' && : > '{d}/opened'\n[ -f '{d}/agy.json' ] || exit 2\ncat '{d}/agy.json'\n"
            ),
        )),
        processes: Arc::new(FakeProcesses::none()),
    }
}

/// No `agy` command, and a running agy whose language server listens on
/// `port`.
fn agy_running(port: u16) -> agy::Tools {
    let processes = FakeProcesses::none().with_own(4242, "/usr/local/bin/agy --resume", &[port]);
    agy::Tools { agy: None, processes: Arc::new(processes) }
}

fn read_agy(tools: &agy::Tools, app: &AppDir) -> Response {
    agy::read(tools, &Cache::new(app, ProviderId::Agy), 0, now())
}

// The fake `agy` is a shell script.
#[cfg(unix)]
#[test]
fn agy_quota_reads_its_windows_and_never_opens_a_browser() {
    let dir = temp_dir("readers-agy-cli");
    let app = AppDir::new(dir.join("app"));
    let tools = agy_cli(&dir);
    let answer = json!({ "command": { "name": "/quota", "data": summary() } });
    std::fs::write(dir.join("agy.json"), answer.to_string()).unwrap();

    let report = read_agy(&tools, &app);
    assert_eq!(row(&report, "agy").state.status, "fresh");
    assert_eq!(left(&report, "agy", "gemini_5h"), Some(80.0));
    assert_eq!(left(&report, "agy", "gemini_weekly"), Some(60.0));
    assert_eq!(left(&report, "agy", "claude_gpt_weekly"), Some(90.0));
    let args = std::fs::read_to_string(dir.join("agy-args")).unwrap();
    assert_eq!(args.lines().collect::<Vec<_>>(), ["-p", "/quota", "--output-format", "json"]);
    assert!(!dir.join("opened").exists(), "`open` only fails for agy");
}

#[test]
fn the_running_language_server_names_its_account_by_a_hash_and_goes_stale() {
    let dir = temp_dir("readers-agy-loopback");
    let app = AppDir::new(dir.join("app"));
    let server = FakeHttp::start();
    server.answer(SUMMARY, 200, &summary().to_string());
    server.answer(USER_STATUS, 200, r#"{"userStatus":{"email":"Lan.Ultra@example.com"}}"#);
    let tools = agy_running(server.port);

    let report = read_agy(&tools, &app);
    let r = row(&report, "agy");
    assert_eq!(r.state.status, "fresh");
    assert_eq!(left(&report, "agy", "gemini_5h"), Some(80.0));
    assert_eq!(r.account_keys, [agy::account_hash("lan.ultra@example.com")]);
    let cached = std::fs::read_to_string(app.path().join("quota/agy.json")).unwrap();
    for text in [serde_json::to_string(&report).unwrap(), cached] {
        assert!(!text.to_lowercase().contains("example.com"), "{text}");
    }

    // The language server fails: the last numbers stay, marked stale.
    server.answer_all(503, "{}");
    let report = read_agy(&tools, &app);
    assert_eq!(row(&report, "agy").state.status, "stale");
    assert_eq!(left(&report, "agy", "gemini_5h"), Some(80.0));
}

#[test]
fn a_signed_out_antigravity_shows_a_sign_in_and_drops_the_old_reading() {
    let dir = temp_dir("readers-agy-signed-out");
    let app = AppDir::new(dir.join("app"));
    let server = FakeHttp::start();
    server.answer(SUMMARY, 200, &summary().to_string());
    let tools = agy_running(server.port);
    assert_eq!(row(&read_agy(&tools, &app), "agy").state.status, "fresh");

    server.answer_all(401, "{}");
    server.answer(USER_STATUS, 401, "{}");
    let report = read_agy(&tools, &app);
    let r = row(&report, "agy");
    assert_eq!(r.state.status, "auth_required");
    assert_eq!(r.state.error.as_deref(), Some("Antigravity sign-in required"));
    assert!(r.windows.is_empty());
    assert!(Cache::new(&app, ProviderId::Agy).last().is_none());
}

#[test]
fn a_mac_without_antigravity_has_no_antigravity_row() {
    let dir = temp_dir("readers-agy-missing");
    let app = AppDir::new(dir.join("app"));
    let tools = agy::Tools { agy: None, processes: Arc::new(FakeProcesses::none()) };
    let report = read_agy(&tools, &app);
    let r = row(&report, "agy");
    assert!(r.not_set_up);
    assert_eq!(r.state.error.as_deref(), Some(agy::NOT_RUNNING));
    assert_eq!(Snapshot::build(None, Some(&report)).agy.status, SectionStatus::NotSetUp);
}

// ===== Claude Code =====

/// Claude Code signed in with `credential`; Anthropic answers its token
/// with 30 % of five hours and 12 % of the week used.
fn claude_mac(tag: &str, login: &str) -> (Mac, FakeAnthropic, AppDir) {
    let mac = Mac::new(tag);
    mac.log_in(login, &oauth("lan@team.example", "uuid-1", "org-team"));
    let anthropic = FakeAnthropic::start();
    anthropic.usage("acc-live", 30.0, Some("2030-01-16T09:00:00Z"), Some(12.0), None);
    let app = AppDir::new(mac.home.join("app"));
    (mac, anthropic, app)
}

fn read_claude(mac: &Mac, anthropic: &FakeAnthropic, app: &AppDir, mode: Keychain) -> Response {
    let endpoints = anthropic.endpoints();
    let l = Live { paths: &mac.paths, security: &*mac.store.security, endpoints: &endpoints, app };
    live::read(&l, mode, &Cache::new(app, ProviderId::Claude), 0, now())
}

#[test]
fn claude_code_reads_fresh_then_stale_and_never_refreshes_its_login() {
    let login = credential("acc-live", "ref-live", in_hours(5.0));
    let (mac, anthropic, app) = claude_mac("readers-claude", &login);

    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    assert_eq!(row(&report, "claude").state.status, "fresh");
    assert_eq!(left(&report, "claude", "five_hour"), Some(70.0));
    assert_eq!(left(&report, "claude", "seven_day"), Some(88.0));
    let s = Snapshot::build(Some(&report), None);
    assert_eq!(s.keychain, Granted::Granted);
    assert_eq!(s.claude.active().unwrap().lines[0].percent, Some(70.0));

    anthropic.usage_status("acc-live", 500, &[]);
    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    assert_eq!(row(&report, "claude").state.status, "stale");
    assert_eq!(left(&report, "claude", "five_hour"), Some(70.0));

    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(mac.live_credential().as_deref(), Some(login.as_str()));
    assert!(!mac.security_calls().contains("add-generic-password"));
}

#[test]
fn an_expired_claude_login_waits_for_claude_code_and_a_refused_one_asks_for_a_sign_in() {
    let (mac, anthropic, app) =
        claude_mac("readers-claude-expired", &credential("acc-live", "ref-live", in_hours(5.0)));
    assert_eq!(
        row(&read_claude(&mac, &anthropic, &app, Keychain::Quiet), "claude").state.status,
        "fresh"
    );

    // Its own time ran out: Claude Code renews it; the last reading stands.
    let expired = credential("acc-old", "ref-old", in_hours(-1.0));
    mac.log_in(&expired, &oauth("lan@team.example", "uuid-1", "org-team"));
    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    let r = row(&report, "claude");
    assert_eq!(r.state.status, "stale");
    assert_eq!(r.state.error.as_deref(), Some(live::TOKEN_EXPIRED));
    assert_eq!(mac.live_credential().as_deref(), Some(expired.as_str()));

    // A token in its time refused: a sign-in, and the old reading goes.
    mac.log_in(
        &credential("acc-gone", "ref-gone", in_hours(5.0)),
        &oauth("x@team.example", "u", "o"),
    );
    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    let r = row(&report, "claude");
    assert_eq!(r.state.status, "auth_required");
    assert_eq!(r.state.error.as_deref(), Some(live::SIGN_IN_REQUIRED));
    assert!(Cache::new(&app, ProviderId::Claude).last().is_none());
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn a_keychain_that_holds_back_waits_for_the_owner() {
    let (mac, anthropic, app) =
        claude_mac("readers-claude-keychain", &credential("acc-live", "ref-live", in_hours(5.0)));
    assert_eq!(
        row(&read_claude(&mac, &anthropic, &app, Keychain::Quiet), "claude").state.status,
        "fresh"
    );

    // macOS refuses the quiet read: the last numbers stay, and the row asks
    // for the owner's permission.
    mac.keychain.fail_find(true);
    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    let r = row(&report, "claude");
    assert_eq!(r.state.status, "stale");
    assert_eq!(r.state.reason.as_deref(), Some(live::ACCESS_REQUIRED));
    assert_eq!(Snapshot::build(Some(&report), None).keychain, Granted::Required);
    assert!(!live::hold_lifted(&app));

    // Allowed again in macOS, the quiet reads still only check the item is
    // there until the owner presses the button.
    mac.keychain.fail_find(false);
    let asked = anthropic.to("/api/oauth/usage").len();
    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    assert_eq!(row(&report, "claude").state.reason.as_deref(), Some(live::ACCESS_REQUIRED));
    assert_eq!(anthropic.to("/api/oauth/usage").len(), asked, "no token was read");

    let report = read_claude(&mac, &anthropic, &app, Keychain::Grant);
    assert_eq!(row(&report, "claude").state.status, "fresh");
    assert!(live::hold_lifted(&app));
}

#[test]
fn a_mac_without_claude_code_has_no_claude_row() {
    let mac = Mac::new("readers-claude-missing");
    let anthropic = FakeAnthropic::start();
    let app = AppDir::new(mac.home.join("app"));
    let report = read_claude(&mac, &anthropic, &app, Keychain::Quiet);
    assert!(row(&report, "claude").not_set_up);
    assert!(anthropic.requests().is_empty());
    assert_eq!(Snapshot::build(Some(&report), None).claude.status, SectionStatus::NotSetUp);
}

// ===== No quota-axi =====

const CHILD: &str = "QUOTA_STATION_TEST_NO_QUOTA_AXI";

/// Runs again in a child whose PATH has only the system's folders, so no
/// quota-axi can be found, and every service still shows its numbers. The
/// fake `agy` is a shell script.
#[cfg(unix)]
#[test]
fn without_quota_axi_on_the_path_every_service_still_shows_numbers() {
    const NAME: &str = "without_quota_axi_on_the_path_every_service_still_shows_numbers";
    if std::env::var_os(CHILD).is_none() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([NAME, "--exact", "--test-threads=1"])
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env(CHILD, "1")
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{text}\n{}", String::from_utf8_lossy(&out.stderr));
        assert!(text.contains("1 passed"), "{text}");
        return;
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    assert!(std::env::split_paths(&path).all(|d| !d.join("quota-axi").exists()));

    let (mac, anthropic, app) =
        claude_mac("readers-no-axi", &credential("acc-live", "ref-live", in_hours(5.0)));
    let claude = read_claude(&mac, &anthropic, &app, Keychain::Quiet);

    let dir = temp_dir("readers-no-axi-agy");
    let answer = json!({ "command": { "name": "/quota", "data": summary() } });
    std::fs::write(dir.join("agy.json"), answer.to_string()).unwrap();
    let agy_report = read_agy(&agy_cli(&dir), &app);

    let server = chatgpt();
    let home = codex_home(&dir, &jwt(now().unix_timestamp() + 3600));
    let codex_report = read_codex(&home, &server, &app, 0);

    let s = Snapshot::build(Some(&claude), Some(&agy_report)).with_codex(Some(&codex_report));
    for (section, percent) in [(&s.claude, 70.0), (&s.agy, 60.0), (&s.codex, 75.0)] {
        let active = section.active().expect("an account in use");
        let shown: Vec<Option<f64>> = active.lines.iter().map(|l| l.percent).collect();
        assert!(shown.contains(&Some(percent)), "{shown:?}");
    }
}
