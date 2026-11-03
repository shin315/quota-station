// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The owner's manual switch of the Claude account without an extension
//! around it, pinned as the windows, the journal, the History tab and a
//! terminal get it: the outcome's JSON, the journal's line, the history's
//! line and what `quota-station claude switch` prints, in every build.
//! Against a fake Keychain in a temporary home: these tests never touch the
//! Keychain, Claude Code's login or launchd.
//!
//! The expected texts were written by a probe of these same cases on the
//! code before the extension's socket ([`Hooks::manual_switch`]), in each
//! build.

mod common;

use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::cli::{self, Cli, Tools as CliTools};
use quota_station_core::claude::switch;
use quota_station_core::ext::{self, Fields, Hooks};
use quota_station_core::history;
use quota_station_core::switching::{switch_claude, Done, Tools};
use serde_json::Map;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic, Mac};

/// Accounts 2, 3 and 4, Claude Code on 2.
fn mac(tag: &str) -> Mac {
    let mac = Mac::new(tag);
    for (slot, email) in
        [(2, "lan@team.example"), (3, "lan.b@team.example"), (4, "lan.c@team.example")]
    {
        let login = credential(&format!("acc-{slot}"), &format!("ref-{slot}"), in_hours(5.0));
        mac.add(slot, email, "org-team", &login);
    }
    mac.log_in(
        &mac.store.credential(2).unwrap().unwrap(),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    mac
}

fn quiet() {}

/// The outcome as the window gets it.
fn json(done: &Done) -> String {
    serde_json::to_string(&done.outcome).unwrap()
}

/// `quota-station claude ARGS…` with `hooks`: the exit code, stdout and
/// stderr.
fn call(cli: &Cli, hooks: &dyn Hooks, args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cli::run(cli, hooks, &args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

/// The journal's last line, as written.
fn journal_tail(app: &AppDir) -> String {
    let text = std::fs::read_to_string(history::path(app)).unwrap();
    text.lines().last().unwrap().to_string()
}

/// The history's newest line, as the History tab gets it.
fn history_head(app: &AppDir, home: &std::path::Path, hooks: &dyn Hooks) -> String {
    let entries = history::collect(app, home, hooks, 1);
    serde_json::to_string(&entries[0]).unwrap()
}

/// 2026-01-16 07:20 UTC.
const T0: i64 = 1_768_548_000;

/// The switch to `slot` as the app makes it without an extension: what the
/// hooks do around it, asked with no argument of their own, then the
/// switch.
fn switch_to(mac: &Mac, slot: u32, settle: &dyn Fn()) -> Done {
    let hooks = ext::none();
    let around = hooks.manual_switch(&Map::new()).unwrap();
    assert!(around.is_none());
    switch_claude(&Tools { store: mac.store.clone() }, slot, around, settle, hooks)
}

fn cli_of(mac: &Mac) -> Cli {
    Cli {
        store: mac.store.clone(),
        tools: CliTools { claude: None, endpoints: FakeAnthropic::start().endpoints() },
    }
}

#[test]
fn without_an_extension_the_outcomes_are_the_apps_own() {
    let m = mac("switch-ext-plain");
    let done = switch_to(&m, 3, &quiet);
    assert_eq!(json(&done), r#"{"kind":"switched","slot":3}"#);
    assert_eq!(done.journal, Fields::new());
    assert_eq!(switch::active_slot(&m.store), Some(3));

    let other = || {
        switch::switch(&m.store, 4).unwrap();
    };
    assert_eq!(json(&switch_to(&m, 2, &other)), r#"{"kind":"raced","slot":2,"active":4}"#);

    let elsewhere = || {
        m.log_in(
            &credential("acc-x", "ref-x", in_hours(5.0)),
            &oauth("x@else.example", "uuid-x", "org-x"),
        )
    };
    assert_eq!(json(&switch_to(&m, 3, &elsewhere)), r#"{"kind":"unconfirmed","slot":3}"#);

    std::fs::create_dir(m.home.join(".claude.lock")).unwrap();
    assert_eq!(
        json(&switch_to(&m, 4, &quiet)),
        r#"{"kind":"failed","slot":4,"error":"busy","detail":"","active":null}"#
    );
}

#[test]
fn without_an_extension_the_journal_and_the_history_are_the_apps_own() {
    let m = mac("switch-ext-plain-history");
    let app = AppDir::new(m.dir.join("app"));
    history::manual_claude(&app, Some(2), 3, Fields::new(), "switched", T0).unwrap();
    assert_eq!(
        journal_tail(&app),
        r#"{"at":1768548000,"app":"claude","from":"2","to":"3","what":{"kind":"manual","result":"switched"}}"#
    );
    assert_eq!(
        history_head(&app, &m.home, ext::none()),
        r#"{"at":"2026-01-16T07:20:00Z","app":"claude","source":"app","from":2,"to":3,"cause":{"kind":"manual","result":"switched"}}"#
    );
}

#[test]
fn without_an_extension_the_terminal_switch_is_the_apps_own() {
    let m = mac("switch-ext-plain-cli");
    let cli = cli_of(&m);
    assert_eq!(
        call(&cli, ext::none(), &["switch", "3"]),
        (0, "switched to account 3\n".to_string(), String::new())
    );
    assert_eq!(switch::active_slot(&m.store), Some(3));
}
