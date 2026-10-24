// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Configuration sets (M15) on a temporary home: Codex's `config.toml`
//! with MCP servers, a custom provider and a `[[skills.config]]` entry,
//! Antigravity's `mcp_config.json`, Claude Code's list kept by a fake
//! `claude` that records its arguments, a shared folder of skills with the
//! agents' links, a fake `npx` that records its arguments, and a fake
//! `security` for the app's Keychain items. Every name, URL and key here is
//! made up; no test reads or writes the real agents' files, the real skills,
//! the Keychain or the real `claude` and `npx`.
//!
//! The keys are traps: each may be in the agent files that hold its server,
//! the fake `claude`'s arguments and the fake Keychain, and nowhere else:
//! not in the app's folder (the sets file, backups, store, log) nor in what
//! the windows get (the preview, the result, the undo).

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::claude::fake_claude;
use common::keychain::fake_security;
use common::{script, temp_dir};
use quota_station_core::agents::batch::{self, Change, To};
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::sets::{self, ApplyError, Provider, Sent, Set, SetError, Want};
use quota_station_core::agents::skills::{self, SkillPlaces};
use quota_station_core::agents::{self, Agent, Places};
use quota_station_core::app_dir::AppDir;
use time::OffsetDateTime;

const TRAPS: &[&str] = &[
    "trap-provider-key-2468",
    "trap-files-token-1357",
    "trap-docs-key-8642",
    "trap-browser-token-9753",
];
const SENTINEL: &str = "trap-claude-account-sentinel";

const CODEX: &str = r#"# Codex config for the sets tests. Every name, URL and key is made up.
model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key-2468"

[mcp_servers.files]
command = "npx"
args = ["-y", "files-mcp"]

[mcp_servers.files.env]
FILES_TOKEN = "trap-files-token-1357"

[mcp_servers.docs]
url = "https://docs.example.com/mcp"
enabled = false

[[skills.config]]
name = "release-notes"
enabled = false
"#;

const AGY: &str = r#"{
  "mcpServers": {
    "docs": {
      "serverUrl": "https://docs.example.com/mcp",
      "headers": {
        "DOCS_API_KEY": "trap-docs-key-8642"
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

const BROWSER: &str = r#"{"type":"stdio","command":"npx","args":["-y","browser-mcp"],"env":{"BROWSER_TOKEN":"trap-browser-token-9753"}}"#;
const NOTES: &str = r#"{"type":"stdio","command":"npx","args":["-y","notes-mcp"],"env":{}}"#;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000 + offset).unwrap()
}

struct Home {
    root: PathBuf,
    home: PathBuf,
    app: AppDir,
    places: Places,
    skills: SkillPlaces,
    secrets: Secrets,
    claude: PathBuf,
    npx: PathBuf,
}

fn skill(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\ndescription: {name}\n---\n"))
        .unwrap();
}

fn link(at: &Path, to: &str) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(to, at).unwrap();
}

fn setup() -> Home {
    let root = temp_dir("sets");
    let home = root.join("home");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
    std::fs::write(home.join(".codex/config.toml"), CODEX).unwrap();
    std::fs::write(home.join(".gemini/config/mcp_config.json"), AGY).unwrap();
    let claude = root.join("claude");
    let cli = fake_claude(&claude, &home, SENTINEL);
    std::fs::write(claude.join("servers/browser"), BROWSER).unwrap();
    std::fs::write(claude.join("servers/notes"), NOTES).unwrap();
    // Builds `~/.claude.json`; not a call the app made.
    std::process::Command::new(&cli).args(["mcp", "list"]).status().unwrap();
    std::fs::remove_file(claude.join("calls")).unwrap();

    let shared = home.join(".agents/skills");
    for n in ["pdf-tools", "ui-polish", "release-notes"] {
        skill(&shared.join(n), n);
    }
    for n in ["pdf-tools", "ui-polish"] {
        link(&home.join(".claude/skills").join(n), &format!("../../.agents/skills/{n}"));
    }
    link(&home.join(".gemini/config/skills/pdf-tools"), "../../../.agents/skills/pdf-tools");
    // Codex reads the shared folder itself.
    std::fs::create_dir_all(home.join(".codex/skills")).unwrap();

    let npx = root.join("npx");
    let npx_bin = script(
        &npx.join("bin/npx"),
        &format!(
            "for a in \"$@\"; do printf '%s\\037' \"$a\" >> '{}/calls'; done\nexit 1\n",
            npx.display()
        ),
    );
    let keychain = root.join("keychain");
    let secrets = Secrets::fake(fake_security(&keychain));
    let places = Places::new(&home, &home.join(".codex"), Some(cli));
    let skills = SkillPlaces::new(&home, &home.join(".codex"), Some(npx_bin));
    Home { app: AppDir::new(root.join("app")), root, home, places, skills, secrets, claude, npx }
}

fn web_app() -> Set {
    let w = |items: &[(&str, Want)]| -> BTreeMap<String, Want> {
        items.iter().map(|(n, w)| (n.to_string(), *w)).collect()
    };
    Set {
        id: String::new(),
        name: "Web app".into(),
        mcp: w(&[("docs", Want::On), ("browser", Want::Off), ("files", Want::Off)]),
        skills: w(&[("ui-polish", Want::On), ("release-notes", Want::On)]),
        codex: None,
    }
}

fn put(h: &Home, set: &Set) -> Set {
    sets::put(&h.app, set, at(0)).unwrap()
}

fn preview(h: &Home, set: &Set) -> sets::Preview {
    sets::preview(&h.places, &h.skills, &h.app, &h.secrets, set)
}

fn sent(p: &sets::Preview) -> Sent {
    Sent {
        mcp: p.mcp.changes.clone(),
        mcp_bases: p.mcp.bases.clone(),
        skills: p.skills.changes.clone(),
        skill_bases: p.skills.bases.clone(),
    }
}

fn apply(h: &Home, id: &str, s: &Sent, codex_now: &str) -> Result<sets::Outcome, ApplyError> {
    sets::apply(&h.places, &h.skills, &h.app, &h.secrets, id, s, codex_now, at(60))
}

fn codex_text(h: &Home) -> String {
    std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap()
}

fn agy_text(h: &Home) -> String {
    std::fs::read_to_string(h.home.join(".gemini/config/mcp_config.json")).unwrap()
}

/// What `claude` was run with, one run per line.
fn claude_calls(h: &Home) -> Vec<Vec<String>> {
    std::fs::read_to_string(h.claude.join("calls"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.split('\x1f').filter(|a| !a.is_empty()).map(str::to_string).collect())
        .collect()
}

/// Each MCP server's box, by agent, as the table reads it now.
fn mcp_boxes(h: &Home) -> BTreeMap<(String, Agent), &'static str> {
    let view = agents::look(&h.places, &h.app, &h.secrets);
    view.rows
        .iter()
        .flat_map(|r| r.cells.iter().map(move |(a, c)| ((r.name.clone(), *a), c.state)))
        .collect()
}

/// Each skill's box, by agent.
fn skill_boxes(h: &Home) -> BTreeMap<(String, Agent), &'static str> {
    let view = sets::skills_view(&h.skills);
    view.rows
        .iter()
        .flat_map(|r| r.cells.iter().map(move |(a, c)| ((r.name.clone(), *a), c.state)))
        .collect()
}

/// Every file under `dir` as text.
fn all_text(dir: &Path) -> String {
    let mut out = String::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let m = std::fs::symlink_metadata(e.path()).unwrap();
            if m.is_dir() {
                stack.push(e.path());
            } else if m.is_file() {
                out.push_str(&String::from_utf8_lossy(&std::fs::read(e.path()).unwrap()));
            }
        }
    }
    out
}

fn no_trap(what: &str, text: &str) {
    for t in TRAPS.iter().chain([&SENTINEL]) {
        assert!(!text.contains(t), "{t} is in {what}");
    }
}

#[test]
fn a_set_is_kept_by_name_and_never_touches_an_agent_file() {
    let h = setup();
    let s = put(&h, &web_app());
    assert!(s.id.starts_with('s'));
    assert_eq!(sets::load(&h.app).sets, std::slice::from_ref(&s));
    // Same name, any case: taken.
    let again = Set { name: "web APP".into(), ..web_app() };
    assert_eq!(sets::put(&h.app, &again, at(1)), Err(SetError::Taken));
    let bad = Set { mcp: [("a b".to_string(), Want::On)].into(), ..web_app() };
    assert_eq!(
        sets::put(&h.app, &Set { name: "Khác".into(), ..bad }, at(1)),
        Err(SetError::BadEntry("a b".into()))
    );
    // In place, by id.
    let renamed = put(&h, &Set { name: "Viết tài liệu".into(), ..s.clone() });
    assert_eq!(renamed.id, s.id);
    assert_eq!(sets::load(&h.app).sets.len(), 1);

    let removed = sets::delete(&h.app, &s.id).unwrap();
    assert!(sets::load(&h.app).sets.is_empty());
    sets::undelete(&h.app, &removed).unwrap();
    assert_eq!(sets::load(&h.app).sets[0].name, "Viết tài liệu");

    assert_eq!(codex_text(&h), CODEX);
    assert_eq!(agy_text(&h), AGY);
    assert!(claude_calls(&h).is_empty());
    assert!(!h.npx.join("calls").exists());
    no_trap("the sets file", &all_text(h.app.path()));
}

#[test]
fn the_preview_lists_the_set_s_entries_for_each_agent_and_writes_nothing() {
    let h = setup();
    let s = put(&h, &web_app());
    let p = preview(&h, &s);
    let mcp: Vec<(&str, Agent, &To)> =
        p.mcp.changes.iter().map(|c| (c.name.as_str(), c.agent, &c.to)).collect();
    // browser: Claude Code turns it off; Antigravity's is off already.
    // docs: Codex turns it on; Antigravity's is on already. files: Codex.
    assert_eq!(
        mcp,
        [
            ("browser", Agent::Claude, &To::Off),
            ("docs", Agent::Codex, &To::On),
            ("files", Agent::Codex, &To::Off)
        ]
    );
    let sk: Vec<(&str, Agent, bool)> =
        p.skills.changes.iter().map(|c| (c.name.as_str(), c.agent, c.on)).collect();
    assert_eq!(
        sk,
        [
            ("release-notes", Agent::Claude, true),
            ("release-notes", Agent::Codex, true),
            ("release-notes", Agent::Agy, true),
            ("ui-polish", Agent::Agy, true),
        ]
    );
    assert!(p.skipped.is_empty(), "{:?}", p.skipped);
    assert_eq!(p.codex, None);
    no_trap("the preview", &serde_json::to_string(&p).unwrap());

    // Nothing written, nothing run.
    assert_eq!(codex_text(&h), CODEX);
    assert_eq!(agy_text(&h), AGY);
    assert!(claude_calls(&h).is_empty());
    assert!(!h.app.path().join("backups").exists());
    assert!(!h.npx.join("calls").exists());
}

#[test]
fn an_entry_this_mac_has_not_got_or_a_locked_box_is_skipped() {
    let h = setup();
    let mut s = web_app();
    s.mcp.insert("tracker".into(), Want::On);
    s.skills.insert("sql-helper".into(), Want::Off);
    // A skill of Claude Code's own, not in the shared folder: locked.
    skill(&h.home.join(".claude/skills/deploy-check"), "deploy-check");
    s.skills.insert("deploy-check".into(), Want::Off);
    let p = preview(&h, &put(&h, &s));
    let skipped: Vec<(&str, &str, Option<Agent>, &str)> =
        p.skipped.iter().map(|k| (k.kind, k.name.as_str(), k.agent, k.reason)).collect();
    assert_eq!(
        skipped,
        [
            ("mcp", "tracker", None, "notHere"),
            ("skill", "deploy-check", Some(Agent::Claude), "hand"),
            ("skill", "sql-helper", None, "notHere"),
        ]
    );
}

#[test]
fn applying_changes_only_the_entries_in_the_set() {
    let h = setup();
    let before_mcp = mcp_boxes(&h);
    let before_skills = skill_boxes(&h);
    let s = put(&h, &web_app());
    let p = preview(&h, &s);
    let out = apply(&h, &s.id, &sent(&p), "custom").unwrap();
    assert!(out.ok, "{out:?}");
    assert_eq!(out.mcp.as_ref().unwrap().done.len(), 2, "Codex's file and Claude Code's list");
    assert_eq!(out.skills.as_ref().unwrap().done.len(), 3);

    let after_mcp = mcp_boxes(&h);
    let changed: Vec<(&(String, Agent), &str, &str)> = before_mcp
        .iter()
        .filter(|(k, v)| after_mcp.get(*k) != Some(*v))
        .map(|(k, v)| (k, *v, after_mcp[k]))
        .collect();
    assert_eq!(
        changed,
        [
            (&("browser".to_string(), Agent::Claude), "on", "off"),
            (&("docs".to_string(), Agent::Codex), "off", "on"),
            (&("files".to_string(), Agent::Codex), "on", "off"),
        ]
    );
    let after_skills = skill_boxes(&h);
    let changed: Vec<&(String, Agent)> = before_skills
        .iter()
        .filter(|(k, v)| after_skills.get(*k) != Some(*v))
        .map(|(k, _)| k)
        .collect();
    assert_eq!(
        changed,
        [
            &("release-notes".to_string(), Agent::Claude),
            &("release-notes".to_string(), Agent::Codex),
            &("release-notes".to_string(), Agent::Agy),
            &("ui-polish".to_string(), Agent::Agy),
        ]
    );

    // Codex: only the set's entries' lines; the provider and its key stay.
    let text = codex_text(&h);
    let lines = batch::diff(CODEX, &text);
    for l in lines.iter().filter(|l| l.op == "add" || l.op == "del") {
        assert!(
            l.text.contains("enabled")
                || l.text.contains("release-notes")
                || l.text.contains("[[skills.config]]")
                || l.text.trim().is_empty(),
            "a line the set does not own changed: {l:?}"
        );
    }
    assert!(text.contains("model_provider = \"custom\""));
    assert!(text.contains("experimental_bearer_token = \"trap-provider-key-2468\""));
    assert!(text.contains("FILES_TOKEN = \"trap-files-token-1357\""));
    // Antigravity had nothing to change: its file is as it was.
    assert_eq!(agy_text(&h), AGY);
    // Claude Code: one command, for browser only.
    let calls = claude_calls(&h);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0][..3], ["mcp", "remove", "browser"]);
    // Nothing is installed or updated.
    assert!(!h.npx.join("calls").exists());

    // The apply is kept, names only, and the set is as applied.
    let kept = sets::load(&h.app);
    let applied = kept.applied.as_ref().unwrap();
    assert_eq!((applied.id.as_str(), applied.at), (s.id.as_str(), at(60).unix_timestamp()));
    assert!(applied.can_undo());
    assert_eq!(applied.codex_before, None);
    let mcp_view = agents::look(&h.places, &h.app, &h.secrets);
    assert!(sets::drift(&s, &mcp_view, &sets::skills_view(&h.skills)).is_empty());
}

#[test]
fn codex_s_provider_never_changes_and_the_undo_knows_the_side_it_was_on() {
    let h = setup();
    let s = put(&h, &Set { codex: Some(Provider::ChatGpt), ..web_app() });
    let p = preview(&h, &s);
    assert_eq!(p.codex, Some(Provider::ChatGpt));
    apply(&h, &s.id, &sent(&p), "custom").unwrap();
    assert!(
        codex_text(&h).contains("model_provider = \"custom\""),
        "only the Codex switch changes the provider"
    );
    assert!(codex_text(&h).contains("[model_providers.custom]"));
    assert_eq!(sets::load(&h.app).applied.unwrap().codex_before.as_deref(), Some("custom"));

    // A set that wants the side Codex is on has nothing to switch back.
    let h = setup();
    let s = put(&h, &Set { codex: Some(Provider::Custom), ..web_app() });
    let p = preview(&h, &s);
    apply(&h, &s.id, &sent(&p), "custom").unwrap();
    assert_eq!(sets::load(&h.app).applied.unwrap().codex_before, None);
}

#[test]
fn a_set_with_only_a_codex_side_writes_nothing() {
    let h = setup();
    let s = put(
        &h,
        &Set {
            mcp: BTreeMap::new(),
            skills: BTreeMap::new(),
            codex: Some(Provider::ChatGpt),
            ..web_app()
        },
    );
    let p = preview(&h, &s);
    assert!(p.mcp.changes.is_empty() && p.skills.changes.is_empty());
    let out = apply(&h, &s.id, &sent(&p), "custom").unwrap();
    assert!(out.ok && out.mcp.is_none() && out.skills.is_none());
    assert_eq!(codex_text(&h), CODEX);
    let applied = sets::load(&h.app).applied.unwrap();
    assert!(!applied.can_undo());
    assert_eq!(applied.codex_before.as_deref(), Some("custom"));
}

#[test]
fn a_file_changed_since_the_preview_stops_the_whole_set() {
    let h = setup();
    let s = put(&h, &web_app());
    let p = preview(&h, &s);
    let edited = CODEX.replace("model = \"gpt-5.5\"", "model = \"gpt-5.5-mini\"");
    std::fs::write(h.home.join(".codex/config.toml"), &edited).unwrap();
    assert_eq!(
        apply(&h, &s.id, &sent(&p), "custom").unwrap_err(),
        ApplyError::Changed(Agent::Codex)
    );
    assert_eq!(codex_text(&h), edited);
    assert!(claude_calls(&h).is_empty(), "Claude Code's part waits for Codex's");
    assert!(!h.home.join(".claude/skills/release-notes").exists());
    assert!(sets::load(&h.app).applied.is_none());

    // A skills folder changed since: nothing written either.
    let h = setup();
    let s = put(&h, &web_app());
    let p = preview(&h, &s);
    link(&h.home.join(".gemini/config/skills/ui-polish"), "../../../.agents/skills/ui-polish");
    assert_eq!(apply(&h, &s.id, &sent(&p), "custom").unwrap_err(), ApplyError::Changed(Agent::Agy));
    assert_eq!(codex_text(&h), CODEX);
    assert!(claude_calls(&h).is_empty());
}

#[test]
fn only_the_boxes_the_set_changes_are_written() {
    let h = setup();
    let s = put(&h, &web_app());
    let p = preview(&h, &s);
    let mut more = sent(&p);
    more.mcp.push(Change { name: "notes".into(), agent: Agent::Claude, to: To::Off });
    assert_eq!(apply(&h, &s.id, &more, "custom").unwrap_err(), ApplyError::NotTheSet);
    let mut more = sent(&p);
    more.skills.push(skills::Change { name: "pdf-tools".into(), agent: Agent::Claude, on: false });
    assert_eq!(apply(&h, &s.id, &more, "custom").unwrap_err(), ApplyError::NotTheSet);
    assert_eq!(codex_text(&h), CODEX);
    assert!(claude_calls(&h).is_empty());
    assert_eq!(apply(&h, "s-gone", &sent(&p), "custom").unwrap_err(), ApplyError::Missing);
}

#[test]
fn drift_names_the_entries_changed_since_the_apply() {
    let h = setup();
    let s = put(&h, &web_app());
    apply(&h, &s.id, &sent(&preview(&h, &s)), "custom").unwrap();
    // files turned back on in the MCP tab.
    let p = batch::preview(
        &h.places,
        &h.app,
        &h.secrets,
        &[Change { name: "files".into(), agent: Agent::Codex, to: To::On }],
    );
    batch::write(&h.places, &h.app, &h.secrets, &p.changes, &p.bases, at(120));
    let d = sets::drift(
        &s,
        &agents::look(&h.places, &h.app, &h.secrets),
        &sets::skills_view(&h.skills),
    );
    assert_eq!(d.len(), 1);
    assert_eq!(
        (d[0].kind, d[0].name.as_str(), d[0].want, d[0].agents.as_slice()),
        ("mcp", "files", Want::Off, &[Agent::Codex][..])
    );
}

#[test]
fn undo_puts_the_whole_set_back() {
    let h = setup();
    let before_mcp = mcp_boxes(&h);
    let before_skills = skill_boxes(&h);
    let s = put(&h, &web_app());
    apply(&h, &s.id, &sent(&preview(&h, &s)), "custom").unwrap();
    let u = sets::undo(&h.places, &h.skills, &h.app, &h.secrets, at(180)).unwrap();
    assert!(u.ok && u.left.is_empty(), "{u:?}");
    assert_eq!(mcp_boxes(&h), before_mcp);
    assert_eq!(skill_boxes(&h), before_skills);
    // Claude Code's browser is back with its own key, from the app's Keychain item.
    let browser = std::fs::read_to_string(h.claude.join("servers/browser")).unwrap();
    assert!(browser.contains("trap-browser-token-9753"));
    assert!(codex_text(&h).contains("model_provider = \"custom\""));
    assert!(sets::load(&h.app).applied.is_none());
    assert_eq!(
        sets::undo(&h.places, &h.skills, &h.app, &h.secrets, at(200)).unwrap_err(),
        sets::UndoError::Nothing
    );
}

#[test]
fn undo_leaves_what_changed_since_and_survives_a_codex_switch() {
    let h = setup();
    let s = put(&h, &Set { codex: Some(Provider::ChatGpt), ..web_app() });
    apply(&h, &s.id, &sent(&preview(&h, &s)), "custom").unwrap();
    // Step 2: the Codex switch changes the provider line.
    let switched =
        codex_text(&h).replace("model_provider = \"custom\"", "model_provider = \"openai\"");
    std::fs::write(h.home.join(".codex/config.toml"), &switched).unwrap();
    // files turned on again by hand; docs removed from Codex by hand.
    let p = batch::preview(
        &h.places,
        &h.app,
        &h.secrets,
        &[
            Change { name: "files".into(), agent: Agent::Codex, to: To::On },
            Change { name: "docs".into(), agent: Agent::Codex, to: To::Absent { store: false } },
        ],
    );
    assert_eq!(p.changes.len(), 2);
    batch::write(&h.places, &h.app, &h.secrets, &p.changes, &p.bases, at(120));

    let u = sets::undo(&h.places, &h.skills, &h.app, &h.secrets, at(180)).unwrap();
    assert!(u.ok, "{u:?}");
    assert_eq!(u.codex_before.as_deref(), Some("custom"));
    assert_eq!(u.left, ["docs"], "docs is gone from Codex: not added back");
    let boxes = mcp_boxes(&h);
    assert_eq!(boxes[&("browser".to_string(), Agent::Claude)], "on");
    assert_eq!(boxes[&("files".to_string(), Agent::Codex)], "on");
    assert_eq!(boxes[&("docs".to_string(), Agent::Codex)], "absent");
    let text = codex_text(&h);
    assert!(text.contains("model_provider = \"openai\""), "the undo does not switch Codex");
    assert!(skill_boxes(&h)[&("release-notes".to_string(), Agent::Codex)] == "off");
}

#[test]
fn no_key_reaches_the_app_s_folder_or_the_windows() {
    let h = setup();
    let s = put(&h, &Set { codex: Some(Provider::ChatGpt), ..web_app() });
    let p = preview(&h, &s);
    let out = apply(&h, &s.id, &sent(&p), "custom").unwrap();
    let mcp_view = agents::look(&h.places, &h.app, &h.secrets);
    let drift = sets::drift(&s, &mcp_view, &sets::skills_view(&h.skills));
    let undone = sets::undo(&h.places, &h.skills, &h.app, &h.secrets, at(180)).unwrap();
    for (what, json) in [
        ("the preview", serde_json::to_string(&p).unwrap()),
        ("the result", serde_json::to_string(&out).unwrap()),
        ("the drift", serde_json::to_string(&drift).unwrap()),
        ("the undo", serde_json::to_string(&undone).unwrap()),
        ("the sets", serde_json::to_string(&sets::load(&h.app)).unwrap()),
    ] {
        no_trap(what, &json);
    }
    // Backups, the store, the logs, the sets file.
    no_trap("the app's folder", &all_text(h.app.path()));
    // The keys stay where they were.
    assert!(codex_text(&h).contains("trap-provider-key-2468"));
    assert!(codex_text(&h).contains("trap-files-token-1357"));
    assert!(agy_text(&h).contains("trap-docs-key-8642"));
    assert!(std::fs::read_to_string(h.home.join(".claude.json")).unwrap().contains(SENTINEL));
    assert!(!h.npx.join("calls").exists(), "a set never runs npx");
    let _ = &h.root;
}

#[test]
fn a_key_pasted_for_codex_s_provider_never_goes_into_a_set() {
    use quota_station_core::codex;
    let h = setup();
    let key = "sk-pasted-sets-trap-0123456789";
    let codex_home = h.home.join(".codex");
    let base = codex::files::sha256(codex_text(&h).as_bytes());
    let out = codex::set_key(&codex_home, &h.app, key, None, &base, at(0));
    assert!(matches!(out, codex::KeyOutcome::Written { .. }), "{out:?}");
    let s = put(&h, &web_app());
    let p = preview(&h, &s);
    assert!(!serde_json::to_string(&p).unwrap().contains(key));
    assert!(!all_text(h.app.path()).contains(key), "nor in the app's folder");
}
