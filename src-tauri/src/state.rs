// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the app holds between reads, and the model every window draws.

#[cfg(feature = "antigravity-accounts")]
use std::collections::BTreeMap;
#[cfg(all(feature = "author-tools", unix))]
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

use quota_station_core::alerts::{self, Memory};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::oauth_server;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::pace::Pace;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::{export, index};
use quota_station_core::app_dir::AppDir;
#[cfg(feature = "author-tools")]
use quota_station_core::claude::rotate;
use quota_station_core::claude::session;
use quota_station_core::claude::state::AccountOut;
use quota_station_core::claude::store::Store;
use quota_station_core::detail;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::detail::AgyDetail;
use quota_station_core::detail::ClaudeDetail;
#[cfg(feature = "author-tools")]
use quota_station_core::firstmate::{self, Reading};
use quota_station_core::history::Seen;
use quota_station_core::icon::{self, Context, Icon};
#[cfg(all(feature = "author-tools", unix))]
use quota_station_core::peers::{self, PeerState, SyncLog};
use quota_station_core::quota::Response;
#[cfg(feature = "author-tools")]
use quota_station_core::rotation::{self, FileState, Rules, Stamp};
#[cfg(feature = "author-tools")]
use quota_station_core::runner;
use quota_station_core::schedule::Schedule;
use quota_station_core::settings::{Language, Menubar, MenubarWindow, Settings, Theme, ThemePack};
#[cfg(feature = "author-tools")]
use quota_station_core::switcher::{Decision, DryRun, LogLine};
use quota_station_core::switching::Tools;
use quota_station_core::Snapshot;
#[cfg(feature = "author-tools")]
use quota_station_core::{local_time, pause};
use serde::{Deserialize, Serialize};
#[cfg(feature = "author-tools")]
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::agents::{self, McpView};
#[cfg(feature = "antigravity-accounts")]
use crate::agy_switch::{self, SwitchView};
#[cfg(feature = "author-tools")]
use crate::alerting::FirstmateView;
use crate::alerting::{self, AlertsView};
#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::{self, AccountsView};
use crate::codex::{self, CodexView};
use crate::install::{self, AppView, InstallState};
use crate::notify;

pub type AppState = Arc<Shared>;

pub struct Shared {
    inner: Mutex<Inner>,
    /// Wakes the reader thread early: a read was asked for, or settings changed.
    pub wake: Condvar,
    /// Wakes the rotation sync thread: a save, or the list of Macs changed.
    #[cfg(all(feature = "author-tools", unix))]
    pub sync_wake: Condvar,
    /// Held while copying the rotation file to the other Macs, so two copies
    /// to the same Mac never run at once.
    #[cfg(all(feature = "author-tools", unix))]
    pub push_lock: Mutex<()>,
    /// Wakes the Antigravity quota thread: a read, the tab, a login.
    pub agy_wake: Condvar,
    /// Held while Antigravity accounts are read and while one logs out, so
    /// a read never writes an account's token or reading back after its
    /// logout removed them.
    #[cfg(feature = "antigravity-accounts")]
    pub agy_accounts: Mutex<()>,
    /// Held while the history compares the newest readings, so a read and a
    /// switch never keep the same change twice.
    pub history: Mutex<()>,
    /// Wakes the Codex thread: a read, the Codex tab, a switch (M8).
    pub codex_wake: Condvar,
}

impl Shared {
    pub fn new(inner: Inner) -> AppState {
        Arc::new(Shared {
            inner: Mutex::new(inner),
            wake: Condvar::new(),
            #[cfg(all(feature = "author-tools", unix))]
            sync_wake: Condvar::new(),
            #[cfg(all(feature = "author-tools", unix))]
            push_lock: Mutex::new(()),
            agy_wake: Condvar::new(),
            #[cfg(feature = "antigravity-accounts")]
            agy_accounts: Mutex::new(()),
            history: Mutex::new(()),
            codex_wake: Condvar::new(),
        })
    }

    /// Asks the sync thread to look at the other Macs now.
    #[cfg(all(feature = "author-tools", unix))]
    pub fn request_sync(&self) {
        self.lock().sync_now = true;
        self.sync_wake.notify_all();
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Asks the reader thread for a read now, and the Codex thread for a look.
    pub fn request_read(&self) {
        {
            let mut inner = self.lock();
            inner.read_now = true;
            inner.codex.now = true;
        }
        self.wake.notify_all();
        self.codex_wake.notify_all();
    }

    /// Asks the reader thread to look at the switcher again: its dry run,
    /// its log and launchd.
    #[cfg(feature = "author-tools")]
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
    /// The last Codex report, for the menu bar and the Đang dùng block;
    /// none on a Mac without a Codex folder.
    pub codex_quota: Option<Response>,
    /// The Claude accounts the app keeps, with their last readings, and
    /// their emails and organizations only while names show. Read from the
    /// Keychain into memory; never written to disk.
    pub claude_list: Option<Vec<ClaudeDetail>>,
    /// The same accounts as the Claude tab lists them.
    pub claude_accounts: Vec<AccountOut>,
    /// The Claude tab's account actions (`crate::claude`).
    pub claude_live: crate::claude::Live,
    /// `claude`, to log a new account in with.
    pub claude_program: Option<PathBuf>,
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
    /// The owner's read of Claude Code's Keychain item runs.
    pub granting: bool,
    /// When the popover last hid itself on losing focus.
    pub popover_hidden_at: Option<Instant>,
    /// The tab the main window shows next: set when something opens it.
    pub main_tab: Tab,
    /// The group Cài đặt shows (`general`, `env`, …, as the page names
    /// them): the last one picked while the app runs, or the one an alert
    /// opened it on.
    pub settings_group: String,
    #[cfg(feature = "author-tools")]
    pub switcher: SwitcherState,
    #[cfg(feature = "author-tools")]
    pub switcher_now: bool,
    /// The slot a manual switch is running for.
    pub switching: Option<u32>,
    /// Where each other Mac stands with this Mac's rotation file.
    #[cfg(all(feature = "author-tools", unix))]
    pub sync: SyncLog,
    #[cfg(all(feature = "author-tools", unix))]
    pub sync_file: PathBuf,
    #[cfg(all(feature = "author-tools", unix))]
    pub sync_now: bool,
    /// Aliases of the Macs a copy is going to now.
    #[cfg(all(feature = "author-tools", unix))]
    pub pushing: BTreeSet<String>,
    /// A rotation save is running.
    #[cfg(feature = "author-tools")]
    pub rotation_saving: bool,
    /// The rules editor holds changes not saved yet: closing the main window asks first.
    #[cfg(feature = "author-tools")]
    pub rules_dirty: bool,
    /// `Thoát Quota Station` waits for the main window to ask about unsaved
    /// rules. Kept here, not only sent as an event, because a window opened
    /// for it may not listen yet; the page takes it with `take_quit`.
    #[cfg(feature = "author-tools")]
    pub quit_pending: bool,
    /// An Antigravity login is running.
    #[cfg(feature = "antigravity-accounts")]
    pub agy_logging_in: bool,
    /// The running login while it waits for the browser: takes a pasted
    /// callback or a cancel.
    #[cfg(feature = "antigravity-accounts")]
    pub agy_login: Option<oauth_server::Handle>,
    /// When each Antigravity account is read next (M4b).
    #[cfg(feature = "antigravity-accounts")]
    pub agy_pace: Pace,
    /// The Antigravity accounts being read now, or whose last read failed,
    /// by id. An account not here was read, or has nothing to report.
    #[cfg(feature = "antigravity-accounts")]
    pub agy_reads: BTreeMap<String, ReadState>,
    /// The Antigravity account number a manual switch is running for.
    #[cfg(feature = "antigravity-accounts")]
    pub agy_switching: Option<usize>,
    /// A logout runs: a switch, or another logout, is refused meanwhile, as
    /// a logout is while a switch runs.
    #[cfg(feature = "antigravity-accounts")]
    pub agy_logging_out: bool,
    /// The alerts sent and answered, kept in `alerts/alerts.json` (M6).
    pub alerts: Memory,
    /// The accounts the last readings showed in use, for the history.
    pub seen: Seen,
    /// Firstmate's state file as last read (at start, then after each read);
    /// `None` with no path set.
    #[cfg(feature = "author-tools")]
    pub firstmate: Option<Result<Reading, firstmate::ReadError>>,
    /// Firstmate's dry rule on the newest numbers, counted by the
    /// app with the same rotation the alerts use.
    #[cfg(feature = "author-tools")]
    pub own_dry: Option<bool>,
    /// macOS's notification center; `None` until the app has started it.
    pub center: Option<Arc<notify::Center>>,
    /// This copy's version, from `tauri.conf.json`.
    pub app_version: String,
    /// Launching at login (M7).
    // @private:begin
    /// And the update from source.
    // @private:end
    pub install: InstallState,
    /// Codex's files, processes and provider as last looked at (M8).
    pub codex: codex::Live,
    /// The MCP table as last read, and the last write (M9).
    pub mcp: agents::Live,
    /// The backup packs and the last restore (M10).
    pub backup: crate::backup::Live,
    /// The skills table, the plugins, the instruction files (M11).
    pub skills: crate::skills::Live,
    /// The check of GitHub Releases running now.
    pub release: crate::release::Live,
    /// Claude Code, Codex and agy as last found, and their check and update.
    pub cli_tools: crate::cli_tools::Live,
    #[cfg(all(feature = "author-tools", unix))]
    pub copy: crate::copy::Live,
    /// The variables that override a login (M14).
    pub env: crate::env::Live,
    /// The configuration sets' deleted set and running write (M15).
    pub sets: crate::sets::Live,
    /// The token statistics' index and running read (M16).
    pub stats: crate::stats::Live,
}

/// How the app's own read of an Antigravity account stands.
#[cfg(feature = "antigravity-accounts")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadState {
    Reading,
    /// `quota::ReadError::kind` and its message.
    Failed {
        kind: &'static str,
        detail: String,
    },
}

/// What the app last saw of the rotation.
#[cfg(feature = "author-tools")]
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
    /// The app's rotation LaunchAgent is loaded.
    pub service: Option<bool>,
    /// An older switcher still loaded (`com.cswap.prefer-team` or
    /// `com.cswap.auto`): nothing switches while it is.
    pub legacy: Option<String>,
    pub checked_at: Option<Instant>,
}

#[cfg(feature = "author-tools")]
#[derive(Debug)]
pub struct DryRunRecord {
    pub run: DryRun,
    pub at: OffsetDateTime,
    /// The pause flag held when the dry run started.
    pub flag_held: bool,
}

#[cfg(feature = "author-tools")]
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

/// The main window's tabs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tab {
    #[default]
    Overview,
    Stats,
    Claude,
    Agy,
    Codex,
    #[cfg(feature = "author-tools")]
    Rules,
    History,
    /// Not a page: the bell's panel (Thông báo), open over the page in view.
    Alerts,
    #[cfg(feature = "author-tools")]
    Firstmate,
    Mcp,
    Skills,
    Plugins,
    Instr,
    Sets,
    /// Cập nhật Agent: Claude Code, the Codex CLI and the Antigravity CLI.
    CliTools,
    Backup,
    #[cfg(all(feature = "author-tools", unix))]
    Copy,
    Settings,
}

impl Inner {
    pub fn new(home: PathBuf, settings_file: PathBuf, system_machine: String) -> Self {
        let settings = Settings::open(&settings_file);
        #[cfg(feature = "author-tools")]
        let firstmate = settings.firstmate_file.as_deref().map(firstmate::read);
        #[cfg(all(feature = "author-tools", unix))]
        let sync_file = settings_file.with_file_name("rotation-sync.json");
        let app_dir =
            AppDir::new(settings_file.parent().map(|p| p.to_path_buf()).unwrap_or_default());
        let claude_program = session::find_claude(&home);
        Inner {
            home,
            settings_file,
            settings,
            system_machine,
            claude: None,
            agy: None,
            codex_quota: None,
            claude_list: None,
            claude_accounts: Vec::new(),
            claude_live: crate::claude::Live::default(),
            claude_program,
            claude_generation: 0,
            agy_generation: 0,
            reading: false,
            reads: 0,
            read_now: true,
            schedule: Schedule::new(),
            granting: false,
            popover_hidden_at: None,
            main_tab: Tab::Overview,
            settings_group: "general".into(),
            #[cfg(feature = "author-tools")]
            switcher: SwitcherState::default(),
            #[cfg(feature = "author-tools")]
            switcher_now: true,
            switching: None,
            #[cfg(all(feature = "author-tools", unix))]
            sync: SyncLog::load(&sync_file),
            #[cfg(all(feature = "author-tools", unix))]
            sync_file,
            #[cfg(all(feature = "author-tools", unix))]
            sync_now: true,
            #[cfg(all(feature = "author-tools", unix))]
            pushing: BTreeSet::new(),
            #[cfg(feature = "author-tools")]
            rotation_saving: false,
            #[cfg(feature = "author-tools")]
            rules_dirty: false,
            #[cfg(feature = "author-tools")]
            quit_pending: false,
            #[cfg(feature = "antigravity-accounts")]
            agy_logging_in: false,
            #[cfg(feature = "antigravity-accounts")]
            agy_login: None,
            #[cfg(feature = "antigravity-accounts")]
            agy_pace: Pace::new(),
            #[cfg(feature = "antigravity-accounts")]
            agy_reads: BTreeMap::new(),
            #[cfg(feature = "antigravity-accounts")]
            agy_switching: None,
            #[cfg(feature = "antigravity-accounts")]
            agy_logging_out: false,
            alerts: alerts::load(&app_dir),
            seen: Seen::default(),
            #[cfg(feature = "author-tools")]
            firstmate,
            #[cfg(feature = "author-tools")]
            own_dry: None,
            center: None,
            app_version: String::new(),
            install: InstallState::default(),
            codex: codex::Live::default(),
            mcp: agents::Live::default(),
            backup: crate::backup::Live::default(),
            skills: crate::skills::Live::default(),
            release: crate::release::Live::default(),
            cli_tools: crate::cli_tools::Live::default(),
            env: crate::env::Live::default(),
            sets: crate::sets::Live::default(),
            stats: crate::stats::Live::default(),
            #[cfg(all(feature = "author-tools", unix))]
            copy: crate::copy::Live::default(),
        }
    }

    /// An Antigravity switch runs; never in a build without the accounts.
    pub fn agy_switching(&self) -> bool {
        #[cfg(feature = "antigravity-accounts")]
        return self.agy_switching.is_some();
        #[cfg(not(feature = "antigravity-accounts"))]
        false
    }

    /// The app's Application Support folder, where `settings.json` is.
    pub fn app_dir(&self) -> AppDir {
        AppDir::new(self.settings_file.parent().map(|p| p.to_path_buf()).unwrap_or_default())
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

    /// Where the Claude accounts the app keeps live on this Mac: in the
    /// folder Tauri gives the app, beside `settings.json`.
    pub fn claude_paths(&self) -> quota_station_core::claude::Paths {
        quota_station_core::claude::Paths::new(&self.home, self.app_dir().path())
    }

    /// The Claude accounts the app keeps, with the Keychain.
    pub fn claude_store(&self) -> Store {
        Store::new(self.claude_paths(), quota_station_core::os::secret_store())
    }

    /// What the rotation's dry run uses.
    #[cfg(feature = "author-tools")]
    pub fn rotation_tools(&self) -> rotate::Tools {
        rotate::Tools { claude: self.claude_program.clone(), ..rotate::Tools::system(&self.home) }
    }

    /// What a manual switch uses.
    pub fn switch_tools(&self) -> Tools {
        Tools {
            store: self.claude_store(),
            #[cfg(feature = "author-tools")]
            scheduler: std::sync::Arc::new(quota_station_core::os::scheduler()),
            #[cfg(feature = "author-tools")]
            pause_file: pause::path(&self.home),
        }
    }

    /// Reads the pause flag again after a command changed it; an unreadable
    /// flag counts as none.
    #[cfg(feature = "author-tools")]
    pub fn reload_pause(&mut self) {
        self.switcher.pause = pause::read(&pause::path(&self.home)).ok().flatten();
        self.switcher.pause_generation += 1;
    }

    /// Stores a Claude report read outside the reader loop (the Keychain
    /// grant, the re-read after a switch), ahead of any read in flight.
    pub fn store_claude(&mut self, report: Response) {
        self.claude = Some(report);
        self.claude_generation += 1;
    }

    /// The Antigravity account the last reading shows in use, by its place
    /// in this app's index.
    #[cfg(feature = "antigravity-accounts")]
    pub fn agy_active_number(&self) -> Option<usize> {
        let list = self.agy.as_ref()?.primary("agy")?.agy_accounts.as_ref()?;
        if list.status != "ok" {
            return None;
        }
        list.accounts.iter().find(|a| a.active).map(|a| a.account as usize)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::build(self.claude.as_ref(), self.agy.as_ref())
            .with_codex(self.codex_quota.as_ref())
    }

    /// The snapshot the windows draw: every account's windows from the
    /// app's own Claude and Antigravity readings, and the
    /// names unless the owner hides them, in which case none is sent.
    pub fn detailed_snapshot(&self) -> Snapshot {
        let mut snapshot = self.snapshot();
        let now = Some(OffsetDateTime::now_utc());
        if let Some(list) = &self.claude_list {
            detail::apply_claude(&mut snapshot.claude, list, now);
        }
        #[cfg(feature = "antigravity-accounts")]
        if snapshot.agy.source.as_deref() == Some(quota_station_core::snapshot::LIST_SOURCE) {
            detail::apply_agy(&mut snapshot.agy, &self.agy_details(), now);
        }
        if self.settings.hide_names {
            detail::hide_names(&mut snapshot.claude);
            detail::hide_names(&mut snapshot.agy);
        }
        snapshot
    }

    /// Each Antigravity account this app logs in, by its number, with its
    /// last reading.
    #[cfg(feature = "antigravity-accounts")]
    fn agy_details(&self) -> Vec<AgyDetail> {
        let app = self.app_dir();
        let Ok(index) = index::load(&app) else { return Vec::new() };
        index
            .accounts
            .iter()
            .enumerate()
            .map(|(i, a)| AgyDetail {
                number: (i + 1) as u32,
                email: Some(a.email.clone()),
                payload: export::read(&app, &a.email).map(|r| r.payload),
            })
            .collect()
    }

    pub fn model(&self) -> Model {
        self.model_with(self.detailed_snapshot())
    }

    /// The menu bar icon for `snapshot`, the detailed one the windows get:
    /// only it has the names and every window.
    // The binding stays for the build that adds to it.
    #[allow(clippy::let_and_return)]
    pub fn icon(&self, snapshot: &Snapshot, now: OffsetDateTime) -> Icon {
        let cx = Context {
            follow: self.settings.menubar,
            window: self.settings.menubar_window,
            codex: self.codex_use(),
            now,
            #[cfg(feature = "author-tools")]
            roles: self.settings.advanced_shown(),
        };
        let icon = icon::icon(self.claude.is_some().then_some(snapshot), &cx);
        #[cfg(feature = "author-tools")]
        let icon = icon.with_pause(self.switcher.paused(now));
        icon
    }

    /// The model for `snapshot`: [`Inner::detailed_snapshot`] reads the
    /// app's Antigravity files, so a caller that has one passes it in.
    pub fn model_with(&self, snapshot: Snapshot) -> Model {
        Model {
            snapshot,
            reading: self.reading,
            reads: self.reads,
            machine: self.machine(),
            machine_default: self.system_machine.clone(),
            granting: self.granting,
            #[cfg(feature = "author-tools")]
            switcher: self.switcher_view(OffsetDateTime::now_utc()),
            claude_switch: ClaudeSwitchView {
                ready: self.claude_paths().in_use(),
                busy: self.switching,
            },
            claude_accounts: crate::claude::view(self),
            show_names: !self.settings.hide_names,
            #[cfg(feature = "author-tools")]
            advanced: AdvancedView {
                on: self.settings.advanced_shown(),
                firstmate: self.settings.firstmate,
            },
            theme: self.settings.theme,
            theme_pack: self.settings.theme_pack,
            language: self.settings.language,
            menubar: self.settings.menubar,
            menubar_window: self.settings.menubar_window,
            #[cfg(feature = "antigravity-accounts")]
            antigravity: antigravity::view(self),
            #[cfg(feature = "antigravity-accounts")]
            agy_switch: agy_switch::view(self),
            capabilities: Capabilities::BUILT,
            #[cfg(feature = "author-tools")]
            rotation: self.rotation_view(),
            alerts: alerting::alerts_view(self),
            #[cfg(feature = "author-tools")]
            firstmate: alerting::firstmate_view(self),
            app: install::view(self),
            codex: codex::view(self),
            mcp: agents::view(self),
            backup: crate::backup::view(self),
            skills: crate::skills::skills_view(self),
            release: crate::release::view(self),
            cli_tools: crate::cli_tools::view(self),
            first_run: !self.settings.setup_seen,
            plugins: crate::skills::plugins_view(self),
            instructions: crate::skills::instr_view(self),
            env: crate::env::view(self),
            sets: crate::sets::view(self),
            stats: crate::stats::view(self),
        }
    }

    /// The rotation file as it is on disk now, and each other Mac against it
    /// (none on Windows, which copies it to no other Mac).
    #[cfg(feature = "author-tools")]
    pub fn rotation_view(&self) -> RotationView {
        let file = self.claude_paths().rotation_file();
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
        #[cfg(unix)]
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
            defaults: Rules::starter(
                &self
                    .claude_accounts
                    .iter()
                    .filter(|a| a.enabled)
                    .map(|a| a.slot)
                    .collect::<Vec<_>>(),
            ),
            saving: self.rotation_saving,
            #[cfg(unix)]
            peers,
            #[cfg(not(unix))]
            peers: [],
            #[cfg(unix)]
            ssh: peers::find_ssh(std::env::var_os("PATH").as_deref()).is_some(),
            #[cfg(not(unix))]
            ssh: false,
        }
    }

    #[cfg(feature = "author-tools")]
    fn switcher_view(&self, now: OffsetDateTime) -> SwitcherView {
        let sw = &self.switcher;
        let stamp = local_time::stamp(now);
        let rfc = |t: OffsetDateTime| t.format(&Rfc3339).ok();
        SwitcherView {
            installed: self.claude_paths().in_use(),
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
            legacy: sw.legacy.clone(),
        }
    }
}

/// Everything the popover and the main window draw.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub snapshot: Snapshot,
    pub reading: bool,
    pub reads: u64,
    pub machine: String,
    /// The name macOS gives this Mac, for the setting's placeholder.
    pub machine_default: String,
    /// The owner's read of Claude Code's Keychain item runs.
    pub granting: bool,
    #[cfg(feature = "author-tools")]
    pub switcher: SwitcherView,
    pub claude_switch: ClaudeSwitchView,
    /// The Claude accounts the app keeps, and their actions.
    pub claude_accounts: crate::claude::AccountsView,
    /// Account names (emails, organizations) may show.
    pub show_names: bool,
    /// Cài đặt › Nâng cao and the Firstmate switch inside it.
    #[cfg(feature = "author-tools")]
    pub advanced: AdvancedView,
    pub theme: Theme,
    /// The theme pack every window is drawn in.
    pub theme_pack: ThemePack,
    pub language: Language,
    /// The service the menu bar title follows.
    pub menubar: Menubar,
    /// The window whose share used the menu bar title shows.
    pub menubar_window: MenubarWindow,
    /// The Antigravity accounts this app logs in itself; only in a build
    /// with them.
    #[cfg(feature = "antigravity-accounts")]
    pub antigravity: AccountsView,
    /// Switching them (M4c).
    #[cfg(feature = "antigravity-accounts")]
    pub agy_switch: SwitchView,
    /// What this build has.
    pub capabilities: Capabilities,
    /// The rules editor and the other Macs; only in a build with the
    /// author's tools.
    #[cfg(feature = "author-tools")]
    pub rotation: RotationView,
    /// The alerts (M6).
    pub alerts: AlertsView,
    /// What Firstmate sees (M6); only in a build with the author's tools.
    #[cfg(feature = "author-tools")]
    pub firstmate: FirstmateView,
    /// The app itself: version, launching at login, the update (M7).
    pub app: AppView,
    /// Switching Codex (M8).
    pub codex: CodexView,
    /// Every agent's MCP servers (M9).
    pub mcp: McpView,
    pub backup: crate::backup::BackupView,
    /// The skills, the plugins and the instruction files (M11).
    pub skills: crate::skills::SkillsView,
    /// A newer version on GitHub Releases.
    pub release: crate::release::ReleaseView,
    /// Cài đặt › Chung › Cập nhật Agent.
    pub cli_tools: crate::cli_tools::CliToolsView,
    /// A new install: the main window shows the first run's steps.
    pub first_run: bool,
    pub plugins: crate::skills::PluginsView,
    pub instructions: crate::skills::InstrView,
    /// The variables that override a login (M14).
    pub env: crate::env::EnvView,
    /// The configuration sets (M15).
    pub sets: crate::sets::SetsView,
    /// The token statistics' read (M16); the numbers come with `stats_report`.
    pub stats: crate::stats::StatsView,
}

/// Cài đặt › Nâng cao: on, the tabs and switches made for the author's own
/// Macs show; `firstmate` is its own switch inside.
#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedView {
    pub on: bool,
    pub firstmate: bool,
}

/// The parts a build may leave out, so the windows draw only what it has.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// The Antigravity accounts the app logs in, reads and switches itself
    /// (the `antigravity-accounts` feature, CC BY-NC-SA 4.0). Without them
    /// Antigravity is the one account signed in on this Mac, read-only.
    pub agy_accounts: bool,
    /// The tools made for the author's own Macs (the `author-tools`
    /// feature, which a release build leaves out): Cài đặt › Nâng cao, the
    /// rules editor and the other Macs, the copy to another Mac, the update
    /// from source and Firstmate. Without them none of their commands is
    /// built and the windows show no switch, tab or setting for them.
    #[cfg(feature = "author-tools")]
    pub author_tools: bool,
    /// The system the app was built for, `macos` or `windows`: the windows
    /// take its type, corners and scroll bars, and its words for its own
    /// parts (Keychain, Finder, the menu bar).
    pub platform: &'static str,
    /// A test build for a guided session on a Windows PC (`crate::session`):
    /// the guide's window and its pictures. False in every other build.
    pub session: bool,
}

impl Capabilities {
    pub const BUILT: Capabilities = Capabilities {
        agy_accounts: cfg!(feature = "antigravity-accounts"),
        #[cfg(feature = "author-tools")]
        author_tools: true,
        platform: if cfg!(windows) { "windows" } else { "macos" },
        session: crate::session::ON,
    };
}

/// The rotation file and the other Macs (F06).
#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationView {
    /// `~/Library/Application Support/com.shin315.quota-station/claude/rotation`
    /// once the app keeps the accounts, else `~/.config/cswap-prefer-team/rotation`.
    pub path: String,
    /// `ok`, `missing`, `invalid` (the file's rules refuse it) or `unreadable`.
    pub status: &'static str,
    pub rules: Option<Rules>,
    /// Why the file is invalid, in the rule's words, or the read error.
    pub reason: Option<String>,
    /// Sent back with a save, so a file changed meanwhile is not overwritten.
    pub version: Option<String>,
    /// The `# updated = … by …` line of a valid file; `None` for a file the app
    /// never saved, and for a missing or invalid one.
    pub stamp: Option<Stamp>,
    pub ignored_keys: Vec<String>,
    /// What "Tạo từ mặc định" starts from ([`Rules::starter`]): the accounts
    /// in the rotation, each in turn. There is no built-in rotation.
    pub defaults: Rules,
    pub saving: bool,
    #[cfg(unix)]
    pub peers: Vec<PeerView>,
    /// Windows copies the file to no other Mac: always `[]`.
    #[cfg(not(unix))]
    pub peers: [(); 0],
    /// An `ssh` to copy the file with; never on Windows.
    pub ssh: bool,
}

#[cfg(all(feature = "author-tools", unix))]
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

/// The rotation as the windows show it (F07).
#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitcherView {
    /// The app keeps the Claude accounts on this Mac, so its rotation can run.
    pub installed: bool,
    pub pause: Option<PauseView>,
    pub dry_run: Option<DryRunView>,
    pub dry_run_error: Option<&'static str>,
    /// `false`: a dry run while the flag held did not say paused, so the
    /// rotation on this Mac does not read the pause flag.
    pub guard: Option<bool>,
    pub last_log: Option<LogLine>,
    /// The app's rotation LaunchAgent is loaded.
    pub service: Option<bool>,
    /// An older switcher still loaded: its label.
    pub legacy: Option<String>,
}

#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseView {
    /// RFC 3339; `None` pauses until the owner resumes.
    pub until: Option<String>,
    pub reason: Option<String>,
    /// The flag held when this model was built.
    pub holds: bool,
}

#[cfg(feature = "author-tools")]
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
    /// The app keeps the Claude accounts on this Mac; before that nothing
    /// can switch.
    pub ready: bool,
    /// The slot a switch is running for.
    pub busy: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inner() -> Inner {
        Inner::new(PathBuf::new(), PathBuf::from("/nonexistent/settings.json"), "Mac Studio".into())
    }

    #[cfg(all(feature = "author-tools", unix))]
    fn sample() -> Rules {
        Rules { team: vec![2, 3, 4], personal: Some(1), ..Rules::starter(&[]) }
    }

    #[test]
    fn the_machine_name_set_by_the_user_wins_over_the_one_macos_gives() {
        let mut inner = inner();
        assert_eq!(inner.model().machine, "Mac Studio");
        inner.settings.machine_name = Some("Mac phòng khách".into());
        let model = inner.model();
        assert_eq!(model.machine, "Mac phòng khách");
        assert_eq!(model.machine_default, "Mac Studio");
    }

    #[cfg(feature = "author-tools")]
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

    #[cfg(all(feature = "author-tools", unix))]
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
        let text = rotation::render(&sample(), &stamp, None);
        let file = rotation::path(&dir);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, &text).unwrap();
        let peer = |name: &str, alias: &str| Peer { name: name.into(), alias: alias.into() };
        inner.settings.peers =
            vec![peer("Mac B", "mac-b"), peer("Mac C", "mac-c"), peer("Mac D", "mac-d")];
        let newer = Stamp::new(at + time::Duration::minutes(9), "Mac B");
        let mut theirs = sample();
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
        assert_eq!(view.rules, Some(sample()));
        assert_eq!(view.stamp, Some(stamp));
        assert_eq!(view.version, Some(rotation::version(&text)));
        assert_eq!(
            view.path, "~/.config/cswap-prefer-team/rotation",
            "before the app keeps the accounts"
        );
        let [b, c, d] = &view.peers[..] else { panic!("three Macs") };
        assert!(b.current);
        assert_eq!(b.their_rules.as_ref().map(|r| r.full), Some(88.0));
        assert!(!c.current, "Mac C has an older file of this Mac");
        assert!(d.state.is_none() && !d.current, "Mac D was never sent anything");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A settings file with Nâng cao and Firstmate on: the windows get them,
    /// with the rules editor, Firstmate and, on a Mac, the update from
    /// source. Windows has no other Mac to copy the rules to.
    #[cfg(feature = "author-tools")]
    #[test]
    fn the_model_carries_the_author_tools() {
        let dir = std::env::temp_dir().join(format!("qs-state-author-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("app/settings.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, r#"{ "advanced": true, "firstmate": true, "sourceDir": "/srv/x" }"#)
            .unwrap();
        let inner = Inner::new(dir.clone(), file, "Mac A".into());
        let json = serde_json::to_value(inner.model()).unwrap();
        assert_eq!(json["capabilities"]["authorTools"], true);
        assert_eq!(json["advanced"]["on"], true);
        assert_eq!(json["advanced"]["firstmate"], true);
        assert!(json.get("rotation").is_some());
        assert!(json.get("firstmate").is_some());
        assert_eq!(json["app"].get("sourceStatus").is_some(), cfg!(unix));
        if cfg!(windows) {
            assert_eq!(json["rotation"]["peers"], serde_json::json!([]));
            assert_eq!(json["rotation"]["ssh"], false);
        }
        assert!(inner.settings.advanced && inner.settings.firstmate, "the file keeps them");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_model_says_what_this_build_has() {
        let json = serde_json::to_value(inner().model()).unwrap();
        let platform = if cfg!(windows) { "windows" } else { "macos" };
        assert_eq!(json["capabilities"]["platform"], platform);
        // Only CI's Windows installer is built for a guided session.
        assert_eq!(json["capabilities"]["session"], option_env!("QS_TEST_SESSION").is_some());
        let agy = cfg!(feature = "antigravity-accounts");
        assert_eq!(json["capabilities"]["agyAccounts"], agy);
        assert!(json["app"].get("version").is_some());
    }

    #[test]
    fn hidden_names_never_reach_the_windows() {
        let mut inner = inner();
        let report =
            include_str!("../../crates/quota-station-core/tests/fixtures/claude-pool.json");
        inner.claude = serde_json::from_str(report).ok();
        inner.claude_list = Some(vec![ClaudeDetail {
            number: 1,
            email: Some("lanpham.dev@example.com".into()),
            organization: Some("Acme Studio".into()),
            five_hour: Some(quota_station_core::detail::Usage { used: 0.0, resets_at: None }),
            seven_day: Some(quota_station_core::detail::Usage { used: 0.0, resets_at: None }),
            fable: Some(quota_station_core::detail::Usage { used: 4.0, resets_at: None }),
        }]);
        let model = inner.model();
        assert!(model.show_names);
        let first = &model.snapshot.claude.accounts[0];
        assert_eq!(first.email.as_deref(), Some("lanpham.dev@example.com"));
        assert_eq!(first.windows.len(), 3);

        inner.settings.hide_names = true;
        let model = inner.model();
        assert!(!model.show_names);
        let json = serde_json::to_string(&model).unwrap();
        assert!(!json.contains("@example.com") && !json.contains("Acme"));
        assert_eq!(model.snapshot.claude.accounts[0].windows.len(), 3, "the numbers stay");
        inner.claude_accounts = vec![AccountOut {
            slot: 1,
            alias: None,
            #[cfg(feature = "author-tools")]
            enabled: true,
            active: false,
            kind: Default::default(),
            status: "fresh",
            fetched_at: None,
            age_seconds: None,
            windows: quota_station_core::claude::state::Windows {
                five_hour: None,
                seven_day: None,
                fable: None,
            },
            error: None,
            email: Some("lanpham.dev@example.com".into()),
        }];
        let json = serde_json::to_string(&inner.model()).unwrap();
        assert!(!json.contains("@example.com"), "nor through the accounts' list");
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
