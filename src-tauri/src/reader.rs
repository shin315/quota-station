//! The reader thread: runs quota-axi on the schedule and pushes each result
//! to the menu bar and the windows.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use quota_station_core::axi::Response;
use quota_station_core::runner::{self, ProviderId, RunError};
use quota_station_core::schedule;
use tauri::{AppHandle, Emitter};

use crate::state::{AppState, Inner};
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
        let wait =
            if inner.read_now { Duration::ZERO } else { inner.schedule.due_in(Instant::now()) };
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
        inner.read_now = false;
        inner.reading = true;
        let path = inner.quota_axi();
        let check_version =
            path.is_some() && inner.version.as_ref().map(|(p, _)| p) != path.as_ref();
        drop(inner);
        publish(app, state);

        let (claude, agy) = read_both(path.clone());
        let version = path.filter(|_| check_version).map(|p| {
            let v = runner::version(&p).map_err(|e| e.to_string());
            (p, v)
        });

        let mut inner = state.lock();
        store(&mut inner, claude, agy);
        if let Some(v) = version {
            inner.version = Some(v);
        }
        inner.reading = false;
        inner.reads += 1;
        inner.schedule.mark_read(Instant::now());
        drop(inner);
        publish(app, state);
    }
}

/// Claude and Antigravity read in parallel: the first `agy` read of a cold
/// cache takes seconds and should not hold the Claude numbers back.
fn read_both(path: Option<PathBuf>) -> (Result<Response, RunError>, Result<Response, RunError>) {
    let Some(path) = path else {
        return (Err(RunError::NotConfigured), Err(RunError::NotConfigured));
    };
    let agy_path = path.clone();
    let agy = thread::spawn(move || runner::read(&agy_path, ProviderId::Agy));
    let claude = runner::read(&path, ProviderId::Claude);
    let agy = agy.join().unwrap_or_else(|_| Err(RunError::Spawn("reader panicked".into())));
    (claude, agy)
}

/// A failed read keeps the last good report, so the popover can show the
/// last numbers with their age next to the error.
pub fn store(
    inner: &mut Inner,
    claude: Result<Response, RunError>,
    agy: Result<Response, RunError>,
) {
    match claude {
        Ok(r) => {
            inner.claude = Some(r);
            inner.claude_error = None;
        }
        Err(e) => inner.claude_error = Some(e),
    }
    match agy {
        Ok(r) => {
            inner.agy = Some(r);
            inner.agy_error = None;
        }
        Err(e) => inner.agy_error = Some(e),
    }
}

/// Pushes the current state to the menu bar icon and to every window.
pub fn publish(app: &AppHandle, state: &AppState) {
    let (model, icon) = {
        let inner = state.lock();
        let snapshot = inner.snapshot();
        let failing = inner.claude_error.is_some();
        let has_data = inner.claude.is_some();
        let icon = quota_station_core::icon(has_data.then_some(&snapshot), failing);
        (inner.model(), icon)
    };
    if std::env::var_os("QUOTA_STATION_LOG").is_some() {
        log_read(&model, &icon);
    }
    tray::apply(app, &icon);
    let _ = app.emit("model", &model);
}

/// One line per update on stderr when `QUOTA_STATION_LOG` is set, to check a
/// build against `quota-axi --tui` without opening the popover. Slot numbers
/// and percentages only.
fn log_read(model: &crate::state::Model, icon: &quota_station_core::Icon) {
    let s = &model.snapshot;
    let lines = |a: &quota_station_core::snapshot::Account| {
        a.lines
            .iter()
            .map(|l| l.percent.map_or("-".to_string(), |p| format!("{p}")))
            .collect::<Vec<_>>()
            .join("/")
    };
    let accounts = |sec: &quota_station_core::snapshot::Section| {
        sec.accounts
            .iter()
            .map(|a| format!("{}{}={}", a.number, if a.active { "*" } else { "" }, lines(a)))
            .collect::<Vec<_>>()
            .join(" ")
    };
    eprintln!(
        "quota-station: reads={} reading={} title={} tone={:?} keychain={:?} claude={:?}[{}] agy={:?}[{}] errors={:?}/{:?}",
        model.reads,
        model.reading,
        icon.title,
        icon.tone,
        s.keychain,
        s.claude.status,
        accounts(&s.claude),
        s.agy.status,
        accounts(&s.agy),
        model.errors.claude.as_ref().map(|e| e.kind),
        model.errors.agy.as_ref().map(|e| e.kind),
    );
}
