// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude accounts' readings, the data file Firstmate reads and the
//! copy its quota-axi reads, and running `claude` as one account, against a fake Keychain and a
//! fake Anthropic in a temporary home. Every login is made up.

mod common;

use quota_station_core::claude::session::{self, SessionError};
use quota_station_core::claude::state;
use quota_station_core::claude::usage;
use quota_station_core::claude::{cc, switch};
use serde_json::Value;
use time::OffsetDateTime;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic, Mac};

/// Accounts 2 and 3, Claude Code on 2.
fn mac(tag: &str) -> (Mac, FakeAnthropic) {
    let mac = Mac::new(tag);
    mac.add(2, "lan@team.example", "org-team", &credential("acc-2", "ref-2", in_hours(5.0)));
    mac.add(3, "lan.b@team.example", "org-team", &credential("acc-3", "ref-3", in_hours(5.0)));
    mac.log_in(
        &credential("acc-2", "ref-2", in_hours(5.0)),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    (mac, FakeAnthropic::start())
}

fn read(mac: &Mac, anthropic: &FakeAnthropic, force: bool) -> usage::Pass {
    usage::read_due(&mac.store, &anthropic.endpoints(), force, true)
}

fn row(mac: &Mac, slot: u32) -> usage::Row {
    usage::load(&mac.store).row(slot).cloned().unwrap()
}

// Readings.

#[test]
fn every_account_is_read_with_its_own_login() {
    let (mac, anthropic) = mac("usage");
    anthropic.usage("acc-2", 30.0, Some("2030-01-16T09:00:00Z"), Some(12.0), Some(40.0));
    anthropic.usage("acc-3", 5.0, None, Some(2.0), None);
    let pass = read(&mac, &anthropic, false);
    assert_eq!(pass.read, vec![2, 3]);
    let two = row(&mac, 2).last_good.unwrap();
    assert_eq!(two.fable().map(|f| f.pct), Some(40.0));
    assert_eq!(two.five_hour.unwrap().pct, 30.0);
    let requests = anthropic.to("/api/oauth/usage");
    assert!(requests.iter().all(|r| r.header("anthropic-beta").as_deref() == Some(usage::BETA)));
}

#[test]
fn a_reading_younger_than_three_minutes_is_reused() {
    let (mac, anthropic) = mac("usage-reuse");
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage("acc-3", 5.0, None, None, None);
    read(&mac, &anthropic, false);
    let calls = anthropic.requests().len();
    assert_eq!(read(&mac, &anthropic, false).read, Vec::<u32>::new());
    assert_eq!(anthropic.requests().len(), calls);
    assert_eq!(read(&mac, &anthropic, true).read, vec![2, 3]);
}

#[test]
fn the_login_in_use_is_never_refreshed_by_the_app() {
    let (mac, anthropic) = mac("usage-live");
    mac.log_in(
        &credential("acc-2", "ref-2", in_hours(-1.0)),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    anthropic.refreshes("ref-2", "acc-2b", "ref-2b");
    anthropic.usage("acc-3", 5.0, None, None, None);
    let pass = read(&mac, &anthropic, false);
    assert_eq!(pass.failed, vec![(2, "token_expired".to_string())]);
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn an_expired_stored_login_is_refreshed_and_kept() {
    let (mac, anthropic) = mac("usage-refresh");
    mac.store.set_credential(3, &credential("acc-3", "ref-3", in_hours(-1.0))).unwrap();
    anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage("acc-3b", 5.0, None, None, None);
    assert_eq!(read(&mac, &anthropic, false).read, vec![2, 3]);
    let kept = mac.store.credential(3).unwrap().unwrap();
    assert_eq!(cc::refresh_token(&kept).as_deref(), Some("ref-3b"));
    let token = anthropic.to("/v1/oauth/token");
    assert_eq!(token.len(), 1);
    assert_eq!(token[0].json()["grant_type"], "refresh_token");
}

#[test]
fn a_refused_token_is_retried_once_after_a_refresh() {
    let (mac, anthropic) = mac("usage-401");
    anthropic.usage_status("acc-3", 401, &[]);
    anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage("acc-3b", 5.0, None, None, None);
    assert_eq!(read(&mac, &anthropic, false).read, vec![2, 3]);
    assert_eq!(anthropic.to("/v1/oauth/token").len(), 1);
}

#[test]
fn a_login_anthropic_no_longer_takes_waits_for_a_new_sign_in() {
    let (mac, anthropic) = mac("usage-relogin");
    mac.store.set_credential(3, &credential("acc-3", "ref-3", in_hours(-1.0))).unwrap();
    anthropic.refuses("ref-3", "invalid_grant");
    anthropic.usage("acc-2", 30.0, None, None, None);
    read(&mac, &anthropic, false);
    assert!(row(&mac, 3).relogin);
    let calls = anthropic.to("/v1/oauth/token").len();
    read(&mac, &anthropic, true);
    assert_eq!(anthropic.to("/v1/oauth/token").len(), calls);
}

/// Marks `slot`'s row as waiting for a new sign-in, its back-off over.
fn mark_relogin(mac: &Mac, slot: u32) {
    let identity = mac.store.load().unwrap().get(slot).unwrap().identity.clone();
    let mut cache = usage::load(&mac.store);
    cache.accounts.insert(
        slot.to_string(),
        usage::Row {
            identity,
            relogin: true,
            failures: 2,
            last_error: Some("invalid_grant".into()),
            backoff_until: Some(1.0),
            ..usage::Row::default()
        },
    );
    usage::save(&mac.store, &cache).unwrap();
}

#[test]
fn the_account_in_use_is_read_even_after_its_stored_login_was_refused() {
    let (mac, anthropic) = mac("usage-relogin-active");
    mark_relogin(&mac, 2);
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage("acc-3", 5.0, None, None, None);
    assert_eq!(read(&mac, &anthropic, false).read, vec![2, 3]);
    assert!(!row(&mac, 2).relogin);
}

/// A Claude Code session running in `slot`'s folder, as this test process.
fn hold_session(mac: &Mac, slot: u32) {
    let sessions = session::folder(&mac.store, slot).join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let record = format!("{{\"pid\": {}}}", std::process::id());
    std::fs::write(sessions.join("1.json"), record).unwrap();
}

#[test]
fn a_login_a_running_session_holds_is_not_refreshed_by_the_app() {
    let (mac, anthropic) = mac("usage-session-holds");
    let folder = session::prepare(&mac.store, 3).unwrap();
    let service = cc::profile_service(&folder).unwrap();
    mac.put_item(&service, &mac.user(), &credential("acc-3w", "ref-3", in_hours(1.0)));
    let expired = credential("acc-3", "ref-3", in_hours(-1.0));
    mac.store.set_credential(3, &expired).unwrap();
    hold_session(&mac, 3);
    anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage("acc-3w", 5.0, None, None, None);
    assert_eq!(read(&mac, &anthropic, false).read, vec![2, 3]);
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(mac.store.credential(3).unwrap(), Some(expired));
}

#[test]
fn a_login_claude_code_holds_is_not_refreshed_by_the_app() {
    let (mac, anthropic) = mac("usage-live-holds");
    let expired = credential("acc-3", "ref-3", in_hours(-1.0));
    mac.store.set_credential(3, &expired).unwrap();
    // Claude Code was switched to 3 while `~/.claude.json` still names 2.
    mac.put_item(
        "Claude Code-credentials",
        &mac.user(),
        &credential("acc-3l", "ref-3", in_hours(1.0)),
    );
    anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
    anthropic.usage("acc-3l", 5.0, None, None, None);
    let pass = read(&mac, &anthropic, false);
    assert!(pass.read.contains(&3), "{pass:?}");
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(mac.store.credential(3).unwrap(), Some(expired));
}

#[test]
fn a_rate_limit_backs_off() {
    let (mac, anthropic) = mac("usage-429");
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage_status("acc-3", 429, &[("retry-after", "120")]);
    let pass = read(&mac, &anthropic, false);
    assert_eq!(pass.failed, vec![(3, "http-429".to_string())]);
    let three = row(&mac, 3);
    let wait = three.backoff_until.unwrap() - three.last_attempt_at.unwrap();
    assert!(wait >= 360.0, "{wait}");
}

// The data file.

#[test]
fn the_data_file_has_every_account_and_no_secret_or_email() {
    let (mac, anthropic) = mac("state");
    mac.show_firstmate(true);
    let mut list = mac.store.load().unwrap();
    list.get_mut(3).unwrap().disabled = true;
    mac.store.save(&list).unwrap();
    anthropic.usage("acc-2", 30.0, Some("2030-01-16T09:00:00Z"), Some(12.0), None);
    anthropic.usage("acc-3", 5.0, None, None, None);
    read(&mac, &anthropic, false);
    state::write(&mac.store, switch::active_slot(&mac.store), OffsetDateTime::now_utc()).unwrap();
    let text = std::fs::read_to_string(mac.paths.state()).unwrap();
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["activeSlot"], 2);
    let accounts = data["accounts"].as_array().unwrap();
    assert_eq!(
        accounts.iter().map(|a| a["slot"].as_u64().unwrap()).collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(accounts[0]["enabled"], true);
    assert_eq!(accounts[1]["enabled"], false);
    assert_eq!(accounts[0]["status"], "fresh");
    assert_eq!(accounts[0]["windows"]["fiveHour"]["usedPercent"], 30.0);
    assert_eq!(accounts[0]["windows"]["fiveHour"]["resetsAt"], "2030-01-16T09:00:00Z");
    for file in [
        mac.paths.state(),
        mac.paths.swap().join("sequence.json"),
        mac.paths.swap().join("cache/usage.json"),
    ] {
        let text = std::fs::read_to_string(&file).unwrap();
        for secret in ["acc-2", "acc-3", "ref-2", "ref-3", "@team.example"] {
            assert!(!text.contains(secret), "{} holds {secret}", file.display());
        }
    }
}

#[test]
fn the_copy_for_firstmate_pairs_each_reading_with_its_account() {
    let (mac, anthropic) = mac("state-swap");
    mac.show_firstmate(true);
    anthropic.usage("acc-2", 30.0, None, None, None);
    anthropic.usage("acc-3", 5.0, None, None, None);
    read(&mac, &anthropic, false);
    state::write(&mac.store, switch::active_slot(&mac.store), OffsetDateTime::now_utc()).unwrap();
    let swap = mac.paths.swap();
    let sequence: Value =
        serde_json::from_str(&std::fs::read_to_string(swap.join("sequence.json")).unwrap())
            .unwrap();
    let usage: Value =
        serde_json::from_str(&std::fs::read_to_string(swap.join("cache/usage.json")).unwrap())
            .unwrap();
    assert_eq!(sequence["activeAccountNumber"], 2);
    assert_eq!(usage["schemaVersion"], 2);
    for slot in ["2", "3"] {
        assert_eq!(sequence["accounts"][slot]["email"], usage["accounts"][slot]["email"]);
        assert_eq!(
            sequence["accounts"][slot]["organizationUuid"],
            usage["accounts"][slot]["organizationUuid"]
        );
    }
    assert_eq!(usage["accounts"]["3"]["lastGood"]["five_hour"]["pct"], 5.0);
}

// Running `claude` as one account.

/// A fake `claude` that writes where it ran and with which folder, and, when
/// `refresh` is given, puts that login in its folder's Keychain item as
/// Claude Code does after a refresh.
fn fake_claude(mac: &Mac, refresh: Option<&str>) -> std::path::PathBuf {
    let out = mac.dir.join("claude-ran");
    let items = mac.dir.join("items");
    let refresh = match refresh {
        Some(c) => {
            let service = cc::profile_service(&session::folder(&mac.store, 3)).unwrap();
            format!("printf '%s' '{c}' > '{}/{service}--{}'\n", items.display(), mac.user())
        }
        None => String::new(),
    };
    common::script(
        &mac.dir.join("bin/claude"),
        &format!(
            "printf '%s\\n' \"${{CLAUDE_CONFIG_DIR:-none}}\" \"$*\" > '{}'\n{refresh}exit 7\n",
            out.display()
        ),
    )
}

#[test]
fn claude_runs_as_another_account_in_its_own_folder() {
    let (mac, _) = mac("session");
    let claude = fake_claude(&mac, None);
    let code = session::run(&mac.store, &claude, 3, true, &["-p".into(), "ok".into()]).unwrap();
    assert_eq!(code, 7);
    let ran = std::fs::read_to_string(mac.dir.join("claude-ran")).unwrap();
    let folder = session::folder(&mac.store, 3);
    assert_eq!(ran, format!("{}\n-p ok\n", folder.display()));
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join(".claude.json")).unwrap())
            .unwrap();
    assert_eq!(config["oauthAccount"]["emailAddress"], "lan.b@team.example");
    let service = cc::profile_service(&folder).unwrap();
    assert_eq!(mac.item(&service, &mac.user()), mac.store.credential(3).unwrap());
    // Claude Code's default login is not touched.
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

#[test]
fn the_account_in_use_runs_as_claude_code_does_unless_a_session_is_required() {
    let (mac, _) = mac("session-active");
    let claude = fake_claude(&mac, None);
    assert_eq!(session::run(&mac.store, &claude, 2, false, &[]).unwrap(), 7);
    assert!(std::fs::read_to_string(mac.dir.join("claude-ran")).unwrap().starts_with("none\n"));
    assert!(matches!(session::run(&mac.store, &claude, 2, true, &[]), Err(SessionError::IsActive)));
}

#[test]
fn a_login_claude_code_refreshed_in_the_folder_comes_back_to_the_app() {
    let (mac, _) = mac("session-adopt");
    let newer = credential("acc-3b", "ref-3b", in_hours(9.0));
    let claude = fake_claude(&mac, Some(&newer));
    session::run(&mac.store, &claude, 3, true, &[]).unwrap();
    assert_eq!(mac.store.credential(3).unwrap(), Some(newer));
}

#[test]
fn a_login_that_comes_back_from_the_folder_is_read_again() {
    let (mac, _) = mac("session-adopt-relogin");
    mark_relogin(&mac, 3);
    let claude = fake_claude(&mac, Some(&credential("acc-3b", "ref-3b", in_hours(9.0))));
    session::run(&mac.store, &claude, 3, true, &[]).unwrap();
    let three = row(&mac, 3);
    assert_eq!((three.relogin, three.failures, three.backoff_until), (false, 0, None));
}
