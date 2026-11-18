// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Thống kê for Antigravity (M16, J), on fake homes holding made-up
//! conversations: SQLite files with the shape of the `agy` command line's
//! (`steps` with a protocol buffer `metadata`), built here by
//! [`agy_log::wire`]. Nothing here reads the real `~/.gemini`.
//!
//! The trap: every step carries the same made-up text in its metadata and in
//! its payload. It must never reach the index, the report, or any file of
//! the app's.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
use quota_station_core::app_dir::AppDir;
use quota_station_core::history::{App, Cause, Entry, Source};
use quota_station_core::stats::agy_log::{self, wire};
use quota_station_core::stats::index::{self, Index, Roots};
use quota_station_core::stats::report::{self, Accounts, Period, Timeline};
use quota_station_core::stats::{Agent, Tokens};
use rusqlite::{params, Connection};
use time::{OffsetDateTime, UtcOffset};

const TRAP: &str = "TRAP-conversation-7c41e9";

/// 2026-01-16 07:20 UTC, 14:20 on a Mac at UTC+7.
const NOW: i64 = 1_768_548_000;
/// 12:00 that day on the Mac's clock.
const NOON: i64 = NOW - (2 * 3600 + 20 * 60);

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW).unwrap()
}

fn plus7(_: OffsetDateTime) -> UtcOffset {
    UtcOffset::from_hms(7, 0, 0).unwrap()
}

/// A fake Mac: a home with Antigravity's folder, and the app's folder.
struct Mac {
    home: PathBuf,
    app: AppDir,
}

impl Mac {
    fn new(tag: &str) -> Self {
        let root = temp_dir(tag);
        let home = root.join("home");
        std::fs::create_dir_all(home.join(agy_log::DIR).join(agy_log::CONVERSATIONS)).unwrap();
        Mac { home, app: AppDir::new(root.join("app")) }
    }

    fn roots(&self) -> Roots {
        Roots::new(&self.home, &self.home.join(".codex"))
    }

    fn db(&self, id: &str) -> PathBuf {
        self.home.join(agy_log::DIR).join(agy_log::CONVERSATIONS).join(format!("{id}.db"))
    }

    /// A conversation file with the `steps` table, and no step yet.
    fn conversation(&self, id: &str) -> Connection {
        let conn = Connection::open(self.db(id)).unwrap();
        conn.execute_batch(
            "CREATE TABLE steps (idx integer, step_type integer NOT NULL DEFAULT 0, \
             status integer NOT NULL DEFAULT 0, metadata blob, step_payload blob, \
             PRIMARY KEY (idx));",
        )
        .unwrap();
        conn
    }

    /// The summaries: which folder each conversation ran in.
    fn summaries(&self, rows: &[(&str, &str)]) {
        let conn = Connection::open(self.home.join(agy_log::DIR).join(agy_log::SUMMARIES)).unwrap();
        conn.execute_batch(
            "CREATE TABLE conversation_summaries (conversation_id text, title text NOT NULL \
             DEFAULT '', workspace_uris text NOT NULL, PRIMARY KEY (conversation_id));",
        )
        .unwrap();
        for (id, uris) in rows {
            conn.execute("INSERT INTO conversation_summaries (conversation_id, workspace_uris) VALUES (?1, ?2)", params![id, uris]).unwrap();
        }
    }

    /// Reads into the index on disk, as the app does; only Antigravity's
    /// folder is looked into.
    fn read(&self) -> Index {
        let mut ix = index::load(&self.app);
        let mut roots = self.roots();
        roots.claude.clear();
        roots.codex.clear();
        ix.update(&roots, NOW, &mut |_| {});
        index::save(&self.app, &ix).unwrap();
        ix
    }
}

fn put_call(conn: &Connection, idx: i64, at: i64, model: u64, tokens: [u64; 4], call: &str) {
    conn.execute(
        "INSERT OR REPLACE INTO steps (idx, step_type, metadata, step_payload) VALUES (?1, 15, ?2, ?3)",
        params![idx, wire::metadata(at, model, tokens, call, TRAP), TRAP.as_bytes()],
    )
    .unwrap();
}

fn put_quiet(conn: &Connection, idx: i64, at: i64) {
    conn.execute(
        "INSERT OR REPLACE INTO steps (idx, step_type, metadata, step_payload) VALUES (?1, 132, ?2, ?3)",
        params![idx, wire::quiet(at, TRAP), TRAP.as_bytes()],
    )
    .unwrap();
}

/// Marks a conversation as written to: the time it changed always differs
/// from the last, as it would a second later on a clock that counts coarsely.
fn bump(mac: &Mac, id: &str) {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let file = std::fs::File::options().append(true).open(mac.db(id)).unwrap();
    file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(n)).unwrap();
}

fn total(ix: &Index) -> Tokens {
    index::totals(ix).get(&Agent::Agy).copied().unwrap_or_default()
}

fn week(ix: &Index) -> report::Report {
    report::report(ix, Period::Week, now(), &plus7, &Timeline::default(), Path::new("/Users/demo"))
}

fn tokens(input: u64, output: u64, cache_write: u64, cache_read: u64) -> Tokens {
    Tokens { input, cache_write, cache_read, output }
}

#[test]
fn a_conversation_gives_the_tokens_of_each_model_call() {
    let mac = Mac::new("agy-calls");
    let conn = mac.conversation("c1");
    put_quiet(&conn, 0, NOON - 100);
    put_call(&conn, 1, NOON, 1528, [3_000, 500, 0, 60_000], "call-a");
    put_quiet(&conn, 2, NOON + 5);
    put_call(&conn, 3, NOON + 60, 1417, [200, 40, 100, 9_000], "call-b");
    drop(conn);
    mac.summaries(&[("c1", r#"["file:///Users/demo/My%20Projects/web-shop"]"#)]);

    let ix = mac.read();
    assert_eq!(total(&ix), tokens(3_200, 540, 100, 69_000));
    let mut models: Vec<&str> = ix.buckets.iter().map(|b| b.model.as_str()).collect();
    models.sort();
    assert_eq!(models, ["M417", "M528"]);
    assert!(ix.buckets.iter().all(|b| b.agent == Agent::Agy));
    assert!(ix.buckets.iter().all(|b| b.project == "/Users/demo/My Projects/web-shop"));
    assert_eq!(ix.skipped()[2].sessions, 0, "every step is a known one");
    assert_eq!(ix.logs_of(Agent::Agy), 1);
}

#[test]
fn the_conversation_text_never_reaches_the_index_or_the_report() {
    let mac = Mac::new("agy-trap");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON, 1528, [10, 20, 0, 30], "call-a");
    drop(conn);
    let ix = mac.read();
    let rep = week(&ix);
    let on_disk = std::fs::read_to_string(index::path(&mac.app)).unwrap();
    for text in [on_disk, serde_json::to_string(&ix).unwrap(), serde_json::to_string(&rep).unwrap()]
    {
        assert!(!text.contains(TRAP), "the text of a conversation was kept");
    }
}

#[test]
fn a_call_written_over_two_steps_counts_once_and_a_second_read_adds_only_what_is_new() {
    let mac = Mac::new("agy-twice");
    let conn = mac.conversation("c1");
    // One call's usage on two of its steps.
    put_call(&conn, 1, NOON, 1528, [100, 10, 0, 1_000], "call-a");
    put_call(&conn, 2, NOON, 1528, [100, 10, 0, 1_000], "call-a");
    let first = mac.read();
    assert_eq!(total(&first), tokens(100, 10, 0, 1_000));

    // Nothing written: nothing changes, and the file is not opened again.
    let again = mac.read();
    assert_eq!(total(&again), tokens(100, 10, 0, 1_000));

    // More steps are read from where the last read stopped.
    put_call(&conn, 3, NOON + 300, 1528, [50, 5, 0, 500], "call-b");
    bump(&mac, "c1");
    let more = mac.read();
    assert_eq!(total(&more), tokens(150, 15, 0, 1_500));
    drop(conn);
    // A read that finds the same steps (the file was touched) counts nothing twice.
    bump(&mac, "c1");
    assert_eq!(total(&mac.read()), tokens(150, 15, 0, 1_500));
}

#[test]
fn a_step_the_agent_finished_after_the_last_read_is_found() {
    let mac = Mac::new("agy-late");
    let conn = mac.conversation("c1");
    // The step is there with no call yet, and the next one is read.
    put_quiet(&conn, 1, NOON);
    put_call(&conn, 2, NOON + 10, 1528, [10, 1, 0, 100], "call-a");
    assert_eq!(total(&mac.read()), tokens(10, 1, 0, 100));
    // The first step is rewritten with its call.
    put_call(&conn, 1, NOON, 1528, [20, 2, 0, 200], "call-z");
    bump(&mac, "c1");
    assert_eq!(total(&mac.read()), tokens(30, 3, 0, 300));
}

#[test]
fn two_conversations_with_the_same_call_id_are_two_calls() {
    let mac = Mac::new("agy-ids");
    for id in ["c1", "c2"] {
        let conn = mac.conversation(id);
        put_call(&conn, 1, NOON, 1528, [10, 1, 0, 100], "same-id");
    }
    assert_eq!(total(&mac.read()), tokens(20, 2, 0, 200));
}

#[test]
fn a_usage_the_app_cannot_read_is_skipped_and_counted_never_guessed() {
    let mac = Mac::new("agy-unknown");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON, 1528, [10, 1, 0, 100], "call-a");
    // A usage with no time, and bytes that are not a protocol buffer.
    let usage = [wire::number(2, 999), wire::bytes(7, b"call-b")].concat();
    conn.execute(
        "INSERT INTO steps (idx, step_type, metadata) VALUES (2, 15, ?1)",
        params![wire::bytes(9, &usage)],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO steps (idx, step_type, metadata) VALUES (3, 15, ?1)",
        params![vec![0x0f_u8, 0xff, 0xff]],
    )
    .unwrap();
    drop(conn);
    let ix = mac.read();
    assert_eq!(total(&ix), tokens(10, 1, 0, 100), "only the readable call counts");
    let skipped = &ix.skipped()[2];
    assert_eq!((skipped.agent, skipped.sessions, skipped.lines), (Agent::Agy, 1, 2));
    assert!(skipped.since.is_some());
    // Read again, the same steps are not counted as skipped twice.
    let mut ix = ix;
    ix.update(&mac.roots(), NOW, &mut |_| {});
    assert_eq!(ix.skipped()[2].lines, 2);
}

#[test]
fn a_file_that_is_not_a_conversation_is_told_once() {
    let mac = Mac::new("agy-garbage");
    std::fs::write(mac.db("junk"), b"this is not a database at all, only some bytes").unwrap();
    let conn = Connection::open(mac.db("other")).unwrap();
    conn.execute_batch("CREATE TABLE notes (text text);").unwrap();
    drop(conn);
    let mut ix = mac.read();
    assert_eq!(ix.skipped()[2].sessions, 2);
    ix.update(&mac.roots(), NOW + 60, &mut |_| {});
    assert_eq!((ix.skipped()[2].sessions, ix.skipped()[2].lines), (2, 2));
    assert_eq!(total(&ix), Tokens::default());
}

#[test]
fn a_conversation_being_written_is_left_for_the_next_read() {
    let mac = Mac::new("agy-busy");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON, 1528, [10, 1, 0, 100], "call-a");
    // Another process holds the file's write lock.
    let writer = Connection::open(mac.db("c1")).unwrap();
    writer.execute_batch("BEGIN EXCLUSIVE;").unwrap();
    let ix = mac.read();
    assert_eq!(total(&ix), Tokens::default(), "nothing was read, and nothing was kept as read");
    assert_eq!(ix.skipped()[2].sessions, 0, "busy is not a format the app does not know");
    writer.execute_batch("COMMIT;").unwrap();
    assert_eq!(total(&mac.read()), tokens(10, 1, 0, 100));
}

#[test]
fn a_conversation_without_a_summary_has_no_folder_and_a_home_with_no_antigravity_nothing() {
    let mac = Mac::new("agy-nofolder");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON, 1528, [10, 1, 0, 100], "call-a");
    drop(conn);
    let ix = mac.read();
    assert!(ix.buckets.iter().all(|b| b.project.is_empty()));

    let bare =
        Mac { home: temp_dir("agy-bare").join("home"), app: AppDir::new(temp_dir("agy-bare-app")) };
    let ix = bare.read();
    assert!(ix.buckets.is_empty() && ix.logs.is_empty());
}

#[test]
fn the_folder_of_a_conversation_is_its_first_workspace_in_the_systems_own_form() {
    let mac = Mac::new("agy-uri");
    mac.summaries(&[
        ("a", r#"["file:///Users/demo/web-shop","file:///Users/demo/other"]"#),
        ("b", r#"["file:///C:/Users/demo/My%20Shop"]"#),
        ("c", r#"["https://example.com/not-a-folder"]"#),
        ("d", "[]"),
        ("e", "not json"),
    ]);
    let folders = agy_log::folders(&mac.home.join(agy_log::DIR).join(agy_log::SUMMARIES));
    assert_eq!(folders.get("a").map(String::as_str), Some("/Users/demo/web-shop"));
    assert_eq!(folders.get("b").map(String::as_str), Some(r"C:\Users\demo\My Shop"));
    assert_eq!(folders.len(), 2);
    assert!(agy_log::folders(&mac.home.join("missing.db")).is_empty());
}

#[test]
fn the_report_has_antigravity_beside_the_others_without_a_price() {
    let mac = Mac::new("agy-report");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON, 1528, [1_000, 200, 0, 5_000], "call-a");
    put_call(&conn, 2, NOON - 86_400, 1417, [100, 20, 0, 500], "call-b");
    drop(conn);
    let ix = mac.read();
    let rep = week(&ix);
    assert_eq!((rep.agy, rep.claude, rep.codex, rep.total), (6_820, 0, 0, 6_820));
    assert_eq!(rep.points.iter().map(|p| p.agy).sum::<u64>(), 6_820);
    assert_eq!(rep.points.last().unwrap().agy, 6_200, "today is the last column");
    assert_eq!(rep.usd, 0.0, "a call by quota has no API price");
    assert!(rep.unpriced.is_empty(), "and is not a model that is missing from the price table");
    assert!(rep.models.iter().all(|m| m.agent == Agent::Agy && m.usd.is_none()));
    assert_eq!(rep.models.len(), 2);
    let today = report::report(
        &ix,
        Period::Today,
        now(),
        &plus7,
        &Timeline::default(),
        Path::new("/Users/demo"),
    );
    assert_eq!(today.points[12].agy, 6_200, "the hour of noon, on the Mac's clock");
}

fn switch(at: i64, from: u32, to: u32) -> Entry {
    Entry {
        at: OffsetDateTime::from_unix_timestamp(at)
            .unwrap()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        app: App::Antigravity,
        source: Source::Observed,
        from: Some(from),
        to: Some(to),
        cause: Cause::Observed,
    }
}

#[test]
fn antigravitys_accounts_are_placed_by_the_switch_history_of_its_own() {
    let mac = Mac::new("agy-accounts");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON - 3600, 1528, [100, 0, 0, 0], "a");
    put_call(&conn, 2, NOON + 3600, 1528, [300, 0, 0, 0], "b");
    drop(conn);
    let ix = mac.read();
    let entries = [switch(NOON, 1, 2)];
    let accounts = Accounts {
        claude: Timeline::default(),
        agy: Timeline::of(&entries, App::Antigravity, Some(2)),
    };
    let rep =
        report::report_of(&ix, Period::Week, now(), &plus7, &accounts, Path::new("/Users/demo"));
    let rows: Vec<(Option<u32>, u64)> =
        rep.agy_accounts.iter().map(|r| (r.slot, r.tokens)).collect();
    assert_eq!(rows, [(Some(2), 300), (Some(1), 100)]);
    assert!(rep.accounts.is_empty(), "Claude's accounts are Claude's");

    // The Claude timeline does not take Antigravity's lines.
    assert!(Timeline::from_history(&entries, None).switches.is_empty());
    // With no history, the tokens are of an account nobody can name.
    let none = report::report(
        &ix,
        Period::Week,
        now(),
        &plus7,
        &Timeline::default(),
        Path::new("/Users/demo"),
    );
    assert_eq!(
        none.agy_accounts.iter().map(|r| (r.slot, r.tokens)).collect::<Vec<_>>(),
        [(None, 400)]
    );
}

#[test]
fn counts_older_than_the_kept_days_are_dropped() {
    let mac = Mac::new("agy-old");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOW - 70 * 86_400, 1528, [100, 0, 0, 0], "old");
    put_call(&conn, 2, NOW - 86_400, 1528, [7, 0, 0, 0], "new");
    drop(conn);
    assert_eq!(total(&mac.read()), tokens(7, 0, 0, 0));
}

#[test]
fn a_conversation_that_was_replaced_is_read_again_without_counting_a_call_twice() {
    let mac = Mac::new("agy-replaced");
    let conn = mac.conversation("c1");
    for i in 1..=40 {
        put_call(&conn, i, NOON + i, 1528, [10, 1, 0, 100], &format!("call-{i}"));
    }
    drop(conn);
    assert_eq!(total(&mac.read()), tokens(400, 40, 0, 4_000));
    // The file is rewritten smaller, under the same name.
    std::fs::remove_file(mac.db("c1")).unwrap();
    let conn = mac.conversation("c1");
    for i in 1..=3 {
        put_call(&conn, i, NOON + i, 1528, [10, 1, 0, 100], &format!("call-{i}"));
    }
    drop(conn);
    assert_eq!(
        total(&mac.read()),
        tokens(400, 40, 0, 4_000),
        "the three calls were counted already"
    );
}

#[test]
fn an_index_with_antigravitys_conversations_loads_back_as_it_was_saved() {
    let mac = Mac::new("agy-roundtrip");
    let conn = mac.conversation("c1");
    put_call(&conn, 1, NOON, 1528, [10, 1, 0, 100], "call-a");
    drop(conn);
    let saved = mac.read();
    let loaded = index::load(&mac.app);
    assert_eq!(loaded, saved);
    assert_eq!(loaded.logs[0].reader.agent(), Agent::Agy);
}
