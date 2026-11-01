// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's sockets for an extension: what a build may add to the app
//! without the app's own code knowing what it adds. [`crate::run`] takes
//! one [`Extension`]; this repository's binary passes [`NoExtension`], with
//! which the app is exactly what it is without the sockets.
//!
//! Each socket is a method of [`Extension`] with a default that adds
//! nothing. The core's own sockets are [`Hooks`] (in
//! `quota_station_core::ext`), handed over by [`Extension::hooks`].
//!
//! Today's sockets:
//!
//! - [`Extension::hooks`]: the core's hooks. The app keeps them for as long
//!   as it runs and passes them where the core calls one (the settings file
//!   and which alerts run, today).
//! - [`Extension::command_names`] and [`Extension::command_handler`]: the
//!   commands the windows can call besides the app's own, both made from one
//!   list by [`extension_commands!`](crate::extension_commands). A command
//!   name the app already has stays the app's ([`shadowed`] lists those).
//! - [`Extension::start`]: once, when the app starts, with the [`Host`], the
//!   extension's way to the app: the settings, a wake for the app's reader,
//!   and a place for the extension's own state ([`Host::keep_state`]), which
//!   its commands take as Tauri state.
//!
//! The names and the handler are apart so a test can read the names without
//! building the handler: a handler holds Tauri's window runtime, which a
//! Windows test program cannot load (it lacks the app's manifest, which
//! only the binary gets). The [`Host`] holds that runtime too, so no test
//! makes one; what the extension keeps is plain data ([`Kept`]).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use quota_station_core::app_dir::AppDir;
use quota_station_core::settings::Settings;
use tauri::{AppHandle, Manager};

pub use quota_station_core::ext::{Hooks, NoHooks};

use crate::state::AppState;

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
}

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

    /// Wakes the app's reader, so what the extension asked of it is seen at
    /// once. Taken under the app's lock: a reader about to wait cannot miss it.
    pub fn wake_reader(&self) {
        let _held = self.state.lock();
        self.state.wake.notify_all();
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
