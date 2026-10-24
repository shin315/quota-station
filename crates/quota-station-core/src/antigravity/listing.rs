// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The Antigravity accounts the app logs in, on the Antigravity row: each
//! one's last reading from the app's own cache ([`super::export`]), its
//! Gemini and Claude/GPT numbers, and which one agy is on.
//!
//! The account in use is told, in this order, by the email the live read
//! names (compared by its hash, never kept: `identity`), by the live
//! reading's resets matching one account's to within five seconds and no
//! other's (`reading`), or by the account the app's last switch put on
//! (`binding`), unless the live reading contradicts it.

use time::OffsetDateTime;

use super::export;
use super::index::{self, AccountIndex};
use super::switch;
use crate::app_dir::AppDir;
use crate::quota::{
    agy, iso, parse_iso, semantics, AgyAccount, AgyAccounts, Provider, Scope, Window,
};

/// A reading older than this is stale.
pub const FRESH_SECONDS: f64 = 900.0;
const RESET_TOLERANCE_SECONDS: f64 = 5.0;

/// Reads the account agy is on ([`agy::read`]) and puts the app's own list
/// on its row.
pub fn read(
    tools: &agy::Tools,
    app: &AppDir,
    max_age: i64,
    now: OffsetDateTime,
) -> crate::quota::Response {
    let cache = crate::quota::cache::Cache::new(app, crate::quota::ProviderId::Agy);
    let mut report = agy::read(tools, &cache, max_age, now);
    if let Some(row) = report.primary_mut("agy") {
        attach(row, app, now);
    }
    report
}

/// Puts the app's own list on `row`; nothing when the app logs no account
/// in here.
pub fn attach(row: &mut Provider, app: &AppDir, now: OffsetDateTime) {
    let index = match index::load(app) {
        Ok(index) if !index.accounts.is_empty() => index,
        Ok(_) => return,
        Err(_) => {
            row.agy_accounts = Some(AgyAccounts {
                status: "unavailable".into(),
                reason: Some("index_unreadable".into()),
                ..AgyAccounts::default()
            });
            return;
        }
    };
    let read: Vec<(AgyAccount, Vec<Window>)> = index
        .accounts
        .iter()
        .enumerate()
        .map(|(i, a)| account(app, (i + 1) as u32, &a.email, now))
        .collect();
    let binding = binding(app);
    let (active, basis) = match active(&index, &read, binding.as_deref(), row, now) {
        Some((n, basis)) => (Some(n), Some(basis.to_string())),
        None => (None, None),
    };
    row.agy_accounts = Some(AgyAccounts {
        status: "ok".into(),
        reason: None,
        active_basis: basis,
        accounts: read
            .into_iter()
            .map(|(mut a, _)| {
                a.active = Some(a.account) == active;
                a
            })
            .collect(),
    });
}

fn binding(app: &AppDir) -> Option<String> {
    let text = app.read_to_string(&switch::binding_path(app)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.pointer("/defaultSettings/bindAccountId")?.as_str().map(String::from)
}

fn account(
    app: &AppDir,
    number: u32,
    email: &str,
    now: OffsetDateTime,
) -> (AgyAccount, Vec<Window>) {
    let no_data = || {
        (
            AgyAccount { account: number, status: "no_data".into(), ..AgyAccount::default() },
            Vec::new(),
        )
    };
    let Some(record) = export::read(app, email) else { return no_data() };
    if !record.email.trim().eq_ignore_ascii_case(email.trim()) {
        return no_data();
    }
    let Ok(at) =
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(record.updated_at) * 1_000_000)
    else {
        return no_data();
    };
    let windows = record
        .payload
        .get("quota_summary")
        .filter(|s| s.is_object())
        .map(agy::quota_summary)
        .unwrap_or_default();
    if windows.is_empty() {
        return no_data();
    }
    let age = (now - at).as_seconds_f64().max(0.0).round();
    let scopes = semantics::AGY_SCOPES.iter().filter_map(|s| scope(s, &windows, now)).collect();
    (
        AgyAccount {
            account: number,
            active: false,
            status: if age <= FRESH_SECONDS { "fresh" } else { "stale" }.into(),
            age_seconds: Some(age),
            fetched_at: Some(iso(at)),
            scopes,
        },
        windows,
    )
}

fn scope(name: &str, windows: &[Window], now: OffsetDateTime) -> Option<Scope> {
    let prefix = format!("{name}_");
    let bounds: Vec<Window> =
        windows.iter().filter(|w| w.id.starts_with(&prefix)).cloned().collect();
    if bounds.is_empty() {
        return None;
    }
    if bounds.iter().any(|w| w.reset_passed(now)) {
        return Some(Scope {
            scope: name.into(),
            status: "reset_pending".into(),
            ..Scope::default()
        });
    }
    let effective =
        semantics::agy(&bounds).effective_availability.into_iter().find(|e| e.scope == name);
    let Some((e, left)) = effective.and_then(|e| e.effective_percent_remaining.map(|l| (e, l)))
    else {
        return Some(Scope { scope: name.into(), status: "unknown".into(), ..Scope::default() });
    };
    let limiting: Vec<&Window> =
        bounds.iter().filter(|w| e.limiting_window_ids.contains(&w.id)).collect();
    let mut resets: Vec<&str> = limiting.iter().filter_map(|w| w.resets_at.as_deref()).collect();
    resets.sort_unstable();
    Some(Scope {
        scope: name.into(),
        status: "known".into(),
        effective_percent_remaining: Some(left),
        limited_by: (!limiting.is_empty())
            .then(|| limiting.iter().map(|w| w.id.as_str()).collect::<Vec<_>>().join(" + ")),
        resets_at: resets.last().map(|r| r.to_string()),
    })
}

/// How many windows of `reading` the live windows agree and disagree with:
/// both used, the reset still ahead, the two resets within five seconds.
fn agreement(live: &[Window], reading: &[Window], now: OffsetDateTime) -> (u32, u32) {
    let used = |w: &Window| w.percent_used.unwrap_or(0.0) > 0.0;
    let mut out = (0, 0);
    for w in reading {
        let Some(other) = live.iter().find(|o| o.id == w.id) else { continue };
        if !used(w) || !used(other) {
            continue;
        }
        let seen = w.resets_at.as_deref().and_then(parse_iso);
        let live_at = other.resets_at.as_deref().and_then(parse_iso);
        let (Some(seen), Some(live_at)) = (seen, live_at) else { continue };
        if seen <= now {
            continue;
        }
        if (seen - live_at).as_seconds_f64().abs() <= RESET_TOLERANCE_SECONDS {
            out.0 += 1;
        } else {
            out.1 += 1;
        }
    }
    out
}

fn active(
    index: &AccountIndex,
    read: &[(AgyAccount, Vec<Window>)],
    binding: Option<&str>,
    row: &Provider,
    now: OffsetDateTime,
) -> Option<(u32, &'static str)> {
    let named = row.account_keys.iter().find(|k| k.starts_with("email:"));
    if let Some(key) = named {
        let i = index.accounts.iter().position(|a| &agy::account_hash(&a.email) == key)?;
        return Some(((i + 1) as u32, "identity"));
    }
    let live: &[Window] = if row.state.stale() { &[] } else { &row.windows };
    let verdicts: Vec<(u32, (u32, u32))> =
        read.iter().map(|(a, w)| (a.account, agreement(live, w, now))).collect();
    let matches: Vec<u32> = verdicts
        .iter()
        .filter(|(_, (agree, disagree))| *agree > 0 && *disagree == 0)
        .map(|v| v.0)
        .collect();
    if let [one] = matches[..] {
        return Some((one, "reading"));
    }
    let bound = binding
        .and_then(|id| index.accounts.iter().position(|a| a.id == id))
        .map(|i| (i + 1) as u32)?;
    let contradicted = if matches.len() > 1 {
        !matches.contains(&bound)
    } else {
        verdicts.iter().any(|(n, (_, disagree))| *n == bound && *disagree > 0)
    };
    (!contradicted).then_some((bound, "binding"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quota::ProviderId;

    fn at(s: &str) -> OffsetDateTime {
        parse_iso(s).unwrap()
    }

    fn w(id: &str, used: f64, reset: &str) -> Window {
        Window::used(id, "session", used, Some(reset.into()))
    }

    #[test]
    fn the_live_resets_tell_the_account_in_use() {
        let now = at("2030-01-15T12:00:00Z");
        let mine = vec![w("gemini_5h", 30.0, "2030-01-15T14:00:02Z")];
        let other = vec![w("gemini_5h", 60.0, "2030-01-15T13:00:00Z")];
        let read = vec![
            (AgyAccount { account: 1, ..AgyAccount::default() }, other),
            (AgyAccount { account: 2, ..AgyAccount::default() }, mine),
        ];
        let row = Provider::fresh(
            ProviderId::Agy,
            vec![w("gemini_5h", 35.0, "2030-01-15T14:00:00Z")],
            now,
        );
        let index = AccountIndex::new();
        assert_eq!(active(&index, &read, None, &row, now), Some((2, "reading")));
        let unread = Provider::failed(ProviderId::Agy, "unavailable", "x");
        assert_eq!(active(&index, &read, None, &unread, now), None, "no binding, no reading");
    }
}
