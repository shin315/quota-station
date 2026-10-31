// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Kun Chen (quota-axi)
// SPDX-License-Identifier: MIT

// [`read`] is ported from quota-axi's `src/providers/claude.ts` (MIT,
// https://github.com/kunchenguid/quota-axi, 0.1.66): Claude Code's login
// from its Keychain item, then from `~/.claude/.credentials.json`, the
// Keychain first; the item's value read only once the owner allowed it, the
// item otherwise only checked for (`keychain_prompt_required`); the usage
// endpoint with the OAuth beta header; the windows (the `limits` list when
// there is one, else the five-hour, weekly and Opus windows, and extra
// usage); a 401 as a sign-in, unless the stored token's own time ran out
// and Claude Code can renew it; a 429 as a rate limit; a Keychain that
// could not be read never taken for a sign-out; the error words. Changes:
// the token is never refreshed; `CLAUDE_CODE_OAUTH_TOKEN` and other
// profiles are not read (an app opened from Finder has no such variable);
// the identity and plan are not asked for; the owner's permission is the
// app's own: a read that macOS holds back stops the quiet reads until the
// owner presses the button.

//! Claude's row, two ways:
//!
//! - [`read`], on a Mac where the app keeps no Claude account: Claude
//!   Code's own login, read-only. The token goes into memory for the one
//!   request; it is never refreshed, logged or kept.
//! - [`own`], where the app keeps the accounts: the row from the app's own
//!   readings (`claude/usage.json`), with every account listed. It reads
//!   nothing more, and no Keychain.
// @private:begin
//!   With the `author-tools` feature also the rotation's room.
// @private:end

use std::time::Duration;

use serde_json::Value;
use time::OffsetDateTime;

use super::cache::Cache;
use super::{
    iso, semantics, slugify, status_for_error, text, ClaudeAccounts, Effective, Provider,
    ProviderId, Response, Scope, Slot, Window,
};
#[cfg(feature = "author-tools")]
use super::{Expire, RoomPool, RoomSlot, Rotation};
use crate::app_dir::AppDir;
use crate::claude::cc::{self, ACTIVE_SERVICE};
use crate::claude::keychain::{self, user_account, KeychainError};
use crate::claude::store::{Account, List};
use crate::claude::usage::{self, Endpoints, Reading};
use crate::claude::Paths;
use crate::local_time;
use crate::os::SecretStore;
#[cfg(feature = "author-tools")]
use crate::rotation::Rules;
use crate::time_util::parse_ts;

const TIMEOUT: Duration = Duration::from_secs(15);
/// The read the owner starts waits this long for their answer to macOS.
pub const GRANT_TIMEOUT: Duration = Duration::from_secs(60);
const USER_AGENT: &str = "quota-station/1.0";
/// In the app's `quota` folder while the quiet reads leave the Keychain
/// item's value alone.
const HOLD: &str = "claude-keychain-hold";
pub const PROMPT_REQUIRED: &str = "keychain_prompt_required";
/// The row's reason while Claude Code's Keychain item waits for the owner.
pub const ACCESS_REQUIRED: &str = "keychain_access_required";
pub const SIGN_IN_REQUIRED: &str = "Claude sign-in required";
pub const TOKEN_EXPIRED: &str = "Claude access token expired";
const UNAVAILABLE: &str = "Claude quota unavailable";
const FIVE_HOURS: f64 = 18_000.0;
const WEEK: f64 = 604_800.0;
/// A reading older than this is stale.
const FRESH_SECONDS: f64 = crate::claude::usage::FRESH_SECONDS as f64;
#[cfg(feature = "author-tools")]
const ACCOUNT_WINDOWS: [&str; 2] = ["five_hour", "seven_day"];

// ===== Claude Code's own login =====

/// What a read of Claude Code's own login uses.
pub struct Live<'a> {
    pub paths: &'a Paths,
    pub security: &'a dyn SecretStore,
    pub endpoints: &'a Endpoints,
    /// The app's folder: the reading cache and the Keychain hold.
    pub app: &'a AppDir,
}

/// Whether a read may wait for macOS to ask the owner about the Keychain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keychain {
    /// The timed reads: a read macOS holds back is not tried again until
    /// the owner says so.
    Quiet,
    /// The owner pressed the button: wait for their answer.
    Grant,
}

struct Credential {
    access: String,
    /// The stored expiry has passed: advice only, the vendor decides.
    expired: bool,
    refreshable: bool,
}

enum Store {
    Found(Credential),
    Missing,
    Invalid,
    /// The Keychain item's value was not read, for this reason.
    Withheld(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
struct Fail {
    status: &'static str,
    error: String,
    /// A sign-out: the last reading belongs to a login that is gone.
    definitive: bool,
    /// The last reading may stand in, as stale.
    stale: bool,
    keychain: bool,
}

impl Fail {
    fn transient(status: &'static str, error: &str) -> Fail {
        Fail { status, error: error.into(), definitive: false, stale: true, keychain: false }
    }

    fn sign_in(error: &str) -> Fail {
        Fail {
            status: "auth_required",
            error: error.into(),
            definitive: true,
            stale: false,
            keychain: false,
        }
    }

    fn keychain(reason: &str) -> Fail {
        Fail { keychain: true, ..Fail::transient(status_for_error(reason), reason) }
    }
}

fn hold(app: &AppDir) -> std::path::PathBuf {
    app.path().join(super::cache::DIR).join(HOLD)
}

fn credential(text: &str, now: OffsetDateTime) -> Store {
    let Some(oauth) = cc::oauth_part(text) else { return Store::Invalid };
    let Some(access) = text_of(&oauth, "accessToken") else { return Store::Invalid };
    let now_ms = (now.unix_timestamp_nanos() / 1_000_000) as i64;
    Store::Found(Credential {
        access,
        expired: oauth.get("expiresAt").and_then(Value::as_i64).is_some_and(|e| e <= now_ms),
        refreshable: text_of(&oauth, "refreshToken").is_some(),
    })
}

fn text_of(v: &Value, key: &str) -> Option<String> {
    text(v.get(key)).filter(|t| !t.trim().is_empty())
}

/// Claude Code's Keychain item: its value when the owner allows it, else
/// only whether it is there.
fn keychain_store(live: &Live, mode: Keychain, now: OffsetDateTime) -> Store {
    let held = hold(live.app);
    let account = user_account();
    if mode == Keychain::Quiet && held.exists() {
        return match live.security.exists(ACTIVE_SERVICE, &account) {
            Ok(true) => Store::Withheld(PROMPT_REQUIRED),
            Ok(false) => Store::Missing,
            Err(_) => Store::Withheld("keychain_presence_check_failed"),
        };
    }
    let wait = if mode == Keychain::Grant { GRANT_TIMEOUT } else { keychain::TIMEOUT };
    let read = live.security.get_within(ACTIVE_SERVICE, &account, wait);
    let held_back = |reason| {
        let _ = live.app.write_private(&held, "1\n");
        Store::Withheld(reason)
    };
    match read {
        Ok(value) => {
            let _ = live.app.remove(&held);
            match value.filter(|v| !v.trim().is_empty()) {
                Some(v) => credential(&v, now),
                None => Store::Missing,
            }
        }
        Err(KeychainError::Timeout) => held_back("keychain_prompt_timeout"),
        Err(KeychainError::Failed { .. }) => held_back("keychain_access_denied"),
        Err(KeychainError::Spawn(_)) => Store::Withheld("keychain_unreachable"),
    }
}

fn file_store(live: &Live, now: OffsetDateTime) -> Store {
    match std::fs::read_to_string(live.paths.claude_home().join(".credentials.json")) {
        Ok(t) if !t.trim().is_empty() => credential(&t, now),
        Ok(_) => Store::Invalid,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Store::Missing,
        Err(_) => Store::Invalid,
    }
}

enum Answer {
    Windows(Vec<Window>),
    Rejected,
    Failed(Fail),
}

fn fetch(endpoints: &Endpoints, token: &str) -> Answer {
    let client = match reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
    {
        Ok(c) => c,
        Err(_) => return Answer::Failed(Fail::transient("error", UNAVAILABLE)),
    };
    let response = match client
        .get(endpoints.usage())
        .bearer_auth(token)
        .header("anthropic-beta", usage::BETA)
        .header("accept", "application/json")
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            let words = if e.is_timeout() { "Claude quota request timed out" } else { UNAVAILABLE };
            return Answer::Failed(Fail::transient("error", words));
        }
    };
    match response.status().as_u16() {
        // A 403 can be a network policy: no sign-out on its word.
        401 => return Answer::Rejected,
        429 => {
            return Answer::Failed(Fail::transient(
                "rate_limited",
                "Claude quota endpoint rate limited",
            ))
        }
        s if !(200..300).contains(&s) => {
            return Answer::Failed(Fail::transient("error", &format!("{UNAVAILABLE} ({s})")))
        }
        _ => {}
    }
    match response.json::<Value>().ok().map(|v| normalize(&v)) {
        Some(w) if !w.is_empty() => Answer::Windows(w),
        _ => Answer::Failed(Fail::transient("error", UNAVAILABLE)),
    }
}

/// Reads Claude Code's own login. A reading younger than `max_age` seconds
/// answers without a request, unless the owner asked for the Keychain; a
/// failed read shows the last reading as stale while it can stand.
pub fn read(
    live: &Live,
    mode: Keychain,
    cache: &Cache,
    max_age: i64,
    now: OffsetDateTime,
) -> Response {
    if mode == Keychain::Quiet {
        if let Some(reused) = cache.reusable(None, max_age, now) {
            return Response::new(now, vec![reused]);
        }
    }
    let stores = [keychain_store(live, mode, now), file_store(live, now)];
    let mut definitive: Option<Fail> = None;
    let mut transient: Option<Fail> = None;
    for c in stores.iter().filter_map(|s| match s {
        Store::Found(c) => Some(c),
        _ => None,
    }) {
        match fetch(live.endpoints, &c.access) {
            Answer::Windows(windows) => {
                let row = Provider::fresh(ProviderId::Claude, windows, now);
                cache.save(&row, None);
                return Response::new(now, vec![row]);
            }
            Answer::Rejected => {
                // A token whose own time ran out is renewed by Claude Code.
                let f = if c.expired && c.refreshable {
                    Fail::transient("unavailable", TOKEN_EXPIRED)
                } else {
                    Fail::sign_in(SIGN_IN_REQUIRED)
                };
                definitive.get_or_insert(f);
            }
            Answer::Failed(f) => {
                transient = Some(f);
                break;
            }
        }
    }
    let withheld = stores.iter().find_map(|s| match s {
        Store::Withheld(reason) => Some(*reason),
        _ => None,
    });
    let found = stores.iter().any(|s| matches!(s, Store::Found(_)));
    if !found {
        match withheld {
            Some(reason) => transient = Some(Fail::keychain(reason)),
            None => {
                let invalid = stores.iter().any(|s| matches!(s, Store::Invalid));
                definitive = Some(Fail::sign_in(if invalid {
                    "credentials_invalid"
                } else {
                    "credentials_missing"
                }));
            }
        }
    }
    let mut failure =
        transient.or(definitive).unwrap_or_else(|| Fail::transient("error", UNAVAILABLE));
    // A Keychain item never read never saw the live session: a 401 from the
    // file beside it is no sign-out.
    if let Some(reason) = withheld {
        if failure.definitive || failure.error == TOKEN_EXPIRED {
            failure = Fail::keychain(reason);
        }
    }
    if failure.definitive {
        cache.retire();
    }
    let reason = (withheld.is_some() && failure.keychain).then(|| ACCESS_REQUIRED.to_string());
    if failure.stale {
        if let Some(mut stale) = cache.stale(&failure.error, now) {
            stale.state.reason = reason;
            return Response::new(now, vec![stale]);
        }
    }
    let mut row = Provider::failed(ProviderId::Claude, failure.status, &failure.error);
    row.state.reason = reason;
    row.not_set_up = stores.iter().all(|s| matches!(s, Store::Missing));
    Response::new(now, vec![row])
}

/// The owner allowed the Keychain: the quiet reads may read its value again.
pub fn hold_lifted(app: &AppDir) -> bool {
    !hold(app).exists()
}

/// The usage answer as quota-axi reads it.
pub fn normalize(data: &Value) -> Vec<Window> {
    let limits: Vec<Window> = data
        .get("limits")
        .and_then(Value::as_array)
        .map(|l| l.iter().filter_map(limit).collect())
        .unwrap_or_default();
    let mut windows = if limits.is_empty() {
        [
            ("five_hour", "session", FIVE_HOURS),
            ("seven_day", "weekly", WEEK),
            ("seven_day_opus", "model", WEEK),
        ]
        .into_iter()
        .filter_map(|(id, kind, seconds)| {
            let raw = data.get(id).filter(|v| v.is_object())?;
            let used = raw.get("utilization")?.as_f64()?;
            let resets = text(raw.get("resets_at")).or_else(|| text(raw.get("reset_at")));
            Some(Window::used(id, kind, used, resets).seconds(seconds))
        })
        .collect()
    } else {
        limits
    };
    windows.extend(extra_usage(data.get("extra_usage")));
    windows
}

fn limit(raw: &Value) -> Option<Window> {
    let percent = raw.get("percent")?.as_f64()?;
    let resets = text(raw.get("resets_at"));
    if let Some(model) = raw.pointer("/scope/model").filter(|m| m.is_object()) {
        if let Some(name) = text(model.get("display_name")) {
            let key = text(model.get("id")).unwrap_or_else(|| slugify(&name));
            return Some(
                Window::used(&format!("model:{key}"), "model", percent, resets).seconds(WEEK),
            );
        }
    }
    match raw.get("group").and_then(Value::as_str) {
        Some("session") => {
            Some(Window::used("five_hour", "session", percent, resets).seconds(FIVE_HOURS))
        }
        Some("weekly") => Some(Window::used("seven_day", "weekly", percent, resets).seconds(WEEK)),
        _ => {
            let id = text(raw.get("kind")).unwrap_or_else(|| "limit".into());
            Some(Window::used(&id, "unknown", percent, resets))
        }
    }
}

fn extra_usage(raw: Option<&Value>) -> Option<Window> {
    let raw = raw.filter(|v| v.is_object())?;
    if raw.get("is_enabled").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let places = raw.get("decimal_places").and_then(Value::as_f64).unwrap_or(2.0);
    let divisor = 10f64.powf(places);
    let spent = raw.get("used_credits").and_then(Value::as_f64).map(|c| c / divisor);
    let cap = raw.get("monthly_limit").and_then(Value::as_f64).map(|c| c / divisor);
    let used = raw.get("utilization").and_then(Value::as_f64).or(match (spent, cap) {
        (Some(s), Some(c)) if c > 0.0 => Some(s / c * 100.0),
        _ => None,
    });
    Some(match used {
        Some(u) => Window::used("extra_usage", "credits", u, None),
        None => {
            Window { id: "extra_usage".into(), kind: Some("credits".into()), ..Window::default() }
        }
    })
}

// ===== The accounts the app keeps =====

/// What the row of a Mac where the app keeps the accounts is built from.
pub struct Own<'a> {
    pub list: &'a List,
    pub usage: &'a usage::Cache,
    /// The account Claude Code is on.
    pub active: Option<u32>,
    /// The rotation file the app follows, when it reads.
    #[cfg(feature = "author-tools")]
    pub rules: Option<&'a Rules>,
}

fn at_epoch(seconds: f64) -> Option<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp_nanos((seconds * 1e9) as i128).ok()
}

/// An account's reading as windows: the five-hour, the weekly, and each
/// model's weekly window as `model:<name>`.
fn windows_of(reading: &Reading) -> Vec<Window> {
    let mut out = Vec::new();
    if let Some(w) = &reading.five_hour {
        out.push(
            Window::used("five_hour", "session", w.pct, w.resets_at.clone()).seconds(FIVE_HOURS),
        );
    }
    if let Some(w) = &reading.seven_day {
        out.push(Window::used("seven_day", "weekly", w.pct, w.resets_at.clone()).seconds(WEEK));
    }
    for s in &reading.scoped {
        let key = slugify(&s.name);
        let id = format!("model:{key}");
        if key.is_empty() || out.iter().any(|w| w.id == id) {
            continue;
        }
        out.push(Window::used(&id, "model", s.pct, s.resets_at.clone()).seconds(WEEK));
    }
    out
}

fn instant(text: &str) -> Option<OffsetDateTime> {
    parse_ts(text).or_else(|| local_time::instant(text))
}

/// The latest reset among `windows`, by time.
fn latest_reset(windows: &[&Window]) -> Option<String> {
    windows
        .iter()
        .filter_map(|w| {
            let at = w.resets_at.as_deref()?;
            Some((instant(at)?, at.to_string()))
        })
        .max_by_key(|(t, _)| *t)
        .map(|(_, s)| s)
}

/// One effective scope of an account: what it has left, which windows hold
/// it there and when the latest of them resets; `reset_pending` once one
/// of its windows reset after the reading.
fn scope(e: &Effective, windows: &[Window], now: OffsetDateTime) -> Scope {
    let bounds: Vec<&Window> = windows.iter().filter(|w| e.bounded_by.contains(&w.id)).collect();
    if bounds.iter().any(|w| w.reset_passed(now)) {
        return Scope {
            scope: e.scope.clone(),
            status: "reset_pending".into(),
            ..Scope::default()
        };
    }
    let Some(left) = e.effective_percent_remaining else {
        return Scope { scope: e.scope.clone(), status: "unknown".into(), ..Scope::default() };
    };
    let limiting: Vec<&Window> =
        windows.iter().filter(|w| e.limiting_window_ids.contains(&w.id)).collect();
    Scope {
        scope: e.scope.clone(),
        status: "known".into(),
        effective_percent_remaining: Some(left),
        limited_by: (!limiting.is_empty())
            .then(|| limiting.iter().map(|w| w.id.as_str()).collect::<Vec<_>>().join(" + ")),
        resets_at: latest_reset(&limiting),
    }
}

fn slot_of(a: &Account, row: Option<&usage::Row>, active: bool, now: OffsetDateTime) -> Slot {
    let base = Slot {
        slot: a.slot,
        active,
        #[cfg(feature = "author-tools")]
        disabled: a.disabled,
        ..Slot::default()
    };
    let no_data = |error: Option<String>| Slot {
        status: "no_data".into(),
        error: error.or_else(|| Some("no_reading".into())),
        ..base.clone()
    };
    let Some(row) = row else { return no_data(None) };
    let reading = row.last_good.as_ref().zip(row.fetched_at.and_then(at_epoch));
    let Some((reading, fetched)) = reading else { return no_data(row.last_error.clone()) };
    let windows = windows_of(reading);
    if windows.is_empty() {
        return no_data(row.last_error.clone());
    }
    let age = (now - fetched).as_seconds_f64().max(0.0).round();
    let scopes: Vec<Scope> = semantics::claude(&windows)
        .effective_availability
        .iter()
        .map(|e| scope(e, &windows, now))
        .collect();
    let headline = scopes.iter().find(|s| s.scope == "all_models").cloned();
    let status = match &headline {
        Some(h) if h.status == "reset_pending" => "reset_pending",
        _ if age <= FRESH_SECONDS => "fresh",
        _ => "stale",
    };
    Slot {
        status: status.into(),
        age_seconds: Some(age),
        fetched_at: Some(iso(fetched)),
        effective_percent_remaining: headline.as_ref().and_then(|h| h.effective_percent_remaining),
        limited_by: headline.as_ref().and_then(|h| h.limited_by.clone()),
        resets_at: headline.as_ref().and_then(|h| h.resets_at.clone()),
        scopes: scopes.into_iter().filter(|s| s.scope != "all_models").collect(),
        error: row.last_error.clone(),
        windows,
        ..base
    }
}

/// The row of a Mac where the app keeps the Claude accounts: the account in
/// use's own reading as the row's windows (stale once older than 15
/// minutes or past a reset), and every account as a slot.
// @private:begin
/// With the `author-tools` feature also the rotation file's keys and the
/// rotation's room.
// @private:end
pub fn own(input: &Own, now: OffsetDateTime) -> Response {
    let row_of = |a: &Account| input.usage.row(a.slot).filter(|r| r.identity == a.identity);
    let active = input.active.filter(|s| input.list.get(*s).is_some());
    let slots: Vec<Slot> = input
        .list
        .accounts
        .iter()
        .map(|a| slot_of(a, row_of(a), active == Some(a.slot), now))
        .collect();

    let reading = active.and_then(|s| input.list.get(s)).and_then(row_of).and_then(|r| {
        let fetched = r.fetched_at.and_then(at_epoch)?;
        Some((r, windows_of(r.last_good.as_ref()?), fetched))
    });
    let mut row = match reading {
        Some((r, windows, fetched)) if !windows.is_empty() => {
            let mut row = Provider::fresh(ProviderId::Claude, windows, fetched);
            let old = (now - fetched).as_seconds_f64() > FRESH_SECONDS;
            let reset = row.windows.iter().any(|w| w.reset_passed(now));
            if old || reset || r.relogin {
                row.state.status = "stale".into();
                row.state.error = Some(r.last_error.clone().unwrap_or_else(|| {
                    if reset { "reset_since_reading" } else { "reading_old" }.into()
                }));
            }
            semantics::apply(&mut row, now);
            row
        }
        Some((r, _, _)) => Provider::failed(
            ProviderId::Claude,
            "unavailable",
            r.last_error.as_deref().unwrap_or("no_reading"),
        ),
        None if active.is_none() => {
            Provider::failed(ProviderId::Claude, "unavailable", "active_account_unknown")
        }
        None => {
            let error = active
                .and_then(|s| input.list.get(s))
                .and_then(row_of)
                .and_then(|r| r.last_error.clone())
                .unwrap_or_else(|| "no_reading".into());
            Provider::failed(ProviderId::Claude, "unavailable", &error)
        }
    };
    #[cfg(feature = "author-tools")]
    let (pool, pool_reason) = match input.rules {
        Some(rules) => match room(rules, &row, &slots, active, now) {
            Ok(pool) => (Some(pool), None),
            Err(reason) => (None, Some(reason.to_string())),
        },
        None => (None, None),
    };
    row.claude_accounts = Some(ClaudeAccounts {
        active_slot: active,
        slots,
        #[cfg(feature = "author-tools")]
        rotation: input.rules.map(rotation_of),
        #[cfg(feature = "author-tools")]
        pool,
        #[cfg(feature = "author-tools")]
        pool_reason,
    });
    Response::new(now, vec![row])
}

#[cfg(feature = "author-tools")]
fn rotation_of(rules: &Rules) -> Rotation {
    let mut expire: Vec<Expire> = rules
        .expire
        .iter()
        .filter_map(|e| Some(Expire { slot: e.slot, at: iso(instant(&e.at)?) }))
        .collect();
    expire.sort_by_key(|e| e.slot);
    Rotation {
        team: rules.team.clone(),
        personal: rules.personal,
        personal_hours: Some(format!("{}-{}", rules.personal_from, rules.personal_to)),
        full: rules.full,
        expire,
    }
}

#[cfg(feature = "author-tools")]
struct Member<'a> {
    slot: u32,
    windows: &'a [Window],
    leaves: Option<OffsetDateTime>,
}

/// Percent used now: none past its reset.
#[cfg(feature = "author-tools")]
fn used_now(w: &Window, now: OffsetDateTime) -> Option<f64> {
    if w.reset_passed(now) {
        return Some(0.0);
    }
    w.percent_used.or(w.percent_remaining.map(|r| 100.0 - r))
}

#[cfg(feature = "author-tools")]
fn round(v: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (v * f + 0.5).floor() / f
}

/// The room the rotation has in every model's scope: for each account it
/// may move to now (the Team accounts before their end, the personal one in
/// its hours, each with a reading at most 15 minutes old, the one in use
/// from the row), the room before `full`, as a percent of that room, in the
/// tighter of the five-hour and weekly windows; and the first reset that
/// gives an account more room. `Err` says why there is no rotation to count.
#[cfg(feature = "author-tools")]
fn room(
    rules: &Rules,
    row: &Provider,
    slots: &[Slot],
    active: Option<u32>,
    now: OffsetDateTime,
) -> Result<RoomPool, &'static str> {
    if row.state.status != "fresh" {
        return Err("claude_row_not_fresh");
    }
    let active = active.ok_or("active_slot_unknown")?;
    let known = row.quota_semantics.as_ref().is_some_and(|s| s.status == "known");
    if !known {
        return Err("claude_row_unmeasured");
    }
    let offset = local_time::offset_at(now);
    let local = now.to_offset(offset);
    let in_hours = rules.personal_from <= local.hour() && local.hour() <= rules.personal_to;
    let hours_end = local.replace_time(time::Time::MIDNIGHT)
        + time::Duration::hours(i64::from(rules.personal_to) + 1);
    let mut candidates: Vec<(u32, bool)> = rules.team.iter().map(|s| (*s, false)).collect();
    if let Some(p) = rules.personal.filter(|p| !rules.team.contains(p)) {
        candidates.push((p, true));
    }
    let mut members = Vec::new();
    for (slot, personal) in candidates {
        let ends = rules.expire.iter().find(|e| e.slot == slot).and_then(|e| instant(&e.at));
        if ends.is_some_and(|e| now >= e) || (personal && !in_hours) {
            continue;
        }
        let Some(s) = slots.iter().find(|s| s.slot == slot) else { continue };
        let windows: &[Window] = if slot == active {
            &row.windows
        } else if s.status == "stale" || s.status == "no_data" {
            continue;
        } else {
            &s.windows
        };
        if windows.is_empty() {
            continue;
        }
        let leaves = [ends, personal.then_some(hours_end)].into_iter().flatten().min();
        members.push(Member { slot, windows, leaves });
    }
    if !members.iter().any(|m| m.slot != active) {
        return Err("no_other_slot");
    }
    let rooms: Vec<(u32, f64, Option<OffsetDateTime>)> = members
        .iter()
        .map(|m| {
            let (left, refill) = member_room(m, rules.full, now);
            (m.slot, left, refill)
        })
        .collect();
    let total: f64 = rooms.iter().map(|r| r.1).sum();
    let next = rooms.iter().filter_map(|r| r.2).min();
    Ok(RoomPool {
        scope: "all_models".into(),
        members: u32::try_from(members.len()).unwrap_or(u32::MAX),
        slots_of_room: round(total / 100.0, 2),
        effective_percent_remaining: round(total.min(100.0), 1),
        next_refill_at: next.map(iso),
        slots: rooms
            .iter()
            .map(|(slot, left, refill)| RoomSlot {
                slot: *slot,
                room_percent: round(*left, 1),
                refill_at: refill.map(iso),
            })
            .collect(),
    })
}

/// One account's room in percent, and the first reset before it leaves the
/// rotation after which it has more.
#[cfg(feature = "author-tools")]
fn member_room(m: &Member, full: f64, now: OffsetDateTime) -> (f64, Option<OffsetDateTime>) {
    let windows: Vec<&Window> =
        m.windows.iter().filter(|w| ACCOUNT_WINDOWS.contains(&w.id.as_str())).collect();
    let percent: Vec<f64> = windows
        .iter()
        .map(|w| used_now(w, now).map_or(0.0, |u| 100.0 * (full - u).max(0.0) / full))
        .collect();
    let left = percent.iter().copied().fold(f64::INFINITY, f64::min);
    let left = if left.is_finite() { left } else { 0.0 };
    let mut resets: Vec<OffsetDateTime> = windows
        .iter()
        .filter_map(|w| w.resets_at.as_deref().and_then(instant))
        .filter(|t| *t > now)
        .collect();
    resets.sort();
    resets.dedup();
    for reset in resets {
        if m.leaves.is_some_and(|l| reset >= l) {
            break;
        }
        let after = windows
            .iter()
            .zip(&percent)
            .map(|(w, p)| {
                let r = w.resets_at.as_deref().and_then(instant);
                if r.is_some_and(|r| r > now && r <= reset) {
                    100.0
                } else {
                    *p
                }
            })
            .fold(f64::INFINITY, f64::min);
        if after > left {
            return (left, Some(reset));
        }
    }
    (left, None)
}

/// [`own`] from the app's files as they are now: its list, its readings and
/// the account Claude Code is on. `None` while the app keeps no account.
// @private:begin
/// With the `author-tools` feature also the rotation file it follows.
// @private:end
pub fn own_now(store: &crate::claude::store::Store, now: OffsetDateTime) -> Option<Response> {
    if !store.paths.in_use() {
        return None;
    }
    let list = store.load().ok()?;
    let usage = usage::load(store);
    let active = crate::claude::switch::active_slot(store);
    #[cfg(feature = "author-tools")]
    let rules = match crate::rotation::load(&store.paths.rotation_file()) {
        crate::rotation::FileState::Ok { parsed, .. } => Some(parsed.rules),
        _ => None,
    };
    let input = Own {
        list: &list,
        usage: &usage,
        active,
        #[cfg(feature = "author-tools")]
        rules: rules.as_ref(),
    };
    Some(own(&input, now))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "author-tools")]
    use crate::quota::parse_iso;
    use serde_json::json;

    #[test]
    fn the_limits_list_wins_and_extra_usage_is_kept() {
        let data = json!({
            "five_hour": {"utilization": 99.0},
            "limits": [
                {"group": "session", "percent": 12.4, "resets_at": "2030-01-15T16:40:00Z"},
                {"group": "weekly", "percent": 40},
                {"scope": {"model": {"display_name": "Fable"}}, "percent": 11.5}
            ],
            "extra_usage": {"is_enabled": true, "used_credits": 500, "monthly_limit": 2000}
        });
        let w = normalize(&data);
        let ids: Vec<&str> = w.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["five_hour", "seven_day", "model:fable", "extra_usage"]);
        assert_eq!(w[0].percent_remaining, Some(88.0));
        assert_eq!(w[2].percent_remaining, Some(88.0), "11.5 used rounds to 12");
        assert_eq!(w[3].percent_used, Some(25.0));
    }

    #[test]
    fn without_limits_the_fixed_windows_read() {
        let data = json!({
            "five_hour": {"utilization": 38.0, "resets_at": "2030-01-15T16:40:00Z"},
            "seven_day": null,
            "seven_day_opus": {"utilization": 5.0}
        });
        let w = normalize(&data);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].resets_at.as_deref(), Some("2030-01-15T16:40:00Z"));
        assert_eq!((w[1].id.as_str(), w[1].kind.as_deref()), ("seven_day_opus", Some("model")));
    }

    #[cfg(feature = "author-tools")]
    fn window(id: &str, used: f64, resets: Option<&str>) -> Window {
        let kind = if id == "five_hour" { "session" } else { "weekly" };
        Window::used(id, kind, used, resets.map(String::from))
    }

    #[cfg(feature = "author-tools")]
    #[test]
    fn an_accounts_room_is_its_tighter_window_before_full() {
        let now = parse_iso("2030-01-15T12:00:00Z").unwrap();
        let windows = [
            window("five_hour", 45.0, Some("2030-01-15T14:00:00Z")),
            window("seven_day", 72.0, Some("2030-01-18T00:00:00Z")),
        ];
        let m = Member { slot: 2, windows: &windows, leaves: None };
        let (left, refill) = member_room(&m, 90.0, now);
        assert_eq!(round(left, 1), 20.0, "weekly: 18 points of 90");
        // The five-hour reset gives no more room: the weekly window holds.
        assert_eq!(refill, parse_iso("2030-01-18T00:00:00Z"));
        let leaving = Member { leaves: parse_iso("2030-01-16T00:00:00Z"), ..m };
        assert_eq!(member_room(&leaving, 90.0, now).1, None, "it leaves before");
        let past = [window("five_hour", 95.0, Some("2030-01-15T11:00:00Z"))];
        let m = Member { slot: 3, windows: &past, leaves: None };
        assert_eq!(member_room(&m, 90.0, now).0, 100.0, "a passed reset is room again");
    }
}
