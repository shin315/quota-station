// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The reader thread: runs quota-axi on the schedule and pushes each result
//! to the menu bar and the windows. It also looks at the switcher: the pause
//! flag on every read, and the dry run, log and launchd while a window is
//! open or after the owner acted. Before quota-axi's Claude read it reads
//! the Claude accounts the app keeps (each account's windows, and its name
//! from the Keychain while names show) and writes the files quota-axi and
//! Firstmate read; beside it runs quota-axi's Codex read where this Mac has
//! a Codex folder. Each read also wakes the Antigravity
//! quota thread for the account in use, and each Antigravity reading goes
//! to the six-hour watch after an Antigravity switch. After each read the
//! history and the alerts look at the new numbers (`alerting`).

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use quota_station_core::axi::Response;
use quota_station_core::claude::keychain::Security;
use quota_station_core::claude::state::AccountOut;
use quota_station_core::claude::store::Store;
use quota_station_core::claude::usage::{self, Endpoints};
use quota_station_core::claude::{agent, rotate, state, switch, Paths};
use quota_station_core::detail::ClaudeDetail;
use quota_station_core::runner::{self, Env, ProviderId, RunError};
use quota_station_core::settings::Menubar;
use quota_station_core::switcher;
use quota_station_core::{icon, local_time, pause, schedule};
use tauri::{AppHandle, Emitter};
use time::OffsetDateTime;

use crate::state::{AppState, DryRunRecord, Inner};
use crate::tray;

/// The longest single wait, so a sleep of the Mac is noticed soon after it wakes.
const WAIT_STEP: Duration = Duration::from_secs(30);

pub fn spawn(app: AppHandle, state: AppState) {
    thread::Builder::new()
        .name("quota-reader".into())
        .spawn(move || run(&app, &state))
        .expect("reader thread starts");
}

fn run(app: &AppHandle, state: &AppState) {
    loop {
        let mut inner = state.lock();
        let wait = if inner.read_now || inner.switcher_now {
            Duration::ZERO
        } else {
            inner.schedule.due_in(Instant::now())
        };
        if !wait.is_zero() {
            let (mono, wall) = (Instant::now(), SystemTime::now());
            let (mut inner, _) = state
                .wake
                .wait_timeout(inner, wait.min(WAIT_STEP))
                .unwrap_or_else(|e| e.into_inner());
            let wall = wall.elapsed().unwrap_or_default();
            if schedule::slept(mono.elapsed(), wall) {
                inner.read_now = true;
            }
            continue;
        }
        let home = inner.home.clone();
        // A switcher look on its own, between reads.
        let read_due = inner.read_now || inner.schedule.due_in(Instant::now()).is_zero();
        let pause_generation = inner.switcher.pause_generation;
        if !read_due {
            inner.switcher_now = false;
            drop(inner);
            let check = check_switcher(&home, true, pause_generation);
            check.store(&mut state.lock());
            publish(app, state);
            continue;
        }
        let full = inner.switcher_now || inner.schedule.visible();
        inner.switcher_now = false;
        inner.read_now = false;
        inner.reading = true;
        let path = inner.quota_axi();
        let accounts = inner.claude_store();
        let names = !inner.settings.hide_names;
        let env = inner.claude_paths().quota_axi_env();
        let generation = inner.claude_generation;
        let agy_generation = inner.agy_generation;
        let check_version =
            path.is_some() && inner.version.as_ref().map(|(p, _)| p) != path.as_ref();
        let codex_path = path.clone().filter(|_| inner.codex_home().is_dir());
        drop(inner);
        publish(app, state);

        // The switcher check runs beside the read, and the numbers are shown
        // as soon as they are in: a slow dry run never holds them back.
        let switcher = thread::spawn(move || check_switcher(&home, full, pause_generation));
        let codex = thread::spawn(move || codex_path.map(|p| runner::read(&p, ProviderId::Codex)));
        let (claude, agy, own) = read_all(path.clone(), &accounts, names, &env);
        let codex =
            codex.join().unwrap_or_else(|_| Some(Err(RunError::Spawn("reader panicked".into()))));
        let version = path.filter(|_| check_version).map(|p| {
            let v = runner::version(&p).map_err(|e| e.to_string());
            (p, v)
        });

        let mut inner = state.lock();
        let stored = store(&mut inner, (generation, agy_generation), claude, agy);
        #[cfg(feature = "antigravity-accounts")]
        if stored {
            crate::agy_switch::observe(&inner);
        }
        #[cfg(not(feature = "antigravity-accounts"))]
        let _ = stored;
        if let Some(v) = version {
            inner.version = Some(v);
        }
        store_own(&mut inner, own);
        store_codex(&mut inner, codex);
        inner.reading = false;
        inner.reads += 1;
        inner.schedule.mark_read(Instant::now());
        // The Antigravity account in use is read with every quota-axi read,
        // once this one has said which account that is.
        #[cfg(feature = "antigravity-accounts")]
        inner.agy_pace.cycle();
        drop(inner);
        state.wake_agy();
        crate::alerting::after_read(state);
        crate::backup::daily(app, state);
        crate::skills::daily(app, state);
        crate::env::every(app, state);
        publish(app, state);

        if let Ok(check) = switcher.join() {
            check.store(&mut state.lock());
            publish(app, state);
        }
    }
}

/// One look at the switcher. Without `full`, only the pause flag.
struct SwitcherCheck {
    pause: Option<pause::Flag>,
    /// The flag's generation when the check started.
    pause_generation: u64,
    full: Option<FullCheck>,
}

struct FullCheck {
    /// The rotation's decision now; `noAccounts` before the app keeps any.
    dry_run: Result<DryRunRecord, &'static str>,
    last_log: Option<switcher::LogLine>,
    service: Option<bool>,
    legacy: Option<String>,
}

fn check_switcher(home: &Path, full: bool, pause_generation: u64) -> SwitcherCheck {
    let flag_file = pause::path(home);
    let pause = pause::read(&flag_file).ok().flatten();
    let full = full.then(|| {
        let at = OffsetDateTime::now_utc();
        let flag_held = pause.as_ref().is_some_and(|f| f.holds(&local_time::stamp(at)));
        let launchctl = Path::new(switcher::LAUNCHCTL);
        let paths = Paths::for_home(home);
        let dry_run = if paths.in_use() {
            let store = Store::new(paths, Security::system());
            let decided = rotate::dry_run(
                &store,
                &rotate::Tools::system(home),
                rotate::rules(&store).as_ref(),
                at,
            );
            Ok(DryRunRecord { run: switcher::decided(&decided.line), at, flag_held })
        } else {
            Err("noAccounts")
        };
        FullCheck {
            dry_run,
            last_log: switcher::last_log_line(&switcher::log_file(home)),
            service: agent::loaded(launchctl),
            legacy: agent::legacy_loaded(launchctl).map(String::from),
        }
    });
    SwitcherCheck { pause, pause_generation, full }
}

impl SwitcherCheck {
    fn store(self, inner: &mut Inner) {
        let sw = &mut inner.switcher;
        // A command changed the flag after this check read it: keep the newer.
        if sw.pause_generation == self.pause_generation {
            sw.pause = self.pause;
        }
        if let Some(full) = self.full {
            match full.dry_run {
                Ok(record) => {
                    sw.dry_run = Some(record);
                    sw.dry_run_error = None;
                }
                Err(kind) => {
                    sw.dry_run = None;
                    sw.dry_run_error = Some(kind);
                }
            }
            sw.last_log = full.last_log;
            sw.service = full.service;
            sw.legacy = full.legacy;
            sw.checked_at = Some(Instant::now());
        }
    }
}

/// The Claude accounts as the app read them: the tab's rows and the
/// windows' details. `None` before the app keeps any.
type Own = Option<(Vec<AccountOut>, Vec<ClaudeDetail>)>;

/// Claude and Antigravity read in parallel: the first `agy` read of a cold
/// cache takes seconds and should not hold the Claude numbers back. The
/// app's own Claude accounts are read first, so quota-axi finds their
/// numbers and the account in use as they are now.
fn read_all(
    path: Option<PathBuf>,
    store: &Store,
    names: bool,
    env: &Env,
) -> (Result<Response, RunError>, Result<Response, RunError>, Own) {
    let agy = path.clone().map(|p| thread::spawn(move || runner::read(&p, ProviderId::Agy)));
    let own = read_own(store, names);
    let claude = match &path {
        Some(p) => runner::read_env(p, ProviderId::Claude, env),
        None => Err(RunError::NotConfigured),
    };
    let agy = match agy {
        Some(t) => t.join().unwrap_or_else(|_| Err(RunError::Spawn("reader panicked".into()))),
        None => Err(RunError::NotConfigured),
    };
    (claude, agy, own)
}

/// Reads each account whose reading is due and writes the files quota-axi
/// and Firstmate read. No token is refreshed while an older switcher runs.
fn read_own(store: &Store, names: bool) -> Own {
    if !store.paths.in_use() {
        return None;
    }
    let legacy = agent::legacy_loaded(Path::new(switcher::LAUNCHCTL));
    usage::read_due(store, &Endpoints::anthropic(), false, legacy.is_none());
    let _ = state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
    Some(crate::claude::load(store, names))
}

/// A failed read keeps the last good report, so the popover can show the
/// last numbers with their age next to the error. A Claude result from a
/// read that started before a Keychain grant, and an Antigravity one from a
/// read that started before or during a switch (`generations` behind), are
/// dropped. Returns whether the Antigravity report was kept.
pub fn store(
    inner: &mut Inner,
    generations: (u64, u64),
    claude: Result<Response, RunError>,
    agy: Result<Response, RunError>,
) -> bool {
    let (generation, agy_generation) = generations;
    if inner.claude_generation == generation {
        match claude {
            Ok(r) => {
                inner.claude = Some(r);
                inner.claude_error = None;
            }
            Err(e) => inner.claude_error = Some(e),
        }
    }
    if inner.agy_generation != agy_generation {
        return false;
    }
    match agy {
        Ok(r) => {
            inner.agy = Some(r);
            inner.agy_error = None;
            true
        }
        Err(e) => {
            inner.agy_error = Some(e);
            false
        }
    }
}

/// The app's own Claude accounts as just read; none before it keeps any.
fn store_own(inner: &mut Inner, own: Own) {
    match own {
        Some((rows, details)) => {
            inner.claude_accounts = rows;
            inner.claude_list = Some(details);
        }
        None => {
            inner.claude_accounts = Vec::new();
            inner.claude_list = None;
        }
    }
}

/// quota-axi's Codex read: a failed read keeps the last report beside the
/// error, as for Claude; no Codex folder, nothing.
fn store_codex(inner: &mut Inner, codex: Option<Result<Response, RunError>>) {
    match codex {
        Some(Ok(r)) => {
            inner.codex_quota = Some(r);
            inner.codex_quota_error = None;
        }
        Some(Err(e)) => inner.codex_quota_error = Some(e),
        None => {
            inner.codex_quota = None;
            inner.codex_quota_error = None;
        }
    }
}

/// Pushes the current state to the menu bar icon and to every window.
pub fn publish(app: &AppHandle, state: &AppState) {
    let (model, icon, follow) = {
        let inner = state.lock();
        let snapshot = inner.detailed_snapshot();
        let icon = inner.icon(&snapshot, OffsetDateTime::now_utc());
        let codex = inner.codex_use();
        let has = |m: Menubar| icon::followed(&snapshot, m, &codex) == m;
        let follow = tray::Follow {
            picked: inner.settings.menubar,
            gemini: has(Menubar::Gemini),
            codex: has(Menubar::Codex),
            window: inner.settings.menubar_window,
            fable: icon::followed(&snapshot, inner.settings.menubar, &codex) == Menubar::Claude,
        };
        (inner.model_with(snapshot), icon, follow)
    };
    tray::apply(app, &icon, follow);
    let _ = app.emit("model", &model);
}

#[cfg(test)]
mod tests {
    use quota_station_core::axi;
    use quota_station_core::snapshot::Keychain;

    use super::*;

    fn report(text: &str) -> Response {
        axi::parse(text).expect("fixture parses")
    }

    fn inner() -> Inner {
        Inner::new(PathBuf::new(), PathBuf::from("/nonexistent/settings.json"), "Mac".into())
    }

    const REQUIRED: &str = include_str!(
        "../../crates/quota-station-core/tests/fixtures/claude-keychain-required.json"
    );
    const GRANTED: &str =
        include_str!("../../crates/quota-station-core/tests/fixtures/claude-pool-schema5.json");
    const AGY: &str =
        include_str!("../../crates/quota-station-core/tests/fixtures/agy-cockpit.json");

    #[test]
    fn a_read_started_before_a_keychain_grant_keeps_the_granted_report() {
        let mut inner = inner();
        let started = inner.claude_generation;
        inner.store_claude(report(GRANTED));
        store(&mut inner, (started, 0), Ok(report(REQUIRED)), Ok(report(AGY)));
        assert_eq!(inner.snapshot().keychain, Keychain::Granted);
        assert!(inner.agy.is_some());

        let next = inner.claude_generation;
        store(&mut inner, (next, 0), Ok(report(REQUIRED)), Ok(report(AGY)));
        assert_eq!(inner.snapshot().keychain, Keychain::Required);
    }

    #[test]
    fn an_antigravity_read_started_before_a_switch_ends_is_dropped() {
        let mut inner = inner();
        let started = inner.agy_generation;
        // A switch starts and stores its own reading meanwhile.
        inner.agy_generation += 2;
        let claude = inner.claude_generation;
        assert!(!store(&mut inner, (claude, started), Ok(report(GRANTED)), Ok(report(AGY))));
        assert!(inner.agy.is_none());
        assert!(inner.claude.is_some(), "the Claude half is kept");
        assert!(!store(
            &mut inner,
            (claude, started),
            Ok(report(GRANTED)),
            Err(RunError::NotConfigured)
        ));
        assert!(inner.agy_error.is_none());

        let now = inner.agy_generation;
        assert!(store(&mut inner, (claude, now), Ok(report(GRANTED)), Ok(report(AGY))));
        assert!(inner.agy.is_some());
    }

    #[test]
    fn a_mac_where_the_app_keeps_no_claude_account_shows_none() {
        let mut inner = inner();
        let account = ClaudeDetail { number: 1, ..Default::default() };
        store_own(&mut inner, Some((Vec::new(), vec![account])));
        assert_eq!(inner.claude_list.as_ref().map(Vec::len), Some(1));
        store_own(&mut inner, None);
        assert!(inner.claude_list.is_none());
    }

    const CODEX: &str =
        include_str!("../../crates/quota-station-core/tests/fixtures/codex-chatgpt.json");

    #[test]
    fn a_failed_codex_read_keeps_the_last_report_and_no_codex_folder_clears_it() {
        let mut inner = inner();
        store_codex(&mut inner, Some(Ok(report(CODEX))));
        assert!(inner.codex_quota.is_some());
        store_codex(&mut inner, Some(Err(RunError::Timeout(Duration::from_secs(45)))));
        assert!(inner.codex_quota.is_some(), "the last numbers stay, with the error");
        assert!(inner.codex_quota_error.is_some());
        store_codex(&mut inner, None);
        assert!(inner.codex_quota.is_none() && inner.codex_quota_error.is_none());
    }

    #[test]
    fn a_switcher_check_that_read_the_flag_before_a_pause_keeps_the_pause() {
        let mut inner = inner();
        let started = inner.switcher.pause_generation;
        // The owner pauses while a check that saw no flag is running.
        let flag = pause::Flag { until: None, reason: Some("tạm dừng".into()) };
        inner.switcher.pause = Some(flag.clone());
        inner.switcher.pause_generation += 1;
        SwitcherCheck { pause: None, pause_generation: started, full: None }.store(&mut inner);
        assert_eq!(inner.switcher.pause, Some(flag));

        // A check that started after the pause is taken as it is.
        let now = inner.switcher.pause_generation;
        SwitcherCheck { pause: None, pause_generation: now, full: None }.store(&mut inner);
        assert_eq!(inner.switcher.pause, None);
    }
}
