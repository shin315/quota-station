// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Claude Code's login on Windows, against a fake secret store in a
//! temporary home: in `.credentials.json` unless Claude Code's Credential
//! Manager store is on, and then in its item. Every login is made up.

#![cfg(windows)]

mod common;

use std::path::PathBuf;

use quota_station_core::agents::claude_mcp;
use quota_station_core::claude::keychain::user_account;
use quota_station_core::claude::{cc, manage, session, switch};
use serde_json::Value;

use common::accounts::{credential, in_hours, oauth, Mac};

const ITEM: &str = "Claude Code-credentials";

fn file(mac: &Mac) -> PathBuf {
    mac.paths.claude_home().join(".credentials.json")
}

fn access(credential: &str) -> String {
    cc::oauth_part(credential).unwrap()["accessToken"].as_str().unwrap().to_string()
}

/// Team accounts 2 and 3 in the app, Claude Code on 2 with its login in
/// `.credentials.json`, as Claude Code for Windows keeps it by default.
fn two_accounts(tag: &str) -> (Mac, String) {
    let mac = Mac::new(tag);
    let (c2, c3) =
        (credential("acc-2", "ref-2", in_hours(5.0)), credential("acc-3", "ref-3", in_hours(5.0)));
    mac.add(2, "lan@team.example", "org-team", &c2);
    mac.add(3, "lan.b@team.example", "org-team", &c3);
    mac.log_in(&c2, &oauth("lan@team.example", "uuid-2", "org-team"));
    mac.keychain.remove(ITEM, &mac.user());
    std::fs::create_dir_all(mac.paths.claude_home()).unwrap();
    std::fs::write(file(&mac), &c2).unwrap();
    (mac, c2)
}

/// Claude Code's `tengu_windows_credman` flag, on, in `~/.claude.json`.
fn flag_on(mac: &Mac) {
    let path = mac.paths.claude_json();
    let mut config: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    config["cachedGrowthBookFeatures"] = serde_json::json!({ "tengu_windows_credman": true });
    std::fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}

#[test]
fn claude_codes_items_are_named_for_claude_code_user() {
    assert_eq!(user_account(), "claude-code-user");
}

#[test]
fn claude_is_found_as_the_native_claude_exe_or_npms_claude_cmd() {
    let mac = Mac::new("win-find");
    let found = claude_mcp::candidates(&mac.home);
    let bin = mac.home.join(".local/bin");
    assert_eq!(found[..2], [bin.join("claude.exe"), bin.join("claude.cmd")]);
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("claude.exe"), "").unwrap();
    assert_eq!(claude_mcp::discover(&mac.home), Some(bin.join("claude.exe")));
    if std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p)
            .any(|d| d.join("claude.exe").is_file() || d.join("claude.cmd").is_file())
    }) {
        return;
    }
    assert_eq!(session::find_claude(&mac.home), Some(bin.join("claude.exe")));
}

#[test]
fn the_live_login_is_read_from_its_file() {
    let (mac, c2) = two_accounts("win-read");
    assert_eq!(cc::read_credential(&mac.paths, &mac.keychain).unwrap(), Some(c2));
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

#[test]
fn a_switch_writes_the_file_claude_code_reads_and_no_item() {
    let (mac, _) = two_accounts("win-file");
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(access(&std::fs::read_to_string(file(&mac)).unwrap()), "acc-3");
    assert_eq!(mac.item(ITEM, &mac.user()), None);
    let calls = mac.security_calls();
    assert!(
        !calls.lines().any(|c| c.starts_with("add-generic-password") && c.ends_with(ITEM)),
        "{calls}"
    );
    assert_eq!(switch::active_slot(&mac.store), Some(3));
}

#[test]
fn a_switch_makes_the_file_when_claude_code_has_no_login_yet() {
    let (mac, _) = two_accounts("win-none");
    std::fs::remove_file(file(&mac)).unwrap();
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(access(&std::fs::read_to_string(file(&mac)).unwrap()), "acc-3");
    assert_eq!(mac.item(ITEM, &mac.user()), None);
}

#[test]
fn with_claude_codes_credential_manager_store_on_the_item_is_written() {
    let (mac, _) = two_accounts("win-flag");
    flag_on(&mac);
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(access(&mac.item(ITEM, &mac.user()).unwrap()), "acc-3");
    // The file that was there follows, so neither holds an older login.
    assert_eq!(access(&std::fs::read_to_string(file(&mac)).unwrap()), "acc-3");
}

#[test]
fn an_item_already_there_is_where_the_login_lives() {
    let mac = Mac::new("win-item");
    let (c2, c3) =
        (credential("acc-2", "ref-2", in_hours(5.0)), credential("acc-3", "ref-3", in_hours(5.0)));
    mac.add(2, "lan@team.example", "org-team", &c2);
    mac.add(3, "lan.b@team.example", "org-team", &c3);
    mac.log_in(&c2, &oauth("lan@team.example", "uuid-2", "org-team"));
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(access(&mac.item(ITEM, &mac.user()).unwrap()), "acc-3");
    assert!(!file(&mac).exists(), "no file is made beside the item");
}

#[test]
fn a_session_folder_gets_the_login_in_its_own_file() {
    let (mac, _) = two_accounts("win-session");
    let folder = session::prepare(&mac.store, 3).unwrap();
    let written = std::fs::read_to_string(folder.join(".credentials.json")).unwrap();
    assert_eq!(Some(written), mac.store.credential(3).unwrap());
    let profile = cc::profile_service(&folder).unwrap();
    assert_eq!(mac.item(&profile, &mac.user()), None);

    // A login Claude Code renewed there is taken back.
    let renewed = credential("acc-3b", "ref-3b", in_hours(9.0));
    std::fs::write(folder.join(".credentials.json"), &renewed).unwrap();
    session::adopt_newer(&mac.store, 3);
    assert_eq!(mac.store.credential(3).unwrap(), Some(renewed));

    manage::remove(&mac.store, 3).unwrap();
    assert!(!folder.exists());
}
