// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The alerts (F09), each sent once for its event, as a macOS card with
//! buttons and as the same card under the bell (Thông báo) in the main window:
//!
//! 1. Claude about to run dry, and back. Dry: the account in use has 10 %
//!    or less, and no other account has 15 % or more; both thresholds are
//!    set in Cài đặt ([`PoolAlert`]). Back: no longer dry. An extension's
//!    pool policy says which accounts count ([`Facts::pool_policy`]).
//!    With a rotation file only the accounts the switcher may choose now
//!    count ([`crate::advanced::pool`]).
//! 2. An account about to leave the rotation: 30 minutes before its
//!    `expire` in the rotation file.
//! 3. A login to renew: a Claude account whose login ends within 3 days, or
//!    whose last reading failed with a login only a new one mends; an
//!    Antigravity account here that needs a new login or a verification.
//! 4. An Antigravity switch overwritten: the six-hour watch after a switch
//!    (M4c) saw the old account back in use.
//!
//! Alert 2 is the advanced part's kind ([`crate::advanced::alerts`]), added
//! through the hooks like any extension's.
//!
//! An extension may add kinds of its own beside these ([`ExtKind`], named by
//! [`Hooks::alert_kinds`]), with a number, a switch and the words of their
//! buttons, and their alerts at each pass of the rules ([`ExtSource`],
//! handed over in [`Facts::ext`]). They are sent, remembered, switched off
//! and answered as the app's own are.
//!
//! And one more of its own (M11, G), not among the five: skills with a newer
//! version on GitHub ([`skill_alert`]). It shows in the app; a macOS card
//! only when the owner turned that on in Cài đặt.
//!
//! And one for each environment variable that overrides a login (M14, I):
//! [`env_alerts`], in the app only, never a macOS card. It names the
//! variable and where it is set, never its value.
//!
//! An alert's key names its event (the account, the time it ends), and a
//! key is sent once; "Nhắc ngày mai" sends it again the next morning while
//! it still holds. The cards name accounts the way the windows do
//! (Tài khoản 1, Antigravity 1, or the extension's names for them,
//! [`Facts::labels`]), never by email. What each button does is
//! [`answer`]'s: some only note the answer, the others return an [`Effect`]
//! for the app to carry out.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use time::{OffsetDateTime, UtcOffset, Weekday};

use crate::agents::env_check;
#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::index::{self, Standing};
#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::switch::{How, Outcome as AgyOutcome};
#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::watch;
use crate::app_dir::AppDir;
use crate::ext::Hooks;
use crate::labels::Labels;
use crate::quota::claude::PoolPolicy;
use crate::quota::{Response, Slot};
use crate::settings::PoolAlert;
use crate::snapshot::Snapshot;
use crate::time_util::parse_ts;

/// In the app's folder, `alerts/alerts.json`.
pub const DIR: &str = "alerts";
pub const FILE: &str = "alerts.json";
/// Alert 3 comes when a Claude login ends within this long.
pub const LOGIN_WITHIN: i64 = 3 * 86400;
/// Alert 4 offers to switch back for this many overwrites of one switch.
pub const OVERWRITE_OFFERS: u32 = 2;
/// "Nhắc ngày mai" reminds at this hour.
const REMIND_HOUR: u8 = 9;
const RECENT_KEEP: usize = 30;
/// A sent key is forgotten after this long.
const SENT_KEEP: i64 = 60 * 86400;
/// The usage reading's errors that only a new login mends: a refresh token
/// Anthropic refused, or a login without one.
const RELOGIN_ERRORS: [&str; 4] =
    ["invalid_grant", "no_refresh_token", "no-access-token", "relogin_required"];

/// A kind of alert: one of the app's own, or one an extension adds
/// ([`Kind::Ext`]). Written as its name: `pool`, `login`, …
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Pool,
    Login,
    Overwrite,
    /// Skills with a newer version (M11): not one of the five, no switch here.
    Skill,
    /// A variable overriding a login (M14): in the app only, no switch.
    Env,
    /// Claude Code, Codex or agy with a newer version: in the app only, no switch.
    Cli,
    /// One of the extension's kinds ([`Hooks::alert_kinds`]), by its name.
    /// Any name that is not one of the app's reads as one; a build whose
    /// extension has no kind of that name ([`Kind::known`]) has no such
    /// kind.
    Ext(String),
}

/// The app's own kinds, by the names they are written as.
const OWN_KINDS: [(&str, Kind); 6] = [
    ("pool", Kind::Pool),
    ("login", Kind::Login),
    ("overwrite", Kind::Overwrite),
    ("skill", Kind::Skill),
    ("env", Kind::Env),
    ("cli", Kind::Cli),
];

impl Kind {
    /// The app's own kinds with a switch in Cài đặt › Thông báo; with the
    /// extension's, [`Kind::all`].
    pub const ALL: [Kind; 3] = [Kind::Pool, Kind::Login, Kind::Overwrite];

    /// Every kind with a switch, the app's and the extension's, in the
    /// order of their numbers.
    pub fn all(hooks: &dyn Hooks) -> Vec<Kind> {
        let mut all: Vec<Kind> = Kind::ALL.to_vec();
        all.extend(ext_kinds(hooks).map(|k| Kind::Ext(k.name.to_string())));
        all.sort_by_key(|k| k.number(hooks));
        all
    }

    /// The kind written as `name`: the app's own of that name, else the
    /// extension's.
    pub fn named(name: &str) -> Kind {
        OWN_KINDS
            .iter()
            .find(|(own, _)| *own == name)
            .map_or_else(|| Kind::Ext(name.to_string()), |(_, kind)| kind.clone())
    }

    /// Its name, as the cards and the windows write it.
    pub fn name(&self) -> &str {
        match self {
            Kind::Ext(name) => name,
            own => OWN_KINDS.iter().find(|(_, k)| k == own).map_or("", |(name, _)| name),
        }
    }

    /// Its number in the plan and in Cài đặt › Thông báo; 0 for a kind
    /// this build does not have.
    pub fn number(&self, hooks: &dyn Hooks) -> u8 {
        match self {
            Kind::Pool => 1,
            Kind::Login => 3,
            Kind::Overwrite => 4,
            Kind::Skill => 6,
            Kind::Env => 7,
            Kind::Cli => 8,
            Kind::Ext(_) => self.ext(hooks).map_or(0, |k| k.number),
        }
    }

    /// The extension's kind this is, if it is one of them.
    pub fn ext(&self, hooks: &dyn Hooks) -> Option<&'static ExtKind> {
        let Kind::Ext(name) = self else { return None };
        ext_kinds(hooks).find(|k| k.name == name)
    }

    /// This build has the kind: it is the app's own, or one of the
    /// extension's.
    pub fn known(&self, hooks: &dyn Hooks) -> bool {
        !matches!(self, Kind::Ext(_)) || self.ext(hooks).is_some()
    }
}

impl Serialize for Kind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Kind, D::Error> {
        String::deserialize(deserializer).map(|name| Kind::named(&name))
    }
}

/// A kind of alert an extension adds to the app's own
/// ([`Hooks::alert_kinds`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtKind {
    /// Its number in Cài đặt › Thông báo, one the app's own kinds do not
    /// have (theirs are 1, 3, 4, 6, 7 and 8). Among the alerts found at one
    /// pass, its cards come in the order of this number.
    pub number: u8,
    /// Its name: its cards' `kind`, and its switch's key in the settings
    /// file's `alerts`. A name of one of the app's own kinds stays the
    /// app's.
    pub name: &'static str,
    /// Its switch where the settings file does not set it.
    pub on: bool,
    /// What its cards say once a button is pressed, where that is not what
    /// the app says for that button.
    pub answers: &'static [ExtAnswer],
}

/// What a card of an extension's kind says under itself once the button
/// `action` is pressed ([`ExtKind::answers`]), in place of the app's words
/// for that button. The button still does what it does on any card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtAnswer {
    pub action: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

/// The extension's kinds, less any named like one of the app's own.
fn ext_kinds(hooks: &dyn Hooks) -> impl Iterator<Item = &'static ExtKind> {
    hooks.alert_kinds().iter().filter(|k| !OWN_KINDS.iter().any(|(own, _)| *own == k.name))
}

/// Which alerts are on: every one of the app's by default, and each of the
/// extension's as its kind says ([`ExtKind::on`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Switches {
    pub pool: bool,
    pub login: bool,
    pub overwrite: bool,
    /// The switches of the extension's kinds, by their names, and any other
    /// key of the settings file's `alerts` this build does not know, as it
    /// was read: written back beside the app's own. A kind's switch missing
    /// here is at its default.
    #[serde(flatten)]
    pub ext: Map<String, Value>,
}

impl Default for Switches {
    fn default() -> Self {
        Switches { pool: true, login: true, overwrite: true, ext: Map::new() }
    }
}

impl Switches {
    /// Whether alerts of `kind` are sent. A kind this build does not have
    /// never is.
    pub fn on(&self, kind: &Kind, hooks: &dyn Hooks) -> bool {
        match kind {
            Kind::Pool => self.pool,
            Kind::Login => self.login,
            Kind::Overwrite => self.overwrite,
            // Its own settings say whether the app checks at all.
            Kind::Skill | Kind::Env | Kind::Cli => true,
            Kind::Ext(name) => kind
                .ext(hooks)
                .is_some_and(|k| self.ext.get(name).and_then(Value::as_bool).unwrap_or(k.on)),
        }
    }

    /// Turns `kind` on or off, and keeps the switches as the settings file
    /// keeps them ([`Switches::tidy`]). A kind without a switch, or one this
    /// build does not have, changes nothing.
    pub fn set(&mut self, kind: &Kind, on: bool, hooks: &dyn Hooks) {
        match kind {
            Kind::Pool => self.pool = on,
            Kind::Login => self.login = on,
            Kind::Overwrite => self.overwrite = on,
            Kind::Skill | Kind::Env | Kind::Cli => {}
            Kind::Ext(name) => {
                if kind.ext(hooks).is_some() {
                    self.ext.insert(name.clone(), Value::Bool(on));
                }
            }
        }
        self.tidy(hooks);
    }

    /// The switches as the settings file keeps them: while every switch,
    /// the app's and the extension's, is at its default, the extension's
    /// are left out (and with nothing else kept, so is `alerts`); otherwise
    /// each of the extension's is written, at its default where it was
    /// missing or not `true` or `false`. Keys that are no kind's stay as
    /// they were.
    pub fn tidy(&mut self, hooks: &dyn Hooks) {
        let values: Vec<(&ExtKind, bool)> = ext_kinds(hooks)
            .map(|k| (k, self.ext.get(k.name).and_then(Value::as_bool).unwrap_or(k.on)))
            .collect();
        let default =
            self.pool && self.login && self.overwrite && values.iter().all(|(k, on)| *on == k.on);
        for (k, on) in values {
            if default {
                self.ext.remove(k.name);
            } else {
                self.ext.insert(k.name.to_string(), Value::Bool(on));
            }
        }
    }

    /// Every switch with a kind, the extension's at their values: what
    /// Cài đặt › Thông báo shows. Keys that are no kind's are left out.
    pub fn shown(&self, hooks: &dyn Hooks) -> Switches {
        let ext = ext_kinds(hooks)
            .map(|k| (k.name.to_string(), Value::Bool(self.on(&Kind::Ext(k.name.into()), hooks))))
            .collect();
        Switches { pool: self.pool, login: self.login, overwrite: self.overwrite, ext }
    }

    pub fn is_default(&self) -> bool {
        *self == Switches::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Ok,
    Warn,
    Bad,
    Info,
}

/// One button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    /// `dismiss`, `snooze`, `switch:N`, `agy:keep`, `agy:<how>:N`,
    /// `agy:login` or `agy:verify:N`.
    pub id: String,
    pub label: String,
}

/// A button `label` that does `id`.
pub fn act(id: impl Into<String>, label: impl Into<String>) -> Act {
    Act { id: id.into(), label: label.into() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    /// Names the event; one key is sent once.
    pub key: String,
    pub kind: Kind,
    pub tone: Tone,
    pub title: String,
    pub body: String,
    pub actions: Vec<Act>,
    /// Unix seconds.
    pub at: i64,
    /// Sent with "Gửi thử": its buttons change nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub test: bool,
}

/// What a button led to, shown under the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Said {
    pub tone: Tone,
    pub title: String,
    pub body: String,
}

fn said(tone: Tone, title: impl Into<String>, body: impl Into<String>) -> Said {
    Said { tone, title: title.into(), body: body.into() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub action: String,
    pub label: String,
    pub at: i64,
    /// `None` while the action runs.
    pub result: Option<Said>,
}

/// One alert as sent: a macOS card and a line in the bell's panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    /// The card's id, unique to this sending (a reminder is a new one).
    pub id: String,
    pub alert: Alert,
    /// macOS took the card; false when notifications are off or cannot
    /// be shown.
    pub notified: bool,
    pub answer: Option<Answer>,
    /// The bell's panel was opened since it came. A card kept before the
    /// bell counted reads counts as read, so the bell starts at nothing.
    #[serde(default = "kept_read")]
    pub read: bool,
}

fn kept_read() -> bool {
    true
}

/// What the app keeps between reads and restarts, in `alerts/alerts.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Memory {
    /// The pool rule's last answer, for alert 1's two edges.
    pub pool_dry: Option<bool>,
    /// Keys sent, and when first.
    pub sent: BTreeMap<String, i64>,
    /// Keys to send again ("Nhắc ngày mai"), and when.
    pub snoozed: BTreeMap<String, i64>,
    /// Newest first.
    pub recent: Vec<Record>,
    /// What the extension's alerts keep between passes
    /// ([`ExtPass::memory`]), and any other key of the file this build does
    /// not know, as it was read: written back beside the app's own.
    #[serde(flatten)]
    pub ext: Map<String, Value>,
}

impl Memory {
    pub fn record(&self, id: &str) -> Option<&Record> {
        self.recent.iter().find(|r| r.id == id)
    }

    /// Cards with buttons nobody has pressed yet.
    pub fn unanswered(&self) -> usize {
        self.recent
            .iter()
            .filter(|r| !r.alert.test && r.answer.is_none() && !r.alert.actions.is_empty())
            .count()
    }

    /// Cards that came since the bell's panel was last opened: its count.
    pub fn unread(&self) -> usize {
        self.recent.iter().filter(|r| !r.read).count()
    }

    /// The bell's panel opened: every card in it is read. Whether any was not.
    pub fn mark_read(&mut self) -> bool {
        let mut changed = false;
        for r in self.recent.iter_mut().filter(|r| !r.read) {
            r.read = true;
            changed = true;
        }
        changed
    }
}

pub fn path(app: &AppDir) -> PathBuf {
    app.path().join(DIR).join(FILE)
}

/// What was kept; none, or an unreadable file, starts afresh. So does a
/// file with a card of a kind this build does not have ([`Kind::known`]
/// with `hooks`): it was never this build's to read.
pub fn load(app: &AppDir, hooks: &dyn Hooks) -> Memory {
    let memory: Memory = app
        .read_to_string(&path(app))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if memory.recent.iter().any(|r| !r.alert.kind.known(hooks)) {
        return Memory::default();
    }
    memory
}

pub fn save(app: &AppDir, memory: &Memory) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(memory).map_err(std::io::Error::other)?;
    app.write_private(&path(app), &text)
}

// ===== What the rules look at =====

/// An Antigravity account logged in here that needs the owner.
#[cfg(feature = "antigravity-accounts")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgyLogin {
    /// Its place in the app's list, from 1.
    pub number: u32,
    pub id: String,
    pub standing: Standing,
}

/// A switch a running Antigravity.app undid, not answered yet.
#[cfg(feature = "antigravity-accounts")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overwrite {
    /// The account switched to, by id and number.
    pub to_id: String,
    pub to: u32,
    /// The account back in use.
    pub from: u32,
    /// When the switch was made, Unix seconds.
    pub switched_at: i64,
    /// When a reading showed the old account back.
    pub at: i64,
    pub overwrites: u32,
    /// Antigravity.app runs; `None` when that could not be told.
    pub running: Option<bool>,
}

pub struct Facts<'a> {
    pub now: OffsetDateTime,
    /// This Mac's offset, for the clock times on the cards and the personal
    /// hours.
    pub offset: UtcOffset,
    pub claude: Option<&'a Response>,
    /// The extension's policy of the Claude pool, read for this pass
    /// ([`Hooks::claude_pool`]): which accounts count in alert 1's check.
    /// `None`: every account counts.
    pub pool_policy: Option<&'a dyn PoolPolicy>,
    #[cfg(feature = "antigravity-accounts")]
    pub agy: Vec<AgyLogin>,
    #[cfg(feature = "antigravity-accounts")]
    pub overwrite: Option<Overwrite>,
    /// The extension's alerts at this pass, from what it read for it; `None`
    /// without an extension, or one that adds none.
    pub ext: Option<&'a dyn ExtSource>,
    /// Alert 1's thresholds, from settings.
    pub pool: PoolAlert,
    /// The extension's names for the accounts on the cards, and alert 1's
    /// title, for the settings in force
    /// ([`Hooks::account_labels`]); `None`: the app's own.
    pub labels: Option<&'a dyn Labels>,
}

/// The Antigravity accounts here that need a new login or a verification.
#[cfg(feature = "antigravity-accounts")]
pub fn agy_logins(app: &AppDir) -> Vec<AgyLogin> {
    let Ok(index) = index::load(app) else { return Vec::new() };
    let statuses = index::load_statuses(app);
    index
        .accounts
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            statuses.accounts.get(&a.id).map(|s| AgyLogin {
                number: i as u32 + 1,
                id: a.id.clone(),
                standing: s.clone(),
            })
        })
        .collect()
}

/// The overwrite the six-hour watch holds, with the accounts by number.
#[cfg(feature = "antigravity-accounts")]
pub fn overwrite(app: &AppDir, running: Option<bool>) -> Option<Overwrite> {
    let w = watch::load(app)?;
    let at = w.alert()?;
    let accounts = index::load(app).ok()?.accounts;
    let number = |id: &str| accounts.iter().position(|a| a.id == id).map(|i| i as u32 + 1);
    Some(Overwrite {
        to: number(&w.to)?,
        from: number(&w.from)?,
        to_id: w.to.clone(),
        switched_at: w.at,
        at,
        overwrites: w.overwrites,
        running,
    })
}

/// The pool rule on the accounts' numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolCheck {
    pub dry: bool,
    pub active: Option<u32>,
    /// Percent left of the account in use; 100 when none is known.
    pub active_left: f64,
    /// The accounts that count besides the one in use, with their percent
    /// left.
    pub others: Vec<(u32, f64)>,
    /// The first reset among the usable accounts under 15 %, and whose.
    pub next_reset: Option<(u32, String)>,
}

impl PoolCheck {
    /// The other account with the most left, when it has more than the one
    /// in use.
    pub fn better(&self) -> Option<(u32, f64)> {
        self.best_other().filter(|(_, left)| *left > self.active_left)
    }

    /// The other account with the most left.
    pub fn best_other(&self) -> Option<(u32, f64)> {
        self.others.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// The dry rule, with the thresholds in `limits`: every account counts, as
/// any can be switched to by hand, unless the extension's pool `policy`
/// says one does not count at `now` (`offset` is this Mac's)
/// ([`PoolPolicy::counts`]). An account past its reset counts as full.
/// `None` without the accounts' numbers.
pub fn pool_check(
    report: &Response,
    policy: Option<&dyn PoolPolicy>,
    limits: PoolAlert,
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Option<PoolCheck> {
    let row = report.primary("claude")?;
    let swap = row.claude_accounts.as_ref()?;
    let (dry_at_most, room_at_least) =
        (f64::from(limits.dry_at_most), f64::from(limits.room_at_least));
    let left = |s: &Slot| {
        if s.status == "reset_pending" {
            100.0
        } else {
            s.effective_percent_remaining.unwrap_or(0.0)
        }
    };
    let active = swap.active_slot;
    let usable: Vec<&Slot> = swap
        .slots
        .iter()
        .filter(|s| policy.is_none_or(|p| p.counts(swap, s, now, offset)))
        .collect();
    let others: Vec<(u32, f64)> =
        usable.iter().filter(|s| Some(s.slot) != active).map(|s| (s.slot, left(s))).collect();
    let active_left =
        active.and_then(|a| swap.slots.iter().find(|s| s.slot == a)).map_or(100.0, left);
    let dry = active_left <= dry_at_most && !others.iter().any(|(_, l)| *l >= room_at_least);
    let next_reset = usable
        .iter()
        .filter(|s| left(s) < room_at_least)
        .filter_map(|s| {
            let at = s.resets_at.as_ref()?;
            parse_ts(at).map(|t| (t, s.slot, at.clone()))
        })
        .min_by_key(|(t, _, _)| *t)
        .map(|(_, slot, at)| (slot, at));
    Some(PoolCheck { dry, active, active_left, others, next_reset })
}

/// Claude accounts by the names the windows give them: the extension's
/// ([`Labels::claude`]), else `Tài khoản N`.
pub fn claude_names(
    report: Option<&Response>,
    labels: Option<&dyn Labels>,
) -> BTreeMap<u32, String> {
    let section = Snapshot::build(report, None).claude;
    section
        .accounts
        .iter()
        .filter(|a| a.number > 0)
        .map(|a| {
            let name = labels
                .and_then(|l| l.claude(&section, a))
                .unwrap_or_else(|| format!("Tài khoản {}", a.number));
            (a.number, name)
        })
        .collect()
}

/// Antigravity account `n` by the name the windows give it: the
/// extension's ([`Labels::agy`]), else `Antigravity N`.
fn agy_name(n: u32, labels: Option<&dyn Labels>) -> String {
    labels.and_then(|l| l.agy(n)).unwrap_or_else(|| format!("Antigravity {n}"))
}

// ===== Words =====

fn local(at: i64, offset: UtcOffset) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(at).unwrap_or(OffsetDateTime::UNIX_EPOCH).to_offset(offset)
}

/// `14:20` on this Mac's clock.
pub fn clock(at: i64, offset: UtcOffset) -> String {
    let t = local(at, offset);
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn weekday(w: Weekday) -> &'static str {
    match w {
        Weekday::Monday => "Thứ Hai",
        Weekday::Tuesday => "Thứ Ba",
        Weekday::Wednesday => "Thứ Tư",
        Weekday::Thursday => "Thứ Năm",
        Weekday::Friday => "Thứ Sáu",
        Weekday::Saturday => "Thứ Bảy",
        Weekday::Sunday => "Chủ nhật",
    }
}

/// `Chủ nhật 19/10`.
pub fn day(at: i64, offset: UtcOffset) -> String {
    let t = local(at, offset);
    format!("{} {:02}/{:02}", weekday(t.weekday()), t.day(), u8::from(t.month()))
}

/// Claude account `slot` by its name in `names`, else `Tài khoản N`.
pub fn name_of(names: &BTreeMap<u32, String>, slot: u32) -> String {
    names.get(&slot).cloned().unwrap_or_else(|| format!("Tài khoản {slot}"))
}

/// The next morning at [`REMIND_HOUR`], this Mac's clock.
pub fn remind_at(now: i64, offset: UtcOffset) -> i64 {
    let today = local(now, offset).date();
    let date = today.next_day().unwrap_or(today);
    date.with_hms(REMIND_HOUR, 0, 0)
        .map(|t| t.assume_offset(offset).unix_timestamp())
        .unwrap_or(now + 86400)
}

// ===== The rules =====

/// The account in use and its share used.
fn pool_amount(c: &PoolCheck, names: &BTreeMap<u32, String>) -> String {
    match c.active {
        Some(a) => format!("{} đã dùng {}%", name_of(names, a), (100.0 - c.active_left).round()),
        None => "Hạn mức Claude gần hết".to_string(),
    }
}

/// The account with the most left now, the one in use among them, and its
/// share used: which account has quota again.
fn pool_back(c: &PoolCheck, names: &BTreeMap<u32, String>) -> String {
    let best = c.active.map(|a| (a, c.active_left)).into_iter().chain(c.best_other());
    match best.max_by(|a, b| a.1.total_cmp(&b.1)) {
        Some((slot, left)) => {
            format!("{} có lại hạn mức: đã dùng {}%.", name_of(names, slot), (100.0 - left).round())
        }
        None => "Có lại hạn mức Claude.".to_string(),
    }
}

fn pool_alerts(
    c: &PoolCheck,
    was_dry: bool,
    names: &BTreeMap<u32, String>,
    f: &Facts,
) -> Option<Alert> {
    let now = f.now.unix_timestamp();
    if c.dry && !was_dry {
        let mut body = pool_amount(c, names) + ".";
        if let Some((slot, at)) = &c.next_reset {
            if let Some(t) = parse_ts(at) {
                body += &format!(
                    " {} có lại hạn mức lúc {}.",
                    name_of(names, *slot),
                    clock(t.unix_timestamp(), f.offset)
                );
            }
        }
        let mut actions = Vec::new();
        if let Some((slot, left)) = c.better() {
            actions.push(act(
                format!("switch:{slot}"),
                format!("Sang {} (đã dùng {}%)", name_of(names, slot), (100.0 - left).round()),
            ));
        }
        actions.push(act("dismiss", "Bỏ qua"));
        let title = pool_title(f.labels);
        return Some(Alert {
            key: format!("pool:dry:{now}"),
            kind: Kind::Pool,
            tone: Tone::Bad,
            title: title.into(),
            body,
            actions,
            at: now,
            test: false,
        });
    }
    if !c.dry && was_dry {
        let body = pool_back(c, names);
        return Some(Alert {
            key: format!("pool:back:{now}"),
            kind: Kind::Pool,
            tone: Tone::Ok,
            title: "Claude đã có lại hạn mức".into(),
            body,
            actions: Vec::new(),
            at: now,
            test: false,
        });
    }
    None
}

/// Alert 1's title when the pool runs dry: the extension's
/// ([`Labels::pool_title`]), else the app's.
fn pool_title(labels: Option<&dyn Labels>) -> &'static str {
    labels.and_then(|l| l.pool_title()).unwrap_or("Hạn mức Claude sắp cạn")
}

/// A reading error that only a new login mends, as one of the known
/// codes (`relogin` for any other that asks for one). The card and its key
/// carry only this code, never the error's own text.
pub fn relogin_code(error: &str) -> Option<&'static str> {
    let e = error.trim().to_ascii_lowercase();
    if let Some(code) = RELOGIN_ERRORS.iter().find(|c| **c == e) {
        Some(code)
    } else if e.contains("relogin") || e.contains("re-login") {
        Some("relogin")
    } else {
        None
    }
}

pub fn relogin_error(error: &str) -> bool {
    relogin_code(error).is_some()
}

const CLAUDE_RELOGIN: &str = "Đăng nhập lại ở tab Claude.";

fn login_alerts(report: &Response, names: &BTreeMap<u32, String>, f: &Facts) -> Vec<Alert> {
    let now = f.now.unix_timestamp();
    let Some(list) = report.primary("claude").and_then(|r| r.claude_accounts.as_ref()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for slot in &list.slots {
        let name = name_of(names, slot.slot);
        let actions = vec![act("snooze", "Nhắc ngày mai"), act("dismiss", "Đã biết")];
        if let Some(error) = slot.error.as_deref().and_then(relogin_code) {
            out.push(Alert {
                key: format!("login:claude:{}:error:{error}", slot.slot),
                kind: Kind::Login,
                tone: Tone::Warn,
                title: format!("{name} cần đăng nhập lại"),
                body: format!(
                    "App không làm mới được đăng nhập của tài khoản này ({error}). {CLAUDE_RELOGIN}"
                ),
                actions,
                at: now,
                test: false,
            });
        }
    }
    out
}

#[cfg(feature = "antigravity-accounts")]
fn agy_login_alerts(logins: &[AgyLogin], now: i64, labels: Option<&dyn Labels>) -> Vec<Alert> {
    logins
        .iter()
        .map(|l| {
            let name = agy_name(l.number, labels);
            let (state, since, title, body, url) = match &l.standing {
                Standing::NeedsLogin { since } => (
                    "login",
                    *since,
                    format!("{name} cần đăng nhập lại"),
                    "Google không nhận đăng nhập của tài khoản này nữa. Đăng nhập lại trong tab Antigravity.".to_string(),
                    false,
                ),
                Standing::NeedsVerification { since, url } => (
                    "verify",
                    *since,
                    format!("{name} cần xác minh"),
                    "Google cần xác minh tài khoản này trước khi cho đọc hạn mức.".to_string(),
                    url.is_some(),
                ),
                Standing::Forbidden { since, url } => (
                    "forbidden",
                    *since,
                    format!("{name} cần đăng nhập lại"),
                    "Google chặn đọc hạn mức của tài khoản này. App đọc lại sau khi đăng nhập lại.".to_string(),
                    url.is_some(),
                ),
            };
            let mut actions = Vec::new();
            if url {
                actions.push(act(format!("agy:verify:{}", l.number), "Mở trang xác minh"));
            }
            if state != "verify" {
                actions.push(act("agy:login", "Đăng nhập lại"));
            }
            actions.push(act("dismiss", "Đã biết"));
            Alert {
                key: format!("login:agy:{}:{state}:{since}", l.id),
                kind: Kind::Login,
                tone: Tone::Warn,
                title,
                body,
                actions,
                at: now,
                test: false,
            }
        })
        .collect()
}

#[cfg(feature = "antigravity-accounts")]
fn overwrite_alert(o: &Overwrite, f: &Facts) -> Alert {
    let now = f.now.unix_timestamp();
    let (to, from) = (agy_name(o.to, f.labels), agy_name(o.from, f.labels));
    let mut body = format!(
        "Lúc {} Antigravity.app đổi về {from}. Bạn đã chuyển sang {to} lúc {}.",
        clock(o.at, f.offset),
        clock(o.switched_at, f.offset)
    );
    if o.overwrites >= 2 {
        body += &format!(" Đây là lần thứ {}.", o.overwrites);
    }
    let mut actions = Vec::new();
    if o.overwrites <= OVERWRITE_OFFERS {
        match o.running {
            Some(true) => {
                actions.push(act(format!("agy:quitFirst:{}", o.to), "Thoát app rồi chuyển lại"));
                actions.push(act(format!("agy:keepRunning:{}", o.to), "Chuyển lại"));
            }
            Some(false) => actions.push(act(format!("agy:plain:{}", o.to), "Chuyển lại")),
            None => actions.push(act(format!("agy:keepRunning:{}", o.to), "Chuyển lại")),
        }
    } else {
        body += " Ứng dụng ngừng đề nghị chuyển lại.";
    }
    actions.push(act("agy:keep", "Bỏ qua"));
    Alert {
        key: format!("overwrite:{}:{}", o.to_id, o.at),
        kind: Kind::Overwrite,
        tone: Tone::Warn,
        title: "Antigravity đổi về tài khoản cũ".into(),
        body,
        actions,
        at: now,
        test: false,
    }
}

/// An extension's alerts at one pass of the rules ([`evaluate`]), drawn
/// from what it read for that pass; the app hands it over in [`Facts::ext`].
pub trait ExtSource {
    /// Its alerts whose event holds now, each of one of its kinds
    /// ([`Hooks::alert_kinds`]). They go out as the app's own do: a key
    /// once, a kind switched off never, and "Nhắc ngày mai" again the next
    /// morning while the event still holds.
    fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert>;
}

/// What an extension's alerts look at ([`ExtSource::alerts`]).
pub struct ExtPass<'a> {
    /// The newest readings, as the app's own rules see them.
    pub facts: &'a Facts<'a>,
    /// Alert 1's check on the Claude accounts, when they were read.
    pub check: Option<&'a PoolCheck>,
    /// The Claude accounts by the names the cards give them.
    pub names: &'a BTreeMap<u32, String>,
    /// What the extension's alerts keep from one pass to the next (an
    /// event's edge, say), in `alerts/alerts.json` beside the app's own
    /// ([`Memory::ext`]). Its keys must not be any of the app's.
    pub memory: &'a mut Map<String, Value>,
}

/// Every alert whose event holds now. Alert 1 comes only on its edges,
/// which `memory` keeps. The extension's come among the app's in the order
/// of their kinds' numbers.
fn found(memory: &mut Memory, f: &Facts, hooks: &dyn Hooks) -> Vec<Alert> {
    let names = claude_names(f.claude, f.labels);
    let check = f.claude.and_then(|r| pool_check(r, f.pool_policy, f.pool, f.now, f.offset));
    let mut out = Vec::new();
    if let Some(c) = &check {
        out.extend(pool_alerts(c, memory.pool_dry.unwrap_or(false), &names, f));
        memory.pool_dry = Some(c.dry);
    }
    if let Some(report) = f.claude {
        out.extend(login_alerts(report, &names, f));
    }
    #[cfg(feature = "antigravity-accounts")]
    {
        out.extend(agy_login_alerts(&f.agy, f.now.unix_timestamp(), f.labels));
        out.extend(f.overwrite.as_ref().map(|o| overwrite_alert(o, f)));
    }
    if let Some(source) = f.ext {
        let mut pass =
            ExtPass { facts: f, check: check.as_ref(), names: &names, memory: &mut memory.ext };
        out.extend(source.alerts(&mut pass));
        out.sort_by_key(|a| a.kind.number(hooks));
    }
    out
}

/// Runs the rules on the newest readings. Returns the ids of the cards to
/// show now, each kept in `memory.recent`: alerts of kinds turned off are
/// left out, a key already sent is not sent again, and a reminder whose
/// time came is sent again while its event still holds. `hooks` has the
/// extension's kinds, whose alerts come from [`Facts::ext`].
pub fn evaluate(
    memory: &mut Memory,
    facts: &Facts,
    switches: &Switches,
    hooks: &dyn Hooks,
) -> Vec<String> {
    let now = facts.now.unix_timestamp();
    let found = found(memory, facts, hooks);
    let holding: BTreeSet<String> = found.iter().map(|a| a.key.clone()).collect();
    // A dead login whose error went away (logged in again) may come back
    // later under the same key; forget it, but only while the accounts are read.
    let claude_read =
        facts.claude.and_then(|r| r.primary("claude")).is_some_and(|r| r.claude_accounts.is_some());
    if claude_read {
        memory.sent.retain(|k, _| {
            !(k.starts_with("login:claude:") && k.contains(":error:")) || holding.contains(k)
        });
    }
    let mut shown = Vec::new();
    for alert in found {
        if !switches.on(&alert.kind, hooks) {
            continue;
        }
        let due = memory.snoozed.get(&alert.key).is_some_and(|t| *t <= now);
        if memory.sent.contains_key(&alert.key) && !due {
            continue;
        }
        memory.snoozed.remove(&alert.key);
        memory.sent.entry(alert.key.clone()).or_insert(now);
        let id = format!("{}@{now}", alert.key);
        memory.recent.retain(|r| r.id != id);
        memory.recent.insert(0, Record::new(id.clone(), alert));
        shown.push(id);
    }
    memory.snoozed.retain(|k, _| holding.contains(k));
    memory.sent.retain(|_, at| now - *at <= SENT_KEEP);
    memory.recent.truncate(RECENT_KEEP);
    shown
}

// ===== Cards and buttons =====

/// A card for macOS, as the notification center shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub id: String,
    pub kind: Kind,
    /// Its kind's number ([`Kind::number`]).
    pub number: u8,
    pub title: String,
    pub body: String,
    pub actions: Vec<Act>,
}

impl Record {
    /// A card just sent: not yet on macOS, answered or read.
    fn new(id: String, alert: Alert) -> Record {
        Record { id, alert, notified: false, answer: None, read: false }
    }

    /// The card for macOS; `hooks` numbers the extension's kinds.
    pub fn card(&self, hooks: &dyn Hooks) -> Card {
        Card {
            id: self.id.clone(),
            kind: self.alert.kind.clone(),
            number: self.alert.kind.number(hooks),
            title: self.alert.title.clone(),
            body: self.alert.body.clone(),
            actions: self.alert.actions.clone(),
        }
    }
}

/// The macOS notification center, or a fake one in the tests. A button
/// pressed on a card comes back through [`answer`] with the card's id.
pub trait Notifier: Send + Sync {
    fn show(&self, card: &Card) -> Result<(), String>;
    /// Takes a card off screen once it was answered in the app.
    fn withdraw(&self, id: &str);
}

/// Shows the cards `ids`, noting the ones macOS took.
pub fn deliver(memory: &mut Memory, ids: &[String], center: &dyn Notifier, hooks: &dyn Hooks) {
    for id in ids {
        if let Some(r) = memory.recent.iter_mut().find(|r| &r.id == id) {
            r.notified = center.show(&r.card(hooks)).is_ok();
        }
    }
}

/// What the app does for a button, beyond noting the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    None,
    /// The switch sheet for this Claude account, to confirm there.
    OpenClaudeSwitch {
        slot: u32,
    },
    /// Switch Antigravity back to this account; [`settle`] takes how it went.
    #[cfg(feature = "antigravity-accounts")]
    SwitchAgy {
        number: u32,
        how: How,
    },
    /// Stop the six-hour watch.
    DismissOverwrite,
    /// The Antigravity tab, where the account logs in again.
    OpenAgyTab,
    OpenVerification {
        number: u32,
    },
    /// The Skills tab, where the new versions are.
    OpenSkills,
    /// Cài đặt's Dịch vụ tab, where the variables' card is.
    OpenSettings,
    /// Agent › Cập nhật, the Cập nhật Agent page.
    OpenCliTools,
    /// "Bỏ qua" on a variable: every place the card named.
    IgnoreEnv {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerError {
    /// No card with this id is kept.
    Unknown,
    /// Pressed already, here or on the other card.
    Answered,
    NoSuchAction,
}

impl AnswerError {
    pub fn code(&self) -> &'static str {
        match self {
            AnswerError::Unknown => "unknown",
            AnswerError::Answered => "answered",
            AnswerError::NoSuchAction => "noSuchAction",
        }
    }
}

#[cfg(feature = "antigravity-accounts")]
fn how(word: &str) -> Option<How> {
    match word {
        "plain" => Some(How::Plain),
        "quitFirst" => Some(How::QuitFirst),
        "keepRunning" => Some(How::KeepRunning),
        _ => None,
    }
}

/// The button `action` on card `id`, pressed at `now`, on the macOS card or
/// the one in the app: notes the answer (once per card), sets a reminder
/// for "Nhắc ngày mai", and says what else the app must do. A card of one
/// of the extension's kinds (`hooks`) says what its kind says for the
/// button ([`ExtKind::answers`]), where it says something, and alert 1's
/// what the extension's pool says ([`Hooks::pool_answers`]).
pub fn answer(
    memory: &mut Memory,
    id: &str,
    action: &str,
    now: i64,
    offset: UtcOffset,
    hooks: &dyn Hooks,
) -> Result<Effect, AnswerError> {
    let Memory { recent, snoozed, .. } = memory;
    let record = recent.iter_mut().find(|r| r.id == id).ok_or(AnswerError::Unknown)?;
    if record.answer.is_some() {
        return Err(AnswerError::Answered);
    }
    let pressed = record
        .alert
        .actions
        .iter()
        .find(|a| a.id == action)
        .cloned()
        .ok_or(AnswerError::NoSuchAction)?;
    let a = &record.alert;
    let (effect, mut said) = if a.test {
        (
            Effect::None,
            Some(said(
                Tone::Info,
                "Thông báo thử",
                format!("Bạn đã bấm “{}”. Thông báo thử không làm gì cả.", pressed.label),
            )),
        )
    } else if action == "dismiss" {
        let said = match a.kind {
            Kind::Pool => said(
                Tone::Info,
                "Đã bỏ qua thông báo này",
                "Không lặp lại cho cùng sự kiện. Quota Station báo lại khi có lại hạn mức.",
            ),
            _ => said(Tone::Info, "Đã ghi nhận", "Không nhắc lại cho lần này."),
        };
        (Effect::None, Some(said))
    } else if action == "snooze" {
        let at = remind_at(now, offset);
        snoozed.insert(a.key.clone(), at);
        let said = said(
            Tone::Info,
            format!("Sẽ nhắc lại lúc {} ngày mai", clock(at, offset)),
            format!("{}.", day(at, offset)),
        );
        (Effect::None, Some(said))
    } else if action == "agy:keep" {
        let said = said(
            Tone::Info,
            "Đã bỏ qua",
            "Antigravity giữ tài khoản đang dùng. Ứng dụng ngừng theo dõi lần chuyển này.",
        );
        (Effect::DismissOverwrite, Some(said))
    } else if action == "skills:open" {
        let said = said(
            Tone::Info,
            "Đã mở tab Skills",
            "Bấm Cập nhật ở skill bạn muốn. App không tự cập nhật.",
        );
        (Effect::OpenSkills, Some(said))
    } else if action == "cli:settings" {
        let said = said(
            Tone::Info,
            "Đã mở Cập nhật Agent",
            "Ở nhóm Agent, mục Cập nhật. App chỉ cập nhật khi bạn bấm.",
        );
        (Effect::OpenCliTools, Some(said))
    } else if action == "env:settings" {
        let said =
            said(Tone::Info, "Đã mở Cài đặt", "Ở tab Dịch vụ. App chỉ sửa file khi bạn bấm Gỡ.");
        (Effect::OpenSettings, Some(said))
    } else if let Some(name) = action.strip_prefix("env:ignore:") {
        let said = said(
            Tone::Info,
            format!("Đã bỏ qua {name}"),
            "Báo lại nếu biến được đặt ở chỗ khác. App không sửa file shell.",
        );
        (Effect::IgnoreEnv { name: name.to_string() }, Some(said))
    } else if action == "agy:login" {
        let said = said(
            Tone::Info,
            "Đã mở tab Antigravity",
            "Bấm Đăng nhập lại ở dòng của tài khoản này.",
        );
        (Effect::OpenAgyTab, Some(said))
    } else if let Some(n) = action.strip_prefix("agy:verify:").and_then(|n| n.parse().ok()) {
        let said =
            said(Tone::Info, "Đã mở trang xác minh", "Trang của Google mở trong trình duyệt.");
        (Effect::OpenVerification { number: n }, Some(said))
    } else if let Some(slot) = action.strip_prefix("switch:").and_then(|n| n.parse().ok()) {
        let said = said(
            Tone::Info,
            "Đã mở hộp thoại chuyển tài khoản",
            "Xác nhận trong hộp thoại để chuyển. Chưa có gì thay đổi.",
        );
        (Effect::OpenClaudeSwitch { slot }, Some(said))
    } else if let Some((h, n)) = action.strip_prefix("agy:").and_then(|r| r.split_once(':')) {
        #[cfg(feature = "antigravity-accounts")]
        match (how(h), n.parse()) {
            (Some(how), Ok(number)) => (Effect::SwitchAgy { number, how }, None),
            _ => return Err(AnswerError::NoSuchAction),
        }
        // Without the Antigravity accounts no card offers a switch back.
        #[cfg(not(feature = "antigravity-accounts"))]
        {
            let _ = (h, n);
            return Err(AnswerError::NoSuchAction);
        }
    } else {
        return Err(AnswerError::NoSuchAction);
    };
    let own = match &a.kind {
        Kind::Pool => hooks.pool_answers().iter().find(|x| x.action == action),
        kind => kind.ext(hooks).and_then(|k| k.answers.iter().find(|x| x.action == action)),
    };
    if let Some(own) = own.filter(|_| !a.test) {
        said = Some(self::said(Tone::Info, own.title, own.body));
    }
    record.answer =
        Some(Answer { action: pressed.id, label: pressed.label, at: now, result: said });
    Ok(effect)
}

/// [`answer`] for a button pressed on the card in the app: once the answer
/// is taken, the macOS card has nothing left to ask and is withdrawn.
pub fn answer_in_app(
    memory: &mut Memory,
    id: &str,
    action: &str,
    now: i64,
    offset: UtcOffset,
    center: &dyn Notifier,
    hooks: &dyn Hooks,
) -> Result<Effect, AnswerError> {
    let effect = answer(memory, id, action, now, offset, hooks)?;
    center.withdraw(id);
    Ok(effect)
}

/// How a switch started from a card ended.
pub fn settle(memory: &mut Memory, id: &str, said: Said) {
    if let Some(answer) =
        memory.recent.iter_mut().find(|r| r.id == id).and_then(|r| r.answer.as_mut())
    {
        answer.result = Some(said);
    }
}

/// The line under the card after "Chuyển lại", naming the accounts as the
/// cards do with `labels` ([`Facts::labels`]).
#[cfg(feature = "antigravity-accounts")]
pub fn agy_result(outcome: &AgyOutcome, labels: Option<&dyn Labels>) -> Said {
    agy_said(outcome, |n| agy_name(n, labels))
}

#[cfg(feature = "antigravity-accounts")]
fn agy_said(outcome: &AgyOutcome, agy_name: impl Fn(u32) -> String) -> Said {
    match outcome {
        AgyOutcome::Switched { number, reopened, .. } => said(
            Tone::Ok,
            format!("Đã chuyển lại sang {}", agy_name(*number as u32)),
            match reopened {
                Some(true) => "Antigravity.app đã mở lại với tài khoản này.",
                Some(false) => "Chưa mở lại được Antigravity.app; mở nó bằng tay.",
                None => "Antigravity.app vẫn mở nên có thể đổi về lần nữa. Nếu vậy, app báo thêm.",
            },
        ),
        AgyOutcome::Unconfirmed { number, .. } => said(
            Tone::Warn,
            format!("Đã ghi đăng nhập {}", agy_name(*number as u32)),
            "agy chưa cho thấy tài khoản mới. Xem lại tab Antigravity sau ít phút.",
        ),
        AgyOutcome::Failed { number, error, .. } => said(
            Tone::Bad,
            format!("Chưa chuyển lại được sang {}", agy_name(*number as u32)),
            format!("Lý do: {error}. Không có gì thay đổi."),
        ),
    }
}

/// A switch from a card that could not start, naming the account as the
/// cards do with `labels` ([`Facts::labels`]).
#[cfg(feature = "antigravity-accounts")]
pub fn agy_refused(number: u32, why: &str, labels: Option<&dyn Labels>) -> Said {
    said(
        Tone::Bad,
        format!("Chưa chuyển lại được sang {}", agy_name(number, labels)),
        format!("Lý do: {why}. Không có gì thay đổi."),
    )
}

/// The card "Gửi thử" sends for `kind`: the prototype's words, marked as a
/// test, with buttons that change nothing, naming the accounts as the
/// cards do with `labels` ([`Facts::labels`]). One of the extension's
/// kinds sends what `hooks` give ([`Hooks::alert_sample`]); `None` when
/// they give nothing, or the build does not have the kind.
pub fn sample(
    kind: &Kind,
    now: i64,
    offset: UtcOffset,
    labels: Option<&dyn Labels>,
    hooks: &dyn Hooks,
) -> Option<Alert> {
    let claude = labels.and_then(|l| l.claude_samples()).unwrap_or([
        "Tài khoản 1",
        "Tài khoản 2",
        "Tài khoản 3",
    ]);
    let agy = [agy_name(1, labels), agy_name(2, labels)];
    let names = (claude, agy.each_ref().map(String::as_str));
    sample_named(kind, now, offset, names, pool_title(labels), hooks)
}

fn sample_named(
    kind: &Kind,
    now: i64,
    offset: UtcOffset,
    ([a, b, c], [g1, g2]): ([&str; 3], [&str; 2]),
    pool: &str,
    hooks: &dyn Hooks,
) -> Option<Alert> {
    let later = clock(now + 2 * 3600, offset);
    let (tone, title, body, actions): (Tone, String, String, Vec<Act>) = match kind {
        Kind::Pool => (
            Tone::Bad,
            pool.into(),
            format!("Thử: {a} đã dùng 92%. {b} có lại hạn mức lúc {later}."),
            vec![act("switch:0", format!("Sang {c} (đã dùng 81%)")), act("dismiss", "Bỏ qua")],
        ),
        Kind::Login => (
            Tone::Warn,
            format!("{b} cần đăng nhập lại"),
            format!(
                "Thử: đăng nhập hết hạn sau 2 ngày ({}). {CLAUDE_RELOGIN}",
                day(now + 2 * 86400, offset)
            ),
            vec![act("snooze", "Nhắc ngày mai"), act("dismiss", "Đã biết")],
        ),
        Kind::Overwrite => (
            Tone::Warn,
            "Antigravity đổi về tài khoản cũ".into(),
            format!(
                "Thử: lúc {} Antigravity.app đổi về {g1}. Bạn đã chuyển sang {g2} lúc {}.",
                clock(now, offset),
                clock(now - 20 * 60, offset)
            ),
            vec![
                act("agy:quitFirst:0", "Thoát app rồi chuyển lại"),
                act("agy:keepRunning:0", "Chuyển lại"),
                act("agy:keep", "Bỏ qua"),
            ],
        ),
        Kind::Skill => (
            Tone::Info,
            "2 skill có bản mới".into(),
            "Thử: ui-polish, sql-helper. App không tự cập nhật.".into(),
            vec![act("skills:open", "Xem"), act("dismiss", "Để sau")],
        ),
        Kind::Env => (
            Tone::Bad,
            "ANTHROPIC_API_KEY đang đè đăng nhập Claude Code".into(),
            "Thử: đặt ở ~/.zshrc dòng 42. App không đọc giá trị.".into(),
            vec![act("env:settings", "Xem"), act("env:ignore:ANTHROPIC_API_KEY", "Bỏ qua")],
        ),
        Kind::Cli => (
            Tone::Info,
            "Codex CLI có bản 0.164.0".into(),
            "Thử: Codex CLI 0.163.0 → 0.164.0. App chỉ cập nhật khi bạn bấm.".into(),
            vec![act("cli:settings", "Xem"), act("dismiss", "Để sau")],
        ),
        Kind::Ext(name) => {
            kind.ext(hooks)?;
            let own = hooks.alert_sample(name, now, offset, [a, b, c])?;
            (own.tone, own.title, own.body, own.actions)
        }
    };
    Some(Alert {
        key: format!("test:{}:{now}", kind.number(hooks)),
        kind: kind.clone(),
        tone,
        title,
        body,
        actions,
        at: now,
        test: true,
    })
}

/// The alert for skills with a newer version (G): one card for each new
/// set of versions (`found` is each skill's name and new tree id), never
/// twice for the same set. Returns its id when it is new.
pub fn skill_alert(memory: &mut Memory, found: &[(String, String)], now: i64) -> Option<String> {
    if found.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = found.iter().map(|(n, to)| format!("{n}@{to}")).collect();
    parts.sort();
    let key = format!("skills:{}", &crate::agents::write::sha256(parts.join(",").as_bytes())[..16]);
    if memory.sent.contains_key(&key) {
        return None;
    }
    let mut names: Vec<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
    names.sort();
    let alert = Alert {
        key: key.clone(),
        kind: Kind::Skill,
        tone: Tone::Info,
        title: format!("{} skill có bản mới", names.len()),
        body: format!("{}. App không tự cập nhật.", names.join(", ")),
        actions: vec![act("skills:open", "Xem"), act("dismiss", "Để sau")],
        at: now,
        test: false,
    };
    memory.sent.insert(key.clone(), now);
    let id = format!("{key}@{now}");
    memory.recent.retain(|r| r.id != id);
    memory.recent.insert(0, Record::new(id.clone(), alert));
    memory.recent.truncate(RECENT_KEEP);
    Some(id)
}

/// The alert for Claude Code, Codex or agy with a newer version: one card
/// for each new set of versions (`found` is each tool's name, its version
/// and the newer one), never twice for the same set. Returns its id when
/// it is new.
pub fn cli_alert(
    memory: &mut Memory,
    found: &[(String, String, String)],
    now: i64,
) -> Option<String> {
    if found.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = found.iter().map(|(n, _, to)| format!("{n}@{to}")).collect();
    parts.sort();
    let key = format!("cli:{}", parts.join(","));
    if memory.sent.contains_key(&key) {
        return None;
    }
    let title = match found {
        [(name, _, to)] => format!("{name} có bản {to}"),
        _ => format!("{} agent có bản mới", found.len()),
    };
    let lines: Vec<String> =
        found.iter().map(|(n, from, to)| format!("{n} {from} → {to}")).collect();
    let alert = Alert {
        key: key.clone(),
        kind: Kind::Cli,
        tone: Tone::Info,
        title,
        body: format!("{}. App chỉ cập nhật khi bạn bấm.", lines.join(", ")),
        actions: vec![act("cli:settings", "Xem"), act("dismiss", "Để sau")],
        at: now,
        test: false,
    };
    memory.sent.insert(key.clone(), now);
    let id = format!("{key}@{now}");
    memory.recent.retain(|r| r.id != id);
    memory.recent.insert(0, Record::new(id.clone(), alert));
    memory.recent.truncate(RECENT_KEEP);
    Some(id)
}

/// The alerts for the variables that override a login (I): one card for
/// each variable, naming every place it is set, sent again only when it is
/// set somewhere new. `found` holds no value, so neither does the card.
/// Returns the new cards' ids.
pub fn env_alerts(memory: &mut Memory, found: &[env_check::Found], now: i64) -> Vec<String> {
    let mut ids = Vec::new();
    for v in &env_check::VARS {
        let here: Vec<&env_check::Found> = found.iter().filter(|f| f.name == v.name).collect();
        let Some(first) = here.first() else { continue };
        let places: Vec<&str> = here.iter().map(|f| f.place.word()).collect();
        let key = format!("env:{}:{}", v.name, places.join(","));
        if memory.sent.contains_key(&key) {
            continue;
        }
        let at: Vec<String> = here.iter().map(|f| f.at()).collect();
        let alert = Alert {
            key: key.clone(),
            kind: Kind::Env,
            tone: match v.weight {
                env_check::Weight::Bad => Tone::Bad,
                env_check::Weight::Warn => Tone::Warn,
            },
            title: format!("{} đang đè đăng nhập {}", v.name, v.agent.label()),
            body: format!("Đặt ở {}. {}", in_words(&at), first.effect),
            actions: vec![
                act("env:settings", "Xem"),
                act(format!("env:ignore:{}", v.name), "Bỏ qua"),
            ],
            at: now,
            test: false,
        };
        memory.sent.insert(key.clone(), now);
        let id = format!("{key}@{now}");
        memory.recent.retain(|r| r.id != id);
        memory.recent.insert(0, Record::new(id.clone(), alert));
        ids.push(id);
    }
    memory.recent.truncate(RECENT_KEEP);
    ids
}

/// `a`, `a và b`, `a, b và c`.
fn in_words(parts: &[String]) -> String {
    match parts.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} và {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// "Bỏ qua" pressed on `name` in Cài đặt or the Claude tab: its cards
/// nobody answered yet take that answer too, so they stop counting.
pub fn env_ignored(memory: &mut Memory, name: &str, now: i64, offset: UtcOffset) -> Vec<String> {
    let action = format!("env:ignore:{name}");
    let ids: Vec<String> = memory
        .recent
        .iter()
        .filter(|r| r.alert.kind == Kind::Env && !r.alert.test && r.answer.is_none())
        .filter(|r| r.alert.actions.iter().any(|a| a.id == action))
        .map(|r| r.id.clone())
        .collect();
    for id in &ids {
        let _ = answer(memory, id, &action, now, offset, crate::ext::none());
    }
    ids
}

/// Keeps a test card in `memory.recent` and returns its id.
pub fn keep_test(memory: &mut Memory, alert: Alert) -> String {
    let id = format!("{}@{}", alert.key, alert.at);
    memory.recent.retain(|r| r.id != id);
    memory.recent.insert(0, Record::new(id.clone(), alert));
    memory.recent.truncate(RECENT_KEEP);
    id
}
