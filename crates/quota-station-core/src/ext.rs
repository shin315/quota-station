// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's side of an extension: the hooks the core calls where an
//! extension may add to what it reads, keeps and shows. Each hook is a
//! method of [`Hooks`] whose default does nothing, so [`NoHooks`] (and any
//! extension that leaves a hook alone) leaves the core exactly as it is
//! without one.
//!
//! The core never finds an extension on its own: a caller that has one
//! passes `&dyn Hooks` in, as a parameter or as a field of the input it
//! already takes, and the core's own callers and tests pass [`NoHooks`].
//! The app's extension (`quota_station::ext::Extension`) hands its hooks
//! over with `Extension::hooks`.
//!
//! Today's hooks:
//!
//! - the settings file ([`Settings::open`]): the extension's keys, kept in
//!   [`Settings::ext`], are read by [`Hooks::tidy_settings`] and settled
//!   for a Mac new to the app by [`Hooks::settle_settings`], and
//!   [`Hooks::settled_key`] names the key that tells a file was settled;
//! - which alerts run ([`Settings::alert_switches`]): [`Hooks::alert_switches`];
//! - the alerts ([`crate::alerts`]): the kinds the extension adds to the
//!   app's, with their switches in the settings file and the words of their
//!   buttons ([`Hooks::alert_kinds`]), and the card "Gửi thử" sends for one
//!   ([`Hooks::alert_sample`]); their alerts at each pass of the rules come
//!   from what the extension read for it ([`crate::alerts::Facts::ext`]);
//! - `quota-station claude …` ([`crate::claude::cli::run`]): the commands
//!   the extension adds to the app's ([`Hooks::claude_command`]) and the
//!   usage that names them ([`Hooks::claude_usage`]).

use std::io::Write;

use serde_json::{Map, Value};
use time::UtcOffset;

use crate::alerts::{Alert, ExtKind, Switches};
use crate::claude::cli::Cli;
use crate::settings::Settings;

/// The hooks an extension gives the core. Every method has a default that
/// does nothing; an extension overrides the ones it needs.
pub trait Hooks: Send + Sync {
    /// The settings key whose value, `true` or `false`, tells that
    /// [`Settings::open`] already settled whether this Mac is new to the
    /// app. `None`, the default, is the app's own (`setupSeen`). An
    /// extension that names its own key has its keys settled
    /// ([`Hooks::settle_settings`]) the first time a build with it opens a
    /// file without that key.
    fn settled_key(&self) -> Option<&'static str> {
        None
    }

    /// The extension's keys of the settings file as [`Settings::open`]
    /// read them, put in the shape the extension keeps them in. `found` is
    /// false when there was no file, or none the app could read, and
    /// `ext` is empty. Called on every open, before anything else reads
    /// them. The default leaves them as they were read.
    fn tidy_settings(&self, ext: &mut Map<String, Value>, found: bool) {
        let _ = (ext, found);
    }

    /// The extension's keys on the first open that settles the file:
    /// `had_app` tells a Mac that had the app before from a new one. What
    /// is set here is written with the file at once. The default sets
    /// nothing.
    fn settle_settings(&self, ext: &mut Map<String, Value>, had_app: bool) {
        let _ = (ext, had_app);
    }

    /// Which of the alerts switched on in `settings` run: `on` is the
    /// app's answer (the switches themselves). The default keeps it.
    fn alert_switches(&self, settings: &Settings, on: Switches) -> Switches {
        let _ = settings;
        on
    }

    /// The kinds of alert the extension adds to the app's own: each one's
    /// number, name, switch and the words of its buttons. Their switches
    /// are kept in the settings file's `alerts` beside the app's, and their
    /// alerts, which the extension draws at each pass of the rules
    /// ([`crate::alerts::ExtSource`]), go out as the app's own do. The
    /// default adds none.
    fn alert_kinds(&self) -> &'static [ExtKind] {
        &[]
    }

    /// The card "Gửi thử" sends for the extension's kind `name`, at `now`:
    /// its tone, title, body and buttons, with `claude` the names the cards
    /// give three Claude accounts. Its key, kind and time are the app's to
    /// set, and so is the mark that makes its buttons change nothing.
    /// `None`, the default, sends nothing.
    fn alert_sample(
        &self,
        name: &str,
        now: i64,
        offset: UtcOffset,
        claude: [&str; 3],
    ) -> Option<Alert> {
        let _ = (name, now, offset, claude);
        None
    }

    /// A command of `quota-station claude …` that is not one of the app's:
    /// `args` are the words after `claude`, the command first. The
    /// extension answers with the exit code, having written what it prints
    /// to `out` and `err`, or `None`, the default, when the command is not
    /// its own either: then the app prints the usage on `err` and exits 2.
    /// The app's own commands never reach the extension.
    fn claude_command(
        &self,
        cli: &Cli,
        args: &[String],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Option<i32> {
        let _ = (cli, args, out, err);
        None
    }

    /// The usage `quota-station claude` prints (for `help`, and on stderr
    /// for a command or an argument it does not know), when the
    /// extension's commands belong in it. `None`, the default, is the
    /// app's own ([`crate::claude::cli::USAGE`]).
    fn claude_usage(&self) -> Option<&'static str> {
        None
    }
}

/// No extension: every hook does nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoHooks;

impl Hooks for NoHooks {}

/// The hooks of no extension, for a caller that has none to pass.
pub fn none() -> &'static dyn Hooks {
    &NoHooks
}
