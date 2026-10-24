// The usage and token calls are claude-swap's (`oauth.py`, `usage_store.py`,
// `poll_policy.py`, MIT, https://github.com/realiti4/claude-swap): the same
// endpoints, beta header, client id and request bodies, the same reading of
// `five_hour`, `seven_day` and the per-model `limits[]`, the same error
// kinds, the five-minute expiry margin, the 180-second reuse and the
// back-off after a failure or a 429.

//! Each account's three windows, read by the app itself. The account Claude
//! Code is on is read with Claude Code's live token, never refreshed here:
//! Claude Code refreshes its own login. Another account whose token is about
//! to expire is refreshed first, under the app's lock, after its stored
//! credential is read again, and the new one is stored in the app's item.
//!
//! The readings go to `claude/usage.json`: windows, times and error kinds,
//! no email and no token.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use super::cc;
use super::locks::AppLock;
use super::session;
use super::store::{Kind, Store};
use super::switch;

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
/// Claude Code's own OAuth client, which its refresh tokens belong to.
pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const BETA: &str = "oauth-2025-04-20";
const USER_AGENT: &str = "quota-station/1.0";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
/// A token with less than this left is refreshed before use.
pub const EXPIRY_MARGIN_MS: i64 = 5 * 60 * 1000;
/// A reading younger than this is used as it is.
pub const REUSE_SECONDS: f64 = 180.0;
/// Another reader's claim on an account lasts at most this long.
const CLAIM_SECONDS: f64 = 90.0;
/// Between two calls in one pass.
const STAGGER: Duration = Duration::from_millis(250);
const VERSION: u32 = 1;

/// Where the reads go. The app always uses [`Endpoints::anthropic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    usage: String,
    token: String,
}

impl Endpoints {
    pub fn anthropic() -> Self {
        Endpoints { usage: USAGE_URL.into(), token: TOKEN_URL.into() }
    }

    /// A local fake at `base`, with the real paths.
    #[cfg(feature = "test-support")]
    pub fn fake(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Endpoints {
            usage: format!("{base}/api/oauth/usage"),
            token: format!("{base}/v1/oauth/token"),
        }
    }
}

/// One window: percent used and when it resets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Win {
    pub pct: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
}

/// A per-model weekly window (`Fable`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scoped {
    pub name: String,
    pub pct: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
}

/// One reading of an account, in claude-swap's `lastGood` shape.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Reading {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<Win>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seven_day: Option<Win>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scoped: Vec<Scoped>,
}

impl Reading {
    pub fn fable(&self) -> Option<&Scoped> {
        self.scoped.iter().find(|s| s.name.eq_ignore_ascii_case("fable"))
    }
}

/// The usage endpoint's answer as claude-swap reads it.
pub fn parse(data: &Value) -> Option<Reading> {
    let win = |v: &Value| -> Option<Win> {
        let pct = v.get("utilization")?.as_f64()?;
        let resets_at =
            v.get("resets_at").and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from);
        Some(Win { pct, resets_at })
    };
    let scoped = data
        .get("limits")
        .and_then(Value::as_array)
        .map(|limits| {
            limits
                .iter()
                .filter_map(|l| {
                    let name = l.pointer("/scope/model/display_name")?.as_str()?.to_string();
                    let pct = l.get("percent")?.as_f64()?;
                    let resets_at = l
                        .get("resets_at")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(String::from);
                    (!name.is_empty()).then_some(Scoped { name, pct, resets_at })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let reading = Reading {
        five_hour: data.get("five_hour").filter(|v| v.is_object()).and_then(win),
        seven_day: data.get("seven_day").filter(|v| v.is_object()).and_then(win),
        scoped,
    };
    (reading.five_hour.is_some() || reading.seven_day.is_some() || !reading.scoped.is_empty())
        .then_some(reading)
}

/// What the app knows of one account's usage.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// The account the row is about: a row of another identity is dropped.
    pub identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_good: Option<Reading>,
    /// When `last_good` was read, Unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_attempt_at: Option<f64>,
    #[serde(default)]
    pub failures: u32,
    /// The last failure's kind: `http-429`, `timeout`, `invalid_grant`…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backoff_until: Option<f64>,
    /// The refresh token was refused: only a new login mends it.
    #[serde(default)]
    pub relogin: bool,
    /// Another reader is fetching it until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_until: Option<f64>,
}

impl Row {
    /// How old the reading is at `now`, in seconds.
    pub fn age(&self, now: f64) -> Option<f64> {
        self.fetched_at.map(|t| (now - t).max(0.0))
    }

    /// The reading counts for the rotation: there is one, not older than
    /// [`super::engine::FRESH_SECONDS`], and the login is not dead.
    pub fn fresh(&self, now: f64) -> bool {
        self.last_good.is_some()
            && !self.relogin
            && self.age(now).is_some_and(|a| a <= super::engine::FRESH_SECONDS as f64)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Cache {
    #[serde(default)]
    pub version: u32,
    /// By slot number, as text.
    #[serde(default)]
    pub accounts: BTreeMap<String, Row>,
}

impl Cache {
    pub fn row(&self, slot: u32) -> Option<&Row> {
        self.accounts.get(&slot.to_string())
    }
}

pub fn load(store: &Store) -> Cache {
    super::read_optional(&store.paths.usage())
        .ok()
        .flatten()
        .and_then(|t| serde_json::from_str::<Cache>(&t).ok())
        .filter(|c| c.version == VERSION)
        .unwrap_or(Cache { version: VERSION, accounts: BTreeMap::new() })
}

pub fn save(store: &Store, cache: &Cache) -> std::io::Result<()> {
    let mut cache = cache.clone();
    cache.version = VERSION;
    super::write_private(
        &store.paths.usage(),
        &(serde_json::to_string_pretty(&cache).expect("serializes") + "\n"),
    )
}

/// Why one call failed.
#[derive(Debug, Clone, PartialEq)]
struct Failure {
    kind: String,
    retry_after: Option<f64>,
}

fn http() -> Result<reqwest::blocking::Client, Failure> {
    reqwest::blocking::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|_| Failure { kind: "network".into(), retry_after: None })
}

fn network(e: reqwest::Error) -> Failure {
    Failure { kind: if e.is_timeout() { "timeout" } else { "network" }.into(), retry_after: None }
}

fn fetch_usage(endpoints: &Endpoints, token: &str) -> Result<Reading, Failure> {
    let response = http()?
        .get(&endpoints.usage)
        .bearer_auth(token)
        .header("anthropic-beta", BETA)
        .send()
        .map_err(network)?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| v.max(0.0));
        return Err(Failure { kind: format!("http-{}", status.as_u16()), retry_after });
    }
    let data: Value =
        response.json().map_err(|_| Failure { kind: "bad-response".into(), retry_after: None })?;
    parse(&data).ok_or(Failure { kind: "bad-response".into(), retry_after: None })
}

/// A refreshed credential, every other member of `credential` kept.
fn refresh(endpoints: &Endpoints, credential: &str, now_ms: i64) -> Result<String, Failure> {
    let transient = || Failure { kind: "refresh-failed".into(), retry_after: None };
    let mut data: Value = serde_json::from_str(credential).map_err(|_| transient())?;
    let refresh_token = data
        .pointer("/claudeAiOauth/refreshToken")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or(Failure { kind: "no_refresh_token".into(), retry_after: None })?
        .to_string();
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "client_id": CLIENT_ID,
    });
    let response = http()?.post(&endpoints.token).json(&body).send().map_err(network)?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().unwrap_or_default();
        let code = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(Value::as_str).map(String::from));
        return Err(match (status.as_u16(), code.as_deref()) {
            (400 | 401 | 403, Some(c @ ("invalid_grant" | "invalid_client"))) => {
                Failure { kind: c.to_string(), retry_after: None }
            }
            _ => transient(),
        });
    }
    let answer: Value = response.json().map_err(|_| transient())?;
    let access = answer.get("access_token").and_then(Value::as_str).ok_or_else(transient)?;
    let expires_in = answer.get("expires_in").and_then(Value::as_i64).ok_or_else(transient)?;
    let oauth =
        data.get_mut("claudeAiOauth").and_then(Value::as_object_mut).ok_or_else(transient)?;
    oauth.insert("accessToken".into(), Value::from(access));
    oauth.insert("expiresAt".into(), Value::from(now_ms + expires_in * 1000));
    if let Some(r) = answer.get("refresh_token").and_then(Value::as_str).filter(|r| !r.is_empty()) {
        oauth.insert("refreshToken".into(), Value::from(r));
    }
    if let Some(scope) = answer.get("scope").and_then(Value::as_str) {
        let scopes: Vec<Value> = scope.split_whitespace().map(Value::from).collect();
        oauth.insert("scopes".into(), Value::Array(scopes));
    }
    Ok(serde_json::to_string(&data).expect("serializes"))
}

fn access_token(credential: &str) -> Option<String> {
    cc::oauth_part(credential)?
        .get("accessToken")?
        .as_str()
        .filter(|t| !t.is_empty())
        .map(String::from)
}

fn expired(credential: &str, now_ms: i64) -> bool {
    cc::expires_at(credential).is_some_and(|e| now_ms + EXPIRY_MARGIN_MS >= e)
}

/// The back-off after `failures` failures in a row, claude-swap's.
fn backoff(kind: &str, failures: u32, retry_after: Option<f64>) -> f64 {
    let n = failures.max(1);
    if kind == "http-429" {
        let grown = (360.0 * 1.5f64.powi(n as i32 - 1)).min(1800.0);
        let asked = match retry_after {
            Some(ra) if ra <= 0.0 => 300.0,
            Some(ra) if ra > 600.0 => ra + 900.0,
            Some(ra) => ra,
            None => 0.0,
        };
        return asked.max(grown).min(4500.0);
    }
    let computed = (30.0 * 2f64.powi(n as i32 - 1)).min(600.0);
    retry_after.map_or(computed, |ra| ra.max(computed)).min(3600.0)
}

/// What one pass did, for the caller to show.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pass {
    /// Accounts read now.
    pub read: Vec<u32>,
    /// Accounts whose read failed now, with the error kind.
    pub failed: Vec<(u32, String)>,
}

/// Reads every account whose reading is due: older than
/// [`REUSE_SECONDS`], not backing off, not already being read by another
/// process, and not waiting for a new login. With `force`, the age and the
/// back-off do not hold an account back (the owner pressed Làm mới, or a
/// switch needs the numbers now); a login that needs a new sign-in still is.
/// Without `may_refresh` no token is refreshed: an account whose token
/// expired waits. The rotation passes that while an older switcher still
/// runs, which keeps its own copies of the same logins.
pub fn read_due(store: &Store, endpoints: &Endpoints, force: bool, may_refresh: bool) -> Pass {
    let now = super::epoch(OffsetDateTime::now_utc());
    let Ok(list) = store.load() else { return Pass::default() };
    let active = switch::active_slot(store);

    // Claim the accounts to read, so another process does not read them too.
    let mut due = Vec::new();
    if let Ok(_app) = AppLock::take(&store.paths) {
        let mut cache = load(store);
        for account in &list.accounts {
            let key = account.slot.to_string();
            let row = cache.accounts.entry(key).or_default();
            if row.identity != account.identity {
                *row = Row { identity: account.identity.clone(), ..Row::default() };
            }
            let claimed = row.claim_until.is_some_and(|c| c > now);
            let young = row.age(now).is_some_and(|a| a < REUSE_SECONDS);
            let waiting = row.backoff_until.is_some_and(|b| b > now);
            if claimed || row.relogin || (!force && (young || waiting)) {
                continue;
            }
            row.claim_until = Some(now + CLAIM_SECONDS);
            due.push(account.clone());
        }
        // The account in use first.
        due.sort_by_key(|a| (Some(a.slot) != active, a.slot));
        if save(store, &cache).is_err() {
            return Pass::default();
        }
    }

    let mut results: Vec<(u32, Result<Reading, Failure>)> = Vec::new();
    for (i, account) in due.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(STAGGER);
        }
        let result = if Some(account.slot) == active {
            read_active(store, endpoints)
        } else {
            read_stored(store, endpoints, account.slot, account.kind, may_refresh)
        };
        results.push((account.slot, result));
    }

    let mut pass = Pass::default();
    let Ok(_app) = AppLock::take(&store.paths) else { return pass };
    let mut cache = load(store);
    let now = super::epoch(OffsetDateTime::now_utc());
    for (slot, result) in results {
        let Some(row) = cache.accounts.get_mut(&slot.to_string()) else { continue };
        row.claim_until = None;
        row.last_attempt_at = Some(now);
        match result {
            Ok(reading) => {
                row.last_good = Some(reading);
                row.fetched_at = Some(now);
                row.failures = 0;
                row.last_error = None;
                row.backoff_until = None;
                row.relogin = false;
                pass.read.push(slot);
            }
            Err(f) => {
                row.failures += 1;
                row.relogin = matches!(f.kind.as_str(), "invalid_grant" | "no_refresh_token");
                row.backoff_until = Some(now + backoff(&f.kind, row.failures, f.retry_after));
                row.last_error = Some(f.kind.clone());
                pass.failed.push((slot, f.kind));
            }
        }
    }
    // Claims of accounts this pass did not get to are let go.
    for account in &due {
        if let Some(row) = cache.accounts.get_mut(&account.slot.to_string()) {
            row.claim_until = None;
        }
    }
    let _ = save(store, &cache);
    pass
}

/// The account Claude Code is on, with its live token, read-only.
fn read_active(store: &Store, endpoints: &Endpoints) -> Result<Reading, Failure> {
    let credential = cc::read_credential(&store.paths, &store.security)
        .map_err(|_| Failure { kind: "keychain_unavailable".into(), retry_after: None })?
        .ok_or(Failure { kind: "no-access-token".into(), retry_after: None })?;
    let now_ms = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    if cc::expires_at(&credential).is_some_and(|e| now_ms >= e) {
        // Claude Code refreshes its own login when it next runs.
        return Err(Failure { kind: "token_expired".into(), retry_after: None });
    }
    let token = access_token(&credential)
        .ok_or(Failure { kind: "no-access-token".into(), retry_after: None })?;
    fetch_usage(endpoints, &token)
}

/// Another account, with the app's copy of its login, refreshed first when
/// it is about to expire, and once more after a 401.
fn read_stored(
    store: &Store,
    endpoints: &Endpoints,
    slot: u32,
    kind: Kind,
    may_refresh: bool,
) -> Result<Reading, Failure> {
    let keychain = |_| Failure { kind: "keychain_unavailable".into(), retry_after: None };
    let credential = {
        let _app = AppLock::take(&store.paths)
            .map_err(|_| Failure { kind: "busy".into(), retry_after: None })?;
        session::adopt_newer(store, slot);
        store.credential(slot).map_err(keychain)?
    }
    .ok_or(Failure { kind: "no_credentials".into(), retry_after: None })?;
    let now_ms = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    let credential = if kind == Kind::Login && expired(&credential, now_ms) {
        if !may_refresh {
            return Err(Failure { kind: "token_expired".into(), retry_after: None });
        }
        refresh_stored(store, endpoints, slot, &credential)?
    } else {
        credential
    };
    let token = access_token(&credential)
        .ok_or(Failure { kind: "no-access-token".into(), retry_after: None })?;
    match fetch_usage(endpoints, &token) {
        Err(f) if f.kind == "http-401" && kind == Kind::Login && may_refresh => {
            let fresh = refresh_stored(store, endpoints, slot, &credential)?;
            let token = access_token(&fresh)
                .ok_or(Failure { kind: "no-access-token".into(), retry_after: None })?;
            fetch_usage(endpoints, &token)
        }
        other => other,
    }
}

/// Refreshes the app's copy of `slot`'s login under the app's lock. When the
/// stored copy is no longer `seen` (another process refreshed it), that copy
/// is used instead: a refresh token is never sent twice.
fn refresh_stored(
    store: &Store,
    endpoints: &Endpoints,
    slot: u32,
    seen: &str,
) -> Result<String, Failure> {
    let _app = AppLock::take(&store.paths)
        .map_err(|_| Failure { kind: "busy".into(), retry_after: None })?;
    let current = store
        .credential(slot)
        .map_err(|_| Failure { kind: "keychain_unavailable".into(), retry_after: None })?
        .ok_or(Failure { kind: "no_credentials".into(), retry_after: None })?;
    let now_ms = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    if cc::refresh_token(&current) != cc::refresh_token(seen) && !expired(&current, now_ms) {
        return Ok(current);
    }
    let fresh = refresh(endpoints, &current, now_ms)?;
    store
        .set_credential(slot, &fresh)
        .map_err(|_| Failure { kind: "keychain_unavailable".into(), retry_after: None })?;
    session::refresh_profile(store, slot);
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_answer_reads_as_claude_swap_reads_it() {
        let data = json!({
            "five_hour": {"utilization": 38.0, "resets_at": "2030-01-15T16:40:00Z"},
            "seven_day": null,
            "seven_day_opus": {"utilization": 99.0},
            "limits": [
                {"scope": {"model": {"display_name": "Fable"}}, "percent": 11, "resets_at": "2030-01-20T19:00:00Z"},
                {"scope": {}, "percent": 5}
            ]
        });
        let r = parse(&data).unwrap();
        assert_eq!(
            r.five_hour,
            Some(Win { pct: 38.0, resets_at: Some("2030-01-15T16:40:00Z".into()) })
        );
        assert_eq!(r.seven_day, None);
        assert_eq!(r.fable().map(|f| f.pct), Some(11.0));
        assert_eq!(r.scoped.len(), 1);
        assert_eq!(parse(&json!({})), None);
    }

    #[test]
    fn backing_off_grows_and_honours_retry_after() {
        assert_eq!(backoff("timeout", 1, None), 30.0);
        assert_eq!(backoff("timeout", 6, None), 600.0);
        assert_eq!(backoff("http-429", 1, None), 360.0);
        assert_eq!(backoff("http-429", 1, Some(0.0)), 360.0);
        assert_eq!(backoff("http-429", 1, Some(1200.0)), 2100.0);
        assert_eq!(backoff("http-429", 9, None), 1800.0);
    }
}
