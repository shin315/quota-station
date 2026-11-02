// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The reader thread: reads the quota on the schedule ([`quota`]) and pushes
//! each result to the menu bar and the windows. At each pass it also runs
//! the extension's work ([`Extension::reader_pass`]): beside the read, or
//! alone between reads when the extension asked for it.
//! Where the app keeps the Claude accounts it reads them (each account's
//! windows, and its name from the Keychain while names show), writes the
//! data file, and builds the Claude row from them; elsewhere it reads Claude Code's own
//! login. Antigravity is read beside it, and Codex where this Mac has a
//! Codex folder. A reading younger than two minutes is reused without
//! asking the vendor. Each read also wakes the Antigravity quota thread for
//! the account in use, and each Antigravity reading goes to the six-hour
//! watch after an Antigravity switch. After each read the history and the
//! alerts look at the new numbers (`alerting`).

use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::state::AccountOut;
use quota_station_core::claude::store::Store;
use quota_station_core::claude::usage::{self, Endpoints};
use quota_station_core::claude::{state, switch};
use quota_station_core::codex::platform::System;
use quota_station_core::detail::ClaudeDetail;
use quota_station_core::quota::cache::{Cache, MAX_AGE_SECONDS};
use quota_station_core::quota::{self, ProviderId, Response};
use quota_station_core::settings::{Language, Menubar};
use quota_station_core::{icon, schedule};
use tauri::{AppHandle, Emitter};
use time::OffsetDateTime;

use crate::ext::{Extension, Hooks, ReaderDone, ReaderPass, ReaderWork};
use crate::state::{AppState, Inner};
use crate::tray;

/// The longest single wait, so a sleep of the Mac is noticed soon after it wakes.
const WAIT_STEP: Duration = Duration::from_secs(30);

pub fn spawn(app: AppHandle, state: AppState, extension: &'static dyn Extension) {
    thread::Builder::new()
        .name("quota-reader".into())
        .spawn(move || run(&app, &state, extension))
        .expect("reader thread starts");
}

fn run(app: &AppHandle, state: &AppState, extension: &dyn Extension) {
    loop {
        let mut inner = state.lock();
        let wait = if asked(&inner, extension) {
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
        // The extension's work runs beside the read, and the numbers are
        // shown as soon as they are in: slow work never holds them back.
        let (read_due, work) = extension_pass(&inner, extension);
        if !read_due {
            // The extension's work on its own, between reads.
            drop(inner);
            if let Some(work) = work {
                finish(app, state, work());
            }
            continue;
        }
        let beside: Option<JoinHandle<ReaderDone>> = work.map(thread::spawn);
        inner.read_now = false;
        inner.reading = true;
        let accounts = inner.claude_store();
        let app_dir = inner.app_dir();
        let names = !inner.settings.hide_names;
        let generation = inner.claude_generation;
        let agy_generation = inner.agy_generation;
        let codex_home = Some(inner.codex_home()).filter(|h| h.is_dir());
        let hooks = inner.hooks;
        drop(inner);
        publish(app, state);

        let codex_app = app_dir.clone();
        let codex = thread::spawn(move || codex_home.map(|h| read_codex(&h, &codex_app)));
        let (claude, agy, own) = read_all(&accounts, &app_dir, names, hooks);
        let codex = codex.join().ok().flatten();

        let mut inner = state.lock();
        let stored = store(&mut inner, (generation, agy_generation), claude, agy);
        #[cfg(feature = "antigravity-accounts")]
        if stored {
            crate::agy_switch::observe(&inner);
        }
        #[cfg(not(feature = "antigravity-accounts"))]
        let _ = stored;
        store_own(&mut inner, own);
        store_codex(&mut inner, codex);
        inner.reading = false;
        inner.reads += 1;
        inner.schedule.mark_read(Instant::now());
        // The Antigravity account in use is read with every read, once this
        // one has said which account that is.
        #[cfg(feature = "antigravity-accounts")]
        inner.agy_pace.cycle();
        drop(inner);
        state.wake_agy();
        crate::alerting::after_read(state);
        crate::backup::daily(app, state);
        crate::skills::daily(app, state);
        crate::release::daily(app, state);
        crate::cli_tools::daily(app, state);
        crate::env::every(app, state);
        publish(app, state);

        if let Some(Ok(done)) = beside.map(JoinHandle::join) {
            finish(app, state, done);
        }
    }
}

/// A read was asked for, or the extension asked: the reader goes now,
/// without waiting for the schedule.
fn asked<E: Extension + ?Sized>(inner: &Inner, extension: &E) -> bool {
    inner.read_now || extension.reader_asked(&inner.kept)
}

/// The extension's work at this pass, asked under the app's lock, and
/// whether the quota is read on it.
fn extension_pass<E: Extension + ?Sized>(
    inner: &Inner,
    extension: &E,
) -> (bool, Option<ReaderWork>) {
    let read_due = inner.read_now || inner.schedule.due_in(Instant::now()).is_zero();
    let app_dir = inner.app_dir();
    let pass = ReaderPass {
        kept: &inner.kept,
        home: &inner.home,
        app_dir: &app_dir,
        read_due,
        window_open: inner.schedule.visible(),
    };
    (read_due, extension.reader_pass(&pass))
}

/// Finishes the extension's work under the app's lock, and shows it.
fn finish(app: &AppHandle, state: &AppState, done: ReaderDone) {
    done(&state.lock().kept);
    publish(app, state);
}

/// The Claude accounts as the app read them: the tab's rows and the
/// windows' details. `None` before the app keeps any.
pub type Own = Option<(Vec<AccountOut>, Vec<ClaudeDetail>)>;

/// Claude and Antigravity read in parallel: an `agy -p /quota` read takes
/// seconds and should not hold the Claude numbers back.
fn read_all(
    store: &Store,
    app: &AppDir,
    names: bool,
    hooks: &dyn Hooks,
) -> (Response, Response, Own) {
    let agy_app = app.clone();
    let agy = thread::spawn(move || read_agy(&agy_app, MAX_AGE_SECONDS));
    let (claude, own) = read_claude(store, app, names, quota::claude::Keychain::Quiet, hooks);
    let agy = agy.join().unwrap_or_else(|_| {
        Response::new(
            OffsetDateTime::now_utc(),
            vec![quota::Provider::failed(ProviderId::Agy, "error", "Antigravity read failed")],
        )
    });
    (claude, agy, own)
}

/// The Claude row: from the accounts the app keeps, read first, where it
/// keeps them, with the pool's policy `hooks` give; else Claude Code's own
/// login, read-only, the Keychain item read as `mode` allows.
pub fn read_claude(
    store: &Store,
    app: &AppDir,
    names: bool,
    mode: quota::claude::Keychain,
    hooks: &dyn Hooks,
) -> (Response, Own) {
    let now = OffsetDateTime::now_utc();
    if let Some(own) = read_own(store, names, hooks) {
        if let Some(report) = quota::claude::own_now(store, hooks, now) {
            return (report, Some(own));
        }
    }
    let live = quota::claude::Live {
        paths: &store.paths,
        security: &*store.security,
        endpoints: &Endpoints::anthropic(),
        app,
    };
    let max_age = if mode == quota::claude::Keychain::Grant { 0 } else { MAX_AGE_SECONDS };
    let cache = Cache::new(app, ProviderId::Claude);
    (quota::claude::read(&live, mode, &cache, max_age, now), None)
}

/// The Antigravity row: the account agy is on, with the app's own list of
/// the accounts it logs in, in a build with them.
pub fn read_agy(app: &AppDir, max_age: i64) -> Response {
    let tools = quota::agy::Tools::system();
    let now = OffsetDateTime::now_utc();
    #[cfg(feature = "antigravity-accounts")]
    return quota_station_core::antigravity::listing::read(&tools, app, max_age, now);
    #[cfg(not(feature = "antigravity-accounts"))]
    quota::agy::read(&tools, &Cache::new(app, ProviderId::Agy), max_age, now)
}

/// Every ChatGPT account of `codex_home`: the one in it and those the app
/// keeps in their own folders.
fn read_codex(codex_home: &Path, app: &AppDir) -> Response {
    quota::codex::read_accounts(
        app,
        codex_home,
        &System::system(),
        &quota::codex::Endpoints::chatgpt(),
        MAX_AGE_SECONDS,
        OffsetDateTime::now_utc(),
    )
}

/// Reads each account whose reading is due and writes the data file. A
/// login's token is refreshed only while `hooks` let it
/// ([`Hooks::may_refresh_logins`]).
fn read_own(store: &Store, names: bool, hooks: &dyn Hooks) -> Own {
    if !store.paths.in_use() {
        return None;
    }
    usage::read_due(store, &Endpoints::anthropic(), false, hooks.may_refresh_logins());
    let _ = state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
    Some(crate::claude::load(store, names))
}

/// Keeps the new reports. A failed read is a report too: its row says
/// why, and shows the last numbers as stale while they stand. A Claude
/// result from a read that started before a Keychain grant, and an
/// Antigravity one from a read that started before or during a switch
/// (`generations` behind), are dropped. Returns whether the Antigravity
/// report was kept.
pub fn store(inner: &mut Inner, generations: (u64, u64), claude: Response, agy: Response) -> bool {
    let (generation, agy_generation) = generations;
    if inner.claude_generation == generation {
        inner.claude = Some(claude);
    }
    if inner.agy_generation != agy_generation {
        return false;
    }
    inner.agy = Some(agy);
    true
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

/// The Codex read; no Codex folder, nothing.
fn store_codex(inner: &mut Inner, codex: Option<Response>) {
    inner.codex_quota = codex;
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
            en: inner.settings.language == Language::En,
        };
        (inner.model_with(snapshot), icon, follow)
    };
    tray::apply(app, &icon, follow);
    let _ = app.emit("model", &model);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    use quota_station_core::quota::{Provider, State, Window};
    use quota_station_core::snapshot::Keychain;

    use super::*;
    use crate::ext::{Kept, NoExtension};

    fn inner() -> Inner {
        Inner::new(
            PathBuf::new(),
            PathBuf::from("/nonexistent/settings.json"),
            "Mac".into(),
            quota_station_core::ext::none(),
        )
    }

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap()
    }

    fn claude(reason: Option<&str>) -> Response {
        let row = match reason {
            Some(r) => Provider {
                provider: "claude".into(),
                state: State {
                    status: "auth_required".into(),
                    error: Some("keychain_prompt_required".into()),
                    reason: Some(r.into()),
                    ..State::default()
                },
                ..Provider::default()
            },
            None => Provider::fresh(
                ProviderId::Claude,
                vec![Window::used("five_hour", "session", 20.0, None)],
                at(),
            ),
        };
        Response::new(at(), vec![row])
    }

    fn agy() -> Response {
        Response::new(
            at(),
            vec![Provider::fresh(
                ProviderId::Agy,
                vec![Window::used("gemini_5h", "session", 10.0, None)],
                at(),
            )],
        )
    }

    #[test]
    fn a_read_started_before_a_keychain_grant_keeps_the_granted_report() {
        let required = || claude(Some(quota::claude::ACCESS_REQUIRED));
        let mut inner = inner();
        let started = inner.claude_generation;
        inner.store_claude(claude(None));
        store(&mut inner, (started, 0), required(), agy());
        assert_eq!(inner.snapshot().keychain, Keychain::Granted);
        assert!(inner.agy.is_some());

        let next = inner.claude_generation;
        store(&mut inner, (next, 0), required(), agy());
        assert_eq!(inner.snapshot().keychain, Keychain::Required);
    }

    #[test]
    fn an_antigravity_read_started_before_a_switch_ends_is_dropped() {
        let mut inner = inner();
        let started = inner.agy_generation;
        // A switch starts and stores its own reading meanwhile.
        inner.agy_generation += 2;
        let generation = inner.claude_generation;
        assert!(!store(&mut inner, (generation, started), claude(None), agy()));
        assert!(inner.agy.is_none());
        assert!(inner.claude.is_some(), "the Claude half is kept");

        let now = inner.agy_generation;
        assert!(store(&mut inner, (generation, now), claude(None), agy()));
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

    #[test]
    fn no_codex_folder_clears_the_codex_report() {
        let mut inner = inner();
        let codex = Response::new(at(), vec![Provider::failed(ProviderId::Codex, "error", "x")]);
        store_codex(&mut inner, Some(codex));
        assert!(inner.codex_quota.is_some());
        store_codex(&mut inner, None);
        assert!(inner.codex_quota.is_none());
    }

    #[test]
    fn a_read_asked_for_goes_without_waiting() {
        let mut inner = inner();
        assert!(asked(&inner, &NoExtension), "the first read, at start");
        inner.read_now = false;
        assert!(!asked(&inner, &NoExtension));
    }

    /// An extension that asks, as the reader sees it: plain data only.
    struct Asking {
        asks: AtomicBool,
        seen: Mutex<Vec<(bool, bool)>>,
    }

    impl Extension for Asking {
        fn reader_asked(&self, _kept: &Kept) -> bool {
            self.asks.load(Ordering::SeqCst)
        }

        fn reader_pass(&self, pass: &ReaderPass<'_>) -> Option<ReaderWork> {
            self.asks.store(false, Ordering::SeqCst);
            self.seen.lock().unwrap().push((pass.read_due, pass.window_open));
            Some(Box::new(|| Box::new(|_: &Kept| ())))
        }
    }

    #[test]
    fn an_extension_that_asks_sends_the_reader_without_a_read() {
        let asking = Asking { asks: AtomicBool::new(true), seen: Mutex::default() };
        let mut inner = inner();
        inner.read_now = false;
        inner.schedule.mark_read(Instant::now());
        assert!(asked(&inner, &asking));

        let (read_due, work) = extension_pass(&inner, &asking);
        assert!(!read_due, "only the extension asked");
        assert!(work.is_some());
        assert!(!asked(&inner, &asking), "the pass took the ask");

        // A read asked for: the extension's work goes beside it.
        inner.read_now = true;
        let (read_due, work) = extension_pass(&inner, &asking);
        assert!(read_due);
        assert!(work.is_some());
        assert_eq!(*asking.seen.lock().unwrap(), vec![(false, false), (true, false)]);
    }

    #[test]
    fn without_an_extension_the_reader_reads_alone() {
        let inner = inner();
        assert!(!NoExtension.reader_asked(&inner.kept));
        let (read_due, work) = extension_pass(&inner, &NoExtension);
        assert!(read_due, "the first read, at start");
        assert!(work.is_none());
    }
}
