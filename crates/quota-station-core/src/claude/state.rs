//! What the app tells other programs about the Claude accounts:
//!
//! - `claude/state.json`, for Firstmate and anyone else: each account's
//!   slot, alias, whether it is in the rotation, its three windows, how old
//!   its reading is and its error, and the account in use. No email, no
//!   token. `quota-station claude status --json` prints the same, and only
//!   with `--emails` adds the emails.
//! - `claude/swap/sequence.json` and `claude/swap/cache/usage.json`, the
//!   same numbers in claude-swap's layout, which quota-axi reads when
//!   `QUOTA_AXI_CLAUDE_SWAP_DIR` points at `claude/swap`. Addresses there
//!   are placeholders (`slot-2@quota-station.local`); quota-axi only
//!   compares the two files' with each other.
//! - [`details`], for the app's own windows: each account's windows and,
//!   only while the owner shows names, its email and organization, read
//!   from its Keychain item into memory.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::store::{Kind, List, Store};
use super::usage::{self, Cache, Reading, Win};
use crate::detail::{ClaudeDetail, Usage};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowOut {
    pub used_percent: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Windows {
    pub five_hour: Option<WindowOut>,
    pub seven_day: Option<WindowOut>,
    pub fable: Option<WindowOut>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountOut {
    pub slot: u32,
    pub alias: Option<String>,
    /// In the rotation (not held out by the owner).
    pub enabled: bool,
    pub active: bool,
    /// `login` or `setupToken`.
    pub kind: Kind,
    /// `fresh` (at most 15 minutes old), `stale`, `noData` or
    /// `reloginRequired`.
    pub status: &'static str,
    pub fetched_at: Option<String>,
    pub age_seconds: Option<u64>,
    pub windows: Windows,
    /// The last read's error kind (`http-429`, `invalid_grant`…).
    pub error: Option<String>,
    /// Only with `--emails`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateOut {
    pub schema_version: u32,
    pub generated_at: String,
    pub active_slot: Option<u32>,
    pub accounts: Vec<AccountOut>,
}

fn rfc(t: f64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(t.floor() as i64).ok()?.format(&Rfc3339).ok()
}

fn out(w: Option<&Win>) -> Option<WindowOut> {
    w.map(|w| WindowOut { used_percent: w.pct, resets_at: w.resets_at.clone() })
}

/// The data file's content for `list`, `cache` and the account in use.
pub fn build(list: &List, cache: &Cache, active: Option<u32>, now: OffsetDateTime) -> StateOut {
    let at = super::epoch(now);
    let accounts = list
        .accounts
        .iter()
        .map(|a| {
            let row = cache.row(a.slot).filter(|r| r.identity == a.identity);
            let reading = row.and_then(|r| r.last_good.as_ref());
            let status = match row {
                Some(r) if r.relogin => "reloginRequired",
                Some(r) if r.fresh(at) => "fresh",
                Some(r) if r.last_good.is_some() => "stale",
                _ => "noData",
            };
            AccountOut {
                slot: a.slot,
                alias: a.alias.clone(),
                enabled: !a.disabled,
                active: active == Some(a.slot),
                kind: a.kind,
                status,
                fetched_at: row.and_then(|r| r.fetched_at).and_then(rfc),
                age_seconds: row.and_then(|r| r.age(at)).map(|s| s as u64),
                windows: Windows {
                    five_hour: out(reading.and_then(|r| r.five_hour.as_ref())),
                    seven_day: out(reading.and_then(|r| r.seven_day.as_ref())),
                    fable: reading
                        .and_then(Reading::fable)
                        .map(|f| WindowOut { used_percent: f.pct, resets_at: f.resets_at.clone() }),
                },
                error: row.and_then(|r| r.last_error.clone()),
                email: None,
            }
        })
        .collect();
    StateOut {
        schema_version: SCHEMA,
        generated_at: now.format(&Rfc3339).unwrap_or_default(),
        active_slot: active.filter(|s| list.get(*s).is_some()),
        accounts,
    }
}

/// The placeholder address the copy for quota-axi gives an account.
pub fn placeholder(slot: u32) -> String {
    format!("slot-{slot}@quota-station.local")
}

/// `sequence.json` in claude-swap's layout.
pub fn sequence(list: &List, active: Option<u32>, now: OffsetDateTime) -> Value {
    let accounts: BTreeMap<String, Value> = list
        .accounts
        .iter()
        .map(|a| {
            let mut record = json!({
                "email": placeholder(a.slot),
                "uuid": a.account_uuid,
                "organizationUuid": a.organization_uuid,
                "organizationName": "",
            });
            if a.disabled {
                record["disabled"] = Value::Bool(true);
            }
            (a.slot.to_string(), record)
        })
        .collect();
    json!({
        "activeAccountNumber": active,
        "lastUpdated": now.format(&Rfc3339).unwrap_or_default(),
        "sequence": list.accounts.iter().map(|a| a.slot).collect::<Vec<_>>(),
        "accounts": accounts,
    })
}

/// `cache/usage.json` in claude-swap's layout (its schema 2).
pub fn swap_usage(list: &List, cache: &Cache) -> Value {
    let accounts: BTreeMap<String, Value> = list
        .accounts
        .iter()
        .filter_map(|a| {
            let row = cache.row(a.slot).filter(|r| r.identity == a.identity)?;
            Some((
                a.slot.to_string(),
                json!({
                    "email": placeholder(a.slot),
                    "organizationUuid": a.organization_uuid,
                    "lastGood": row.last_good,
                    "fetchedAt": row.fetched_at,
                    "lastError": row.last_error,
                }),
            ))
        })
        .collect();
    json!({ "schemaVersion": 2, "accounts": accounts })
}

/// Writes the data file and the copy for quota-axi.
pub fn write(store: &Store, active: Option<u32>, now: OffsetDateTime) -> std::io::Result<()> {
    let list = store.load().map_err(|e| std::io::Error::other(e.to_string()))?;
    let cache = usage::load(store);
    let state = build(&list, &cache, active, now);
    let pretty = |v: &Value| serde_json::to_string_pretty(v).expect("serializes") + "\n";
    super::write_private(
        &store.paths.state(),
        &(serde_json::to_string_pretty(&state).expect("serializes") + "\n"),
    )?;
    let swap = store.paths.swap();
    super::write_private(&swap.join("sequence.json"), &pretty(&sequence(&list, active, now)))?;
    super::write_private(&swap.join("cache/usage.json"), &pretty(&swap_usage(&list, &cache)))
}

/// Each account the app keeps with its last reading, for the windows. With
/// `names`, also its email and organization, from its Keychain item: they
/// stay in memory.
pub fn details(store: &Store, names: bool) -> Vec<ClaudeDetail> {
    let Ok(list) = store.load() else { return Vec::new() };
    let cache = usage::load(store);
    let usage = |w: Option<&Win>| w.map(|w| Usage { used: w.pct, resets_at: w.resets_at.clone() });
    list.accounts
        .iter()
        .map(|a| {
            let reading = cache
                .row(a.slot)
                .filter(|r| r.identity == a.identity)
                .and_then(|r| r.last_good.as_ref());
            let oauth = if names { store.oauth_account(a.slot).ok().flatten() } else { None };
            let text = |key: &str| {
                oauth
                    .as_ref()
                    .and_then(|o| o.get(key))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(String::from)
            };
            ClaudeDetail {
                number: a.slot,
                email: text("emailAddress"),
                organization: text("organizationName"),
                five_hour: usage(reading.and_then(|r| r.five_hour.as_ref())),
                seven_day: usage(reading.and_then(|r| r.seven_day.as_ref())),
                fable: reading
                    .and_then(Reading::fable)
                    .map(|f| Usage { used: f.pct, resets_at: f.resets_at.clone() }),
            }
        })
        .collect()
}
