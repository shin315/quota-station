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
//! - which alerts run ([`Settings::alert_switches`]): [`Hooks::alert_switches`].

use serde_json::{Map, Value};

use crate::alerts::Switches;
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
}

/// No extension: every hook does nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoHooks;

impl Hooks for NoHooks {}

/// The hooks of no extension, for a caller that has none to pass.
pub fn none() -> &'static dyn Hooks {
    &NoHooks
}
