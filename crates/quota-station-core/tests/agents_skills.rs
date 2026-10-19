//! The Skills, Plugin and File hướng dẫn tabs (M11) on a temporary home:
//! a shared folder of skills with its lock file, Claude Code's and
//! Antigravity's folders of links, Codex's `config.toml`, a fake `npx` and
//! fake `claude` and `codex` that record their arguments, and a fake GitHub
//! on a local port. Every name, repository and key here is made up; no test
//! reads or writes the real agents' files, the real skills, the Keychain,
//! or reaches the network.
//!
//! Each skill holds a `run.sh` that leaves a mark when it runs: nothing the
//! app does runs it. The keys are traps: never in a backup, a pack, or
//! what the windows get.

mod common;

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use common::{script, temp_dir};
use quota_station_core::agents::instructions::{self, Choices};
use quota_station_core::agents::npx::{self, Npx};
use quota_station_core::agents::pack;
use quota_station_core::agents::plugins;
use quota_station_core::agents::skill_ops;
use quota_station_core::agents::skill_updates::{self, GitHub, Updates};
use quota_station_core::agents::skills::{self, Change, Entry, SkillPlaces};
use quota_station_core::agents::{self, Agent};
use quota_station_core::alerts::{self, Effect, Memory};
use quota_station_core::antigravity::fs::AppDir;
use time::{OffsetDateTime, UtcOffset};

const PROVIDER_TRAP: &str = "trap-provider-key-1234";
const MCP_TRAP: &str = "trap-mcp-token-5678";

const CODEX: &str = r#"# Codex config for the skills tests. Every name, URL and key is made up.
model = "gpt-5.5"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key-1234"

[mcp_servers.files]
command = "npx"
args = ["-y", "files-mcp"]

[mcp_servers.files.env]
FILES_TOKEN = "trap-mcp-token-5678"

[[skills.config]]
name = "release-notes"
enabled = false
"#;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000 + offset).unwrap()
}

struct Home {
    root: PathBuf,
    home: PathBuf,
    app: AppDir,
    places: SkillPlaces,
    /// Where a skill's `run.sh` leaves its mark.
    mark: PathBuf,
}

/// A skill folder: `SKILL.md`, a `run.sh` that marks `mark` when run.
fn skill(dir: &Path, name: &str, mark: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {name} for the tests\n---\n# {name}\n"),
    )
    .unwrap();
    script(&dir.join("run.sh"), &format!("touch '{}'\n", mark.display()));
}

fn link(at: &Path, to: &str) {
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(to, at).unwrap();
}

fn lock_entry(name: &str, repo: &str, path: &str, hash: &str) -> String {
    format!(
        r#""{name}": {{"source": "{repo}", "sourceType": "github", "sourceUrl": "https://github.com/{repo}.git", "skillPath": "{path}", "skillFolderHash": "{hash}", "installedAt": "2026-09-12T08:00:00.000Z", "updatedAt": "2026-09-12T08:00:00.000Z"}}"#
    )
}

fn setup() -> Home {
    let root = temp_dir("skills");
    let home = root.join("home");
    let mark = root.join("ran");
    let shared = home.join(".agents/skills");
    for n in ["pdf-tools", "ui-polish", "release-notes", "sql-helper", "find-notes"] {
        skill(&shared.join(n), n, &mark);
    }
    std::fs::write(shared.join("ui-polish/guide.md"), "Use small radii.\n").unwrap();
    let hash = |n: &str| skill_updates::tree_hash(&shared.join(n)).unwrap();
    let entries = [
        lock_entry(
            "pdf-tools",
            "acme/agent-skills",
            "skills/pdf-tools/SKILL.md",
            &hash("pdf-tools"),
        ),
        lock_entry(
            "ui-polish",
            "acme/agent-skills",
            "skills/ui-polish/SKILL.md",
            &hash("ui-polish"),
        ),
        lock_entry(
            "release-notes",
            "acme/agent-skills",
            "skills/release-notes/SKILL.md",
            &hash("release-notes"),
        ),
        // Installed at another version, then edited on this Mac.
        lock_entry(
            "sql-helper",
            "dataworks/skills",
            "sql-helper/SKILL.md",
            "0123456789abcdef0123456789abcdef01234567",
        ),
    ];
    std::fs::write(
        home.join(".agents/.skill-lock.json"),
        format!(
            "{{\"version\": 3, \"skills\": {{{}}}, \"dismissed\": {{}}}}\n",
            entries.join(",\n")
        ),
    )
    .unwrap();
    // Claude Code: links to the shared folder, and one installed by hand.
    for n in ["pdf-tools", "ui-polish", "release-notes", "sql-helper"] {
        link(&home.join(".claude/skills").join(n), &format!("../../.agents/skills/{n}"));
    }
    skill(&home.join(".claude/skills/deploy-check"), "deploy-check", &mark);
    // Antigravity: one link, one by hand, one link to nothing.
    link(&home.join(".gemini/config/skills/pdf-tools"), "../../../.agents/skills/pdf-tools");
    skill(&home.join(".gemini/config/skills/trace-reader"), "trace-reader", &mark);
    link(&home.join(".gemini/config/skills/old-linter"), "../../../gone/old-linter");
    // Codex: its config and its own skills.
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::write(home.join(".codex/config.toml"), CODEX).unwrap();
    skill(&home.join(".codex/skills/.system/imagegen"), "imagegen", &mark);
    let places = SkillPlaces::new(&home, &home.join(".codex"), None);
    Home { app: AppDir::new(root.join("app")), root, home, places, mark }
}

fn change(name: &str, agent: Agent, on: bool) -> Change {
    Change { name: name.into(), agent, on }
}

fn cell(view: &skills::View, name: &str, agent: Agent) -> (&'static str, Option<&'static str>) {
    let r = view.rows.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name} is a row"));
    let c = &r.cells[&agent];
    (c.state, c.lock)
}

/// Every file under `dir` as text, for the trap checks.
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

#[test]
fn the_table_reads_every_agent_s_folder() {
    let h = setup();
    let look = skills::read(&h.places);
    let view = skills::view(&h.places, &look, Vec::new(), &Updates::default());
    let names: Vec<&str> = view.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "deploy-check",
            "find-notes",
            "pdf-tools",
            "release-notes",
            "sql-helper",
            "trace-reader",
            "ui-polish"
        ]
    );
    assert_eq!(cell(&view, "pdf-tools", Agent::Claude), ("on", None));
    assert_eq!(cell(&view, "pdf-tools", Agent::Codex), ("on", None));
    assert_eq!(cell(&view, "pdf-tools", Agent::Agy), ("on", None));
    assert_eq!(cell(&view, "ui-polish", Agent::Agy), ("off", None));
    assert_eq!(cell(&view, "release-notes", Agent::Codex), ("off", None));
    // In the shared folder with no lock entry: installed by hand there.
    let find = view.rows.iter().find(|r| r.name == "find-notes").unwrap();
    assert_eq!((find.source, find.home), ("hand", None));
    assert_eq!(cell(&view, "find-notes", Agent::Claude), ("off", None));
    // A folder of its own in Claude Code's folder: gather it first.
    let deploy = view.rows.iter().find(|r| r.name == "deploy-check").unwrap();
    assert_eq!(
        (deploy.source, deploy.home, deploy.importable),
        ("hand", Some(Agent::Claude), true)
    );
    assert_eq!(cell(&view, "deploy-check", Agent::Claude), ("on", Some("hand")));
    assert_eq!(cell(&view, "deploy-check", Agent::Codex), ("absent", Some("hand")));
    assert_eq!(cell(&view, "trace-reader", Agent::Agy), ("on", Some("hand")));
    let pdf = view.rows.iter().find(|r| r.name == "pdf-tools").unwrap();
    assert_eq!((pdf.source, pdf.repo.as_deref()), ("github", Some("acme/agent-skills")));
    assert!(!pdf.edited);
    assert!(view.rows.iter().find(|r| r.name == "sql-helper").unwrap().edited);
    assert_eq!(view.broken.len(), 1);
    assert_eq!((view.broken[0].agent, view.broken[0].name.as_str()), (Agent::Agy, "old-linter"));
    assert_eq!(view.owned[0].what, "Codex");
    assert_eq!(view.owned[0].names, ["imagegen"]);
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains(PROVIDER_TRAP) && !json.contains(MCP_TRAP));
    assert!(!h.mark.exists(), "no skill's file ran");
}

#[test]
fn a_batch_makes_links_and_codex_entries_and_undoes_them() {
    let h = setup();
    let changes = [
        change("ui-polish", Agent::Agy, true),
        change("release-notes", Agent::Codex, true),
        change("ui-polish", Agent::Codex, false),
        change("pdf-tools", Agent::Claude, false),
        change("deploy-check", Agent::Codex, true),
        change("deploy-check", Agent::Claude, false),
    ];
    let p = skills::preview(&h.places, &changes);
    let refused: Vec<(&str, Agent, &str)> =
        p.refused.iter().map(|r| (r.name.as_str(), r.agent, r.reason)).collect();
    assert_eq!(
        refused,
        [("deploy-check", Agent::Codex, "hand"), ("deploy-check", Agent::Claude, "hand")]
    );
    assert_eq!(p.changes.len(), 4);
    let codex = p.groups.iter().find(|g| g.agent == Agent::Codex).unwrap();
    let added: Vec<&str> =
        codex.lines.iter().filter(|l| l.op == "add").map(|l| l.text.as_str()).collect();
    // One entry turned on, one off: the entry's name changes.
    assert_eq!(added, ["name = \"ui-polish\""]);
    assert!(codex.lines.iter().any(|l| l.op == "del" && l.text == "name = \"release-notes\""));
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains(PROVIDER_TRAP), "the preview masks Codex's keys");
    let agy = p.groups.iter().find(|g| g.agent == Agent::Agy).unwrap();
    assert_eq!(agy.lines[0].text, "~/.gemini/config/skills/ui-polish → ~/.agents/skills/ui-polish");

    let out = skills::write(&h.places, &h.app, &p.changes, &p.bases, at(0));
    assert!(out.failed.is_none(), "{:?}", out.failed);
    assert_eq!(out.done.len(), 3);
    let made = h.home.join(".gemini/config/skills/ui-polish");
    assert_eq!(
        std::fs::read_link(&made).unwrap(),
        PathBuf::from("../../../.agents/skills/ui-polish")
    );
    // Off is no link: the skill's folder stays.
    assert!(std::fs::symlink_metadata(h.home.join(".claude/skills/pdf-tools")).is_err());
    assert!(h.home.join(".agents/skills/pdf-tools/SKILL.md").is_file());
    let text = std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap();
    assert!(
        text.starts_with(CODEX.split("[[skills.config]]").next().unwrap()),
        "only [[skills.config]] changed"
    );
    assert!(text.contains("name = \"ui-polish\"\nenabled = false"));
    assert!(!text.contains("release-notes"));
    // The backups: Codex's without its keys, the links as they were.
    let backups = all_text(&h.app.path().join("backups"));
    assert!(
        backups.contains("[[skills.config]]")
            && !backups.contains(PROVIDER_TRAP)
            && !backups.contains(MCP_TRAP)
    );
    assert!(std::fs::read_dir(h.app.path().join("backups/skills")).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("claude-links.json.")));

    let undo = out.undo.unwrap();
    let back = skills::write(&h.places, &h.app, &undo.changes, &undo.bases, at(60));
    assert!(back.failed.is_none(), "{:?}", back.failed);
    assert_eq!(std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap(), CODEX);
    assert!(std::fs::symlink_metadata(&made).is_err());
    assert_eq!(
        std::fs::read_link(h.home.join(".claude/skills/pdf-tools")).unwrap(),
        PathBuf::from("../../.agents/skills/pdf-tools")
    );
    assert!(!h.mark.exists());
}

#[test]
fn a_folder_changed_after_the_preview_is_not_written() {
    let h = setup();
    let p = skills::preview(
        &h.places,
        &[change("ui-polish", Agent::Codex, false), change("find-notes", Agent::Claude, true)],
    );
    link(&h.home.join(".claude/skills/other"), "../../.agents/skills/pdf-tools");
    let out = skills::write(&h.places, &h.app, &p.changes, &p.bases, at(0));
    assert!(matches!(out.failed.as_ref().map(|f| &f.error), Some(skills::Failure::Changed)));
    assert!(out.done.is_empty());
    assert_eq!(
        std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap(),
        CODEX,
        "nothing written"
    );
    assert!(std::fs::symlink_metadata(h.home.join(".claude/skills/find-notes")).is_err());
}

#[test]
fn turning_off_never_removes_a_folder() {
    let h = setup();
    let hand = h.home.join(".claude/skills/deploy-check");
    assert!(skills::unlink(&h.places, Agent::Claude, "deploy-check").is_err());
    assert!(skills::drop_broken(&h.places, Agent::Claude, "deploy-check").is_err());
    assert!(hand.join("SKILL.md").is_file());
    // A link to nothing goes; only the link.
    skills::drop_broken(&h.places, Agent::Agy, "old-linter").unwrap();
    assert!(std::fs::symlink_metadata(h.home.join(".gemini/config/skills/old-linter")).is_err());
}

#[test]
fn a_codex_config_in_another_shape_is_read_only() {
    let h = setup();
    std::fs::write(h.home.join(".codex/config.toml"), "skills = { config = [] }\n").unwrap();
    let look = skills::read(&h.places);
    let view = skills::view(&h.places, &look, Vec::new(), &Updates::default());
    let col = view.columns.iter().find(|c| c.agent == Agent::Codex).unwrap();
    assert_eq!((col.state, col.writable), ("invalid", false));
    assert_eq!(cell(&view, "pdf-tools", Agent::Codex).1, Some("readOnly"));
}

#[test]
fn gathering_moves_a_skill_into_the_shared_folder_and_back() {
    let h = setup();
    let g =
        skills::import(&h.places, &h.app, &["deploy-check".into(), "trace-reader".into()], at(0));
    assert!(g.failed.is_none(), "{:?}", g.failed);
    assert_eq!(g.moved.len(), 2);
    let shared = h.home.join(".agents/skills");
    assert!(shared.join("deploy-check/SKILL.md").is_file());
    assert_eq!(
        std::fs::read_link(h.home.join(".claude/skills/deploy-check")).unwrap(),
        PathBuf::from("../../.agents/skills/deploy-check")
    );
    assert_eq!(
        std::fs::read_link(h.home.join(".gemini/config/skills/trace-reader")).unwrap(),
        PathBuf::from("../../../.agents/skills/trace-reader")
    );
    // The agents that had them still do; Codex, which did not, has them off.
    let look = skills::read(&h.places);
    assert!(look.on(&h.places, "deploy-check", Agent::Claude));
    assert!(!look.on(&h.places, "deploy-check", Agent::Agy));
    assert!(!look.on(&h.places, "deploy-check", Agent::Codex));
    assert!(look.on(&h.places, "trace-reader", Agent::Agy));
    // A copy of each was kept first.
    let kept: Vec<String> = std::fs::read_dir(h.app.path().join("backups/skills"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(kept.iter().any(|n| n.starts_with("deploy-check.folder.")));
    assert!(!h.mark.exists(), "gathering ran nothing");

    let n = skills::unimport(&h.places, &h.app, &g.moved, at(60)).unwrap();
    assert_eq!(n, 2);
    assert!(h.home.join(".claude/skills/deploy-check/SKILL.md").is_file());
    assert!(!std::fs::symlink_metadata(h.home.join(".claude/skills/deploy-check"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!shared.join("deploy-check").exists());
    assert_eq!(std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap(), CODEX);
}

// ===== npx skills =====

/// A fake `npx` that records its arguments and does what `npx skills`
/// does to the folders and the lock file.
fn fake_npx(h: &Home) -> Npx {
    let dir = h.root.join("npx");
    let body = r#"d='@DIR@'; home='@HOME@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
pwd > "$d/cwd"
shift 2
cmd="$1"; shift
case "$cmd" in
  add)
    repo="$1"; shift
    if [ "$1" = "--list" ]; then
      [ "$repo" = "acme/missing" ] && { echo "Repository not found" >&2; exit 1; }
      printf '\033[?25l│\n◇  Available Skills\n│\n│    changelog-writer\n│\n│      Writes a changelog.\n│\n│    pdf-tools\n│\n│      Reads PDF files.\n│\n└  Use --skill <name> to install specific skills\n'
      exit 0
    fi
    names=""
    while [ $# -gt 0 ]; do
      case "$1" in --skill) names="$names $2"; shift 2 ;; *) shift ;; esac
    done
    for n in $names; do
      mkdir -p "$home/.agents/skills/$n"
      printf -- '---\nname: %s\n---\n' "$n" > "$home/.agents/skills/$n/SKILL.md"
      printf 'touch "%s"\n' '@MARK@' > "$home/.agents/skills/$n/run.sh"; chmod +x "$home/.agents/skills/$n/run.sh"
      mkdir -p "$home/.claude/skills"
      ln -s "../../.agents/skills/$n" "$home/.claude/skills/$n"
      printf '%s\n' "$n" >> "$d/installed"
    done ;;
  use) printf 'Use the following SKILL.md:\n<SKILL.md>\n---\nname: x\n---\n# Changelog\n</SKILL.md>\n' ;;
  update)
    printf 'More.\n' >> "$home/.agents/skills/$1/SKILL.md"
    # The tool links agents of its own choosing.
    rm -f "$home/.claude/skills/$1"
    ln -s "../../../.agents/skills/$1" "$home/.gemini/config/skills/$1" ;;
  remove)
    for n in "$@"; do
      case "$n" in -*) continue ;; esac
      rm -rf "$home/.agents/skills/$n"; rm -f "$home/.claude/skills/$n"
    done ;;
esac
"#
    .replace("@DIR@", &dir.display().to_string())
    .replace("@HOME@", &h.home.display().to_string())
    .replace("@MARK@", &h.mark.display().to_string());
    let program = script(&dir.join("bin/npx"), &body);
    Npx::new(program, npx::run_dir(h.app.path()))
}

fn calls(h: &Home) -> Vec<Vec<String>> {
    std::fs::read_to_string(h.root.join("npx/calls"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.split('\u{1f}').filter(|a| !a.is_empty()).map(str::to_string).collect())
        .collect()
}

#[test]
fn a_repository_s_skills_are_listed_and_read_through_npx() {
    let h = setup();
    let n = fake_npx(&h);
    let listed = n.list("acme/agent-skills").unwrap();
    assert_eq!(
        listed.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
        ["changelog-writer", "pdf-tools"]
    );
    assert_eq!(listed[0].description, "Writes a changelog.");
    assert!(
        matches!(n.list("acme/missing"), Err(npx::NpxError::Failed(m)) if m.contains("not found"))
    );
    assert_eq!(n.list("not a repo"), Err(npx::NpxError::BadRepo));
    assert!(n.read("acme/agent-skills", "changelog-writer").unwrap().contains("# Changelog"));
    assert_eq!(calls(&h)[0], ["-y", "skills", "add", "acme/agent-skills", "--list"]);
    assert_eq!(calls(&h)[2], ["-y", "skills", "use", "acme/agent-skills@changelog-writer"]);
    // In the app's own empty folder, never a project's.
    let cwd = std::fs::read_to_string(h.root.join("npx/cwd")).unwrap();
    assert!(cwd.trim().ends_with("app/agents/npx"), "{cwd}");
}

#[test]
fn installing_sets_each_agent_as_asked_and_runs_nothing_of_the_skill() {
    let h = setup();
    let n = fake_npx(&h);
    let want = BTreeMap::from([(Agent::Claude, false), (Agent::Codex, false), (Agent::Agy, true)]);
    let out = skill_ops::install(
        &h.places,
        &h.app,
        &n,
        "acme/agent-skills",
        &["changelog-writer".into(), "pdf-tools".into()],
        &want,
        at(0),
    );
    assert_eq!(out.names, ["changelog-writer"], "pdf-tools is installed already");
    assert!(
        out.error.is_none() && out.outcome.failed.is_none(),
        "{:?} {:?}",
        out.error,
        out.outcome.failed
    );
    assert_eq!(
        calls(&h)[0],
        [
            "-y",
            "skills",
            "add",
            "acme/agent-skills",
            "--skill",
            "changelog-writer",
            "-g",
            "-y",
            "-a",
            "claude-code"
        ]
    );
    let look = skills::read(&h.places);
    assert!(
        !look.on(&h.places, "changelog-writer", Agent::Claude),
        "the tool's Claude Code link went"
    );
    assert!(look.on(&h.places, "changelog-writer", Agent::Agy));
    assert!(!look.on(&h.places, "changelog-writer", Agent::Codex));
    assert!(!h.mark.exists(), "installing ran nothing of the skill");
}

#[test]
fn updating_keeps_a_copy_and_every_agent_s_box() {
    let h = setup();
    let n = fake_npx(&h);
    let before = std::fs::read_to_string(h.home.join(".agents/skills/ui-polish/SKILL.md")).unwrap();
    let out = skill_ops::update(&h.places, &h.app, &n, "ui-polish", at(0));
    assert!(out.error.is_none(), "{:?}", out.error);
    assert_eq!(calls(&h)[0], ["-y", "skills", "update", "ui-polish", "-g", "-y"]);
    let id = out.backup.unwrap();
    assert_eq!(
        std::fs::read_to_string(h.app.path().join("backups/skills").join(id).join("SKILL.md"))
            .unwrap(),
        before
    );
    let look = skills::read(&h.places);
    assert!(look.on(&h.places, "ui-polish", Agent::Claude), "Claude Code's link is back");
    assert!(!look.on(&h.places, "ui-polish", Agent::Agy), "the tool's Antigravity link went");
    assert!(!h.mark.exists());
}

#[test]
fn removing_for_good_keeps_a_copy_and_clears_the_app_s_own_entries() {
    let h = setup();
    let n = fake_npx(&h);
    let off = skills::preview(&h.places, &[change("pdf-tools", Agent::Codex, false)]);
    skills::write(&h.places, &h.app, &off.changes, &off.bases, at(0));
    let out = skill_ops::remove(&h.places, &h.app, &n, &["pdf-tools".into()], at(60));
    assert!(out.error.is_none(), "{:?}", out.error);
    assert_eq!(out.names, ["pdf-tools"]);
    assert_eq!(calls(&h)[0], ["-y", "skills", "remove", "pdf-tools", "-g", "-y"]);
    assert!(h.app.path().join("backups/skills").join(&out.backups[0]).join("SKILL.md").is_file());
    assert!(
        std::fs::symlink_metadata(h.home.join(".gemini/config/skills/pdf-tools")).is_err(),
        "the link left to nothing went"
    );
    assert_eq!(
        std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap(),
        CODEX,
        "the app's entry went"
    );
    // A skill installed by hand is never removed by the tool.
    let hand = skill_ops::remove(&h.places, &h.app, &n, &["find-notes".into()], at(120));
    assert!(hand.error.is_some() && hand.names.is_empty());
    assert!(h.home.join(".agents/skills/find-notes").is_dir());
}

// ===== Newer versions on a fake GitHub =====

struct FakeGitHub {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
}

fn fake_github(answers: BTreeMap<String, (u16, String)>) -> FakeGitHub {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                head.push_str(&line);
                line.clear();
            }
            let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
            seen.lock().unwrap().push(head.clone());
            let (status, body) =
                answers.get(&path).cloned().unwrap_or((404, "{\"message\":\"Not Found\"}".into()));
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let mut rest = Vec::new();
            let _ = reader.read_to_end(&mut rest);
        }
    });
    FakeGitHub { base, requests }
}

#[test]
fn newer_versions_are_found_with_one_question_per_repository() {
    let h = setup();
    let shared = h.home.join(".agents/skills");
    let lock = skills::read_lock(&h.places.lock).unwrap();
    let local = |n: &str| skill_updates::file_hashes(&shared.join(n)).unwrap();
    let tree_hash = |n: &str| skill_updates::tree_hash(&shared.join(n)).unwrap();
    let mut entries: Vec<String> = Vec::new();
    for n in ["pdf-tools", "release-notes"] {
        entries.push(format!(r#"{{"path":"skills/{n}","type":"tree","sha":"{}"}}"#, tree_hash(n)));
        for (p, sha) in local(n) {
            entries.push(format!(r#"{{"path":"skills/{n}/{p}","type":"blob","sha":"{sha}"}}"#));
        }
    }
    // ui-polish: a new SKILL.md and one more file.
    entries.push(r#"{"path":"skills/ui-polish","type":"tree","sha":"1111111111111111111111111111111111111111"}"#.into());
    for (p, sha) in local("ui-polish") {
        let sha = if p == "SKILL.md" {
            "2222222222222222222222222222222222222222".to_string()
        } else {
            sha
        };
        entries.push(format!(r#"{{"path":"skills/ui-polish/{p}","type":"blob","sha":"{sha}"}}"#));
    }
    entries.push(r#"{"path":"skills/ui-polish/examples.md","type":"blob","sha":"3333333333333333333333333333333333333333"}"#.into());
    let acme = format!(r#"{{"sha":"aaaa","truncated":false,"tree":[{}]}}"#, entries.join(","));
    let dataworks = r#"{"sha":"bbbb","tree":[{"path":"sql-helper","type":"tree","sha":"4444444444444444444444444444444444444444"},{"path":"sql-helper/SKILL.md","type":"blob","sha":"5555555555555555555555555555555555555555"}]}"#;
    let gh = fake_github(BTreeMap::from([
        ("/repos/acme/agent-skills/git/trees/HEAD?recursive=1".to_string(), (200, acme)),
        (
            "/repos/dataworks/skills/git/trees/HEAD?recursive=1".to_string(),
            (200, dataworks.to_string()),
        ),
    ]));
    let before_hash = tree_hash("ui-polish");
    let u =
        skill_updates::check(&GitHub::fake(&gh.base), &lock, &shared, &Updates::default(), 1000);
    let asked = gh.requests.lock().unwrap().clone();
    assert_eq!(asked.len(), 2, "one question per repository");
    assert!(
        asked.iter().all(|r| !r.to_ascii_lowercase().contains("authorization")),
        "no token sent"
    );
    assert!(u.error.is_none() && u.failed.is_empty(), "{:?} {:?}", u.error, u.failed);
    assert_eq!(u.found.keys().collect::<Vec<_>>(), ["sql-helper", "ui-polish"]);
    let ui = &u.found["ui-polish"];
    assert_eq!(skill_updates::plain(&ui.changes), "Sửa hướng dẫn, thêm 1 file");
    assert_eq!(ui.seen_at, 1000);
    assert_eq!(tree_hash("ui-polish"), before_hash, "the app never updates on its own");
    assert!(!h.mark.exists());

    // Shown on the row, until skipped.
    let look = skills::read(&h.places);
    let view = skills::view(&h.places, &look, Vec::new(), &u);
    let row = view.rows.iter().find(|r| r.name == "ui-polish").unwrap();
    let upd = row.update.as_ref().unwrap();
    assert_eq!(upd.link, "https://github.com/acme/agent-skills/commits/HEAD/skills/ui-polish");
    let mut skipped = u.clone();
    skipped.skipped.insert("ui-polish".into(), ui.to.clone());
    assert!(skipped.pending("ui-polish", &lock["ui-polish"]).is_none());
    // A second check keeps when each version was first seen.
    let again = skill_updates::check(&GitHub::fake(&gh.base), &lock, &shared, &u, 5000);
    assert_eq!(again.found["ui-polish"].seen_at, 1000);
}

#[test]
fn a_repository_github_does_not_answer_for_keeps_what_was_found() {
    let h = setup();
    let lock = skills::read_lock(&h.places.lock).unwrap();
    let gh = fake_github(BTreeMap::new());
    let mut before = Updates::default();
    before.found.insert(
        "sql-helper".into(),
        skill_updates::Found {
            repo: "dataworks/skills".into(),
            from: "x".into(),
            to: "y".into(),
            seen_at: 7,
            changes: Default::default(),
        },
    );
    let u = skill_updates::check(
        &GitHub::fake(&gh.base),
        &lock,
        &h.home.join(".agents/skills"),
        &before,
        100,
    );
    assert_eq!(u.failed, ["acme/agent-skills", "dataworks/skills"]);
    assert_eq!(u.found["sql-helper"].seen_at, 7);
}

#[test]
fn the_alert_for_new_versions_comes_once_and_opens_the_skills_tab() {
    let mut m = Memory::default();
    let found = vec![
        ("ui-polish".to_string(), "1111".to_string()),
        ("sql-helper".to_string(), "4444".to_string()),
    ];
    let id = alerts::skill_alert(&mut m, &found, 100).unwrap();
    assert!(
        alerts::skill_alert(&mut m, &found, 200).is_none(),
        "the same versions: no second alert"
    );
    let r = m.record(&id).unwrap();
    assert_eq!(r.alert.title, "2 skill có bản mới");
    assert_eq!(r.alert.body, "sql-helper, ui-polish. App không tự cập nhật.");
    let e = alerts::answer(&mut m, &id, "skills:open", 300, UtcOffset::UTC).unwrap();
    assert_eq!(e, Effect::OpenSkills);
    assert!(alerts::skill_alert(&mut m, &[("ui-polish".into(), "9999".into())], 400).is_some());
}

// ===== Plugins =====

#[test]
fn plugins_are_read_from_the_agents_own_commands_and_never_changed() {
    let h = setup();
    let cache = h.home.join(".claude/plugins/cache/acme-plugins/issue-hub/1.0.0");
    std::fs::create_dir_all(cache.join("commands")).unwrap();
    std::fs::write(cache.join("commands/triage.md"), "x").unwrap();
    std::fs::write(
        cache.join(".mcp.json"),
        r#"{"mcpServers":{"issues":{"command":"issues-mcp"}}}"#,
    )
    .unwrap();
    skill(&cache.join("skills/issue-notes"), "issue-notes", &h.mark);
    let claude_list = format!(
        r#"[{{"id":"issue-hub@acme-plugins","version":"1.0.0","scope":"user","enabled":true,"installPath":"{}"}},
{{"id":"review-kit@acme-plugins","scope":"user","enabled":false,"installPath":"/elsewhere"}},
{{"id":"local-thing@acme-plugins","scope":"project","enabled":true,"projectPath":"/work/app"}}]"#,
        cache.display()
    );
    let markets = r#"[{"name":"acme-plugins","source":"github","repo":"acme/claude-plugins"},{"name":"tools","source":"git","url":"https://user:trap-git-pw@git.example.com/tools.git"}]"#;
    let dir = h.root.join("cli");
    let claude = script(
        &dir.join("claude"),
        &format!(
            "printf '%s ' \"$@\" >> '{d}/claude-calls'; echo >> '{d}/claude-calls'\ncase \"$*\" in\n  'plugin list --json') cat <<'EOF'\n{claude_list}\nEOF\n;;\n  'plugin marketplace list --json') cat <<'EOF'\n{markets}\nEOF\n;;\n  *) exit 3 ;;\nesac\n",
            d = dir.display()
        ),
    );
    let codex_list = r#"{"installed":[{"pluginId":"sheets@codex-market","name":"sheets","marketplaceName":"codex-market","installed":true,"enabled":true,"source":{"source":"git"},"marketplaceSource":{"sourceType":"git","source":"https://github.com/tools-co/codex-market.git"}},
{"pluginId":"browser@openai-bundled","name":"browser","marketplaceName":"openai-bundled","installed":true,"enabled":true,"source":{"source":"local","path":"/Applications/x"},"marketplaceSource":{"sourceType":"local","source":"/x"}}],"available":[]}"#;
    let codex = script(
        &dir.join("codex"),
        &format!(
            "printf '%s ' \"$@\" >> '{d}/codex-calls'; echo >> '{d}/codex-calls'\ncat <<'EOF'\n{codex_list}\nEOF\n",
            d = dir.display()
        ),
    );
    let c = plugins::read_claude(Some(&claude), &h.home);
    assert_eq!(c.state, "ok");
    assert_eq!(
        c.plugins.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        ["issue-hub@acme-plugins", "review-kit@acme-plugins"]
    );
    let b = &c.plugins[0].brings;
    assert_eq!(
        (b.mcp, b.skills.as_slice(), b.commands),
        (1, ["issue-notes".to_string()].as_slice(), 1)
    );
    assert_eq!(
        c.marketplaces[1].source.as_deref(),
        Some("https://git.example.com/tools.git"),
        "no password kept"
    );
    let x = plugins::read_codex(Some(&codex), &h.home);
    assert_eq!(x.plugins.len(), 2);
    assert_eq!(
        plugins::reinstall(&x),
        [
            "codex plugin marketplace add tools-co/codex-market",
            "codex plugin add sheets@codex-market"
        ]
    );
    assert_eq!(
        plugins::reinstall(&c),
        [
            "claude plugin marketplace add acme/claude-plugins",
            "claude plugin install issue-hub@acme-plugins",
            "claude plugin install review-kit@acme-plugins",
            "claude plugin disable review-kit@acme-plugins",
        ]
    );
    // Only listed: nothing installed, removed, turned on or off.
    let ran = std::fs::read_to_string(dir.join("claude-calls")).unwrap()
        + &std::fs::read_to_string(dir.join("codex-calls")).unwrap();
    for word in ["install", "uninstall", "remove", "enable", "disable", "add "] {
        assert!(!ran.contains(word), "ran {word}: {ran}");
    }
    let owned = plugins::owned_skills(&[c.clone(), x]);
    assert_eq!(owned[0].names, ["issue-notes"]);
    // No `claude`: said so, nothing guessed.
    assert_eq!(plugins::read_claude(None, &h.home).state, "noCli");
    assert!(!h.mark.exists());
}

// ===== Instruction files =====

const KEY_TRAP: &str = "trap-deploy-value-9876";

#[test]
fn an_instruction_file_with_a_key_never_reaches_a_pack_or_the_windows() {
    let h = setup();
    let claude = h.home.join(".claude");
    std::fs::create_dir_all(claude.join("rules")).unwrap();
    std::fs::write(claude.join("CLAUDE.md"), "# Notes\nAnswer briefly.\n").unwrap();
    std::fs::write(claude.join("rules/style.md"), "- Short sentences.\n").unwrap();
    std::fs::write(
        claude.join("rules/deploy.md"),
        format!("# Deploy\n\nDEPLOY_TOKEN={KEY_TRAP}\n"),
    )
    .unwrap();
    // A file holding an MCP server's key from Codex's config.
    std::fs::write(h.home.join(".codex/AGENTS.md"), format!("Files token is {MCP_TRAP}.\n"))
        .unwrap();
    std::fs::create_dir_all(h.home.join(".gemini")).unwrap();
    std::fs::write(h.home.join(".gemini/GEMINI.md"), "Gemini notes.\n").unwrap();
    let before: Vec<(PathBuf, Vec<u8>)> = ["CLAUDE.md", "rules/style.md", "rules/deploy.md"]
        .iter()
        .map(|f| (claude.join(f), std::fs::read(claude.join(f)).unwrap()))
        .collect();

    let mcp_places = agents::Places::new(&h.home, &h.home.join(".codex"), None);
    let readings = agents::read_all(&mcp_places);
    let known = pack::secret_values(&readings);
    let mut choices = Choices::default();
    let read = instructions::read_all(&h.home, &h.home.join(".codex"), &choices, &known);
    let shown: Vec<&str> = read.iter().map(|r| r.shown.as_str()).collect();
    assert_eq!(
        shown,
        [
            "~/.claude/CLAUDE.md",
            "~/.claude/rules/deploy.md",
            "~/.claude/rules/style.md",
            "~/.codex/AGENTS.md",
            "~/.gemini/GEMINI.md"
        ]
    );
    let deploy = read.iter().find(|r| r.shown.ends_with("deploy.md")).unwrap();
    assert_eq!(deploy.keys, [3]);
    assert_eq!(read.iter().find(|r| r.shown.ends_with("AGENTS.md")).unwrap().keys, [1]);
    let packed = instructions::packed(&read, &choices);
    assert_eq!(
        packed.iter().map(|p| p.path.as_str()).collect::<Vec<_>>(),
        ["~/.claude/CLAUDE.md", "~/.claude/rules/style.md", "~/.gemini/GEMINI.md"]
    );

    let s = instructions::show(deploy, &known, &choices.not_keys).unwrap();
    assert_eq!(s.lines[2].text, "DEPLOY_TOKEN=••••••");
    assert!(s.lines[2].key);
    let view = instructions::view(&read, &choices, None);
    let json = serde_json::to_string(&(&view, &s)).unwrap();
    assert!(!json.contains(KEY_TRAP) && !json.contains(MCP_TRAP));

    // A pack with them: no key anywhere in its files.
    let store = agents::kho::load(&h.app);
    let mut p = pack::content(&h.home, &readings, &store, "Studio mini", "2.0.0", at(0));
    let look = skills::read(&h.places);
    let (list, folders) = skills::packed(&h.places, &look, &known);
    p.extend(Some(list), None, Some(pack::InstructionList { files: packed.clone() }));
    p.folders = folders;
    let root = h.root.join("packs");
    let id = pack::write(&root, &h.home, &p, &readings).unwrap();
    let all = all_text(&root.join(&id));
    assert!(!all.contains(KEY_TRAP) && !all.contains(MCP_TRAP) && !all.contains(PROVIDER_TRAP));
    let back = pack::load(&root, &id).unwrap();
    assert_eq!(back.manifest.sections, ["mcp", "skills", "instructions"]);
    assert_eq!(back.instructions.unwrap().files.len(), 3);
    let skills_in = back.skills.unwrap();
    let pdf = skills_in.skills.iter().find(|s| s.name == "pdf-tools").unwrap();
    assert_eq!((pdf.repo.as_deref(), pdf.hand), (Some("acme/agent-skills"), false));
    assert_eq!(pdf.on, [Agent::Claude, Agent::Codex, Agent::Agy]);
    assert!(skills_in.skills.iter().find(|s| s.name == "deploy-check").unwrap().hand);
    assert!(pack::same_as_newest(&root, "Studio mini", &p));
    let summary = pack::summary(&id, &pack::load(&root, &id).unwrap());
    assert!(summary.skills.contains(&"pdf-tools".to_string()));

    // Compared with the pack after an edit: line by line.
    std::fs::write(claude.join("rules/style.md"), "- Short sentences.\n- Vietnamese.\n").unwrap();
    let read = instructions::read_all(&h.home, &h.home.join(".codex"), &choices, &known);
    let style = read.iter().find(|r| r.shown.ends_with("style.md")).unwrap();
    let lines = instructions::compare(
        style,
        packed.iter().find(|p| p.path.ends_with("style.md")).unwrap(),
        &known,
    )
    .unwrap();
    assert!(lines.iter().any(|l| l.op == "add" && l.text == "- Vietnamese."));
    let view = instructions::view(&read, &choices, Some((0, &packed)));
    assert_eq!(
        view.files.iter().find(|f| f.path.ends_with("style.md")).unwrap().vs_pack,
        "changed"
    );

    // "Không phải khoá": that line only, then the file may go in.
    assert!(instructions::mark_not_key(&mut choices, deploy, 3));
    let read = instructions::read_all(&h.home, &h.home.join(".codex"), &choices, &known);
    assert!(read.iter().find(|r| r.shown.ends_with("deploy.md")).unwrap().keys.is_empty());
    assert!(
        !serde_json::to_string(&choices).unwrap().contains(KEY_TRAP),
        "only a hash of the line is kept"
    );
    // "Trong gói" off keeps a file out.
    choices.excluded.push("~/.gemini/GEMINI.md".into());
    assert!(!instructions::packed(&read, &choices).iter().any(|p| p.path.ends_with("GEMINI.md")));

    // The app never wrote one of them.
    for (f, bytes) in &before {
        if !f.ends_with("style.md") {
            assert_eq!(&std::fs::read(f).unwrap(), bytes);
        }
    }
}

#[test]
fn files_added_by_hand_stay_in_the_home_folder() {
    let h = setup();
    std::fs::create_dir_all(h.home.join("notes")).unwrap();
    std::fs::write(h.home.join("notes/team.md"), "x").unwrap();
    std::fs::write(h.home.join("notes/run.sh"), "x").unwrap();
    assert_eq!(
        instructions::check_extra(&h.home, h.app.path(), "~/notes/team.md"),
        Ok("~/notes/team.md".to_string())
    );
    assert_eq!(instructions::check_extra(&h.home, h.app.path(), "~/notes/run.sh"), Err("type"));
    assert_eq!(instructions::check_extra(&h.home, h.app.path(), "/etc/hosts.md"), Err("outside"));
    assert_eq!(
        instructions::check_extra(&h.home, h.app.path(), "~/notes/../../x.md"),
        Err("outside")
    );
    let choices = Choices { extra: vec!["~/notes/team.md".into()], ..Choices::default() };
    let files = instructions::files(&h.home, &h.home.join(".codex"), &choices);
    assert!(files.iter().any(|(p, a)| p.ends_with("notes/team.md") && a.is_none()));
}

#[test]
fn links_that_are_not_to_the_shared_folder_are_skills_of_their_own() {
    let h = setup();
    let elsewhere = h.root.join("elsewhere/linter");
    skill(&elsewhere, "linter", &h.mark);
    link(&h.home.join(".codex/skills/linter"), &elsewhere.display().to_string());
    let look = skills::read(&h.places);
    assert!(matches!(
        look.reading(Agent::Codex).entries.get("linter"),
        Some(Entry::Own { link: Some(_) })
    ));
    let view = skills::view(&h.places, &look, Vec::new(), &Updates::default());
    assert_eq!(
        cell(&view, "linter", Agent::Codex),
        ("on", None),
        "Codex reads its own folder: its box works"
    );
    // Gathered: copied in, the link replaced; the folder elsewhere stays.
    let g = skills::import(&h.places, &h.app, &["linter".into()], at(0));
    assert!(g.failed.is_none(), "{:?}", g.failed);
    assert!(h.home.join(".agents/skills/linter/SKILL.md").is_file());
    assert!(elsewhere.join("SKILL.md").is_file());
    assert!(std::fs::symlink_metadata(h.home.join(".codex/skills/linter")).is_err());
    assert!(!h.mark.exists());
}
