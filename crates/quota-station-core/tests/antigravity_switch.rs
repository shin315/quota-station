// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Switching the Antigravity account end to end, in a temporary home with a
//! local fake Google, a fake `security`, a fake Antigravity.app (a shell
//! process) with fake `plutil`, `pgrep`, `osascript` and `open`, and a fake
//! `agy`. These tests never reach Google, the Keychain, the real
//! Antigravity.app or the real agy.
#![cfg(feature = "antigravity-accounts")]
// Unix only: the fake `security`, `agy` and Antigravity.app tools are shell scripts.
#![cfg(unix)]

mod common;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use common::desktop::{agy_quota, fake_agy, remember, FakeApp};
use common::google::{
    token_body, FakeBrowser, FakeGoogle, Home, Script, ACCOUNT_ID, INVALID_GRANT, LOAD_CODE_ASSIST,
};
use quota_station_core::antigravity::credential::SharedLogin;
use quota_station_core::antigravity::index::{self, Standing};
use quota_station_core::antigravity::listing;
use quota_station_core::antigravity::login;
use quota_station_core::antigravity::switch::{
    self, switch_antigravity, CloudCodeProbe, Deps, Done, How, Limits, NoProbe, Outcome, Probe,
    ProjectMeta, Request,
};
use quota_station_core::antigravity::token::TokenData;
use quota_station_core::antigravity::version::AntigravityApp;
use quota_station_core::antigravity::watch;
use quota_station_core::quota::agy;

const NOW: i64 = 1_768_548_000;
const SECOND: &str = "ultra.two@example.com";
const OLD_LOGIN: &str = "go-keyring-base64:old-login";
/// When each account's last reading says its Gemini five hours reset; a
/// live reading resetting at the same time is that account's.
const FIRST_RESET: &str = "2030-01-15T13:00:00Z";
const SECOND_RESET: &str = "2030-01-15T14:00:00Z";

/// One switch at a time in the whole app, so one at a time here too.
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

struct Setup {
    home: Home,
    google: FakeGoogle,
    app: FakeApp,
    agy: agy::Tools,
    shared: SharedLogin,
    /// The second account's id: the one switched to.
    second: String,
}

/// Two accounts logged in (the first is current), each with a reading, the
/// shared login holding an old value, Antigravity.app `version` installed
/// (`None`: not) and not running, and agy reading what the second account's
/// reading says.
fn setup(tag: &str, version: Option<&str>) -> Setup {
    let home = Home::new(tag);
    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    let now = || NOW;
    for (email, access, refresh) in [
        (common::google::EMAIL, "fake-access-1", "fake-refresh-1"),
        (SECOND, "fake-access-b", "fake-refresh-b"),
    ] {
        google.set(|s| {
            s.userinfo.1 = format!(r#"{{"id":"1","email":"{email}"}}"#);
            s.exchange.1 = token_body(access, Some(refresh));
        });
        let flow = login::prepare(&deps).unwrap();
        login::complete(&deps, flow, &FakeBrowser::following(), Duration::from_secs(20), &now)
            .unwrap();
    }
    std::fs::write(home.sec.join("items/gemini--antigravity"), OLD_LOGIN).unwrap();
    let app = FakeApp::new(&home.home.join("desktop"), version);
    app.watch_item(&home.sec.join("items/gemini--antigravity"));
    remember(&home.app, common::google::EMAIL, 40.0, FIRST_RESET, NOW * 1000);
    remember(&home.app, SECOND, 20.0, SECOND_RESET, NOW * 1000);
    let agy = fake_agy(&home.home);
    let s = Setup {
        shared: SharedLogin::fake(home.sec.join("bin/security"), Duration::from_secs(5)),
        second: index::account_id(SECOND),
        home,
        google,
        app,
        agy,
    };
    s.report(Some(&agy_quota(25.0, SECOND_RESET)));
    s
}

impl Setup {
    /// What the fake agy's `/quota` prints; `None` makes it fail.
    fn report(&self, json: Option<&str>) {
        let file = self.home.home.join("agy.json");
        match json {
            Some(text) => std::fs::write(file, text).unwrap(),
            None => {
                let _ = std::fs::remove_file(file);
            }
        }
    }

    fn switch(&self, how: How, now: i64, probe: Arc<dyn Probe>, limits: Limits) -> Done {
        self.switch_from(Some(ACCOUNT_ID), how, now, probe, limits)
    }

    /// A switch to the second account, `from` being the account the last
    /// reading showed in use.
    fn switch_from(
        &self,
        from: Option<&str>,
        how: How,
        now: i64,
        probe: Arc<dyn Probe>,
        limits: Limits,
    ) -> Done {
        let endpoints = self.google.endpoints();
        let desktop = self.app.desktop();
        let deps = Deps {
            app: &self.home.app,
            endpoints: &endpoints,
            keychain: &self.home.keychain,
            probe,
            shared: &self.shared,
            desktop: &desktop,
            agy: &self.agy,
        };
        let request = Request { id: self.second.clone(), how, from: from.map(String::from) };
        switch_antigravity(&deps, &request, limits, &|| now)
    }

    fn run(&self, how: How) -> Done {
        self.switch(how, NOW + 60, Arc::new(NoProbe), limits())
    }

    fn shared_item(&self) -> String {
        std::fs::read_to_string(self.home.sec.join("items/gemini--antigravity")).unwrap()
    }

    /// The `security` runs that named the shared item.
    fn shared_calls(&self) -> Vec<Vec<String>> {
        self.home
            .security_calls()
            .into_iter()
            .filter(|c| c.iter().any(|a| a == "gemini" || a == "antigravity"))
            .collect()
    }

    fn binding(&self) -> Option<serde_json::Value> {
        let text = std::fs::read_to_string(switch::binding_path(&self.home.app)).ok()?;
        Some(serde_json::from_str(&text).unwrap())
    }

    fn current(&self) -> Option<String> {
        index::load(&self.home.app).unwrap().current_account_id
    }
}

/// Waits that end early when things go right, long enough for a loaded
/// machine; the tests that wait them out set their own.
fn limits() -> Limits {
    Limits {
        prepare: Duration::from_secs(10),
        quit: Duration::from_secs(10),
        settle: Duration::ZERO,
    }
}

/// The token a shared-login value holds.
fn decode(value: &str) -> serde_json::Value {
    let b64 = value.strip_prefix("go-keyring-base64:").expect("go-keyring's prefix");
    serde_json::from_slice(&STANDARD.decode(b64).unwrap()).unwrap()
}

fn switched(number: usize, how: How, reopened: Option<bool>) -> Outcome {
    Outcome::Switched { number, how, basis: Some("reading".into()), reopened, saved: true }
}

#[test]
fn a_switch_writes_cockpits_exact_command_then_the_pointers_and_checks() {
    let _one = serial();
    let s = setup("agy-switch-plain", Some("2.24.0"));
    let done = s.run(How::Plain);
    assert_eq!(done.outcome, switched(2, How::Plain, None));
    assert!(done.report.is_some());

    // Exactly Cockpit's command, once, with the second account's login.
    let calls = s.shared_calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let call = &calls[0];
    assert_eq!(call.len(), 9, "{call:?}");
    assert_eq!(
        call[..7],
        ["add-generic-password", "-U", "-s", "gemini", "-a", "antigravity", "-w"]
    );
    assert_eq!(call[8], "-A");
    assert_eq!(call[7], s.shared_item());
    let payload = decode(&call[7]);
    assert_eq!(payload["auth_method"], "consumer");
    assert_eq!(payload["token"]["access_token"], "fake-access-b");
    assert_eq!(payload["token"]["refresh_token"], "fake-refresh-b");
    assert_eq!(payload["token"]["token_type"], "Bearer");
    assert_eq!(payload["token"]["expiry"], "2026-01-16T08:19:59.000000Z");
    // The app never reads or deletes the shared item.
    for c in s.home.security_calls() {
        assert!(c[0] == "add-generic-password" || !c.iter().any(|a| a == "gemini"), "{c:?}");
    }

    // Then the pointers the account in use is told by and Cockpit's layout uses.
    assert_eq!(s.current().as_deref(), Some(s.second.as_str()));
    assert_eq!(s.binding().unwrap()["defaultSettings"]["bindAccountId"], s.second.as_str());
    let index = index::load(&s.home.app).unwrap();
    assert_eq!(index.accounts[1].last_used, NOW + 60);

    // Nothing asked the app to quit or open; the check asked agy afresh.
    assert!(s.app.runs_of("osascript").is_empty());
    assert!(s.app.runs_of("open").is_empty());
    let args = std::fs::read_to_string(s.home.home.join("agy-args")).unwrap();
    assert_eq!(args.lines().collect::<Vec<_>>(), ["-p", "/quota", "--output-format", "json"]);

    // One log line, ids only; the watch starts and has seen the new account.
    let log = switch::read_log(&s.home.app);
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].from.as_deref(), Some(ACCOUNT_ID));
    assert_eq!(log[0].to, s.second);
    assert_eq!(log[0].result, "switched");
    let text = std::fs::read_to_string(switch::log_path(&s.home.app)).unwrap();
    assert!(!text.contains("@example.com"), "{text}");
    let w = watch::load(&s.home.app).unwrap();
    assert_eq!(
        (w.from.as_str(), w.to.as_str(), w.at, w.seen),
        (ACCOUNT_ID, s.second.as_str(), NOW + 60, true)
    );
}

#[test]
fn quit_first_asks_the_app_to_quit_writes_while_it_is_gone_then_opens_it() {
    let _one = serial();
    let s = setup("agy-switch-quit", Some("2.24.0"));
    s.app.start();
    let done = s.run(How::QuitFirst);
    assert_eq!(done.outcome, switched(2, How::QuitFirst, Some(true)));

    assert_eq!(
        s.app.runs_of("osascript"),
        [vec!["-e".to_string(), "tell application id \"com.google.antigravity\" to quit".into()]]
    );
    assert_eq!(s.app.runs_of("open"), [vec!["-b".to_string(), "com.google.antigravity".into()]]);
    let pgrep = s.app.runs_of("pgrep");
    assert!(!pgrep.is_empty());
    let process = s.app.app.join("Contents/MacOS/Antigravity").display().to_string();
    assert!(pgrep.iter().all(|args| *args == ["-f".to_string(), process.clone()]), "{pgrep:?}");
    // The old login was still there when the app was asked to quit, and the
    // new one when it was opened again.
    assert_eq!(s.app.item_at("quit").as_deref(), Some(OLD_LOGIN));
    assert_eq!(s.app.item_at("open"), Some(s.shared_item()));
    assert_ne!(s.shared_item(), OLD_LOGIN);
    assert!(s.app.wait_running(true), "the app runs again");
}

#[test]
fn an_app_that_does_not_quit_stops_the_switch_before_anything_is_written() {
    let _one = serial();
    let s = setup("agy-switch-stays", Some("2.24.0"));
    s.app.start();
    s.app.stay();
    let limits = Limits { quit: Duration::from_millis(600), ..limits() };
    let done = s.switch(How::QuitFirst, NOW + 60, Arc::new(NoProbe), limits);
    assert_eq!(
        done.outcome,
        Outcome::Failed { number: 2, error: "notQuit", detail: String::new(), reopened: None }
    );
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert!(s.shared_calls().is_empty());
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
    assert!(s.binding().is_none());
    assert!(watch::load(&s.home.app).is_none());
    // Never killed, never opened again.
    assert!(s.app.running());
    assert!(s.app.runs_of("open").is_empty());
    assert_eq!(switch::read_log(&s.home.app)[0].error.as_deref(), Some("notQuit"));
}

#[test]
fn a_process_list_that_cannot_be_read_stops_a_quit_first_switch() {
    let _one = serial();
    let s = setup("agy-switch-pgrep", Some("2.24.0"));
    s.app.start();
    s.app.fail("pgrep");
    let done = s.run(How::QuitFirst);
    let Outcome::Failed { error, .. } = done.outcome else { panic!("{:?}", done.outcome) };
    assert_eq!(error, "appState");
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert!(s.app.runs_of("osascript").is_empty());
}

#[test]
fn a_plain_switch_stops_when_the_app_runs_again() {
    let _one = serial();
    let s = setup("agy-switch-plain-running", Some("2.24.0"));
    // The sheet saw it closed; the owner opened it before pressing "Chuyển".
    s.app.start();
    let done = s.run(How::Plain);
    assert_eq!(
        done.outcome,
        Outcome::Failed { number: 2, error: "appRunning", detail: String::new(), reopened: None }
    );
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert!(s.shared_calls().is_empty());
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
    assert!(s.app.runs_of("osascript").is_empty());
    assert!(s.app.running());
}

#[test]
fn an_app_whose_version_cannot_be_read_stops_the_switch_before_any_write() {
    let _one = serial();
    let s = setup("agy-switch-noplist", Some("2.24.0"));
    s.app.start();
    // The bundle is there, its version is not: not taken for a missing app.
    std::fs::remove_file(s.app.app.join("Contents/Info.plist")).unwrap();
    let keychain_before = s.home.item(&s.second);
    for how in [How::QuitFirst, How::Plain, How::KeepRunning] {
        let done = s.switch(how, NOW + 3500, Arc::new(NoProbe), limits());
        let Outcome::Failed { error, detail, .. } = done.outcome else {
            panic!("{:?}", done.outcome)
        };
        assert_eq!(error, "appVersion");
        assert!(detail.contains("file does not exist"), "{detail}");
    }
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert!(s.shared_calls().is_empty());
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
    assert!(s.binding().is_none());
    // Stopped before the preparation too: the token due for a refresh is not refreshed.
    assert_eq!(s.home.item(&s.second), keychain_before);
    assert!(s.app.runs_of("osascript").is_empty());
    assert!(s.app.runs_of("pgrep").is_empty());
    assert!(s.app.running());
}

#[test]
fn a_version_with_no_number_stops_the_switch_too() {
    let _one = serial();
    let s = setup("agy-switch-dev", Some("dev"));
    let Outcome::Failed { error, .. } = s.run(How::Plain).outcome else { panic!() };
    assert_eq!(error, "appVersion");
    assert_eq!(s.shared_item(), OLD_LOGIN);
}

/// Sets or clears the immutable flag of `path`, so writing over it fails.
fn immutable(path: &std::path::Path, on: bool) {
    let flag = if on { "uchg" } else { "nouchg" };
    let ok = std::process::Command::new("/usr/bin/chflags").arg(flag).arg(path).status().unwrap();
    assert!(ok.success());
}

/// Clears the flag again, even when the test fails.
struct Mutable(std::path::PathBuf);

impl Drop for Mutable {
    fn drop(&mut self) {
        immutable(&self.0, false);
    }
}

#[test]
fn the_two_pointers_are_written_together_or_not_at_all() {
    let _one = serial();
    // The binding cannot be written: the index is put back as it was.
    let s = setup("agy-switch-binding-fails", Some("2.24.0"));
    let index_path = index::index_path(&s.home.app);
    let before = std::fs::read_to_string(&index_path).unwrap();
    std::fs::create_dir_all(switch::binding_path(&s.home.app)).unwrap();
    let done = s.run(How::Plain);
    assert!(matches!(done.outcome, Outcome::Switched { saved: false, .. }), "{:?}", done.outcome);
    assert_eq!(std::fs::read_to_string(&index_path).unwrap(), before);
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
    assert!(switch::binding_path(&s.home.app).is_dir());
    // The login itself is switched; the pointers say the old account.
    assert_ne!(s.shared_item(), OLD_LOGIN);

    // The index cannot be written: the binding is not written either.
    let s = setup("agy-switch-index-fails", Some("2.24.0"));
    let index_path = index::index_path(&s.home.app);
    let before = std::fs::read_to_string(&index_path).unwrap();
    immutable(&index_path, true);
    let _mutable = Mutable(index_path.clone());
    let done = s.run(How::Plain);
    assert!(matches!(done.outcome, Outcome::Switched { saved: false, .. }), "{:?}", done.outcome);
    assert_eq!(std::fs::read_to_string(&index_path).unwrap(), before);
    assert!(s.binding().is_none());
    assert_ne!(s.shared_item(), OLD_LOGIN);
}

#[test]
fn keep_running_writes_while_the_app_runs() {
    let _one = serial();
    let s = setup("agy-switch-keep", Some("2.24.0"));
    s.app.start();
    let done = s.run(How::KeepRunning);
    assert_eq!(done.outcome, switched(2, How::KeepRunning, None));
    assert!(s.app.runs_of("osascript").is_empty());
    assert!(s.app.running());
    assert_ne!(s.shared_item(), OLD_LOGIN);
}

#[test]
fn a_failed_write_keeps_the_old_login_and_the_pointers_and_brings_the_app_back() {
    let _one = serial();
    let s = setup("agy-switch-write", Some("2.24.0"));
    s.app.start();
    s.home.fail_security("add");
    let done = s.run(How::QuitFirst);
    let Outcome::Failed { error, detail, reopened, .. } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert_eq!((*error, *reopened), ("write", Some(true)));
    assert!(detail.contains("User interaction is not allowed"), "{detail}");
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
    assert!(s.binding().is_none());
    assert!(s.app.wait_running(true));
}

#[test]
fn a_reopen_that_fails_is_reported_and_the_switch_stands() {
    let _one = serial();
    let s = setup("agy-switch-reopen", Some("2.24.0"));
    s.app.start();
    s.app.fail("open");
    let done = s.run(How::QuitFirst);
    assert_eq!(done.outcome, switched(2, How::QuitFirst, Some(false)));
    assert_eq!(s.current().as_deref(), Some(s.second.as_str()));
}

#[test]
fn the_check_says_switched_still_old_or_cannot_tell() {
    let _one = serial();
    let s = setup("agy-switch-check", Some("2.24.0"));
    let unconfirmed = |active| Outcome::Unconfirmed {
        number: 2,
        how: How::Plain,
        active,
        reopened: None,
        saved: true,
    };

    let basis = |done: Done| match done.outcome {
        Outcome::Switched { basis, .. } => basis,
        other => panic!("{other:?}"),
    };
    assert_eq!(basis(s.run(How::Plain)).as_deref(), Some("reading"));
    // An unused window tells no account: only the binding answers.
    s.report(Some(&agy_quota(0.0, SECOND_RESET)));
    assert_eq!(basis(s.run(How::Plain)).as_deref(), Some("binding"));
    // agy fails or answers nonsense: the binding answers too.
    s.report(None);
    assert_eq!(basis(s.run(How::Plain)).as_deref(), Some("binding"));
    s.report(Some("not json"));
    assert_eq!(basis(s.run(How::Plain)).as_deref(), Some("binding"));

    // agy still reads the old account.
    s.report(Some(&agy_quota(45.0, FIRST_RESET)));
    assert_eq!(s.run(How::Plain).outcome, unconfirmed(Some(1)));
    // A reading neither account's matches contradicts the binding.
    s.report(Some(&agy_quota(55.0, "2030-01-15T15:30:00Z")));
    assert_eq!(s.run(How::Plain).outcome, unconfirmed(None));

    let results: Vec<String> =
        switch::read_log(&s.home.app).into_iter().map(|l| l.result).collect();
    assert_eq!(
        results,
        ["switched", "switched", "switched", "switched", "unconfirmed", "unconfirmed"]
    );
}

#[test]
fn an_antigravity_app_older_than_2_0_is_refused() {
    let _one = serial();
    let s = setup("agy-switch-old", Some("1.20.5"));
    let done = s.run(How::Plain);
    assert_eq!(
        done.outcome,
        Outcome::Failed { number: 2, error: "oldApp", detail: "1.20.5".into(), reopened: None }
    );
    assert!(s.shared_calls().is_empty());
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
}

#[test]
fn without_antigravity_app_only_agys_login_changes() {
    let _one = serial();
    let s = setup("agy-switch-noapp", None);
    let done = s.run(How::QuitFirst);
    assert_eq!(done.outcome, switched(2, How::QuitFirst, None));
    // The process list is still asked, and shows nothing to quit.
    assert_eq!(s.app.runs_of("pgrep").len(), 1);
    assert!(s.app.runs_of("osascript").is_empty());
    assert!(s.app.runs_of("open").is_empty());
    assert_eq!(s.shared_calls().len(), 1);
}

#[test]
fn a_token_about_to_expire_is_refreshed_before_the_write() {
    let _one = serial();
    let s = setup("agy-switch-refresh", Some("2.24.0"));
    let later = NOW + 3500;
    let done = s.switch(How::Plain, later, Arc::new(NoProbe), limits());
    assert!(matches!(done.outcome, Outcome::Switched { .. }), "{:?}", done.outcome);
    let payload = decode(&s.shared_item());
    assert_eq!(payload["token"]["access_token"], "fake-access-2", "the refreshed token");
    assert_eq!(payload["token"]["refresh_token"], "fake-refresh-b", "the refresh token kept");
    let kept: TokenData = serde_json::from_str(&s.home.item(&s.second).unwrap()).unwrap();
    assert_eq!(kept.access_token, "fake-access-2");
    assert_eq!(s.google.to("/token").len(), 3, "two logins and one refresh");
}

#[test]
fn an_account_that_needs_a_new_login_is_not_switched_to() {
    let _one = serial();
    let s = setup("agy-switch-relogin", Some("2.24.0"));
    index::set_standing(&s.home.app, &s.second, Some(Standing::NeedsLogin { since: NOW })).unwrap();
    let Outcome::Failed { error, .. } = s.run(How::Plain).outcome else { panic!() };
    assert_eq!(error, "needsLogin");

    // Google refusing the refresh during the preparation says the same.
    index::set_standing(&s.home.app, &s.second, None).unwrap();
    s.google.set(|g| g.refresh = (400, INVALID_GRANT.into()));
    let done = s.switch(How::Plain, NOW + 3500, Arc::new(NoProbe), limits());
    let Outcome::Failed { error, .. } = done.outcome else { panic!("{:?}", done.outcome) };
    assert_eq!(error, "needsLogin");
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert!(matches!(
        index::load_statuses(&s.home.app).accounts.get(&s.second),
        Some(Standing::NeedsLogin { .. })
    ));
}

#[test]
fn an_unknown_account_is_refused() {
    let _one = serial();
    let s = setup("agy-switch-unknown", Some("2.24.0"));
    let endpoints = s.google.endpoints();
    let desktop = s.app.desktop();
    let deps = Deps {
        app: &s.home.app,
        endpoints: &endpoints,
        keychain: &s.home.keychain,
        probe: Arc::new(NoProbe),
        shared: &s.shared,
        desktop: &desktop,
        agy: &s.agy,
    };
    let request = Request { id: "antigravity_nobody".into(), how: How::Plain, from: None };
    let done = switch_antigravity(&deps, &request, limits(), &|| NOW);
    let Outcome::Failed { error, number, .. } = done.outcome else { panic!() };
    assert_eq!((error, number), ("unknownAccount", 0));
    assert_eq!(s.shared_item(), OLD_LOGIN);
}

/// Answers the project lookup after `delay`, counting its calls.
struct SlowProbe {
    delay: Duration,
    calls: Mutex<u32>,
}

impl Probe for SlowProbe {
    fn probe(&self, _: &TokenData, _: &str) -> ProjectMeta {
        *self.calls.lock().unwrap() += 1;
        std::thread::sleep(self.delay);
        ProjectMeta { project_id: Some("fake-project".into()), is_gcp_tos: Some(false) }
    }
}

fn slow(delay: Duration) -> Arc<SlowProbe> {
    Arc::new(SlowProbe { delay, calls: Mutex::new(0) })
}

#[test]
fn the_project_is_looked_up_once_when_missing_and_kept() {
    let _one = serial();
    let s = setup("agy-switch-probe", Some("2.24.0"));
    let probe = slow(Duration::ZERO);
    let done = s.switch(How::Plain, NOW + 60, probe.clone(), limits());
    assert!(matches!(done.outcome, Outcome::Switched { .. }));
    let kept: TokenData = serde_json::from_str(&s.home.item(&s.second).unwrap()).unwrap();
    assert_eq!((kept.project_id.as_deref(), kept.is_gcp_tos), (Some("fake-project"), Some(false)));
    // Known now: the next switch does not ask again.
    s.switch(How::Plain, NOW + 120, probe.clone(), limits());
    assert_eq!(*probe.calls.lock().unwrap(), 1);
}

#[test]
fn the_switch_asks_load_code_assist_as_the_quota_read_does() {
    let _one = serial();
    let s = setup("agy-switch-cloud-code", Some("2.24.0"));
    let probe = || {
        Arc::new(CloudCodeProbe {
            hosts: s.google.hosts(),
            identity: AntigravityApp::at(s.home.home.join("no-bundle")).identity(),
        })
    };
    let done = s.switch(How::Plain, NOW + 60, probe(), limits());
    assert!(matches!(done.outcome, Outcome::Switched { .. }), "{:?}", done.outcome);
    let calls = s.google.calls(LOAD_CODE_ASSIST);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].header("authorization").as_deref(), Some("Bearer fake-access-b"));
    // Kept as the quota read keeps it: no mark from Google reads as no GCP ToS.
    let kept: TokenData = serde_json::from_str(&s.home.item(&s.second).unwrap()).unwrap();
    assert_eq!(
        (kept.project_id.as_deref(), kept.is_gcp_tos),
        (Some("fixture-project-1001"), Some(false))
    );
    // Known now: the next switch does not ask again.
    s.switch(How::Plain, NOW + 120, probe(), limits());
    assert_eq!(s.google.calls(LOAD_CODE_ASSIST).len(), 1);
}

#[test]
fn a_load_code_assist_that_fails_does_not_stop_the_switch() {
    let _one = serial();
    let s = setup("agy-switch-cloud-code-down", Some("2.24.0"));
    s.google.set(|g| g.load_code_assist = (503, r#"{"error":{"code":503}}"#.into()));
    let probe = Arc::new(CloudCodeProbe {
        hosts: s.google.hosts(),
        identity: AntigravityApp::at(s.home.home.join("no-bundle")).identity(),
    });
    let done = s.switch(How::Plain, NOW + 60, probe, limits());
    assert!(matches!(done.outcome, Outcome::Switched { .. }), "{:?}", done.outcome);
    assert!(!s.google.calls(LOAD_CODE_ASSIST).is_empty());
    assert_ne!(s.shared_item(), OLD_LOGIN);
    let kept: TokenData = serde_json::from_str(&s.home.item(&s.second).unwrap()).unwrap();
    assert_eq!(kept.is_gcp_tos, None, "asked again next time");
}

/// Logs the account out of the app while its token is being prepared, as
/// a logout from the other window would.
struct LogoutProbe {
    app: quota_station_core::app_dir::AppDir,
    endpoints: quota_station_core::antigravity::oauth::Endpoints,
    keychain: quota_station_core::antigravity::store::Keychain,
    id: String,
}

impl Probe for LogoutProbe {
    fn probe(&self, _: &TokenData, _: &str) -> ProjectMeta {
        let deps =
            login::Deps { app: &self.app, endpoints: &self.endpoints, keychain: &self.keychain };
        login::logout(&deps, &self.id).unwrap();
        ProjectMeta::default()
    }
}

#[test]
fn an_account_logged_out_while_it_is_prepared_is_not_switched_to() {
    let _one = serial();
    let s = setup("agy-switch-logout", Some("2.24.0"));
    let probe = Arc::new(LogoutProbe {
        app: s.home.app.clone(),
        endpoints: s.google.endpoints(),
        keychain: s.home.keychain.clone(),
        id: s.second.clone(),
    });
    let done = s.switch(How::Plain, NOW + 60, probe, limits());
    let Outcome::Failed { error, .. } = done.outcome else { panic!("{:?}", done.outcome) };
    assert_eq!(error, "unknownAccount");
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert!(s.shared_calls().is_empty());
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
    assert!(s.binding().is_none());
}

struct PanicProbe;

impl Probe for PanicProbe {
    fn probe(&self, _: &TokenData, _: &str) -> ProjectMeta {
        panic!("the lookup broke");
    }
}

#[test]
fn a_preparation_that_breaks_is_not_reported_as_slow() {
    let _one = serial();
    let s = setup("agy-switch-broken", Some("2.24.0"));
    let done = s.switch(How::Plain, NOW + 60, Arc::new(PanicProbe), limits());
    let Outcome::Failed { error, .. } = done.outcome else { panic!("{:?}", done.outcome) };
    assert_eq!(error, "prepare");
    assert_eq!(s.shared_item(), OLD_LOGIN);
}

#[test]
fn the_preparation_gives_up_after_its_limit_and_nothing_is_written() {
    let _one = serial();
    let s = setup("agy-switch-slow", Some("2.24.0"));
    let limits = Limits { prepare: Duration::from_millis(200), ..limits() };
    let started = std::time::Instant::now();
    let done = s.switch(How::Plain, NOW + 60, slow(Duration::from_secs(2)), limits);
    assert!(started.elapsed() < Duration::from_millis(1500));
    let Outcome::Failed { error, .. } = done.outcome else { panic!("{:?}", done.outcome) };
    assert_eq!(error, "prepareTimeout");
    assert_eq!(s.shared_item(), OLD_LOGIN);
    assert_eq!(s.current().as_deref(), Some(ACCOUNT_ID));
}

#[test]
fn a_second_switch_while_one_runs_is_refused_as_busy() {
    let _one = serial();
    let s = Arc::new(setup("agy-switch-busy", Some("2.24.0")));
    let first = {
        let s = s.clone();
        std::thread::spawn(move || {
            s.switch(How::Plain, NOW + 60, slow(Duration::from_millis(800)), limits())
        })
    };
    std::thread::sleep(Duration::from_millis(200));
    let second = s.run(How::Plain);
    assert_eq!(
        second.outcome,
        Outcome::Failed { number: 0, error: "busy", detail: String::new(), reopened: None }
    );
    assert!(matches!(first.join().unwrap().outcome, Outcome::Switched { .. }));
    // Once it is done, the next one runs.
    assert!(matches!(s.run(How::Plain).outcome, Outcome::Switched { .. }));
    assert_eq!(s.shared_calls().len(), 2);
}

#[test]
fn readings_after_the_switch_feed_the_six_hour_watch() {
    let _one = serial();
    let s = setup("agy-switch-watch", Some("2.24.0"));
    s.run(How::KeepRunning);
    let index = index::load(&s.home.app).unwrap();
    let read = |quota: &str, at: i64| {
        s.report(Some(quota));
        let report = listing::read(&s.agy, &s.home.app, 0, time::OffsetDateTime::now_utc());
        let active = switch::shown_active_id(&report, &index);
        watch::observe(&s.home.app, active.as_deref(), at)
    };
    // Antigravity.app wrote its own login back an hour later.
    assert!(read(&agy_quota(45.0, FIRST_RESET), NOW + 3600));
    let w = watch::load(&s.home.app).unwrap();
    assert_eq!((w.overwrites, w.alert()), (1, Some(NOW + 3600)));
    // A reading neither account's matches names no account.
    assert!(!read(&agy_quota(55.0, "2030-01-15T15:30:00Z"), NOW + 3700));

    // "Thôi" hides it; switching again starts a new watch keeping the count,
    // and agy reads the new account.
    watch::dismiss(&s.home.app).unwrap();
    assert_eq!(watch::load(&s.home.app).unwrap().alert(), None);
    s.report(Some(&agy_quota(25.0, SECOND_RESET)));
    s.switch(How::Plain, NOW + 3800, Arc::new(NoProbe), limits());
    let w = watch::load(&s.home.app).unwrap();
    assert_eq!((w.overwrites, w.dismissed, w.seen), (1, false, true));

    // A switch whose previous account is not known (the index already points
    // at the account) leaves no earlier watch to answer for it.
    s.switch_from(None, How::Plain, NOW + 3900, Arc::new(NoProbe), limits());
    assert!(watch::load(&s.home.app).is_none());
}
