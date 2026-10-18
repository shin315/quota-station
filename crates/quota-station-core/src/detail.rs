//! Every window of every account, on top of quota-axi's one number per
//! account: Claude's from claude-swap's list (`cswap list --json`, see
//! [`crate::cswap::list`]), Antigravity's from the app's own readings
//! ([`crate::antigravity::export`]). Matched by account number. An account
//! neither one knows keeps only quota-axi's lines, and the windows show
//! those.
//!
//! A window whose reset passed after its reading has no number
//! (`pending`), as everywhere else: never an assumed 100 %.

use serde_json::Value;
use time::OffsetDateTime;

use crate::cswap::{SwapAccount, Usage};
use crate::snapshot::{Line, LineKind, Section};
use crate::time_util::is_after;

fn line(kind: LineKind, left: f64, resets_at: Option<String>, now: Option<OffsetDateTime>) -> Line {
    let passed = resets_at.as_deref().is_some_and(|at| !is_after(at, now));
    Line {
        kind,
        percent: (!passed).then_some(left.clamp(0.0, 100.0)),
        resets_at,
        pending: passed,
        at_least: false,
    }
}

fn usage_line(kind: LineKind, usage: &Usage, now: Option<OffsetDateTime>) -> Line {
    line(kind, 100.0 - usage.used, usage.resets_at.clone(), now)
}

/// The account's five-hour, weekly and Fable windows, those claude-swap has.
pub fn claude_windows(account: &SwapAccount, now: Option<OffsetDateTime>) -> Vec<Line> {
    [
        (LineKind::Session, &account.five_hour),
        (LineKind::Week, &account.seven_day),
        (LineKind::FableWeek, &account.fable),
    ]
    .into_iter()
    .filter_map(|(kind, usage)| usage.as_ref().map(|u| usage_line(kind, u, now)))
    .collect()
}

/// Gives each Claude account claude-swap lists its windows (unless it has
/// them already, from quota-axi's live reading), its email and its
/// organization.
pub fn apply_claude(section: &mut Section, list: &[SwapAccount], now: Option<OffsetDateTime>) {
    for account in &mut section.accounts {
        let Some(swap) = list.iter().find(|s| s.number == account.number) else {
            continue;
        };
        if account.windows.is_empty() {
            account.windows = claude_windows(swap, now);
        }
        account.email = swap.email.clone();
        account.org = swap.organization.clone();
    }
}

/// The four windows of an Antigravity reading, from the summary under
/// `quota_summary`: Gemini and Claude/GPT, five hours and a week each.
pub fn agy_windows(payload: &Value, now: Option<OffsetDateTime>) -> Vec<Line> {
    let groups = payload["quota_summary"]["groups"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut found: Vec<Line> = groups
        .iter()
        .flat_map(|g| {
            g["buckets"].as_array().map(Vec::as_slice).unwrap_or(&[]).iter().map(move |b| (g, b))
        })
        .filter_map(|(group, bucket)| {
            let id = bucket["bucketId"].as_str().unwrap_or_default();
            let name = group["displayName"].as_str().unwrap_or_default();
            let gemini = if id.starts_with("gemini") {
                true
            } else if id.starts_with("3p") {
                false
            } else {
                name.to_ascii_lowercase().contains("gemini")
            };
            let kind = match (gemini, bucket["window"].as_str()?) {
                (true, "5h") => LineKind::GeminiSession,
                (true, "weekly") => LineKind::GeminiWeek,
                (false, "5h") => LineKind::ClaudeGptSession,
                (false, "weekly") => LineKind::ClaudeGptWeek,
                _ => return None,
            };
            let resets_at =
                bucket["resetTime"].as_str().filter(|s| !s.is_empty()).map(String::from);
            // Google leaves out a fraction of zero: a used-up bucket.
            let fraction = match bucket["remainingFraction"].as_f64() {
                Some(f) => f,
                None if resets_at.is_some() => 0.0,
                None => return None,
            };
            Some(line(kind, fraction * 100.0, resets_at, now))
        })
        .collect();
    found.sort_by_key(|l| l.kind as u8);
    found.dedup_by_key(|l| l.kind);
    found
}

/// One Antigravity account's reading, as the app found it on disk.
pub struct AgyDetail {
    pub number: u32,
    pub email: Option<String>,
    pub payload: Option<Value>,
}

/// Gives each Antigravity account the app reads its four windows and
/// email. Only for accounts quota-axi numbered from this app's own list.
pub fn apply_agy(section: &mut Section, details: &[AgyDetail], now: Option<OffsetDateTime>) {
    if section.source.as_deref() != Some("quota-station") {
        return;
    }
    for account in &mut section.accounts {
        let Some(detail) = details.iter().find(|d| d.number == account.number) else {
            continue;
        };
        if let Some(payload) = &detail.payload {
            let windows = agy_windows(payload, now);
            if !windows.is_empty() {
                account.windows = windows;
            }
        }
        account.email = detail.email.clone();
    }
}

/// Takes every email and organization out, for when the owner hides names.
pub fn hide_names(section: &mut Section) {
    for account in &mut section.accounts {
        account.email = None;
        account.org = None;
    }
}
