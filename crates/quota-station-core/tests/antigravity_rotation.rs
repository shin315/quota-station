// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The Antigravity rotation and `quota-station antigravity …`, in a
//! temporary app folder with made-up accounts and readings, a fake world
//! that records the switches it is asked for instead of switching, and a
//! fake `launchctl`. Nothing here reaches Google, the Keychain,
//! Antigravity or agy.
#![cfg(feature = "antigravity-accounts")]

mod common;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use common::desktop::remember;
use common::temp_dir;
use quota_station_core::antigravity::cli::{self, Cli};
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary, Standing};
use quota_station_core::antigravity::rotate::{self, Decision, Fresh, Schedule, Settings, World};
use quota_station_core::antigravity::switch::{self, How, LogLine, Outcome};
use quota_station_core::antigravity::{listing, watch};
use quota_station_core::app_dir::AppDir;
use quota_station_core::quota::{agy, Provider, ProviderId, Response};
use time::OffsetDateTime;

const EMAILS: [&str; 3] = ["one@example.com", "two@example.com", "three@example.com"];
const RESET: &str = "2030-01-15T14:00:00Z";

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn now_ms() -> i64 {
    (now().unix_timestamp_nanos() / 1_000_000) as i64
}

/// The app's folder with three accounts, each with `used[i]` per cent of
/// Gemini used in a reading taken just now.
fn accounts(tag: &str, used: [f64; 3]) -> AppDir {
    let app = AppDir::new(temp_dir(tag).join("app"));
    let mut index = AccountIndex::new();
    for (i, email) in EMAILS.iter().enumerate() {
        index.accounts.push(AccountSummary {
            id: index::account_id(email),
            email: email.to_string(),
            name: Some(format!("Name {}", i + 1)),
            created_at: 0,
            last_used: 0,
        });
        remember(&app, email, used[i], RESET, now_ms());
    }
    index::save(&app, &index).unwrap();
    app
}

/// A switch asked for: the account, the one in use, the button.
type Asked = (String, Option<String>, Option<How>);

/// Reads as agy on the account `active` (from 1), and records the switches
/// it is asked for, answering each with `answer`.
struct FakeWorld {
    active: usize,
    answer: fn(usize) -> Outcome,
    asked: RefCell<Vec<Asked>>,
    reads: RefCell<Vec<Fresh>>,
}

impl FakeWorld {
    fn on(active: usize) -> FakeWorld {
        FakeWorld {
            active,
            answer: |n| Outcome::Switched {
                number: n,
                how: How::Plain,
                basis: Some("identity".into()),
                reopened: None,
                saved: true,
            },
            asked: RefCell::new(Vec::new()),
            reads: RefCell::new(Vec::new()),
        }
    }
}

impl World for FakeWorld {
    fn read(&self, app: &AppDir, fresh: Fresh, now: OffsetDateTime) -> Response {
        self.reads.borrow_mut().push(fresh);
        let mut row = Provider::fresh(ProviderId::Agy, Vec::new(), now);
        row.account_keys = vec![agy::account_hash(EMAILS[self.active - 1])];
        listing::attach(&mut row, app, now);
        Response::new(now, vec![row])
    }

    fn switch(&self, app: &AppDir, id: &str, from: Option<String>, how: Option<How>) -> Outcome {
        self.asked.borrow_mut().push((id.to_string(), from, how));
        let index = index::load(app).unwrap();
        let n = index.accounts.iter().position(|a| a.id == id).unwrap() + 1;
        (self.answer)(n)
    }
}

fn id(n: usize) -> String {
    index::account_id(EMAILS[n - 1])
}

/// A line of the switch log `minutes_ago`, as any switch leaves one.
fn switched_before(app: &AppDir, minutes_ago: i64) {
    let line = LogLine {
        at: now().unix_timestamp() - minutes_ago * 60,
        from: Some(id(1)),
        to: id(2),
        how: How::Plain,
        result: "switched".into(),
        error: None,
        reopened: None,
    };
    let text = format!("{}\n", serde_json::to_string(&line).unwrap());
    app.write_private(&switch::log_path(app), &text).unwrap();
}

#[test]
fn a_low_account_is_switched_to_the_one_with_the_most_gemini_and_logged() {
    let app = accounts("agy-rot-switch", [96.0, 40.0, 10.0]);
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), false);
    assert_eq!(run.decision, Decision::Switch { from: 1, to: 3, from_left: 4.0, to_left: 90.0 });
    assert_eq!(*world.asked.borrow(), vec![(id(3), Some(id(1)), None)]);
    assert_eq!(*world.reads.borrow(), vec![Fresh::Due]);
    assert!(matches!(run.outcome, Some(Outcome::Switched { number: 3, .. })));
    assert!(run.print().contains("switched to account 3"), "{}", run.print());

    let log = rotate::read_log(&app);
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].from, log[0].to, log[0].result.as_str()), (1, 3, "switched"));
    assert_eq!(log[0].threshold, 5.0);
    let last = rotate::last_run(&app).unwrap();
    assert_eq!(last["decision"], "switch");
    // Ids and numbers only: no email in the rotation's files.
    for path in [rotate::log_path(&app), rotate::state_path(&app)] {
        let text = std::fs::read_to_string(path).unwrap();
        assert!(EMAILS.iter().all(|e| !text.contains(e)), "{text}");
    }
}

#[test]
fn a_failed_switch_is_logged_with_its_error() {
    let app = accounts("agy-rot-failed", [100.0, 20.0, 50.0]);
    let mut world = FakeWorld::on(1);
    world.answer =
        |n| Outcome::Failed { number: n, error: "notQuit", detail: String::new(), reopened: None };
    let run = rotate::run(&app, &world, now(), false);
    assert!(matches!(run.outcome, Some(Outcome::Failed { number: 2, .. })));
    let log = rotate::read_log(&app);
    assert_eq!((log[0].result.as_str(), log[0].error.as_deref()), ("failed", Some("notQuit")));
}

#[test]
fn a_dry_run_says_what_it_would_do_and_writes_nothing() {
    let app = accounts("agy-rot-dry", [97.0, 40.0, 10.0]);
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), true);
    assert!(matches!(run.decision, Decision::Switch { to: 3, .. }));
    assert!(run.print().contains("would switch to account 3"), "{}", run.print());
    assert!(world.asked.borrow().is_empty());
    assert!(rotate::read_log(&app).is_empty());
    assert!(rotate::last_run(&app).is_none());
}

#[test]
fn above_the_threshold_nothing_is_switched() {
    let app = accounts("agy-rot-enough", [94.0, 0.0, 0.0]);
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), false);
    assert_eq!(run.decision, Decision::Enough { active: 1, left: 6.0 });
    assert!(world.asked.borrow().is_empty());
    assert!(rotate::read_log(&app).is_empty());
}

#[test]
fn the_threshold_comes_from_the_settings() {
    let app = accounts("agy-rot-threshold", [85.0, 0.0, 50.0]);
    let world = FakeWorld::on(1);
    assert!(matches!(rotate::run(&app, &world, now(), true).decision, Decision::Enough { .. }));
    let settings = Settings { threshold: 20.0, ..Settings::default() };
    rotate::save_settings(&app, &settings).unwrap();
    let run = rotate::run(&app, &world, now(), false);
    assert_eq!(run.decision, Decision::Switch { from: 1, to: 2, from_left: 15.0, to_left: 100.0 });
}

#[test]
fn no_switch_when_no_account_is_better() {
    let app = accounts("agy-rot-nobetter", [98.0, 99.0, 92.0]);
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), false);
    assert_eq!(
        run.decision,
        Decision::NoBetter { active: 1, left: 2.0, best: Some(3), best_left: Some(8.0) }
    );
    assert!(world.asked.borrow().is_empty());
}

#[test]
fn an_account_that_needs_a_login_is_never_switched_to() {
    let app = accounts("agy-rot-login", [99.0, 0.0, 30.0]);
    index::set_standing(&app, &id(2), Some(Standing::NeedsLogin { since: 0 })).unwrap();
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), false);
    assert!(matches!(run.decision, Decision::Switch { to: 3, .. }), "{:?}", run.decision);
}

#[test]
fn a_recent_switch_holds_the_rotation_until_the_cooldown_ends() {
    let app = accounts("agy-rot-cooldown", [99.0, 0.0, 30.0]);
    let world = FakeWorld::on(1);
    switched_before(&app, 10);
    let run = rotate::run(&app, &world, now(), false);
    assert!(matches!(run.decision, Decision::Cooldown { to: 2, .. }), "{:?}", run.decision);
    assert!(run.print().contains("too recent"), "{}", run.print());
    assert!(world.asked.borrow().is_empty());

    switched_before(&app, 31);
    let run = rotate::run(&app, &world, now(), false);
    assert!(matches!(run.decision, Decision::Switch { to: 2, .. }), "{:?}", run.decision);

    // A shorter cooldown set by the owner.
    let app = accounts("agy-rot-cooldown-set", [99.0, 0.0, 30.0]);
    rotate::save_settings(&app, &Settings { cooldown_minutes: 5, ..Settings::default() }).unwrap();
    switched_before(&app, 10);
    assert!(matches!(rotate::run(&app, &world, now(), true).decision, Decision::Switch { .. }));
}

#[test]
fn a_switch_undone_by_a_running_agy_is_not_fought() {
    let app = accounts("agy-rot-undone", [99.0, 0.0, 30.0]);
    // The last switch went from 1 to 2, 2 was seen in use, then 1 came back.
    watch::begin(&app, &id(1), &id(2), now().unix_timestamp() - 3600).unwrap();
    watch::observe(&app, Some(&id(2)), now().unix_timestamp() - 3000);
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), false);
    assert_eq!(run.decision, Decision::Overwritten { active: 1, overwrites: 1 });
    assert!(world.asked.borrow().is_empty());
    assert!(run.print().contains("undone"), "{}", run.print());

    // The owner answered it ("Thôi"): the rotation goes on.
    watch::dismiss(&app).unwrap();
    assert!(matches!(rotate::run(&app, &world, now(), false).decision, Decision::Switch { .. }));
}

#[test]
fn stale_readings_are_not_trusted() {
    let app = accounts("agy-rot-stale", [99.0, 0.0, 30.0]);
    // The account in use read an hour ago.
    remember(&app, EMAILS[0], 99.0, RESET, now_ms() - 3_600_000);
    let world = FakeWorld::on(1);
    let run = rotate::run(&app, &world, now(), false);
    assert_eq!(run.decision, Decision::ActiveUnread { active: 1 });
    assert!(world.asked.borrow().is_empty());
}

#[test]
fn with_one_account_or_none_there_is_nothing_to_rotate() {
    let app = AppDir::new(temp_dir("agy-rot-none").join("app"));
    let world = FakeWorld::on(1);
    assert!(matches!(
        rotate::run(&app, &world, now(), false).decision,
        Decision::NoAccounts { .. }
    ));
    assert!(world.reads.borrow().is_empty());
}

// The commands.

fn cli(tag: &str, used: [f64; 3], active: usize) -> (Cli, PathBuf) {
    let app = accounts(tag, used);
    let dir = app.path().parent().unwrap().to_path_buf();
    let cli = Cli {
        app,
        home: dir.join("home"),
        exe: dir.join("Quota Station.app/Contents/MacOS/quota-station"),
        world: Box::new(FakeWorld::on(active)),
        schedule: Schedule { launchctl: dir.join("no-launchctl") },
    };
    (cli, dir)
}

fn call(cli: &Cli, args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cli::run(cli, &args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

#[test]
fn status_lists_every_account_its_windows_and_the_one_in_use() {
    let (cli, _) = cli("agy-cli-status", [96.0, 40.0, 10.0], 2);
    let (code, out, err) = call(&cli, &["status", "--json"]);
    assert_eq!(code, 0, "{err}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["active"], 2);
    assert_eq!(v["activeBasis"], "identity");
    let accounts = v["accounts"].as_array().unwrap();
    assert_eq!(accounts.len(), 3);
    assert_eq!(accounts[0]["label"], "Name 1");
    assert_eq!(accounts[1]["active"], true);
    assert_eq!(accounts[0]["status"], "fresh");
    assert!(accounts[0]["ageSeconds"].as_f64().unwrap() < 60.0);
    assert_eq!(accounts[0]["scopes"][0]["scope"], "gemini");
    assert_eq!(accounts[0]["scopes"][0]["effectivePercentRemaining"], 4.0);
    assert_eq!(accounts[0]["windows"][0]["id"], "gemini_5h");
    assert_eq!(accounts[0]["windows"][0]["resetsAt"], "2030-01-15T14:00:00.000Z");
    assert_eq!(v["rotation"]["threshold"], 5.0);
    assert!(!out.contains("@example.com"), "no email without --emails: {out}");

    let (_, out, _) = call(&cli, &["status", "--json", "--emails"]);
    assert!(out.contains(EMAILS[0]), "{out}");
    let (code, text, _) = call(&cli, &["status"]);
    assert_eq!(code, 0);
    assert!(text.lines().any(|l| l.starts_with("*  2  Gemini  60%")), "{text}");
}

/// The fake, shared with the test that hands it to the commands.
struct Shared(Rc<FakeWorld>);

impl World for Shared {
    fn read(&self, app: &AppDir, fresh: Fresh, now: OffsetDateTime) -> Response {
        self.0.read(app, fresh, now)
    }

    fn switch(&self, app: &AppDir, id: &str, from: Option<String>, how: Option<How>) -> Outcome {
        self.0.switch(app, id, from, how)
    }
}

#[test]
fn status_reads_the_cache_unless_asked_to_refresh_and_switch_passes_the_button() {
    let world = Rc::new(FakeWorld::on(1));
    let (mut cli, _) = cli("agy-cli-refresh", [0.0; 3], 1);
    cli.world = Box::new(Shared(world.clone()));
    call(&cli, &["status"]);
    call(&cli, &["status", "--refresh"]);
    assert_eq!(*world.reads.borrow(), vec![Fresh::Cached, Fresh::All]);
    call(&cli, &["switch", "2", "--keep-running"]);
    call(&cli, &["switch", "3"]);
    let asked: Vec<Option<How>> = world.asked.borrow().iter().map(|a| a.2).collect();
    assert_eq!(asked, vec![Some(How::KeepRunning), None]);
}

#[test]
fn switch_takes_a_number_or_an_email_and_says_how_it_ended() {
    let (cli, _) = cli("agy-cli-switch", [96.0, 40.0, 10.0], 1);
    let (code, out, err) = call(&cli, &["switch", "3"]);
    assert_eq!((code, out.as_str()), (0, "switched to account 3\n"), "{err}");
    let (code, out, _) = call(&cli, &["switch", "TWO@example.com"]);
    assert_eq!((code, out.as_str()), (0, "switched to account 2\n"));
    let (code, out, _) = call(&cli, &["switch", "1"]);
    assert_eq!((code, out.as_str()), (0, "already on account 1\n"));
    let (code, _, err) = call(&cli, &["switch", "4"]);
    assert_eq!(code, 1);
    assert!(err.contains("no account 4"), "{err}");
    assert_eq!(call(&cli, &["switch"]).0, 2);
    let (code, out, _) = call(&cli, &["switch", "2", "--keep-running", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&out).unwrap()["kind"], "switched");
}

#[test]
fn a_failed_or_unconfirmed_switch_exits_non_zero() {
    let (mut cli, _) = cli("agy-cli-switch-fail", [0.0; 3], 1);
    let mut world = FakeWorld::on(1);
    world.answer = |n| Outcome::Failed {
        number: n,
        error: "appRunning",
        detail: String::new(),
        reopened: None,
    };
    cli.world = Box::new(world);
    let (code, out, _) = call(&cli, &["switch", "2"]);
    assert_eq!(code, 1);
    assert!(out.contains("failed (appRunning)"), "{out}");

    let mut world = FakeWorld::on(1);
    world.answer = |n| Outcome::Unconfirmed {
        number: n,
        how: How::QuitFirst,
        active: Some(1),
        reopened: Some(true),
        saved: true,
    };
    cli.world = Box::new(world);
    assert_eq!(call(&cli, &["switch", "2"]).0, 3);
}

#[test]
fn rotate_runs_the_rotation_once() {
    let (cli, _) = cli("agy-cli-rotate", [99.0, 40.0, 10.0], 1);
    let (code, out, _) = call(&cli, &["rotate", "--dry-run"]);
    assert_eq!(code, 0);
    assert!(out.contains("would switch to account 3"), "{out}");
    assert!(rotate::read_log(&cli.app).is_empty());
    let (code, out, _) = call(&cli, &["rotate"]);
    assert_eq!(code, 0);
    assert!(out.contains("switched to account 3"), "{out}");
    assert_eq!(rotate::read_log(&cli.app).len(), 1);
}

#[test]
fn rotation_settings_are_checked_and_kept() {
    let (cli, _) = cli("agy-cli-settings", [0.0; 3], 1);
    for bad in [&["rotation", "on", "--threshold", "x"][..], &["rotation", "on", "--threshold"]] {
        assert_eq!(call(&cli, bad).0, 1, "{bad:?}");
    }
    assert_eq!(call(&cli, &["rotation", "on", "--cooldown", "-3"]).0, 1);
    assert_eq!(rotate::load_settings(&cli.app), Settings::default(), "nothing kept");
    assert_eq!(call(&cli, &["rotation"]).0, 2);
    assert_eq!(call(&cli, &["nonsense"]).0, 2);
}

#[cfg(unix)]
mod launchd {
    use std::path::{Path, PathBuf};

    use common::script;

    use super::*;

    /// A `launchctl` that keeps loaded labels as files in `dir/loaded/`.
    fn fake_launchctl(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir.join("loaded")).unwrap();
        let d = dir.display();
        script(
            &dir.join("bin/launchctl"),
            &format!(
                r#"printf '%s\n' "$*" >> '{d}/launchctl-calls'
case "$1" in
  list) [ -e '{d}/loaded/'"$2" ] && exit 0; exit 113 ;;
  bootstrap) label=$(basename "$3" .plist); : > '{d}/loaded/'"$label" ;;
  bootout) label=${{2##*/}}; [ -e '{d}/loaded/'"$label" ] || exit 3; rm '{d}/loaded/'"$label" ;;
  *) exit 2 ;;
esac
"#
            ),
        )
    }

    #[test]
    fn rotation_on_loads_its_own_launch_agent_and_off_unloads_it() {
        let (mut cli, dir) = cli("agy-cli-rotation", [0.0; 3], 1);
        cli.schedule = Schedule { launchctl: fake_launchctl(&dir) };
        let plist = cli.home.join(format!("Library/LaunchAgents/{}.plist", rotate::LABEL));

        let (_, out, _) = call(&cli, &["rotation", "status", "--json"]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["on"], false, "off for a new owner");

        let (code, out, err) =
            call(&cli, &["rotation", "on", "--threshold", "8", "--cooldown", "45"]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("8% of Gemini"), "{out}");
        let text = std::fs::read_to_string(&plist).unwrap();
        assert!(text.contains(&format!("<string>{}</string>", cli.exe.display())), "{text}");
        assert!(text.contains("<string>antigravity</string>\n    <string>rotate</string>"));
        let settings = rotate::load_settings(&cli.app);
        assert_eq!((settings.threshold, settings.cooldown_minutes), (8.0, 45));

        let (_, out, _) = call(&cli, &["rotation", "status"]);
        assert!(out.starts_with("rotation on: switch at 8%"), "{out}");
        // The Claude rotation's agent is not touched.
        let calls = std::fs::read_to_string(dir.join("launchctl-calls")).unwrap();
        assert!(calls.lines().all(|l| !l.contains("quota-station.rotation")), "{calls}");

        assert_eq!(call(&cli, &["rotation", "off"]).0, 0);
        assert!(!plist.exists());
        let (_, out, _) = call(&cli, &["rotation", "status"]);
        assert!(out.starts_with("rotation off"), "{out}");
        // The settings outlive it.
        assert_eq!(rotate::load_settings(&cli.app).threshold, 8.0);
    }
}
