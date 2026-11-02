// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's test hooks (`quota_station_core::ext::Hooks`): each core
//! socket's tests fill its hook here and check that the core calls it, and
//! that `NoHooks` leaves the core as it is.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use quota_station_core::alerts::{
    self, act, Alert, ExtAnswer, ExtKind, ExtPass, ExtSource, Kind, Switches, Tone,
};
use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::cli::Cli;
use quota_station_core::claude::Paths;
use quota_station_core::ext::Fields;
use quota_station_core::ext::Hooks;
use quota_station_core::history::{App, ExtCause, ExtLine};
use quota_station_core::labels::Labels;
use quota_station_core::quota::claude::PoolPolicy;
use quota_station_core::quota::{ClaudeAccounts, Provider, Response, Slot};
use quota_station_core::settings::Settings;
use quota_station_core::snapshot::{Account, PoolKind, Section, Snapshot};
use serde_json::{json, Map, Value};
use time::{OffsetDateTime, UtcOffset};

/// Hooks a test can see the core call.
///
/// The settings file: they keep their own keys, all named `test…`. A file
/// is settled once it has `testSettled`, which a new install writes off and
/// a Mac that had the app on; every open notes whether a file was read
/// (`testFound`) and drops `testDropped`. While `testQuiet` is on, alert 1
/// does not run.
///
/// `quota-station claude …`: they add `test-word` (and would take `status`
/// too, were it ever asked), with a usage of their own.
///
/// The alerts: one kind of their own, `testKind` ([`TEST_KINDS`]), whose
/// cards [`TestSource`] draws, and its card for "Gửi thử".
///
/// The Claude pool: their policy ([`TestPolicy`]) counts the accounts with
/// an odd number only, and alert 1's card has their own words once
/// "dismiss" is pressed ([`TEST_POOL_ANSWERS`]).
///
/// The accounts' names: while `testLabels` is on in the settings, their own
/// ([`TestLabels`]); otherwise the app's.
///
/// The snapshot: on every account of every section, `testNumber` (ten
/// times its number) after its number and `testEnd` at its end; on Claude's
/// section, `testListKeys` (the keys of their own the report's list of
/// accounts had) after `stale`; and where that list has `testPool`, Claude's
/// pool is theirs (kind `testPool`, `testPool`'s value as its percent, and
/// `testWhole` after its kind).
///
/// The switch history: the lines in `test-history.json` under the home
/// ([`TEST_HISTORY`]), from their own source (`testSource`).
#[derive(Debug, Default)]
pub struct TestHooks;

impl Hooks for TestHooks {
    fn settled_key(&self) -> Option<&'static str> {
        Some("testSettled")
    }

    fn tidy_settings(&self, ext: &mut Map<String, Value>, found: bool) {
        ext.insert("testFound".into(), Value::Bool(found));
        ext.remove("testDropped");
    }

    fn settle_settings(&self, ext: &mut Map<String, Value>, had_app: bool) {
        ext.insert("testSettled".into(), Value::Bool(had_app));
    }

    fn alert_switches(&self, settings: &Settings, mut on: Switches) -> Switches {
        if settings.ext.get("testQuiet") == Some(&Value::Bool(true)) {
            on.pool = false;
        }
        on
    }

    /// `test-word ARGS…` prints `test ARGS…` and a note on stderr, and
    /// exits 3; `status` would exit 9; anything else is not theirs.
    fn claude_command(
        &self,
        cli: &Cli,
        args: &[String],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Option<i32> {
        let _ = cli;
        match args.first().map(String::as_str) {
            Some("test-word") => {
                let _ = writeln!(out, "test {}", args[1..].join(" "));
                let _ = writeln!(err, "noted");
                Some(3)
            }
            Some("status") => Some(9),
            _ => None,
        }
    }

    fn claude_usage(&self) -> Option<&'static str> {
        Some(TEST_USAGE)
    }

    fn alert_kinds(&self) -> &'static [ExtKind] {
        &TEST_KINDS
    }

    /// `testKind`'s sample names the first Claude account; a name of no
    /// kind of theirs has none.
    fn alert_sample(
        &self,
        name: &str,
        now: i64,
        offset: UtcOffset,
        claude: [&str; 3],
    ) -> Option<Alert> {
        (name == TEST_KIND).then(|| Alert {
            key: "ignored".into(),
            kind: Kind::Pool,
            tone: Tone::Info,
            title: format!("Test sample for {} at {}", claude[0], alerts::clock(now, offset)),
            body: "Thử: a test event.".into(),
            actions: vec![act("dismiss", "Fine")],
            at: 0,
            test: false,
        })
    }

    fn claude_pool(&self, paths: &Paths) -> Option<Box<dyn PoolPolicy>> {
        Some(Box::new(TestPolicy { home: paths.home.clone() }))
    }

    fn pool_answers(&self) -> &'static [ExtAnswer] {
        &TEST_POOL_ANSWERS
    }

    fn account_labels(&self, settings: &Settings) -> Option<Box<dyn Labels>> {
        let on = settings.ext.get("testLabels") == Some(&Value::Bool(true));
        on.then(|| Box::new(TestLabels) as Box<dyn Labels>)
    }

    fn decorate_snapshot(&self, snapshot: &mut Snapshot, claude: Option<&Response>) {
        for section in [&mut snapshot.claude, &mut snapshot.agy, &mut snapshot.codex] {
            for a in &mut section.accounts {
                a.ext.insert_after("number", "testNumber", json!(a.number * 10));
                a.ext.insert("testEnd", json!(true));
            }
        }
        let list = claude
            .and_then(|r| r.primary("claude"))
            .and_then(|row| row.claude_accounts.as_ref())
            .map_or_else(Fields::new, |l| l.ext.clone());
        let keys: Vec<&str> = list.iter().map(|(k, _)| k).collect();
        snapshot.claude.ext.insert_after("stale", "testListKeys", json!(keys));
        if let Some(percent) = list.get("testPool").and_then(Value::as_f64) {
            let pool = &mut snapshot.claude.pool;
            pool.kind = PoolKind::Ext("testPool");
            pool.percent = Some(percent);
            pool.ext.insert_after("kind", "testWhole", json!(true));
        }
    }

    /// Each line of [`TEST_HISTORY`]: a Claude line at `at` to `to`, a
    /// switch when `switch` is true, whose reason is `testCause` with its
    /// `note`.
    fn history_lines(&self, home: &Path, app: &AppDir) -> Vec<ExtLine> {
        let _ = app;
        let Ok(text) = std::fs::read_to_string(home.join(TEST_HISTORY)) else { return Vec::new() };
        let lines: Vec<Value> = serde_json::from_str(&text).unwrap();
        lines
            .iter()
            .map(|l| ExtLine {
                at: l["at"].as_i64().unwrap(),
                app: App::Claude,
                source: TEST_SOURCE,
                from: None,
                to: l["to"].as_u64().map(|n| n as u32),
                cause: ExtCause::new(
                    &json!({ "kind": "testCause", "note": l["note"] }),
                    l["switch"] == json!(true),
                )
                .unwrap(),
            })
            .collect()
    }
}

/// Where the test hooks read their lines of the history, under the home: a
/// JSON array of `{ at, to, switch, note }`.
pub const TEST_HISTORY: &str = "test-history.json";

/// The source the test hooks' lines of the history go by.
pub const TEST_SOURCE: &str = "testSource";

/// The test hooks' names for the accounts: a Claude account with an odd
/// number goes by `Test N of M` (M the accounts there are), one with an
/// even number keeps the app's name; an Antigravity account goes by
/// `Test agy N`; the cards "Gửi thử" sends name `Test one`, `Test two` and
/// `Test three`; and alert 1 has its own title.
#[derive(Debug, Default)]
pub struct TestLabels;

impl Labels for TestLabels {
    fn claude(&self, section: &Section, account: &Account) -> Option<String> {
        let all = section.accounts.len();
        (account.number % 2 == 1).then(|| format!("Test {} of {all}", account.number))
    }

    fn agy(&self, number: u32) -> Option<String> {
        Some(format!("Test agy {number}"))
    }

    fn claude_samples(&self) -> Option<[&'static str; 3]> {
        Some(["Test one", "Test two", "Test three"])
    }

    fn pool_title(&self) -> Option<&'static str> {
        Some("Test pool running dry")
    }
}

/// The test hooks' words under alert 1's card once "dismiss" is pressed.
pub const TEST_POOL_ANSWERS: [ExtAnswer; 1] = [ExtAnswer {
    action: "dismiss",
    title: "Test pool noted",
    body: "Not again until the test pool has room.",
}];

/// What the test policy saw at one read of the accounts the app keeps
/// ([`TestPolicy::room`]): the home it was read for, the account in use and
/// the accounts' numbers.
pub type PoolRead = (PathBuf, Option<u32>, Vec<u32>);

/// Every [`PoolRead`] so far.
pub static POOL_READS: Mutex<Vec<PoolRead>> = Mutex::new(Vec::new());

/// The reads [`POOL_READS`] holds for `home`.
pub fn pool_reads(home: &Path) -> Vec<(Option<u32>, Vec<u32>)> {
    let reads = POOL_READS.lock().unwrap_or_else(|e| e.into_inner());
    reads.iter().filter(|r| r.0 == home).map(|r| (r.1, r.2.clone())).collect()
}

/// The test hooks' policy of the Claude pool, read for `home`: an account
/// counts while its number is odd. At a read it notes what it saw
/// ([`POOL_READS`]) and writes nothing.
pub struct TestPolicy {
    pub home: PathBuf,
}

impl PoolPolicy for TestPolicy {
    fn room(&self, _row: &Provider, accounts: &mut ClaudeAccounts, _now: OffsetDateTime) {
        let slots = accounts.slots.iter().map(|s| s.slot).collect();
        let mut reads = POOL_READS.lock().unwrap_or_else(|e| e.into_inner());
        reads.push((self.home.clone(), accounts.active_slot, slots));
    }

    fn counts(
        &self,
        _accounts: &ClaudeAccounts,
        slot: &Slot,
        _now: OffsetDateTime,
        _offset: UtcOffset,
    ) -> bool {
        slot.slot % 2 == 1
    }
}

/// The test hooks' kind of alert, by name.
pub const TEST_KIND: &str = "testKind";

/// Their one kind: number 2, which comes between the app's 1 and 3; off by
/// default; and their own words under a card once "dismiss" is pressed.
pub const TEST_KINDS: [ExtKind; 1] = [ExtKind {
    number: 2,
    name: TEST_KIND,
    on: false,
    answers: &[ExtAnswer {
        action: "dismiss",
        title: "Test noted",
        body: "Not again for this test event.",
    }],
}];

/// The test hooks' kind.
pub fn test_kind() -> Kind {
    Kind::Ext(TEST_KIND.into())
}

/// The test extension's alerts at one pass, from what it "read": a
/// `testKind` card for each Claude account in `events` (named as the cards
/// name it, with a button to switch to it), and one of a kind no hooks have
/// (`testStranger`), which never goes out. It counts its passes in the
/// alerts' memory (`testPasses`).
pub struct TestSource {
    pub events: Vec<u32>,
}

impl ExtSource for TestSource {
    fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert> {
        let passes = pass.memory.get("testPasses").and_then(Value::as_u64).unwrap_or(0);
        pass.memory.insert("testPasses".into(), (passes + 1).into());
        let now = pass.facts.now.unix_timestamp();
        let card = |key: String, kind: Kind, title: String, actions| Alert {
            key,
            kind,
            tone: Tone::Warn,
            title,
            body: "A test event.".into(),
            actions,
            at: now,
            test: false,
        };
        let mut out: Vec<Alert> = self
            .events
            .iter()
            .map(|n| {
                let name = alerts::name_of(pass.names, *n);
                let actions = vec![act("dismiss", "Fine"), act(format!("switch:{n}"), "Go")];
                card(format!("test:{n}"), test_kind(), format!("Test event for {name}"), actions)
            })
            .collect();
        out.push(card("stranger".into(), Kind::Ext("testStranger".into()), "Never".into(), vec![]));
        out
    }
}

/// The usage the test hooks give `quota-station claude`.
pub const TEST_USAGE: &str = "usage: quota-station claude test-word ARGS...\n";
