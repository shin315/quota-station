//! Restoring a pack's skills, plugins and instruction files (M12) between
//! two temporary homes: the Mac the pack comes from, with a shared folder
//! of skills, skills installed by hand, plugins listed by a fake `claude`
//! and `codex`, and instruction files; and a Mac set up again. A fake `npx`
//! and the fake `claude` and `codex` record every argument they get and
//! keep their state in files. The packs go to a temporary folder standing
//! in for iCloud Drive. Every name, repository and key is made up; no test
//! reads or writes the real agents' files, skills, the Keychain or iCloud
//! Drive, or reaches the network.
//!
//! Each skill holds a `run.sh` that leaves a mark when it runs: nothing the
//! app does runs it. The keys are traps: never in a pack, the plan, the
//! result, the restore's record or the app's backups.

mod common;

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use common::{script, temp_dir};
use quota_station_core::agents::instructions::{self, Choices};
use quota_station_core::agents::npx::{self, Npx};
use quota_station_core::agents::pack::{self, InstructionList, Pack, PluginList};
use quota_station_core::agents::plugins;
use quota_station_core::agents::restore::{self, More, Plan, PluginRef, Tools, UndoError};
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::skill_updates;
use quota_station_core::agents::skills::{self, SkillPlaces};
use quota_station_core::agents::{self, kho, Agent, Places};
use quota_station_core::antigravity::fs::AppDir;
use time::OffsetDateTime;

const PROVIDER_TRAP: &str = "trap-provider-key-1234";
const MCP_TRAP: &str = "trap-mcp-token-5678";
const SKILL_TRAP: &str = "trap-skill-key-9012";
const HERE_TRAP: &str = "trap-deploy-here-3456";
const GIT_TRAP: &str = "trap-git-pw";
const TRAPS: [&str; 5] = [PROVIDER_TRAP, MCP_TRAP, SKILL_TRAP, HERE_TRAP, GIT_TRAP];

const OLD_CODEX: &str = r#"# Codex config for the restore tests. Every name, URL and key is made up.
model = "gpt-5.5"
model_provider = "custom"

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
name = "pdf-tools"
enabled = false
"#;

/// The Mac set up again: Codex with its provider and nothing else.
const FRESH_CODEX: &str = r#"model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key-1234"
"#;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000 + offset).unwrap()
}

struct Mac {
    root: PathBuf,
    home: PathBuf,
    app: AppDir,
    places: Places,
    skills: SkillPlaces,
    secrets: Secrets,
    claude: PathBuf,
    codex: PathBuf,
    npx: Npx,
    /// Where a skill's `run.sh` leaves its mark.
    mark: PathBuf,
}

/// A skill folder: `SKILL.md`, and a `run.sh` that marks `mark` when run.
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

/// A fake `npx skills`: `add` makes the skill in the shared folder, its
/// lock entry and Claude Code's link, `remove` takes them away. A
/// repository named in `fail` cannot be reached.
fn fake_npx(root: &Path, home: &Path, mark: &Path) -> PathBuf {
    let body = r#"d='@DIR@'; home='@HOME@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
shift 2
cmd="$1"; shift
lock() {
  mkdir -p "$d/lock"
  { printf '{"version": 3, "skills": {'; first=1
    for f in "$d"/lock/*; do [ -e "$f" ] || continue; [ $first = 1 ] || printf ','; first=0; cat "$f"; done
    printf '}}\n'; } > "$home/.agents/.skill-lock.json"
}
case "$cmd" in
  add)
    repo="$1"; shift
    if grep -qx "$repo" "$d/fail" 2>/dev/null; then
      echo "fatal: could not read Username for 'https://github.com'" >&2; exit 1
    fi
    names=""
    while [ $# -gt 0 ]; do
      case "$1" in --skill) names="$names $2"; shift 2 ;; *) shift ;; esac
    done
    for n in $names; do
      mkdir -p "$home/.agents/skills/$n"
      printf -- '---\nname: %s\n---\n# %s from GitHub\n' "$n" "$n" > "$home/.agents/skills/$n/SKILL.md"
      printf 'touch "%s"\n' '@MARK@' > "$home/.agents/skills/$n/run.sh"; chmod +x "$home/.agents/skills/$n/run.sh"
      mkdir -p "$home/.claude/skills" "$d/lock"
      ln -s "../../.agents/skills/$n" "$home/.claude/skills/$n"
      printf '"%s": {"source": "%s", "sourceType": "github", "skillPath": "skills/%s/SKILL.md"}' "$n" "$repo" "$n" > "$d/lock/$n"
    done
    lock ;;
  remove)
    for n in "$@"; do
      case "$n" in -*) continue ;; esac
      rm -rf "$home/.agents/skills/$n"; rm -f "$home/.claude/skills/$n" "$d/lock/$n"
    done
    lock ;;
esac
"#
    .replace("@DIR@", &root.join("npx").display().to_string())
    .replace("@HOME@", &home.display().to_string())
    .replace("@MARK@", &mark.display().to_string());
    script(&root.join("npx/bin/npx"), &body)
}

/// A fake `claude` for plugins: its plugins and marketplaces one file each.
/// `names` maps a marketplace's source to its name; an id in `fail` cannot
/// be installed without logging in.
fn fake_claude(dir: &Path) -> PathBuf {
    for d in ["installed", "markets"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    let body = r#"d='@DIR@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
list() { printf '['; first=1; for f in "$1"/*; do [ -e "$f" ] || continue; [ $first = 1 ] || printf ','; first=0; cat "$f"; done; printf ']\n'; }
case "$1 $2" in
  "plugin list") list "$d/installed" ;;
  "plugin marketplace")
    case "$3" in
      list) list "$d/markets" ;;
      add)
        n=$(awk -v s="$4" '$1==s {print $2}' "$d/names")
        [ -n "$n" ] || { echo "Invalid marketplace source format" >&2; exit 1; }
        case "$4" in
          *://*) printf '{"name":"%s","source":"git","url":"%s"}' "$n" "$4" > "$d/markets/$n" ;;
          *) printf '{"name":"%s","source":"github","repo":"%s"}' "$n" "$4" > "$d/markets/$n" ;;
        esac ;;
      remove) rm -f "$d/markets/$4"; for f in "$d"/installed/*@"$4"; do rm -f "$f"; done ;;
    esac ;;
  "plugin install")
    if grep -qx "$3" "$d/fail" 2>/dev/null; then echo "Error: Authentication required for git.example.com" >&2; exit 1; fi
    [ -e "$d/markets/${3#*@}" ] || { echo "Marketplace ${3#*@} not found" >&2; exit 1; }
    printf '{"id":"%s","scope":"user","enabled":true}' "$3" > "$d/installed/$3" ;;
  "plugin disable") [ -e "$d/installed/$3" ] || exit 1; printf '{"id":"%s","scope":"user","enabled":false}' "$3" > "$d/installed/$3" ;;
  "plugin uninstall") rm "$d/installed/$3" || exit 1 ;;
  *) exit 3 ;;
esac
"#
    .replace("@DIR@", &dir.display().to_string());
    script(&dir.join("bin/claude"), &body)
}

/// A fake `codex` for plugins, the same way.
fn fake_codex(dir: &Path) -> PathBuf {
    for d in ["installed", "markets"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    let body = r#"d='@DIR@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
case "$1 $2" in
  "plugin list")
    printf '{"installed":['; first=1
    for f in "$d"/installed/*; do [ -e "$f" ] || continue; [ $first = 1 ] || printf ','; first=0; cat "$f"; done
    printf '],"available":[]}\n' ;;
  "plugin marketplace")
    n=$(awk -v s="$4" '$1==s {print $2}' "$d/names")
    [ -n "$n" ] || { echo "invalid marketplace source format" >&2; exit 1; }
    printf 'https://github.com/%s.git' "$4" > "$d/markets/$n" ;;
  "plugin add")
    m="${3#*@}"
    [ -e "$d/markets/$m" ] || { echo "marketplace $m not found" >&2; exit 1; }
    printf '{"pluginId":"%s","marketplaceName":"%s","installed":true,"enabled":true,"source":{"source":"git"},"marketplaceSource":{"sourceType":"git","source":"%s"}}' "$3" "$m" "$(cat "$d/markets/$m")" > "$d/installed/$3" ;;
  "plugin remove") rm "$d/installed/$3" || exit 1 ;;
  *) exit 3 ;;
esac
"#
    .replace("@DIR@", &dir.display().to_string());
    script(&dir.join("bin/codex"), &body)
}

fn mac(tag: &str, codex: &str) -> Mac {
    let root = temp_dir(tag);
    let home = root.join("Users").join(tag);
    for d in [".codex", ".claude", ".gemini/config", ".agents/skills"] {
        std::fs::create_dir_all(home.join(d)).unwrap();
    }
    std::fs::write(home.join(".codex/config.toml"), codex).unwrap();
    let mark = root.join("ran");
    let claude = fake_claude(&root.join("claude"));
    let codex = fake_codex(&root.join("codex"));
    let npx = Npx::new(fake_npx(&root, &home, &mark), npx::run_dir(&root.join("app")));
    let keychain = root.join("keychain");
    Mac {
        places: Places::new(&home, &home.join(".codex"), None),
        skills: SkillPlaces::new(&home, &home.join(".codex"), Some(npx.program.clone())),
        secrets: Secrets::fake(common::google::fake_security(&keychain)),
        app: AppDir::new(root.join("app")),
        root,
        home,
        claude,
        codex,
        npx,
        mark,
    }
}

/// The Mac the pack comes from.
fn old_mac() -> Mac {
    let m = mac("old", OLD_CODEX);
    let shared = m.home.join(".agents/skills");
    // From GitHub, on for Claude Code and Antigravity, off for Codex.
    skill(&shared.join("pdf-tools"), "pdf-tools", &m.mark);
    std::fs::write(
        m.home.join(".agents/.skill-lock.json"),
        r#"{"version": 3, "skills": {"pdf-tools": {"source": "acme/agent-skills", "sourceType": "github", "skillPath": "skills/pdf-tools/SKILL.md"}}}"#,
    )
    .unwrap();
    link(&m.home.join(".claude/skills/pdf-tools"), "../../.agents/skills/pdf-tools");
    link(&m.home.join(".gemini/config/skills/pdf-tools"), "../../../.agents/skills/pdf-tools");
    // By hand, gathered into the shared folder: on for Claude Code.
    skill(&shared.join("notes-helper"), "notes-helper", &m.mark);
    std::fs::create_dir_all(shared.join("notes-helper/docs")).unwrap();
    std::fs::write(shared.join("notes-helper/docs/guide.md"), "Keep notes short.\n").unwrap();
    link(&m.home.join(".claude/skills/notes-helper"), "../../.agents/skills/notes-helper");
    // By hand in Claude Code's own folder.
    skill(&m.home.join(".claude/skills/deploy-check"), "deploy-check", &m.mark);
    // By hand, holding a key: stays out of the pack.
    skill(&m.home.join(".claude/skills/secret-skill"), "secret-skill", &m.mark);
    std::fs::write(
        m.home.join(".claude/skills/secret-skill/env.sh"),
        format!("export API_TOKEN={SKILL_TRAP}\n"),
    )
    .unwrap();
    // By hand, with a link inside: stays out too.
    skill(&m.home.join(".claude/skills/linky"), "linky", &m.mark);
    std::os::unix::fs::symlink("/etc/hosts", m.home.join(".claude/skills/linky/hosts")).unwrap();

    // Plugins, as the agents' own commands list them.
    let c = m.root.join("claude");
    let put = |dir: &Path, name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
    put(
        &c.join("installed"),
        "issue-hub@acme-plugins",
        r#"{"id":"issue-hub@acme-plugins","scope":"user","enabled":true}"#,
    );
    put(
        &c.join("installed"),
        "review-kit@acme-plugins",
        r#"{"id":"review-kit@acme-plugins","scope":"user","enabled":false}"#,
    );
    put(
        &c.join("installed"),
        "formatter@tools-market",
        r#"{"id":"formatter@tools-market","scope":"user","enabled":true}"#,
    );
    put(
        &c.join("markets"),
        "acme-plugins",
        r#"{"name":"acme-plugins","source":"github","repo":"acme/claude-plugins"}"#,
    );
    put(
        &c.join("markets"),
        "tools-market",
        &format!(
            r#"{{"name":"tools-market","source":"git","url":"https://someone:{GIT_TRAP}@git.example.com/tools.git"}}"#
        ),
    );
    let x = m.root.join("codex");
    let codex_plugin = |id: &str, on: bool| {
        format!(
            r#"{{"pluginId":"{id}","marketplaceName":"codex-market","installed":true,"enabled":{on},"source":{{"source":"git"}},"marketplaceSource":{{"sourceType":"git","source":"https://github.com/tools-co/codex-market.git"}}}}"#
        )
    };
    put(&x.join("installed"), "sheets@codex-market", &codex_plugin("sheets@codex-market", true));
    put(&x.join("installed"), "notes@codex-market", &codex_plugin("notes@codex-market", false));
    put(
        &x.join("installed"),
        "browser@openai-bundled",
        r#"{"pluginId":"browser@openai-bundled","marketplaceName":"openai-bundled","installed":true,"enabled":true,"source":{"source":"local","path":"/Applications/x"},"marketplaceSource":{"sourceType":"local","source":"/x"}}"#,
    );

    // Instruction files.
    std::fs::create_dir_all(m.home.join(".claude/rules")).unwrap();
    std::fs::write(
        m.home.join(".claude/CLAUDE.md"),
        "# Notes\nAnswer briefly.\nUse short words.\n",
    )
    .unwrap();
    std::fs::write(m.home.join(".claude/rules/style.md"), "- Short sentences.\n").unwrap();
    std::fs::write(m.home.join(".codex/AGENTS.md"), "Run the tests first.\n").unwrap();
    std::fs::write(m.home.join(".gemini/GEMINI.md"), "Gemini notes.\n").unwrap();
    m
}

/// The Mac set up again: no skill, no plugin, a CLAUDE.md of its own, a
/// GEMINI.md holding a key, the same style rules.
fn new_mac() -> Mac {
    let m = mac("new", FRESH_CODEX);
    std::fs::create_dir_all(m.home.join(".claude/rules")).unwrap();
    std::fs::write(m.home.join(".claude/CLAUDE.md"), "# Notes\nOld line.\n").unwrap();
    std::fs::write(m.home.join(".claude/rules/style.md"), "- Short sentences.\n").unwrap();
    std::fs::write(
        m.home.join(".gemini/GEMINI.md"),
        format!("Deploy with DEPLOY_TOKEN={HERE_TRAP}\n"),
    )
    .unwrap();
    std::fs::write(
        m.root.join("claude/names"),
        "acme/claude-plugins acme-plugins\nhttps://git.example.com/tools.git tools-market\n",
    )
    .unwrap();
    std::fs::write(m.root.join("codex/names"), "tools-co/codex-market codex-market\n").unwrap();
    m
}

fn tools(m: &Mac, root: &Path) -> Tools {
    Tools {
        skills: m.skills.clone(),
        npx: Some(m.npx.clone()),
        claude: Some(m.claude.clone()),
        codex: Some(m.codex.clone()),
        codex_home: m.home.join(".codex"),
        root: root.to_path_buf(),
    }
}

/// Packs everything of `m` into `root`, as "Sao lưu bây giờ" does.
fn back_up(m: &Mac, root: &Path) -> (String, Pack) {
    let readings = agents::read_all(&m.places);
    let known = pack::secret_values(&readings);
    let mut p =
        pack::content(&m.home, &readings, &kho::load(&m.app), "Studio mini", "2.0.0", at(0));
    let look = skills::read(&m.skills);
    let (list, folders) = skills::packed(&m.skills, &look, &known);
    let sides = vec![
        plugins::read_claude(Some(&m.claude), &m.home),
        plugins::read_codex(Some(&m.codex), &m.home),
    ];
    let choices = Choices::default();
    let read = instructions::read_all(&m.home, &m.home.join(".codex"), &choices, &known);
    let files = instructions::packed(&read, &choices);
    p.extend(Some(list), Some(PluginList { agents: sides }), Some(InstructionList { files }));
    p.folders = folders;
    let id = pack::write(root, &m.home, &p, &readings).unwrap();
    let back = pack::load(root, &id).unwrap();
    (id, back)
}

fn calls(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("calls"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.split('\u{1f}').filter(|a| !a.is_empty()).collect::<Vec<_>>().join(" "))
        .collect()
}

fn clear_calls(m: &Mac) {
    for d in ["claude", "codex", "npx"] {
        let _ = std::fs::remove_file(m.root.join(d).join("calls"));
    }
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
    for t in TRAPS {
        assert!(!text.contains(t), "{t} in {what}");
    }
}

/// What the preview ticks to begin with.
fn ticked(plan: &Plan) -> More {
    More {
        skills: plan
            .skills
            .iter()
            .flat_map(|s| &s.items)
            .filter(|i| i.checked)
            .map(|i| i.name.clone())
            .collect(),
        plugins: plan
            .plugins
            .iter()
            .flat_map(|s| &s.items)
            .filter(|i| i.checked)
            .map(|i| PluginRef { agent: i.agent, id: i.id.clone() })
            .collect(),
        files: plan
            .files
            .iter()
            .flat_map(|f| &f.items)
            .filter(|i| i.checked)
            .map(|i| i.path.clone())
            .collect(),
        file_bases: plan.files.as_ref().map(|f| f.bases.clone()).unwrap_or_default(),
    }
}

fn run(
    m: &Mac,
    root: &Path,
    id: &str,
    p: &Pack,
    plan: &Plan,
    more: &More,
    when: i64,
) -> restore::Done {
    restore::run(
        &m.places,
        &m.app,
        &m.secrets,
        &tools(m, root),
        id,
        p,
        &[],
        more,
        &[],
        &plan.bases,
        at(when),
    )
    .unwrap()
}

fn is_link_to_shared(m: &Mac, at: &Path, name: &str) -> bool {
    std::fs::symlink_metadata(at).is_ok_and(|x| x.file_type().is_symlink())
        && std::fs::canonicalize(at).ok()
            == std::fs::canonicalize(m.home.join(".agents/skills").join(name)).ok()
}

#[test]
fn a_pack_copies_skills_installed_by_hand_and_not_those_with_a_source() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let list = p.skills.as_ref().unwrap();
    let get = |n: &str| list.skills.iter().find(|s| s.name == n).unwrap_or_else(|| panic!("{n}"));

    let pdf = get("pdf-tools");
    assert!(!pdf.hand && !pdf.copied, "a skill from GitHub is installed from there again");
    assert!(!pack::skill_dir(&root, &id, "pdf-tools").exists());
    assert_eq!(pdf.on, [Agent::Claude, Agent::Agy]);

    for name in ["notes-helper", "deploy-check"] {
        let s = get(name);
        assert!(s.hand && s.copied, "{name}");
        let dir = pack::skill_dir(&root, &id, name);
        assert_eq!(skill_updates::tree_hash(&dir).ok(), s.hash, "{name} as it was");
        assert_eq!(s.exec, ["run.sh"]);
    }
    assert_eq!(
        std::fs::read_to_string(pack::skill_dir(&root, &id, "notes-helper").join("docs/guide.md"))
            .unwrap(),
        "Keep notes short.\n"
    );
    assert_eq!(get("secret-skill").left_out.as_deref(), Some("key"));
    assert_eq!(get("linky").left_out.as_deref(), Some("link"));
    assert!(!pack::skill_dir(&root, &id, "secret-skill").exists());
    assert!(!pack::skill_dir(&root, &id, "linky").exists());

    // No key anywhere in the pack, the Git password of a marketplace included.
    no_trap("the pack", &all_text(&root.join(&id)));
    let plugins = p.plugins.as_ref().unwrap();
    assert_eq!(plugins.agents.len(), 2);
    assert!(!old.mark.exists(), "nothing of a skill ran");

    // The same skills again: nothing new. A skill edited: a new pack.
    let readings = agents::read_all(&old.places);
    let mut again = p.clone();
    again.folders =
        skills::packed(&old.skills, &skills::read(&old.skills), &pack::secret_values(&readings)).1;
    assert!(pack::same_as_newest(&root, "Studio mini", &again));
    std::fs::write(old.home.join(".agents/skills/notes-helper/docs/guide.md"), "Longer notes.\n")
        .unwrap();
    let look = skills::read(&old.skills);
    let (list, folders) = skills::packed(&old.skills, &look, &pack::secret_values(&readings));
    again.extend(Some(list), again.plugins.clone(), again.instructions.clone());
    again.folders = folders;
    assert!(!pack::same_as_newest(&root, "Studio mini", &again));

    // A skill that gains a key between the check and the write stops it.
    let mut late = again.clone();
    std::fs::write(
        old.home.join(".agents/skills/notes-helper/docs/guide.md"),
        format!("TOKEN={SKILL_TRAP}\n"),
    )
    .unwrap();
    late.manifest.at += 60;
    assert_eq!(pack::write(&root, &old.home, &late, &readings), Err(pack::PackError::Secret));
}

#[test]
fn restore_installs_copies_and_sets_each_agent_without_running_a_skill() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let new = new_mac();
    let t = tools(&new, &root);

    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    no_trap("the plan", &serde_json::to_string(&plan).unwrap());
    let sp = plan.skills.as_ref().unwrap();
    let kinds: BTreeMap<&str, &str> = sp.items.iter().map(|i| (i.name.as_str(), i.kind)).collect();
    assert_eq!(
        kinds,
        BTreeMap::from([
            ("deploy-check", "hand"),
            ("notes-helper", "hand"),
            ("pdf-tools", "github")
        ])
    );
    let left: Vec<(&str, &str)> =
        sp.left_out.iter().map(|l| (l.name.as_str(), l.why.as_str())).collect();
    assert_eq!(left, [("linky", "link"), ("secret-skill", "key")]);
    assert!(sp.items.iter().all(|i| i.checked && i.blocked.is_none()));
    assert!(!new.home.join(".agents/skills/notes-helper").exists(), "the plan writes nothing");

    clear_calls(&new);
    let more = More { plugins: Vec::new(), files: Vec::new(), ..ticked(&plan) };
    let done = run(&new, &root, &id, &p, &plan, &more, 60);
    assert!(done.parts.iter().all(|r| r.ok), "{:?}", done.parts);
    assert_eq!(
        calls(&new.root.join("npx")),
        ["-y skills add acme/agent-skills --skill pdf-tools -g -y -a claude-code"],
        "one install from GitHub; nothing else asked of npx"
    );
    // Copied by hand from the pack, the same tree, the script still runnable.
    let shared = new.home.join(".agents/skills");
    for name in ["notes-helper", "deploy-check"] {
        let s = p.skills.as_ref().unwrap().skills.iter().find(|s| s.name == name).unwrap();
        assert_eq!(skill_updates::tree_hash(&shared.join(name)).ok(), s.hash, "{name}");
        let mode =
            std::fs::metadata(shared.join(name).join("run.sh")).unwrap().permissions().mode();
        assert_ne!(mode & 0o100, 0);
    }
    // Each agent's box as in the pack.
    let claude = new.home.join(".claude/skills");
    let agy = new.home.join(".gemini/config/skills");
    assert!(is_link_to_shared(&new, &claude.join("pdf-tools"), "pdf-tools"));
    assert!(is_link_to_shared(&new, &agy.join("pdf-tools"), "pdf-tools"));
    assert!(is_link_to_shared(&new, &claude.join("notes-helper"), "notes-helper"));
    assert!(!agy.join("notes-helper").exists());
    assert!(is_link_to_shared(&new, &claude.join("deploy-check"), "deploy-check"));
    let look = skills::read(&new.skills);
    assert!(!look.on(&new.skills, "pdf-tools", Agent::Codex), "off for Codex as in the pack");
    assert!(
        look.on(&new.skills, "notes-helper", Agent::Codex),
        "Codex read it from the shared folder"
    );
    assert!(
        !look.on(&new.skills, "deploy-check", Agent::Codex),
        "Codex did not see Claude Code's own"
    );
    let codex = std::fs::read_to_string(new.home.join(".codex/config.toml")).unwrap();
    assert!(codex.starts_with(FRESH_CODEX), "nothing but skills entries added:\n{codex}");
    assert!(!new.mark.exists(), "nothing of a skill ran");

    // The whole restore undone: the skills, links and Codex's entries go.
    let record = restore::last(&new.app).unwrap();
    no_trap("the record", &serde_json::to_string(&record).unwrap());
    assert_eq!(record.skills.len(), 3);
    clear_calls(&new);
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(90)).unwrap();
    assert_eq!((undone.skills, undone.left), (3, 0));
    assert_eq!(calls(&new.root.join("npx")), ["-y skills remove pdf-tools -g -y"]);
    for name in ["pdf-tools", "notes-helper", "deploy-check"] {
        assert!(std::fs::symlink_metadata(shared.join(name)).is_err(), "{name}");
        assert!(std::fs::symlink_metadata(claude.join(name)).is_err(), "{name}'s link");
    }
    assert!(std::fs::symlink_metadata(agy.join("pdf-tools")).is_err());
    assert_eq!(std::fs::read_to_string(new.home.join(".codex/config.toml")).unwrap(), FRESH_CODEX);
    no_trap("the app's folder", &all_text(new.app.path()));
    assert!(!new.mark.exists());
}

#[test]
fn plugin_commands_are_the_agents_own_and_one_failing_leaves_the_others() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let new = new_mac();
    let t = tools(&new, &root);
    // Claude Code here has the formatter's marketplace behind a login.
    std::fs::write(new.root.join("claude/fail"), "formatter@tools-market\n").unwrap();

    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    let pp = plan.plugins.as_ref().unwrap();
    type Line<'a> = (Agent, &'a str, Option<&'a str>, bool, Option<&'a str>);
    let items: Vec<Line> = pp
        .items
        .iter()
        .map(|i| (i.agent, i.id.as_str(), i.market.as_deref(), i.checked, i.note))
        .collect();
    assert_eq!(
        items,
        [
            (Agent::Claude, "formatter@tools-market", Some("tools-market"), true, None),
            (Agent::Claude, "issue-hub@acme-plugins", Some("acme-plugins"), true, None),
            (Agent::Claude, "review-kit@acme-plugins", Some("acme-plugins"), true, None),
            (Agent::Codex, "notes@codex-market", Some("codex-market"), false, Some("codexOff")),
            (Agent::Codex, "sheets@codex-market", Some("codex-market"), true, None),
        ],
        "Codex App's own plugin comes with it; an off Codex plugin is not ticked"
    );
    // Reading the plugins is all the plan ran.
    assert_eq!(
        calls(&new.root.join("claude")),
        ["plugin list --json", "plugin marketplace list --json"]
    );
    assert_eq!(calls(&new.root.join("codex")), ["plugin list --json"]);

    clear_calls(&new);
    let more = More { skills: Vec::new(), files: Vec::new(), ..ticked(&plan) };
    let done = run(&new, &root, &id, &p, &plan, &more, 60);
    let ran = calls(&new.root.join("claude"));
    let wrote: Vec<&String> = ran.iter().filter(|c| !c.ends_with("--json")).collect();
    assert_eq!(
        wrote,
        [
            "plugin marketplace add https://git.example.com/tools.git",
            "plugin install formatter@tools-market",
            "plugin marketplace add acme/claude-plugins",
            "plugin install issue-hub@acme-plugins",
            "plugin install review-kit@acme-plugins",
            "plugin disable review-kit@acme-plugins",
        ]
    );
    let ran = calls(&new.root.join("codex"));
    let wrote: Vec<&String> = ran.iter().filter(|c| !c.ends_with("--json")).collect();
    assert_eq!(
        wrote,
        ["plugin marketplace add tools-co/codex-market", "plugin add sheets@codex-market"]
    );
    // The commands the Plugin tab shows for the pack are the ones that ran.
    let shown: Vec<String> =
        p.plugins.as_ref().unwrap().agents.iter().flat_map(plugins::reinstall).collect();
    for c in calls(&new.root.join("claude")).iter().filter(|c| !c.ends_with("--json")) {
        assert!(shown.contains(&format!("claude {c}")), "claude {c} not in {shown:?}");
    }
    assert!(shown.contains(&"codex plugin add sheets@codex-market".to_string()));

    let result = |id: &str| done.parts.iter().find(|r| r.id == id).unwrap();
    let f = result("formatter@tools-market");
    assert_eq!((f.ok, f.why, f.retry), (false, Some("login"), true));
    for id in ["issue-hub@acme-plugins", "review-kit@acme-plugins", "sheets@codex-market"] {
        assert!(result(id).ok, "{id}");
    }
    assert!(done.can_undo);
    no_trap("the result", &serde_json::to_string(&done).unwrap());

    // "Thử lại" after logging in: only the formatter, kept in the same record.
    std::fs::remove_file(new.root.join("claude/fail")).unwrap();
    clear_calls(&new);
    let again = restore::retry(
        &new.places,
        &new.app,
        &t,
        &id,
        &p,
        "plugin",
        "formatter@tools-market",
        Some(Agent::Claude),
        at(80),
    );
    assert!(again.ok, "{again:?}");
    let ran = calls(&new.root.join("claude"));
    assert_eq!(
        ran.iter().filter(|c| !c.ends_with("--json")).collect::<Vec<_>>(),
        ["plugin install formatter@tools-market"],
        "its marketplace is here already"
    );
    let record = restore::last(&new.app).unwrap();
    assert_eq!(record.plugins.len(), 4);
    assert_eq!(record.markets, ["tools-market", "acme-plugins"]);

    // The undo takes them away with the same commands, and the
    // marketplaces the restore added.
    clear_calls(&new);
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(90)).unwrap();
    assert_eq!((undone.plugins, undone.left), (4, 0));
    let ran = calls(&new.root.join("claude"));
    let wrote: Vec<&String> = ran.iter().filter(|c| !c.ends_with("--json")).collect();
    assert_eq!(
        wrote,
        [
            "plugin uninstall issue-hub@acme-plugins",
            "plugin uninstall review-kit@acme-plugins",
            "plugin uninstall formatter@tools-market",
            "plugin marketplace remove tools-market",
            "plugin marketplace remove acme-plugins",
        ]
    );
    let ran = calls(&new.root.join("codex"));
    assert_eq!(ran, ["plugin remove sheets@codex-market"]);
    assert_eq!(std::fs::read_to_string(new.home.join(".codex/config.toml")).unwrap(), FRESH_CODEX);
}

#[test]
fn an_instruction_file_that_differs_keeps_the_mac_s_own_unless_ticked() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let new = new_mac();
    let t = tools(&new, &root);
    let claude_md = new.home.join(".claude/CLAUDE.md");

    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    let fp = plan.files.as_ref().unwrap();
    let items: Vec<(&str, &str, bool)> =
        fp.items.iter().map(|i| (i.path.as_str(), i.kind, i.checked)).collect();
    assert_eq!(
        items,
        [("~/.claude/CLAUDE.md", "replace", false), ("~/.codex/AGENTS.md", "add", true)]
    );
    let cmp = &fp.items[0];
    assert_eq!(cmp.changed, 2);
    assert!(cmp.diff.iter().any(|l| l.op == "del" && l.text == "Old line."));
    assert!(cmp.diff.iter().any(|l| l.op == "add" && l.text == "Answer briefly."));
    assert_eq!(fp.same, ["~/.claude/rules/style.md"]);
    let kept: Vec<(&str, &str)> = fp.kept.iter().map(|k| (k.path.as_str(), k.why)).collect();
    assert_eq!(kept, [("~/.gemini/GEMINI.md", "key")], "a file here with a key is never replaced");
    no_trap("the plan", &serde_json::to_string(&plan).unwrap());

    // As ticked to begin with: only the file this Mac has not got.
    let more = More { skills: Vec::new(), plugins: Vec::new(), ..ticked(&plan) };
    let done = run(&new, &root, &id, &p, &plan, &more, 60);
    assert!(done.parts.iter().all(|r| r.ok), "{:?}", done.parts);
    assert_eq!(
        std::fs::read_to_string(&claude_md).unwrap(),
        "# Notes\nOld line.\n",
        "the Mac's own stays"
    );
    assert_eq!(
        std::fs::read_to_string(new.home.join(".codex/AGENTS.md")).unwrap(),
        "Run the tests first.\n"
    );
    assert!(std::fs::read_to_string(new.home.join(".gemini/GEMINI.md"))
        .unwrap()
        .contains(HERE_TRAP));
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(70)).unwrap();
    assert_eq!(undone.files, 1);
    assert!(!new.home.join(".codex/AGENTS.md").exists(), "the file it added goes");

    // Ticked: replaced, the old one kept first, and put back by the undo.
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    let mut more = More { skills: Vec::new(), plugins: Vec::new(), ..ticked(&plan) };
    more.files.push("~/.claude/CLAUDE.md".into());
    let done = run(&new, &root, &id, &p, &plan, &more, 100);
    assert!(done.parts.iter().all(|r| r.ok), "{:?}", done.parts);
    assert_eq!(
        std::fs::read_to_string(&claude_md).unwrap(),
        "# Notes\nAnswer briefly.\nUse short words.\n"
    );
    let backups = new.app.path().join(restore::FILE_BACKUPS);
    assert!(all_text(&backups).contains("Old line."));

    // Edited after the restore: asked first; with a yes, left as edited.
    std::fs::write(new.home.join(".codex/AGENTS.md"), "Run the tests first.\nAnd lint.\n").unwrap();
    match restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(110)) {
        Err(UndoError::Changed(c)) => assert_eq!(c.files, ["~/.codex/AGENTS.md"]),
        other => panic!("{other:?}"),
    }
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, true, at(120)).unwrap();
    assert_eq!((undone.files, undone.left), (1, 1));
    assert_eq!(std::fs::read_to_string(&claude_md).unwrap(), "# Notes\nOld line.\n");
    assert_eq!(
        std::fs::read_to_string(new.home.join(".codex/AGENTS.md")).unwrap(),
        "Run the tests first.\nAnd lint.\n"
    );
    no_trap("the app's folder", &all_text(new.app.path()));
}

#[test]
fn one_part_failing_leaves_the_others_to_run_and_can_be_tried_again() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let new = new_mac();
    let t = tools(&new, &root);
    // GitHub cannot be reached, and iCloud Drive has not downloaded one of
    // the skills copied by hand.
    std::fs::write(new.root.join("npx/fail"), "acme/agent-skills\n").unwrap();
    let deploy = pack::skill_dir(&root, &id, "deploy-check");
    std::fs::remove_file(deploy.join("SKILL.md")).unwrap();
    std::fs::write(deploy.join(".SKILL.md.icloud"), "").unwrap();

    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    let blocked: Vec<(&str, Option<&str>, bool)> = plan
        .skills
        .as_ref()
        .unwrap()
        .items
        .iter()
        .map(|i| (i.name.as_str(), i.blocked, i.checked))
        .collect();
    assert_eq!(
        blocked,
        [
            ("deploy-check", Some("cloud"), false),
            ("notes-helper", None, true),
            ("pdf-tools", None, true)
        ]
    );
    let mut more = ticked(&plan);
    more.skills.push("deploy-check".into());
    let done = run(&new, &root, &id, &p, &plan, &more, 60);
    let result = |id: &str| done.parts.iter().find(|r| r.id == id).unwrap();
    assert_eq!(
        (result("pdf-tools").ok, result("pdf-tools").why, result("pdf-tools").retry),
        (false, Some("npx"), true)
    );
    assert_eq!((result("deploy-check").ok, result("deploy-check").why), (false, Some("cloud")));
    assert!(result("deploy-check").retry);
    // The rest ran.
    assert!(result("notes-helper").ok);
    assert!(result("issue-hub@acme-plugins").ok);
    assert!(result("~/.codex/AGENTS.md").ok);
    assert!(new.home.join(".agents/skills/notes-helper/SKILL.md").exists());

    // Both tried again once the cause is gone.
    std::fs::remove_file(new.root.join("npx/fail")).unwrap();
    std::fs::remove_file(deploy.join(".SKILL.md.icloud")).unwrap();
    std::fs::copy(old.home.join(".claude/skills/deploy-check/SKILL.md"), deploy.join("SKILL.md"))
        .unwrap();
    for name in ["pdf-tools", "deploy-check"] {
        let r = restore::retry(&new.places, &new.app, &t, &id, &p, "skill", name, None, at(80));
        assert!(r.ok, "{r:?}");
    }
    let claude = new.home.join(".claude/skills");
    assert!(is_link_to_shared(&new, &claude.join("deploy-check"), "deploy-check"));
    assert!(is_link_to_shared(
        &new,
        &new.home.join(".gemini/config/skills/pdf-tools"),
        "pdf-tools"
    ));

    // A skill edited after the restore is asked about, and stays with a yes.
    std::fs::write(new.home.join(".agents/skills/notes-helper/SKILL.md"), "# edited\n").unwrap();
    match restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(90)) {
        Err(UndoError::Changed(c)) => assert_eq!(c.skills, ["notes-helper"]),
        other => panic!("{other:?}"),
    }
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, true, at(100)).unwrap();
    assert_eq!((undone.skills, undone.plugins, undone.files, undone.left), (2, 4, 1, 1));
    assert!(new.home.join(".agents/skills/notes-helper/SKILL.md").exists());
    assert!(!new.home.join(".agents/skills/pdf-tools").exists());
    assert!(!new.mark.exists(), "nothing of a skill ran");
}

#[test]
fn the_restore_path_never_holds_a_key_value() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let new = new_mac();
    let t = tools(&new, &root);
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    let mut more = ticked(&plan);
    more.files.push("~/.claude/CLAUDE.md".into());
    // Even when asked to, a file holding a key here is not written.
    more.files.push("~/.gemini/GEMINI.md".into());
    let done = run(&new, &root, &id, &p, &plan, &more, 60);
    assert!(!done.parts.iter().any(|r| r.id.ends_with("GEMINI.md")));
    assert!(std::fs::read_to_string(new.home.join(".gemini/GEMINI.md"))
        .unwrap()
        .contains(HERE_TRAP));
    for (what, text) in [
        ("the plan", serde_json::to_string(&plan).unwrap()),
        ("the result", serde_json::to_string(&done).unwrap()),
        ("the record", serde_json::to_string(&restore::last(&new.app).unwrap()).unwrap()),
        ("the pack", all_text(&root)),
        ("the app's folder", all_text(new.app.path())),
        ("the shared skills", all_text(&new.home.join(".agents/skills"))),
        (
            "the commands run",
            all_text(&new.root.join("claude"))
                + &all_text(&new.root.join("codex"))
                + &all_text(&new.root.join("npx")),
        ),
    ] {
        // The Codex provider's key is in Codex's own file here, and only there.
        no_trap(what, &text);
    }
    let codex = std::fs::read_to_string(new.home.join(".codex/config.toml")).unwrap();
    assert!(codex.starts_with(FRESH_CODEX), "the provider and login stay:\n{codex}");
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(90)).unwrap();
    assert_eq!(undone.left, 0);
    no_trap("the app's folder after the undo", &all_text(new.app.path()));
}

#[test]
fn servers_and_skills_written_in_one_restore_are_undone_without_asking() {
    let old = old_mac();
    let root = temp_dir("packs");
    let (id, p) = back_up(&old, &root);
    let new = new_mac();
    let t = tools(&new, &root);
    let plan = restore::plan(&new.places, &new.app, &new.secrets, &t, &id, &p);
    let picks: Vec<restore::Pick> = plan
        .items
        .iter()
        .map(|i| restore::Pick { name: i.name.clone(), kind: i.kind.to_string() })
        .collect();
    assert_eq!(picks.len(), 1, "the files server, for Codex");
    let pasted = [restore::Pasted {
        mcp: "files".into(),
        slot: "env:FILES_TOKEN".into(),
        value: "pasted-files-token".into(),
    }];
    let more = More { plugins: Vec::new(), ..ticked(&plan) };
    let done = restore::run(
        &new.places,
        &new.app,
        &new.secrets,
        &t,
        &id,
        &p,
        &picks,
        &more,
        &pasted,
        &plan.bases,
        at(60),
    )
    .unwrap();
    assert_eq!(done.items[0].done, [Agent::Codex]);
    // Codex's file got the server, then the skills' entries.
    let codex = std::fs::read_to_string(new.home.join(".codex/config.toml")).unwrap();
    assert!(
        codex.contains("[mcp_servers.files]") && codex.contains("[[skills.config]]"),
        "{codex}"
    );
    // Undone in one go: the restore's own writes are not "changed since".
    let undone = restore::undo(&new.places, &new.app, &new.secrets, &t, false, at(90)).unwrap();
    assert!(undone.outcome.is_some_and(|o| o.failed.is_none()));
    assert_eq!(undone.left, 0);
    assert_eq!(std::fs::read_to_string(new.home.join(".codex/config.toml")).unwrap(), FRESH_CODEX);
}
