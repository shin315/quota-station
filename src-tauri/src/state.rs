//! What the app holds between reads, and the model every window draws.

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

use quota_station_core::axi::Response;
use quota_station_core::runner::{self, RunError};
use quota_station_core::schedule::Schedule;
use quota_station_core::settings::Settings;
use quota_station_core::Snapshot;
use serde::{Deserialize, Serialize};

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
    /// Bumped when a Keychain grant stores a Claude report, so a read that
    /// started before it cannot put the older report back.
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

    /// Stores the report a Keychain grant read, ahead of any read in flight.
    pub fn store_grant(&mut self, report: Response) {
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
