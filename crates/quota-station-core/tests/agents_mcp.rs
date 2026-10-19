//! The MCP tab (M9) on a temporary home: Codex's `config.toml` and
//! Antigravity's `mcp_config.json` from the fixtures below, Claude Code's
//! `~/.claude.json` kept by a fake `claude` that records its arguments, and
//! a fake `security` for the app's Keychain items. Every name, URL and key
//! here is made up; no test reads or writes the real agents' files, the
//! Keychain or the real `claude`.
//!
//! The secrets in the fixtures are traps: each may be in the agent files
//! that hold that server, the fake `claude`'s arguments and the fake
//! Keychain, and nowhere else: not in the app's folder (backups, store,
//! log), nor in what the windows get (the table, the preview, the result).
//! The sentinel outside `mcpServers` in `~/.claude.json` is never read.

mod common;

use std::path::{Path, PathBuf};

use common::claude::fake_claude;
use common::google::fake_security;
use common::temp_dir;
use quota_station_core::agents::batch::{self, Change, Outcome, Preview, To};
use quota_station_core::agents::def::{Def, Pair, MASK};
use quota_station_core::agents::secrets::{self, Secrets};
use quota_station_core::agents::{self, kho, redact, Agent, Places};
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::codex::files as codex_files;
use time::OffsetDateTime;

const TRAPS: &[&str] = &[
    "trap-files-token",
    "trap-docs-key",
    "trap-provider-key",
    "trap-tracker-key",
    "trap-notes-token",
];
const SENTINEL: &str = "trap-claude-account-sentinel";

const CODEX: &str = r#"# Codex config for the MCP tests. Every name, URL and key is made up.
model = "gpt-5.5"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key"

[mcp_servers.files]
command = "npx"
args = ["-y", "files-mcp"]

[mcp_servers.files.env]
FILES_TOKEN = "trap-files-token"

[mcp_servers.docs]
url = "https://docs.example.com/mcp"
enabled = false

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
    },
    "browser": {
      "command": "npx",
      "args": ["-y", "browser-mcp"],
      "disabled": true
    }
  }
}
"#;

const BROWSER: &str = r#"{"type":"stdio","command":"npx","args":["-y","browser-mcp"],"env":{}}"#;
const TRACKER: &str = r#"{"type":"sse","url":"https://tracker.example.com/sse","headers":{"Authorization":"Bearer trap-tracker-key"}}"#;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000 + offset).unwrap()
}

struct Home {
    home: PathBuf,
    app: AppDir,
    places: Places,
    secrets: Secrets,
    claude: PathBuf,
    keychain: PathBuf,
}

fn setup() -> Home {
    let root = temp_dir("mcp");
    let home = root.join("home");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
    std::fs::write(home.join(".codex/config.toml"), CODEX).unwrap();
    std::fs::write(home.join(".gemini/config/mcp_config.json"), AGY).unwrap();
    let claude = root.join("claude");
    let cli = fake_claude(&claude, &home, SENTINEL);
    std::fs::write(claude.join("servers/browser"), BROWSER).unwrap();
    std::fs::write(claude.join("servers/tracker"), TRACKER).unwrap();
    // Builds `~/.claude.json`; not a call the app made.
    std::process::Command::new(&cli).args(["mcp", "list"]).status().unwrap();
    std::fs::remove_file(claude.join("calls")).unwrap();
    let keychain = root.join("keychain");
    let secrets = Secrets::fake(fake_security(&keychain));
    let places = Places::new(&home, &home.join(".codex"), Some(cli));
    Home { home, app: AppDir::new(root.join("app")), places, secrets, claude, keychain }
}

fn change(name: &str, agent: Agent, to: To) -> Change {
    Change { name: name.into(), agent, to }
}

fn preview(h: &Home, changes: &[Change]) -> Preview {
    batch::preview(&h.places, &h.app, &h.secrets, changes)
}

fn write(h: &Home, p: &Preview) -> Outcome {
    batch::write(&h.places, &h.app, &h.secrets, &p.changes, &p.bases, at(0))
}

fn codex_text(h: &Home) -> String {
    std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap()
}

fn agy_text(h: &Home) -> String {
    std::fs::read_to_string(h.home.join(".gemini/config/mcp_config.json")).unwrap()
}

fn claude_servers(h: &Home) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(h.claude.join("servers"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// What `claude` was run with, one run per line.
fn claude_calls(h: &Home) -> Vec<Vec<String>> {
    std::fs::read_to_string(h.claude.join("calls"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.split('\x1f').filter(|a| !a.is_empty()).map(str::to_string).collect())
        .collect()
}

/// Every file under `dir`, as text.
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

/// Nothing the app keeps or sends holds a secret.
fn app_holds_no_secret(h: &Home) {
    for (path, text) in files_under(h.app.path()) {
        no_trap(&path.display().to_string(), &text);
    }
    let view = agents::look(&h.places, &h.app, &h.secrets);
    no_trap("the table", &serde_json::to_string(&view).unwrap());
}

#[test]
fn the_table_reads_every_agent_and_holds_no_secret() {
    let h = setup();
    let view = agents::look(&h.places, &h.app, &h.secrets);
    assert!(view.columns.iter().all(|c| c.state == "ok" && c.writable), "{:?}", view.columns);
    let names: Vec<&str> = view.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["browser", "docs", "files", "tracker"]);
    assert_eq!(view.owned.len(), 1);
    assert_eq!((view.owned[0].name.as_str(), view.owned[0].owner.as_str()), ("node_repl", "Codex"));

    let cell = |name: &str, a: Agent| {
        let row = view.rows.iter().find(|r| r.name == name).unwrap();
        let c = &row.cells[&a];
        (c.state, c.lock)
    };
    assert_eq!(cell("browser", Agent::Claude), ("on", None));
    assert_eq!(cell("browser", Agent::Codex), ("absent", None));
    assert_eq!(cell("browser", Agent::Agy), ("off", None));
    assert_eq!(cell("docs", Agent::Codex), ("off", None));
    assert_eq!(cell("docs", Agent::Agy), ("on", None));
    assert_eq!(cell("files", Agent::Agy), ("absent", None));
    assert_eq!(cell("tracker", Agent::Codex), ("absent", Some("sse")), "Codex does not speak SSE");

    let docs = view.rows.iter().find(|r| r.name == "docs").unwrap();
    assert!(docs.differs, "Antigravity's has a header Codex's has not");
    assert_eq!(docs.secrets.len(), 1);
    assert_eq!((docs.secrets[0].id.as_str(), docs.secrets[0].have), ("header:DOCS_API_KEY", true));
    assert_eq!(docs.host.as_deref(), Some("docs.example.com"));
    let files = view.rows.iter().find(|r| r.name == "files").unwrap();
    assert_eq!(files.program.as_deref(), Some("files-mcp"));
    assert_eq!(files.secrets[0].id, "env:FILES_TOKEN");

    no_trap("the table", &serde_json::to_string(&view).unwrap());
    assert!(!h.keychain.join("calls").exists(), "secrets found in the files need no Keychain look");
}

#[test]
fn one_preview_then_one_write_per_file_and_only_the_ticked_entries_change() {
    let h = setup();
    let changes = [
        change("docs", Agent::Codex, To::On),
        change("files", Agent::Agy, To::On),
        change("browser", Agent::Claude, To::Off),
        change("docs", Agent::Claude, To::On),
    ];
    let p = preview(&h, &changes);
    assert!(p.refused.is_empty(), "{:?}", p.refused);
    assert_eq!(p.changes.len(), 4);
    no_trap("the preview", &serde_json::to_string(&p).unwrap());
    let group = |a: Agent| p.groups.iter().find(|g| g.agent == a).unwrap();
    let codex = &group(Agent::Codex).blocks;
    assert_eq!(codex.len(), 1);
    let ops: Vec<(&str, &str)> = codex[0].lines.iter().map(|l| (l.op, l.text.as_str())).collect();
    assert!(ops.contains(&("del", "enabled = false")), "{ops:?}");
    assert!(!ops.iter().any(|(op, _)| *op == "add"), "{ops:?}");
    let agy = &group(Agent::Agy).blocks[0];
    assert_eq!((agy.action, agy.from, agy.secret), ("add", Some(Agent::Codex), true));
    assert!(
        agy.lines.iter().any(|l| l.op == "add" && l.secret && l.text.contains(MASK)),
        "{:?}",
        agy.lines
    );
    let claude = &group(Agent::Claude).blocks;
    assert_eq!(claude[0].commands, ["claude mcp remove browser --scope user"]);
    assert!(claude[0].kept);
    assert!(
        claude[1].commands[0].starts_with("claude mcp add-json docs '{"),
        "{:?}",
        claude[1].commands
    );
    assert!(claude[1].commands[0].contains(MASK));
    assert!(!h.claude.join("calls").exists(), "the preview runs nothing");
    assert_eq!(codex_text(&h), CODEX, "the preview writes nothing");

    let out = write(&h, &p);
    assert!(out.failed.is_none(), "{:?}", out.failed);
    assert_eq!(out.done.len(), 3);
    no_trap("the result", &serde_json::to_string(&out).unwrap());

    // Codex: one line gone, every other byte as it was.
    assert_eq!(
        codex_text(&h),
        CODEX.replace(
            "url = \"https://docs.example.com/mcp\"\nenabled = false\n",
            "url = \"https://docs.example.com/mcp\"\n"
        )
    );
    // Antigravity: a member added after the last, its secret from Codex.
    let agy_after = agy_text(&h);
    assert!(agy_after.starts_with(AGY.trim_end_matches("\n  }\n}\n")), "{agy_after}");
    let servers = agents::agy_mcp::read(&agy_after).unwrap();
    let files = servers.iter().find(|s| s.name == "files").unwrap();
    let Some(Def::Stdio { env, .. }) = &files.def else { panic!() };
    assert_eq!(env, &[Pair::new("FILES_TOKEN", "trap-files-token")]);
    // Claude Code: through `claude`, one run per change.
    assert_eq!(claude_servers(&h), ["docs", "tracker"]);
    let calls = claude_calls(&h);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0], ["mcp", "remove", "browser", "--scope", "user"]);
    assert_eq!(&calls[1][..3], ["mcp", "add-json", "docs"]);
    assert_eq!(&calls[1][4..], ["--scope", "user"]);
    let sent: serde_json::Value = serde_json::from_str(&calls[1][3]).unwrap();
    assert_eq!(sent["type"], "http");
    assert_eq!(sent["headers"]["DOCS_API_KEY"], "trap-docs-key", "the secret goes to claude");

    // One backup per file, none with a secret.
    for a in Agent::ALL {
        let (dir, prefix) = a.backups();
        let kept = agents::write::list(&h.app.path().join(dir), prefix);
        assert_eq!(kept.len(), 1, "{a:?}: {kept:?}");
    }
    let (dir, prefix) = Agent::Codex.backups();
    let codex_backup = &agents::write::list(&h.app.path().join(dir), prefix)[0];
    assert_eq!(std::fs::read_to_string(codex_backup).unwrap(), redact::toml(CODEX).unwrap());
    let store = kho::load(&h.app);
    assert!(store.parked.contains_key("browser"), "Claude Code's off is kept by the app");
    assert_eq!(batch::log_read(&h.app).len(), 3);
    app_holds_no_secret(&h);

    // The undo puts the whole batch back.
    let undo = out.undo.expect("an undo");
    let back = batch::write(&h.places, &h.app, &h.secrets, &undo.changes, &undo.bases, at(60));
    assert!(back.failed.is_none(), "{:?}", back.failed);
    assert_eq!(codex_text(&h), CODEX);
    assert_eq!(agy_text(&h), AGY);
    assert_eq!(claude_servers(&h), ["browser", "tracker"]);
    let browser: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(h.claude.join("servers/browser")).unwrap())
            .unwrap();
    assert_eq!(browser["command"], "npx");
    assert_eq!(browser["args"], serde_json::json!(["-y", "browser-mcp"]));
    assert!(!kho::load(&h.app).parked.contains_key("browser"));
    app_holds_no_secret(&h);
}

#[test]
fn a_file_changed_after_the_preview_stops_the_whole_batch() {
    let h = setup();
    let p =
        preview(&h, &[change("docs", Agent::Codex, To::On), change("browser", Agent::Agy, To::On)]);
    // Antigravity rewrites its file meanwhile.
    let changed = AGY.replace("browser-mcp", "browser-mcp@2");
    std::fs::write(h.home.join(".gemini/config/mcp_config.json"), &changed).unwrap();
    let out = write(&h, &p);
    let failed = out.failed.expect("refused");
    assert_eq!((failed.agent, failed.error), (Agent::Agy, batch::Failure::Changed));
    assert!(out.done.is_empty() && out.undo.is_none());
    assert_eq!(codex_text(&h), CODEX, "no file is written, not even one that did not change");
    assert_eq!(agy_text(&h), changed);
    assert!(files_under(&h.app.path().join("backups")).is_empty());
}

#[test]
fn a_file_that_does_not_parse_is_shown_and_never_written() {
    let h = setup();
    let broken = CODEX.replace("[mcp_servers.docs]", "[mcp_servers.docs");
    std::fs::write(h.home.join(".codex/config.toml"), &broken).unwrap();
    let view = agents::look(&h.places, &h.app, &h.secrets);
    let codex = view.columns.iter().find(|c| c.agent == Agent::Codex).unwrap();
    assert_eq!((codex.state, codex.writable), ("invalid", false));
    assert!(view.rows.iter().all(|r| r.cells[&Agent::Codex].lock == Some("readOnly")));
    let p = preview(&h, &[change("browser", Agent::Codex, To::On)]);
    assert_eq!(p.refused[0].reason, "readOnly");
    assert!(p.changes.is_empty() && p.groups.is_empty());
    let mut bases = p.bases.clone();
    bases.insert(Agent::Codex, codex_files::sha256(broken.as_bytes()));
    let forced = batch::write(
        &h.places,
        &h.app,
        &h.secrets,
        &[change("browser", Agent::Codex, To::On)],
        &bases,
        at(0),
    );
    assert!(matches!(forced.failed.unwrap().error, batch::Failure::Refused { .. }));
    assert_eq!(codex_text(&h), broken);
}

#[test]
fn claude_code_s_off_keeps_the_server_and_its_secret_in_the_app() {
    let h = setup();
    let out = write(&h, &preview(&h, &[change("tracker", Agent::Claude, To::Off)]));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    assert_eq!(claude_servers(&h), ["browser"]);
    let item = h.keychain.join(format!("items/{}--tracker:header:Authorization", secrets::SERVICE));
    assert_eq!(std::fs::read_to_string(&item).unwrap(), "\"Bearer trap-tracker-key\"");
    let view = agents::look(&h.places, &h.app, &h.secrets);
    let tracker = view.rows.iter().find(|r| r.name == "tracker").unwrap();
    let cell = &tracker.cells[&Agent::Claude];
    assert_eq!((cell.state, cell.parked, cell.lock), ("off", true, None));
    app_holds_no_secret(&h);

    // The Keychain item gone: the box is locked until the key is pasted.
    std::fs::remove_file(&item).unwrap();
    let view = agents::look(&h.places, &h.app, &h.secrets);
    let tracker = view.rows.iter().find(|r| r.name == "tracker").unwrap();
    assert_eq!(tracker.cells[&Agent::Claude].lock, Some("missingKey"));
    assert!(!tracker.secrets[0].have);
    let p = preview(&h, &[change("tracker", Agent::Claude, To::On)]);
    assert_eq!(p.refused[0].reason, "missingKey");

    h.secrets.save("tracker", "header:Authorization", "Bearer trap-tracker-key").unwrap();
    let out = write(&h, &preview(&h, &[change("tracker", Agent::Claude, To::On)]));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    let back: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(h.claude.join("servers/tracker")).unwrap())
            .unwrap();
    assert_eq!(back["type"], "sse");
    assert_eq!(back["headers"]["Authorization"], "Bearer trap-tracker-key");
    assert!(kho::load(&h.app).parked.is_empty());
    app_holds_no_secret(&h);
}

#[test]
fn put_away_from_every_agent_then_brought_back() {
    let h = setup();
    let out = write(&h, &preview(&h, &[change("files", Agent::Codex, To::Absent { store: true })]));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    assert!(!codex_text(&h).contains("files"), "{}", codex_text(&h));
    assert!(codex_text(&h).contains("[mcp_servers.docs]"));
    let view = agents::look(&h.places, &h.app, &h.secrets);
    assert!(view.rows.iter().all(|r| r.name != "files"));
    assert_eq!(view.stored.len(), 1);
    assert_eq!(
        (view.stored[0].name.as_str(), view.stored[0].agents.as_slice()),
        ("files", &[Agent::Codex][..])
    );
    assert!(view.stored[0].missing.is_empty());
    app_holds_no_secret(&h);

    let p =
        preview(&h, &[change("files", Agent::Codex, To::On), change("files", Agent::Agy, To::On)]);
    assert!(p.refused.is_empty(), "{:?}", p.refused);
    let out = write(&h, &p);
    assert!(out.failed.is_none(), "{:?}", out.failed);
    let codex = agents::codex_mcp::read(&codex_text(&h)).unwrap();
    let files = codex.iter().find(|s| s.name == "files").unwrap();
    assert_eq!(
        files.def.as_ref().unwrap().get("env:FILES_TOKEN"),
        Some("trap-files-token"),
        "from the Keychain"
    );
    assert!(agy_text(&h).contains("\"FILES_TOKEN\": \"trap-files-token\""));
    assert!(kho::load(&h.app).stored.is_empty());
    app_holds_no_secret(&h);
}

#[test]
fn editing_replaces_one_entry_and_keeps_its_secret() {
    let h = setup();
    let def = Def::Stdio {
        command: "npx".into(),
        args: vec!["-y".into(), "files-mcp".into(), "--root".into(), "/work".into()],
        env: vec![Pair::new("FILES_TOKEN", MASK)],
        cwd: None,
    };
    let to = To::Def { def, from: Some(Agent::Codex), keychain: vec![], off: false };
    let out = write(&h, &preview(&h, &[change("files", Agent::Codex, to)]));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    let after = codex_text(&h);
    assert!(after.contains("args = [\"-y\", \"files-mcp\", \"--root\", \"/work\"]\nenv = { FILES_TOKEN = \"trap-files-token\" }\n"), "{after}");
    assert!(!after.contains("[mcp_servers.files.env]"));
    assert!(
        after.contains("experimental_bearer_token = \"trap-provider-key\""),
        "the provider is not touched"
    );
    app_holds_no_secret(&h);
}

#[test]
fn a_new_server_takes_its_typed_secret_from_the_keychain() {
    let h = setup();
    h.secrets.save("notes", "env:NOTES_TOKEN", "trap-notes-token").unwrap();
    let def = Def::Stdio {
        command: "notes-mcp".into(),
        args: vec![],
        env: vec![Pair::new("NOTES_TOKEN", MASK)],
        cwd: None,
    };
    let to = || To::Def {
        def: def.clone(),
        from: None,
        keychain: vec!["env:NOTES_TOKEN".into()],
        off: false,
    };
    let p = preview(
        &h,
        &[
            change("notes", Agent::Claude, to()),
            change("notes", Agent::Codex, to()),
            change("notes", Agent::Agy, to()),
        ],
    );
    assert!(p.refused.is_empty(), "{:?}", p.refused);
    no_trap("the preview", &serde_json::to_string(&p).unwrap());
    let out = write(&h, &p);
    assert!(out.failed.is_none(), "{:?}", out.failed);
    assert!(codex_text(&h).contains("[mcp_servers.notes]\ncommand = \"notes-mcp\"\nenv = { NOTES_TOKEN = \"trap-notes-token\" }\n"));
    assert!(agy_text(&h).contains("\"NOTES_TOKEN\": \"trap-notes-token\""));
    assert!(std::fs::read_to_string(h.claude.join("servers/notes"))
        .unwrap()
        .contains("trap-notes-token"));
    app_holds_no_secret(&h);

    // Without the item, and nowhere in a file, it cannot be added.
    let h = setup();
    let p = preview(&h, &[change("notes", Agent::Codex, to())]);
    assert_eq!(p.refused[0].reason, "missingKey");
}

#[test]
fn a_pasted_server_s_secret_goes_to_the_keychain_and_the_add_takes_it_from_there() {
    let h = setup();
    let text = r#"{"mcpServers": {"wiki": {"command": "wiki-mcp", "args": ["--space", "team"], "env": {"WIKI_TOKEN": "trap-wiki-token"}}}}"#;
    // Read as it is typed: nothing saved, nothing sent but the mask.
    let look = agents::paste::paste(text, None);
    assert!(!std::fs::read_to_string(h.keychain.join("calls"))
        .unwrap_or_default()
        .contains("wiki"));
    assert!(!serde_json::to_string(&look).unwrap().contains("trap-wiki-token"));
    // "Tiếp": the secret goes to the Keychain under the server's name.
    let saved = agents::paste::paste(text, Some(&h.secrets));
    assert_eq!(saved.error, None);
    let s = &saved.servers[0];
    assert_eq!(h.secrets.read("wiki", "env:WIKI_TOKEN").unwrap(), "trap-wiki-token");
    assert!(!serde_json::to_string(&saved).unwrap().contains("trap-wiki-token"));
    let keychain: Vec<String> = s.slots.iter().map(|x| x.id.clone()).collect();
    let to = To::Def { def: s.def.clone(), from: None, keychain, off: false };
    let p = preview(&h, &[change("wiki", Agent::Codex, to)]);
    assert!(p.refused.is_empty(), "{:?}", p.refused);
    assert!(!serde_json::to_string(&p).unwrap().contains("trap-wiki-token"));
    assert!(write(&h, &p).failed.is_none());
    assert!(codex_text(&h).contains("env = { WIKI_TOKEN = \"trap-wiki-token\" }"));
    for (path, text) in files_under(h.app.path()) {
        assert!(!text.contains("trap-wiki-token"), "{}", path.display());
    }
}

#[test]
fn an_app_s_own_server_sse_for_codex_and_bad_names_are_refused() {
    let h = setup();
    let p = preview(
        &h,
        &[
            change("node_repl", Agent::Codex, To::Off),
            change("tracker", Agent::Codex, To::On),
            change("-x", Agent::Claude, To::On),
        ],
    );
    let reasons: Vec<&str> = p.refused.iter().map(|r| r.reason).collect();
    assert_eq!(reasons, ["owned", "sse", "badName"]);
    assert!(p.groups.is_empty());
}

#[test]
fn what_claude_prints_is_shown_without_the_secret() {
    let h = setup();
    std::fs::write(h.claude.join("fail"), "").unwrap();
    let out = write(&h, &preview(&h, &[change("docs", Agent::Claude, To::On)]));
    let failed = out.failed.expect("claude failed");
    let text = serde_json::to_string(&failed).unwrap();
    assert!(text.contains("could not add docs"), "{text}");
    assert!(text.contains(MASK), "{text}");
    no_trap("the failure", &text);
    app_holds_no_secret(&h);
}

#[test]
fn every_keychain_call_names_the_app_s_own_service() {
    let h = setup();
    write(&h, &preview(&h, &[change("tracker", Agent::Claude, To::Off)]));
    write(&h, &preview(&h, &[change("files", Agent::Codex, To::Absent { store: true })]));
    agents::look(&h.places, &h.app, &h.secrets);
    let calls = std::fs::read_to_string(h.keychain.join("calls")).unwrap();
    assert!(!calls.is_empty());
    for call in calls.lines() {
        let args: Vec<&str> = call.split('\x1f').collect();
        let s = args.iter().position(|a| *a == "-s").expect("a service");
        assert_eq!(args[s + 1], secrets::SERVICE, "{call}");
        assert!(!args.contains(&"-A"), "{call}");
    }
}

#[test]
fn the_codex_tab_does_not_count_the_app_s_own_mcp_write_as_made_outside() {
    let h = setup();
    let mut record = codex_files::Record {
        written_hash: Some(codex_files::sha256(CODEX.as_bytes())),
        ..Default::default()
    };
    codex_files::save(&h.app, &record).unwrap();
    let out = write(&h, &preview(&h, &[change("docs", Agent::Codex, To::On)]));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    let now = codex_files::sha256(codex_text(&h).as_bytes());
    record = codex_files::load(&h.app);
    assert!(!record.changed_outside(&now));
    assert_eq!(record.written_hash.as_deref(), Some(now.as_str()));
}

#[test]
fn the_mcp_code_names_no_login_file_and_runs_security_in_one_place() {
    let sources = [
        ("mod.rs", include_str!("../src/agents/mod.rs")),
        ("agy_mcp.rs", include_str!("../src/agents/agy_mcp.rs")),
        ("batch.rs", include_str!("../src/agents/batch.rs")),
        ("claude_mcp.rs", include_str!("../src/agents/claude_mcp.rs")),
        ("codex_mcp.rs", include_str!("../src/agents/codex_mcp.rs")),
        ("def.rs", include_str!("../src/agents/def.rs")),
        ("json_text.rs", include_str!("../src/agents/json_text.rs")),
        ("kho.rs", include_str!("../src/agents/kho.rs")),
        ("paste.rs", include_str!("../src/agents/paste.rs")),
        ("redact.rs", include_str!("../src/agents/redact.rs")),
        ("secrets.rs", include_str!("../src/agents/secrets.rs")),
        ("write.rs", include_str!("../src/agents/write.rs")),
    ];
    for (name, text) in sources {
        for banned in
            ["auth.json", ".credentials.json", ".cc-switch", "claude-swap", "oauth_creds", "\"-A\""]
        {
            assert!(!text.contains(banned), "{name} names {banned}");
        }
        if name != "secrets.rs" {
            assert!(!text.contains("/usr/bin/security"), "{name} runs security");
            assert!(!text.contains("generic-password"), "{name} touches a Keychain item");
        }
    }
}
