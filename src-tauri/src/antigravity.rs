//! The Antigravity accounts this app logs in (M4a) and reads (M4b): what the
//! windows show of them, and the commands to add one through Google's
//! consent screen, take a pasted callback, cancel, log out, open Google's
//! verification page, and read every account when the tab opens. Tokens
//! stay in the app's own Keychain items; the windows only ever get each
//! account's id (Cockpit's MD5 of its email), its number, its status, how
//! the app's read of it stands and, when the owner turns them on, the
//! emails. The numbers themselves come from quota-axi.

use quota_station_core::antigravity::client::{self, ClientStatus};
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::antigravity::index::{self, Standing};
use quota_station_core::antigravity::login::{
    self, Browser, Deps, LoginError, LogoutError, SystemBrowser,
};
use quota_station_core::antigravity::oauth::Endpoints;
use quota_station_core::antigravity::oauth_server::{CallbackError, LOGIN_TIMEOUT};
use quota_station_core::antigravity::store::Keychain;
use quota_station_core::runner;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner, Model, ReadState};

/// The Antigravity accounts card and the settings row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsView {
    pub client: ClientStatus,
    /// Where `oauth-client.json` goes, `~/…`.
    pub client_path: String,
    pub accounts: Vec<AccountView>,
    /// `accounts.json` is there and cannot be read.
    pub index_error: bool,
    /// A login is waiting for the browser.
    pub logging_in: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    /// Cockpit's id: `antigravity_` and the MD5 of the email. Not the email,
    /// but a guessed address can be checked against it (SECURITY.md).
    pub id: String,
    /// Its place in the index, from 1, as quota-axi numbers it.
    pub number: usize,
    /// Only when the owner turned emails on.
    pub email: Option<String>,
    pub name: Option<String>,
    /// `ok`, `needsLogin`, `needsVerification` or `forbidden`.
    pub status: &'static str,
    /// Google sent a verification page for it.
    pub verify: bool,
    /// The app is reading its quota now.
    pub reading: bool,
    /// The app's last read of it failed; the last numbers stay.
    pub read_error: Option<ReadErrorView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadErrorView {
    /// `quota::ReadError::kind`: `network`, `rateLimited`, `server`…
    pub kind: &'static str,
    pub detail: String,
    /// After a 429 or a 5xx: when the app reads it again, RFC 3339.
    pub retry_at: Option<String>,
}

pub fn view(inner: &Inner) -> AccountsView {
    let app = inner.app_dir();
    let show = inner.settings.show_email;
    let now = OffsetDateTime::now_utc();
    let now_ms = (now.unix_timestamp_nanos() / 1_000_000) as i64;
    let rfc = |ms: i64| {
        OffsetDateTime::from_unix_timestamp(ms.div_euclid(1000))
            .ok()
            .and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok())
    };
    let index = index::load(&app);
    let statuses = index::load_statuses(&app);
    let accounts = index
        .as_ref()
        .map(|i| i.accounts.as_slice())
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let standing = statuses.accounts.get(&a.id);
            let read = inner.agy_reads.get(&a.id);
            AccountView {
                id: a.id.clone(),
                number: i + 1,
                email: show.then(|| a.email.clone()),
                name: if show { a.name.clone() } else { None },
                status: match standing {
                    None => "ok",
                    Some(Standing::NeedsLogin { .. }) => "needsLogin",
                    Some(Standing::NeedsVerification { .. }) => "needsVerification",
                    Some(Standing::Forbidden { .. }) => "forbidden",
                },
                verify: verification_page(standing).is_some(),
                reading: matches!(read, Some(ReadState::Reading)),
                read_error: match read {
                    Some(ReadState::Failed { kind, detail }) => Some(ReadErrorView {
                        kind,
                        detail: detail.clone(),
                        retry_at: inner.agy_pace.waiting_until(&a.id, now_ms).and_then(rfc),
                    }),
                    _ => None,
                },
            }
        })
        .collect();
    AccountsView {
        client: client::status(&app),
        client_path: runner::display_path(&client::path(&app), &inner.home),
        accounts,
        index_error: index.is_err(),
        logging_in: inner.agy_logging_in,
    }
}

fn unix_now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// A login that did not keep an account, for the UI to word.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub kind: &'static str,
    pub detail: String,
}

impl From<&LoginError> for Failure {
    fn from(e: &LoginError) -> Self {
        let detail = match e {
            // `open`'s message may repeat the consent URL.
            LoginError::Browser(_) => String::new(),
            LoginError::Client(status) => format!("{status:?}"),
            other => other.to_string(),
        };
        Failure { kind: e.kind(), detail }
    }
}

fn failure(kind: &'static str) -> Failure {
    Failure { kind, detail: String::new() }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginDone {
    pub number: usize,
    /// Not listed before: a new account rather than a new login of one.
    pub added: bool,
}

/// Adds an Antigravity account, or logs one in again: opens Google's consent
/// screen in the default browser and waits up to 10 minutes for it to come
/// back, then keeps the account. Rejects with `busy` while another login
/// waits, `switching` while an Antigravity switch runs and `loggingOut`
/// while a logout runs, which the login would otherwise race.
#[tauri::command]
pub async fn agy_login(app: AppHandle, state: State<'_, AppState>) -> Result<LoginDone, Failure> {
    let shared = state.inner().clone();
    let dir = {
        let mut inner = shared.lock();
        if let Some(why) = inner.agy_login_refusal() {
            return Err(failure(why));
        }
        inner.agy_logging_in = true;
        inner.app_dir()
    };
    let done = tauri::async_runtime::spawn_blocking({
        let shared = shared.clone();
        let app = app.clone();
        move || {
            let endpoints = Endpoints::google();
            let keychain = Keychain::system();
            let deps = Deps { app: &dir, endpoints: &endpoints, keychain: &keychain };
            let flow = login::prepare(&deps)?;
            shared.lock().agy_login = Some(flow.handle());
            reader::publish(&app, &shared);
            login::complete(&deps, flow, &SystemBrowser, LOGIN_TIMEOUT, &unix_now)
        }
    })
    .await;
    {
        let mut inner = shared.lock();
        inner.agy_logging_in = false;
        inner.agy_login = None;
    }
    reader::publish(&app, &shared);
    match done {
        Ok(Ok(kept)) => {
            // Its quota is read now, as Cockpit reads it right after a login.
            shared.lock().agy_pace.ask(&kept.id);
            shared.wake_agy();
            Ok(LoginDone { number: kept.number, added: kept.added })
        }
        Ok(Err(e)) => Err(Failure::from(&e)),
        Err(e) => Err(Failure { kind: "spawn", detail: e.to_string() }),
    }
}

/// Finishes the waiting login with the callback URL the owner pasted, for a
/// browser that could not reach the app. A URL that fails the checks leaves
/// the login waiting.
#[tauri::command]
pub fn agy_login_callback(state: State<'_, AppState>, url: String) -> Result<(), String> {
    let handle = state.lock().agy_login.clone().ok_or("noLogin")?;
    handle.submit_callback_url(&url).map_err(|e| {
        match e {
            CallbackError::NoCode => "noCode",
            CallbackError::NoState => "noState",
            CallbackError::StateMismatch => "stateMismatch",
            CallbackError::AlreadyHandled => "noLogin",
            CallbackError::BadUrl(_) | CallbackError::BadRequest(_) => "badUrl",
        }
        .to_string()
    })
}

#[tauri::command]
pub fn agy_login_cancel(state: State<'_, AppState>) {
    if let Some(handle) = state.lock().agy_login.clone() {
        handle.cancel();
    }
}

/// Logs an account out of this app only. Antigravity.app and agy keep their
/// own login. Rejects with `busy` while an Antigravity switch runs, which
/// could otherwise point Antigravity at the account just logged out, with
/// `loggingOut` while another logout runs and with `loggingIn` while a login
/// runs, which would rewrite the account list under it. A read that started
/// before the logout is dropped, so it cannot show the account again.
#[tauri::command]
pub async fn agy_logout(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Model, String> {
    let shared = state.inner().clone();
    let dir = {
        let mut inner = shared.lock();
        if let Some(why) = inner.agy_logout_refusal() {
            return Err(why.into());
        }
        inner.agy_logging_out = true;
        inner.agy_generation += 1;
        inner.app_dir()
    };
    let result = tauri::async_runtime::spawn_blocking({
        let shared = shared.clone();
        move || {
            // Waits for a quota read that is under way to end first.
            let _no_read = shared.agy_accounts.lock().unwrap_or_else(|e| e.into_inner());
            let endpoints = Endpoints::google();
            let keychain = Keychain::system();
            let deps = Deps { app: &dir, endpoints: &endpoints, keychain: &keychain };
            let result = login::logout(&deps, &id);
            if result.is_ok() {
                let mut inner = shared.lock();
                inner.agy_reads.remove(&id);
                inner.agy_pace.forget(&id);
            }
            result
        }
    })
    .await;
    {
        let mut inner = shared.lock();
        inner.agy_logging_out = false;
        // Reads that started while it ran may still list the account.
        inner.agy_generation += 1;
    }
    let result = result.map_err(|e| e.to_string())?;
    reader::publish(&app, &shared);
    result.map_err(|e| match e {
        LogoutError::NotFound => "notFound".to_string(),
        LogoutError::Keychain(_) => "keychain".to_string(),
        LogoutError::Index(_) => "index".to_string(),
    })?;
    let model = shared.lock().model();
    Ok(model)
}

/// The verification page Google sent with a status, if any.
fn verification_page(standing: Option<&Standing>) -> Option<&str> {
    match standing {
        Some(Standing::NeedsVerification { url: Some(url), .. })
        | Some(Standing::Forbidden { url: Some(url), .. }) => Some(url),
        _ => None,
    }
}

/// Opens the verification page Google sent for an account, when it is one
/// of Google's own pages.
#[tauri::command]
pub fn agy_open_verification(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let dir = state.lock().app_dir();
    let statuses = index::load_statuses(&dir);
    let Some(url) = verification_page(statuses.accounts.get(&id)) else {
        return Err("noPage".into());
    };
    if !is_google_page(url) {
        return Err("noPage".into());
    }
    SystemBrowser.open(url).map_err(|_| "browser".to_string())
}

/// The `Antigravity` tab opened: every account is read now, unless it was
/// read in the last minute (`quota_station_core::antigravity::pace`).
#[tauri::command]
pub fn agy_tab_opened(state: State<'_, AppState>) {
    state.lock().agy_pace.ask_all();
    state.wake_agy();
}

fn is_google_page(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else { return false };
    let host = url.host_str().unwrap_or("");
    url.scheme() == "https" && (host == "google.com" || host.ends_with(".google.com"))
}

/// Shows or hides the emails of the accounts logged in here.
#[tauri::command]
pub fn set_show_email(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.show_email = on;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

impl Inner {
    /// Why an Antigravity login cannot start now: `busy` while another
    /// login waits, `switching` while an Antigravity switch runs,
    /// `loggingOut` while a logout runs; the login and the logout both
    /// rewrite the account list.
    pub fn agy_login_refusal(&self) -> Option<&'static str> {
        if self.agy_logging_in {
            Some("busy")
        } else if self.agy_switching.is_some() {
            Some("switching")
        } else if self.agy_logging_out {
            Some("loggingOut")
        } else {
            None
        }
    }

    /// Why an Antigravity logout cannot start now: `busy` while a switch
    /// runs, `loggingOut` while another logout runs, `loggingIn` while a
    /// login runs.
    pub fn agy_logout_refusal(&self) -> Option<&'static str> {
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

    /// The app's Application Support folder, where `settings.json` is.
    pub fn app_dir(&self) -> AppDir {
        AppDir::new(self.settings_file.parent().map(|p| p.to_path_buf()).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_googles_own_pages_open_for_a_verification() {
        assert!(is_google_page("https://accounts.google.com/signin/continue?x=1"));
        assert!(is_google_page("https://google.com/x"));
        assert!(!is_google_page("http://accounts.google.com/x"));
        assert!(!is_google_page("https://accounts.google.com.example.net/x"));
        assert!(!is_google_page("https://example.com/?google.com"));
        assert!(!is_google_page("file:///etc/passwd"));
    }

    #[test]
    fn a_login_is_refused_while_another_login_a_switch_or_a_logout_runs() {
        let mut inner = Inner::new(
            std::path::PathBuf::new(),
            std::path::PathBuf::from("/nonexistent/settings.json"),
            "Mac".into(),
        );
        assert_eq!(inner.agy_login_refusal(), None);

        inner.agy_logging_in = true;
        assert_eq!(inner.agy_login_refusal(), Some("busy"));
        inner.agy_logging_in = false;

        inner.agy_switching = Some(1);
        assert_eq!(inner.agy_login_refusal(), Some("switching"));
        inner.agy_switching = None;
        assert_eq!(inner.agy_login_refusal(), None, "it can start once the switch ends");

        inner.agy_logging_out = true;
        assert_eq!(inner.agy_login_refusal(), Some("loggingOut"));
        inner.agy_logging_out = false;
        assert_eq!(inner.agy_login_refusal(), None, "it can start once the logout ends");
    }

    #[test]
    fn a_logout_is_refused_while_a_switch_another_logout_or_a_login_runs() {
        let mut inner = Inner::new(
            std::path::PathBuf::new(),
            std::path::PathBuf::from("/nonexistent/settings.json"),
            "Mac".into(),
        );
        assert_eq!(inner.agy_logout_refusal(), None);

        inner.agy_switching = Some(1);
        assert_eq!(inner.agy_logout_refusal(), Some("busy"));
        inner.agy_switching = None;

        inner.agy_logging_out = true;
        assert_eq!(inner.agy_logout_refusal(), Some("loggingOut"));
        inner.agy_logging_out = false;

        inner.agy_logging_in = true;
        assert_eq!(inner.agy_logout_refusal(), Some("loggingIn"));
        inner.agy_logging_in = false;
        assert_eq!(inner.agy_logout_refusal(), None, "it can start once the login ends");
    }
}
