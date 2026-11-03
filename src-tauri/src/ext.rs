// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's sockets for an extension: what a build may add to the app
//! without the app's own code knowing what it adds. [`crate::run`] takes
//! one [`Extension`]; this repository's binary passes
//! [`quota_station::advanced::Advanced`](crate::advanced::Advanced), the
//! advanced part, and the full build (`ext/app`) passes a layer of its own over
//! it. [`NoExtension`] is the app with no extension: with it the app is
//! exactly what it is without the sockets. No binary passes it; it is the
//! state's extension until [`crate::run`] sets the binary's, and what the tests
//! of the app without an extension pass.
//!
//! Each socket is a method of [`Extension`] with a default that adds
//! nothing. The core's own sockets are [`Hooks`] (in
//! `quota_station_core::ext`), handed over by [`Extension::hooks`].
//!
//! Today's sockets:
//!
//! - [`Extension::hooks`]: the core's hooks. The app keeps them for as long
//!   as it runs and passes them where the core calls one (the settings file,
//!   the kinds of alert and which alerts run, the commands of
//!   `quota-station claude …`, the policy of the Claude pool at each read of
//!   the accounts and each pass of the alert rules, and what the extension
//!   does around the owner's manual switch of the Claude account, today).
//! - [`Extension::command_names`] and [`Extension::command_handler`]: the
//!   commands the windows can call besides the app's own, both made from one
//!   list by [`extension_commands!`](crate::extension_commands). A command
//!   name the app already has stays the app's ([`shadowed`] lists those).
//! - [`Extension::start`]: once, when the app starts, with the [`Host`], the
//!   extension's way to the app: the settings and a change of them saved
//!   ([`Host::update_settings`]), the home folder, the machine's name, the
//!   core's hooks the app keeps, the Claude accounts the app keeps (where
//!   they live, their store and the `claude` program the app found), a note
//!   in the log of a test session ([`Host::note`]), a wake for the app's
//!   reader, the model every window draws and a redraw of the windows and
//!   the menu bar ([`Host::publish`]), the main window's close, work under
//!   the app's lock ([`Host::locked`], where a reading of the Claude
//!   accounts may be held too), an action of the extension's on the
//!   Claude accounts the app keeps, run as the Claude tab runs its own
//!   ([`Host::claude_action`]), the agents' setup on this machine as the
//!   app reads it ([`Host::local`], which a terminal's subcommand opens
//!   without a window with [`Local::open`]), a write of that setup run as
//!   the app runs a restore's ([`Host::write_agents`]) and the backup the
//!   app makes after a change ([`Host::agents_changed`]), and a place for
//!   the extension's own state ([`Host::keep_state`]), which its commands
//!   take as Tauri state.
//!   The extension's commands reach the host with [`Host::of`] and return
//!   the [`Model`] as the app's own commands do.
//! - [`Extension::start_threads`]: once, when the app's own reader runs: the
//!   extension's own threads start here.
//! - [`Extension::reader_asked`] and [`Extension::reader_pass`]: the app's
//!   reader goes at once when the extension asks, and runs the extension's
//!   work at each pass, beside its read or alone between reads
//!   ([`ReaderPass`]).
//! - [`Extension::model`] and [`Extension::capabilities`]: the extension's
//!   part of the model every window draws, and of what the build has
//!   (`capabilities` in it), as JSON keys beside the app's own, drawn from
//!   what the app hands it ([`ModelInput`]).
//! - [`Extension::view_keys`]: the extension's keys inside some of the
//!   app's own views in that model ([`View`]), drawn from the same.
//! - [`Extension::tabs`]: the main window's tabs the extension adds to the
//!   app's own, by the names the windows know them by. A name that is one of
//!   the app's tabs stays the app's ([`shadowed_tabs`] lists those).
//! - [`Extension::hold_close`]: whether the main window closes, and the app
//!   quits, at once when the owner asks ([`Closing`]), or waits while the
//!   main window asks the owner first ([`Hold`]), for work of the
//!   extension's that is not kept yet.
//! - [`Extension::alert_read`]: what the extension reads for its own
//!   alerts after each read, before the app's alert rules run
//!   ([`AlertInput`]); its alerts at that pass come from it
//!   ([`AlertRead::source`]), of the kinds its hooks name
//!   ([`Hooks::alert_kinds`]).
//! - [`Extension::after_switch`]: once the owner's manual switch of the
//!   Claude account is over, the extension's own state follows it, and the
//!   app's reader may go at once for what the extension asks of it (what
//!   the extension does around the switch itself is a core hook,
//!   [`Hooks::manual_switch`]).
//! - [`Extension::after_accounts_change`]: once an action of the Claude tab
//!   on the accounts the app keeps is over, the extension's own state
//!   follows it, and the app's reader may go at once for what the extension
//!   asks of it.
//! - [`Extension::icon_badge`]: a badge on the menu bar icon, drawn inside
//!   its gauge, with words its label reads ([`Badge`]), from what the
//!   extension keeps.
//! - [`Extension::subcommand`]: a subcommand from a terminal
//!   (`quota-station <word> …`) that is not one of the app's, answered
//!   before the app starts and without a window. The app's own subcommands
//!   (`claude`, `quota`) stay the app's; the commands the extension adds to
//!   `quota-station claude …` are a core hook ([`Hooks::claude_command`]).
//! - [`Extension::build_name`], [`Extension::diagnostics`],
//!   [`Extension::self_check`] and [`Extension::hidden_keys`]: the
//!   extension's part of the app's diagnostics: its name for the build and
//!   its words on the system line in the diagnostic info, its own
//!   self-checks in a guided test session, and the keys the diagnostic
//!   bundle hides besides the app's.
//!
//! The names and the handler are apart so a test can read the names without
//! building the handler: a handler holds Tauri's window runtime, which a
//! Windows test program cannot load (it lacks the app's manifest, which
//! only the binary gets). The [`Host`] holds that runtime too, so no test
//! makes one; what the extension keeps is plain data ([`Kept`]), and so is
//! what the reader hands it ([`ReaderPass`]), what the model is drawn
//! from ([`ModelInput`]) and the agents' setup ([`Local`]). The app's state keeps only the extension's
//! sockets it asks under its lock (the model and the keys in its views,
//! what the build has, the tabs and the hold on a close, what it reads for
//! its alerts, what follows a manual switch or a change of the Claude
//! accounts, the icon's badge, and its part of the diagnostics), apart from
//! the rest, so a test can build that state and its model without the
//! runtime.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use quota_station_core::alerts::ExtSource;
use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::state::AccountOut;
use quota_station_core::settings::Settings;
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;

pub use quota_station_core::ext::{Hooks, NoHooks};
pub use quota_station_core::icon::{Badge, Shape};

/// The model every window draws, as the app's commands return it after a
/// change: an extension's command returns it too (`Result<Model, String>`).
pub use crate::state::Model;
/// The main window's tabs, the app's own and the extension's
/// ([`Extension::tabs`]), as the windows name them.
pub use crate::state::Tab;

/// How an action on the Claude accounts ended, as the Claude tab shows it
/// (`claudeAccounts.last`): an extension's action ([`Host::claude_action`])
/// ends with one too.
pub use crate::claude::Done;
/// Why an action on a Claude account failed, as the core tells it.
pub use quota_station_core::claude::manage::ManageError;

/// The commit the running app was built from, `None` for a build made
/// another way (the app's own row in the settings shows it).
pub use crate::install::COMMIT;

/// A pack's sections besides the MCP lists (the skills, the plugins, the
/// instruction files), and the folders of the skills it copies, as the
/// app's backup gathers them ([`Local::pack_extras`]).
pub use crate::skills::PackExtras;

use quota_station_core::agents::restore;
use quota_station_core::agents::skills::SkillPlaces;
use quota_station_core::agents::Places;
use quota_station_core::claude::store::Store;
use quota_station_core::claude::Paths;
use quota_station_core::quota::Response;

use crate::state::{AppState, Inner, Shared};
use crate::{agents, backup, claude, reader, session, windows};

/// What an extension adds to the app. Every method has a default that adds
/// nothing; an extension overrides the sockets it fills.
pub trait Extension: Send + Sync + 'static {
    /// The hooks the extension gives the core.
    fn hooks(&self) -> &dyn Hooks {
        quota_station_core::ext::none()
    }

    /// The names of the commands the extension adds to the app's own, as
    /// the windows call them, in the order they were listed.
    fn command_names(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// The handler that answers those commands. Asked once, when the app
    /// starts.
    fn command_handler(&self) -> Handler {
        Box::new(|_| false)
    }

    /// Called once when the app starts: after its state is made and before
    /// its threads start or a window opens. The extension keeps what it
    /// needs of `host`.
    fn start(&self, host: Host) {
        let _ = host;
    }

    /// Called once when the app starts, after [`Extension::start`], once the
    /// menu bar is up and the app's reader runs: the extension starts its
    /// own threads here.
    fn start_threads(&self, host: Host) {
        let _ = host;
    }

    /// Whether the extension asks the app's reader to go now rather than
    /// at its next time. Asked each time the reader is about to wait, under
    /// the app's lock: it reads only `kept`, never the [`Host`].
    fn reader_asked(&self, kept: &Kept) -> bool {
        let _ = kept;
        false
    }

    /// The extension's work at one pass of the app's reader, asked under
    /// the app's lock: it reads only `pass`, never the [`Host`]. With a read
    /// due ([`ReaderPass::read_due`]) the work runs in a thread of its own
    /// beside the read and is finished after it; without one (the reader
    /// went because [`Extension::reader_asked`] said so) it runs alone, and
    /// the reader waits again after it. Either way what the work returns is
    /// called under the app's lock with what the extension keeps, and the
    /// windows and the menu bar are redrawn. An extension that asked clears
    /// what made it ask here.
    fn reader_pass(&self, pass: &ReaderPass<'_>) -> Option<ReaderWork> {
        let _ = pass;
        None
    }

    /// The extension's part of the model every window draws: JSON keys the
    /// windows find beside the app's own, at the model's top level. Asked
    /// each time the model is drawn, under the app's lock: it reads only
    /// `input`, never the [`Host`]. Its keys must not be any of the app's.
    fn model(&self, input: &ModelInput<'_>) -> Map<String, Value> {
        let _ = input;
        Map::new()
    }

    /// The extension's part of what the build has (`capabilities` in the
    /// model): JSON keys beside the app's own, none of them the app's.
    fn capabilities(&self) -> Map<String, Value> {
        Map::new()
    }

    /// The extension's keys in one of the app's own views in the model
    /// ([`View`]): JSON keys the windows find beside the view's own, inside
    /// it. Asked each time the view is drawn, under the app's lock: it reads
    /// only `input`, never the [`Host`]. Its keys must not be any of the
    /// view's.
    fn view_keys(&self, view: View, input: &ModelInput<'_>) -> Map<String, Value> {
        let _ = (view, input);
        Map::new()
    }

    /// The main window's tabs the extension adds to the app's own, by the
    /// names the windows know them by (the name the window asks to open, and
    /// the one the app tells it to show). A window asking for any other name
    /// than the app's tabs and these opens nothing. A name that is one of the
    /// app's tabs stays the app's.
    fn tabs(&self) -> &[&'static str] {
        &[]
    }

    /// Whether `closing` happens at once ([`Hold::Close`]) or waits while
    /// the main window asks the owner first ([`Hold::Ask`]), say because
    /// the extension holds work the owner has not kept. Asked each time the
    /// owner closes the main window or quits the app, under the app's lock:
    /// it reads only `kept`, never the [`Host`]. Once the owner answered,
    /// the extension's own commands close the window or quit.
    fn hold_close(&self, kept: &Kept, closing: Closing) -> Hold {
        let _ = (kept, closing);
        Hold::Close
    }

    /// What the extension reads for its own alerts, after each read of the
    /// app's, before the app's alert rules run: without the app's lock, so
    /// it may read files, but from `input` only, never the [`Host`]. Its
    /// alerts at this pass come from what it returns
    /// ([`AlertRead::source`]), and it gets what it read back once the
    /// rules ran ([`AlertRead::done`]). `None`, the default, adds no alert.
    fn alert_read(&self, input: &AlertInput<'_>) -> Option<Box<dyn AlertRead>> {
        let _ = input;
        None
    }

    /// The owner's manual switch of the Claude account is over, whatever
    /// its outcome, and the app has kept the report it brought: called
    /// under the app's lock with what the extension keeps and the owner's
    /// home (never the [`Host`]), so the extension's own state can follow
    /// what the switch changed. `true` asks the app to wake its reader once
    /// the switch is in the history, for what the extension then asks of it
    /// ([`Extension::reader_asked`]). The default changes nothing and
    /// returns `false`.
    fn after_switch(&self, kept: &Kept, home: &Path) -> bool {
        let _ = (kept, home);
        false
    }

    /// An action of the Claude tab on the accounts the app keeps is over
    /// (an import, a login, an account added, removed or renamed, whatever
    /// its outcome), and the app has taken the accounts in again and asked
    /// for a read: called under the app's lock with what the extension
    /// keeps (never the [`Host`]), so its own state can follow the change.
    /// `true` asks the app to wake its reader then, for what the extension
    /// asks of it ([`Extension::reader_asked`]). The default changes nothing
    /// and returns `false`.
    fn after_accounts_change(&self, kept: &Kept) -> bool {
        let _ = kept;
        false
    }

    /// The extension's badge on the menu bar icon at `now`: shapes the icon
    /// draws inside its gauge and words its label reads after the app's
    /// ([`Badge`]). Asked each time the icon is built, under the app's lock:
    /// it reads only `kept`, never the [`Host`]. Claude used up and no
    /// number keep their own glyph whatever it answers. `None`, the
    /// default, leaves the icon the app's own.
    fn icon_badge(&self, kept: &Kept, now: OffsetDateTime) -> Option<Badge> {
        let _ = (kept, now);
        None
    }

    /// A subcommand from a terminal: `args` are the words after
    /// `quota-station`, the subcommand first (none when the app is simply
    /// launched). The extension answers with the exit code, having printed
    /// what it prints, and the app exits with it without starting; or
    /// `None`, the default, when the words are not its own: then the app
    /// answers them, or starts. Asked before the app starts, so the
    /// extension has no [`Host`] and nothing it keeps yet; `hooks` are the
    /// core's hooks the app runs with (those of the extension the app was
    /// given, which may wrap this one), for a subcommand that opens the
    /// app's state without a window ([`Local::open`]). The app's own
    /// subcommands (`claude`, `quota`) never reach the extension.
    fn subcommand(&self, args: &[String], hooks: &'static dyn Hooks) -> Option<i32> {
        let _ = (args, hooks);
        None
    }

    /// The extension's name for this build in the app's diagnostic info
    /// (what "Chép" copies), after the app's own on its first line. `None`,
    /// the default, adds no name.
    fn build_name(&self) -> Option<&'static str> {
        None
    }

    /// Words the extension adds at the end of the system line of the app's
    /// diagnostic info, each after a ` · `, read from `settings`. Asked
    /// under the app's lock; a word names no person, account, path or
    /// secret. The default adds none.
    fn diagnostics(&self, settings: &Settings) -> Vec<String> {
        let _ = settings;
        Vec::new()
    }

    /// The extension's self-check `id` in a guided test session, read from
    /// the app's folder `app_dir`: asked only for an id that is none of the
    /// app's, under the app's lock. It reads no secret: files counted, their
    /// times, never their text. `None`, the default, when the id is not its
    /// own: the session then reports the id unknown.
    fn self_check(&self, id: &str, app_dir: &AppDir) -> Option<Value> {
        let _ = (id, app_dir);
        None
    }

    /// Keys whose value never goes in the app's diagnostic bundle, besides
    /// the app's own, matched as those are: whatever their case, `_` and
    /// `-` left out. The default hides no more.
    fn hidden_keys(&self) -> &[&'static str] {
        &[]
    }
}

/// What the owner is closing ([`Extension::hold_close`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closing {
    /// The main window, by its close button or the system's shortcut.
    MainWindow,
    /// The app, by `Thoát Quota Station` in the menu bar item's menu.
    App,
}

/// The extension's answer when the owner closes the main window or quits
/// ([`Extension::hold_close`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Hold {
    /// It closes at once, as without the extension.
    #[default]
    Close,
    /// It waits: the main window stays and the app tells it to ask the
    /// owner (the `closeRequested` event when the window is closing,
    /// `quitRequested` when the app is). The window comes forward on `tab`,
    /// one of the main window's tabs, when it names one; a quit opens the
    /// window if it is not open.
    Ask {
        /// The tab the window asks on, by its name; `None` keeps the tab in
        /// view.
        tab: Option<&'static str>,
    },
}

/// One of the app's own views in the model, which the extension may add
/// keys to ([`Extension::view_keys`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The Claude accounts the app keeps, as the Claude tab shows them
    /// (`claudeAccounts`).
    ClaudeAccounts,
    /// The app itself, as its row in the settings shows it (`app`).
    App,
}

/// What the extension draws its part of the model from
/// ([`Extension::model`], [`Extension::view_keys`]): plain data, taken
/// under the app's lock.
pub struct ModelInput<'a> {
    /// What the extension keeps ([`Host::keep_state`]).
    pub kept: &'a Kept,
    /// The settings as the app holds them now.
    pub settings: &'a Settings,
    /// The home folder of the user the app runs as.
    pub home: &'a Path,
    /// The app's folder, where its settings file is.
    pub app_dir: &'a AppDir,
    /// The Claude accounts the app keeps, as its Claude tab lists them.
    pub claude_accounts: &'a [AccountOut],
}

/// What the extension reads its alerts' facts with ([`Extension::alert_read`]):
/// plain data, taken under the app's lock and handed over without it.
pub struct AlertInput<'a> {
    /// The settings as the app holds them now.
    pub settings: &'a Settings,
    /// The home folder of the user the app runs as.
    pub home: &'a Path,
    /// The app's folder, where its settings file is.
    pub app_dir: &'a AppDir,
}

/// What the extension read for its own alerts at one pass of the app's
/// alert rules ([`Extension::alert_read`]).
pub trait AlertRead {
    /// Its alerts at this pass, drawn from what it read.
    fn source(&self) -> &dyn ExtSource;

    /// Called once the rules ran and their cards went out, under the app's
    /// lock, with what the extension keeps: what it read is its again (to
    /// draw in its part of the model, say). It never calls the [`Host`].
    fn done(self: Box<Self>, kept: &Kept);
}

/// The extension's sockets the app asks under its lock (what draws,
/// [`Extension::model`], [`Extension::capabilities`] and
/// [`Extension::view_keys`], the tabs, [`Extension::tabs`], the hold on a
/// close, [`Extension::hold_close`], what follows a manual switch,
/// [`Extension::after_switch`], and a change of the Claude accounts,
/// [`Extension::after_accounts_change`], the icon's badge,
/// [`Extension::icon_badge`], and its part of the app's diagnostics,
/// [`Extension::build_name`], [`Extension::diagnostics`],
/// [`Extension::self_check`] and [`Extension::hidden_keys`]), and what it
/// reads for its alerts ([`Extension::alert_read`]), as the
/// app's state keeps them: apart from the rest
/// of [`Extension`], so that state holds nothing of the window runtime the
/// command handler brings, and a test can build it.
pub(crate) trait StateSockets: Send + Sync {
    fn model_part(&self, input: &ModelInput<'_>) -> Map<String, Value>;
    fn capabilities_part(&self) -> Map<String, Value>;
    fn view_keys_part(&self, view: View, input: &ModelInput<'_>) -> Map<String, Value>;
    fn tabs_part(&self) -> &[&'static str];
    fn hold_close_part(&self, kept: &Kept, closing: Closing) -> Hold;
    fn alert_read_part(&self, input: &AlertInput<'_>) -> Option<Box<dyn AlertRead>>;
    fn after_switch_part(&self, kept: &Kept, home: &Path) -> bool;
    fn after_accounts_change_part(&self, kept: &Kept) -> bool;
    fn icon_badge_part(&self, kept: &Kept, now: OffsetDateTime) -> Option<Badge>;
    fn build_name_part(&self) -> Option<&'static str>;
    fn diagnostics_part(&self, settings: &Settings) -> Vec<String>;
    fn self_check_part(&self, id: &str, app_dir: &AppDir) -> Option<Value>;
    fn hidden_keys_part(&self) -> &[&'static str];
}

impl<E: Extension> StateSockets for E {
    fn model_part(&self, input: &ModelInput<'_>) -> Map<String, Value> {
        self.model(input)
    }

    fn capabilities_part(&self) -> Map<String, Value> {
        self.capabilities()
    }

    fn view_keys_part(&self, view: View, input: &ModelInput<'_>) -> Map<String, Value> {
        self.view_keys(view, input)
    }

    fn tabs_part(&self) -> &[&'static str] {
        self.tabs()
    }

    fn hold_close_part(&self, kept: &Kept, closing: Closing) -> Hold {
        self.hold_close(kept, closing)
    }

    fn alert_read_part(&self, input: &AlertInput<'_>) -> Option<Box<dyn AlertRead>> {
        self.alert_read(input)
    }

    fn after_switch_part(&self, kept: &Kept, home: &Path) -> bool {
        self.after_switch(kept, home)
    }

    fn after_accounts_change_part(&self, kept: &Kept) -> bool {
        self.after_accounts_change(kept)
    }

    fn icon_badge_part(&self, kept: &Kept, now: OffsetDateTime) -> Option<Badge> {
        self.icon_badge(kept, now)
    }

    fn build_name_part(&self) -> Option<&'static str> {
        self.build_name()
    }

    fn diagnostics_part(&self, settings: &Settings) -> Vec<String> {
        self.diagnostics(settings)
    }

    fn self_check_part(&self, id: &str, app_dir: &AppDir) -> Option<Value> {
        self.self_check(id, app_dir)
    }

    fn hidden_keys_part(&self) -> &[&'static str] {
        self.hidden_keys()
    }
}

/// The app's reader at one pass, as an extension sees it
/// ([`Extension::reader_pass`]): plain data, taken under the app's lock.
pub struct ReaderPass<'a> {
    /// What the extension keeps ([`Host::keep_state`]).
    pub kept: &'a Kept,
    /// The home folder of the user the app runs as.
    pub home: &'a Path,
    /// The app's folder, where its settings file is.
    pub app_dir: &'a AppDir,
    /// The quota is read on this pass: asked for, or on its schedule.
    pub read_due: bool,
    /// A window of the app is open, so the reader reads more often.
    pub window_open: bool,
}

/// The extension's work at a pass of the reader, run without the app's
/// lock. What it returns finishes it under that lock.
pub type ReaderWork = Box<dyn FnOnce() -> ReaderDone + Send>;

/// What finishes an extension's work at a pass of the reader, called under
/// the app's lock with what the extension keeps.
pub type ReaderDone = Box<dyn FnOnce(&Kept) + Send>;

/// No extension: the app as it is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoExtension;

impl Extension for NoExtension {}

/// The handler of an extension's commands, as Tauri calls one: it answers
/// the commands it knows and returns `false` for any other.
pub type Handler = Box<dyn Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static>;

/// The extension's way to the app while it runs. A cheap clone: keep one
/// where it is needed.
///
/// The app may hold its own lock while it takes a lock of the extension's
/// state (to draw it, say), so the extension never calls the host while it
/// holds a lock of its own.
#[derive(Clone)]
pub struct Host {
    app: AppHandle,
    state: AppState,
}

impl Host {
    pub(crate) fn new(app: AppHandle, state: AppState) -> Host {
        Host { app, state }
    }

    /// The host of the running app `app`, as a command reaches it. Only
    /// once the app has started.
    pub fn of(app: &AppHandle) -> Host {
        Host::new(app.clone(), app.state::<AppState>().inner().clone())
    }

    /// The settings as the app holds them now.
    pub fn settings(&self) -> Settings {
        self.state.lock().settings.clone()
    }

    /// Changes the settings with `change`, saves them to the app's settings
    /// file and holds them from now on, all under the app's lock. A save
    /// that fails changes nothing and returns its error as words. The
    /// windows see the change once the extension publishes it
    /// ([`Host::publish`]).
    pub fn update_settings(&self, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
        self.state.lock().change_settings(change)
    }

    /// The home folder of the user the app runs as.
    pub fn home(&self) -> PathBuf {
        self.state.lock().home.clone()
    }

    /// The name shown for this machine: the owner's, else the system's.
    pub fn machine(&self) -> String {
        self.state.lock().machine()
    }

    /// The core's hooks the app keeps ([`Extension::hooks`]), as it passes
    /// them where the core calls one.
    pub fn hooks(&self) -> &'static dyn Hooks {
        self.state.lock().hooks
    }

    /// Where the Claude accounts the app keeps live on this machine.
    pub fn claude_paths(&self) -> Paths {
        self.state.lock().claude_paths()
    }

    /// The Claude accounts the app keeps, with their logins and what the
    /// extension keeps on each, as the app opens them.
    pub fn claude_store(&self) -> Store {
        self.state.lock().claude_store()
    }

    /// The `claude` program the app found when it started, if it found one.
    pub fn claude_program(&self) -> Option<PathBuf> {
        self.state.lock().claude_program.clone()
    }

    /// Writes `kind` and `data` to the log of the test session that runs
    /// now, as the app notes its own events there; nothing outside one.
    pub fn note(&self, kind: &str, data: Value) {
        session::note(kind, data);
    }

    /// Runs `work` under the app's lock, with the app's state as it holds it
    /// ([`Locked`]), for work that must stay whole with that state: a model
    /// drawn meanwhile sees all of it or none. `work` may take a lock of the
    /// extension's own inside (the app's lock first, then the extension's),
    /// and never calls the host, whose every call takes the app's lock.
    pub fn locked<R>(&self, work: impl FnOnce(&mut Locked<'_>) -> R) -> R {
        let mut inner = self.state.lock();
        work(&mut Locked::new(&mut inner))
    }

    /// The model every window draws, as it is now.
    pub fn model(&self) -> Model {
        self.state.lock().model()
    }

    /// Sends the model to the windows and redraws the menu bar, after a
    /// change of the app's state or of the extension's.
    pub fn publish(&self) {
        reader::publish(&self.app, &self.state);
    }

    /// Closes the main window, if it is open.
    pub fn close_main_window(&self) {
        windows::close_main(&self.app);
    }

    /// The app's folder, where its settings file is.
    pub fn app_dir(&self) -> AppDir {
        self.state.lock().app_dir()
    }

    /// Keeps `state` for as long as the app runs, once for its type: the
    /// extension's commands take it as Tauri state (`State<'_, Arc<T>>`).
    pub fn keep_state<T: Any + Send + Sync>(&self, state: Arc<T>) {
        self.state.lock().kept.keep(state.clone());
        self.app.manage(state);
    }

    /// The value of type `T` the extension keeps ([`Host::keep_state`]), if
    /// it keeps one.
    pub fn kept_state<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.state.lock().kept.get()
    }

    /// Wakes the app's reader, so what the extension asked of it is seen at
    /// once. Taken under the app's lock: a reader about to wait cannot miss it.
    pub fn wake_reader(&self) {
        let _held = self.state.lock();
        self.state.wake.notify_all();
    }

    /// The running app, as Tauri hands it over: for the extension's own
    /// threads, which tell the windows what changed.
    pub fn app_handle(&self) -> &AppHandle {
        &self.app
    }

    /// Runs `work` on the Claude accounts the app keeps as the Claude tab
    /// runs its own actions, named `action` there: one action at a time
    /// (`Err("busy")` while another runs), off the main thread, with the
    /// accounts' store; then its [`Done`] shown, the accounts taken in
    /// again, the extension told ([`Extension::after_accounts_change`]),
    /// everything read again and the windows redrawn. Returns the model, as
    /// the app's own actions do.
    pub async fn claude_action(
        &self,
        action: &'static str,
        work: impl FnOnce(&Store) -> Done + Send + 'static,
    ) -> Result<Model, String> {
        claude::run(self.app.clone(), self.state.clone(), action, work).await
    }

    /// The agents' setup on this machine as the running app reads it
    /// ([`Local`]).
    pub fn local(&self) -> Local {
        Local { state: self.state.clone() }
    }

    /// Runs `work`, a write of the agents' setup, as the app runs a
    /// restore's: one write at a time with the app's own (`Err("busy")`
    /// while one runs), off the main thread, with where each agent keeps
    /// its MCP list, the app's folder and what a restore uses; then the
    /// Agents tabs read again. `Err` too when `work` did not finish.
    pub async fn write_agents<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Places, &AppDir, &restore::Tools) -> T + Send + 'static,
    ) -> Result<T, String> {
        agents::write_as_restore(&self.app, &self.state, work).await
    }

    /// The agents' setup changed: the app makes a backup pack, as after a
    /// change of its own, when the owner asked for one after each change.
    pub fn agents_changed(&self) {
        backup::after_change(&self.app, &self.state);
    }
}

/// The agents' setup on this machine, as the app reads it: where each
/// agent keeps its MCP list and its skills, the Codex folder, the app's
/// folder, the machine's name, the app's version, a pack's other sections
/// and what a restore uses. The running app's is [`Host::local`]; a
/// subcommand from a terminal, which runs without a window, opens its own
/// ([`Local::open`]). It holds no window runtime, so a test may open one.
#[derive(Clone)]
pub struct Local {
    state: AppState,
}

impl Local {
    /// The app's state for `home` as the app would start with it, without
    /// a window: the settings file in the app's folder for `home`, the
    /// name the system gives this machine, this build's version, and
    /// `hooks` where the core calls one.
    pub fn open(home: PathBuf, hooks: &'static dyn Hooks) -> Local {
        let settings_file = quota_station_core::claude::app_dir(&home).join("settings.json");
        let mut inner = Inner::new(home, settings_file, crate::machine_name(), hooks);
        inner.app_version = env!("CARGO_PKG_VERSION").to_string();
        Local { state: Shared::new(inner) }
    }

    /// The home folder of the user the app runs as.
    pub fn home(&self) -> PathBuf {
        self.state.lock().home.clone()
    }

    /// The app's folder, where its settings file is.
    pub fn app_dir(&self) -> AppDir {
        self.state.lock().app_dir()
    }

    /// The name shown for this machine: the owner's, else the system's.
    pub fn machine(&self) -> String {
        self.state.lock().machine()
    }

    /// The app's version.
    pub fn app_version(&self) -> String {
        self.state.lock().app_version.clone()
    }

    /// The Codex folder the app reads and edits.
    pub fn codex_home(&self) -> PathBuf {
        self.state.lock().codex_home()
    }

    /// Where each agent keeps its MCP list.
    pub fn mcp_places(&self) -> Places {
        self.state.lock().mcp_places()
    }

    /// Where each agent keeps its skills.
    pub fn skill_places(&self) -> SkillPlaces {
        self.state.lock().skill_places()
    }

    /// What a restore uses besides the MCP files.
    pub fn restore_tools(&self) -> restore::Tools {
        self.state.lock().restore_tools()
    }

    /// A pack's sections besides the MCP lists, as the app's backup
    /// gathers them.
    pub fn pack_extras(&self) -> PackExtras {
        crate::skills::pack_extras(&self.state)
    }
}

/// The [`Done`] of `action` on the Claude account in `slot`: the account
/// changed (`result` is `Ok`), or why not, as the app's own actions on one
/// account tell it.
pub fn claude_done(action: &'static str, slot: u32, result: Result<(), ManageError>) -> Done {
    claude::changed(action, slot, result)
}

/// The app's state under its lock, as [`Host::locked`] hands it to the
/// extension: what it reads there, and the settings it may save there.
pub struct Locked<'a> {
    inner: &'a mut Inner,
}

impl<'a> Locked<'a> {
    pub(crate) fn new(inner: &'a mut Inner) -> Locked<'a> {
        Locked { inner }
    }

    /// The home folder of the user the app runs as.
    pub fn home(&self) -> &Path {
        &self.inner.home
    }

    /// The settings as the app holds them now.
    pub fn settings(&self) -> &Settings {
        &self.inner.settings
    }

    /// Saves `settings` to the app's settings file and holds them from now
    /// on. A save that fails changes nothing and returns its error as words.
    pub fn save_settings(&mut self, settings: Settings) -> Result<(), String> {
        self.inner.save_settings(settings)
    }

    /// Holds `report`, a reading of the Claude accounts the extension made
    /// outside the app's reader, as the app's newest, ahead of any read in
    /// flight. The windows see it once the extension publishes it
    /// ([`Host::publish`]).
    pub fn store_claude_report(&mut self, report: Response) {
        self.inner.store_claude(report)
    }
}

/// What an extension keeps in the app ([`Host::keep_state`]): one value of
/// each type, found again by its type. The app holds it and never looks
/// inside.
#[derive(Default)]
pub struct Kept(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);

impl Kept {
    /// Keeps `state`, in place of any value of its type kept before.
    pub fn keep<T: Any + Send + Sync>(&mut self, state: Arc<T>) {
        self.0.insert(TypeId::of::<T>(), state);
    }

    /// The value of type `T` kept, if one is.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.0.get(&TypeId::of::<T>()).cloned().and_then(|s| s.downcast().ok())
    }
}

/// The names in `names` that the app's own commands already answer: the
/// app's command is the one called, so these never reach the extension.
pub fn shadowed(names: &[&'static str]) -> Vec<&'static str> {
    let own = crate::command_names();
    names.iter().copied().filter(|name| own.contains(name)).collect()
}

/// The names in `names` that are the app's own tabs: the app's tab is the
/// one opened, so these never name the extension's ([`Extension::tabs`]).
pub fn shadowed_tabs(names: &[&'static str]) -> Vec<&'static str> {
    names.iter().copied().filter(|name| Tab::own(name).is_some()).collect()
}

/// An extension's [`Extension::command_names`] and
/// [`Extension::command_handler`], written inside its `impl Extension`, from
/// one list of `module::command` paths to `#[tauri::command]` functions,
/// each one optionally under a `#[cfg(...)]` that the calling crate decides.
/// The names are the functions' names, as the windows call them.
///
/// ```text
/// impl quota_station::ext::Extension for Mine {
///     quota_station::extension_commands![
///         tools::first,
///         #[cfg(unix)]
///         tools::second,
///     ];
/// }
/// ```
#[macro_export]
macro_rules! extension_commands {
    ($( $(#[cfg($c:meta)])? $module:ident :: $name:ident ),* $(,)?) => {
        fn command_names(&self) -> ::std::vec::Vec<&'static str> {
            #[allow(unused_mut)]
            let mut names: ::std::vec::Vec<&'static str> = ::std::vec::Vec::new();
            $( $(#[cfg($c)])? names.push(::core::stringify!($name)); )*
            names
        }

        fn command_handler(&self) -> $crate::ext::Handler {
            ::std::boxed::Box::new(::tauri::generate_handler![$( $(#[cfg($c)])? $module::$name ),*])
        }
    };
}
