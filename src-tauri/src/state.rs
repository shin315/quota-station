//! What the app holds between reads, and the model every window draws.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

use quota_station_core::alerts::{self, Memory};
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::antigravity::oauth_server;
use quota_station_core::antigravity::pace::Pace;
use quota_station_core::axi::Response;
use quota_station_core::firstmate::{self, Reading};
use quota_station_core::history::Seen;
use quota_station_core::peers::{self, PeerState, SyncLog};
use quota_station_core::rotation::{self, FileState, Rules, Stamp};
use quota_station_core::runner::{self, RunError};
use quota_station_core::schedule::Schedule;
use quota_station_core::settings::Settings;
use quota_station_core::switcher::{Decision, DryRun, LogLine};
use quota_station_core::switching::Tools;
use quota_station_core::{cswap, local_time, pause, switcher, Snapshot};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::agy_switch::{self, SwitchView};
use crate::alerting::{self, AlertsView, FirstmateView};
use crate::antigravity::{self, AccountsView};
use crate::notify;

pub type AppState = Arc<Shared>;

pub struct Shared {
    inner: Mutex<Inner>,
    /// Wakes the reader thread early: a read was asked for, or settings changed.
    pub wake: Condvar,
    /// Wakes the rotation sync thread: a save, or the list of Macs changed.
    pub sync_wake: Condvar,
    /// Held while copying the rotation file to the other Macs, so two copies
    /// to the same Mac never run at once.
    pub push_lock: Mutex<()>,
    /// Wakes the Antigravity quota thread: a read, the tab, a login.
    pub agy_wake: Condvar,
    /// Held while Antigravity accounts are read and while one logs out, so
    /// a read never writes an account's token or reading back after its
    /// logout removed them.
    pub agy_accounts: Mutex<()>,
    /// Held while the history compares the newest readings, so a read and a
    /// switch never keep the same change twice.
    pub history: Mutex<()>,
}

impl Shared {
    pub fn new(inner: Inner) -> AppState {
        Arc::new(Shared {
            inner: Mutex::new(inner),
            wake: Condvar::new(),
            sync_wake: Condvar::new(),
            push_lock: Mutex::new(()),
            agy_wake: Condvar::new(),
            agy_accounts: Mutex::new(()),
            history: Mutex::new(()),
        })
    }

    /// Asks the sync thread to look at the other Macs now.
    pub fn request_sync(&self) {
        self.lock().sync_now = true;
        self.sync_wake.notify_all();
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

    /// Wakes the Antigravity quota thread after a change to its pace.
    pub fn wake_agy(&self) {
        self.agy_wake.notify_all();
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
    /// Bumped when an Antigravity switch or logout starts and ends, so a read
    /// from before cannot put the old account back, count as an overwrite,
    /// or show an account logged out.
    pub agy_generation: u64,
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
    /// Where each other Mac stands with this Mac's rotation file.
    pub sync: SyncLog,
    pub sync_file: PathBuf,
    pub sync_now: bool,
    /// Aliases of the Macs a copy is going to now.
    pub pushing: BTreeSet<String>,
    /// A rotation save is running.
    pub rotation_saving: bool,
    /// The rules editor holds changes not saved yet: closing the main window asks first.
    pub rules_dirty: bool,
    /// `Thoát Quota Station` waits for the main window to ask about unsaved
    /// rules. Kept here, not only sent as an event, because a window opened
    /// for it may not listen yet; the page takes it with `take_quit`.
    pub quit_pending: bool,
    /// An Antigravity login is running.
    pub agy_logging_in: bool,
    /// The running login while it waits for the browser: takes a pasted
    /// callback or a cancel.
    pub agy_login: Option<oauth_server::Handle>,
    /// When each Antigravity account is read next (M4b).
    pub agy_pace: Pace,
    /// The Antigravity accounts being read now, or whose last read failed,
    /// by id. An account not here was read, or has nothing to report.
    pub agy_reads: BTreeMap<String, ReadState>,
    /// The Antigravity account number a manual switch is running for.
    pub agy_switching: Option<usize>,
    /// A logout runs: a switch, or another logout, is refused meanwhile, as
    /// a logout is while a switch runs.
    pub agy_logging_out: bool,
    /// The alerts sent and answered, kept in `alerts/alerts.json` (M6).
    pub alerts: Memory,
    /// The accounts the last readings showed in use, for the history.
    pub seen: Seen,
    /// Firstmate's state file as last read (at start, then after each read);
    /// `None` with no path set.
    pub firstmate: Option<Result<Reading, firstmate::ReadError>>,
    /// Firstmate's dry rule on the newest quota-axi numbers, counted by the
    /// app with the same rotation the alerts use.
    pub own_dry: Option<bool>,
    /// macOS's notification center; `None` until the app has started it.
    pub center: Option<Arc<notify::Center>>,
}

/// How the app's own read of an Antigravity account stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadState {
    Reading,
    /// `quota::ReadError::kind` and its message.
    Failed {
        kind: &'static str,
        detail: String,
    },
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

/// The main window's tabs that this build has. Codex comes later.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tab {
    #[default]
    Overview,
    Claude,
    Agy,
    Rules,
    History,
    Alerts,
    Firstmate,
    Settings,
}

impl Inner {
    pub fn new(home: PathBuf, settings_file: PathBuf, system_machine: String) -> Self {
        let settings = Settings::load(&settings_file);
        let firstmate = settings.firstmate_file.as_deref().map(firstmate::read);
        let sync_file = settings_file.with_file_name("rotation-sync.json");
        let app_dir =
            AppDir::new(settings_file.parent().map(|p| p.to_path_buf()).unwrap_or_default());
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
            agy_generation: 0,
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
            sync: SyncLog::load(&sync_file),
            sync_file,
            sync_now: true,
            pushing: BTreeSet::new(),
            rotation_saving: false,
            rules_dirty: false,
            quit_pending: false,
            agy_logging_in: false,
            agy_login: None,
            agy_pace: Pace::new(),
            agy_reads: BTreeMap::new(),
            agy_switching: None,
            agy_logging_out: false,
            alerts: alerts::load(&app_dir),
            seen: Seen::default(),
            firstmate,
            own_dry: None,
            center: None,
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

    /// quota-axi's account in use, by its place in this app's Antigravity
    /// index, when quota-axi read this app's folder.
    pub fn agy_active_number(&self) -> Option<usize> {
        let cockpit = self.agy.as_ref()?.primary("agy")?.cockpit.as_ref()?;
        if cockpit.status != "ok" || cockpit.source.as_deref() != Some("quota-station") {
            return None;
        }
        cockpit.accounts.iter().find(|a| a.active).map(|a| a.account as usize)
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
            show_email: self.settings.show_email,
            antigravity: antigravity::view(self),
            agy_switch: agy_switch::view(self),
            rotation: self.rotation_view(),
            alerts: alerting::alerts_view(self),
            firstmate: alerting::firstmate_view(self),
        }
    }

    /// The rotation file as it is on disk now, and each other Mac against it.
    pub fn rotation_view(&self) -> RotationView {
        let file = rotation::path(&self.home);
        let state = rotation::load(&file);
        let stamp = state.stamp().cloned();
        let version = state.version();
        let (status, rules, reason, ignored_keys) = match &state {
            FileState::Missing => ("missing", None, None, Vec::new()),
            FileState::Unreadable(e) => ("unreadable", None, Some(e.clone()), Vec::new()),
            FileState::Invalid { reason, .. } => {
                ("invalid", None, Some(reason.clone()), Vec::new())
            }
            FileState::Ok { parsed, .. } => {
                ("ok", Some(parsed.rules.clone()), None, parsed.ignored_keys.clone())
            }
        };
        let peers = self
            .settings
            .peers
            .iter()
            .map(|p| {
                let record = self.sync.peers.get(&p.alias);
                PeerView {
                    name: p.name.clone(),
                    alias: p.alias.clone(),
                    current: record.zip(version.as_ref()).is_some_and(|(r, v)| r.against == *v),
                    state: record.map(|r| r.state.clone()),
                    their_rules: record
                        .and_then(|r| r.their_text.as_deref())
                        .and_then(|t| rotation::parse(t).ok())
                        .map(|p| p.rules),
                    pushing: self.pushing.contains(&p.alias),
                }
            })
            .collect();
        RotationView {
            path: runner::display_path(&file, &self.home),
            status,
            rules,
            reason,
            version,
            stamp,
            ignored_keys,
            defaults: Rules::defaults(),
            saving: self.rotation_saving,
            peers,
            ssh: peers::find_ssh(std::env::var_os("PATH").as_deref()).is_some(),
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
    /// Emails of the accounts logged in here may show.
    pub show_email: bool,
    /// The Antigravity accounts this app logs in itself.
    pub antigravity: AccountsView,
    /// Switching them (M4c).
    pub agy_switch: SwitchView,
    pub rotation: RotationView,
    /// The five alerts (M6).
    pub alerts: AlertsView,
    /// What Firstmate sees (M6).
    pub firstmate: FirstmateView,
}

/// The rotation file and the other Macs (F06).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationView {
    /// `~/.config/cswap-prefer-team/rotation`
    pub path: String,
    /// `ok`, `missing`, `invalid` (quota-axi would refuse it) or `unreadable`.
    pub status: &'static str,
    pub rules: Option<Rules>,
    /// quota-axi's reason for an invalid file, or the read error.
    pub reason: Option<String>,
    /// Sent back with a save, so a file changed meanwhile is not overwritten.
    pub version: Option<String>,
    /// The `# updated = … by …` line of a valid file; `None` for a file the app
    /// never saved, and for a missing or invalid one.
    pub stamp: Option<Stamp>,
    pub ignored_keys: Vec<String>,
    /// What "Tạo từ mặc định" writes: the switcher's built-in values.
    pub defaults: Rules,
    pub saving: bool,
    pub peers: Vec<PeerView>,
    /// An `ssh` to copy the file with.
    pub ssh: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub name: String,
    pub alias: String,
    /// `None`: nothing sent to it yet.
    pub state: Option<PeerState>,
    /// The state is about this Mac's file as it is now.
    pub current: bool,
    /// Its rules, when it holds another copy (newer, or edited there).
    pub their_rules: Option<Rules>,
    /// A copy is going to it now.
    pub pushing: bool,
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
    fn each_other_mac_is_shown_against_the_file_on_disk_now() {
        use quota_station_core::peers::PeerRecord;
        use quota_station_core::settings::Peer;
        let dir = std::env::temp_dir().join(format!("qs-state-rotation-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut inner = Inner::new(dir.clone(), dir.join("app/settings.json"), "Mac A".into());
        assert_eq!(inner.rotation_view().status, "missing");

        let at = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
        let stamp = Stamp::new(at, "Mac A");
        let text = rotation::render(&Rules::defaults(), &stamp, None);
        let file = rotation::path(&dir);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, &text).unwrap();
        let peer = |name: &str, alias: &str| Peer { name: name.into(), alias: alias.into() };
        inner.settings.peers =
            vec![peer("Mac B", "mac-b"), peer("Mac C", "mac-c"), peer("Mac D", "mac-d")];
        let newer = Stamp::new(at + time::Duration::minutes(9), "Mac B");
        let mut theirs = Rules::defaults();
        theirs.full = 88.0;
        inner.sync.peers.insert(
            "mac-b".into(),
            PeerRecord {
                state: PeerState::Newer { their: newer.clone() },
                against: rotation::version(&text),
                their_text: Some(rotation::render(&theirs, &newer, None)),
            },
        );
        inner.sync.peers.insert(
            "mac-c".into(),
            PeerRecord {
                state: PeerState::Synced { at: "2026-01-16T07:00:00Z".into() },
                against: "2026-01-16T13:00:00+07:00".into(),
                their_text: None,
            },
        );

        let view = inner.rotation_view();
        assert_eq!(view.status, "ok");
        assert_eq!(view.rules, Some(Rules::defaults()));
        assert_eq!(view.stamp, Some(stamp));
        assert_eq!(view.version, Some(rotation::version(&text)));
        assert_eq!(view.path, "~/.config/cswap-prefer-team/rotation");
        let [b, c, d] = &view.peers[..] else { panic!("three Macs") };
        assert!(b.current);
        assert_eq!(b.their_rules.as_ref().map(|r| r.full), Some(88.0));
        assert!(!c.current, "Mac C has an older file of this Mac");
        assert!(d.state.is_none() && !d.current, "Mac D was never sent anything");
        let _ = std::fs::remove_dir_all(&dir);
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
