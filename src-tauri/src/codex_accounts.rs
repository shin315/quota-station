// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Several Codex accounts (M8): the list the windows draw (the ChatGPT
//! sign-ins the app keeps, each in its own folder, then the provider tables
//! of `config.toml`) and its commands: use one, sign one in with `codex
//! login` in a folder of its own, add, rename or remove one.
//!
//! The windows never get a token or a key. A sign-in is only moved, never
//! read here beyond who it is ([`quota_station_core::codex::accounts`]); a
//! key typed for a new provider goes one way, from the window to
//! `config.toml`, and comes back only as its hint.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::accounts::{self, Added, Place, Store, CHATGPT_ID};
use quota_station_core::codex::config::{self, Custom, KeyRefusal, Parsed, Refusal, Target};
use quota_station_core::codex::files::WriteError;
use quota_station_core::codex::login::Login;
use quota_station_core::codex::platform::Unix;
use quota_station_core::codex::probe::{self, KeyCheck};
use quota_station_core::codex::{self, procs, Outcome, ProviderOutcome, ProviderPreview};
use quota_station_core::quota::cache::Cache;
use quota_station_core::quota::{self, ProviderId};
use serde::Serialize;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::codex::{list, rfc, write, Reach};
use crate::reader;
use crate::state::{AppState, Inner};

/// How long `codex login` waits for the browser.
const LOGIN_WAIT: Duration = Duration::from_secs(10 * 60);
/// How long `codex logout` may take before the removal goes on without it.
const LOGOUT_WAIT: Duration = Duration::from_secs(30);
/// What `codex login` printed, kept in its folder until it ends.
const LOGIN_LOG: &str = "login.log";

/// One account of the list. Never a token or a key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    /// `chatgpt:<n>` for a sign-in, the table's id for a provider.
    pub id: String,
    /// `chatgpt` or `custom`.
    pub kind: &'static str,
    /// The nickname, else the email's name or `ChatGPT <n>`, or the table's `name`.
    pub name: String,
    pub nickname: bool,
    pub number: Option<u32>,
    /// Only while the owner shows names.
    pub email: Option<String>,
    pub plan: Option<String>,
    /// Codex sends its requests through it now.
    pub active: bool,
    /// Its sign-in is the one in the Codex folder.
    pub signed_in: bool,
    /// `ok`, `relogin` (ChatGPT refused it, or it is gone) or `idle` (its
    /// token ran out while it waited in its folder).
    pub login: &'static str,
    pub custom: Option<Custom>,
    pub reach: Option<Reach>,
    /// Why "Chuyển" to it cannot run.
    pub refusal: Option<Refusal>,
    pub added_at: Option<String>,
}

/// `plus` → `Plus`.
fn plan_name(plan: &str) -> String {
    let mut c = plan.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// How a kept account's last quota read went.
fn login_of(inner: &Inner, number: u32, place: Place) -> &'static str {
    if place == Place::Missing {
        return "relogin";
    }
    let key = format!("{}{number}", quota::codex::KEPT_KEY);
    let row = inner.codex_quota.as_ref().and_then(|r| {
        r.providers.iter().find(|p| p.provider == "codex" && p.account_keys.contains(&key))
    });
    match row {
        Some(r) if r.state.status == "auth_required" => "relogin",
        Some(r) if r.state.error.as_deref() == Some(quota::codex::TOKEN_EXPIRED) => "idle",
        _ => "ok",
    }
}

/// The accounts the Codex tab, the popover and Tổng quan list.
pub fn view(
    inner: &Inner,
    parsed: Option<&Parsed>,
    login: Login,
    current: &str,
) -> Vec<AccountView> {
    let live = &inner.codex;
    let names = !inner.settings.hide_names;
    let mut out: Vec<AccountView> = live
        .accounts
        .iter()
        .map(|a| {
            let e = &a.entry;
            let own = match (&e.email, names) {
                (Some(email), true) => email.split('@').next().unwrap_or(email).to_string(),
                _ => format!("ChatGPT {}", e.number),
            };
            AccountView {
                id: format!("{CHATGPT_ID}{}", e.number),
                kind: "chatgpt",
                name: e.nickname.clone().unwrap_or(own),
                nickname: e.nickname.is_some(),
                number: Some(e.number),
                email: e.email.clone().filter(|_| names),
                plan: e.plan.as_deref().map(plan_name),
                active: a.place == Place::Home && current == "chatgpt",
                signed_in: a.place == Place::Home,
                login: login_of(inner, e.number, a.place),
                custom: None,
                reach: None,
                refusal: None,
                added_at: e.added_at.and_then(rfc),
            }
        })
        .collect();
    if out.is_empty() && matches!(login, Login::ChatGpt | Login::Keychain) {
        // Codex's own sign-in, before the app lists it (or kept in the
        // Keychain, where the app does not look).
        out.push(AccountView {
            id: format!("{CHATGPT_ID}0"),
            kind: "chatgpt",
            name: "ChatGPT".into(),
            nickname: false,
            number: Some(0),
            email: None,
            plan: None,
            active: current == "chatgpt",
            signed_in: true,
            login: "ok",
            custom: None,
            reach: None,
            refusal: None,
            added_at: None,
        });
    }
    let plugins = inner.settings.codex.plugins;
    for c in parsed.map(|p| p.providers.as_slice()).unwrap_or_default() {
        let refusal = parsed.and_then(|p| {
            let mut with = p.clone();
            with.custom = Some(c.clone());
            config::plan(&with, Target::Custom, login.on_disk(), plugins).err()
        });
        let nickname = live.nicknames.get(&c.id);
        out.push(AccountView {
            id: c.id.clone(),
            kind: "custom",
            name: nickname.cloned().unwrap_or_else(|| c.name.clone()),
            nickname: nickname.is_some(),
            number: None,
            email: None,
            plan: None,
            active: parsed.is_some_and(|p| p.selected.as_deref() == Some(c.id.as_str())),
            signed_in: false,
            login: "ok",
            custom: Some(c.clone()),
            reach: live.reaches.get(&c.id).cloned(),
            refusal,
            added_at: None,
        });
    }
    out
}

// ===== Commands =====

/// What an account command did. Holds no token or key.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AccountDone {
    Added {
        id: String,
    },
    /// The sign-in is an account already in the list: nothing added.
    Dup {
        id: String,
    },
    Renewed {
        id: String,
    },
    Renamed {
        id: String,
    },
    Removed {
        id: String,
        name: String,
        logged_out: bool,
    },
    /// `cancelled`, `timeout`, `port`, `noCodex`, `denied` or `other`.
    Failed {
        reason: &'static str,
        detail: Option<String>,
    },
    /// `busy`, `inUse`, `signedIn`, `changed`, `badUrl`, `name`, `empty` or `key`.
    Refused {
        reason: &'static str,
    },
}

fn refused(reason: &'static str) -> AccountDone {
    AccountDone::Refused { reason }
}

fn failed(reason: &'static str, detail: Option<String>) -> AccountDone {
    AccountDone::Failed { reason, detail }
}

fn chatgpt_number(id: &str) -> Option<u32> {
    id.strip_prefix(CHATGPT_ID)?.parse().ok()
}

/// The list read again and shown, and a quota read asked for.
fn relist(app: &AppHandle, state: &AppState) {
    let (home, dir) = {
        let inner = state.lock();
        (inner.codex_home(), inner.app_dir())
    };
    if let Some((accounts, nicknames)) = list(&home, &dir) {
        let mut inner = state.lock();
        inner.codex.accounts = accounts;
        inner.codex.nicknames = nicknames;
    }
    state.request_read();
    reader::publish(app, state);
}

/// Switches Codex to one account after the owner confirmed it on the
/// sheet: a ChatGPT sign-in moves into the Codex folder, a provider is
/// chosen in `config.toml`. Rejects with `busy`.
#[tauri::command]
pub async fn codex_use(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    base: Option<String>,
) -> Result<Outcome, String> {
    let shared = state.inner().clone();
    let plugins = shared.lock().settings.codex.plugins;
    let out = write(&app, &shared, move |home, dir| {
        codex::use_account(
            home,
            dir,
            &Unix::system(),
            &id,
            base.as_deref(),
            plugins,
            OffsetDateTime::now_utc(),
        )
    })
    .await?;
    shared.request_read();
    Ok(out)
}

/// Signs a new ChatGPT account in, or `id` again: `codex login` with a new,
/// empty folder of its own as `CODEX_HOME`, so the Codex in use and its
/// sign-in stay as they are. Waits up to ten minutes for the browser.
#[tauri::command]
pub async fn codex_login(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<AccountDone, String> {
    let again = match id.as_deref() {
        None => None,
        Some(i) => match chatgpt_number(i) {
            Some(n) => Some(n),
            None => return Ok(failed("other", None)),
        },
    };
    let shared = state.inner().clone();
    let (home, dir, user_home, cancel) = {
        let mut inner = shared.lock();
        if inner.codex.signing.is_some() || inner.codex.busy.is_some() {
            return Ok(refused("busy"));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        inner.codex.signing = Some(cancel.clone());
        (inner.codex_home(), inner.app_dir(), inner.home.clone(), cancel)
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        sign_in(&home, &dir, &user_home, again, &cancel)
    })
    .await
    .unwrap_or_else(|_| failed("other", None));
    shared.lock().codex.signing = None;
    relist(&app, &shared);
    Ok(done)
}

/// Stops the sign-in that waits for the browser.
#[tauri::command]
pub fn codex_cancel_login(state: State<'_, AppState>) {
    if let Some(cancel) = &state.lock().codex.signing {
        cancel.store(true, Ordering::SeqCst);
    }
}

fn sign_in(
    home: &Path,
    dir: &AppDir,
    user_home: &Path,
    again: Option<u32>,
    cancel: &AtomicBool,
) -> AccountDone {
    let Some(codex_cli) = procs::discover(user_home) else { return failed("noCodex", None) };
    let platform = Unix::system();
    let store = Store::new(dir, home, &platform);
    let pending = match store.begin_login() {
        Ok(p) => p,
        Err(e) => return failed("other", Some(e.to_string())),
    };
    let ended = run_login(&codex_cli, &pending, cancel);
    let printed = std::fs::read_to_string(pending.join(LOGIN_LOG)).unwrap_or_default();
    let _ = std::fs::remove_file(pending.join(LOGIN_LOG));
    match ended {
        Ok(true) => {}
        Ok(false) => {
            store.discard(&pending);
            return login_failure(&printed);
        }
        Err(reason) => {
            store.discard(&pending);
            return failed(reason, None);
        }
    }
    match store.finish_login(&pending, again, OffsetDateTime::now_utc()) {
        Ok(Added::Added { number }) => AccountDone::Added { id: format!("{CHATGPT_ID}{number}") },
        Ok(Added::Dup { number }) => AccountDone::Dup { id: format!("{CHATGPT_ID}{number}") },
        Ok(Added::Renewed { number }) => {
            AccountDone::Renewed { id: format!("{CHATGPT_ID}{number}") }
        }
        Ok(Added::NoLogin) => failed("other", Some("no ChatGPT sign-in".into())),
        Err(e) => {
            store.discard(&pending);
            failed("other", Some(e.to_string()))
        }
    }
}

/// Runs `codex login` in `folder` until it ends: `Ok(succeeded)`, or
/// `Err(cancelled | timeout | other)` after stopping it.
fn run_login(codex_cli: &Path, folder: &Path, cancel: &AtomicBool) -> Result<bool, &'static str> {
    // Codex keeps this sign-in in the folder's `auth.json`, whatever the
    // Mac's own config says.
    std::fs::write(folder.join(codex::CONFIG), "cli_auth_credentials_store = \"file\"\n")
        .map_err(|_| "other")?;
    let printed = File::create(folder.join(LOGIN_LOG)).map_err(|_| "other")?;
    let printed_err = printed.try_clone().map_err(|_| "other")?;
    let mut child = Command::new(codex_cli)
        .arg("login")
        .env("CODEX_HOME", folder)
        .stdin(Stdio::null())
        .stdout(printed)
        .stderr(printed_err)
        .spawn()
        .map_err(|_| "noCodex")?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.success()),
            Ok(None) => {}
            Err(_) => return Err("other"),
        }
        let stop = if cancel.load(Ordering::SeqCst) {
            Some("cancelled")
        } else if started.elapsed() > LOGIN_WAIT {
            Some("timeout")
        } else {
            None
        };
        if let Some(why) = stop {
            let _ = child.kill();
            let _ = child.wait();
            return Err(why);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Why `codex login` ended without a sign-in, from what it printed. The
/// detail is its last line, links left out.
fn login_failure(printed: &str) -> AccountDone {
    let low = printed.to_lowercase();
    if low.contains("address already in use") || low.contains("port 1455") {
        return failed("port", None);
    }
    if low.contains("access_denied") || low.contains("denied") {
        return failed("denied", None);
    }
    let last = printed.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    let words: Vec<&str> = last.split_whitespace().filter(|w| !w.contains("://")).collect();
    let detail: String = words.join(" ").chars().take(160).collect();
    failed("other", (!detail.is_empty()).then_some(detail))
}

/// "Kiểm tra" on a new provider: its address with no key, then with the
/// key, sent to that address only.
#[tauri::command]
pub async fn codex_check_provider(base_url: String, key: String) -> Result<KeyCheck, String> {
    tauri::async_runtime::spawn_blocking(move || probe::check_key(&base_url, &key))
        .await
        .map_err(|_| "error".to_string())
}

/// The lines a new provider adds to `config.toml`, the key only as its hint.
#[tauri::command]
pub async fn codex_provider_preview(
    state: State<'_, AppState>,
    name: String,
    base_url: String,
    key: String,
) -> Result<ProviderPreview, String> {
    let (home, dir) = {
        let inner = state.lock();
        (inner.codex_home(), inner.app_dir())
    };
    tauri::async_runtime::spawn_blocking(move || {
        codex::provider_preview(&home, &dir, &name, &base_url, &key)
    })
    .await
    .map_err(|_| "error".to_string())
}

fn key_refused(refusal: &KeyRefusal) -> AccountDone {
    refused(match refusal {
        KeyRefusal::Name => "name",
        KeyRefusal::BadUrl => "badUrl",
        KeyRefusal::Empty => "empty",
        KeyRefusal::InUse => "inUse",
        KeyRefusal::Unknown => "changed",
        _ => "key",
    })
}

fn provider_done(out: ProviderOutcome, name: String) -> AccountDone {
    match out {
        ProviderOutcome::Added { id, .. } => AccountDone::Added { id },
        ProviderOutcome::Removed { id, .. } => AccountDone::Removed { id, name, logged_out: false },
        ProviderOutcome::Refused { refusal } => key_refused(&refusal),
        ProviderOutcome::Failed { error: WriteError::Changed } => refused("changed"),
        ProviderOutcome::Failed { .. } => failed("other", None),
        ProviderOutcome::Invalid { detail } => failed("other", Some(detail)),
    }
}

/// Adds the provider the preview showed, when `config.toml` still has the
/// hash `base`. Codex is not switched to it.
#[tauri::command]
pub async fn codex_add_provider(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    base_url: String,
    key: String,
    base: String,
) -> Result<AccountDone, String> {
    let shared = state.inner().clone();
    let named = name.clone();
    let out = write(&app, &shared, move |home, dir| {
        codex::add_provider(home, dir, &name, &base_url, &key, &base, OffsetDateTime::now_utc())
    })
    .await;
    match out {
        Ok(out) => Ok(provider_done(out, named)),
        Err(e) if e == "busy" => Ok(refused("busy")),
        // A join error's text could name the closure's values: say no more.
        Err(_) => Ok(failed("other", None)),
    }
}

/// A nickname for one account, kept by the app; empty goes back to its own name.
#[tauri::command]
pub async fn codex_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<AccountDone, String> {
    let (home, dir) = {
        let inner = state.lock();
        (inner.codex_home(), inner.app_dir())
    };
    let target = id.clone();
    let out = tauri::async_runtime::spawn_blocking(move || {
        Store::new(&dir, &home, &Unix::system()).rename(&target, &name)
    })
    .await
    .map_err(|e| e.to_string())?;
    relist(&app, state.inner());
    Ok(match out {
        Ok(()) => AccountDone::Renamed { id },
        Err(accounts::Refusal::Name) => refused("name"),
        Err(accounts::Refusal::Unknown) => refused("changed"),
        Err(e) => failed("other", Some(format!("{e:?}"))),
    })
}

/// Removes an account Codex is not using: a ChatGPT one's folder (signed
/// out first with `codex logout` in it when `logout`), or a provider's
/// table, when `config.toml` still has the hash `base`.
#[tauri::command]
pub async fn codex_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    logout: bool,
    base: Option<String>,
) -> Result<AccountDone, String> {
    let shared = state.inner().clone();
    let (name, current) = {
        let inner = shared.lock();
        let reading = inner.codex.reading.as_ref();
        let parsed = reading.and_then(|r| r.parsed());
        let login = reading.map(|r| r.login).unwrap_or(Login::Missing);
        let current = parsed.map(|p| p.current());
        let name = view(&inner, parsed, login, "")
            .into_iter()
            .find(|a| a.id == id)
            .map(|a| a.name)
            .unwrap_or_else(|| id.clone());
        (name, current)
    };
    let Some(number) = chatgpt_number(&id) else {
        let out = write(&app, &shared, move |home, dir| {
            codex::remove_provider(
                home,
                dir,
                &id,
                base.as_deref().unwrap_or_default(),
                OffsetDateTime::now_utc(),
            )
        })
        .await;
        return Ok(match out {
            Ok(out) => provider_done(out, name),
            Err(e) if e == "busy" => refused("busy"),
            Err(_) => failed("other", None),
        });
    };
    let (home, dir, user_home) = {
        let mut inner = shared.lock();
        if inner.codex.busy.is_some() || inner.codex.signing.is_some() {
            return Ok(refused("busy"));
        }
        inner.codex.busy = Some("switching");
        (inner.codex_home(), inner.app_dir(), inner.home.clone())
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        let platform = Unix::system();
        let store = Store::new(&dir, &home, &platform);
        let folder = store.removable(number)?;
        let logged_out = logout && sign_out(&user_home, &folder);
        store.remove(number)?;
        Cache::account(&dir, ProviderId::Codex, number).retire();
        Ok::<bool, accounts::Refusal>(logged_out)
    })
    .await;
    shared.lock().codex.busy = None;
    relist(&app, &shared);
    Ok(match done {
        Ok(Ok(logged_out)) => AccountDone::Removed { id, name, logged_out },
        Ok(Err(accounts::Refusal::InUse)) => {
            refused(if current == Some(config::Current::ChatGpt) { "inUse" } else { "signedIn" })
        }
        Ok(Err(accounts::Refusal::Unknown)) => refused("changed"),
        Ok(Err(e)) => failed("other", Some(format!("{e:?}"))),
        Err(_) => failed("other", None),
    })
}

/// `codex logout` with the account's folder as `CODEX_HOME`: revokes that
/// sign-in at OpenAI. Gives up after [`LOGOUT_WAIT`].
fn sign_out(user_home: &Path, folder: &PathBuf) -> bool {
    let Some(codex_cli) = procs::discover(user_home) else { return false };
    let Ok(mut child) = Command::new(codex_cli)
        .arg("logout")
        .env("CODEX_HOME", folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < LOGOUT_WAIT => {
                std::thread::sleep(Duration::from_millis(200))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A temporary home with a fake `codex` in `~/.local/bin` running `body`.
    fn fake_codex(name: &str, body: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("qs-codex-login-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let script = bin.join("codex");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        home
    }

    /// A made-up ChatGPT sign-in of workspace `ws-<user>`; its tokens are not real.
    fn made_up(user: &str) -> String {
        serde_json::json!({ "tokens": { "access_token": "not-real", "refresh_token": "not-real", "account_id": format!("ws-{user}") } })
            .to_string()
    }

    #[test]
    fn a_sign_in_runs_codex_login_in_a_folder_of_its_own_and_adds_the_account() {
        let auth = made_up("ann").replace('"', "\\\"");
        let home = fake_codex(
            "ok",
            &format!("echo \"$@\" > \"$CODEX_HOME/args\"; cat \"$CODEX_HOME/config.toml\" > \"$CODEX_HOME/seen\"; printf '%s' \"{auth}\" > \"$CODEX_HOME/auth.json\""),
        );
        let codex_home = home.join(".codex");
        std::fs::create_dir_all(&codex_home).unwrap();
        let dir = AppDir::new(home.join("app"));
        let done = sign_in(&codex_home, &dir, &home, None, &AtomicBool::new(false));
        assert!(matches!(done, AccountDone::Added { ref id } if id == "chatgpt:1"), "{done:?}");
        let folder = dir.path().join(accounts::DIR).join("1");
        assert_eq!(std::fs::read_to_string(folder.join("args")).unwrap().trim(), "login");
        assert!(std::fs::read_to_string(folder.join("seen")).unwrap().contains("\"file\""));
        assert!(folder.join(codex::AUTH).is_file());
        assert!(!folder.join(LOGIN_LOG).exists(), "what it printed is not kept");
        assert!(!codex_home.join(codex::AUTH).exists(), "the Codex folder is not touched");
    }

    #[test]
    fn a_sign_in_that_fails_or_is_stopped_leaves_nothing() {
        let home =
            fake_codex("port", "echo 'Error: Address already in use (os error 48)' >&2; exit 1");
        let codex_home = home.join(".codex");
        let dir = AppDir::new(home.join("app"));
        let done = sign_in(&codex_home, &dir, &home, None, &AtomicBool::new(false));
        assert!(matches!(done, AccountDone::Failed { reason: "port", .. }), "{done:?}");

        let home = fake_codex("wait", "sleep 30");
        let dir = AppDir::new(home.join("app"));
        let done = sign_in(&home.join(".codex"), &dir, &home, None, &AtomicBool::new(true));
        assert!(matches!(done, AccountDone::Failed { reason: "cancelled", .. }), "{done:?}");
        let left: Vec<_> = std::fs::read_dir(dir.path().join(accounts::DIR))
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert!(left.is_empty(), "no folder left: {left:?}");
    }

    #[test]
    fn what_codex_login_printed_says_why_without_its_links() {
        assert!(matches!(
            login_failure("error: access_denied"),
            AccountDone::Failed { reason: "denied", .. }
        ));
        let other = login_failure("Starting server\nOpen https://auth.openai.com/x?state=1 to continue\nunexpected response\n");
        assert!(
            matches!(other, AccountDone::Failed { reason: "other", detail: Some(ref d) } if d == "unexpected response"),
            "{other:?}"
        );
        let link = login_failure("see https://example.com/a");
        assert!(
            matches!(link, AccountDone::Failed { detail: Some(ref d), .. } if !d.contains("://")),
            "{link:?}"
        );
        assert_eq!(plan_name("plus"), "Plus");
    }
}
