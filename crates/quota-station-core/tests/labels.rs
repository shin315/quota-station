// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The names the app shows for the accounts where no name is shown,
//! pinned for one fixed set of accounts in each build: four Claude
//! accounts (`claude-pool.json`, the second in use) and two Antigravity
//! accounts, none with a name shown. Where they show: the menu bar's
//! title, the cards of a pass of the alert rules, the cards "Gửi thử"
//! sends and the line under a card after a switch. The names come from the
//! hooks the app passes ([`Hooks::account_labels`]) for its settings.

mod common;

use std::collections::BTreeMap;

use common::fixture;
use quota_station_core::alerts::{self, Facts, Kind, Memory, Switches};
use quota_station_core::ext::Hooks;
use quota_station_core::icon::{self, CodexUse, Context};
use quota_station_core::settings::{Menubar, MenubarWindow, PoolAlert, Settings};
use quota_station_core::Snapshot;
use time::{OffsetDateTime, UtcOffset};

/// 14:20 at UTC+7, the fixtures' afternoon.
const T0: i64 = 1_768_548_000;

fn offset() -> UtcOffset {
    UtcOffset::from_hms(7, 0, 0).unwrap()
}

/// Every name the app shows for the fixed accounts.
#[derive(Debug, PartialEq)]
struct Shown {
    /// The menu bar's title on Claude, then on Gemini.
    titles: [String; 2],
    /// Each Claude account as the windows' icon names it.
    icon_claude: Vec<String>,
    /// Each Antigravity account as the icon names it.
    icon_agy: Vec<String>,
    /// The Claude accounts as the cards name them.
    cards: BTreeMap<u32, String>,
    /// Alert 1's card at a pass where the pool is dry: its title, its
    /// body and its buttons.
    pool_card: (String, String, Vec<String>),
    /// "Gửi thử" for alert 1: its title, its body and its buttons.
    pool_sample: (String, String, Vec<String>),
    /// "Gửi thử" for alert 3: its title.
    login_sample: String,
    /// "Gửi thử" for alert 4: its body.
    overwrite_sample: String,
    /// The line under alert 4's card when the switch to Antigravity
    /// account 2 could not start.
    #[cfg(feature = "antigravity-accounts")]
    refused: String,
}

fn snapshot() -> Snapshot {
    let mut s = Snapshot::build(Some(&fixture("claude-pool.json")), None);
    // Two Antigravity accounts, the first in use, without a name shown.
    let mut agy = Snapshot::build(None, Some(&fixture("agy-single.json"))).agy;
    agy.accounts[0].number = 1;
    agy.accounts[0].email = None;
    let mut second = agy.accounts[0].clone();
    second.number = 2;
    second.email = None;
    second.active = false;
    agy.accounts.push(second);
    s.agy = agy;
    s
}

fn texts(actions: &[alerts::Act]) -> Vec<String> {
    actions.iter().map(|a| a.label.clone()).collect()
}

/// What the app shows with `hooks` and `settings`.
fn shown(hooks: &dyn Hooks, settings: &Settings) -> Shown {
    let labels = hooks.account_labels(settings);
    let labels = labels.as_deref();
    let s = snapshot();
    let report = fixture("claude-pool.json");
    let now = OffsetDateTime::from_unix_timestamp(T0).unwrap();
    let title = |follow| {
        let cx = Context {
            follow,
            window: MenubarWindow::Tightest,
            codex: CodexUse::ChatGpt,
            now,
            labels,
        };
        icon::icon(Some(&s), &cx).title
    };
    // A pass where any account in use runs dry and no other has room.
    let facts = Facts {
        now,
        offset: offset(),
        claude: Some(&report),
        pool_policy: None,
        #[cfg(feature = "antigravity-accounts")]
        agy: Vec::new(),
        #[cfg(feature = "antigravity-accounts")]
        overwrite: None,
        ext: None,
        pool: PoolAlert { dry_at_most: 100, room_at_least: 101 },
        labels,
    };
    let mut memory = Memory::default();
    let ids = alerts::evaluate(&mut memory, &facts, &Switches::default(), hooks);
    let card = &memory.record(&ids[0]).unwrap().alert;
    let sample = |kind| alerts::sample(&kind, T0, offset(), labels, hooks).unwrap();
    let pool = sample(Kind::Pool);
    Shown {
        titles: [title(Menubar::Claude), title(Menubar::Gemini)],
        icon_claude: s
            .claude
            .accounts
            .iter()
            .map(|a| icon::claude_name(&s.claude, a, labels))
            .collect(),
        icon_agy: s.agy.accounts.iter().map(|a| icon::agy_name(a, labels)).collect(),
        cards: alerts::claude_names(Some(&report), labels),
        pool_card: (card.title.clone(), card.body.clone(), texts(&card.actions)),
        pool_sample: (pool.title, pool.body, texts(&pool.actions)),
        login_sample: sample(Kind::Login).title,
        overwrite_sample: sample(Kind::Overwrite).body,
        #[cfg(feature = "antigravity-accounts")]
        refused: alerts::agy_refused(2, "x", labels).title,
    }
}

fn strings<const N: usize>(names: [&str; N]) -> Vec<String> {
    names.map(String::from).to_vec()
}

/// The app's own names: by number.
fn by_number() -> Shown {
    Shown {
        titles: ["Tài khoản 2 87%".into(), "Antigravity… 6%".into()],
        icon_claude: strings(["Tài khoản 1", "Tài khoản 2", "Tài khoản 3", "Tài khoản 4"]),
        icon_agy: strings(["Antigravity 1", "Antigravity 2"]),
        cards: (1..=4).map(|n| (n, format!("Tài khoản {n}"))).collect(),
        pool_card: (
            "Hạn mức Claude sắp cạn".into(),
            "Tài khoản 2 đã dùng 73%. Tài khoản 4 có lại hạn mức lúc 17:09.".into(),
            strings(["Sang Tài khoản 3 (đã dùng 0%)", "Bỏ qua"]),
        ),
        pool_sample: (
            "Hạn mức Claude sắp cạn".into(),
            "Thử: Tài khoản 1 đã dùng 92%. Tài khoản 2 có lại hạn mức lúc 16:20.".into(),
            strings(["Sang Tài khoản 3 (đã dùng 81%)", "Bỏ qua"]),
        ),
        login_sample: "Tài khoản 2 cần đăng nhập lại".into(),
        overwrite_sample: "Thử: lúc 14:20 Antigravity.app đổi về Antigravity 1. \
                           Bạn đã chuyển sang Antigravity 2 lúc 14:00."
            .into(),
        #[cfg(feature = "antigravity-accounts")]
        refused: "Chưa chuyển lại được sang Antigravity 2".into(),
    }
}

/// Without an extension's names the accounts go by number everywhere, in
/// every build.
#[test]
fn without_an_extensions_names_the_accounts_go_by_number() {
    assert_eq!(shown(quota_station_core::ext::none(), &Settings::default()), by_number());
}
