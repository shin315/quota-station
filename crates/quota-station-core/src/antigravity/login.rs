//! Logging an Antigravity account in and out, and getting its token right
//! before a use. The login runs in Cockpit Tools' order
//! (`start_oauth_login` in `src-tauri/src/commands/oauth.rs`, 1.3.70): open
//! the consent screen, wait for the callback, exchange the code, insist on a
//! refresh token, ask userinfo for the email, then keep the account. Saving
//! is the app's own: the token goes to the app's Keychain item first, and the
//! index is written only once that worked.

use std::path::Path;
use std::time::Duration;

use super::client::{self, ClientStatus};
use super::fs::AppDir;
use super::index::{self, AccountSummary, IndexError, Standing};
use super::oauth::{self, Endpoints, OAuthError};
use super::oauth_server::{self, Flow, FlowError};
use super::store::{Keychain, StoreError};
use super::token::TokenData;
use crate::runner;

/// Opens the consent screen.
pub trait Browser {
    fn open(&self, url: &str) -> Result<(), String>;
}

/// The default browser, through `open`.
pub struct SystemBrowser;

impl Browser for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), String> {
        let out =
            runner::run(Path::new("/usr/bin/open"), &[url.to_string()], Duration::from_secs(15))
                .map_err(|e| e.to_string())?;
        if out.code == Some(0) {
            Ok(())
        } else {
            Err(runner::first_line(&out.stderr))
        }
    }
}

/// Where a login keeps things and whom it talks to.
pub struct Deps<'a> {
    pub app: &'a AppDir,
    pub endpoints: &'a Endpoints,
    pub keychain: &'a Keychain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginError {
    /// `oauth-client.json` is not usable.
    Client(ClientStatus),
    Flow(FlowError),
    Browser(String),
    Exchange(OAuthError),
    /// Google kept the refresh token for an earlier consent.
    NoRefreshToken,
    UserInfo(OAuthError),
    Index(IndexError),
    Keychain(StoreError),
}

impl LoginError {
    /// A short name for the UI to choose its words by.
    pub fn kind(&self) -> &'static str {
        match self {
            LoginError::Client(_) => "client",
            LoginError::Flow(FlowError::Bind(_)) => "bind",
            LoginError::Flow(FlowError::Callback(oauth_server::CallbackError::StateMismatch)) => {
                "stateMismatch"
            }
            LoginError::Flow(FlowError::Callback(oauth_server::CallbackError::NoCode)) => "noCode",
            LoginError::Flow(FlowError::Callback(_)) => "callback",
            LoginError::Flow(FlowError::Cancelled) => "cancelled",
            LoginError::Flow(FlowError::Timeout) => "timeout",
            LoginError::Browser(_) => "browser",
            LoginError::Exchange(e) if e.is_invalid_grant() => "invalidGrant",
            LoginError::Exchange(OAuthError::Network(_))
            | LoginError::UserInfo(OAuthError::Network(_)) => "network",
            LoginError::Exchange(_) => "exchange",
            LoginError::NoRefreshToken => "noRefreshToken",
            LoginError::UserInfo(_) => "userInfo",
            LoginError::Index(_) => "index",
            LoginError::Keychain(_) => "keychain",
        }
    }
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::Client(s) => write!(f, "oauth-client.json is not usable: {s:?}"),
            LoginError::Flow(e) => e.fmt(f),
            LoginError::Browser(e) => write!(f, "could not open the browser: {e}"),
            LoginError::Exchange(e) => write!(f, "code exchange failed: {e}"),
            LoginError::NoRefreshToken => f.write_str(NO_REFRESH_TOKEN),
            LoginError::UserInfo(e) => write!(f, "userinfo failed: {e}"),
            LoginError::Index(e) => e.fmt(f),
            LoginError::Keychain(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for LoginError {}

/// Cockpit's advice when Google sends no refresh token, in English for logs;
/// the UI says it in Vietnamese.
pub const NO_REFRESH_TOKEN: &str = "Google sent no refresh token, most likely because this \
     account approved the app before. Remove the access of \"Antigravity Tools\" at \
     https://myaccount.google.com/permissions, then log in again.";

/// The account a login kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedIn {
    pub id: String,
    /// Its place in the index, from 1: the number the app shows.
    pub number: usize,
    /// It was not in the index before.
    pub added: bool,
}

/// Reads the OAuth client and starts listening for the callback. The flow's
/// [`Flow::handle`] takes a pasted callback or cancels while
/// [`complete`] waits.
pub fn prepare(deps: &Deps) -> Result<Flow, LoginError> {
    let client = client::load(deps.app).map_err(LoginError::Client)?;
    oauth_server::prepare(deps.endpoints, &client).map_err(LoginError::Flow)
}

/// Opens the consent screen, waits up to `timeout` for the callback, and
/// keeps the account. `now` is the clock in Unix seconds.
pub fn complete(
    deps: &Deps,
    flow: Flow,
    browser: &dyn Browser,
    timeout: Duration,
    now: &dyn Fn() -> i64,
) -> Result<LoggedIn, LoginError> {
    // Dropping the flow on an error stops its listener.
    browser.open(flow.handle().auth_url()).map_err(LoginError::Browser)?;
    let client = flow.client().clone();
    let (code, redirect_uri) = flow.wait(timeout).map_err(LoginError::Flow)?;

    let token_res = oauth::exchange_code(deps.endpoints, &client, &code, &redirect_uri)
        .map_err(LoginError::Exchange)?;
    let refresh_token = token_res.refresh_token.ok_or(LoginError::NoRefreshToken)?;

    let user_info = oauth::get_user_info(deps.endpoints, &token_res.access_token)
        .map_err(LoginError::UserInfo)?;

    let token_data = TokenData::new(
        token_res.access_token,
        refresh_token,
        token_res.expires_in,
        Some(user_info.email.clone()),
        None,
        user_info.id.clone(),
        now(),
    )
    .with_oauth_metadata(token_res.oauth_client_key, token_res.id_token);

    upsert_account(deps, &user_info.email, user_info.get_display_name(), token_data, now())
}

/// Keeps a logged-in account: a new one gets Cockpit's id for its email, one
/// already listed keeps its id and creation time, and gets back the project
/// and Google id its last token had when the new one has none. The Keychain
/// item is written first; on a failure the index is not touched.
fn upsert_account(
    deps: &Deps,
    email: &str,
    name: Option<String>,
    token: TokenData,
    now: i64,
) -> Result<LoggedIn, LoginError> {
    let email = email.trim().to_string();
    let mut index = index::load(deps.app).map_err(LoginError::Index)?;
    let existing = index.find_email(&email).map(|s| s.id.clone());
    let id = existing.clone().unwrap_or_else(|| index::account_id(&email));

    let mut next_token = token;
    if existing.is_some() {
        if let Ok(old) = deps.keychain.read(&id) {
            if next_token.project_id.is_none() {
                next_token.project_id = old.project_id;
            }
            if next_token.session_id.is_none() {
                next_token.session_id = old.session_id;
            }
        }
    }
    deps.keychain.save(&id, &next_token).map_err(LoginError::Keychain)?;

    match index.accounts.iter_mut().find(|s| s.id == id) {
        Some(summary) => {
            summary.email = email;
            if name.is_some() {
                summary.name = name;
            }
            summary.last_used = now;
        }
        None => index.accounts.push(AccountSummary {
            id: id.clone(),
            email,
            name,
            created_at: now,
            last_used: now,
        }),
    }
    if index.current_account_id.is_none() {
        index.current_account_id = Some(id.clone());
    }
    if let Err(e) = index::save(deps.app, &index) {
        // A new account's item would be left with nothing pointing at it.
        if existing.is_none() {
            let _ = deps.keychain.delete(&id);
        }
        return Err(LoginError::Index(e));
    }
    // A new login answers "needs a new login" and "needs a verification".
    let _ = index::set_standing(deps.app, &id, None);

    let number = index.accounts.iter().position(|s| s.id == id).map_or(0, |i| i + 1);
    Ok(LoggedIn { id, number, added: existing.is_none() })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogoutError {
    NotFound,
    Index(IndexError),
    Keychain(StoreError),
}

impl std::fmt::Display for LogoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogoutError::NotFound => write!(f, "no such account"),
            LogoutError::Index(e) => e.fmt(f),
            LogoutError::Keychain(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for LogoutError {}

/// Forgets an account in this app: its Keychain item, its index line and its
/// status. Antigravity.app and agy stay logged in as they are, and Google is
/// not asked to revoke anything (Cockpit does not either). The item goes
/// first, so a failure leaves the account listed rather than a token behind.
pub fn logout(deps: &Deps, id: &str) -> Result<(), LogoutError> {
    let mut index = index::load(deps.app).map_err(LogoutError::Index)?;
    if !index.accounts.iter().any(|s| s.id == id) {
        return Err(LogoutError::NotFound);
    }
    deps.keychain.delete(id).map_err(LogoutError::Keychain)?;
    index.accounts.retain(|s| s.id != id);
    if index.current_account_id.as_deref() == Some(id) {
        index.current_account_id = None;
    }
    index::save(deps.app, &index).map_err(LogoutError::Index)?;
    let _ = index::set_standing(deps.app, id, None);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseError {
    Client(ClientStatus),
    Keychain(StoreError),
    /// Google refused the refresh token; the account now needs a new login.
    NeedsLogin,
    Refresh(OAuthError),
}

impl std::fmt::Display for UseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseError::Client(s) => write!(f, "oauth-client.json is not usable: {s:?}"),
            UseError::Keychain(e) => e.fmt(f),
            UseError::NeedsLogin => write!(f, "Google refused the refresh token: log in again"),
            UseError::Refresh(e) => write!(f, "token refresh failed: {e}"),
        }
    }
}

impl std::error::Error for UseError {}

/// The account's token for a use about to happen (a quota read, a switch),
/// refreshed first when it has less than five minutes left. Nothing refreshes
/// tokens on a timer. A refreshed token goes back to the Keychain; an
/// `invalid_grant` marks the account as needing a new login.
pub fn fresh_token(deps: &Deps, id: &str, now: i64) -> Result<TokenData, UseError> {
    let client = client::load(deps.app).map_err(UseError::Client)?;
    let token = deps.keychain.read(id).map_err(UseError::Keychain)?;
    match oauth::ensure_fresh_token(deps.endpoints, &client, &token, now) {
        Ok(fresh) => {
            if fresh.access_token != token.access_token {
                // As Cockpit: the fresh token works now even if keeping it failed.
                let _ = deps.keychain.save(id, &fresh);
            }
            Ok(fresh)
        }
        Err(e) if e.is_invalid_grant() => {
            let _ = index::set_standing(deps.app, id, Some(Standing::NeedsLogin { since: now }));
            Err(UseError::NeedsLogin)
        }
        Err(e) => Err(UseError::Refresh(e)),
    }
}
