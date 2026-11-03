// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! `Nhập từ claude-swap` with the advanced part (the core's `claude_import`
//! tests the app's own), against a made-up claude-swap folder and a fake
//! Keychain in a temporary home: every account comes in the rotation, the
//! rotation file and the priming come over once, and running it again
//! keeps an account the owner held out.

mod common;

use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use quota_station_core::claude::import;
use quota_station_core::ext;
use serde_json::{json, Value};

use common::accounts::{credential, in_hours, oauth};
use common::advanced::Mac;

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

#[test]
fn every_account_comes_in_the_rotation_whatever_claude_swap_held_out() {
    let mac = Mac::new("import-in-rotation");
    let dir = claude_swap(&mac);
    let file = dir.join("sequence.json");
    let mut sequence: Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    sequence["accounts"]["2"]["disabled"] = Value::Bool(true);
    std::fs::write(&file, sequence.to_string()).unwrap();
    import::import(&mac.store, &dir, ext::none()).unwrap();
    let list = mac.store.load().unwrap();
    assert!(list.accounts.iter().all(|a| !quota_station_core::advanced::held::disabled(a)));
    assert_eq!(list.accounts.len(), 3);
}

#[test]
fn importing_again_changes_nothing_the_owner_set() {
    let mac = Mac::new("import-again");
    let dir = claude_swap(&mac);
    import::import(&mac.store, &dir, ext::none()).unwrap();
    let mut list = mac.store.load().unwrap();
    list.get_mut(2).unwrap().alias = Some("Đã đổi".into());
    quota_station_core::advanced::held::set(list.get_mut(3).unwrap(), true);
    mac.store.save(&list).unwrap();
    let saved = std::fs::read(mac.paths.accounts()).unwrap();
    let again = import::import(&mac.store, &dir, ext::none()).unwrap();
    assert_eq!((again.added, again.updated, again.unchanged), (vec![], vec![], vec![1, 2, 3]));
    assert_eq!(std::fs::read(mac.paths.accounts()).unwrap(), saved);
}

#[test]
fn the_rotation_file_and_priming_come_over_once() {
    use quota_station_core::advanced::import::ImportHooks;
    let mac = Mac::new("import-rotation");
    let dir = claude_swap(&mac);
    let config = mac.home.join(".config/cswap-prefer-team");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("rotation"), "team = 2 3\npersonal = 1\nfull = 85\n").unwrap();
    std::fs::write(config.join("prime"), "").unwrap();
    let state = mac.home.join(".local/state/cswap-prefer-team");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("prime-3"), "1894778400\n").unwrap();
    let done = import::import(&mac.store, &dir, &ImportHooks).unwrap();
    assert_eq!(done.ext.get("rotation"), Some(&Value::Bool(true)));
    assert_eq!(done.ext.get("prime"), Some(&Value::Bool(true)));
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
    let again = import::import(&mac.store, &dir, &ImportHooks).unwrap();
    assert_eq!(again.ext.get("rotation"), Some(&Value::Bool(false)));
    assert_eq!(again.ext.get("prime"), Some(&Value::Bool(false)));
    // The outcome ends with both, as its own fields did.
    let outcome = serde_json::to_string(&again).unwrap();
    assert!(outcome.ends_with(r#""skipped":[],"rotation":false,"prime":false}"#), "{outcome}");
    assert_eq!(std::fs::read_to_string(mac.paths.rotation()).unwrap(), "team = 3\n");
}
