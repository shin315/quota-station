// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude accounts the app keeps (see `docs/claude-accounts.md`): what
//! the Claude tab shows of them, and its actions: import from claude-swap,
//! log a new account in, add the login Claude Code is on or a setup token,
//! remove an account and give it an alias. An extension runs its own
//! actions on the accounts the same way ([`crate::ext::Host::claude_action`]).
//!
//! The windows only ever get account numbers, aliases and readings; an
//! email only while the owner shows names, and never a token.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use quota_station_core::claude::import::{self, Imported};
use quota_station_core::claude::manage::{self, Added, ManageError};
use quota_station_core::claude::state::{self, AccountOut};
use quota_station_core::claude::store::Store;
use quota_station_core::claude::switch;
use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::ext::{Host, View};
use crate::reader;
use crate::state::{AppState, Inner, Model};

/// What the Claude tab's account actions hold between reads.
#[derive(Debug, Default)]
pub struct Live {
    /// The action running now: `import`, `login`, `current`, `token`,
    /// `remove` or `alias`, or an extension's, by the name it gives.
    pub working: Option<&'static str>,
    /// Stops a `claude auth login` that waits for the browser.
    pub cancel: Option<Arc<AtomicBool>>,
    /// The last action's result, until the next one.
    pub last: Option<Done>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub action: &'static str,
    pub ok: bool,
    /// The error's kind, for the words.
    pub error: Option<String>,
    pub detail: Option<String>,
    /// The account added or changed.
    pub slot: Option<u32>,
    /// An add renewed an account the app had.
    pub renewed: bool,
    pub imported: Option<Imported>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsView {
    /// The app keeps the accounts on this Mac: imported or added.
    pub ready: bool,
    /// claude-swap's folder is on this Mac, to import from.
    pub importable: bool,
    pub accounts: Vec<AccountOut>,
    pub working: Option<&'static str>,
    pub last: Option<Done>,
    /// The extension's keys in the view ([`crate::ext::Extension::view_keys`]).
    #[serde(flatten)]
    pub ext: Map<String, Value>,
    /// `claude` is on this Mac, to log a new account in with.
    pub claude: bool,
}

pub fn view(inner: &Inner) -> AccountsView {
    let paths = inner.claude_paths();
    let mut accounts = inner.claude_accounts.clone();
    if inner.settings.hide_names {
        for a in &mut accounts {
            a.email = None;
        }
    }
    AccountsView {
        ready: paths.in_use(),
        importable: import::available(&inner.home),
        accounts,
        working: inner.claude_live.working,
        last: inner.claude_live.last.clone(),
        ext: inner.view_keys(View::ClaudeAccounts),
        claude: inner.claude_program.is_some(),
    }
}

/// The accounts' rows and details from disk, with emails while names show.
/// Reads the Keychain for the emails: off the main thread.
pub fn load(
    store: &Store,
    names: bool,
) -> (Vec<AccountOut>, Vec<quota_station_core::detail::ClaudeDetail>) {
    let details = state::details(store, names);
    let now = OffsetDateTime::now_utc();
    let rows = match store.load() {
        Ok(list) => {
            let cache = quota_station_core::claude::usage::load(store);
            let active = switch::active_slot(store);
            let mut rows = state::build(&list, &cache, active, store.account_keys(), now).accounts;
            for row in &mut rows {
                row.email =
                    details.iter().find(|d| d.number == row.slot).and_then(|d| d.email.clone());
            }
            rows
        }
        Err(_) => Vec::new(),
    };
    (rows, details)
}

/// Starts `action`, or says another is running.
fn start(state: &AppState, action: &'static str) -> Result<(Store, bool), String> {
    let mut inner = state.lock();
    if inner.claude_live.working.is_some() {
        return Err("busy".into());
    }
    inner.claude_live.working = Some(action);
    inner.claude_live.last = None;
    Ok((inner.claude_store(), !inner.settings.hide_names))
}

/// Ends the running action with `done`, takes the accounts in again, and
/// has everything read again.
fn finish(app: &AppHandle, state: &AppState, store: &Store, names: bool, done: Done) -> Model {
    let _ = state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
    let (rows, details) = load(store, names);
    {
        let mut inner = state.lock();
        inner.claude_live.working = None;
        inner.claude_live.cancel = None;
        inner.claude_live.last = Some(done);
        inner.claude_accounts = rows;
        inner.claude_list = Some(details);
    }
    state.request_read();
    if state.lock().after_accounts_change() {
        Host::of(app).wake_reader();
    }
    reader::publish(app, state);
    state.lock().model()
}

fn failed(action: &'static str, e: &ManageError) -> Done {
    Done {
        action,
        ok: false,
        error: Some(e.kind().to_string()),
        detail: match e {
            ManageError::Login(why) => Some(why.clone()),
            ManageError::Keychain(k) => Some(k.to_string()),
            _ => None,
        },
        slot: None,
        renewed: false,
        imported: None,
    }
}

fn added(action: &'static str, result: Result<Added, ManageError>) -> Done {
    match result {
        Ok(a) => Done {
            action,
            ok: true,
            error: None,
            detail: None,
            slot: Some(a.slot),
            renewed: !a.new,
            imported: None,
        },
        Err(e) => failed(action, &e),
    }
}

/// The result of `action` on the account in `slot`.
pub(crate) fn changed(action: &'static str, slot: u32, result: Result<(), ManageError>) -> Done {
    match result {
        Ok(()) => Done {
            action,
            ok: true,
            error: None,
            detail: None,
            slot: Some(slot),
            renewed: false,
            imported: None,
        },
        Err(e) => failed(action, &e),
    }
}

/// Runs `work` off the main thread as `action`, then finishes it.
pub(crate) async fn run(
    app: AppHandle,
    state: AppState,
    action: &'static str,
    work: impl FnOnce(&Store) -> Done + Send + 'static,
) -> Result<Model, String> {
    let (store, names) = start(&state, action)?;
    reader::publish(&app, &state);
    let done = {
        let store = store.clone();
        tauri::async_runtime::spawn_blocking(move || work(&store)).await
    };
    let done = done.unwrap_or_else(|e| Done {
        action,
        ok: false,
        error: Some("crash".into()),
        detail: Some(e.to_string()),
        slot: None,
        renewed: false,
        imported: None,
    });
    Ok(finish(&app, &state, &store, names, done))
}

/// Carries claude-swap's accounts over. claude-swap is left as it is.
#[tauri::command]
pub async fn claude_import(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    let home = state.lock().home.clone();
    run(app, state.inner().clone(), "import", move |store| {
        match import::import(store, &import::source(&home)) {
            Ok(imported) => Done {
                action: "import",
                ok: true,
                error: None,
                detail: None,
                slot: None,
                renewed: false,
                imported: Some(imported),
            },
            Err(e) => Done {
                action: "import",
                ok: false,
                error: Some(
                    match e {
                        import::ImportError::NotFound => "notFound",
                        import::ImportError::Corrupt => "corrupt",
                        import::ImportError::Lock(_) => "busy",
                        import::ImportError::Store(_) => "store",
                    }
                    .into(),
                ),
                detail: None,
                slot: None,
                renewed: false,
                imported: None,
            },
        }
    })
    .await
}

/// Logs a new account in with `claude auth login` in a folder of its own:
/// the browser opens, and Claude Code's own login is not touched.
#[tauri::command]
pub async fn claude_add_login(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    let shared = state.inner().clone();
    let Some(claude) = shared.lock().claude_program.clone() else {
        return Err("noClaude".into());
    };
    let holder = shared.clone();
    run(app, shared, "login", move |store| {
        let cancel = Arc::new(AtomicBool::new(false));
        holder.lock().claude_live.cancel = Some(cancel.clone());
        added("login", manage::login(store, &claude, false, &cancel))
    })
    .await
}

/// Stops a login that waits for the browser.
#[tauri::command]
pub fn claude_cancel_login(state: State<'_, AppState>) {
    if let Some(c) = &state.lock().claude_live.cancel {
        c.store(true, Ordering::Relaxed);
    }
}

/// Adds the login Claude Code is on now.
#[tauri::command]
pub async fn claude_add_current(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Model, String> {
    run(app, state.inner().clone(), "current", |store| added("current", manage::adopt_live(store)))
        .await
}

/// Adds a token from `claude setup-token`, with the account's email when
/// the owner gives it.
#[tauri::command]
pub async fn claude_add_token(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    email: Option<String>,
) -> Result<Model, String> {
    run(app, state.inner().clone(), "token", move |store| {
        added("token", manage::add_token(store, &token, email.as_deref()))
    })
    .await
}

#[tauri::command]
pub async fn claude_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    slot: u32,
) -> Result<Model, String> {
    run(app, state.inner().clone(), "remove", move |store| {
        changed("remove", slot, manage::remove(store, slot))
    })
    .await
}

#[tauri::command]
pub async fn claude_set_alias(
    app: AppHandle,
    state: State<'_, AppState>,
    slot: u32,
    alias: String,
) -> Result<Model, String> {
    run(app, state.inner().clone(), "alias", move |store| {
        changed("alias", slot, manage::set_alias(store, slot, &alias))
    })
    .await
}
