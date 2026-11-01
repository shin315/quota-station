// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the app holds between reads, and the model every window draws.

#[cfg(feature = "antigravity-accounts")]
use std::collections::BTreeMap;
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
use quota_station_core::claude::session;
use quota_station_core::claude::state::AccountOut;
use quota_station_core::claude::store::Store;
use quota_station_core::detail;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::detail::AgyDetail;
use quota_station_core::detail::ClaudeDetail;
use quota_station_core::ext::Hooks;
use quota_station_core::history::Seen;
use quota_station_core::icon::{self, Context, Icon};
use quota_station_core::quota::Response;
use quota_station_core::schedule::Schedule;
use quota_station_core::settings::{Language, Menubar, MenubarWindow, Settings, Theme, ThemePack};
use quota_station_core::switching::Tools;
use quota_station_core::Snapshot;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

use crate::agents::{self, McpView};
#[cfg(feature = "antigravity-accounts")]
use crate::agy_switch::{self, SwitchView};
use crate::alerting::{self, AlertsView};
#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::{self, AccountsView};
use crate::codex::{self, CodexView};
use crate::ext::{Kept, ModelInput, ModelSockets, NoExtension};
use crate::install::{self, AppView, InstallState};
use crate::notify;

pub type AppState = Arc<Shared>;

pub struct Shared {
    inner: Mutex<Inner>,
    /// Wakes the reader thread early: a read was asked for, settings
    /// changed, or the extension asked (`ext::Host::wake_reader`).
    pub wake: Condvar,
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
            agy_wake: Condvar::new(),
            #[cfg(feature = "antigravity-accounts")]
            agy_accounts: Mutex::new(()),
            history: Mutex::new(()),
            codex_wake: Condvar::new(),
        })
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

    /// Wakes the Antigravity quota thread after a change to its pace.
    pub fn wake_agy(&self) {
        self.agy_wake.notify_all();
    }
}

pub struct Inner {
    pub home: PathBuf,
    pub settings_file: PathBuf,
    pub settings: Settings,
    /// The hooks of the app's extension (`ext::Extension::hooks`), kept for
    /// as long as the app runs.
    pub hooks: &'static dyn Hooks,
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
    /// The slot a manual switch is running for.
    pub switching: Option<u32>,
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
    /// What the app's extension keeps in it (`ext::Host::keep_state`).
    pub kept: Kept,
    /// The extension's part of the model and of what the build has
    /// (`ext::Extension::model`); nothing until the app sets it at start.
    pub extension: &'static dyn ModelSockets,
    /// macOS's notification center; `None` until the app has started it.
    pub center: Option<Arc<notify::Center>>,
    /// This copy's version, from `tauri.conf.json`.
    pub app_version: String,
    /// Launching at login (M7).
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
    History,
    /// Not a page: the bell's panel (Thông báo), open over the page in view.
    Alerts,
    Mcp,
    Skills,
    Plugins,
    Instr,
    Sets,
    /// Cập nhật Agent: Claude Code, the Codex CLI and the Antigravity CLI.
    CliTools,
    Backup,
    Settings,
}

impl Inner {
    pub fn new(
        home: PathBuf,
        settings_file: PathBuf,
        system_machine: String,
        hooks: &'static dyn Hooks,
    ) -> Self {
        let settings = Settings::open(&settings_file, hooks);
        let app_dir =
            AppDir::new(settings_file.parent().map(|p| p.to_path_buf()).unwrap_or_default());
        let claude_program = session::find_claude(&home);
        Inner {
            home,
            settings_file,
            settings,
            hooks,
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
            switching: None,
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
            kept: Kept::default(),
            extension: &NoExtension,
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

    /// What a manual switch uses.
    pub fn switch_tools(&self) -> Tools {
        Tools { store: self.claude_store() }
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
        };
        let icon = icon::icon(self.claude.is_some().then_some(snapshot), &cx);
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
            claude_switch: ClaudeSwitchView {
                ready: self.claude_paths().in_use(),
                busy: self.switching,
            },
            claude_accounts: crate::claude::view(self),
            show_names: !self.settings.hide_names,
            theme: self.settings.theme,
            theme_pack: self.settings.theme_pack,
            language: self.settings.language,
            menubar: self.settings.menubar,
            menubar_window: self.settings.menubar_window,
            #[cfg(feature = "antigravity-accounts")]
            antigravity: antigravity::view(self),
            #[cfg(feature = "antigravity-accounts")]
            agy_switch: agy_switch::view(self),
            capabilities: Capabilities::built(self.extension.capabilities_part()),
            alerts: alerting::alerts_view(self),
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
            ext: self.extension.model_part(&ModelInput {
                kept: &self.kept,
                settings: &self.settings,
                home: &self.home,
                app_dir: &self.app_dir(),
                claude_accounts: &self.claude_accounts,
            }),
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
    pub claude_switch: ClaudeSwitchView,
    /// The Claude accounts the app keeps, and their actions.
    pub claude_accounts: crate::claude::AccountsView,
    /// Account names (emails, organizations) may show.
    pub show_names: bool,
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
    /// The alerts (M6).
    pub alerts: AlertsView,
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
    /// The extension's part (`ext::Extension::model`), its keys beside these.
    #[serde(flatten)]
    pub ext: Map<String, Value>,
}

/// The parts a build may leave out, so the windows draw only what it has.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// The Antigravity accounts the app logs in, reads and switches itself
    /// (the `antigravity-accounts` feature, CC BY-NC-SA 4.0). Without them
    /// Antigravity is the one account signed in on this Mac, read-only.
    pub agy_accounts: bool,
    /// What the extension adds (`ext::Extension::capabilities`), its keys
    /// here, after `agyAccounts`.
    #[serde(flatten)]
    pub ext: Map<String, Value>,
    /// The system the app was built for, `macos` or `windows`: the windows
    /// take its type, corners and scroll bars, and its words for its own
    /// parts (Keychain, Finder, the menu bar).
    pub platform: &'static str,
    /// A test build for a guided session on a Windows PC (`crate::session`):
    /// the guide's window and its pictures. False in every other build.
    pub session: bool,
}

impl Capabilities {
    /// What this build has, with what the extension adds (`ext`).
    pub fn built(ext: Map<String, Value>) -> Capabilities {
        Capabilities {
            agy_accounts: cfg!(feature = "antigravity-accounts"),
            ext,
            platform: if cfg!(windows) { "windows" } else { "macos" },
            session: crate::session::ON,
        }
    }
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
        Inner::new(
            PathBuf::new(),
            PathBuf::from("/nonexistent/settings.json"),
            "Mac Studio".into(),
            quota_station_core::ext::none(),
        )
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

    /// An extension that draws a part of the model from what the app hands
    /// it, and adds to what the build has.
    struct Draws;

    impl crate::ext::Extension for Draws {
        fn model(&self, input: &ModelInput<'_>) -> Map<String, Value> {
            let part = serde_json::json!({
                "machine": input.settings.machine_name,
                "accounts": input.claude_accounts.len(),
                "kept": input.kept.get::<u32>().map(|n| *n),
                "home": input.home,
            });
            Map::from_iter([("drawn".to_string(), part)])
        }

        fn capabilities(&self) -> Map<String, Value> {
            Map::from_iter([("drawn".to_string(), Value::Bool(true))])
        }
    }

    #[test]
    fn an_extension_draws_its_part_beside_the_apps_own() {
        let mut inner = inner();
        let before = serde_json::to_value(inner.model()).unwrap();
        inner.extension = &Draws;
        inner.settings.machine_name = Some("Mac A".into());
        inner.kept.keep(Arc::new(7_u32));
        let json = serde_json::to_value(inner.model()).unwrap();
        assert_eq!(
            json["drawn"],
            serde_json::json!({ "machine": "Mac A", "accounts": 0, "kept": 7, "home": "" })
        );
        assert_eq!(json["capabilities"]["drawn"], true);
        // The app's own keys are all still there, beside the extension's.
        let Value::Object(mut after) = json else { panic!("an object") };
        after.remove("drawn");
        after["capabilities"].as_object_mut().unwrap().remove("drawn");
        after.insert("machine".into(), before["machine"].clone());
        assert_eq!(Value::Object(after), before);
    }

    #[test]
    fn without_an_extension_the_model_has_only_the_apps_keys() {
        let model = inner().model();
        assert!(model.ext.is_empty() && model.capabilities.ext.is_empty());
        let json = serde_json::to_value(&model).unwrap();
        let keys: Vec<&str> =
            json["capabilities"].as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["agyAccounts", "platform", "session"]);
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

    /// A state with something in every part of the model, none of it from
    /// the clock, the time zone or the machine the test runs on.
    #[cfg(target_os = "macos")]
    fn fixed_state(dir: &std::path::Path) -> Inner {
        let home = dir.join("home");
        let file = dir.join("app/settings.json");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        #[allow(unused_mut)]
        let mut settings = serde_json::json!({
            "setupSeen": true,
            "machineName": "Mac phòng khách",
            "theme": "dark",
        });
        std::fs::write(&file, settings.to_string()).unwrap();
        let hooks = quota_station_core::ext::none();
        let mut inner = Inner::new(home.clone(), file, "Mac Studio".into(), hooks);
        inner.app_version = "1.2.3".into();
        inner.reading = true;
        inner.reads = 7;
        inner.switching = Some(2);
        inner.claude = serde_json::from_str(include_str!(
            "../../crates/quota-station-core/tests/fixtures/claude-pool.json"
        ))
        .ok();
        let account = |slot: u32, active: bool| AccountOut {
            slot,
            alias: Some(format!("tài khoản {slot}")),
            active,
            kind: Default::default(),
            status: "fresh",
            fetched_at: Some("2026-01-16T07:00:00Z".into()),
            age_seconds: Some(60),
            windows: quota_station_core::claude::state::Windows {
                five_hour: None,
                seven_day: None,
                fable: None,
            },
            error: None,
            email: None,
        };
        inner.claude_accounts = vec![account(1, false), account(2, true), account(3, false)];
        inner
    }

    /// The model JSON the windows get from [`fixed_state`] in this feature
    /// set, with the test's folder written as `/TEST` and the one thing
    /// read outside it (whether this Mac has CC Switch in /Applications) as
    /// `false`.
    #[cfg(target_os = "macos")]
    fn fixed_model_json() -> serde_json::Value {
        let dir = std::env::temp_dir().join(format!("qs-model-golden-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let inner = fixed_state(&dir);
        let text = serde_json::to_string(&inner.model()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let folder = serde_json::to_string(&dir.to_string_lossy()).unwrap();
        let folder = folder.trim_matches('"');
        let mut json: serde_json::Value =
            serde_json::from_str(&text.replace(folder, "/TEST")).unwrap();
        json["codex"]["ccSwitch"]["installed"] = false.into();
        json
    }

    /// The model JSON is the same, key for key and value for value, as the
    /// golden file of this feature set, written by the build before the
    /// extension drew its own part of it. Only on a Mac: the files are a
    /// Mac's model (Windows draws its own platform and paths).
    /// `UPDATE_MODEL_GOLDEN=1` rewrites the file after a change the windows
    /// are meant to see.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_model_json_is_the_golden_one_of_this_feature_set() {
        let golden = golden_model();
        let json = fixed_model_json();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&golden);
        if std::env::var_os("UPDATE_MODEL_GOLDEN").is_some() {
            std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap() + "\n").unwrap();
            return;
        }
        let text = std::fs::read_to_string(&path).expect("the golden file");
        let expected: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json, expected, "the model JSON differs from {golden}");
    }

    /// This feature set's golden model file, from the crate's folder.
    #[cfg(target_os = "macos")]
    fn golden_model() -> String {
        let dir = "tests/fixtures";
        let agy = if cfg!(feature = "antigravity-accounts") { "-antigravity-accounts" } else { "" };
        format!("{dir}/model{agy}.json")
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
