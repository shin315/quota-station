// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Switching the Antigravity account from the app (M4c): what the sheet
//! asks about Antigravity.app before it offers its buttons, the switch
//! itself after the owner confirms, and the six-hour watch that tells the
//! popover when a running Antigravity.app wrote its old login back.
//!
//! The switch writes the login Antigravity.app and agy share, with the same
//! `security` command Cockpit Tools uses, and only when the owner confirms.
//! The windows only ever get account numbers and ids, never a token.

use std::sync::Arc;

use quota_station_core::antigravity::cloud_code::Hosts;
use quota_station_core::antigravity::credential::SharedLogin;
use quota_station_core::antigravity::desktop::{Desktop, Installed};
use quota_station_core::antigravity::index;
use quota_station_core::antigravity::oauth::Endpoints;
use quota_station_core::antigravity::store::Keychain;
use quota_station_core::antigravity::switch::{
    self, CloudCodeProbe, Deps, How, Limits, Outcome, Request,
};
use quota_station_core::antigravity::version::AntigravityApp;
use quota_station_core::antigravity::watch;
use quota_station_core::axi::Response;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner, Model};

/// What the switch parts of the windows show.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchView {
    /// The account number a switch is running for.
    pub busy: Option<usize>,
    /// quota-axi lists this app's accounts, so a row's number is the
    /// account's number here and its row can switch.
    pub listed: bool,
    /// A switch was undone and the owner has not answered yet.
    pub overwritten: Option<OverwriteView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverwriteView {
    /// The account switched to, and the one back in use, by number.
    pub to: usize,
    pub from: usize,
    /// When the reading showed it, RFC 3339.
    pub at: String,
    /// Overwrites of this switch so far.
    pub overwrites: u32,
}

pub fn view(inner: &Inner) -> SwitchView {
    let app = inner.app_dir();
    let index = index::load(&app).unwrap_or_default();
    let number = |id: &str| index.accounts.iter().position(|a| a.id == id).map(|i| i + 1);
    let overwritten = watch::load(&app).and_then(|w| {
        let at = w.alert()?;
        Some(OverwriteView {
            to: number(&w.to)?,
            from: number(&w.from)?,
            at: OffsetDateTime::from_unix_timestamp(at).ok()?.format(&Rfc3339).ok()?,
            overwrites: w.overwrites,
        })
    });
    SwitchView {
        busy: inner.agy_switching,
        listed: inner.agy.as_ref().is_some_and(|r| lists(r, &index)),
        overwritten,
    }
}

/// The report numbers its accounts as the app's own list does.
fn lists(report: &Response, index: &index::AccountIndex) -> bool {
    report.primary("agy").and_then(|row| row.cockpit.as_ref()).is_some_and(|c| {
        c.status == "ok"
            && c.source.as_deref() == Some("quota-station")
            && c.accounts.len() == index.accounts.len()
    })
}

/// Feeds the newest Antigravity reading to the six-hour watch.
pub fn observe(inner: &Inner) {
    let Some(report) = inner.agy.as_ref() else { return };
    let app = inner.app_dir();
    let Ok(index) = index::load(&app) else { return };
    if !lists(report, &index) {
        return;
    }
    let active = switch::shown_active_id(report, &index);
    watch::observe(&app, active.as_deref(), unix_now());
}

fn unix_now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// Antigravity.app as the sheet needs it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    /// `missing`, `supported`, `tooOld` or `unreadable` (there, but its
    /// version could not be read: the sheet does not offer a switch).
    pub installed: &'static str,
    pub version: Option<String>,
    /// `None` when the process list could not be read.
    pub running: Option<bool>,
}

/// Is Antigravity.app installed, which version, does it run. Asked when
/// the switch sheet opens; it only runs `plutil` and `pgrep`. Rejects when
/// it could not ask, and the sheet then says it does not know.
#[tauri::command]
pub async fn agy_app_state() -> Result<AppView, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let desktop = Desktop::system();
        let (installed, version) = match desktop.installed() {
            Installed::Missing => ("missing", None),
            Installed::Supported { version } => ("supported", Some(version)),
            Installed::TooOld { version } => ("tooOld", Some(version)),
            Installed::Unreadable { .. } => ("unreadable", None),
        };
        let running = if installed == "missing" { Some(false) } else { desktop.running().ok() };
        AppView { installed, version: version.filter(|v| !v.is_empty()), running }
    })
    .await
    .map_err(|e| e.to_string())
}

impl Inner {
    /// Why an Antigravity switch cannot start now: `busy` while a switch
    /// runs, `loggingOut` while a logout runs, `loggingIn` while a login
    /// runs, which could otherwise change the account list and the Keychain
    /// items under the switch.
    pub fn agy_switch_refusal(&self) -> Option<&'static str> {
        if self.agy_switching.is_some() {
            Some("busy")
        } else if self.agy_logging_out {
            Some("loggingOut")
        } else if self.agy_logging_in {
            Some("loggingIn")
        } else {
            None
        }
    }
}

/// Switches Antigravity to the account `id` after the owner confirmed it on
/// the sheet, the way `how` says. Rejects with `busy` while a switch runs,
/// `loggingOut` while a logout runs, `loggingIn` while a login runs, and
/// `unknownAccount` for an id this app does not list.
#[tauri::command]
pub async fn switch_agy(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    how: How,
) -> Result<Outcome, String> {
    switch(&app, state.inner().clone(), id, how).await
}

/// One switch, from the sheet or from an alert card's "Chuyển lại".
pub async fn switch(
    app: &AppHandle,
    shared: AppState,
    id: String,
    how: How,
) -> Result<Outcome, String> {
    let (dir, quota_axi, from) = {
        let mut inner = shared.lock();
        if let Some(why) = inner.agy_switch_refusal() {
            return Err(why.into());
        }
        let dir = inner.app_dir();
        let index = index::load(&dir).map_err(|_| "index")?;
        let number = index.accounts.iter().position(|a| a.id == id).ok_or("unknownAccount")? + 1;
        let from = inner.agy.as_ref().and_then(|r| switch::shown_active_id(r, &index));
        inner.agy_switching = Some(number);
        inner.agy_generation += 1;
        (dir, inner.quota_axi(), from)
    };
    reader::publish(app, &shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        let endpoints = Endpoints::google();
        let keychain = Keychain::system();
        let shared_login = SharedLogin::system();
        let desktop = Desktop::system();
        let deps = Deps {
            app: &dir,
            endpoints: &endpoints,
            keychain: &keychain,
            probe: Arc::new(CloudCodeProbe {
                hosts: Hosts::google(),
                identity: AntigravityApp::installed().identity(),
            }),
            shared: &shared_login,
            desktop: &desktop,
            quota_axi: quota_axi.as_deref(),
        };
        let request = Request { id, how, from };
        switch::switch_antigravity(&deps, &request, Limits::default(), &unix_now)
    })
    .await;
    {
        let mut inner = shared.lock();
        inner.agy_switching = None;
        // Reads that started before now may hold the account from before.
        inner.agy_generation += 1;
        if let Ok(Some(report)) = done.as_ref().map(|d| d.report.clone()) {
            inner.agy = Some(report);
            inner.agy_error = None;
        }
    }
    // The switch log has its line: the change it made is not seen as one of its own.
    crate::alerting::track_history(&shared);
    reader::publish(app, &shared);
    done.map(|d| d.outcome).map_err(|e| e.to_string())
}

/// "Thôi" on the overwrite banner: stops the six-hour watch.
#[tauri::command]
pub fn agy_dismiss_overwrite(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    let dir = state.lock().app_dir();
    watch::dismiss(&dir).map_err(|e| e.to_string())?;
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn inner() -> Inner {
        Inner::new(PathBuf::new(), PathBuf::from("/nonexistent/settings.json"), "Mac".into())
    }

    #[test]
    fn a_switch_is_refused_while_a_login_a_logout_or_another_switch_runs() {
        let mut inner = inner();
        assert_eq!(inner.agy_switch_refusal(), None);

        inner.agy_logging_in = true;
        assert_eq!(inner.agy_switch_refusal(), Some("loggingIn"));
        inner.agy_logging_in = false;
        assert_eq!(inner.agy_switch_refusal(), None, "it can start once the login ends");

        inner.agy_logging_out = true;
        assert_eq!(inner.agy_switch_refusal(), Some("loggingOut"));
        inner.agy_logging_out = false;

        inner.agy_switching = Some(2);
        assert_eq!(inner.agy_switch_refusal(), Some("busy"));
    }
}
