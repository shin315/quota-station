// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Kun Chen (quota-axi)
// SPDX-License-Identifier: MIT

// Ported from quota-axi's `src/interpretation.ts` (MIT,
// https://github.com/kunchenguid/quota-axi, 0.1.66): which windows bound
// which scope for Claude, Codex and Antigravity, the effective percent as
// the smallest percent left among them, the limiting windows, a stale row's
// scopes as unknown, and an Antigravity row marked stale once one of its
// resets has passed. The pace, runway and selection figures are not ported:
// nothing in the app shows them.

//! What each scope of a row has left.

use time::OffsetDateTime;

use super::{Effective, Provider, QuotaSemantics, Window};

/// Fills `row.quota_semantics` for its provider, as of `now`.
pub fn apply(row: &mut Provider, now: OffsetDateTime) {
    if row.provider == "agy" && row.state.status == "fresh" {
        // A window past its reset is a cycle the reading never saw.
        if row.windows.iter().any(|w| w.reset_passed(now)) {
            row.state.status = "stale".into();
        }
    }
    let mut semantics = match row.provider.as_str() {
        "claude" => claude(&row.windows),
        "codex" => codex(&row.windows),
        "agy" => agy(&row.windows),
        _ => QuotaSemantics { status: "unknown".into(), effective_availability: Vec::new() },
    };
    if row.state.stale() {
        semantics.status =
            if semantics.status == "partial" { "partial" } else { "unknown" }.to_string();
        for e in &mut semantics.effective_availability {
            e.status = "unknown".into();
            e.effective_percent_remaining = None;
            e.limiting_window_ids.clear();
        }
    }
    row.quota_semantics = Some(semantics);
}

/// The scope bounded by `windows`: the smallest percent left, and the
/// windows at it. Unknown when any of them has no percent.
pub fn availability(scope: &str, windows: &[&Window]) -> Effective {
    let bounded_by = windows.iter().map(|w| w.id.clone()).collect();
    let remaining: Vec<Option<f64>> = windows.iter().map(|w| w.percent_remaining).collect();
    if remaining.is_empty() || remaining.iter().any(Option::is_none) {
        return Effective {
            scope: scope.to_string(),
            status: "unknown".into(),
            effective_percent_remaining: None,
            bounded_by,
            limiting_window_ids: Vec::new(),
        };
    }
    let min = remaining.iter().flatten().copied().fold(f64::INFINITY, f64::min);
    Effective {
        scope: scope.to_string(),
        status: "known".into(),
        effective_percent_remaining: Some(min),
        bounded_by,
        limiting_window_ids: windows
            .iter()
            .filter(|w| w.percent_remaining == Some(min))
            .map(|w| w.id.clone())
            .collect(),
    }
}

fn known(effective: Vec<Effective>) -> QuotaSemantics {
    QuotaSemantics {
        status: if effective.is_empty() { "unknown" } else { "known" }.into(),
        effective_availability: effective,
    }
}

fn partial() -> QuotaSemantics {
    QuotaSemantics { status: "partial".into(), effective_availability: Vec::new() }
}

/// The five-hour and weekly windows bound every model; a model's own
/// weekly window is one more bound on that model.
pub fn claude(windows: &[Window]) -> QuotaSemantics {
    let account: Vec<&Window> =
        windows.iter().filter(|w| w.id == "five_hour" || w.id == "seven_day").collect();
    let models: Vec<&Window> =
        windows.iter().filter(|w| w.kind.as_deref() == Some("model")).collect();
    let unresolved = windows.iter().any(|w| {
        !["five_hour", "seven_day", "extra_usage"].contains(&w.id.as_str())
            && w.kind.as_deref() != Some("model")
    });
    if unresolved {
        return partial();
    }
    let mut effective = Vec::new();
    if !account.is_empty() {
        effective.push(availability("all_models", &account));
    }
    for model in models {
        let mut bounds = account.clone();
        bounds.push(model);
        effective.push(availability(&model.id, &bounds));
    }
    known(effective)
}

/// `five_hour`, `weekly` (a repeat numbered `_2`) or an unfamiliar length.
fn codex_account_window(id: &str) -> bool {
    let plain = |name: &str| {
        id == name
            || id
                .strip_prefix(name)
                .and_then(|r| r.strip_prefix('_'))
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    plain("five_hour") || plain("weekly") || id.starts_with("window:")
}

/// The model a named Codex window meters: its id without the window part.
fn codex_model_scope(id: &str) -> String {
    let mut s = id.to_string();
    if let Some((head, tail)) = s.rsplit_once('_') {
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            s = head.to_string();
        }
    }
    for suffix in [":5h", ":7d"] {
        if let Some(head) = s.strip_suffix(suffix) {
            return head.to_string();
        }
    }
    if let Some(at) = s.rfind(":window:") {
        if !s[at + ":window:".len()..].contains(':') {
            return s[..at].to_string();
        }
    }
    s
}

/// The base five-hour and weekly windows bound every model; code review
/// is a scope of its own; a named model window adds its own bound. A model
/// whose own windows still have room while a base window is at zero is a
/// contradiction, not an exhaustion: unknown.
pub fn codex(windows: &[Window]) -> QuotaSemantics {
    let spend: Vec<&Window> = windows.iter().filter(|w| w.id == "spend_control").collect();
    let account: Vec<&Window> = if spend.len() == windows.len() {
        spend.clone()
    } else {
        windows.iter().filter(|w| codex_account_window(&w.id)).collect()
    };
    let review: Vec<&Window> = windows
        .iter()
        .filter(|w| {
            w.id.starts_with("code_review_five_hour")
                || w.id.starts_with("code_review_weekly")
                || w.id.starts_with("code_review_window:")
        })
        .collect();
    let model_windows: Vec<&Window> =
        windows.iter().filter(|w| w.kind.as_deref() == Some("model")).collect();
    let recognized =
        |w: &Window| [&account, &spend, &review, &model_windows].iter().any(|set| set.contains(&w));
    if windows.iter().any(|w| !recognized(w)) {
        return partial();
    }
    let mut effective = Vec::new();
    if !account.is_empty() {
        effective.push(availability("all_models", &account));
    }
    if !review.is_empty() {
        effective.push(availability("code_review", &review));
    }
    let mut scopes: Vec<(String, Vec<&Window>)> = Vec::new();
    for w in &model_windows {
        let scope = codex_model_scope(&w.id);
        match scopes.iter_mut().find(|(s, _)| *s == scope) {
            Some((_, list)) => list.push(w),
            None => scopes.push((scope, vec![w])),
        }
    }
    for (scope, own) in scopes {
        let mut bounds = account.clone();
        bounds.extend(own.iter().copied());
        let live = own.iter().all(|w| w.percent_remaining.is_some_and(|p| p > 0.0));
        let conflict = live && account.iter().any(|w| w.percent_remaining == Some(0.0));
        let mut e = availability(&scope, &bounds);
        if conflict {
            e.status = "unknown".into();
            e.effective_percent_remaining = None;
            e.limiting_window_ids.clear();
        }
        effective.push(e);
    }
    known(effective)
}

/// Antigravity's two scopes, each bound by its own five-hour and weekly
/// windows (`gemini_5h`, `gemini_weekly`).
pub const AGY_SCOPES: [&str; 2] = ["gemini", "claude_gpt"];

/// Gemini's five-hour and weekly windows bound Gemini; Claude/GPT's bound
/// Claude/GPT. Other windows stay apart.
pub fn agy(windows: &[Window]) -> QuotaSemantics {
    let gemini: Vec<&Window> =
        windows.iter().filter(|w| w.id == "gemini_5h" || w.id == "gemini_weekly").collect();
    let claude_gpt: Vec<&Window> =
        windows.iter().filter(|w| w.id == "claude_gpt_5h" || w.id == "claude_gpt_weekly").collect();
    let mut effective = Vec::new();
    if !gemini.is_empty() {
        effective.push(availability("gemini", &gemini));
    }
    if !claude_gpt.is_empty() {
        effective.push(availability("claude_gpt", &claude_gpt));
    }
    let unresolved = windows.len() > gemini.len() + claude_gpt.len();
    if unresolved {
        return QuotaSemantics {
            status: if effective.is_empty() { "unknown" } else { "partial" }.into(),
            effective_availability: effective,
        };
    }
    known(effective)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(id: &str, kind: &str, left: Option<f64>) -> Window {
        Window {
            id: id.into(),
            kind: Some(kind.into()),
            percent_remaining: left,
            percent_used: left.map(|l| 100.0 - l),
            ..Window::default()
        }
    }

    #[test]
    fn claude_models_are_bounded_by_the_account_windows_too() {
        let s = claude(&[
            w("five_hour", "session", Some(62.0)),
            w("seven_day", "weekly", Some(40.0)),
            w("model:fable", "model", Some(80.0)),
        ]);
        assert_eq!(s.status, "known");
        let all = &s.effective_availability[0];
        assert_eq!(
            (all.scope.as_str(), all.effective_percent_remaining),
            ("all_models", Some(40.0))
        );
        assert_eq!(all.limiting_window_ids, ["seven_day"]);
        let fable = &s.effective_availability[1];
        assert_eq!(fable.scope, "model:fable");
        assert_eq!(fable.effective_percent_remaining, Some(40.0));
        assert_eq!(fable.bounded_by, ["five_hour", "seven_day", "model:fable"]);
    }

    #[test]
    fn an_unfamiliar_claude_window_leaves_no_number() {
        let s = claude(&[w("five_hour", "session", Some(62.0)), w("odd", "unknown", Some(1.0))]);
        assert_eq!(s.status, "partial");
        assert!(s.effective_availability.is_empty());
    }

    #[test]
    fn codex_scopes_follow_its_ids() {
        assert!(codex_account_window("five_hour"));
        assert!(codex_account_window("weekly_2"));
        assert!(codex_account_window("window:3h"));
        assert!(!codex_account_window("code_review_weekly"));
        assert_eq!(codex_model_scope("model:gpt-5-codex:5h"), "model:gpt-5-codex");
        assert_eq!(codex_model_scope("model:gpt-5-codex:7d_2"), "model:gpt-5-codex");
        assert_eq!(codex_model_scope("model:x:window:3h"), "model:x");
        let s = codex(&[
            w("five_hour", "session", Some(0.0)),
            w("weekly", "weekly", Some(70.0)),
            w("code_review_weekly", "weekly", Some(90.0)),
            w("model:spark:5h", "model", Some(50.0)),
        ]);
        let scope = |name: &str| s.effective_availability.iter().find(|e| e.scope == name).unwrap();
        assert_eq!(scope("all_models").effective_percent_remaining, Some(0.0));
        assert_eq!(scope("code_review").effective_percent_remaining, Some(90.0));
        assert_eq!(scope("model:spark").status, "unknown", "a base zero against a live model");
    }

    #[test]
    fn antigravity_groups_are_apart() {
        let s = agy(&[
            w("gemini_5h", "session", Some(80.0)),
            w("gemini_weekly", "weekly", Some(55.0)),
            w("claude_gpt_5h", "session", Some(100.0)),
        ]);
        assert_eq!(s.status, "known");
        assert_eq!(s.effective_availability[0].effective_percent_remaining, Some(55.0));
        assert_eq!(s.effective_availability[1].effective_percent_remaining, Some(100.0));
        let s = agy(&[w("gemini_5h", "session", Some(80.0)), w("quota_unknown", "unknown", None)]);
        assert_eq!(s.status, "partial");
    }

    #[test]
    fn a_stale_row_has_no_effective_number_and_a_passed_reset_makes_agy_stale() {
        let now = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
        let mut gone = w("gemini_5h", "session", Some(80.0));
        gone.resets_at = Some("2026-01-16T07:00:00.000Z".into());
        let mut row = Provider::fresh(super::super::ProviderId::Agy, vec![gone], now);
        apply(&mut row, now);
        assert_eq!(row.state.status, "stale");
        let s = row.quota_semantics.unwrap();
        assert_eq!(s.status, "unknown");
        assert_eq!(s.effective_availability[0].effective_percent_remaining, None);
    }
}
