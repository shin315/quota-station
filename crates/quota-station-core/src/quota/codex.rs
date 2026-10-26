// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Kun Chen (quota-axi)
// SPDX-License-Identifier: MIT

// Ported from quota-axi's `src/providers/codex.ts` (MIT,
// https://github.com/kunchenguid/quota-axi, 0.1.66): the login read from
// `auth.json` (the access token, the account id from the file or from the
// tokens' claims, the stored expiry as advice only), the two usage
// endpoints in order with the `ChatGPT-Account-Id` header, 401/403 as a
// sign-in, 429 as a rate limit, the windows (the five-hour and weekly
// limits told by their length, code review, named model limits and the
// workspace credit cap) and the error words. Changes: only Codex's own
// login (`codex-home`) is read, never Pi's; Codex's `app-server` is never
// started, since Codex renews its own token when it runs and this read must
// not; credits and the plan are not kept.

//! Codex's ChatGPT login: its five-hour and weekly windows. The token is
//! read from `auth.json` into memory for the one request and dropped; it is
//! never refreshed, logged or kept, and `auth.json` is never written.

use std::path::Path;
use std::time::Duration;

use base64::Engine;
use serde_json::Value;
use time::OffsetDateTime;

use super::cache::{fingerprint, Cache};
use super::{epoch_or_iso, number, text, Provider, ProviderId, Response, Window};
use crate::app_dir::AppDir;
use crate::codex::accounts::{Place, Store};
use crate::codex::platform::Platform;

pub const ENDPOINTS: [&str; 2] =
    ["https://chatgpt.com/backend-api/wham/usage", "https://chatgpt.com/backend-api/codex/usage"];
const TIMEOUT: Duration = Duration::from_secs(15);
pub const SIGN_IN_REQUIRED: &str = "Codex sign-in required";
pub const TOKEN_EXPIRED: &str = "Codex access token expired";
pub const UNAVAILABLE: &str = "Codex quota unavailable";
const FIVE_HOURS: f64 = 18_000.0;
const WEEK: f64 = 604_800.0;
/// The key of Codex's own login, as quota-axi names it.
pub const ACCOUNT_KEY: &str = "codex-home";

/// Where the usage is asked. The app always uses [`Endpoints::chatgpt`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    usage: Vec<String>,
}

impl Endpoints {
    pub fn chatgpt() -> Self {
        // The tests talk to a local fake; one that reached for ChatGPT stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake ChatGPT");
        }
        Endpoints { usage: ENDPOINTS.iter().map(|s| s.to_string()).collect() }
    }

    /// A local fake at `base`, with the real paths.
    #[cfg(feature = "test-support")]
    pub fn fake(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Endpoints {
            usage: vec![
                format!("{base}/backend-api/wham/usage"),
                format!("{base}/backend-api/codex/usage"),
            ],
        }
    }
}

/// The login `auth.json` holds, as read for one request.
struct Login {
    access: String,
    account_id: Option<String>,
    /// The token's own expiry has passed: a rejection then is an expiry.
    expired: bool,
    refreshable: bool,
    /// Tells a reading of this login from one of another.
    fingerprint: String,
}

enum Credential {
    Missing,
    Invalid,
    Found(Login),
}

fn claims(token: &str) -> Option<Value> {
    let part = token.split('.').nth(1)?;
    let bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(part.trim_end_matches('=')).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn read_login(codex_home: &Path, now: OffsetDateTime) -> Credential {
    let text_of = match std::fs::read_to_string(codex_home.join(crate::codex::AUTH)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Credential::Missing,
        Err(_) => return Credential::Invalid,
    };
    let Ok(data) = serde_json::from_str::<Value>(&text_of) else { return Credential::Invalid };
    let Some(tokens) = data.get("tokens").filter(|t| t.is_object()) else {
        return Credential::Invalid;
    };
    let Some(access) = text(tokens.get("access_token")).or_else(|| text(tokens.get("accessToken")))
    else {
        return Credential::Invalid;
    };
    let id = text(tokens.get("id_token")).or_else(|| text(tokens.get("idToken")));
    let access_claims = claims(&access);
    let decoded = id.as_deref().and_then(claims).or_else(|| access_claims.clone());
    let account_id = text(tokens.get("account_id"))
        .or_else(|| text(tokens.get("accountId")))
        .or_else(|| {
            decoded.as_ref().and_then(|d| text(d.get("https://api.openai.com/auth/account_id")))
        })
        .or_else(|| decoded.as_ref().and_then(|d| text(d.get("account_id"))));
    let expired = access_claims
        .as_ref()
        .and_then(|c| number(c.get("exp")))
        .is_some_and(|exp| exp <= now.unix_timestamp() as f64);
    let refreshable = tokens.get("refresh_token").is_some() || tokens.get("refreshToken").is_some();
    Credential::Found(Login {
        fingerprint: fingerprint(&format!("{}\0{access}", account_id.as_deref().unwrap_or(""))),
        access,
        account_id,
        expired,
        refreshable,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Failure {
    /// 401 or 403 from every endpoint.
    Rejected,
    RateLimited,
    Transient(String),
}

fn fetch(endpoints: &Endpoints, login: &Login) -> Result<Vec<Window>, Failure> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("quota-station")
        .build()
        .map_err(|_| Failure::Transient(UNAVAILABLE.into()))?;
    let mut rejected = false;
    let mut last: Option<Failure> = None;
    for url in &endpoints.usage {
        let mut request =
            client.get(url).bearer_auth(&login.access).header("accept", "application/json");
        if let Some(id) = &login.account_id {
            request = request.header("ChatGPT-Account-Id", id);
        }
        let response = match request.send() {
            Ok(r) => r,
            Err(e) => {
                let words =
                    if e.is_timeout() { "Codex quota request timed out" } else { UNAVAILABLE };
                last = Some(Failure::Transient(words.into()));
                continue;
            }
        };
        let status = response.status().as_u16();
        if status == 401 || status == 403 {
            rejected = true;
            continue;
        }
        if status == 429 {
            return Err(Failure::RateLimited);
        }
        if !response.status().is_success() {
            last = Some(Failure::Transient(UNAVAILABLE.into()));
            continue;
        }
        match response.json::<Value>().ok().map(|v| normalize(&v, OffsetDateTime::now_utc())) {
            Some(windows) if !windows.is_empty() => return Ok(windows),
            _ => last = Some(Failure::Transient(UNAVAILABLE.into())),
        }
    }
    match last {
        Some(f) => Err(f),
        None if rejected => Err(Failure::Rejected),
        None => Err(Failure::Transient(UNAVAILABLE.into())),
    }
}

/// One login, taken into memory for one read.
pub struct Held(Credential);

/// Takes the login in `folder` (the Codex folder, or an account's own)
/// into memory.
pub fn hold(folder: &Path, now: OffsetDateTime) -> Held {
    Held(read_login(folder, now))
}

/// Reads Codex's ChatGPT login in `codex_home`. A reading of the same login
/// younger than `max_age` seconds answers without a request; a failed read
/// shows the last reading as stale, unless the login is gone.
pub fn read(
    codex_home: &Path,
    endpoints: &Endpoints,
    cache: &Cache,
    max_age: i64,
    now: OffsetDateTime,
) -> Response {
    let mut row = read_held(hold(codex_home, now), endpoints, cache, max_age, now, false);
    row.account_keys = vec![ACCOUNT_KEY.to_string()];
    Response::new(now, vec![row])
}

/// The key of a ChatGPT account the app keeps: `codex:<n>`.
pub const KEPT_KEY: &str = "codex:";

/// Every ChatGPT account the app keeps for `codex_home`: the one in the
/// Codex folder (keyed `codex-home` as [`read`] has it, and `codex:<n>`),
/// then each in its own folder (`codex:<n>`), one row each, read side by
/// side. The sign-ins are taken into memory while nothing moves them; an
/// account whose token ran out in its own folder is not asked (Codex renews
/// it only when it runs there): its last reading shows as stale.
pub fn read_accounts(
    app: &AppDir,
    codex_home: &Path,
    platform: &dyn Platform,
    endpoints: &Endpoints,
    max_age: i64,
    now: OffsetDateTime,
) -> Response {
    let store = Store::new(app, codex_home, platform);
    let held = store.with_list(now, |list| {
        let home = list.iter().find(|a| a.place == Place::Home).map(|a| a.entry.number);
        let mut out = vec![(home, true, hold(codex_home, now))];
        out.extend(list.iter().filter(|a| a.place == Place::Folder).map(|a| {
            let n = a.entry.number;
            (Some(n), false, hold(&store.folder(n), now))
        }));
        out
    });
    let Ok(held) = held else {
        return read(codex_home, endpoints, &Cache::new(app, ProviderId::Codex), max_age, now);
    };
    let rows = std::thread::scope(|scope| {
        let reads: Vec<_> = held
            .into_iter()
            .map(|(number, home, login)| {
                scope.spawn(move || {
                    let cache = match number {
                        Some(n) => Cache::account(app, ProviderId::Codex, n),
                        None => Cache::new(app, ProviderId::Codex),
                    };
                    let mut row = read_held(login, endpoints, &cache, max_age, now, !home);
                    row.account_keys = home
                        .then(|| ACCOUNT_KEY.to_string())
                        .into_iter()
                        .chain(number.map(|n| format!("{KEPT_KEY}{n}")))
                        .collect();
                    row
                })
            })
            .collect();
        reads.into_iter().filter_map(|r| r.join().ok()).collect()
    });
    Response::new(now, rows)
}

/// One login's row. `waiting`: the login waits in its own folder, so a
/// token whose time ran out is not sent.
fn read_held(
    held: Held,
    endpoints: &Endpoints,
    cache: &Cache,
    max_age: i64,
    now: OffsetDateTime,
    waiting: bool,
) -> Provider {
    let login = match held.0 {
        Credential::Found(login) => login,
        missing => {
            // No login to read: the last reading belongs to one that is gone.
            cache.retire();
            let mut failed = Provider::failed(ProviderId::Codex, "auth_required", SIGN_IN_REQUIRED);
            // No `auth.json` at all: Codex is not set up on this Mac.
            failed.not_set_up = matches!(missing, Credential::Missing);
            return failed;
        }
    };
    if waiting && login.expired {
        return cache
            .stale(TOKEN_EXPIRED, now)
            .unwrap_or_else(|| Provider::failed(ProviderId::Codex, "unavailable", TOKEN_EXPIRED));
    }
    if let Some(reused) = cache.reusable(Some(&login.fingerprint), max_age, now) {
        return reused;
    }
    match fetch(endpoints, &login) {
        Ok(windows) => {
            let fresh = Provider::fresh(ProviderId::Codex, windows, now);
            cache.save(&fresh, Some(&login.fingerprint));
            fresh
        }
        Err(Failure::Rejected) if !login.expired || !login.refreshable => {
            cache.retire();
            Provider::failed(ProviderId::Codex, "auth_required", SIGN_IN_REQUIRED)
        }
        Err(failure) => {
            let (status, error) = match failure {
                // The token's own time ran out: Codex renews it when it runs.
                Failure::Rejected => ("unavailable", TOKEN_EXPIRED.to_string()),
                Failure::RateLimited => {
                    ("rate_limited", "Codex quota endpoint rate limited".to_string())
                }
                Failure::Transient(words) => ("error", words),
            };
            cache
                .stale(&error, now)
                .unwrap_or_else(|| Provider::failed(ProviderId::Codex, status, &error))
        }
    }
}

/// The usage answer's windows: the base five-hour and weekly limits, code
/// review's, the named model limits and the workspace credit cap. The HTTP
/// answer's snake_case and the app-server's camelCase both read.
pub fn normalize(data: &Value, now: OffsetDateTime) -> Vec<Window> {
    if !data.is_object() {
        return Vec::new();
    }
    let rate = ["rate_limit", "rateLimits", "rate_limits"]
        .iter()
        .find_map(|k| data.get(*k).filter(|v| v.is_object()))
        .unwrap_or(data);
    let mut windows = pair(Some(rate), Names::account(), now);
    windows.extend(pair(
        data.get("code_review_rate_limit").filter(|v| v.is_object()),
        Names::code_review(),
        now,
    ));
    if let Some(extra) = data.get("additional_rate_limits").and_then(Value::as_array) {
        for item in extra {
            let id = text(item.get("metered_feature")).or_else(|| text(item.get("limit_name")));
            let label = text(item.get("limit_name")).or_else(|| id.clone());
            let container = item.get("rate_limit").filter(|v| v.is_object());
            if let (Some(id), Some(_), Some(c)) = (id, label, container) {
                windows.extend(pair(Some(c), Names::model(&id), now));
            }
        }
    }
    if let Some(by_id) = data.get("rateLimitsByLimitId").and_then(Value::as_object) {
        for (id, item) in by_id {
            let named = text(item.get("limitName")).or_else(|| text(item.get("limit_name")));
            if named.is_some() && item.is_object() {
                windows.extend(pair(Some(item), Names::model(id), now));
            }
        }
    }
    windows.extend(spend_control(data.get("spend_control"), now));
    // A repeated id gets `_2`, `_3`, as quota-axi numbers them.
    let mut seen: Vec<(String, u32)> = Vec::new();
    for w in &mut windows {
        let count = match seen.iter_mut().find(|(id, _)| *id == w.id) {
            Some((_, n)) => {
                *n += 1;
                *n
            }
            None => {
                seen.push((w.id.clone(), 1));
                1
            }
        };
        if count > 1 {
            w.id = format!("{}_{count}", w.id);
        }
    }
    windows
}

/// How one container's two windows are named.
struct Names {
    session: (String, &'static str),
    weekly: (String, &'static str),
    /// The id prefix of a window of another length.
    other: String,
    other_kind: &'static str,
}

impl Names {
    fn account() -> Names {
        Names {
            session: ("five_hour".into(), "session"),
            weekly: ("weekly".into(), "weekly"),
            other: "window".into(),
            other_kind: "unknown",
        }
    }

    fn code_review() -> Names {
        Names {
            session: ("code_review_five_hour".into(), "session"),
            weekly: ("code_review_weekly".into(), "weekly"),
            other: "code_review_window".into(),
            other_kind: "unknown",
        }
    }

    fn model(id: &str) -> Names {
        Names {
            session: (format!("model:{id}:5h"), "model"),
            weekly: (format!("model:{id}:7d"), "model"),
            other: format!("model:{id}:window"),
            other_kind: "model",
        }
    }
}

/// `5h`, `2.5h`: a length in hours as quota-axi writes it.
fn hours(seconds: f64) -> String {
    let h = seconds / 3600.0;
    if h.fract() == 0.0 {
        format!("{h}h")
    } else {
        let rounded = (h * 100.0).round() / 100.0;
        format!("{rounded}h")
    }
}

fn pair(container: Option<&Value>, names: Names, now: OffsetDateTime) -> Vec<Window> {
    let Some(c) = container else { return Vec::new() };
    let primary = c.get("primary_window").or_else(|| c.get("primary"));
    let secondary = c.get("secondary_window").or_else(|| c.get("secondary"));
    [(primary, &names.session), (secondary, &names.weekly)]
        .into_iter()
        .filter_map(|(raw, fallback)| one(raw?, fallback, &names, now))
        .collect()
}

fn one(
    raw: &Value,
    fallback: &(String, &'static str),
    names: &Names,
    now: OffsetDateTime,
) -> Option<Window> {
    if !raw.is_object() {
        return None;
    }
    let used = number(raw.get("used_percent")).or_else(|| number(raw.get("usedPercent")))?;
    let seconds = number(raw.get("limit_window_seconds"))
        .or_else(|| number(raw.get("windowDurationMins")).map(|m| m * 60.0));
    let (id, kind) = match seconds {
        None => (fallback.0.clone(), fallback.1),
        Some(s) if s == FIVE_HOURS => (names.session.0.clone(), names.session.1),
        Some(s) if s == WEEK => (names.weekly.0.clone(), names.weekly.1),
        Some(s) => (format!("{}:{}", names.other, hours(s)), names.other_kind),
    };
    let after = number(raw.get("reset_after_seconds"))
        .map(|s| super::iso(now + time::Duration::seconds_f64(s)));
    let resets_at = raw
        .get("reset_at")
        .and_then(epoch_or_iso)
        .or_else(|| raw.get("resetsAt").and_then(epoch_or_iso))
        .or(after);
    let mut window = Window::used(&id, kind, used, resets_at);
    window.window_seconds = seconds;
    Some(window)
}

fn spend_control(raw: Option<&Value>, now: OffsetDateTime) -> Option<Window> {
    let control = raw.filter(|v| v.is_object())?;
    let cap = control.get("individual_limit").filter(|v| v.is_object())?;
    let limit = number(cap.get("limit"));
    let used = number(cap.get("used"));
    let remaining = number(cap.get("remaining"));
    let totals = match (limit, used, remaining) {
        (Some(l), Some(u), Some(r)) if l > 0.0 && u >= 0.0 && r >= 0.0 => Some((l, u, r)),
        _ => None,
    };
    let bound = |v: Option<f64>| v.filter(|x| x.is_finite()).map(|x| x.clamp(0.0, 100.0));
    let reached = control.get("reached").and_then(Value::as_bool) == Some(true);
    let (pct_used, pct_left) = if reached {
        (Some(100.0), Some(0.0))
    } else {
        match totals {
            Some((l, u, r)) => (bound(Some(u / l * 100.0)), bound(Some(r / l * 100.0))),
            None => (
                bound(number(cap.get("used_percent"))),
                bound(number(cap.get("remaining_percent"))),
            ),
        }
    };
    let resets_at =
        number(cap.get("reset_at")).and_then(|s| epoch_or_iso(&Value::from(s))).or_else(|| {
            number(cap.get("reset_after_seconds"))
                .filter(|s| *s >= 0.0)
                .map(|s| super::iso(now + time::Duration::seconds_f64(s)))
        });
    Some(Window {
        id: "spend_control".into(),
        kind: Some("credits".into()),
        percent_used: pct_used,
        percent_remaining: pct_left,
        resets_at,
        window_seconds: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap()
    }

    #[test]
    fn the_http_answer_reads_as_quota_axi_reads_it() {
        let data = json!({
            "plan_type": "pro",
            "rate_limit": {
                "primary_window": {"used_percent": 37.6, "limit_window_seconds": 18000, "reset_at": 1768561200},
                "secondary_window": {"used_percent": 12, "limit_window_seconds": 604800, "reset_after_seconds": 3600}
            },
            "code_review_rate_limit": {"primary_window": {"used_percent": 1, "limit_window_seconds": 604800}},
            "additional_rate_limits": [{"metered_feature": "spark", "limit_name": "Spark", "rate_limit": {"primary_window": {"used_percent": 50, "limit_window_seconds": 18000}}}]
        });
        let w = normalize(&data, now());
        let ids: Vec<&str> = w.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["five_hour", "weekly", "code_review_weekly", "model:spark:5h"]);
        assert_eq!(w[0].percent_remaining, Some(62.0));
        assert_eq!(w[0].resets_at.as_deref(), Some("2026-01-16T11:00:00.000Z"));
        assert_eq!(w[1].resets_at.as_deref(), Some("2026-01-16T08:20:00.000Z"));
        assert_eq!(w[1].kind.as_deref(), Some("weekly"));
    }

    #[test]
    fn a_window_of_another_length_and_a_repeat_get_their_own_ids() {
        let data = json!({"rateLimits": {
            "primary": {"usedPercent": 5, "windowDurationMins": 150},
            "secondary": {"usedPercent": 9}
        }});
        let w = normalize(&data, now());
        assert_eq!(w[0].id, "window:2.5h");
        assert_eq!(w[1].id, "weekly", "no length: the slot's own name");
        assert!(normalize(&json!([]), now()).is_empty());
    }

    #[test]
    fn the_credit_cap_uses_the_totals_when_whole() {
        let data = json!({"spend_control": {"individual_limit": {"limit": 200, "used": 50, "remaining": 150, "reset_at": 1768561200}}});
        let w = normalize(&data, now());
        assert_eq!(w[0].id, "spend_control");
        assert_eq!((w[0].percent_used, w[0].percent_remaining), (Some(25.0), Some(75.0)));
    }

    #[test]
    fn the_account_id_comes_from_the_file_or_the_claims() {
        let dir = std::env::temp_dir().join(format!("qs-codex-login-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            br#"{"exp":1768540000,"https://api.openai.com/auth/account_id":"acct-from-claims"}"#,
        );
        let token = format!("h.{payload}.s");
        std::fs::write(
            dir.join("auth.json"),
            json!({"tokens": {"access_token": token, "refresh_token": "r"}}).to_string(),
        )
        .unwrap();
        let Credential::Found(login) = read_login(&dir, now()) else { panic!("a login") };
        assert_eq!(login.account_id.as_deref(), Some("acct-from-claims"));
        assert!(login.expired && login.refreshable);
        std::fs::write(dir.join("auth.json"), json!({"OPENAI_API_KEY": "k"}).to_string()).unwrap();
        assert!(matches!(read_login(&dir, now()), Credential::Invalid));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(read_login(&dir, now()), Credential::Missing));
    }
}
