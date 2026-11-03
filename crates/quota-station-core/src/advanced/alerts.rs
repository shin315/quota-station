// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's alert, a kind the app knows only through the hooks
//! ([`crate::ext::Hooks::alert_kinds`]): 2, an account about to leave the
//! rotation, 30 minutes before its `expire` in the report's rotation, else
//! in the rotation file this Mac reads ([`Source::rules`]). On by default.
//!
//! [`KINDS`] holds its number, switch and the words of its button;
//! [`Source`] draws it at a pass of the rules, from the rotation file as
//! the app read it for that pass; [`AlertHooks`] gives the core the kind
//! and the card "Gửi thử" sends. Nâng cao holds it while it is off
//! (`super::settings::SettingsHooks`).

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::alerts::{
    act, clock, name_of, Alert, ExtAnswer, ExtKind, ExtPass, ExtSource, Kind, PoolCheck, Tone,
};
use crate::ext::Hooks;
use crate::local_time;
use crate::quota::Response;
use crate::rotation::Rules;

/// Alert 2 comes this long before an account leaves the rotation.
pub const LEAVING_BEFORE: i64 = 30 * 60;

/// Alert 2's name: its cards' `kind` and its switch in the settings file.
pub const LEAVING: &str = "leaving";

/// Alert 2.
pub const KINDS: [ExtKind; 1] = [ExtKind {
    number: 2,
    name: LEAVING,
    on: true,
    answers: &[ExtAnswer {
        action: "dismiss",
        title: "Đã ghi nhận",
        body: "Không nhắc lại cho lần rời vòng này.",
    }],
}];

/// Alert 2's kind.
pub fn leaving() -> Kind {
    Kind::Ext(LEAVING.into())
}

/// The advanced part's hooks for the alerts: its kind and the card
/// "Gửi thử" sends for them. The app's extension
/// gives the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct AlertHooks;

impl Hooks for AlertHooks {
    fn alert_kinds(&self) -> &'static [ExtKind] {
        &KINDS
    }

    fn alert_sample(
        &self,
        name: &str,
        now: i64,
        offset: UtcOffset,
        claude: [&str; 3],
    ) -> Option<Alert> {
        sample(name, now, offset, claude)
    }
}

/// Alert 2 at one pass of the rules, with what was read for that pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct Source<'a> {
    /// The rules of the rotation file this Mac follows
    /// ([`super::pool::rules`]), for a report without the rotation's keys
    /// (`None` where it did not read).
    pub rules: Option<&'a Rules>,
}

impl ExtSource for Source<'_> {
    fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert> {
        let mut out = Vec::new();
        if let Some(report) = pass.facts.claude {
            out.extend(leaving_alerts(report, self.rules, pass));
        }
        out
    }
}

/// An `expire` time: RFC 3339 from the report, local `2026-10-16T17:45`
/// from the rotation file.
fn instant(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok().or_else(|| local_time::instant(text))
}

/// `2 ngày`, `5 giờ`, `40 phút`.
fn span(secs: i64) -> String {
    if secs >= 86400 {
        format!("{} ngày", secs / 86400)
    } else if secs >= 3600 {
        format!("{} giờ", secs / 3600)
    } else {
        format!("{} phút", (secs / 60).max(1))
    }
}

fn leaving_alerts(report: &Response, rules: Option<&Rules>, pass: &ExtPass<'_>) -> Vec<Alert> {
    let (f, check) = (pass.facts, pass.check);
    let now = f.now.unix_timestamp();
    // The report's rotation, else the rotation file this Mac reads, as for the pool.
    let rotation = report
        .primary("claude")
        .and_then(|r| r.claude_accounts.as_ref())
        .and_then(super::pool::rotation_in);
    let expire: Vec<(u32, &str)> = match rotation.as_ref() {
        Some(r) => r.expire.iter().map(|e| (e.slot, e.at.as_str())).collect(),
        None => rules
            .map_or_else(Vec::new, |r| r.expire.iter().map(|e| (e.slot, e.at.as_str())).collect()),
    };
    let active = check.and_then(|c| c.active);
    expire
        .into_iter()
        .filter_map(|(slot, at)| {
            let at = instant(at)?.unix_timestamp();
            if !(now < at && at - now <= LEAVING_BEFORE) {
                return None;
            }
            let name = name_of(pass.names, slot);
            let mut body = format!("Còn {}.", span(at - now));
            let mut actions = vec![act("dismiss", "Ổn")];
            if active == Some(slot) {
                body += " Sau đó tự động chuyển sẽ đổi sang tài khoản khác trong vòng xoay.";
                if let Some((slot, _)) = check.and_then(PoolCheck::best_other) {
                    actions.push(act(format!("switch:{slot}"), "Chuyển ngay"));
                }
            } else {
                body += " Sau đó tài khoản này không còn trong vòng.";
            }
            Some(Alert {
                key: format!("leave:{slot}:{at}"),
                kind: leaving(),
                tone: Tone::Warn,
                title: format!("{name} rời vòng lúc {}", clock(at, f.offset)),
                body,
                actions,
                at: now,
                test: false,
            })
        })
        .collect()
}

/// The card "Gửi thử" sends for alert 2: the prototype's words, with `a`
/// the first of the Claude accounts as the cards name them.
pub fn sample(name: &str, now: i64, offset: UtcOffset, [a, _, _]: [&str; 3]) -> Option<Alert> {
    let (tone, title, body, actions) = match name {
        LEAVING => (
            Tone::Warn,
            format!("{a} rời vòng lúc {}", clock(now + 30 * 60, offset)),
            "Thử: còn 30 phút. Sau đó tự động chuyển sẽ đổi sang tài khoản khác trong vòng xoay."
                .to_string(),
            vec![act("dismiss", "Ổn"), act("switch:0", "Chuyển ngay")],
        ),
        _ => return None,
    };
    Some(Alert {
        key: String::new(),
        kind: Kind::Ext(name.into()),
        tone,
        title,
        body,
        actions,
        at: now,
        test: true,
    })
}
