// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Logging Antigravity accounts in and out end to end, against a local fake
//! Google, a fake browser and a fake `security` in a temporary home. These
//! tests never reach Google and never touch the Keychain.
#![cfg(feature = "antigravity-accounts")]
// Unix only: the fake `security` is a shell script.
#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use common::google::{
    redirect_of, token_body, Consent, FakeBrowser, FakeGoogle, Home, Script, ACCESS, ACCOUNT_ID,
    CLIENT_ID, CLIENT_SECRET, EMAIL, INVALID_GRANT, REFRESH,
};
use quota_station_core::antigravity::client::{self, ClientStatus, OAuthClient, CLIENT_KEY};
use quota_station_core::antigravity::index::{self, Standing};
use quota_station_core::antigravity::login::{self, LoginError, LogoutError, UseError};
use quota_station_core::antigravity::oauth::{self, OAuthError, SCOPES};
use quota_station_core::antigravity::oauth_server::{CallbackError, FlowError, LOGIN_TIMEOUT};
use quota_station_core::antigravity::store::SERVICE;
use quota_station_core::antigravity::token::TokenData;

const NOW: i64 = 1_768_548_000;
const WAIT: Duration = Duration::from_secs(20);

fn now() -> i64 {
    NOW
}

/// One login with the fake browser following Google's redirect.
fn log_in(home: &Home, google: &FakeGoogle) -> (Result<login::LoggedIn, LoginError>, FakeBrowser) {
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let browser = FakeBrowser::following();
    let result =
        login::prepare(&deps).and_then(|flow| login::complete(&deps, flow, &browser, WAIT, &now));
    (result, browser)
}

fn stored_token(home: &Home, id: &str) -> TokenData {
    serde_json::from_str(&home.item(id).expect("the app's item")).unwrap()
}

#[test]
fn a_login_opens_cockpits_consent_url_and_keeps_the_account() {
    let home = Home::new("agy-login");
    let google = FakeGoogle::start(Script::default());

    let (result, browser) = log_in(&home, &google);
    let done = result.unwrap();
    assert_eq!(done, login::LoggedIn { id: ACCOUNT_ID.into(), number: 1, added: true });

    // The consent URL: Antigravity's client, Cockpit's scopes and parameters
    // in Cockpit's order, a loopback redirect and a UUID state.
    let opened = browser.opened();
    assert_eq!(opened.len(), 1);
    let url = url::Url::parse(&opened[0]).unwrap();
    assert_eq!(url.path(), "/o/oauth2/v2/auth");
    let params: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    let keys: Vec<&str> = params.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        keys,
        ["client_id", "redirect_uri", "response_type", "scope", "access_type", "prompt", "state"]
    );
    let get = |k: &str| params.iter().find(|(key, _)| key == k).unwrap().1.clone();
    assert_eq!(get("client_id"), CLIENT_ID);
    assert_eq!(get("response_type"), "code");
    assert_eq!(get("scope"), SCOPES.join(" "));
    assert_eq!(get("access_type"), "offline");
    assert_eq!(get("prompt"), "consent");
    let redirect = get("redirect_uri");
    let port: u16 = redirect
        .strip_prefix("http://localhost:")
        .and_then(|r| r.strip_suffix("/oauth-callback"))
        .expect("http://localhost:<port>/oauth-callback")
        .parse()
        .unwrap();
    assert!(port > 0);
    let state = uuid::Uuid::parse_str(&get("state")).unwrap();
    assert_eq!(state.get_version_num(), 4);
    assert!(!opened[0].contains(CLIENT_SECRET), "the secret never goes to the browser");

    // The callback page told the browser it worked.
    let pages = browser.pages();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].status, 200);
    assert!(pages[0].body.contains("Đã đăng nhập"));

    // The code exchange, then userinfo with the new access token.
    let tokens = google.to("/token");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].method, "POST");
    assert_eq!(
        tokens[0].form(),
        [
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("code", "fake-code"),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
    );
    let userinfo = google.to("/oauth2/v2/userinfo");
    assert_eq!(userinfo.len(), 1);
    assert_eq!(userinfo[0].header("authorization").as_deref(), Some(&*format!("Bearer {ACCESS}")));

    // The token went to the app's own item, without -A.
    let calls = home.security_calls();
    let add = calls.iter().find(|c| c[0] == "add-generic-password").unwrap();
    assert_eq!(add[..7], ["add-generic-password", "-U", "-s", SERVICE, "-a", ACCOUNT_ID, "-w"]);
    assert_eq!(add.len(), 8, "no -A: {add:?}");
    let token = stored_token(&home, ACCOUNT_ID);
    assert_eq!(token.access_token, ACCESS);
    assert_eq!(token.refresh_token, REFRESH);
    assert_eq!(token.expires_in, 3599);
    assert_eq!(token.expiry_timestamp, NOW + 3599);
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.email.as_deref(), Some(EMAIL));
    assert_eq!(token.session_id.as_deref(), Some("1000001"));
    assert_eq!(token.oauth_client_key.as_deref(), Some(CLIENT_KEY));
    assert_eq!(token.id_token.as_deref(), Some("fake-id-token"));
    assert_eq!(token.project_id, None);

    // Cockpit's index layout, with no token in it.
    let index = home.index_json();
    assert_eq!(index["version"], "2.0");
    assert_eq!(index["current_account_id"], ACCOUNT_ID);
    let entry = &index["accounts"][0];
    assert_eq!(entry["id"], ACCOUNT_ID);
    assert_eq!(entry["email"], EMAIL);
    assert_eq!(entry["name"], "Người Thử");
    assert_eq!(entry["created_at"], NOW);
    assert_eq!(entry["last_used"], NOW);
    let text = std::fs::read_to_string(home.index_file()).unwrap();
    assert!(!text.contains(ACCESS) && !text.contains(REFRESH));
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&home.index_file()), 0o600);
    assert_eq!(mode(home.index_file().parent().unwrap()), 0o700);
}

#[test]
fn the_account_id_is_cockpits_for_the_email() {
    assert_eq!(index::account_id(EMAIL), ACCOUNT_ID);
    assert_eq!(index::account_id("  SomeOne@Example.COM "), ACCOUNT_ID);
}

#[test]
fn a_callback_with_another_state_ends_the_login_and_keeps_nothing() {
    let home = Home::new("agy-state");
    let google = FakeGoogle::start(Script { consent: Consent::WrongState, ..Script::default() });

    let (result, browser) = log_in(&home, &google);

    assert_eq!(
        result.unwrap_err(),
        LoginError::Flow(FlowError::Callback(CallbackError::StateMismatch))
    );
    assert!(google.to("/token").is_empty(), "no code exchange");
    assert!(home.security_calls().is_empty());
    assert!(!home.index_file().exists());
    let page = &browser.pages()[0];
    assert_eq!(page.status, 400);
    assert!(page.body.contains("không khớp"));
}

#[test]
fn a_consent_without_a_code_ends_the_login() {
    let home = Home::new("agy-denied");
    let google = FakeGoogle::start(Script { consent: Consent::Denied, ..Script::default() });
    let (result, _) = log_in(&home, &google);
    let err = result.unwrap_err();
    assert_eq!(err, LoginError::Flow(FlowError::Callback(CallbackError::NoCode)));
    assert_eq!(err.kind(), "noCode");
    assert!(google.to("/token").is_empty());
}

#[test]
fn no_refresh_token_gives_cockpits_advice_and_keeps_nothing() {
    let home = Home::new("agy-norefresh");
    let google = FakeGoogle::start(Script {
        exchange: (200, token_body(ACCESS, None)),
        ..Script::default()
    });

    let (result, _) = log_in(&home, &google);

    let err = result.unwrap_err();
    assert_eq!(err, LoginError::NoRefreshToken);
    assert_eq!(err.kind(), "noRefreshToken");
    let advice = err.to_string();
    assert!(advice.contains("https://myaccount.google.com/permissions"));
    assert!(advice.contains("Antigravity Tools"));
    assert!(google.to("/oauth2/v2/userinfo").is_empty(), "Cockpit stops before userinfo");
    assert!(home.security_calls().is_empty());
    assert!(!home.index_file().exists());
}

#[test]
fn an_invalid_grant_on_the_exchange_keeps_nothing_and_no_body() {
    let home = Home::new("agy-exchange");
    let google =
        FakeGoogle::start(Script { exchange: (400, INVALID_GRANT.into()), ..Script::default() });

    let (result, _) = log_in(&home, &google);

    let err = result.unwrap_err();
    assert_eq!(
        err,
        LoginError::Exchange(OAuthError::Rejected {
            status: 400,
            code: Some("invalid_grant".into())
        })
    );
    assert_eq!(err.kind(), "invalidGrant");
    assert!(!err.to_string().contains("expired or revoked"), "Google's description is not kept");
    assert!(home.security_calls().is_empty());
    assert!(!home.index_file().exists());
}

#[test]
fn a_failing_keychain_leaves_the_index_unwritten() {
    let home = Home::new("agy-keychain");
    let google = FakeGoogle::start(Script::default());
    home.fail_security("add");

    let (result, _) = log_in(&home, &google);

    let LoginError::Keychain(e) = result.unwrap_err() else { panic!() };
    assert!(e.to_string().contains("exited 36"), "{e}");
    assert!(!home.index_file().exists(), "no index without the token");
    assert!(!home.index_file().parent().unwrap().exists(), "nothing written at all");
}

#[test]
fn an_index_that_cannot_be_written_takes_the_new_item_back() {
    let home = Home::new("agy-noindex");
    let google = FakeGoogle::start(Script::default());
    let dir = home.index_file().parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    // An immutable folder: making it 700 before the write fails.
    let flags = |flag: &str| {
        let status = std::process::Command::new("chflags").arg(flag).arg(&dir).status().unwrap();
        assert!(status.success());
    };
    flags("uchg");

    let (result, _) = log_in(&home, &google);
    assert!(matches!(result, Err(LoginError::Index(index::IndexError::Write(..)))));
    assert!(home.item(ACCOUNT_ID).is_none(), "the new item is taken back");

    home.fail_security("delete");
    let (result, _) = log_in(&home, &google);
    let Err(LoginError::Index(e)) = result else { panic!() };
    assert!(e.to_string().contains("the new Keychain item is left too"), "{e}");
    assert!(home.item(ACCOUNT_ID).is_some());
    flags("nouchg");
}

#[test]
fn a_failing_keychain_leaves_an_existing_index_as_it_was() {
    let home = Home::new("agy-keychain2");
    let google = FakeGoogle::start(Script::default());
    log_in(&home, &google).0.unwrap();
    let before = std::fs::read_to_string(home.index_file()).unwrap();

    google.set(|s| s.userinfo.1 = r#"{"id":"1000002","email":"ultra.two@example.com"}"#.into());
    home.fail_security("add");
    let (result, _) = log_in(&home, &google);

    assert!(matches!(result, Err(LoginError::Keychain(_))));
    assert_eq!(std::fs::read_to_string(home.index_file()).unwrap(), before);
}

#[test]
fn a_second_account_is_added_and_a_new_login_of_the_first_keeps_its_place() {
    let home = Home::new("agy-relogin");
    let google = FakeGoogle::start(Script::default());
    log_in(&home, &google).0.unwrap();

    google.set(|s| {
        s.userinfo.1 = r#"{"id":"1000002","email":"ultra.two@example.com"}"#.into();
        s.exchange.1 = token_body("fake-access-b", Some("fake-refresh-b"));
    });
    let second = log_in(&home, &google).0.unwrap();
    assert_eq!(second.number, 2);
    assert!(second.added);
    assert_eq!(second.id, "antigravity_db0ef0cd84fd7974935f923bd7ccdd00");

    // Give the first account a project, as a quota read would, then log it
    // in again: same id, same place, created time kept, project kept.
    let mut token = stored_token(&home, ACCOUNT_ID);
    token.project_id = Some("fake-project".into());
    std::fs::write(
        home.sec.join(format!("items/{SERVICE}--{ACCOUNT_ID}")),
        serde_json::to_string(&token).unwrap(),
    )
    .unwrap();
    index::set_standing(&home.app, ACCOUNT_ID, Some(Standing::NeedsLogin { since: NOW })).unwrap();
    google.set(|s| {
        s.userinfo.1 = format!(r#"{{"id":"1000001","email":"{}"}}"#, EMAIL.to_uppercase());
        s.exchange.1 = token_body("fake-access-c", Some("fake-refresh-c"));
    });
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let browser = FakeBrowser::following();
    let flow = login::prepare(&deps).unwrap();
    let again = login::complete(&deps, flow, &browser, WAIT, &|| NOW + 60).unwrap();
    assert_eq!(again, login::LoggedIn { id: ACCOUNT_ID.into(), number: 1, added: false });

    let token = stored_token(&home, ACCOUNT_ID);
    assert_eq!(token.refresh_token, "fake-refresh-c");
    assert_eq!(token.project_id.as_deref(), Some("fake-project"));
    let index = index::load(&home.app).unwrap();
    assert_eq!(index.accounts.len(), 2);
    assert_eq!(index.accounts[0].created_at, NOW);
    assert_eq!(index.accounts[0].last_used, NOW + 60);
    assert_eq!(
        index.accounts[0].name.as_deref(),
        Some("Người Thử"),
        "a login without a name keeps it"
    );
    assert_eq!(index.current_account_id.as_deref(), Some(ACCOUNT_ID));
    assert!(index::load_statuses(&home.app).accounts.is_empty(), "the new login answered it");
}

#[test]
fn a_login_without_a_callback_gives_up_at_the_timeout() {
    assert_eq!(LOGIN_TIMEOUT, Duration::from_secs(600), "Cockpit waits 10 minutes");
    let home = Home::new("agy-timeout");
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let browser = FakeBrowser::default();
    let flow = login::prepare(&deps).unwrap();
    let handle = flow.handle();

    let started = Instant::now();
    let result = login::complete(&deps, flow, &browser, Duration::from_millis(300), &now);

    assert_eq!(result.unwrap_err(), LoginError::Flow(FlowError::Timeout));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(browser.opened().len(), 1);
    // The listener stopped: a late callback finds nobody waiting.
    let late = redirect_of(handle.auth_url());
    assert_eq!(handle.submit_callback_url(&late), Err(CallbackError::AlreadyHandled));
    let port = url::Url::parse(handle.redirect_uri()).unwrap().port().unwrap();
    // Its thread looks every 25 ms; a busy machine may take longer than one look.
    let closed = (0..40).any(|_| {
        std::thread::sleep(Duration::from_millis(50));
        std::net::TcpStream::connect(("127.0.0.1", port)).is_err()
    });
    assert!(closed, "the port is closed");
    assert!(home.security_calls().is_empty());
}

#[test]
fn a_pasted_callback_url_finishes_the_login_and_a_wrong_one_does_not() {
    let home = Home::new("agy-paste");
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let browser = FakeBrowser::default();
    let flow = login::prepare(&deps).unwrap();
    let handle = flow.handle();

    let paster = std::thread::spawn(move || {
        let callback = redirect_of(handle.auth_url());
        let wrong = callback.replace("state=", "state=x");
        assert_eq!(handle.submit_callback_url(&wrong), Err(CallbackError::StateMismatch));
        assert!(matches!(
            handle.submit_callback_url("/elsewhere?code=c&state=s"),
            Err(CallbackError::BadUrl(_))
        ));
        let query = callback.split_once('?').unwrap().1.to_string();
        handle.submit_callback_url(&query).unwrap();
        assert_eq!(handle.submit_callback_url(&query), Err(CallbackError::AlreadyHandled));
    });
    let done = login::complete(&deps, flow, &browser, WAIT, &now).unwrap();
    paster.join().unwrap();
    assert_eq!(done.id, ACCOUNT_ID);
    assert_eq!(google.to("/token").len(), 1);
}

#[test]
fn a_cancelled_login_stops_waiting() {
    let home = Home::new("agy-cancel");
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let flow = login::prepare(&deps).unwrap();
    let handle = flow.handle();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        handle.cancel();
    });
    let result = login::complete(&deps, flow, &FakeBrowser::default(), WAIT, &now);
    canceller.join().unwrap();
    assert_eq!(result.unwrap_err(), LoginError::Flow(FlowError::Cancelled));
}

#[test]
fn the_listener_answers_other_paths_and_preflights_and_keeps_waiting() {
    use std::io::{Read, Write};
    let home = Home::new("agy-paths");
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let flow = login::prepare(&deps).unwrap();
    let handle = flow.handle();
    let port = url::Url::parse(handle.redirect_uri()).unwrap().port().unwrap();
    let ask = |request: &str| {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(request.as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    };
    assert!(ask("GET /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n").starts_with("HTTP/1.1 404"));
    assert!(ask("OPTIONS /oauth-callback HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .contains("Access-Control-Allow-Origin: *"));
    let callback = redirect_of(handle.auth_url());
    let target = callback.split_once(&format!("{port}")).unwrap().1.to_string();
    assert!(ask(&format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n"))
        .starts_with("HTTP/1.1 200"));
    let (code, redirect) = flow.wait(WAIT).unwrap();
    assert_eq!(code, "fake-code");
    assert_eq!(redirect, handle.redirect_uri());
}

#[test]
fn a_browser_that_does_not_open_ends_the_login() {
    let home = Home::new("agy-browser");
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let flow = login::prepare(&deps).unwrap();
    let handle = flow.handle();
    let browser = FakeBrowser { fail: true, ..Default::default() };
    let err = login::complete(&deps, flow, &browser, WAIT, &now).unwrap_err();
    assert_eq!(err.kind(), "browser");
    // The login is over: even the right callback finds nobody waiting.
    let callback = redirect_of(handle.auth_url());
    assert_eq!(handle.submit_callback_url(&callback), Err(CallbackError::AlreadyHandled));
}

#[test]
fn a_token_is_refreshed_only_when_it_has_under_five_minutes_left() {
    let home = Home::new("agy-refresh");
    let google = FakeGoogle::start(Script::default());
    log_in(&home, &google).0.unwrap();
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let expiry = NOW + 3599;

    // More than 300 seconds left: the stored token, no call to Google.
    let token = login::fresh_token(&deps, ACCOUNT_ID, expiry - 301).unwrap();
    assert_eq!(token.access_token, ACCESS);
    assert_eq!(google.to("/token").len(), 1, "only the login's exchange");

    // 300 seconds or less: refreshed with the same client, refresh token kept.
    let at = expiry - 300;
    let token = login::fresh_token(&deps, ACCOUNT_ID, at).unwrap();
    assert_eq!(token.access_token, "fake-access-2");
    assert_eq!(token.refresh_token, REFRESH);
    assert_eq!(token.expiry_timestamp, at + 3599);
    assert_eq!(token.email.as_deref(), Some(EMAIL));
    assert_eq!(token.session_id.as_deref(), Some("1000001"));
    assert_eq!(token.id_token.as_deref(), Some("fake-id-token"));
    let refresh = google.to("/token").pop().unwrap();
    assert_eq!(
        refresh.form(),
        [
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("refresh_token", REFRESH),
            ("grant_type", "refresh_token"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
    );
    assert_eq!(stored_token(&home, ACCOUNT_ID).access_token, "fake-access-2", "kept for next time");
}

#[test]
fn ensure_fresh_keeps_the_gcp_tos_mark_and_the_project() {
    let google = FakeGoogle::start(Script::default());
    let client = OAuthClient::new(CLIENT_ID, CLIENT_SECRET).unwrap();
    let mut token = TokenData::new(
        "old".into(),
        REFRESH.into(),
        100,
        Some(EMAIL.into()),
        Some("p".into()),
        None,
        NOW,
    );
    token.is_gcp_tos = Some(true);
    let fresh = oauth::ensure_fresh_token(&google.endpoints(), &client, &token, NOW).unwrap();
    assert_eq!(fresh.is_gcp_tos, Some(true));
    assert_eq!(fresh.project_id.as_deref(), Some("p"));
    assert_eq!(fresh.oauth_client_key.as_deref(), Some(CLIENT_KEY));
}

#[test]
fn an_invalid_grant_on_refresh_marks_the_account_as_needing_a_new_login() {
    let home = Home::new("agy-revoked");
    let google =
        FakeGoogle::start(Script { refresh: (400, INVALID_GRANT.into()), ..Script::default() });
    log_in(&home, &google).0.unwrap();
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);

    let err = login::fresh_token(&deps, ACCOUNT_ID, NOW + 3599).unwrap_err();

    assert_eq!(err, UseError::NeedsLogin);
    let statuses = index::load_statuses(&home.app);
    assert_eq!(
        statuses.accounts.get(ACCOUNT_ID),
        Some(&Standing::NeedsLogin { since: NOW + 3599 })
    );
    assert_eq!(index::load(&home.app).unwrap().accounts.len(), 1, "still listed");
    assert_eq!(stored_token(&home, ACCOUNT_ID).access_token, ACCESS, "the old token stays");

    // Another refusal is a refusal, not a new login.
    google.set(|s| s.refresh = (500, r#"{"error":"internal_failure"}"#.into()));
    let err = login::fresh_token(&deps, ACCOUNT_ID, NOW + 3599).unwrap_err();
    assert!(matches!(err, UseError::Refresh(OAuthError::Rejected { status: 500, .. })));
}

#[test]
fn logging_out_removes_only_the_apps_item_and_the_index_line() {
    let home = Home::new("agy-logout");
    let google = FakeGoogle::start(Script::default());
    log_in(&home, &google).0.unwrap();
    google.set(|s| s.userinfo.1 = r#"{"id":"1000002","email":"ultra.two@example.com"}"#.into());
    let second = log_in(&home, &google).0.unwrap();
    index::set_standing(&home.app, ACCOUNT_ID, Some(Standing::NeedsLogin { since: NOW })).unwrap();
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);

    login::logout(&deps, ACCOUNT_ID).unwrap();

    let delete = home.security_calls().pop().unwrap();
    assert_eq!(delete, ["delete-generic-password", "-s", SERVICE, "-a", ACCOUNT_ID]);
    assert!(home.item(ACCOUNT_ID).is_none());
    assert!(home.item(&second.id).is_some());
    let index = index::load(&home.app).unwrap();
    assert_eq!(
        index.accounts.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
        [second.id.as_str()]
    );
    assert_eq!(index.current_account_id, None, "it was the current one");
    assert!(index::load_statuses(&home.app).accounts.is_empty());
    assert_eq!(login::logout(&deps, ACCOUNT_ID), Err(LogoutError::NotFound));
}

#[test]
fn a_logout_the_keychain_refuses_keeps_the_account_listed() {
    let home = Home::new("agy-logout-fail");
    let google = FakeGoogle::start(Script::default());
    log_in(&home, &google).0.unwrap();
    let before = std::fs::read_to_string(home.index_file()).unwrap();
    home.fail_security("delete");
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);

    assert!(matches!(login::logout(&deps, ACCOUNT_ID), Err(LogoutError::Keychain(_))));
    assert_eq!(std::fs::read_to_string(home.index_file()).unwrap(), before);
}

#[test]
fn a_logout_of_an_item_already_gone_still_forgets_the_account() {
    let home = Home::new("agy-logout-gone");
    let google = FakeGoogle::start(Script::default());
    log_in(&home, &google).0.unwrap();
    std::fs::remove_file(home.sec.join(format!("items/{SERVICE}--{ACCOUNT_ID}"))).unwrap();
    let endpoints = google.endpoints();
    login::logout(&home.deps(&endpoints), ACCOUNT_ID).unwrap();
    assert!(index::load(&home.app).unwrap().accounts.is_empty());
}

#[test]
fn the_oauth_client_file_must_be_private_and_filled_in() {
    let home = Home::new("agy-client");
    assert_eq!(client::status(&home.app), ClientStatus::Ok);
    let loaded = client::load(&home.app).unwrap();
    assert_eq!(loaded.id(), CLIENT_ID);
    assert!(
        !format!("{loaded:?}").contains(CLIENT_SECRET)
            && !format!("{loaded:?}").contains(CLIENT_ID)
    );

    home.write_client(
        &format!(r#"{{"client_id":"{CLIENT_ID}","client_secret":"{CLIENT_SECRET}"}}"#),
        0o644,
    );
    assert_eq!(client::status(&home.app), ClientStatus::TooOpen);
    home.write_client("not json", 0o600);
    assert_eq!(client::status(&home.app), ClientStatus::Invalid);
    home.write_client(
        r#"{"client_id":"<client id from Cockpit Tools>.apps.googleusercontent.com","client_secret":"<client secret>"}"#,
        0o600,
    );
    assert_eq!(client::status(&home.app), ClientStatus::Invalid, "the README's placeholders");
    home.write_client(r#"{"client_id":"","client_secret":"x"}"#, 0o600);
    assert_eq!(client::status(&home.app), ClientStatus::Invalid);
    std::fs::remove_file(home.app.path().join("oauth-client.json")).unwrap();
    assert_eq!(client::status(&home.app), ClientStatus::Missing);
    std::fs::create_dir(home.app.path().join("oauth-client.json")).unwrap();
    assert_eq!(client::status(&home.app), ClientStatus::Unreadable);

    // A login with no usable client never starts listening.
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let err = login::prepare(&home.deps(&endpoints)).err().unwrap();
    assert_eq!(err, LoginError::Client(ClientStatus::Unreadable));
}

#[test]
fn an_unreadable_index_is_reported_and_left_alone() {
    let home = Home::new("agy-badindex");
    std::fs::create_dir_all(home.index_file().parent().unwrap()).unwrap();
    std::fs::write(home.index_file(), "{ broken").unwrap();
    let google = FakeGoogle::start(Script::default());

    let (result, _) = log_in(&home, &google);

    assert!(matches!(result, Err(LoginError::Index(index::IndexError::Unreadable(..)))));
    assert_eq!(std::fs::read_to_string(home.index_file()).unwrap(), "{ broken");
    assert!(home.security_calls().is_empty(), "nothing kept when the index cannot be read");
}

#[test]
fn an_unreadable_status_file_is_left_alone() {
    let home = Home::new("agy-badstatus");
    let google =
        FakeGoogle::start(Script { refresh: (400, INVALID_GRANT.into()), ..Script::default() });
    log_in(&home, &google).0.unwrap();
    let status = home.index_file().with_file_name("status.json");
    let broken = r#"{"accounts":{"antigravity_other":{"state":"later"}}}"#;
    std::fs::write(&status, broken).unwrap();
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);

    assert!(matches!(
        index::set_standing(&home.app, ACCOUNT_ID, Some(Standing::NeedsLogin { since: NOW })),
        Err(index::IndexError::Unreadable(..))
    ));
    assert_eq!(login::fresh_token(&deps, ACCOUNT_ID, NOW + 3599), Err(UseError::NeedsLogin));
    log_in(&home, &google).0.unwrap();

    assert_eq!(std::fs::read_to_string(&status).unwrap(), broken, "no status was dropped");
    assert!(index::load_statuses(&home.app).accounts.is_empty(), "the windows show none");
}
