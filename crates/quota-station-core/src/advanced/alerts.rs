// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's alerts, kinds the app knows only through the hooks
//! ([`crate::ext::Hooks::alert_kinds`]), both on by default:
//!
//! - 2, an account about to leave the rotation, 30 minutes before its
//!   `expire` in the report's rotation, else in the rotation file this Mac
//!   reads ([`Source::rules`]). Nâng cao holds it while it is off
//!   (`super::settings::SettingsHooks`).
//! - 9, "Tự chuyển khi hết quota" moved Claude to another account: the
//!   rotation's last log line ([`Source::last_log`]) when it is such a
//!   switch, at most [`SWITCHED_WITHIN`] old. Once per switch. Shown with
//!   Nâng cao off too: it is a main feature.
//!
//! [`KINDS`] holds their numbers, switches and the words of their buttons;
//! [`Source`] draws them at a pass of the rules, from what the app read for
//! that pass; [`AlertHooks`] gives the core the kinds and the cards
//! "Gửi thử" sends.

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::alerts::{
    act, clock, name_of, Alert, ExtAnswer, ExtKind, ExtPass, ExtSource, Kind, PoolCheck, Tone,
};
use crate::ext::Hooks;
use crate::local_time;
use crate::quota::Response;
use crate::rotation::Rules;
use crate::switcher::{self, Decision, LogEvent, LogLine, Why};

/// Alert 2 comes this long before an account leaves the rotation.
pub const LEAVING_BEFORE: i64 = 30 * 60;

/// Alert 2's name: its cards' `kind` and its switch in the settings file.
pub const LEAVING: &str = "leaving";

/// Alert 9's name: its cards' `kind` and its switch in the settings file.
pub const SWITCHED: &str = "switched";

/// Alert 9 tells of a switch at most this long after it, so a start of the
/// app long after a switch does not tell of it as news.
pub const SWITCHED_WITHIN: i64 = 30 * 60;

/// Alerts 2 and 9.
pub const KINDS: [ExtKind; 2] = [
    ExtKind {
        number: 2,
        name: LEAVING,
        on: true,
        answers: &[ExtAnswer {
            action: "dismiss",
            title: "Đã ghi nhận",
            body: "Không nhắc lại cho lần rời vòng này.",
        }],
    },
    ExtKind {
        number: 9,
        name: SWITCHED,
        on: true,
        answers: &[ExtAnswer { action: "dismiss", title: "Đã ghi nhận", body: "" }],
    },
];

/// Alert 2's kind.
pub fn leaving() -> Kind {
    Kind::Ext(LEAVING.into())
}

/// Alert 9's kind.
pub fn switched() -> Kind {
    Kind::Ext(SWITCHED.into())
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
    /// The rotation's last log line, for alert 9 (`None` where it did not read).
    pub last_log: Option<&'a LogLine>,
}

impl ExtSource for Source<'_> {
    fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert> {
        let mut out = Vec::new();
        if let Some(report) = pass.facts.claude {
            out.extend(leaving_alerts(report, self.rules, pass));
        }
        out.extend(self.last_log.and_then(|line| switched_alert(line, pass)));
        out
    }
}

/// `5 giờ`, `tuần`.
fn window_words(window: &str) -> &'static str {
    if window == "week" {
        "tuần"
    } else {
        "5 giờ"
    }
}

/// Alert 9 for the rotation's last log line, when it is a switch of
/// "Tự chuyển khi hết quota" made in the last [`SWITCHED_WITHIN`].
fn switched_alert(line: &LogLine, pass: &ExtPass<'_>) -> Option<Alert> {
    let LogEvent::Switch { slot } = line.event else { return None };
    let at = line.at.as_deref().and_then(instant)?.unix_timestamp();
    let now = pass.facts.now.unix_timestamp();
    if !(0..=SWITCHED_WITHIN).contains(&(now - at)) {
        return None;
    }
    // `switch to slot 3 (full 5h: …)`, read as the decision it logged.
    let reason = line.raw.split_once(&format!("switch to slot {slot} "))?.1;
    let Decision::Switch { why: Why::Full { window, from, at: full, .. }, .. } =
        switcher::parse_decision(&format!("switch {slot} {reason}"))
    else {
        return None;
    };
    let name = name_of(pass.names, slot);
    Some(Alert {
        key: format!("switched:{slot}:{at}"),
        kind: switched(),
        tone: Tone::Info,
        title: format!("Đã tự chuyển Claude sang {name}"),
        body: format!(
            "{} đã dùng {}% hạn mức {}. Phiên Claude Code đang chạy vẫn chạy tiếp.",
            name_of(pass.names, from),
            full as i64,
            window_words(&window)
        ),
        actions: vec![act("dismiss", "Ổn")],
        at,
        test: false,
    })
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
        SWITCHED => (
            Tone::Info,
            "Đã tự chuyển Claude sang tài khoản khác".to_string(),
            format!(
                "Thử: {a} đã dùng 90% hạn mức 5 giờ. Phiên Claude Code đang chạy vẫn chạy tiếp."
            ),
            vec![act("dismiss", "Ổn")],
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
