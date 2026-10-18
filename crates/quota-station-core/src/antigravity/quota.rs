//! Reading one Antigravity account's quota, in Cockpit Tools' order
//! (`fetch_quota_with_fresh_token` in `src-tauri/src/modules/account.rs` and
//! `fetch_quota_with_context` in `src-tauri/src/modules/quota.rs`, 1.3.70):
//! a token refreshed when it is due, `fetchAvailableModels`,
//! `retrieveUserQuotaSummary`, then the reading in the cache quota-axi reads.
//! Cockpit's account handling decides what a refusal means: a refused
//! refresh token needs a new login, a 403 stops the reads, and a request for
//! verification is shown to the owner.
//!
//! Fewer calls than Cockpit, never more: a reading younger than a minute is
//! used before any call is made, and `loadCodeAssist` (which Cockpit asks
//! before every reading) is asked only while the token has no project or no
//! GCP ToS mark: after a login, or when Google gave none. Unlike Cockpit,
//! which reads on without it, a failed `loadCodeAssist` ends the read like a
//! failed quota call: the last reading stays with its age, and a 429 or a
//! 5xx backs the account off, so a struggling Google is not asked again
//! every minute.

use super::client::ClientStatus;
use super::cloud_code::{self, CloudCode, CloudCodeError, Context, Hosts};
use super::export;
use super::fs::AppDir;
use super::index::{self, AccountIndex, IndexError, Standing};
use super::login::{self, UseError};
use super::oauth::OAuthError;
use super::pace::{Candidate, Finish};
use super::store::StoreError;
use super::version::Identity;

/// Where a read keeps things and whom it talks to.
pub struct Deps<'a> {
    pub login: &'a login::Deps<'a>,
    pub hosts: &'a Hosts,
    pub identity: &'a Identity,
}

/// How one read ended.
pub type ReadResult = Result<Outcome, ReadError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The reading in the cache is younger than a minute: nothing was asked.
    Cached,
    /// A new reading is in the cache.
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Not in the index any more.
    NotFound,
    /// Google refused the refresh token, now or before: only a new login helps.
    NeedsLogin,
    /// The quota API refused the account (403), now or before: no more
    /// reads until a new login.
    Forbidden,
    Index(IndexError),
    Keychain(StoreError),
    Client(ClientStatus),
    Refresh(OAuthError),
    CloudCode(CloudCodeError),
    /// The reading could not be kept.
    Cache(String),
}

impl ReadError {
    /// A short name for the windows to choose their words by.
    pub fn kind(&self) -> &'static str {
        match self {
            ReadError::NotFound => "notFound",
            ReadError::NeedsLogin => "needsLogin",
            ReadError::Forbidden => "forbidden",
            ReadError::Index(_) => "index",
            ReadError::Keychain(_) => "keychain",
            ReadError::Client(_) => "client",
            ReadError::Refresh(OAuthError::Network(_)) => "network",
            ReadError::Refresh(_) => "refresh",
            ReadError::CloudCode(CloudCodeError::Network { .. }) => "network",
            ReadError::CloudCode(CloudCodeError::Parse { .. }) => "parse",
            ReadError::CloudCode(e) => match e.status() {
                Some(401) => "unauthorized",
                Some(429) => "rateLimited",
                Some(s) if s >= 500 => "server",
                _ if e.api_error().is_some_and(|a| a.needs_verification()) => "verification",
                _ => "rejected",
            },
            ReadError::Cache(_) => "cache",
        }
    }

    /// A 429 or a 5xx: the account is read less often for a while.
    pub fn throttled(&self) -> bool {
        matches!(self, ReadError::CloudCode(e) if e.throttled())
    }

    /// The account's status in `status.json` says it all; the windows show
    /// no read error for it.
    pub fn in_status(&self) -> bool {
        matches!(self, ReadError::NotFound | ReadError::NeedsLogin | ReadError::Forbidden)
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::NotFound => write!(f, "no such account"),
            ReadError::NeedsLogin => write!(f, "the account needs a new login"),
            ReadError::Forbidden => write!(f, "Google refused this account (403)"),
            ReadError::Index(e) => e.fmt(f),
            ReadError::Keychain(e) => e.fmt(f),
            ReadError::Client(s) => write!(f, "oauth-client.json is not usable: {s:?}"),
            ReadError::Refresh(e) => write!(f, "token refresh failed: {e}"),
            ReadError::CloudCode(e) => e.fmt(f),
            ReadError::Cache(e) => write!(f, "cannot keep the reading: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<UseError> for ReadError {
    fn from(e: UseError) -> Self {
        match e {
            UseError::Client(s) => ReadError::Client(s),
            UseError::Keychain(e) => ReadError::Keychain(e),
            UseError::NeedsLogin => ReadError::NeedsLogin,
            UseError::Refresh(e) => ReadError::Refresh(e),
        }
    }
}

/// The accounts the app could read, from the index, the statuses and the
/// cache, for [`super::pace::Pace::take`].
pub fn candidates(app: &AppDir) -> Result<(AccountIndex, Vec<Candidate>), IndexError> {
    let index = index::load(app)?;
    let statuses = index::load_statuses(app);
    let list = index
        .accounts
        .iter()
        .map(|a| Candidate {
            id: a.id.clone(),
            read_at: export::read(app, &a.email).map(|r| r.updated_at),
            skip: matches!(
                statuses.accounts.get(&a.id),
                Some(Standing::NeedsLogin { .. } | Standing::Forbidden { .. })
            ),
        })
        .collect();
    Ok((index, list))
}

/// The account in use: quota-axi's `activeAccount` (its place in the
/// index, from 1) when quota-axi read this app's folder, else the index's
/// current account.
pub fn active_id(index: &AccountIndex, quota_axi_number: Option<usize>) -> Option<String> {
    quota_axi_number
        .and_then(|n| n.checked_sub(1))
        .and_then(|i| index.accounts.get(i))
        .map(|a| a.id.clone())
        .or_else(|| index.current_account_id.clone())
}

/// How a read ended, for the pace.
pub fn finish(result: &ReadResult) -> Finish {
    match result {
        Ok(_) => Finish::Done,
        Err(e) if e.throttled() => Finish::Throttled,
        Err(_) => Finish::Failed,
    }
}

/// Reads `ids` one after another, never two at once. `progress` hears of
/// each read as it starts (`None`) and as it ends.
pub fn read_in_turn(
    deps: &Deps,
    ids: &[String],
    clock: &dyn Fn() -> i64,
    progress: &mut dyn FnMut(&str, Option<&ReadResult>),
) {
    for id in ids {
        progress(id, None);
        let result = read_account(deps, id, clock());
        progress(id, Some(&result));
    }
}

/// Reads one account's quota into the cache, unless the cache already has
/// a reading younger than a minute. `now_ms` is the clock in Unix
/// milliseconds. A failure leaves the last reading as it was, so quota-axi
/// keeps showing it with its age.
pub fn read_account(deps: &Deps, id: &str, now_ms: i64) -> ReadResult {
    let app = deps.login.app;
    let now = now_ms.div_euclid(1000);
    let index = index::load(app).map_err(ReadError::Index)?;
    let account = index.accounts.iter().find(|a| a.id == id).ok_or(ReadError::NotFound)?;
    let email = account.email.clone();
    let standing = index::load_statuses(app).accounts.remove(id);
    match standing {
        Some(Standing::NeedsLogin { .. }) => return Err(ReadError::NeedsLogin),
        Some(Standing::Forbidden { .. }) => return Err(ReadError::Forbidden),
        _ => {}
    }

    let token = deps.login.keychain.read(id).map_err(ReadError::Keychain)?;
    let cached = export::read(app, &email);
    let ctx = Context::from_token(&token);
    if cached.is_some_and(|r| r.fresh(now_ms) && r.usable_for(ctx.project_id.as_deref())) {
        return Ok(Outcome::Cached);
    }

    let mut token = login::refresh_if_due(deps.login, id, &token, now)?;
    let cloud = CloudCode { hosts: deps.hosts, identity: deps.identity };
    let has_project = cloud_code::valid_project_id(token.project_id.as_deref()).is_some();
    if token.is_gcp_tos.is_none() || !has_project {
        let info = cloud
            .load_code_assist(&token.access_token, &email, &ctx)
            .map_err(|e| refused(app, id, now, e))?;
        if info.project_id.is_some() {
            token.project_id = info.project_id;
        }
        // Asked and answered: a token without the mark is asked again.
        token.is_gcp_tos = Some(info.is_gcp_tos.unwrap_or(false));
        let _ = deps.login.keychain.save(id, &token);
    }

    let ctx = Context::from_token(&token);
    match cloud.fetch_quota(&token.access_token, &email, &ctx) {
        Ok(payload) => {
            export::write(app, &email, ctx.project_id.clone(), payload, now_ms)
                .map_err(|e| ReadError::Cache(e.to_string()))?;
            // A full reading answers a verification Google asked for, as in Cockpit.
            if matches!(standing, Some(Standing::NeedsVerification { .. })) {
                let _ = index::set_standing(app, id, None);
            }
            Ok(Outcome::Read)
        }
        Err(e) if e.forbidden() => {
            let url = e.api_error().and_then(|a| a.validation_url.clone());
            // Unsaved, the status would not stop the reads: say why instead.
            index::set_standing(app, id, Some(Standing::Forbidden { since: now, url }))
                .map_err(ReadError::Index)?;
            Err(ReadError::Forbidden)
        }
        Err(e) => Err(refused(app, id, now, e)),
    }
}

/// A Cloud Code call failed without stopping the reads (not the 403 of
/// `fetchAvailableModels`): the last reading stays, and a verification
/// Google asks for is kept as the account's status.
fn refused(app: &AppDir, id: &str, now: i64, e: CloudCodeError) -> ReadError {
    if let Some(api) = e.api_error().filter(|a| a.needs_verification()) {
        let url = api.validation_url.clone();
        let verify = Standing::NeedsVerification { since: now, url };
        let _ = index::set_standing(app, id, Some(verify));
    }
    ReadError::CloudCode(e)
}
