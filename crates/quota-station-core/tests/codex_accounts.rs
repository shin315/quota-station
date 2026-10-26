// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Several Codex accounts, on a temporary Codex folder and a temporary app
//! folder: every sign-in here is a made-up `auth.json` whose tokens are not
//! real, and no test reads or writes the real `~/.codex`.
//!
//! What these prove: a sign-in lives in exactly one place (the Codex folder
//! or its account's own folder), a switch moves files and copies none, a
//! switch cut short is put back, a sign-in Codex got on its own joins the
//! list instead of being lost, the custom providers are tables of
//! `config.toml` added and removed without touching another line.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine;
use common::{fixture_text, temp_dir};
use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::accounts::{self, Added, Moved, Place, Refusal, Store};
use quota_station_core::codex::config;
use quota_station_core::codex::platform::{Platform, Unix};
use quota_station_core::codex::{self, files, ProviderOutcome};
use time::OffsetDateTime;

const T0: i64 = 1_768_548_000;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(T0 + offset).unwrap()
}

/// The real moves and modes, and a record of what was kept out of backups.
#[derive(Default)]
struct Spy {
    excluded: Mutex<Vec<PathBuf>>,
}

impl Platform for Spy {
    fn private_dir(&self, dir: &Path) -> std::io::Result<()> {
        Unix::default().private_dir(dir)
    }
    fn private_file(&self, file: &Path) -> std::io::Result<()> {
        Unix::default().private_file(file)
    }
    fn move_file(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        Unix::default().move_file(from, to)
    }
    fn exclude_from_backup(&self, dir: &Path) {
        self.excluded.lock().unwrap().push(dir.to_path_buf());
    }
}

/// A made-up ChatGPT `auth.json` for `user` in workspace `ws`: its
/// `id_token` carries the claims Codex's has, its tokens are not real.
fn sign_in(user: &str, ws: &str, plan: &str) -> String {
    let b64 = |v: serde_json::Value| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v.to_string().as_bytes())
    };
    let id_token = format!(
        "{}.{}.not-a-signature",
        b64(serde_json::json!({"alg": "none"})),
        b64(serde_json::json!({
            "email": format!("{user}@example.com"),
            "sub": format!("auth0|{user}"),
            "https://api.openai.com/auth": {
                "chatgpt_account_id": ws,
                "chatgpt_user_id": format!("user-{user}"),
                "chatgpt_plan_type": plan,
            }
        }))
    );
    serde_json::json!({
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": id_token,
            "access_token": format!("test-access-{user}-not-real"),
            "refresh_token": format!("test-refresh-{user}-not-real"),
            "account_id": ws,
        },
        "last_refresh": "2026-10-20T07:00:00Z"
    })
    .to_string()
}

struct Mac {
    home: PathBuf,
    app: AppDir,
    spy: Spy,
}

impl Mac {
    /// A Codex folder with `config` (a fixture) and, when given, a sign-in.
    fn new(config: &str, signed_in: Option<&str>) -> Mac {
        let root = temp_dir("codex-accounts");
        let home = root.join("codex-home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(codex::CONFIG), fixture_text(&format!("codex/{config}"))).unwrap();
        if let Some(text) = signed_in {
            std::fs::write(home.join(codex::AUTH), text).unwrap();
        }
        Mac { home, app: AppDir::new(root.join("app")), spy: Spy::default() }
    }

    fn store(&self) -> Store<'_> {
        Store::new(&self.app, &self.home, &self.spy)
    }

    fn home_auth(&self) -> Option<String> {
        std::fs::read_to_string(self.home.join(codex::AUTH)).ok()
    }

    fn folder_auth(&self, n: u32) -> Option<String> {
        std::fs::read_to_string(self.store().folder(n).join(codex::AUTH)).ok()
    }

    /// What `codex login` would leave in a folder it was given.
    fn codex_login(&self, user: &str, ws: &str) -> PathBuf {
        let pending = self.store().begin_login().unwrap();
        std::fs::write(pending.join(codex::AUTH), sign_in(user, ws, "plus")).unwrap();
        pending
    }

    /// Every `auth.json` under the app's folder and the Codex folder, by text.
    fn every_sign_in(&self) -> Vec<String> {
        fn walk(dir: &Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.file_name().is_some_and(|n| n == codex::AUTH) {
                    out.push(std::fs::read_to_string(&p).unwrap());
                }
            }
        }
        let mut out = Vec::new();
        walk(self.app.path(), &mut out);
        walk(&self.home, &mut out);
        out
    }
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn the_sign_in_codex_has_joins_the_list_where_it_is() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    let list = mac.store().list(at(0)).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].place, Place::Home);
    assert_eq!(list[0].entry.number, 1);
    assert_eq!(list[0].entry.email.as_deref(), Some("ann@example.com"));
    assert_eq!(list[0].entry.plan.as_deref(), Some("plus"));
    assert_eq!(list[0].entry.added_at, None, "it was Codex's already");
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()), "left where it is, as it is");
    assert_eq!(mac.every_sign_in().len(), 1, "no copy made");
    // Asked again, it is the same account, not a second one.
    assert_eq!(mac.store().list(at(60)).unwrap().len(), 1);
}

#[test]
fn a_new_sign_in_gets_its_own_private_folder_and_codex_is_not_touched() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    let added = mac.store().finish_login(&pending, None, at(10)).unwrap();
    assert_eq!(added, Added::Added { number: 2 });
    assert!(!pending.exists(), "the sign-in's temporary folder is gone");
    let folder = mac.store().folder(2);
    assert!(mac.folder_auth(2).unwrap().contains("bob@example.com") || mac.folder_auth(2).is_some());
    assert_eq!(mode(&folder), 0o700);
    assert_eq!(mode(&folder.join(codex::AUTH)), 0o600);
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()), "Codex keeps its sign-in");
    let list = mac.store().list(at(20)).unwrap();
    let bob = list.iter().find(|a| a.entry.number == 2).unwrap();
    assert_eq!(bob.place, Place::Folder);
    assert_eq!(bob.entry.added_at, Some(T0 + 10));
    assert!(
        mac.spy.excluded.lock().unwrap().iter().any(|d| d.ends_with(accounts::DIR)),
        "the account folders are kept out of backups"
    );
}

#[test]
fn signing_in_to_an_account_already_listed_adds_nothing_and_keeps_its_sign_in() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("ann", "ws-ann");
    assert_eq!(mac.store().finish_login(&pending, None, at(10)).unwrap(), Added::Dup { number: 1 });
    assert!(!pending.exists());
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
    assert_eq!(mac.store().list(at(20)).unwrap().len(), 1);
    // The same person in another workspace is another account.
    let team = mac.codex_login("ann", "ws-team");
    assert_eq!(mac.store().finish_login(&team, None, at(30)).unwrap(), Added::Added { number: 2 });
}

#[test]
fn a_folder_with_no_chatgpt_sign_in_adds_nothing() {
    let mac = Mac::new("config-chatgpt.toml", None);
    let pending = mac.store().begin_login().unwrap();
    std::fs::write(pending.join(codex::AUTH), fixture_text("codex/auth-apikey.json")).unwrap();
    assert_eq!(mac.store().finish_login(&pending, None, at(0)).unwrap(), Added::NoLogin);
    assert!(!pending.exists());
    assert!(mac.store().list(at(1)).unwrap().is_empty());
}

#[test]
fn switching_moves_the_two_sign_ins_and_copies_none() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let bob = sign_in("bob", "ws-bob", "pro");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    std::fs::write(pending.join(codex::AUTH), &bob).unwrap();
    mac.store().finish_login(&pending, None, at(10)).unwrap();

    let moved = mac.store().switch(2, true, at(20)).unwrap();
    assert_eq!(moved, Moved::Moved { out: Some(1), into: 2 });
    assert_eq!(mac.home_auth().as_deref(), Some(bob.as_str()), "Codex now has bob's");
    assert_eq!(mac.folder_auth(1).as_deref(), Some(ann.as_str()), "ann's went to her folder");
    assert_eq!(mac.folder_auth(2), None, "bob's left his folder");
    let all = mac.every_sign_in();
    assert_eq!(all.len(), 2);
    assert_eq!(all.iter().filter(|t| **t == ann).count(), 1);
    assert_eq!(all.iter().filter(|t| **t == bob).count(), 1);
    assert_eq!(mode(&mac.store().folder(1).join(codex::AUTH)), 0o600);
    assert_eq!(mac.store().registry().moving, None, "nothing left under way");
    let list = mac.store().list(at(30)).unwrap();
    let place = |n| list.iter().find(|a| a.entry.number == n).unwrap().place;
    assert_eq!((place(1), place(2)), (Place::Folder, Place::Home));

    // And back.
    assert_eq!(mac.store().switch(1, true, at(40)).unwrap(), Moved::Moved { out: Some(2), into: 1 });
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
    assert_eq!(mac.folder_auth(2).as_deref(), Some(bob.as_str()));
}

#[test]
fn switching_to_the_account_codex_has_moves_nothing() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    assert_eq!(mac.store().switch(1, true, at(1)).unwrap(), Moved::Already);
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
}

#[test]
fn a_switch_is_refused_without_moving_anything_when_it_cannot_finish() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    mac.store().finish_login(&pending, None, at(10)).unwrap();

    assert_eq!(mac.store().switch(9, true, at(20)), Err(Refusal::Unknown));
    assert_eq!(mac.store().switch(2, false, at(20)), Err(Refusal::Keychain));
    // Something already sits where ann's sign-in would go.
    std::fs::write(mac.store().folder(1).join(codex::AUTH), "{}").unwrap();
    assert_eq!(mac.store().switch(2, true, at(20)), Err(Refusal::Conflict));
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
    assert!(mac.folder_auth(2).is_some());
    std::fs::remove_file(mac.store().folder(1).join(codex::AUTH)).unwrap();
    // bob's sign-in went missing: he signs in again first.
    std::fs::remove_file(mac.store().folder(2).join(codex::AUTH)).unwrap();
    assert_eq!(mac.store().switch(2, true, at(30)), Err(Refusal::NoLogin));
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
}

#[test]
fn a_switch_cut_short_is_put_back_when_the_app_next_looks() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let bob = sign_in("bob", "ws-bob", "pro");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    std::fs::write(pending.join(codex::AUTH), &bob).unwrap();
    mac.store().finish_login(&pending, None, at(10)).unwrap();

    // The app stopped after moving ann's out and before moving bob's in.
    let mut registry = mac.store().registry();
    registry.moving = Some(accounts::Moving { out: Some(1), into: 2 });
    let text = serde_json::to_string(&registry).unwrap();
    mac.app.write_private(&mac.app.path().join(accounts::REGISTRY), &text).unwrap();
    std::fs::rename(mac.home.join(codex::AUTH), mac.store().folder(1).join(codex::AUTH)).unwrap();
    assert_eq!(mac.home_auth(), None);

    mac.store().list(at(20)).unwrap();
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()), "put back as it was");
    assert_eq!(mac.folder_auth(2).as_deref(), Some(bob.as_str()));
    assert_eq!(mac.store().registry().moving, None);
}

#[test]
fn a_sign_in_codex_got_on_its_own_is_kept_before_a_switch() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    mac.store().finish_login(&pending, None, at(10)).unwrap();
    // The owner ran `codex login` as cat in Terminal: ann's sign-in is gone.
    let cat = sign_in("cat", "ws-cat", "team");
    std::fs::write(mac.home.join(codex::AUTH), &cat).unwrap();

    assert_eq!(mac.store().switch(2, true, at(20)).unwrap(), Moved::Moved { out: Some(3), into: 2 });
    let list = mac.store().list(at(30)).unwrap();
    let find = |n: u32| list.iter().find(|a| a.entry.number == n).unwrap();
    assert_eq!(find(3).entry.email.as_deref(), Some("cat@example.com"));
    assert_eq!(find(3).place, Place::Folder);
    assert_eq!(mac.folder_auth(3).as_deref(), Some(cat.as_str()), "cat's sign-in kept, not lost");
    assert_eq!(find(1).place, Place::Missing, "ann signs in again");
    assert_eq!(find(2).place, Place::Home);
}

#[test]
fn signing_in_again_replaces_only_that_accounts_sign_in() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    mac.store().finish_login(&pending, None, at(10)).unwrap();
    let again = mac.codex_login("bob", "ws-bob");
    std::fs::write(again.join(codex::AUTH), sign_in("bob", "ws-bob", "pro")).unwrap();
    assert_eq!(
        mac.store().finish_login(&again, Some(2), at(20)).unwrap(),
        Added::Renewed { number: 2 }
    );
    assert!(mac.folder_auth(2).unwrap().contains("test-access-bob"));
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
    assert_eq!(mac.store().list(at(30)).unwrap().len(), 2);
    // Signing in again to the account Codex has replaces the Codex folder's.
    let ann_again = mac.codex_login("ann", "ws-ann");
    let fresh = sign_in("ann", "ws-ann", "pro");
    std::fs::write(ann_again.join(codex::AUTH), &fresh).unwrap();
    assert_eq!(
        mac.store().finish_login(&ann_again, Some(1), at(40)).unwrap(),
        Added::Renewed { number: 1 }
    );
    assert_eq!(mac.home_auth().as_deref(), Some(fresh.as_str()));
    assert_eq!(mac.every_sign_in().len(), 2);
}

#[test]
fn only_an_account_codex_is_not_using_can_be_removed() {
    let ann = sign_in("ann", "ws-ann", "plus");
    let mac = Mac::new("config-chatgpt.toml", Some(&ann));
    mac.store().list(at(0)).unwrap();
    let pending = mac.codex_login("bob", "ws-bob");
    mac.store().finish_login(&pending, None, at(10)).unwrap();
    assert_eq!(mac.store().removable(1), Err(Refusal::InUse));
    assert_eq!(mac.store().removable(2).unwrap(), mac.store().folder(2));
    mac.store().remove(2).unwrap();
    assert!(!mac.store().folder(2).exists());
    assert_eq!(mac.store().list(at(20)).unwrap().len(), 1);
    assert_eq!(mac.home_auth().as_deref(), Some(ann.as_str()));
}

#[test]
fn nicknames_are_the_apps_own_and_checked() {
    let mac = Mac::new("config-chatgpt.toml", Some(&sign_in("ann", "ws-ann", "plus")));
    mac.store().list(at(0)).unwrap();
    mac.store().rename("chatgpt:1", "  Main  ").unwrap();
    mac.store().rename("custom", "Relay").unwrap();
    assert_eq!(mac.store().registry().accounts[0].nickname.as_deref(), Some("Main"));
    assert_eq!(mac.store().registry().nicknames.get("custom").map(String::as_str), Some("Relay"));
    assert_eq!(mac.store().rename("chatgpt:1", "two\nlines"), Err(Refusal::Name));
    assert_eq!(mac.store().rename("chatgpt:7", "x"), Err(Refusal::Unknown));
    mac.store().rename("chatgpt:1", "").unwrap();
    assert_eq!(mac.store().registry().accounts[0].nickname, None);
    assert!(!config_text(&mac.home).contains("Relay"), "a nickname never goes to Codex");
}

// ===== Custom providers: tables of config.toml =====

fn config_text(home: &Path) -> String {
    std::fs::read_to_string(home.join(codex::CONFIG)).unwrap()
}

fn hash(home: &Path) -> String {
    files::sha256(&std::fs::read(home.join(codex::CONFIG)).unwrap())
}

#[test]
fn every_provider_table_is_listed() {
    let mut text = fixture_text("codex/config-chatgpt.toml");
    text.push_str("\n[model_providers.relay-eu]\nname = \"relay-eu\"\nbase_url = \"https://eu.relay.example.com/v1\"\nexperimental_bearer_token = \"test-key-eu-not-real\"\n");
    let parsed = config::parse(&text, None).unwrap();
    let ids: Vec<&str> = parsed.providers.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["custom", "relay-eu"]);
}

#[test]
fn adding_a_provider_appends_one_table_and_changes_no_other_line() {
    let mac = Mac::new("config-chatgpt.toml", None);
    let before = config_text(&mac.home);
    let preview =
        codex::provider_preview(&mac.home, &mac.app, "Relay 2", "https://two.example.com/v1", "sk-test-0000000000abcd");
    assert_eq!(preview.id.as_deref(), Some("relay-2"));
    let shown: String = preview.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(!shown.contains("sk-test-0000000000abcd"), "the preview shows only the key's hint");
    assert!(shown.contains("sk-…abcd"));
    let out = codex::add_provider(
        &mac.home,
        &mac.app,
        "Relay 2",
        "https://two.example.com/v1",
        "sk-test-0000000000abcd",
        preview.hash.as_deref().unwrap(),
        at(0),
    );
    assert!(matches!(out, ProviderOutcome::Added { ref id, .. } if id == "relay-2"), "{out:?}");
    let after = config_text(&mac.home);
    assert!(after.starts_with(&before), "every line before stays as it was");
    let parsed = config::parse(&after, None).unwrap();
    let added = parsed.providers.iter().find(|p| p.id == "relay-2").unwrap();
    assert_eq!(added.name, "Relay 2");
    assert_eq!(added.base_url.as_deref(), Some("https://two.example.com/v1"));
    assert_eq!(added.key, config::KeyKind::Bearer);
    assert_eq!(parsed.selected, None, "Codex is not switched by adding");
    let backups = files::backups(&mac.app.path().join(files::BACKUP_DIR));
    assert_eq!(backups.len(), 1);
    assert!(!std::fs::read_to_string(&backups[0]).unwrap().contains("test-key-not-real"));
}

#[test]
fn a_provider_name_that_cannot_be_a_table_is_refused() {
    let mac = Mac::new("config-chatgpt.toml", None);
    let base = hash(&mac.home);
    let add = |name: &str, url: &str, key: &str| codex::add_provider(&mac.home, &mac.app, name, url, key, &base, at(0));
    for name in ["", "openai", "ollama", "   "] {
        assert!(matches!(add(name, "https://x.example.com/v1", "sk-test-0000000000abcd"), ProviderOutcome::Refused { .. }), "{name}");
    }
    assert!(matches!(add("x", "ftp://x.example.com", "sk-test-0000000000abcd"), ProviderOutcome::Refused { .. }));
    assert!(matches!(add("x", "https://x.example.com/v1", ""), ProviderOutcome::Refused { .. }));
    // A name already taken gets the next free id.
    let out = add("custom", "https://x.example.com/v1", "sk-test-0000000000abcd");
    assert!(matches!(out, ProviderOutcome::Added { ref id, .. } if id == "custom-2"), "{out:?}");
    assert_eq!(config_text(&mac.home).matches("[model_providers.").count(), 2);
}

#[test]
fn switching_goes_to_the_provider_asked_for_among_several() {
    let mac = Mac::new("config-chatgpt.toml", Some(&sign_in("ann", "ws-ann", "plus")));
    let base = hash(&mac.home);
    codex::add_provider(&mac.home, &mac.app, "relay-eu", "https://eu.example.com/v1", "sk-test-0000000000abcd", &base, at(0));
    let out = codex::switch_provider(&mac.home, &mac.app, "relay-eu", Some(&hash(&mac.home)), true, at(10));
    assert!(matches!(out, codex::Outcome::Switched { ref to, .. } if to == "relay-eu"), "{out:?}");
    let parsed = config::parse(&config_text(&mac.home), None).unwrap();
    assert_eq!(parsed.selected.as_deref(), Some("relay-eu"));
    let out = codex::switch_provider(&mac.home, &mac.app, "custom", Some(&hash(&mac.home)), true, at(20));
    assert!(matches!(out, codex::Outcome::Switched { ref from, ref to, .. } if from == "relay-eu" && to == "custom"), "{out:?}");
    let out = codex::switch_provider(&mac.home, &mac.app, "nowhere", Some(&hash(&mac.home)), true, at(30));
    assert!(matches!(out, codex::Outcome::Refused { .. }), "{out:?}");
}

#[test]
fn removing_a_provider_takes_out_its_table_only_and_never_the_one_in_use() {
    let mac = Mac::new("config-chatgpt.toml", None);
    let base = hash(&mac.home);
    let before = config_text(&mac.home);
    codex::add_provider(&mac.home, &mac.app, "relay-eu", "https://eu.example.com/v1", "sk-test-0000000000abcd", &base, at(0));
    let out = codex::remove_provider(&mac.home, &mac.app, "relay-eu", &hash(&mac.home), at(10));
    assert!(matches!(out, ProviderOutcome::Removed { .. }), "{out:?}");
    assert_eq!(config_text(&mac.home), before, "back to the bytes before the add");

    codex::switch_provider(&mac.home, &mac.app, "custom", Some(&hash(&mac.home)), true, at(20));
    let out = codex::remove_provider(&mac.home, &mac.app, "custom", &hash(&mac.home), at(30));
    assert!(matches!(out, ProviderOutcome::Refused { .. }), "{out:?}");
    assert!(config_text(&mac.home).contains("[model_providers.custom]"));
}
