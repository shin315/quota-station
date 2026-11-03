// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part as the app's extension ([`crate::ext::Extension`]):
//! the app's binary (`main.rs`) passes [`Advanced`] to [`crate::run`], and it
//! fills the sockets the advanced part needs.
//!
//! Filled so far: the commands of the advanced part (the Nâng cao switch,
//! the pause, the rotation and its rules, the quit guard for unsaved rules
//! and, on a Mac only, the rules' copy to the other Macs, the copy to
//! another Mac and the update from source; those of the switch, the pause
//! and the quit guard are here and
//! reach the app only through the host, [`commands`], and so is the switch
//! that turns the rotation on or off, run through the host as an action of
//! the Claude tab on the accounts, [`claude`], and the update from source,
//! a Mac's only, with its keys in the app's own row, [`update`]), the core's
//! hooks for its settings ([`hooks`]), its own state in the running app
//! ([`state`]), made when the app starts and kept through the host, and
//! what it runs beside the app ([`start`]): its threads and its look at the
//! switcher at each pass of the app's reader and after an action of the
//! Claude tab on the accounts, its views in the model every window draws
//! and its keys in the Claude accounts' view ([`model`]), its tabs in the
//! main window ([`tabs`]), its hold on the main window's close and on
//! `Thoát` while the rules have changes not saved ([`close`]), its subcommands from a terminal
//! ([`subcommand`]: `quota-station agents …`; its commands of
//! `quota-station claude …` are among its hooks for the core), and its
//! alert (2, an account leaving the rotation), whose kind is among its
//! hooks for the core and whose facts it reads after each read
//! ([`alerts`]), and the rotation as the
//! policy of the Claude pool, among its hooks for the core; and after the
//! owner's manual switch, the pause flag read again and a look at the
//! switcher asked of the reader ([`start`]; what it does around the switch
//! is among its hooks for the core), and the menu bar icon's badge while
//! the pause holds ([`icon`]), and the accounts held out of the rotation
//! (their keys on the Claude accounts among its hooks for the core, and
//! the command that holds one out or puts it back, run through the host as
//! an action of the Claude tab on the accounts, [`held`]), and its part of
//! the app's diagnostics: Nâng cao on the system line, the rotation's
//! self-check and the other Macs hidden in the bundle ([`diagnostics`]).

pub mod alerts;
pub mod claude;
pub mod close;
pub mod commands;
pub mod diagnostics;
pub mod held;
pub mod hooks;
pub mod icon;
pub mod model;
pub mod start;
pub mod state;
pub mod subcommand;
pub mod tabs;
#[cfg(unix)]
pub mod update;

use std::path::Path;
use std::sync::Arc;

use quota_station_core::app_dir::AppDir;
use quota_station_core::icon::Badge;
use quota_station_core::settings::Settings;
use serde_json::{Map, Value};
use time::OffsetDateTime;

#[cfg(unix)]
use crate::copy;
use crate::ext::{
    AlertInput, AlertRead, Closing, Extension, Hold, Hooks, Host, Kept, ModelInput, ReaderPass,
    ReaderWork, View,
};
use crate::rules;

/// The advanced part, as the extension the binary passes to [`crate::run`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Advanced;

impl Extension for Advanced {
    fn hooks(&self) -> &dyn Hooks {
        &hooks::AdvancedHooks
    }

    fn start(&self, host: Host) {
        let state = state::AdvancedState::new(&host.app_dir());
        host.keep_state(Arc::new(state));
    }

    fn start_threads(&self, host: Host) {
        start::threads(host);
    }

    fn reader_asked(&self, kept: &Kept) -> bool {
        start::asked(kept)
    }

    fn reader_pass(&self, pass: &ReaderPass<'_>) -> Option<ReaderWork> {
        Some(start::pass(pass))
    }

    fn model(&self, input: &ModelInput<'_>) -> Map<String, Value> {
        model::model(input)
    }

    fn capabilities(&self) -> Map<String, Value> {
        model::capabilities()
    }

    fn view_keys(&self, view: View, input: &ModelInput<'_>) -> Map<String, Value> {
        model::view_keys(view, input)
    }

    fn tabs(&self) -> &[&'static str] {
        tabs::TABS
    }

    fn alert_read(&self, input: &AlertInput<'_>) -> Option<Box<dyn AlertRead>> {
        Some(Box::new(alerts::read(input)))
    }

    fn hold_close(&self, kept: &Kept, closing: Closing) -> Hold {
        close::hold(kept, closing)
    }

    fn after_switch(&self, kept: &Kept, home: &Path) -> bool {
        start::after_switch(kept, home)
    }

    fn after_accounts_change(&self, kept: &Kept) -> bool {
        start::after_accounts_change(kept)
    }

    fn icon_badge(&self, kept: &Kept, now: OffsetDateTime) -> Option<Badge> {
        icon::badge(kept, now)
    }

    fn subcommand(&self, args: &[String], hooks: &'static dyn Hooks) -> Option<i32> {
        subcommand::answer(args, hooks)
    }

    fn diagnostics(&self, settings: &Settings) -> Vec<String> {
        diagnostics::words(settings)
    }

    fn self_check(&self, id: &str, app_dir: &AppDir) -> Option<Value> {
        diagnostics::self_check(id, app_dir)
    }

    fn hidden_keys(&self) -> &[&'static str] {
        diagnostics::HIDDEN
    }

    crate::extension_commands![
        commands::set_advanced,
        commands::pause_switching,
        commands::extend_pause,
        commands::resume_switching,
        held::claude_set_enabled,
        claude::claude_set_rotation,
        rules::save_rotation,
        #[cfg(unix)]
        rules::adopt_rotation,
        #[cfg(unix)]
        rules::keep_rotation,
        #[cfg(unix)]
        rules::retry_rotation,
        #[cfg(unix)]
        rules::set_peers,
        #[cfg(unix)]
        rules::test_peer,
        rules::preview_rotation,
        commands::rules_dirty,
        commands::take_quit,
        commands::close_main,
        #[cfg(unix)]
        copy::copy_compare,
        #[cfg(unix)]
        copy::copy_run,
        #[cfg(unix)]
        copy::copy_undo,
        #[cfg(unix)]
        update::set_source_dir,
        #[cfg(unix)]
        update::update_from_source,
        #[cfg(unix)]
        update::relaunch,
    ];
}

#[cfg(test)]
mod tests {
    use super::Advanced;
    use crate::ext::{shadowed, Extension};

    #[test]
    fn the_private_commands_shadow_no_app_command() {
        assert_eq!(shadowed(&Advanced.command_names()), Vec::<&str>::new());
    }

    #[test]
    fn the_private_commands_are_named_once() {
        let listed = Advanced.command_names();
        let mut names = listed.clone();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), listed.len());
    }
}
