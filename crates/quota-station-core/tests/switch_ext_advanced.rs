// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The owner's manual switch of the Claude account with the advanced part
//! around it (the core's `switch_ext` has it without an extension), pinned
//! as the windows, the journal, the History tab and a terminal get it: the
//! outcome's JSON, the journal's line, the history's line and what
//! `quota-station claude switch` prints. Against a fake Keychain and a fake
//! launchd in a temporary home: these tests never touch the Keychain,
//! Claude Code's login or launchd.
//!
//! The expected texts were written by a probe of these same cases on the
//! code before the extension's socket
//! ([`Hooks::manual_switch`](quota_station_core::ext::Hooks::manual_switch)).

mod common;

use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::cli::{self, Cli, Tools as CliTools};
use quota_station_core::claude::switch;
use quota_station_core::ext::{Fields, Hooks};
use quota_station_core::history;
use quota_station_core::switching::{switch_claude, Done, Tools};
use serde_json::Map;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::Mac;

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

// With the advanced part around the switch: the rotation's pause as the
// owner's hold, the older switcher's refusal, and the rotation's switch
// back named in the outcome.
mod private {
    use super::*;
    use quota_station_core::advanced::cli::CliHooks;
    use quota_station_core::advanced::history::HistoryHooks;
    use quota_station_core::advanced::pool::PoolHooks;
    use quota_station_core::advanced::switch::{SwitchHooks, HOLD_UNTIL};
    use quota_station_core::claude::rotate;
    use quota_station_core::local_time;
    use quota_station_core::pause;
    use serde_json::json;
    use std::path::PathBuf;
    use time::format_description::well_known::Rfc3339;
    use time::{Duration, OffsetDateTime};

    fn switch_to(mac: &Mac, slot: u32, hold: Option<OffsetDateTime>, settle: &dyn Fn()) -> Done {
        switch_with(mac, pause::path(&mac.home), slot, hold, settle)
    }

    fn switch_with(
        mac: &Mac,
        pause_file: PathBuf,
        slot: u32,
        hold: Option<OffsetDateTime>,
        settle: &dyn Fn(),
    ) -> Done {
        let hooks = SwitchHooks { scheduler: mac.scheduler.clone(), pause_file: Some(pause_file) };
        let mut asked = Map::new();
        if let Some(hold) = hold {
            asked.insert(HOLD_UNTIL.into(), json!(hold.format(&Rfc3339).unwrap()));
        }
        let around = hooks.manual_switch(&asked).unwrap();
        switch_claude(&Tools { store: mac.store.clone() }, slot, around, settle, &PoolHooks)
    }

    fn cli_of(mac: &Mac) -> (Cli, CliHooks) {
        let cli = Cli {
            store: mac.store.clone(),
            tools: CliTools { claude: None, endpoints: FakeAnthropic::start().endpoints() },
        };
        let switch = SwitchHooks {
            scheduler: mac.scheduler.clone(),
            pause_file: Some(pause::path(&mac.home)),
        };
        (cli, CliHooks { exe: mac.home.join("fakes/bin/quota-station"), switch })
    }

    /// The end of a hold until `until`, as the outcome says it.
    fn shown(until: OffsetDateTime) -> String {
        local_time::instant(&local_time::minute(until)).unwrap().format(&Rfc3339).unwrap()
    }

    #[test]
    fn the_private_part_holds_and_reports_around_a_switch() {
        let m = mac("switch-ext-private");
        // A hold the window cannot have meant refuses the switch at once.
        let hooks = SwitchHooks { scheduler: m.scheduler.clone(), pause_file: None };
        let mut asked = Map::new();
        asked.insert(HOLD_UNTIL.into(), json!("soon"));
        assert_eq!(hooks.manual_switch(&asked).err(), Some("badTime".to_string()));

        let done = switch_to(&m, 3, None, &quiet);
        assert_eq!(json(&done), r#"{"kind":"switched","slot":3,"hold":null}"#);
        assert_eq!(done.journal, Fields::new());
        assert!(!pause::path(&m.home).exists());

        let past = OffsetDateTime::now_utc() - Duration::minutes(1);
        assert_eq!(
            json(&switch_to(&m, 2, Some(past), &quiet)),
            r#"{"kind":"switched","slot":2,"hold":null}"#
        );
        assert!(!pause::path(&m.home).exists());

        let hour = OffsetDateTime::now_utc() + Duration::hours(1);
        let done = switch_to(&m, 3, Some(hour), &quiet);
        assert_eq!(
            json(&done).replace(&shown(hour), "<until>"),
            r#"{"kind":"switched","slot":3,"hold":{"until":"<until>","failed":false}}"#
        );
        // The journal keeps the hold written.
        assert_eq!(
            serde_json::to_string(&done.journal).unwrap().replace(&shown(hour), "<until>"),
            r#"{"holdUntil":"<until>"}"#
        );
        assert!(pause::read(&pause::path(&m.home)).unwrap().is_some());
        std::fs::remove_file(pause::path(&m.home)).unwrap();

        let rotation = || {
            switch::switch(&m.store, 3).unwrap();
            rotate::log(&m.store, "switch to slot 3 (use it first)", OffsetDateTime::now_utc());
        };
        let text = json(&switch_to(&m, 4, Some(hour), &rotation));
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let at = value["bySwitcherAt"].as_str().expect("the rotation's switch").to_string();
        assert!(OffsetDateTime::parse(&at, &Rfc3339).is_ok());
        assert_eq!(
            text.replace(&shown(hour), "<until>").replace(&at, "<at>"),
            r#"{"kind":"raced","slot":4,"active":3,"bySwitcherAt":"<at>","hold":{"until":"<until>","failed":false}}"#
        );
        std::fs::remove_file(pause::path(&m.home)).unwrap();

        let other = || {
            switch::switch(&m.store, 4).unwrap();
        };
        assert_eq!(
            json(&switch_to(&m, 2, None, &other)),
            r#"{"kind":"raced","slot":2,"active":4,"bySwitcherAt":null,"hold":null}"#
        );

        let elsewhere = || {
            m.log_in(
                &credential("acc-x", "ref-x", in_hours(5.0)),
                &oauth("x@else.example", "uuid-x", "org-x"),
            )
        };
        assert_eq!(
            json(&switch_to(&m, 3, None, &elsewhere)),
            r#"{"kind":"unconfirmed","slot":3,"hold":null}"#
        );
        m.log_in(
            &m.store.credential(3).unwrap().unwrap(),
            &oauth("lan.b@team.example", "uuid-3", "org-team"),
        );

        std::fs::write(m.home.join("blocked"), "").unwrap();
        let done = switch_with(&m, m.home.join("blocked/pause"), 2, Some(hour), &quiet);
        assert_eq!(
            json(&done).replace(&shown(hour), "<until>"),
            r#"{"kind":"switched","slot":2,"hold":{"until":"<until>","failed":true}}"#
        );
        // A hold not written is not kept.
        assert_eq!(done.journal, Fields::new());

        std::fs::create_dir(m.home.join(".claude.lock")).unwrap();
        assert_eq!(
            json(&switch_to(&m, 4, Some(hour), &quiet)),
            r#"{"kind":"failed","slot":4,"error":"busy","detail":"","active":null}"#
        );
        assert!(!pause::path(&m.home).exists());
        std::fs::remove_dir(m.home.join(".claude.lock")).unwrap();

        m.load_label("com.cswap.prefer-team");
        assert_eq!(
            json(&switch_to(&m, 4, Some(hour), &quiet)),
            r#"{"kind":"failed","slot":4,"error":"legacy","detail":"com.cswap.prefer-team","active":null}"#
        );
        assert_eq!(switch::active_slot(&m.store), Some(2));
        assert!(!pause::path(&m.home).exists());
    }

    #[test]
    fn the_private_part_keeps_the_hold_in_the_journal_and_the_history() {
        let m = mac("switch-ext-private-history");
        let app = AppDir::new(m.dir.join("app"));
        let mut held = Fields::new();
        held.insert(HOLD_UNTIL, json!("2026-01-16T08:20:00Z"));
        history::manual_claude(&app, Some(2), 3, held, "switched", T0).unwrap();
        assert_eq!(
            journal_tail(&app),
            r#"{"at":1768548000,"app":"claude","from":"2","to":"3","what":{"kind":"manual","holdUntil":"2026-01-16T08:20:00Z","result":"switched"}}"#
        );
        assert_eq!(
            history_head(&app, &m.home, &HistoryHooks),
            r#"{"at":"2026-01-16T07:20:00Z","app":"claude","source":"app","from":2,"to":3,"cause":{"kind":"manual","holdUntil":"2026-01-16T08:20:00Z","result":"switched"}}"#
        );
        history::manual_claude(&app, Some(3), 4, Fields::new(), "raced", T0 + 60).unwrap();
        assert_eq!(
            journal_tail(&app),
            r#"{"at":1768548060,"app":"claude","from":"3","to":"4","what":{"kind":"manual","result":"raced"}}"#
        );
        assert_eq!(
            history_head(&app, &m.home, &HistoryHooks),
            r#"{"at":"2026-01-16T07:21:00Z","app":"claude","source":"app","from":3,"to":4,"cause":{"kind":"manual","holdUntil":null,"result":"raced"}}"#
        );
    }

    #[test]
    fn the_private_parts_terminal_switch_waits_for_the_older_switcher() {
        let m = mac("switch-ext-private-cli");
        let (cli, hooks) = cli_of(&m);
        assert_eq!(
            call(&cli, &hooks, &["switch", "3"]),
            (0, "switched to account 3\n".to_string(), String::new())
        );
        m.load_label("com.cswap.prefer-team");
        assert_eq!(
            call(&cli, &hooks, &["switch", "4"]),
            (
                1,
                String::new(),
                "quota-station claude switch: com.cswap.prefer-team is loaded; stop it first \
                 (see the README's cutover)\n"
                    .to_string()
            )
        );
        assert_eq!(switch::active_slot(&m.store), Some(3));
    }
}
