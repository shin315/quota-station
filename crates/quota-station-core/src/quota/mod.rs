// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Kun Chen (quota-axi)
// SPDX-License-Identifier: MIT

// The reading model and the way a reading is told are quota-axi's
// (`src/types.ts`, `src/lib/time.ts`, `src/providers/common.ts`, MIT,
// https://github.com/kunchenguid/quota-axi, 0.1.66): the window ids, the
// state words, percent used rounded and clamped, percent left as 100 less
// it, and a failed read served from the last good reading as `stale`.

//! The quota the app reads itself, one row per provider: Codex's ChatGPT
//! login ([`codex`]), the Antigravity account signed in ([`agy`]) and, on a
//! Mac where the app keeps no Claude account, Claude Code's own login
//! ([`claude`]). Where the app keeps the Claude accounts the row is built
//! from its own readings of them ([`claude::own`]). [`cache`] keeps each
//! provider's last good reading, so a read within two minutes of the last
//! asks the vendor nothing and a failed read shows the last numbers as stale.
//!
//! Nothing here refreshes, rotates or writes a login: an expired login is
//! shown as one, and the vendor's own tool renews it when it next runs.
//!
//! A row carries only windows, times and error words: no token, no header,
//! and no email (the account a reading names is compared, never kept).

pub mod agy;
pub mod cache;
pub mod claude;
pub mod codex;
pub mod semantics;

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// A provider the app reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    Claude,
    Agy,
    Codex,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Claude => "claude",
            ProviderId::Agy => "agy",
            ProviderId::Codex => "codex",
        }
    }
}

/// One read of one provider: when it was built and its rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub generated_at: String,
    #[serde(default)]
    pub providers: Vec<Provider>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub provider: String,
    /// The logins this row speaks for: Codex's own login is `codex-home`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub account_keys: Vec<String>,
    #[serde(default)]
    pub windows: Vec<Window>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_semantics: Option<QuotaSemantics>,
    #[serde(default)]
    pub state: State,
    /// Nothing of the provider is on this Mac.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub not_set_up: bool,
    /// The Claude accounts the app keeps, with their readings and the
    /// rotation file; only on the Claude row of a Mac where it keeps them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_accounts: Option<ClaudeAccounts>,
    /// The Antigravity accounts the app logs in itself; only in a build
    /// with them, on the Antigravity row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agy_accounts: Option<AgyAccounts>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub id: String,
    /// `session`, `weekly`, `model`, `credits` or `unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_used: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_remaining: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_seconds: Option<f64>,
}

impl Window {
    /// A window of `kind` with `used` percent used, rounded and clamped,
    /// and the percent left that follows.
    pub fn used(id: &str, kind: &str, used: f64, resets_at: Option<String>) -> Window {
        let used = clamp_percent(used);
        Window {
            id: id.to_string(),
            kind: Some(kind.to_string()),
            percent_used: Some(used),
            percent_remaining: Some(percent_remaining(used)),
            resets_at,
            window_seconds: None,
        }
    }

    pub fn seconds(mut self, seconds: f64) -> Window {
        self.window_seconds = Some(seconds);
        self
    }

    /// The window's reset is at or before `now`.
    pub fn reset_passed(&self, now: OffsetDateTime) -> bool {
        self.resets_at.as_deref().and_then(parse_iso).is_some_and(|at| at <= now)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// `fresh`, `stale`, `auth_required`, `rate_limited`, `unavailable` or
    /// `error`.
    #[serde(default)]
    pub status: String,
    /// When the numbers were read; a stale row keeps its reading's time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refreshed_at: Option<String>,
    /// Why the last read failed, in words that hold no secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// A code for the state, when there is one (`keychain_access_required`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl State {
    pub fn stale(&self) -> bool {
        self.status == "stale"
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSemantics {
    /// `known`, `partial` or `unknown`.
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub effective_availability: Vec<Effective>,
}

/// What one scope (every model, one model, one Antigravity group) has
/// left: the smallest percent left among the windows bounding it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effective {
    pub scope: String,
    /// `known` or `unknown`.
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_percent_remaining: Option<f64>,
    #[serde(default)]
    pub bounded_by: Vec<String>,
    #[serde(default)]
    pub limiting_window_ids: Vec<String>,
}

/// One effective scope of a listed account: what it has left, which
/// windows hold it there and when the latest of them resets.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub scope: String,
    /// `known`, `unknown` or `reset_pending`.
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_percent_remaining: Option<f64>,
    /// The limiting windows, `five_hour + seven_day`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limited_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
}

/// The Claude accounts the app keeps, as the row lists them.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAccounts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_slot: Option<u32>,
    #[serde(default)]
    pub slots: Vec<Slot>,
    /// The rotation file the app follows, when it reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,
    /// The rotation's room, when there is a rotation to count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<RoomPool>,
    /// Why the rotation's room is not counted (`claude_row_not_fresh`,
    /// `no_other_slot`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_reason: Option<String>,
}

/// The room the rotation has: each account it may move to now, with the
/// room it has before the switch threshold.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomPool {
    /// `all_models`.
    pub scope: String,
    pub members: u32,
    /// Room left, in fresh accounts (2.3 is two and a third).
    pub slots_of_room: f64,
    /// The room's sum in percent of one account, at most 100.
    pub effective_percent_remaining: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_refill_at: Option<String>,
    #[serde(default)]
    pub slots: Vec<RoomSlot>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSlot {
    pub slot: u32,
    /// 100 is a whole account's worth of room, 0 an account at its threshold.
    pub room_percent: f64,
    /// The first reset that gives it more room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refill_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub slot: u32,
    #[serde(default)]
    pub active: bool,
    /// Held out of the rotation by the owner.
    #[serde(default)]
    pub disabled: bool,
    /// `fresh` (at most 15 minutes old), `stale`, `reset_pending` or `no_data`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    /// Every model's scope: the five-hour and weekly windows together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_percent_remaining: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limited_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    /// The per-model scopes (`model:fable`).
    #[serde(default)]
    pub scopes: Vec<Scope>,
    /// The last read's error kind (`invalid_grant`, `http-429`), or why no
    /// reading could be used (`no_reading`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The account's windows as read, for the rotation's room.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<Window>,
}

/// The rotation file's keys the windows show.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rotation {
    #[serde(default)]
    pub team: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_hours: Option<String>,
    #[serde(default)]
    pub full: f64,
    #[serde(default)]
    pub expire: Vec<Expire>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expire {
    pub slot: u32,
    /// RFC 3339.
    pub at: String,
}

/// The Antigravity accounts the app logs in, as the row lists them.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgyAccounts {
    /// `ok`, or `unavailable` with `reason`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// How the account in use was told: `identity`, `reading` (the live
    /// reading's resets matched its numbers) or `binding` (the account the
    /// app's last switch put on).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_basis: Option<String>,
    #[serde(default)]
    pub accounts: Vec<AgyAccount>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgyAccount {
    /// Its place in the app's list, from 1.
    pub account: u32,
    #[serde(default)]
    pub active: bool,
    /// `fresh`, `stale` or `no_data`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    /// `gemini` and `claude_gpt`.
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

impl Response {
    pub fn new(now: OffsetDateTime, providers: Vec<Provider>) -> Response {
        Response { generated_at: iso(now), providers }
    }

    /// The first row of `provider`.
    pub fn primary(&self, provider: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.provider == provider)
    }

    pub fn primary_mut(&mut self, provider: &str) -> Option<&mut Provider> {
        self.providers.iter_mut().find(|p| p.provider == provider)
    }

    /// The reading `provider`'s row was taken at: its own time, else the
    /// report's.
    pub fn read_at(&self, provider: ProviderId) -> Option<OffsetDateTime> {
        let row = self.primary(provider.as_str())?;
        parse_iso(row.state.refreshed_at.as_deref().unwrap_or(&self.generated_at))
    }
}

impl Provider {
    /// A row with numbers read now.
    pub fn fresh(provider: ProviderId, windows: Vec<Window>, now: OffsetDateTime) -> Provider {
        let mut row = Provider {
            provider: provider.as_str().to_string(),
            windows,
            state: State {
                status: "fresh".into(),
                refreshed_at: Some(iso(now)),
                error: None,
                reason: None,
            },
            ..Provider::default()
        };
        semantics::apply(&mut row, now);
        row
    }

    /// A row whose read failed, with no numbers.
    pub fn failed(provider: ProviderId, status: &str, error: &str) -> Provider {
        Provider {
            provider: provider.as_str().to_string(),
            state: State {
                status: status.to_string(),
                refreshed_at: None,
                error: Some(error.to_string()),
                reason: None,
            },
            ..Provider::default()
        }
    }

    pub fn window(&self, id: &str) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }
}

/// quota-axi's status for a failure's words (`statusFromError`).
pub fn status_for_error(error: &str) -> &'static str {
    let e = error.to_lowercase();
    if e == "keychain_prompt_required"
        || e == "credentials_expired"
        || ["sign-in", "required", "reauth", "access token expired"].iter().any(|w| e.contains(w))
    {
        return "auth_required";
    }
    // `rate.?limit`
    let rate = e.match_indices("rate").any(|(i, _)| {
        let rest = &e[i + 4..];
        let mut skip = rest.chars();
        skip.next();
        rest.starts_with("limit") || skip.as_str().starts_with("limit")
    });
    if rate {
        return "rate_limited";
    }
    "error"
}

/// `Math.round`: halves go up.
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// A percent rounded to a whole number and kept within 0 to 100, as
/// quota-axi keeps every percent.
pub fn clamp_percent(v: f64) -> f64 {
    if !v.is_finite() {
        return 0.0;
    }
    js_round(v).clamp(0.0, 100.0)
}

/// The percent left of a window with `used` percent used.
pub fn percent_remaining(used: f64) -> f64 {
    clamp_percent(100.0 - used)
}

/// `YYYY-MM-DDTHH:MM:SS.sssZ`, as JavaScript's `toISOString` writes it.
pub fn iso(t: OffsetDateTime) -> String {
    let t = t.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        t.millisecond()
    )
}

pub fn parse_iso(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

/// A time as a vendor sends it: Unix seconds, or a date in text. A text
/// that is not a date is kept as it is.
pub fn epoch_or_iso(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Number(n) => {
            let secs = n.as_f64().filter(|s| s.is_finite())?;
            let nanos = (secs * 1e9) as i128;
            OffsetDateTime::from_unix_timestamp_nanos(nanos).ok().map(iso)
        }
        serde_json::Value::String(s) if !s.trim().is_empty() => {
            Some(parse_iso(s).map(iso).unwrap_or_else(|| s.clone()))
        }
        _ => None,
    }
}

/// A number, or a number written as text.
pub(crate) fn number(v: Option<&serde_json::Value>) -> Option<f64> {
    match v? {
        serde_json::Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        serde_json::Value::String(s) if !s.trim().is_empty() => {
            s.trim().parse::<f64>().ok().filter(|x| x.is_finite())
        }
        _ => None,
    }
}

/// A non-empty string.
pub(crate) fn text(v: Option<&serde_json::Value>) -> Option<String> {
    v?.as_str().filter(|s| !s.is_empty()).map(String::from)
}

/// Lowercase words joined by `_`: `Gemini 3 Pro` is `gemini_3_pro`.
pub(crate) fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in value.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percents_round_and_clamp_as_quota_axi_does() {
        assert_eq!(clamp_percent(37.5), 38.0);
        assert_eq!(clamp_percent(37.49), 37.0);
        assert_eq!(clamp_percent(-3.0), 0.0);
        assert_eq!(clamp_percent(130.0), 100.0);
        assert_eq!(clamp_percent(f64::NAN), 0.0);
        // Math.round: halves go up.
        assert_eq!(percent_remaining(37.5), 63.0);
        let w = Window::used("five_hour", "session", 12.6, None);
        assert_eq!((w.percent_used, w.percent_remaining), (Some(13.0), Some(87.0)));
    }

    #[test]
    fn times_print_as_javascript_prints_them() {
        let t = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
        assert_eq!(iso(t), "2026-01-16T07:20:00.000Z");
        assert_eq!(
            epoch_or_iso(&serde_json::json!(1_768_548_000)).as_deref(),
            Some("2026-01-16T07:20:00.000Z")
        );
        assert_eq!(
            epoch_or_iso(&serde_json::json!("2026-01-16T14:20:00+07:00")).as_deref(),
            Some("2026-01-16T07:20:00.000Z")
        );
        assert_eq!(epoch_or_iso(&serde_json::json!("soon")).as_deref(), Some("soon"));
        assert_eq!(epoch_or_iso(&serde_json::json!("")), None);
    }

    #[test]
    fn slugs_keep_letters_and_digits() {
        assert_eq!(slugify("  Gemini 3 Pro (High) "), "gemini_3_pro_high");
        assert_eq!(slugify("Fable"), "fable");
        assert_eq!(slugify("--"), "");
    }
}
