// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Copying the agents' setup to another Mac (M13) between two temporary
//! homes: "Studio mini" with servers, skills, plugins and instruction files,
//! and "Travel air" with fewer. The other Mac is reached through a fake
//! `ssh` first on a `PATH`, which records every argument and stdin, runs the
//! fixed script in that Mac's temporary home, and finds there a fake
//! "Quota Station.app" whose program records its words and stdin and replays
//! canned answers. Fake `npx`, `claude`, `codex` and `security` keep their
//! state in files. Every name, repository, host and key is made up; no test
//! opens a real SSH connection or touches the real agents' files, the
//! Keychain or the switcher.
//!
//! The keys are traps: never in what is read, never on a command line, and
//! on stdin only when "Gửi cả khoá qua SSH" is ticked.

#![cfg(feature = "author-tools")]
// Unix only: the fake agents and `npx` are shell scripts.
#![cfg(unix)]

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{script, temp_dir};
use quota_station_core::agents::copy::{
    self, ApplyError, CopyError, Here, Inventory, Job, JobInput, Remote, Selection, UndoAsk, Word,
};
use quota_station_core::agents::instructions::{self, Choices};
use quota_station_core::agents::npx::{self, Npx};
use quota_station_core::agents::pack::{self, InstructionList, PluginList};
use quota_station_core::agents::plugins;
use quota_station_core::agents::restore::{self, PluginRef, RunError, Tools};
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::skills::{self, SkillPlaces};
use quota_station_core::agents::{self, kho, Agent, Places};
use quota_station_core::app_dir::AppDir;
use quota_station_core::peers;
use time::OffsetDateTime;

const PROVIDER_TRAP: &str = "trap-provider-key-1234";
const MCP_TRAP: &str = "trap-mcp-token-5678";
const SKILL_TRAP: &str = "trap-skill-key-9012";
const TRAPS: [&str; 3] = [PROVIDER_TRAP, MCP_TRAP, SKILL_TRAP];

const MINI_CODEX: &str = r#"# Codex config for the copy tests. Every name, URL and key is made up.
model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "relay"
base_url = "https://relay.example.com/v1"
experimental_bearer_token = "trap-provider-key-1234"

[mcp_servers.files]
command = "/Users/owner/.nvm/versions/node/v22/bin/files-mcp"
args = ["--root", "~/Workspace"]

[mcp_servers.tracker]
command = "npx"
args = ["-y", "tracker-mcp"]

[mcp_servers.tracker.env]
TRACKER_TOKEN = "trap-mcp-token-5678"
"#;

const AIR_CODEX: &str = r#"model = "gpt-5.5"
model_provider = "openai"

[mcp_servers.files]
command = "@HOME@/.local/bin/files-mcp"
args = ["--root", "~/Code"]

[mcp_servers.notes]
command = "notes-mcp"
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
    name: &'static str,
}

fn skill(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n# {name}\n")).unwrap();
}

/// A fake `npx skills`: `add` makes the skill and its lock entry, `update`
/// changes its text, `remove` takes it away.
fn fake_npx(root: &Path, home: &Path) -> PathBuf {
    let body = r#"d='@DIR@'; home='@HOME@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
shift 2
cmd="$1"; shift
case "$cmd" in
  add)
    repo="$1"; shift
    while [ $# -gt 0 ]; do
      case "$1" in --skill) n="$2"; shift 2
        mkdir -p "$home/.agents/skills/$n"
        printf -- '---\nname: %s\n---\n# %s from GitHub\n' "$n" "$n" > "$home/.agents/skills/$n/SKILL.md" ;;
      *) shift ;; esac
    done ;;
  update) printf 'Newest.\n' >> "$home/.agents/skills/$1/SKILL.md" ;;
  remove) for n in "$@"; do case "$n" in -*) ;; *) rm -rf "$home/.agents/skills/$n" ;; esac; done ;;
esac
"#
    .replace("@DIR@", &root.join("npx").display().to_string())
    .replace("@HOME@", &home.display().to_string());
    script(&root.join("npx/bin/npx"), &body)
}

/// A fake `claude` or `codex` that lists the plugins in `installed/` and
/// installs by making a file there.
fn fake_agent(dir: &Path, codex: bool) -> PathBuf {
    for d in ["installed", "markets"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    let body = if codex {
        r#"d='@DIR@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
case "$1 $2" in
  "plugin list")
    printf '{"installed":['; first=1
    for f in "$d"/installed/*; do [ -e "$f" ] || continue; [ $first = 1 ] || printf ','; first=0; cat "$f"; done
    printf '],"available":[]}\n' ;;
  *) exit 3 ;;
esac
"#
    } else {
        r#"d='@DIR@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
list() { printf '['; first=1; for f in "$1"/*; do [ -e "$f" ] || continue; [ $first = 1 ] || printf ','; first=0; cat "$f"; done; printf ']\n'; }
case "$1 $2" in
  "plugin list") list "$d/installed" ;;
  "plugin marketplace")
    case "$3" in
      list) list "$d/markets" ;;
      add) printf '{"name":"acme-plugins","source":"github","repo":"%s"}' "$4" > "$d/markets/acme-plugins" ;;
    esac ;;
  "plugin install") printf '{"id":"%s","scope":"user","enabled":true}' "$3" > "$d/installed/$3" ;;
  "plugin uninstall") rm "$d/installed/$3" || exit 1 ;;
  *) exit 3 ;;
esac
"#
    }
    .replace("@DIR@", &dir.display().to_string());
    script(&dir.join(if codex { "bin/codex" } else { "bin/claude" }), &body)
}

fn mac(tag: &str, name: &'static str, codex: &str) -> Mac {
    let root = temp_dir(tag);
    let home = root.join("Users").join(tag);
    for d in [".codex", ".claude", ".gemini/config", ".agents/skills"] {
        std::fs::create_dir_all(home.join(d)).unwrap();
    }
    std::fs::write(
        home.join(".codex/config.toml"),
        codex.replace("@HOME@", &home.display().to_string()),
    )
    .unwrap();
    script(&home.join(".local/bin/files-mcp"), "exit 0\n");
    let claude = fake_agent(&root.join("claude"), false);
    let codex = fake_agent(&root.join("codex"), true);
    let npx = Npx::new(fake_npx(&root, &home), npx::run_dir(&root.join("app")));
    Mac {
        places: Places::new(&home, &home.join(".codex"), None),
        skills: SkillPlaces::new(&home, &home.join(".codex"), Some(npx.program.clone())),
        secrets: Secrets::fake(common::keychain::fake_security(&root.join("keychain"))),
        app: AppDir::new(root.join("app")),
        root,
        home,
        claude,
        codex,
        npx,
        name,
    }
}

fn lock(m: &Mac, entries: &[(&str, &str, &str)]) {
    let body: Vec<String> = entries
        .iter()
        .map(|(n, hash, updated)| {
            format!(
                r#""{n}": {{"source": "acme/agent-skills", "sourceType": "github", "skillPath": "skills/{n}/SKILL.md", "skillFolderHash": "{hash}", "updatedAt": "{updated}"}}"#
            )
        })
        .collect();
    std::fs::write(
        m.home.join(".agents/.skill-lock.json"),
        format!(r#"{{"version": 3, "skills": {{{}}}}}"#, body.join(", ")),
    )
    .unwrap();
}

/// The Mac the copy starts from.
fn mini() -> Mac {
    let m = mac("mini", "Studio mini", MINI_CODEX);
    let shared = m.home.join(".agents/skills");
    skill(&shared.join("ui-polish"), "ui-polish");
    skill(&shared.join("pdf-tools"), "pdf-tools");
    lock(
        &m,
        &[
            ("ui-polish", "e4f5a6b", "2026-10-17T08:00:00Z"),
            ("pdf-tools", "aa11", "2026-09-01T08:00:00Z"),
        ],
    );
    // Installed by hand, gathered into the shared folder.
    skill(&shared.join("deploy-check"), "deploy-check");
    std::fs::write(shared.join("deploy-check/steps.md"), "Check the build.\n").unwrap();
    // By hand, holding a key: stays here.
    skill(&shared.join("secret-skill"), "secret-skill");
    std::fs::write(shared.join("secret-skill/env.sh"), format!("export API_TOKEN={SKILL_TRAP}\n"))
        .unwrap();
    std::fs::write(
        m.root.join("claude/installed/formatter@acme-plugins"),
        r#"{"id":"formatter@acme-plugins","scope":"user","enabled":true}"#,
    )
    .unwrap();
    std::fs::write(
        m.root.join("claude/markets/acme-plugins"),
        r#"{"name":"acme-plugins","source":"github","repo":"acme/claude-plugins"}"#,
    )
    .unwrap();
    std::fs::create_dir_all(m.home.join(".claude/rules")).unwrap();
    std::fs::write(
        m.home.join(".claude/CLAUDE.md"),
        "# Chung\n- Dùng pnpm.\n- Chạy test trước khi commit.\n",
    )
    .unwrap();
    std::fs::write(m.home.join(".claude/rules/style.md"), "- Short sentences.\n").unwrap();
    m
}

/// The other Mac: an older ui-polish, pdf-tools as here, another `files`.
fn air() -> Mac {
    let m = mac("air", "Travel air", AIR_CODEX);
    let shared = m.home.join(".agents/skills");
    skill(&shared.join("ui-polish"), "ui-polish");
    skill(&shared.join("pdf-tools"), "pdf-tools");
    lock(
        &m,
        &[
            ("ui-polish", "a1b2c3d", "2026-09-12T08:00:00Z"),
            ("pdf-tools", "aa11", "2026-09-01T08:00:00Z"),
        ],
    );
    std::fs::create_dir_all(m.home.join(".claude/rules")).unwrap();
    std::fs::write(
        m.home.join(".claude/CLAUDE.md"),
        "# Chung\n- Dùng npm.\n- Chạy test trước khi commit.\n",
    )
    .unwrap();
    std::fs::write(m.home.join(".claude/rules/style.md"), "- Short sentences.\n").unwrap();
    m
}

fn tools(m: &Mac) -> Tools {
    Tools {
        skills: m.skills.clone(),
        npx: Some(m.npx.clone()),
        claude: Some(m.claude.clone()),
        codex: Some(m.codex.clone()),
        codex_home: m.home.join(".codex"),
        root: m.root.join("packs"),
    }
}

/// The Mac as the app reads it for a copy.
fn here(m: &Mac) -> Here {
    let readings = agents::read_all(&m.places);
    let known = pack::secret_values(&readings);
    let mut p = pack::content(&m.home, &readings, &kho::load(&m.app), m.name, "2.0.0", at(0));
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
    Here {
        pack: p,
        files: copy::file_infos(&read, &choices),
        versions: copy::versions(&look),
        agents: Agent::ALL.into_iter().filter(|a| m.places.present(*a)).collect(),
        readings,
    }
}

fn no_trap(what: &str, text: &str) {
    for t in TRAPS {
        assert!(!text.contains(t), "{t} in {what}");
    }
}

fn item<'a>(d: &'a copy::Diff, group: &str, id: &str) -> &'a copy::DiffItem {
    d.items.iter().find(|i| i.group == group && i.id == id).unwrap_or_else(|| {
        panic!("{group} {id} in {:?}", d.items.iter().map(|i| &i.id).collect::<Vec<_>>())
    })
}

#[test]
fn what_is_read_holds_no_key_and_the_comparison_finds_each_kind() {
    let (a, b) = (mini(), air());
    let mine = here(&a).inventory();
    let theirs = here(&b).inventory();
    no_trap("the inventory", &serde_json::to_string(&mine).unwrap());

    let d = copy::diff(&mine, &theirs);
    // Only here, only there.
    assert_eq!(item(&d, "mcp", "tracker").state, "mine");
    assert_eq!(item(&d, "mcp", "tracker").keys, vec!["TRACKER_TOKEN"]);
    assert_eq!(item(&d, "mcp", "notes").state, "theirs");
    // `files` differs in its arguments; its command at another path does not count.
    let files = item(&d, "mcp", "files");
    assert_eq!((files.state, files.what.clone()), ("diff", vec!["args"]));
    // ui-polish is newer here: it goes there, it does not come back.
    let ui = item(&d, "skill", "ui-polish");
    assert_eq!((ui.state, ui.push, ui.pull), ("ver", None, Some("older")));
    // Installed by hand: its files go; the one with a key stays.
    assert_eq!(item(&d, "skill", "deploy-check").push, None);
    let secret = item(&d, "skill", "secret-skill");
    assert_eq!((secret.push, secret.note.as_deref()), (Some("leftOut"), Some("key")));
    assert_eq!(item(&d, "plugin", "formatter@acme-plugins").state, "mine");
    let claude_md = item(&d, "file", "~/.claude/CLAUDE.md");
    assert_eq!((claude_md.state, claude_md.changed), ("text", 1));
    for same in ["pdf-tools", "~/.claude/rules/style.md"] {
        assert!(d.same.iter().any(|s| s == same), "{same}");
    }
    no_trap("the comparison", &serde_json::to_string(&d).unwrap());
}

#[test]
fn keys_go_only_when_asked_and_only_as_values_on_stdin() {
    let a = mini();
    let h = here(&a);
    let sel = Selection { mcp: vec!["tracker".into()], ..Selection::default() };
    let without = copy::payload(&h, &sel, None).unwrap();
    assert!(without.keys.is_empty());
    no_trap("a payload without keys", &serde_json::to_string(&without).unwrap());
    let with = copy::payload(&h, &sel, Some(&a.secrets)).unwrap();
    assert_eq!(with.keys.len(), 1);
    assert_eq!((with.keys[0].mcp.as_str(), with.keys[0].value.as_str()), ("tracker", MCP_TRAP));
    // Its debug text never shows the value.
    assert!(!format!("{with:?}").contains(MCP_TRAP));
}

fn push(
    a: &Mac,
    b: &Mac,
    sel: &Selection,
    keys: bool,
    when: i64,
) -> Result<restore::Done, ApplyError> {
    let theirs = here(b).inventory();
    let mut p = copy::payload(&here(a), sel, keys.then_some(&a.secrets)).unwrap();
    p.bases = theirs.bases.clone();
    p.file_bases = theirs.file_bases();
    copy::apply(&b.places, &b.app, &b.secrets, &tools(b), &p, at(when))
}

#[test]
fn the_other_mac_writes_as_a_restore_and_undoes_it() {
    let (a, b) = (mini(), air());
    let before = std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap();
    let claude_before = std::fs::read_to_string(b.home.join(".claude/CLAUDE.md")).unwrap();
    let sel = Selection {
        mcp: vec!["files".into(), "tracker".into()],
        skills: vec!["deploy-check".into()],
        updates: vec!["ui-polish".into()],
        plugins: vec![PluginRef { agent: Agent::Claude, id: "formatter@acme-plugins".into() }],
        files: vec!["~/.claude/CLAUDE.md".into()],
    };
    let done = push(&a, &b, &sel, false, 10).unwrap();
    assert!(done.can_undo);
    let config = std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap();
    // `files` took this Mac's arguments (`~` is the other Mac's home), with
    // the other Mac's own command; the provider stays; tracker waits for its
    // key, kept off by the app.
    let workspace = format!(r#"args = ["--root", "{}/Workspace"]"#, b.home.display());
    assert!(config.contains(&workspace), "{config}");
    assert!(
        config.contains(&format!("command = \"{}/.local/bin/files-mcp\"", b.home.display())),
        "{config}"
    );
    assert!(config.contains(r#"model_provider = "openai""#));
    assert!(!config.contains("[mcp_servers.tracker]"), "{config}");
    assert!(kho::load(&b.app).waiting.contains_key("tracker"));
    no_trap("the other Mac's config", &config);
    assert!(b.home.join(".agents/skills/deploy-check/steps.md").is_file());
    let ui = std::fs::read_to_string(b.home.join(".agents/skills/ui-polish/SKILL.md")).unwrap();
    assert!(ui.ends_with("Newest.\n"));
    assert!(b.root.join("claude/installed/formatter@acme-plugins").is_file());
    assert!(std::fs::read_to_string(b.home.join(".claude/CLAUDE.md")).unwrap().contains("pnpm"));
    // The pack it came in is gone; the restore record names no key.
    assert!(!b.app.path().join(copy::INCOMING).exists());
    no_trap("the record", &std::fs::read_to_string(b.app.path().join(restore::FILE)).unwrap());

    // Undo runs there: everything as it was.
    let wrong = copy::undo(
        &b.places,
        &b.app,
        &b.secrets,
        &tools(&b),
        &UndoAsk { at: done.at + 1, force: false },
        at(20),
    );
    assert_eq!(wrong.unwrap_err(), serde_json::json!("gone"));
    copy::undo(
        &b.places,
        &b.app,
        &b.secrets,
        &tools(&b),
        &UndoAsk { at: done.at, force: false },
        at(20),
    )
    .unwrap();
    assert_eq!(std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap(), before);
    assert!(before.contains(".local/bin/files-mcp"));
    assert_eq!(std::fs::read_to_string(b.home.join(".claude/CLAUDE.md")).unwrap(), claude_before);
    assert!(!b.home.join(".agents/skills/deploy-check").exists());
    let ui = std::fs::read_to_string(b.home.join(".agents/skills/ui-polish/SKILL.md")).unwrap();
    assert!(!ui.contains("Newest."));
    assert!(!kho::load(&b.app).waiting.contains_key("tracker"));
}

#[test]
fn a_key_sent_goes_into_the_other_mac_s_keychain() {
    let (a, b) = (mini(), air());
    let sel = Selection { mcp: vec!["tracker".into()], ..Selection::default() };
    let done = push(&a, &b, &sel, true, 10).unwrap();
    assert_eq!(done.keys, 1);
    assert_eq!(b.secrets.read("tracker", "env:TRACKER_TOKEN").unwrap(), MCP_TRAP);
    let config = std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap();
    assert!(config.contains("[mcp_servers.tracker]"), "{config}");
    // Not in the backups or the record.
    no_trap("the record", &std::fs::read_to_string(b.app.path().join(restore::FILE)).unwrap());
}

#[test]
fn a_file_changed_there_since_the_comparison_is_not_written() {
    let (a, b) = (mini(), air());
    let theirs = here(&b).inventory();
    let sel = Selection { mcp: vec!["files".into()], ..Selection::default() };
    let mut p = copy::payload(&here(&a), &sel, None).unwrap();
    p.bases = theirs.bases.clone();
    let edited = format!(
        "{}\n# edited there\n",
        std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap()
    );
    std::fs::write(b.home.join(".codex/config.toml"), &edited).unwrap();
    let r = copy::apply(&b.places, &b.app, &b.secrets, &tools(&b), &p, at(10));
    assert!(matches!(r, Err(ApplyError::Run(RunError::Changed(_)))), "{r:?}");
    assert_eq!(std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap(), edited);
}

#[test]
fn names_and_paths_the_app_does_not_write_are_refused() {
    let (a, b) = (mini(), air());
    let sel = Selection { skills: vec!["deploy-check".into()], ..Selection::default() };
    let mut p = copy::payload(&here(&a), &sel, None).unwrap();
    p.skill_files[0].files[0].path = "../../escape.md".into();
    let r = copy::apply(&b.places, &b.app, &b.secrets, &tools(&b), &p, at(10));
    assert!(matches!(r, Err(ApplyError::Invalid(_))), "{r:?}");
    assert!(!b.root.join("escape.md").exists());
    p.protocol = 99;
    assert_eq!(
        copy::apply(&b.places, &b.app, &b.secrets, &tools(&b), &p, at(10)).unwrap_err(),
        ApplyError::Protocol
    );
}

// ===== Over SSH =====

/// A fake network: `ssh` records its words and stdin, then runs the
/// script in the alias's home; alias `down` cannot be reached.
struct Net {
    dir: PathBuf,
    ssh: PathBuf,
}

impl Net {
    fn new(tag: &str) -> Net {
        let dir = temp_dir(tag);
        let macs = dir.join("macs");
        std::fs::create_dir_all(&macs).unwrap();
        let ssh = script(
            &dir.join("bin/ssh"),
            &format!(
                r#"macs='{macs}'
for a in "$@"; do printf '%s\037' "$a" >> "$macs/argv"; done
printf '\n' >> "$macs/argv"
while [ $# -gt 0 ]; do
  case "$1" in
    -o) shift 2 ;;
    -T) shift ;;
    *) break ;;
  esac
done
alias=$1; shift
if [ "$alias" = down ]; then
  echo "ssh: connect to host down port 22: Operation timed out" >&2
  exit 255
fi
mkdir -p "$macs/$alias"
cat > "$macs/$alias/.stdin"
HOME="$macs/$alias" /bin/sh -c "$*" < "$macs/$alias/.stdin"
"#,
                macs = macs.display()
            ),
        );
        Net { dir, ssh }
    }

    fn home(&self, alias: &str) -> PathBuf {
        self.dir.join("macs").join(alias)
    }

    /// A Quota Station on `alias`, speaking `protocol` (none: version 1),
    /// answering each word from `answers/<word>` with `answers/<word>.code`.
    fn app(&self, alias: &str, protocol: Option<u32>) -> PathBuf {
        let bundle = self.home(alias).join("Applications/Quota Station.app/Contents");
        std::fs::create_dir_all(bundle.join("MacOS")).unwrap();
        let key = protocol
            .map(|p| format!("<key>{}</key><integer>{p}</integer>", copy::PLIST_KEY))
            .unwrap_or_default();
        std::fs::write(
            bundle.join("Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleExecutable</key><string>quota-station</string>{key}</dict></plist>
"#
            ),
        )
        .unwrap();
        let state = self.home(alias).join("app");
        std::fs::create_dir_all(state.join("answers")).unwrap();
        script(
            &bundle.join("MacOS/quota-station"),
            &format!(
                r#"d='{state}'
printf '%s\n' "$*" >> "$d/words"
cat >> "$d/stdin"
w="$2"
n=$(cat "$d/n-$w" 2>/dev/null || echo 0); n=$((n+1)); echo $n > "$d/n-$w"
f="$d/answers/$w.$n"; [ -e "$f" ] || f="$d/answers/$w"
cat "$f" 2>/dev/null
exit $(cat "$d/answers/$w.code" 2>/dev/null || echo 0)
"#,
                state = state.display()
            ),
        );
        state
    }

    fn remote(&self, alias: &str) -> Remote {
        let mut r = Remote::new(self.ssh.clone(), alias);
        // Only the fake Mac's own folder: never this Mac's Applications.
        r.folders = vec!["$HOME/Applications".into()];
        r
    }

    fn argv(&self) -> Vec<Vec<String>> {
        std::fs::read_to_string(self.dir.join("macs/argv"))
            .unwrap_or_default()
            .lines()
            .map(|l| l.split('\u{1f}').filter(|a| !a.is_empty()).map(str::to_string).collect())
            .collect()
    }
}

fn answer(state: &Path, word: &str, text: &str) {
    std::fs::write(state.join("answers").join(word), text).unwrap();
}

#[test]
fn the_other_mac_runs_only_the_fixed_script_in_batch_mode() {
    let net = Net::new("copy-ssh-read");
    let state = net.app("travel-air", Some(copy::PROTOCOL));
    let inv = here(&air()).inventory();
    answer(&state, "read", &serde_json::to_string(&inv).unwrap());
    let r = net.remote("travel-air");
    assert_eq!(r.read().unwrap(), inv);

    let argv = net.argv();
    assert_eq!(argv.len(), 1);
    assert_eq!(
        argv[0],
        vec![
            "-o".to_string(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=5".into(),
            "-T".into(),
            "travel-air".into(),
            format!("/bin/sh -c '{}'", copy::script(Word::Read, &["$HOME/Applications"])),
        ]
    );
    assert_eq!(std::fs::read_to_string(state.join("words")).unwrap(), "agents read\n");
    // Each call waits at most 20 seconds, as the rotation file's copies do.
    assert_eq!(peers::SSH_TIMEOUT, Duration::from_secs(20));
}

#[test]
fn a_mac_on_version_1_is_never_run_so_never_written_to() {
    let net = Net::new("copy-ssh-old");
    let state = net.app("studio-m6", None);
    let r = net.remote("studio-m6");
    assert_eq!(r.read().unwrap_err(), CopyError::Old);
    let p = copy::payload(
        &here(&mini()),
        &Selection { mcp: vec!["files".into()], ..Selection::default() },
        None,
    )
    .unwrap();
    assert_eq!(r.start(&JobInput::Apply { payload: Box::new(p) }).unwrap_err(), CopyError::Old);
    assert_eq!(r.export(&Selection::default(), false).unwrap_err(), CopyError::Old);
    assert!(!state.join("words").exists(), "the old app ran");

    // An app that says an older protocol in what it reads is old too.
    let net = Net::new("copy-ssh-old2");
    let state = net.app("studio-m6", Some(copy::PROTOCOL));
    let mut inv = here(&air()).inventory();
    inv.protocol = 0;
    answer(&state, "read", &serde_json::to_string(&inv).unwrap());
    assert_eq!(net.remote("studio-m6").read().unwrap_err(), CopyError::Old);
}

#[test]
fn no_app_and_no_answer_are_said_apart() {
    let net = Net::new("copy-ssh-none");
    std::fs::create_dir_all(net.home("bare")).unwrap();
    assert_eq!(net.remote("bare").read().unwrap_err(), CopyError::NoApp);
    assert!(
        matches!(net.remote("down").read().unwrap_err(), CopyError::Unreachable(e) if e.contains("timed out"))
    );
    assert!(matches!(net.remote("-oProxyCommand=x").read().unwrap_err(), CopyError::Failed(_)));
}

#[test]
fn the_keys_travel_on_stdin_only_and_only_when_ticked() {
    let a = mini();
    let sel = Selection { mcp: vec!["tracker".into()], ..Selection::default() };
    for keys in [false, true] {
        let net = Net::new(if keys { "copy-ssh-keys" } else { "copy-ssh-nokeys" });
        let state = net.app("travel-air", Some(copy::PROTOCOL));
        let job = Job {
            kind: "apply".into(),
            state: "running".into(),
            started: 5,
            pid: 42,
            result: None,
            error: None,
        };
        answer(&state, "start", &serde_json::to_string(&job).unwrap());
        let p = copy::payload(&here(&a), &sel, keys.then_some(&a.secrets)).unwrap();
        let back =
            net.remote("travel-air").start(&JobInput::Apply { payload: Box::new(p) }).unwrap();
        assert_eq!(back, job);

        let argv = net.argv().concat().join(" ");
        no_trap("ssh's words", &argv);
        assert!(!argv.contains("tracker"), "{argv}");
        let stdin = std::fs::read_to_string(state.join("stdin")).unwrap();
        assert_eq!(stdin.contains(MCP_TRAP), keys, "{stdin}");
        assert_eq!(std::fs::read_to_string(state.join("words")).unwrap(), "agents start\n");
    }
}

#[test]
fn the_app_asks_until_the_write_there_ends() {
    let net = Net::new("copy-ssh-wait");
    let state = net.app("travel-air", Some(copy::PROTOCOL));
    let job = |s: &str, started: i64| Job {
        kind: "apply".into(),
        state: s.into(),
        started,
        pid: 42,
        result: (s == "done").then(|| serde_json::json!({ "at": 7 })),
        error: None,
    };
    // The job before, then this one running, then done.
    answer(&state, "status.1", &serde_json::to_string(&job("done", 1)).unwrap());
    answer(&state, "status.2", &serde_json::to_string(&job("running", 5)).unwrap());
    answer(&state, "status", &serde_json::to_string(&job("done", 5)).unwrap());
    let r = net.remote("travel-air");
    let slept = std::cell::Cell::new(0);
    let end = r
        .wait(&job("running", 5), Duration::from_secs(1), Duration::from_secs(60), &|_| {
            slept.set(slept.get() + 1)
        })
        .unwrap();
    assert_eq!((end.state.as_str(), slept.get()), ("done", 2));

    // A refusal says why; a job that never ends stops the wait.
    answer(&state, "start", r#"{"error":"busy"}"#);
    std::fs::write(state.join("answers/start.code"), "7").unwrap();
    let undo = JobInput::Undo { ask: UndoAsk { at: 7, force: false } };
    assert_eq!(r.start(&undo).unwrap_err(), CopyError::Refused("busy".into()));
    answer(&state, "status", &serde_json::to_string(&job("running", 9)).unwrap());
    assert_eq!(
        r.wait(&job("running", 9), Duration::from_secs(1), Duration::from_secs(2), &|_| {})
            .unwrap_err(),
        CopyError::Waiting
    );
}

#[test]
fn a_job_runs_the_copy_and_says_how_it_ended() {
    let (a, b) = (mini(), air());
    let theirs = here(&b).inventory();
    let mut p = copy::payload(
        &here(&a),
        &Selection { mcp: vec!["files".into()], ..Selection::default() },
        None,
    )
    .unwrap();
    p.bases = theirs.bases.clone();
    let job = copy::run_job(
        &b.places,
        &b.app,
        &b.secrets,
        &tools(&b),
        &JobInput::Apply { payload: Box::new(p) },
        5,
        at(10),
    );
    assert_eq!(job.state, "done");
    assert_eq!(copy::job(&b.app).unwrap().state, "done");
    assert!(!copy::busy(&b.app));
    let at_ = job.result.as_ref().and_then(|r| r.get("at")).and_then(|v| v.as_i64()).unwrap();
    let undo = copy::run_job(
        &b.places,
        &b.app,
        &b.secrets,
        &tools(&b),
        &JobInput::Undo { ask: UndoAsk { at: at_, force: false } },
        6,
        at(20),
    );
    assert_eq!(undo.state, "done", "{undo:?}");
    assert_eq!(
        std::fs::read_to_string(b.home.join(".codex/config.toml")).unwrap(),
        AIR_CODEX.replace("@HOME@", &b.home.display().to_string())
    );
    // Undone once: nothing left to undo.
    let again = copy::run_job(
        &b.places,
        &b.app,
        &b.secrets,
        &tools(&b),
        &JobInput::Undo { ask: UndoAsk { at: at_, force: false } },
        7,
        at(30),
    );
    assert_eq!(
        (again.state.as_str(), again.error.clone()),
        ("failed", Some(serde_json::json!("gone")))
    );
    // A running job whose process is gone reads as lost.
    let mut lost = again.clone();
    lost.state = "running".into();
    lost.pid = 999_999;
    copy::save_job(&b.app, &lost).unwrap();
    assert_eq!(copy::job(&b.app).unwrap().state, "lost");
}

#[test]
fn the_app_bundle_says_it_speaks_the_copy() {
    let plist = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/Info.plist"),
    )
    .unwrap();
    let want = format!("<key>{}</key>\n\t<integer>{}</integer>", copy::PLIST_KEY, copy::PROTOCOL);
    assert!(plist.contains(&want), "{plist}");
}

#[test]
fn inventory_files_say_why_one_stays() {
    let b = air();
    std::fs::write(b.home.join(".claude/CLAUDE.md"), format!("Use TOKEN={MCP_TRAP}\n")).unwrap();
    let inv = here(&b).inventory();
    let f = inv.files.iter().find(|f| f.path == "~/.claude/CLAUDE.md").unwrap();
    assert_eq!((f.held.as_deref(), f.text.is_none()), (Some("key"), true));
    no_trap("the inventory", &serde_json::to_string(&inv).unwrap());
    let _unused: BTreeMap<String, String> = inv.file_bases();
    let _: Option<Inventory> = None;
}

#[test]
fn a_key_pasted_for_codex_s_provider_never_goes_to_another_mac() {
    use quota_station_core::codex;
    let a = mini();
    let key = "sk-pasted-copy-trap-0123456789";
    let codex_home = a.home.join(".codex");
    let h = codex::files::sha256(&std::fs::read(codex_home.join(codex::CONFIG)).unwrap());
    let out = codex::set_key(&codex_home, &a.app, key, None, &h, at(0));
    assert!(matches!(out, codex::KeyOutcome::Written { .. }), "{out:?}");
    let here = here(&a);
    assert!(!serde_json::to_string(&here.inventory()).unwrap().contains(key));
    let sel = Selection { mcp: vec!["tracker".into(), "files".into()], ..Selection::default() };
    let with = copy::payload(&here, &sel, Some(&a.secrets)).unwrap();
    assert!(!serde_json::to_string(&with).unwrap().contains(key), "not even with its keys");
}
