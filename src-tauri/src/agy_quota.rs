// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The Antigravity quota thread (M4b): reads the quota of the accounts this
//! app logs in, one at a time and never more often than Cockpit Tools reads
//! them (`quota_station_core::antigravity::pace`), and leaves each reading
//! in the app's cache, which the Antigravity row lists
//! (`quota_station_core::antigravity::listing`); this thread also tells the
//! windows which account it is reading and which read failed.

use std::thread;
use std::time::Duration;

use quota_station_core::antigravity::cloud_code::Hosts;
use quota_station_core::antigravity::oauth::Endpoints;
use quota_station_core::antigravity::quota::{self, Outcome};
use quota_station_core::antigravity::store::Keychain;
use quota_station_core::antigravity::version::AntigravityApp;
use quota_station_core::antigravity::{listing, login};
use tauri::AppHandle;
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, ReadState};

/// The longest wait between two looks at the pace, for the 10-minute reads.
const WAIT_STEP: Duration = Duration::from_secs(30);

pub fn spawn(app: AppHandle, state: AppState) {
    thread::Builder::new()
        .name("agy-quota".into())
        .spawn(move || run(&app, &state))
        .expect("Antigravity quota thread starts");
}

fn now_ms() -> i64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

fn run(app: &AppHandle, state: &AppState) {
    loop {
        {
            let inner = state.lock();
            if !inner.agy_pace.pending() {
                let _ = state.agy_wake.wait_timeout(inner, WAIT_STEP);
            }
        }
        round(app, state);
    }
}

/// Reads the accounts that are due, in turn.
fn round(app: &AppHandle, state: &AppState) {
    let (dir, number) = {
        let inner = state.lock();
        (inner.app_dir(), inner.agy_active_number())
    };
    let Ok((index, candidates)) = quota::candidates(&dir) else {
        // An unreadable index has nothing to read; the card says why.
        state.lock().agy_pace.take(&[], None, now_ms());
        return;
    };
    let active = quota::active_id(&index, number);
    let due = state.lock().agy_pace.take(&candidates, active.as_deref(), now_ms());
    if due.is_empty() {
        return;
    }

    let identity = AntigravityApp::installed().identity();
    let (endpoints, hosts, keychain) = (Endpoints::google(), Hosts::google(), Keychain::system());
    let login = login::Deps { app: &dir, endpoints: &endpoints, keychain: &keychain };
    let deps = quota::Deps { login: &login, hosts: &hosts, identity: &identity };
    let mut new_reading = false;
    // A logout waits for these reads, and a read after it finds no account.
    let no_logout = state.agy_accounts.lock().unwrap_or_else(|e| e.into_inner());
    quota::read_in_turn(&deps, &due, &now_ms, &mut |id, result| {
        let now = now_ms();
        {
            let mut inner = state.lock();
            match result {
                None => {
                    inner.agy_pace.started(id, now);
                    inner.agy_reads.insert(id.to_string(), ReadState::Reading);
                }
                Some(result) => {
                    inner.agy_pace.finished(id, quota::finish(result), now);
                    match result {
                        // Needs a new login, refused: the account's status says it.
                        Err(e) if !e.in_status() => {
                            let failed =
                                ReadState::Failed { kind: e.kind(), detail: e.to_string() };
                            inner.agy_reads.insert(id.to_string(), failed);
                        }
                        _ => {
                            inner.agy_reads.remove(id);
                        }
                    }
                    new_reading |= matches!(result, Ok(Outcome::Read));
                }
            }
        }
        reader::publish(app, state);
    });
    drop(no_logout);

    if new_reading {
        show(app, state);
    }
}

/// Shows new readings at once: the app's list is built again from its
/// files on the last Antigravity reading. Nothing is asked of a vendor; the
/// account agy is on is read at the next timed read.
fn show(app: &AppHandle, state: &AppState) {
    {
        let mut inner = state.lock();
        let dir = inner.app_dir();
        let Some(row) = inner.agy.as_mut().and_then(|r| r.primary_mut("agy")) else { return };
        listing::attach(row, &dir, OffsetDateTime::now_utc());
    }
    reader::publish(app, state);
}
