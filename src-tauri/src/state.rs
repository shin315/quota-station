//! What the app holds between reads, and the model every window draws.

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

use quota_station_core::axi::Response;
use quota_station_core::runner::{self, RunError};
use quota_station_core::schedule::Schedule;
use quota_station_core::settings::Settings;
use quota_station_core::switcher::{Decision, DryRun, LogLine};
use quota_station_core::switching::Tools;
use quota_station_core::{cswap, local_time, pause, switcher, Snapshot};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub type AppState = Arc<Shared>;

pub struct Shared {
    inner: Mutex<Inner>,
    /// Wakes the reader thread early: a read was asked for, or settings changed.
    pub wake: Condvar,
}

impl Shared {
    pub fn new(inner: Inner) -> AppState {
        Arc::new(Shared { inner: Mutex::new(inner), wake: Condvar::new() })
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Asks the reader thread for a read now.
    pub fn request_read(&self) {
        self.lock().read_now = true;
        self.wake.notify_all();
    }

    /// Asks the reader thread to look at the switcher again: its dry run,
    /// its log and launchd.
    pub fn request_switcher(&self) {
        self.lock().switcher_now = true;
        self.wake.notify_all();
    }
}

pub struct Inner {
    pub home: PathBuf,
    pub settings_file: PathBuf,
    pub settings: Settings,
    /// The name macOS gives this Mac, used when the user has set none.
    pub system_machine: String,
    pub claude: Option<Response>,
    pub agy: Option<Response>,
    pub claude_error: Option<RunError>,
    pub agy_error: Option<RunError>,
    /// Bumped when a Keychain grant or a switch stores a Claude report, so a
    /// read that started before it cannot put the older report back.
    pub claude_generation: u64,
    pub reading: bool,
    /// Completed reads, so the UI can tell when the read it asked for is done.
    pub reads: u64,
    pub read_now: bool,
    pub schedule: Schedule,
    /// `--version` of the quota-axi in use, checked once per path.
    pub version: Option<(PathBuf, Result<String, String>)>,
    pub granting: bool,
    /// When the popover last hid itself on losing focus.
    pub popover_hidden_at: Option<Instant>,
    /// The tab the main window shows next: set when something opens it.
    pub main_tab: Tab,
    pub switcher: SwitcherState,
    pub switcher_now: bool,
    /// The slot a manual switch is running for.
    pub switching: Option<u32>,
}

/// What the app last saw of the `cswap-prefer-team` switcher.
#[derive(Debug, Default)]
pub struct SwitcherState {
    /// The pause flag; `None` when there is none.
    pub pause: Option<pause::Flag>,
    /// Bumped when a command writes or removes the flag, so a reader check
    /// that read the flag before cannot put the older one back.
    pub pause_generation: u64,
    pub dry_run: Option<DryRunRecord>,
    pub dry_run_error: Option<&'static str>,
    pub last_log: Option<LogLine>,
    /// `com.cswap.prefer-team` is loaded.
    pub service: Option<bool>,
    /// `com.cswap.auto`, claude-swap's own switcher, is loaded.
    pub auto_service: Option<bool>,
    pub checked_at: Option<Instant>,
}

#[derive(Debug)]
pub struct DryRunRecord {
    pub run: DryRun,
    pub at: OffsetDateTime,
    /// The pause flag held when the dry run started.
    pub flag_held: bool,
}

impl SwitcherState {
    /// The pause is on now: the flag holds by the guard's rule, and the
    /// switcher has not shown that it ignores the flag.
    pub fn paused(&self, now: OffsetDateTime) -> bool {
        let now = local_time::stamp(now);
        self.pause.as_ref().is_some_and(|f| f.holds(&now)) && self.guard() != Some(false)
    }

    /// Whether the switcher's guard reads the pause flag: known only from a
    /// dry run made while the flag held.
    pub fn guard(&self) -> Option<bool> {
        let record = self.dry_run.as_ref().filter(|r| r.flag_held)?;
        Some(matches!(record.run.decision, Decision::Paused { .. }))
    }
}

/// The main window's tabs that this build has. Later milestones add more.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tab {
    #[default]
    Overview,
    Claude,
    Agy,
    Settings,
}

impl Inner {
    pub fn new(home: PathBuf, settings_file: PathBuf, system_machine: String) -> Self {
        let settings = Settings::load(&settings_file);
        Inner {
            home,
            settings_file,
            settings,
            system_machine,
            claude: None,
            agy: None,
            claude_error: None,
            agy_error: None,
            claude_generation: 0,
            reading: false,
            reads: 0,
            read_now: true,
            schedule: Schedule::new(),
            version: None,
            granting: false,
            popover_hidden_at: None,
            main_tab: Tab::Overview,
            switcher: SwitcherState::default(),
            switcher_now: true,
            switching: None,
        }
    }

    /// The name shown for this Mac: the user's, else the one macOS gives.
    pub fn machine(&self) -> String {
        self.settings.machine_name.clone().unwrap_or_else(|| self.system_machine.clone())
    }

    /// Records whether the popover or the main window is on screen. Returns
    /// true when one just opened and a read is due.
    pub fn set_visible(&mut self, popover: bool, main: bool) -> bool {
        self.schedule.set_visible(popover || main, Instant::now())
    }

    /// The quota-axi to run: the user's path, else the first usual place.
    pub fn quota_axi(&self) -> Option<PathBuf> {
        self.settings.quota_axi_path.clone().or_else(|| runner::discover(&self.home))
    }

    /// The programs and files a manual switch uses.
    pub fn switch_tools(&self) -> Tools {
        Tools {
            cswap: cswap::discover(&self.home),
            quota_axi: self.quota_axi(),
            pause_file: pause::path(&self.home),
            log_file: switcher::log_file(&self.home),
        }
    }

    /// Reads the pause flag again after a command changed it; an unreadable
    /// flag counts as none.
    pub fn reload_pause(&mut self) {
        self.switcher.pause = pause::read(&pause::path(&self.home)).ok().flatten();
        self.switcher.pause_generation += 1;
    }

    /// Stores a Claude report read outside the reader loop (the Keychain
    /// grant, the re-read after a switch), ahead of any read in flight.
    pub fn store_claude(&mut self, report: Response) {
        self.claude = Some(report);
        self.claude_error = None;
        self.claude_generation += 1;
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::build(self.claude.as_ref(), self.agy.as_ref())
    }

    pub fn model(&self) -> Model {
        let path = self.quota_axi();
        let version = self
            .version
            .as_ref()
            .filter(|(p, _)| Some(p) == path.as_ref())
            .and_then(|(_, v)| v.as_ref().ok().cloned());
        Model {
            snapshot: self.snapshot(),
            errors: Errors {
                claude: self.claude_error.as_ref().map(ErrorView::from),
                agy: self.agy_error.as_ref().map(ErrorView::from),
            },
            reading: self.reading,
            reads: self.reads,
            machine: self.machine(),
            machine_default: self.system_machine.clone(),
            quota_axi: QuotaAxi {
                exists: path.as_ref().is_some_and(|p| p.is_file()),
                path: path.as_ref().map(|p| runner::display_path(p, &self.home)),
                custom: self.settings.quota_axi_path.is_some(),
                fork: version.as_deref().map(runner::is_fork),
                version,
            },
            granting: self.granting,
            switcher: self.switcher_view(OffsetDateTime::now_utc()),
            claude_switch: ClaudeSwitchView {
                cswap: cswap::discover(&self.home).is_some(),
                busy: self.switching,
            },
        }
    }

    fn switcher_view(&self, now: OffsetDateTime) -> SwitcherView {
        let sw = &self.switcher;
        let stamp = local_time::stamp(now);
        let rfc = |t: OffsetDateTime| t.format(&Rfc3339).ok();
        SwitcherView {
            installed: switcher::script(&self.home).is_file(),
            pause: sw.pause.as_ref().map(|f| PauseView {
                until: f.until.as_deref().and_then(local_time::instant).and_then(rfc),
                reason: f.reason.clone(),
                holds: f.holds(&stamp),
            }),
            dry_run: sw.dry_run.as_ref().map(|r| DryRunView {
                decision: r.run.decision.clone(),
                raw: r.run.raw.clone(),
                at: rfc(r.at).unwrap_or_default(),
            }),
            dry_run_error: sw.dry_run_error,
            guard: sw.guard(),
            last_log: sw.last_log.clone(),
            service: sw.service,
            auto_service: sw.auto_service,
        }
    }
}

/// Everything the popover and the main window draw.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub snapshot: Snapshot,
    pub errors: Errors,
    pub reading: bool,
    pub reads: u64,
    pub machine: String,
    /// The name macOS gives this Mac, for the setting's placeholder.
    pub machine_default: String,
    pub quota_axi: QuotaAxi,
    pub granting: bool,
    pub switcher: SwitcherView,
    pub claude_switch: ClaudeSwitchView,
}

/// The switcher as the windows show it (F07).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitcherView {
    /// `cswap-prefer-team` is on this Mac.
    pub installed: bool,
    pub pause: Option<PauseView>,
    pub dry_run: Option<DryRunView>,
    pub dry_run_error: Option<&'static str>,
    /// `false`: a dry run while the flag held did not say paused, so the
    /// switcher on this Mac does not read the pause flag yet.
    pub guard: Option<bool>,
    pub last_log: Option<LogLine>,
    pub service: Option<bool>,
    pub auto_service: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseView {
    /// RFC 3339; `None` pauses until the owner resumes.
    pub until: Option<String>,
    pub reason: Option<String>,
    /// The flag held when this model was built.
    pub holds: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunView {
    pub decision: Decision,
    pub raw: String,
    /// When the dry run ran, RFC 3339.
    pub at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSwitchView {
    /// claude-swap is on this Mac; without it nothing can switch.
    pub cswap: bool,
    /// The slot a switch is running for.
    pub busy: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Errors {
    pub claude: Option<ErrorView>,
    pub agy: Option<ErrorView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaAxi {
    /// `~/…` for paths under home.
    pub path: Option<String>,
    /// Set by the user rather than found.
    pub custom: bool,
    pub exists: bool,
    pub version: Option<String>,
    pub fork: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorView {
    pub kind: &'static str,
    pub detail: String,
}

impl From<&RunError> for ErrorView {
    fn from(e: &RunError) -> Self {
        let kind = match e {
            RunError::NotConfigured => "notConfigured",
            RunError::NotFound(_) => "notFound",
            RunError::Spawn(_) => "spawn",
            RunError::Timeout(_) => "timeout",
            RunError::Exit { .. } => "exit",
            RunError::Parse(_) => "parse",
        };
        let detail = match e {
            // Never echo an absolute path from this Mac into the UI.
            RunError::NotFound(_) | RunError::NotConfigured => String::new(),
            other => other.to_string(),
        };
        ErrorView { kind, detail }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inner() -> Inner {
        Inner::new(PathBuf::new(), PathBuf::from("/nonexistent/settings.json"), "Mac mini".into())
    }

    #[test]
    fn the_machine_name_set_by_the_user_wins_over_the_one_macos_gives() {
        let mut inner = inner();
        assert_eq!(inner.model().machine, "Mac mini");
        inner.settings.machine_name = Some("Mac phòng khách".into());
        let model = inner.model();
        assert_eq!(model.machine, "Mac phòng khách");
        assert_eq!(model.machine_default, "Mac mini");
    }

    #[test]
    fn a_pause_counts_until_a_dry_run_shows_the_switcher_ignores_it() {
        use quota_station_core::switcher::parse_decision;
        let now = OffsetDateTime::now_utc();
        let later = local_time::minute(now + time::Duration::hours(1));
        let mut inner = inner();
        assert!(!inner.switcher.paused(now));

        inner.switcher.pause =
            Some(pause::Flag { until: Some(later.clone()), reason: Some("lý do".into()) });
        assert!(inner.switcher.paused(now));
        assert_eq!(inner.switcher.guard(), None, "no dry run yet");
        let view = inner.model().switcher;
        let pause = view.pause.unwrap();
        assert!(pause.holds);
        let until = OffsetDateTime::parse(&pause.until.unwrap(), &Rfc3339).unwrap();
        assert_eq!(local_time::minute(until), later);

        let record = |line: &str, flag_held| DryRunRecord {
            run: DryRun { decision: parse_decision(line), raw: line.into() },
            at: now,
            flag_held,
        };
        inner.switcher.dry_run = Some(record("stay", true));
        assert_eq!(inner.switcher.guard(), Some(false), "the switcher decided as if not paused");
        assert!(!inner.switcher.paused(now));
        inner.switcher.dry_run = Some(record(&format!("paused until {later}: lý do"), true));
        assert_eq!(inner.switcher.guard(), Some(true));
        assert!(inner.switcher.paused(now));
        // A dry run from before the pause says nothing about the guard.
        inner.switcher.dry_run = Some(record("stay", false));
        assert_eq!(inner.switcher.guard(), None);

        let expired = local_time::minute(now - time::Duration::minutes(5));
        inner.switcher.pause = Some(pause::Flag { until: Some(expired), reason: None });
        assert!(!inner.switcher.paused(now));
        assert!(!inner.model().switcher.pause.unwrap().holds);
    }

    #[test]
    fn the_main_window_counts_as_open_for_the_read_pace() {
        let mut inner = inner();
        inner.schedule.mark_read(Instant::now());
        assert!(!inner.set_visible(false, true), "a read just happened");
        assert!(inner.schedule.visible());
        inner.set_visible(true, true);
        inner.set_visible(false, true);
        assert!(inner.schedule.visible(), "closing the popover keeps the window's pace");
        inner.set_visible(false, false);
        assert!(!inner.schedule.visible());
    }
}
