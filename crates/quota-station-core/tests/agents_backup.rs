// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Backup packs and restores (M10) between two temporary homes: the Mac
//! the pack comes from, with servers in Codex's `config.toml`, Antigravity's
//! `mcp_config.json` and a fake `claude`'s list, and a Mac set up again
//! under another user name. The packs go to a temporary folder standing in
//! for iCloud Drive; a fake `security` stands in for the Keychain. Every
//! name, URL and key is made up; no test reads or writes the real agents'
//! files, the Keychain, iCloud Drive or the real `claude`.
//!
//! The secrets are traps: each may be in the agent files and the fake
//! Keychain, and nowhere else: not in a pack, the app's folder (store,
//! backups, log, the restore record), the plan or the result.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::claude::fake_claude;
use common::keychain::fake_security;
use common::{script, temp_dir};
use quota_station_core::agents::batch::{self, Change, To};
use quota_station_core::agents::def::Def;
use quota_station_core::agents::pack::{self, Pack};
use quota_station_core::agents::restore::{
    self, More, Pasted, Pick, Plan, RunError, Tools, UndoError,
};
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::skills::SkillPlaces;
use quota_station_core::agents::{self, kho, Agent, Places};
use quota_station_core::app_dir::AppDir;
use time::OffsetDateTime;

const TRAPS: &[&str] =
    &["trap-files-token", "trap-docs-key", "trap-provider-key", "trap-tracker-key", "trap-pasted"];
const SENTINEL: &str = "trap-claude-account-sentinel";

/// The Mac the pack comes from. `@HOME@` is its home folder.
const CODEX: &str = r#"# Codex config for the backup tests. Every name, URL and key is made up.
model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key"

[mcp_servers.files]
command = "npx"
args = ["-y", "files-mcp", "--root=@HOME@/work"]

[mcp_servers.files.env]
FILES_TOKEN = "trap-files-token"

[mcp_servers.docs]
url = "https://docs.example.com/mcp"
enabled = false

[mcp_servers.notes]
command = "@HOME@/.local/bin/notes-mcp"

[mcp_servers.gone]
command = "/opt/nowhere/bin/gone-mcp"

[mcp_servers.node_repl]
command = "/Applications/Codex.app/Contents/Resources/node_repl"
"#;

const AGY: &str = r#"{
  "mcpServers": {
    "docs": {
      "serverUrl": "https://docs.example.com/mcp",
      "headers": {
        "DOCS_API_KEY": "trap-docs-key"
      }
    }
  }
}
"#;

const TRACKER: &str = r#"{"type":"sse","url":"https://tracker.example.com/sse","headers":{"Authorization":"Bearer trap-tracker-key"}}"#;
const BROWSER: &str = r#"{"type":"stdio","command":"npx","args":["-y","browser-mcp"],"env":{}}"#;

/// The Mac set up again: Codex with its provider and no server.
const FRESH_CODEX: &str = r#"model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key"
"#;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000 + offset).unwrap()
}

struct Mac {
    home: PathBuf,
    app: AppDir,
    places: Places,
    secrets: Secrets,
    claude: PathBuf,
    keychain: PathBuf,
}

fn mac(tag: &str, codex: &str, agy: Option<&str>, claude: &[(&str, &str)]) -> Mac {
    let root = temp_dir(tag);
    let home = root.join("Users").join(tag);
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
    std::fs::write(
        home.join(".codex/config.toml"),
        codex.replace("@HOME@", &home.to_string_lossy()),
    )
    .unwrap();
    if let Some(agy) = agy {
        std::fs::write(home.join(".gemini/config/mcp_config.json"), agy).unwrap();
    }
    let dir = root.join("claude");
    let cli = fake_claude(&dir, &home, SENTINEL);
    for (name, json) in claude {
        std::fs::write(dir.join("servers").join(name), json).unwrap();
    }
    std::process::Command::new(&cli).args(["mcp", "list"]).status().unwrap();
    std::fs::remove_file(dir.join("calls")).unwrap();
    let keychain = root.join("keychain");
    let secrets = Secrets::fake(fake_security(&keychain));
    let places = Places::new(&home, &home.join(".codex"), Some(cli));
    Mac { home, app: AppDir::new(root.join("app")), places, secrets, claude: dir, keychain }
}

fn old_mac() -> Mac {
    mac("old", CODEX, Some(AGY), &[("tracker", TRACKER), ("browser", BROWSER)])
}

fn new_mac() -> Mac {
    let m = mac("new", FRESH_CODEX, None, &[]);
    script(&m.home.join(".local/bin/notes-mcp"), "exit 0\n");
    m
}

fn files_under(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(files_under(&p));
        } else {
            out.push((p.clone(), String::from_utf8_lossy(&std::fs::read(&p).unwrap()).to_string()));
        }
    }
    out
}

fn no_trap(what: &str, text: &str) {
    for t in TRAPS.iter().chain([&SENTINEL]) {
        assert!(!text.contains(t), "{t} in {what}:\n{text}");
    }
}

fn holds_no_secret(dir: &Path) {
    for (path, text) in files_under(dir) {
        no_trap(&path.display().to_string(), &text);
    }
}

/// Packs `m`'s servers into `root`.
fn back_up(m: &Mac, root: &Path, when: OffsetDateTime) -> (String, Pack) {
    let readings = agents::read_all(&m.places);
    let store = kho::load(&m.app);
    let p = pack::content(&m.home, &readings, &store, "Studio mini", "2.0.0", when);
    let id = pack::write(root, &m.home, &p, &readings).unwrap();
    (id, p)
}

/// The app's MCP items in the fake Keychain.
fn keychain_items(m: &Mac) -> Vec<String> {
    std::fs::read_dir(m.keychain.join("items"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("com.shin315.quota-station.mcp-secret--"))
        .collect()
}

fn codex_text(m: &Mac) -> String {
    std::fs::read_to_string(m.home.join(".codex/config.toml")).unwrap()
}

/// What a restore uses besides the MCP files: no `npx`, no plugin commands.
fn tools(m: &Mac, root: &Path) -> Tools {
    Tools {
        skills: SkillPlaces::new(&m.home, &m.home.join(".codex"), None),
        npx: None,
        claude: None,
        codex: None,
        codex_home: m.home.join(".codex"),
        root: root.to_path_buf(),
    }
}

fn picks(plan: &Plan) -> Vec<Pick> {
    plan.items
        .iter()
        .filter(|i| i.checked)
        .map(|i| Pick { name: i.name.clone(), kind: i.kind.to_string() })
        .collect()
}

#[test]
fn a_pack_holds_every_server_and_no_secret() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    assert!(id.starts_with("agents-") && id.ends_with("-studio-mini"), "{id}");
    holds_no_secret(&root);
    no_trap("the pack", &format!("{p:?}"));

    let names: Vec<&str> = p.mcp.servers.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["browser", "docs", "files", "gone", "notes", "tracker"],
        "an app's own is left out"
    );
    assert_eq!(p.manifest.mcp, 6);
    assert_eq!(p.manifest.sections, ["mcp"]);
    let docs = p.mcp.servers.iter().find(|s| s.name == "docs").unwrap();
    let on: Vec<(Agent, bool)> = docs.entries.iter().map(|e| (e.agent, e.enabled)).collect();
    assert_eq!(on, [(Agent::Codex, false), (Agent::Agy, true)]);
    let needed: Vec<(&str, &str)> =
        p.secrets.secrets.iter().map(|n| (n.mcp.as_str(), n.slot.as_str())).collect();
    assert_eq!(
        needed,
        [
            ("docs", "header:DOCS_API_KEY"),
            ("files", "env:FILES_TOKEN"),
            ("tracker", "header:Authorization")
        ]
    );

    // The home folder goes as `~`: the same pack reads on another Mac.
    let text = std::fs::read_to_string(root.join(&id).join("mcp.json")).unwrap();
    assert!(text.contains("--root=~/work") && text.contains("~/.local/bin/notes-mcp"), "{text}");
    assert!(!text.contains(&*old.home.to_string_lossy()), "{text}");

    let listed = pack::list(&root, "Studio mini");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].state, listed[0].mine, listed[0].mcp, listed[0].keys),
        ("ok", true, 6, 3)
    );
    assert_eq!(pack::load(&root, &id).unwrap(), p);
    assert!(!old.keychain.join("calls").exists(), "a pack reads no Keychain item");
}

#[test]
fn a_pack_that_would_hold_a_secret_is_not_written() {
    let old = old_mac();
    let root = temp_dir("packs");
    let readings = agents::read_all(&old.places);
    let mut p =
        pack::content(&old.home, &readings, &kho::load(&old.app), "Studio mini", "2.0.0", at(0));
    let Def::Stdio { env, .. } = &mut p.mcp.servers[2].entries[0].def else { panic!() };
    env[0].value = "trap-files-token".into();
    assert_eq!(pack::write(&root, &old.home, &p, &readings), Err(pack::PackError::Secret));
    assert!(pack::list(&root, "Studio mini").is_empty());
    holds_no_secret(&root);
}

#[test]
fn restore_asks_for_keys_adds_without_one_turned_off_and_follows_the_mac() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    let new = new_mac();

    let plan = restore::plan(&new.places, &new.app, &new.secrets, &tools(&new, &root), &id, &p);
    no_trap("the plan", &serde_json::to_string(&plan).unwrap());
    let item = |name: &str| plan.items.iter().find(|i| i.name == name).unwrap();
    assert_eq!(item("files").agents, [Agent::Codex]);
    assert_eq!(item("docs").agents, [Agent::Codex, Agent::Agy]);
    assert_eq!(item("docs").off, [Agent::Codex], "off in the pack, off here");
    assert_eq!(item("tracker").agents, [Agent::Claude], "Codex does not speak SSE");
    assert_eq!(item("gone").missing_command.as_deref(), Some("gone-mcp"));
    assert!(item("notes").missing_command.is_none(), "the command is in this home folder");
    let keys: Vec<(&str, &str)> =
        plan.keys.iter().map(|k| (k.mcp.as_str(), k.slot.as_str())).collect();
    assert_eq!(
        keys,
        [
            ("docs", "header:DOCS_API_KEY"),
            ("files", "env:FILES_TOKEN"),
            ("tracker", "header:Authorization")
        ]
    );
    assert_eq!(codex_text(&new), FRESH_CODEX, "the plan writes nothing");

    // One key pasted; the other two left empty.
    let pasted = [Pasted {
        mcp: "files".into(),
        slot: "env:FILES_TOKEN".into(),
        value: "trap-pasted".into(),
    }];
    let done = restore::run(
        &new.places,
        &new.app,
        &new.secrets,
        &tools(&new, &root),
        &id,
        &p,
        &picks(&plan),
        &More::default(),
        &pasted,
        &plan.bases,
        at(60),
    )
    .unwrap();
    assert!(done.failed.is_none(), "{:?}", done.failed);
    assert!(done.can_undo);
    no_trap("the result", &serde_json::to_string(&done).unwrap());
    holds_no_secret(new.app.path());

    // Codex: the servers added, the provider as it was.
    let codex = codex_text(&new);
    assert!(codex.starts_with(FRESH_CODEX), "{codex}");
    let servers = agents::codex_mcp::read(&codex).unwrap();
    let get = |n: &str| servers.iter().find(|s| s.name == n);
    let Some(Def::Stdio { args, env, .. }) = &get("files").unwrap().def else { panic!() };
    assert_eq!(args[2], format!("--root={}/work", new.home.display()), "paths follow the Mac");
    assert_eq!(env[0].value, "trap-pasted", "the pasted key, from the Keychain");
    assert!(!get("docs").unwrap().enabled, "off as it was in the pack");
    let Some(Def::Stdio { command, .. }) = &get("notes").unwrap().def else { panic!() };
    assert_eq!(command, &format!("{}/.local/bin/notes-mcp", new.home.display()));
    assert!(get("gone").is_none(), "no command here: kept off by the app");
    // Antigravity's docs and Claude Code's tracker wait for their keys.
    assert!(!new.home.join(".gemini/config/mcp_config.json").exists());
    let store = kho::load(&new.app);
    assert_eq!(store.waiting["docs"].agents, [Agent::Agy]);
    assert_eq!(store.waiting["gone"].agents, [Agent::Codex]);
    assert!(store.parked.contains_key("tracker"));
    let claude: Vec<String> = std::fs::read_dir(new.claude.join("servers"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(claude, ["browser"], "through claude; tracker waits for its key");
    assert_eq!(store.hint("files", "env:FILES_TOKEN"), None, "no hint was given");

    // The table shows them off, locked until the key is pasted.
    let view = agents::look(&new.places, &new.app, &new.secrets);
    no_trap("the table", &serde_json::to_string(&view).unwrap());
    let cell = |name: &str, a: Agent| {
        let c = &view.rows.iter().find(|r| r.name == name).unwrap().cells[&a];
        (c.state, c.parked, c.lock)
    };
    assert_eq!(cell("docs", Agent::Agy), ("off", true, Some("missingKey")));
    assert_eq!(cell("tracker", Agent::Claude), ("off", true, Some("missingKey")));
    assert_eq!(cell("gone", Agent::Codex), ("off", true, Some("noCommand")));
    let docs = view.rows.iter().find(|r| r.name == "docs").unwrap();
    assert!(docs.secrets.iter().any(|s| s.id == "header:DOCS_API_KEY" && !s.have));

    // The key pasted later in the MCP tab; the box ticked.
    new.secrets.save("docs", "header:DOCS_API_KEY", "trap-docs-key").unwrap();
    let view = agents::look(&new.places, &new.app, &new.secrets);
    let c = &view.rows.iter().find(|r| r.name == "docs").unwrap().cells[&Agent::Agy];
    assert_eq!(c.lock, None);
    let on = [Change { name: "docs".into(), agent: Agent::Agy, to: To::On }];
    let prev = batch::preview(&new.places, &new.app, &new.secrets, &on);
    assert!(prev.refused.is_empty(), "{:?}", prev.refused);
    let out =
        batch::write(&new.places, &new.app, &new.secrets, &prev.changes, &prev.bases, at(120));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    let agy = std::fs::read_to_string(new.home.join(".gemini/config/mcp_config.json")).unwrap();
    assert!(agy.contains("trap-docs-key"), "{agy}");
    assert!(!kho::load(&new.app).waiting.contains_key("docs"), "no longer waiting");
    holds_no_secret(new.app.path());
}

#[test]
fn the_same_server_is_left_alone_and_a_different_one_is_not_ticked() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    // The same Mac, after Codex's files got a different root.
    let mine = codex_text(&old).replace("--root=", "--root=/elsewhere");
    std::fs::write(old.home.join(".codex/config.toml"), &mine).unwrap();
    let plan = restore::plan(&old.places, &old.app, &old.secrets, &tools(&old, &root), &id, &p);
    let files: Vec<_> = plan.items.iter().filter(|i| i.name == "files").collect();
    assert_eq!(files.len(), 1);
    assert_eq!((files[0].kind, files[0].checked), ("replace", false));
    assert_eq!(files[0].compare.len(), 1);
    assert!(plan.same.contains(&"docs".to_string()) && plan.same.contains(&"browser".to_string()));
    assert!(files[0].keys.is_empty(), "the key in Codex's file is used");
    no_trap("the plan", &serde_json::to_string(&plan).unwrap());

    // Nothing ticked: nothing changes.
    let done = restore::run(
        &old.places,
        &old.app,
        &old.secrets,
        &tools(&old, &root),
        &id,
        &p,
        &[],
        &More::default(),
        &[],
        &plan.bases,
        at(60),
    )
    .unwrap();
    assert!(!done.can_undo);
    assert_eq!(codex_text(&old), mine);

    // Ticked: the pack's in place of the Mac's, with the Mac's own key.
    let pick = [Pick { name: "files".into(), kind: "replace".into() }];
    let plan = restore::plan(&old.places, &old.app, &old.secrets, &tools(&old, &root), &id, &p);
    let done = restore::run(
        &old.places,
        &old.app,
        &old.secrets,
        &tools(&old, &root),
        &id,
        &p,
        &pick,
        &More::default(),
        &[],
        &plan.bases,
        at(90),
    )
    .unwrap();
    assert_eq!(done.items[0].done, [Agent::Codex]);
    let codex = codex_text(&old);
    assert!(codex.contains("--root=") && !codex.contains("/elsewhere"), "{codex}");
    assert!(codex.contains("trap-files-token"));
    holds_no_secret(old.app.path());
}

#[test]
fn a_file_changed_after_the_preview_stops_the_restore() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    let new = new_mac();
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &tools(&new, &root), &id, &p);
    std::fs::write(new.home.join(".codex/config.toml"), format!("{FRESH_CODEX}# edited\n"))
        .unwrap();
    let pasted = [Pasted {
        mcp: "files".into(),
        slot: "env:FILES_TOKEN".into(),
        value: "trap-pasted".into(),
    }];
    let err = restore::run(
        &new.places,
        &new.app,
        &new.secrets,
        &tools(&new, &root),
        &id,
        &p,
        &picks(&plan),
        &More::default(),
        &pasted,
        &plan.bases,
        at(60),
    )
    .unwrap_err();
    assert_eq!(err, RunError::Changed(vec![Agent::Codex]));
    assert!(kho::load(&new.app).waiting.is_empty());
    assert!(keychain_items(&new).is_empty(), "no key saved");
}

#[test]
fn the_whole_restore_is_undone_and_a_file_changed_since_is_asked_first() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    let new = new_mac();
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &tools(&new, &root), &id, &p);
    let pasted = [Pasted {
        mcp: "files".into(),
        slot: "env:FILES_TOKEN".into(),
        value: "trap-pasted".into(),
    }];
    restore::run(
        &new.places,
        &new.app,
        &new.secrets,
        &tools(&new, &root),
        &id,
        &p,
        &picks(&plan),
        &More::default(),
        &pasted,
        &plan.bases,
        at(60),
    )
    .unwrap();
    let record = restore::last(&new.app).unwrap();
    no_trap("the record", &serde_json::to_string(&record).unwrap());

    // Codex's file edited after the restore: asked first.
    let edited = format!("# edited after\n{}", codex_text(&new));
    std::fs::write(new.home.join(".codex/config.toml"), &edited).unwrap();
    match restore::undo(&new.places, &new.app, &new.secrets, &tools(&new, &root), false, at(90)) {
        Err(UndoError::Changed(c)) => assert_eq!(c.agents, [Agent::Codex]),
        other => panic!("{other:?}"),
    }
    assert_eq!(codex_text(&new), edited, "nothing written before the owner says");

    let undone =
        restore::undo(&new.places, &new.app, &new.secrets, &tools(&new, &root), true, at(120))
            .unwrap();
    assert!(undone.outcome.as_ref().is_some_and(|o| o.failed.is_none()));
    assert_eq!(
        codex_text(&new).trim_end(),
        format!("# edited after\n{FRESH_CODEX}").trim_end(),
        "only the restored entries go"
    );
    let store = kho::load(&new.app);
    assert!(store.waiting.is_empty() && store.parked.is_empty());
    let items = keychain_items(&new);
    assert!(items.is_empty(), "the Keychain items the restore made are gone: {items:?}");
    assert!(restore::last(&new.app).is_none());
    assert!(matches!(
        restore::undo(&new.places, &new.app, &new.secrets, &tools(&new, &root), false, at(130)),
        Err(UndoError::Nothing)
    ));
    holds_no_secret(new.app.path());
}

#[test]
fn a_pack_icloud_has_not_downloaded_is_listed_and_not_read() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, _) = back_up(&old, &root, at(0));
    // iCloud Drive's placeholder in place of a pack from another Mac.
    let cloud = root.join("agents-20260116-090000-travel-air");
    std::fs::create_dir_all(&cloud).unwrap();
    std::fs::write(cloud.join(".manifest.json.icloud"), b"bplist").unwrap();
    let listed = pack::list(&root, "Studio mini");
    assert_eq!(listed.len(), 2);
    let c = listed.iter().find(|i| i.id == "agents-20260116-090000-travel-air").unwrap();
    assert_eq!((c.state, c.mine, c.mac.as_deref()), ("cloud", false, None));
    assert!(c.at.is_some(), "the time comes from the name");
    assert_eq!(pack::load(&root, &c.id), Err(pack::LoadError::Cloud));
    assert!(pack::load(&root, &id).is_ok());
    assert_eq!(pack::load(&root, "../app"), Err(pack::LoadError::BadName));
}

#[test]
fn the_newest_ten_packs_of_each_mac_stay() {
    let old = old_mac();
    let root = temp_dir("packs");
    let other = root.join("agents-20250101-000000-travel-air");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("notes.txt"), "the owner's file").unwrap();
    let mut ids = Vec::new();
    for n in 0..12 {
        ids.push(back_up(&old, &root, at(n * 3600)).0);
    }
    let mine: Vec<String> =
        pack::list(&root, "Studio mini").into_iter().filter(|i| i.mine).map(|i| i.id).collect();
    assert_eq!(mine.len(), pack::KEPT);
    assert!(!mine.contains(&ids[0]) && !mine.contains(&ids[1]) && mine.contains(&ids[11]));
    assert!(
        other.join("notes.txt").exists(),
        "another Mac's folder, and anything not the app's, stays"
    );
}

#[test]
fn restore_never_touches_the_codex_provider_or_logins() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    let new = new_mac();
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &tools(&new, &root), &id, &p);
    let all: Vec<Pick> = plan
        .items
        .iter()
        .map(|i| Pick { name: i.name.clone(), kind: i.kind.to_string() })
        .collect();
    restore::run(
        &new.places,
        &new.app,
        &new.secrets,
        &tools(&new, &root),
        &id,
        &p,
        &all,
        &More::default(),
        &[],
        &plan.bases,
        at(60),
    )
    .unwrap();
    let text = codex_text(&new);
    assert!(text.starts_with(FRESH_CODEX), "{text}");
    let claude = std::fs::read_to_string(new.home.join(".claude.json")).unwrap();
    assert!(claude.contains(SENTINEL), "Claude Code's own keys stay as its claude wrote them");
    let mut seen = BTreeMap::new();
    for (path, _) in files_under(new.app.path()) {
        seen.insert(path.file_name().unwrap().to_string_lossy().to_string(), ());
    }
    assert!(!seen.keys().any(|n| n.contains("auth")), "{seen:?}");
}

#[test]
fn a_waiting_server_deleted_in_the_mcp_tab_leaves_the_files_as_they_are() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root, at(0));
    let new = new_mac();
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &tools(&new, &root), &id, &p);
    restore::run(
        &new.places,
        &new.app,
        &new.secrets,
        &tools(&new, &root),
        &id,
        &p,
        &picks(&plan),
        &More::default(),
        &[],
        &plan.bases,
        at(60),
    )
    .unwrap();
    let before = codex_text(&new);
    let backups = |m: &Mac| files_under(&m.app.path().join("backups")).len();
    let n = backups(&new);
    let del =
        [Change { name: "gone".into(), agent: Agent::Codex, to: To::Absent { store: false } }];
    let prev = batch::preview(&new.places, &new.app, &new.secrets, &del);
    assert!(prev.refused.is_empty(), "{:?}", prev.refused);
    let out = batch::write(&new.places, &new.app, &new.secrets, &prev.changes, &prev.bases, at(90));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    assert_eq!(codex_text(&new), before);
    assert_eq!(backups(&new), n, "no file written, no backup");
    assert!(!kho::load(&new.app).waiting.contains_key("gone"));
    let view = agents::look(&new.places, &new.app, &new.secrets);
    assert!(!view.rows.iter().any(|r| r.name == "gone"));
}
