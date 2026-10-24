// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! `Nhập từ claude-swap` against a made-up claude-swap folder and a fake
//! Keychain in a temporary home: what is carried over, that running it
//! again changes nothing, and that claude-swap's own data is never touched.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use quota_station_core::claude::import;
use quota_station_core::claude::store::Kind;
use quota_station_core::claude::usage;
use serde_json::{json, Value};

use common::accounts::{credential, in_hours, oauth, Mac};

const PERSONAL: &str = "lan@example.com";
const TEAM: &str = "lan@team.example";
const TOKEN: &str = "lan.ci@team.example";

/// claude-swap's folder with three accounts: 1 with its login in an `.enc`
/// file, 2 in claude-swap's Keychain item, 3 a setup token with no config.
fn claude_swap(mac: &Mac) -> PathBuf {
    let dir = import::source(&mac.home);
    std::fs::create_dir_all(dir.join("configs")).unwrap();
    std::fs::create_dir_all(dir.join("credentials")).unwrap();
    std::fs::create_dir_all(dir.join("cache")).unwrap();
    let sequence = json!({
        "activeAccountNumber": 2,
        "sequence": [1, 2, 3],
        "accounts": {
            "1": { "email": PERSONAL, "organizationUuid": "org-personal", "uuid": "uuid-1" },
            "2": { "email": TEAM, "organizationUuid": "org-team", "uuid": "uuid-2", "alias": "Team chính" },
            "3": { "email": TOKEN, "organizationUuid": "", "uuid": "" }
        }
    });
    std::fs::write(dir.join("sequence.json"), serde_json::to_string_pretty(&sequence).unwrap())
        .unwrap();
    for (slot, email, org) in [(1, PERSONAL, "org-personal"), (2, TEAM, "org-team")] {
        let config = json!({ "oauthAccount": oauth(email, &format!("uuid-{slot}"), org) });
        std::fs::write(
            dir.join(format!("configs/.claude-config-{slot}-{email}.json")),
            config.to_string(),
        )
        .unwrap();
    }
    write_enc(&dir, 1, PERSONAL, &credential("acc-1", "ref-1", in_hours(5.0)));
    mac.put_item(
        "claude-swap",
        &format!("account-2-{TEAM}"),
        &credential("acc-2", "ref-2", in_hours(5.0)),
    );
    let token =
        json!({ "claudeAiOauth": { "accessToken": "setup-3", "scopes": ["user:inference"] } })
            .to_string();
    mac.put_item("claude-swap", &format!("account-3-{TOKEN}"), &token);
    dir
}

fn write_enc(dir: &Path, slot: u32, email: &str, credential: &str) {
    std::fs::write(
        dir.join(format!("credentials/.creds-{slot}-{email}.enc")),
        STANDARD.encode(credential),
    )
    .unwrap();
}

/// Every file under `dir` and its bytes.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

#[test]
fn every_claude_swap_account_comes_over_under_its_slot() {
    let mac = Mac::new("import");
    let dir = claude_swap(&mac);
    let done = import::import(&mac.store, &dir).unwrap();
    assert_eq!(done.added, vec![1, 2, 3]);
    let list = mac.store.load().unwrap();
    assert_eq!(list.accounts.iter().map(|a| a.slot).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(list.get(2).unwrap().alias.as_deref(), Some("Team chính"));
    assert_eq!(list.get(3).unwrap().kind, Kind::SetupToken);
    assert_eq!(mac.store.credential(1).unwrap(), Some(credential_of(&dir, 1, PERSONAL)));
    assert_eq!(
        mac.store.credential(2).unwrap(),
        mac.item("claude-swap", &format!("account-2-{TEAM}"))
    );
    assert_eq!(mac.store.oauth_account(2).unwrap().unwrap()["emailAddress"], TEAM);
    assert_eq!(mac.store.oauth_account(3).unwrap().unwrap()["emailAddress"], TOKEN);
}

fn credential_of(dir: &Path, slot: u32, email: &str) -> String {
    let text = std::fs::read_to_string(dir.join(format!("credentials/.creds-{slot}-{email}.enc")))
        .unwrap();
    String::from_utf8(STANDARD.decode(text.trim()).unwrap()).unwrap()
}

#[test]
fn claude_swaps_own_data_is_left_as_it_was() {
    let mac = Mac::new("import-untouched");
    let dir = claude_swap(&mac);
    let before = snapshot(&dir);
    let items = snapshot(&mac.dir.join("items"));
    import::import(&mac.store, &dir).unwrap();
    assert_eq!(snapshot(&dir), before);
    let after: BTreeMap<_, _> = snapshot(&mac.dir.join("items"))
        .into_iter()
        .filter(|(p, _)| p.file_name().unwrap().to_string_lossy().starts_with("claude-swap--"))
        .collect();
    let theirs: BTreeMap<_, _> = items
        .into_iter()
        .filter(|(p, _)| p.file_name().unwrap().to_string_lossy().starts_with("claude-swap--"))
        .collect();
    assert_eq!(after, theirs);
    assert!(!mac
        .security_calls()
        .lines()
        .any(|l| !l.starts_with("find-generic-password") && l.contains("-s claude-swap")));
}

#[test]
fn every_account_comes_in_the_rotation_whatever_claude_swap_held_out() {
    let mac = Mac::new("import-in-rotation");
    let dir = claude_swap(&mac);
    let file = dir.join("sequence.json");
    let mut sequence: Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    sequence["accounts"]["2"]["disabled"] = Value::Bool(true);
    std::fs::write(&file, sequence.to_string()).unwrap();
    import::import(&mac.store, &dir).unwrap();
    let list = mac.store.load().unwrap();
    assert!(list.accounts.iter().all(|a| !a.disabled));
    assert_eq!(list.accounts.len(), 3);
}

#[test]
fn importing_again_changes_nothing_the_owner_set() {
    let mac = Mac::new("import-again");
    let dir = claude_swap(&mac);
    import::import(&mac.store, &dir).unwrap();
    let mut list = mac.store.load().unwrap();
    list.get_mut(2).unwrap().alias = Some("Đã đổi".into());
    list.get_mut(3).unwrap().disabled = true;
    mac.store.save(&list).unwrap();
    let saved = std::fs::read(mac.paths.accounts()).unwrap();
    let again = import::import(&mac.store, &dir).unwrap();
    assert_eq!((again.added, again.updated, again.unchanged), (vec![], vec![], vec![1, 2, 3]));
    assert_eq!(std::fs::read(mac.paths.accounts()).unwrap(), saved);
}

#[test]
fn a_newer_login_in_claude_swap_replaces_the_apps_copy() {
    let mac = Mac::new("import-newer");
    let dir = claude_swap(&mac);
    import::import(&mac.store, &dir).unwrap();
    let newer = credential("acc-1b", "ref-1b", in_hours(9.0));
    write_enc(&dir, 1, PERSONAL, &newer);
    let older = credential("acc-2-old", "ref-2-old", in_hours(1.0));
    mac.put_item("claude-swap", &format!("account-2-{TEAM}"), &older);
    let done = import::import(&mac.store, &dir).unwrap();
    assert_eq!(done.updated, vec![1]);
    assert_eq!(mac.store.credential(1).unwrap(), Some(newer));
    assert_ne!(mac.store.credential(2).unwrap(), Some(older));
}

#[test]
fn a_slot_the_app_uses_for_another_account_is_not_taken() {
    let mac = Mac::new("import-taken");
    mac.add(2, "someone@else.example", "org-x", &credential("acc-x", "ref-x", in_hours(5.0)));
    let dir = claude_swap(&mac);
    let done = import::import(&mac.store, &dir).unwrap();
    assert_eq!(done.added, vec![1, 3]);
    assert!(done.skipped.contains(&(2, "slotTaken".to_string())));
    assert_eq!(
        mac.store.oauth_account(2).unwrap().unwrap()["emailAddress"],
        "someone@else.example"
    );
}

#[test]
fn an_api_key_account_is_left_in_claude_swap() {
    let mac = Mac::new("import-key");
    let dir = claude_swap(&mac);
    let mut sequence: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("sequence.json")).unwrap()).unwrap();
    sequence["accounts"]["4"] = json!({ "email": "api@example.com", "kind": "api_key" });
    std::fs::write(dir.join("sequence.json"), sequence.to_string()).unwrap();
    let done = import::import(&mac.store, &dir).unwrap();
    assert!(done.skipped.contains(&(4, "apiKey".to_string())));
    assert!(mac.store.load().unwrap().get(4).is_none());
}

#[test]
fn the_rotation_file_and_priming_come_over_once() {
    let mac = Mac::new("import-rotation");
    let dir = claude_swap(&mac);
    let config = mac.home.join(".config/cswap-prefer-team");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("rotation"), "team = 2 3\npersonal = 1\nfull = 85\n").unwrap();
    std::fs::write(config.join("prime"), "").unwrap();
    let state = mac.home.join(".local/state/cswap-prefer-team");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("prime-3"), "1894778400\n").unwrap();
    let done = import::import(&mac.store, &dir).unwrap();
    assert!(done.rotation && done.prime);
    assert_eq!(
        std::fs::read_to_string(mac.paths.rotation()).unwrap(),
        "team = 2 3\npersonal = 1\nfull = 85\n"
    );
    assert!(mac.paths.prime_flag().exists());
    assert_eq!(
        std::fs::read_to_string(mac.paths.prime_state().join("prime-3")).unwrap(),
        "1894778400\n"
    );
    // The app's own file is the owner's from now on.
    std::fs::write(mac.paths.rotation(), "team = 3\n").unwrap();
    let again = import::import(&mac.store, &dir).unwrap();
    assert!(!again.rotation && !again.prime);
    assert_eq!(std::fs::read_to_string(mac.paths.rotation()).unwrap(), "team = 3\n");
}

#[test]
fn claude_swaps_last_readings_seed_the_apps_only_for_the_same_account() {
    let mac = Mac::new("import-usage");
    let dir = claude_swap(&mac);
    let cache = json!({
        "schemaVersion": 2,
        "accounts": {
            "1": { "email": PERSONAL, "fetchedAt": 1894778000.0,
                   "lastGood": { "five_hour": { "pct": 42.0, "resets_at": "2030-01-16T09:00:00Z" } } },
            "2": { "email": "someone@else.example", "fetchedAt": 1894778000.0,
                   "lastGood": { "five_hour": { "pct": 77.0 } } }
        }
    });
    std::fs::write(dir.join("cache/usage.json"), cache.to_string()).unwrap();
    import::import(&mac.store, &dir).unwrap();
    let ours = usage::load(&mac.store);
    let one = ours.row(1).unwrap().last_good.as_ref().unwrap();
    assert_eq!(one.five_hour.as_ref().unwrap().pct, 42.0);
    assert!(ours.row(2).is_none_or(|r| r.last_good.is_none()));
}

#[test]
fn a_mac_without_claude_swap_has_nothing_to_import() {
    let mac = Mac::new("import-none");
    assert!(!import::available(&mac.home));
    assert!(matches!(
        import::import(&mac.store, &import::source(&mac.home)),
        Err(import::ImportError::NotFound)
    ));
}
