//! Restoring a pack on this Mac (M10 for the MCP servers, M12 for the
//! skills, plugins and instruction files), in the steps the Khôi phục sheet
//! shows: [`plan`] (what each would become, read now; nothing written),
//! then [`run`] with what was ticked and the keys pasted from Bitwarden,
//! [`retry`] of one part that failed, then, if wanted, [`undo`] of the
//! whole restore.
//!
//! - A server not in an agent here is added for it. One the agent has
//!   already, the same apart from its secrets and the home folder, is left
//!   alone; a different one is offered, unticked: the Mac's own stays
//!   unless the owner ticks it.
//! - Paths follow the Mac ([`pack::localize`]): `~` is this home folder, a
//!   command missing at its full path is looked for by its name.
//! - A secret is never in the pack. A key this Mac has (in an agent's file
//!   or the app's Keychain item) is used; one pasted goes to the Keychain
//!   first. A server still short of a key, or whose command is not on this
//!   Mac, is added turned off: the app keeps it ([`super::kho::Kho`]'s
//!   `waiting`, or `parked` for Claude Code), and the MCP table shows it off
//!   with its lock until the key is pasted or the command installed.
//! - Every file is written through [`batch::write`]: checked against what
//!   the plan read, backed up first without secrets, each file once.
//! - Codex's provider, logins and accounts are never touched.
//!
//! The other parts (M12) run one after the other, each on its own: one that
//! fails leaves the others to run, and says whether "Thử lại" may help.
//!
//! - Skills: one from GitHub is installed again with `npx skills add` (the
//!   newest version); one installed by hand is copied from the pack into
//!   the shared folder, its tree id checked. Then each agent's box is set as
//!   the pack had it, through the Skills tab's checked batch. A skill this
//!   Mac has already is left as it is. Nothing in a skill is run.
//! - Plugins: each is installed with the agent's own command, its
//!   marketplace added first when this Mac has not got it, Claude Code's
//!   turned off again when it was off ([`plugins::install_steps`]). One this
//!   Mac has is left as it is.
//! - Instruction files: one this Mac has not got is written; one that
//!   differs is offered unticked, and the Mac's own stays unless the owner
//!   ticks it. One whose copy here has a line shaped like a key is never
//!   replaced. Each write is checked and the old file kept first.
//!
//! The last restore is kept in `agents/restore-last.json` (names only, no
//! secret) so that it can be undone after the app restarts: the batch that
//! puts the files back, the servers the app kept off, the Keychain items
//! the restore made, the skills and plugins it installed and the
//! instruction files it wrote.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Map;
use time::OffsetDateTime;

use super::batch::{self, Change, Line, To, Undo};
use super::def::{Def, REDACTED};
use super::kho::{self, Kept};
use super::npx::{Npx, NpxError};
use super::pack::{self, Pack};
use super::plugins::{self, Side};
use super::secrets::Secrets;
use super::skills::{self, SkillPlaces};
use super::{def_view, kept_view, missing_command, writable, Agent, DefView, Have, Places};
use super::{instructions, skill_ops, skill_updates, write};
use crate::antigravity::fs::AppDir;
use crate::runner;

/// The last restore, in the app's folder.
pub const FILE: &str = "agents/restore-last.json";
/// The instruction files a restore replaced, as they were, in the app's folder.
pub const FILE_BACKUPS: &str = "backups/instructions";

/// What a restore needs besides the MCP files: the skills' folders, `npx`,
/// the agents' own commands for plugins, Codex's folder and the packs'.
#[derive(Debug, Clone)]
pub struct Tools {
    pub skills: SkillPlaces,
    pub npx: Option<Npx>,
    pub claude: Option<PathBuf>,
    pub codex: Option<PathBuf>,
    pub codex_home: PathBuf,
    /// The folder the packs are in.
    pub root: PathBuf,
}

impl Tools {
    fn program(&self, agent: Agent) -> Option<&Path> {
        match agent {
            Agent::Claude => self.claude.as_deref(),
            _ => self.codex.as_deref(),
        }
    }

    /// The agent's plugins on this Mac now, read with its own command.
    fn plugins_here(&self, agent: Agent) -> Side {
        let home = &self.skills.home;
        match agent {
            Agent::Claude => plugins::read_claude(self.claude.as_deref(), home),
            _ => plugins::read_codex(self.codex.as_deref(), home),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compare {
    pub agent: Agent,
    pub here: DefView,
    pub pack: DefView,
}

/// One line of the preview: a server to add for some agents, or one to
/// put in place of the Mac's own.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    /// `add` or `replace`.
    pub kind: &'static str,
    pub agents: Vec<Agent>,
    /// Ticked to begin with: an add is, a replace is not.
    pub checked: bool,
    /// Agents it goes to turned off, as it was in the pack.
    pub off: Vec<Agent>,
    /// Its command, when it is not on this Mac: added off.
    pub missing_command: Option<String>,
    /// Its command is somewhere else on this Mac than in the pack.
    pub moved: bool,
    /// Its secrets this Mac has nowhere.
    pub keys: Vec<String>,
    /// Over the network with no key: the agent may ask to log in again.
    pub login: bool,
    /// For a replace: the Mac's own and the pack's, secrets masked.
    pub compare: Vec<Compare>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyNeed {
    pub mcp: String,
    pub slot: String,
    pub label: String,
    pub bitwarden: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub agent: Agent,
    /// `absent` (not on this Mac), `readOnly` (its file does not read, or
    /// no `claude`), or `sse` (Codex and an SSE server, by name).
    pub reason: &'static str,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub pack: String,
    pub mac: String,
    pub at: i64,
    pub items: Vec<Item>,
    /// Servers this Mac has as the pack does.
    pub same: Vec<String>,
    pub keys: Vec<KeyNeed>,
    pub skipped: Vec<Skipped>,
    /// What each file was when read, to send back with the run.
    pub bases: BTreeMap<Agent, String>,
    /// The pack's skills, plugins and instruction files, when it has them.
    pub skills: Option<SkillPlan>,
    pub plugins: Option<PluginPlan>,
    pub files: Option<FilePlan>,
}

/// A skill of the pack this Mac has not got.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillItem {
    pub name: String,
    /// `github`: installed from its repository again, the newest version;
    /// `hand`: installed by hand, copied from the pack.
    pub kind: &'static str,
    pub repo: Option<String>,
    /// The agents here it goes on for, as in the pack.
    pub on: Vec<Agent>,
    pub checked: bool,
    /// Why it cannot be restored now: `noNpx` (no Node on this Mac) or
    /// `cloud` (iCloud Drive has not downloaded its files).
    pub blocked: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftOut {
    pub name: String,
    /// `key`, `link`, `large`, `unreadable`, or `old` (a pack from before
    /// the files went into it).
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPlan {
    pub items: Vec<SkillItem>,
    /// Skills this Mac has already: left as they are.
    pub same: Vec<String>,
    /// Skills installed by hand whose files are not in the pack.
    pub left_out: Vec<LeftOut>,
}

/// A plugin of the pack this Mac has not got.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginItem {
    pub agent: Agent,
    /// `name@marketplace`.
    pub id: String,
    pub enabled: bool,
    /// The marketplace added first, when this Mac has not got it.
    pub market: Option<String>,
    pub checked: bool,
    /// `noSource`: its marketplace is one the app cannot add here (a folder
    /// on the other Mac); `codexOff`: off in the pack, and Codex has no
    /// command to turn it off, so installing turns it on.
    pub note: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPlan {
    pub items: Vec<PluginItem>,
    /// Plugins this Mac has already: left as they are.
    pub same: Vec<String>,
    /// Agents whose plugins are left out: `noCli` (the command is not on
    /// this Mac) or `failed` (its list does not read).
    pub skipped: Vec<Skipped>,
}

/// An instruction file of the pack to write here.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileItem {
    /// `~/…` on this Mac.
    pub path: String,
    pub agent: Option<Agent>,
    /// `add` (this Mac has not got it) or `replace` (this Mac's differs).
    pub kind: &'static str,
    /// An add is ticked; a replace is not: the Mac's own stays.
    pub checked: bool,
    /// For a replace: how many lines differ, and this Mac's (`del`)
    /// against the pack's (`add`), keys masked.
    pub changed: usize,
    pub diff: Vec<Line>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileKept {
    pub path: String,
    pub agent: Option<Agent>,
    /// `key` (this Mac's has a line shaped like a key), `absent` (its agent
    /// is not on this Mac), `unreadable`, or `path` (not a file the app
    /// writes).
    pub why: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePlan {
    pub items: Vec<FileItem>,
    /// Files this Mac has as the pack does.
    pub same: Vec<String>,
    /// Files left as they are here, and why.
    pub kept: Vec<FileKept>,
    /// What each file was when read: its SHA-256, empty when not there.
    pub bases: BTreeMap<String, String>,
}

/// What a server is in an agent here: its entry, or the app's own copy.
fn here(
    readings: &[super::Reading],
    store: &kho::Kho,
    name: &str,
    agent: Agent,
) -> Option<(Def, DefView)> {
    let r = readings.iter().find(|r| r.agent == agent)?;
    if let Some(s) = r.server(name) {
        let d = s.def.as_ref()?;
        return Some((d.stripped(), def_view(d, &s.extra)));
    }
    let k = match agent {
        Agent::Claude => store.parked.get(name),
        _ => None,
    }
    .or_else(|| store.waiting.get(name).filter(|_| store.waits(name, agent)))?;
    Some((k.def.clone(), kept_view(k)))
}

/// What restoring `pack` would do here, read now. Writes nothing; the only
/// commands run are the agents' own lists of their plugins.
pub fn plan(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    tools: &Tools,
    id: &str,
    pack: &Pack,
) -> Plan {
    let readings = super::read_all(places);
    let store = kho::load(app);
    let have = Have { readings: &readings, secrets };
    let mut items = Vec::new();
    let mut same = Vec::new();
    let mut skipped: Vec<Skipped> = Vec::new();
    let mut keys: Vec<KeyNeed> = Vec::new();
    for server in &pack.mcp.servers {
        let name = server.name.as_str();
        let mut add = Vec::new();
        let mut off = Vec::new();
        let mut replace = Vec::new();
        let mut compare = Vec::new();
        let mut alike = 0;
        let mut defs: Vec<Def> = Vec::new();
        let mut moved = false;
        for entry in &server.entries {
            let Some(r) = readings.iter().find(|r| r.agent == entry.agent) else { continue };
            if !r.present {
                if !skipped.iter().any(|s| s.agent == r.agent && s.reason == "absent") {
                    skipped.push(Skipped { agent: r.agent, reason: "absent", name: None });
                }
                continue;
            }
            if !writable(places, r) {
                if !skipped.iter().any(|s| s.agent == r.agent && s.reason == "readOnly") {
                    skipped.push(Skipped { agent: r.agent, reason: "readOnly", name: None });
                }
                continue;
            }
            if r.server(name).is_some_and(|s| s.owner.is_some()) {
                continue;
            }
            let (def, m) = pack::localize(&entry.def, &places.home);
            if entry.agent == Agent::Codex && def.is_sse() {
                skipped.push(Skipped { agent: r.agent, reason: "sse", name: Some(name.into()) });
                continue;
            }
            match here(&readings, &store, name, entry.agent) {
                Some((mine, view)) => {
                    if mine.same(&def) || pack::portable(&mine, &places.home).same(&entry.def) {
                        alike += 1;
                    } else {
                        replace.push(entry.agent);
                        compare.push(Compare {
                            agent: entry.agent,
                            here: view,
                            pack: def_view(&def.masked(super::def::MASK), &[]),
                        });
                        moved |= m;
                        defs.push(def);
                    }
                }
                None => {
                    add.push(entry.agent);
                    if !entry.enabled {
                        off.push(entry.agent);
                    }
                    moved |= m;
                    defs.push(def);
                }
            }
        }
        if add.is_empty() && replace.is_empty() {
            if alike > 0 {
                same.push(name.to_string());
            }
            continue;
        }
        let mut need: Vec<String> = Vec::new();
        for d in &defs {
            for slot in pack::holes(d) {
                if !need.contains(&slot) && !have.exists(name, &slot) {
                    need.push(slot);
                }
            }
        }
        for slot in &need {
            if !keys.iter().any(|k| k.mcp == name && &k.slot == slot) {
                let n = pack.needed(name, slot);
                keys.push(KeyNeed {
                    mcp: name.to_string(),
                    slot: slot.clone(),
                    label: n
                        .map(|n| n.label.clone())
                        .filter(|l| !l.is_empty())
                        .unwrap_or_else(|| pack::slot_label(defs.first(), slot)),
                    bitwarden: n
                        .and_then(|n| n.bitwarden.clone())
                        .or_else(|| store.hint(name, slot).map(str::to_string)),
                });
            }
        }
        let first = defs.first();
        let missing = first
            .and_then(|d| missing_command(d, &places.home))
            .filter(|m| m.path)
            .map(|m| m.command.rsplit('/').next().unwrap_or(&m.command).to_string());
        let login = first.is_some_and(|d| d.is_http() && pack::holes(d).is_empty());
        let item = |kind, agents: Vec<Agent>, off: Vec<Agent>, compare: Vec<Compare>| Item {
            name: name.to_string(),
            kind,
            checked: kind == "add",
            agents,
            off,
            missing_command: missing.clone(),
            moved,
            keys: need.clone(),
            login,
            compare,
        };
        if !add.is_empty() {
            items.push(item("add", add, off, Vec::new()));
        }
        if !replace.is_empty() {
            items.push(item("replace", replace, Vec::new(), compare));
        }
    }
    let bases = readings.iter().filter(|r| r.ok()).map(|r| (r.agent, r.base.clone())).collect();
    let known = pack::secret_values(&readings);
    Plan {
        pack: id.to_string(),
        mac: pack.manifest.mac.clone(),
        at: pack.manifest.at,
        items,
        same,
        keys,
        skipped,
        bases,
        skills: plan_skills(tools, id, pack),
        plugins: plan_plugins(tools, pack),
        files: plan_files(places, app, tools, pack, &known),
    }
}

// ===== Skills =====

/// The Mac has a skill of that name: in the shared folder, an agent's own,
/// Codex's own, or anything at its place in the shared folder.
fn has_skill(places: &SkillPlaces, look: &skills::Look, name: &str) -> bool {
    look.shared.contains(name)
        || !look.own(name).is_empty()
        || look.system.iter().any(|s| s == name)
        || std::fs::symlink_metadata(places.shared.join(name)).is_ok()
}

fn plan_skills(tools: &Tools, id: &str, pack: &Pack) -> Option<SkillPlan> {
    let list = pack.skills.as_ref()?;
    let places = &tools.skills;
    let look = skills::read(places);
    let mut out = SkillPlan { items: Vec::new(), same: Vec::new(), left_out: Vec::new() };
    for s in &list.skills {
        if has_skill(places, &look, &s.name) {
            out.same.push(s.name.clone());
            continue;
        }
        let on: Vec<Agent> = s.on.iter().copied().filter(|a| places.present(*a)).collect();
        let item = |kind, blocked: Option<&'static str>| SkillItem {
            name: s.name.clone(),
            kind,
            repo: s.repo.clone(),
            on: on.clone(),
            checked: blocked.is_none(),
            blocked,
        };
        if !s.hand {
            out.items.push(item("github", tools.npx.is_none().then_some("noNpx")));
        } else if s.copied {
            let dir = pack::skill_dir(&tools.root, id, &s.name);
            if pack::in_cloud_under(&dir) {
                out.items.push(item("hand", Some("cloud")));
            } else if dir.is_dir() {
                out.items.push(item("hand", None));
            } else {
                out.left_out.push(LeftOut { name: s.name.clone(), why: "unreadable".into() });
            }
        } else {
            let why = s.left_out.clone().unwrap_or_else(|| "old".into());
            out.left_out.push(LeftOut { name: s.name.clone(), why });
        }
    }
    Some(out)
}

// ===== Plugins =====

fn plan_plugins(tools: &Tools, pack: &Pack) -> Option<PluginPlan> {
    let list = pack.plugins.as_ref()?;
    let mut out = PluginPlan { items: Vec::new(), same: Vec::new(), skipped: Vec::new() };
    for side in list.agents.iter().filter(|s| s.state == "ok" || s.state == "kept") {
        let wanted: Vec<&plugins::Plugin> =
            side.plugins.iter().filter(|p| !plugins::builtin(side, p)).collect();
        if wanted.is_empty() {
            continue;
        }
        let here = tools.plugins_here(side.agent);
        if !here.ok() {
            let reason = if here.state == "noCli" { "noCli" } else { "failed" };
            out.skipped.push(Skipped { agent: side.agent, reason, name: None });
            continue;
        }
        for p in wanted {
            if here.plugins.iter().any(|h| h.id == p.id) {
                out.same.push(p.id.clone());
                continue;
            }
            let name = p.marketplace();
            let (market, mut note) = if here.marketplaces.iter().any(|m| m.name == name) {
                (None, None)
            } else if plugins::market_step(side, name).is_some() {
                (Some(name.to_string()), None)
            } else {
                (None, Some("noSource"))
            };
            if note.is_none() && side.agent == Agent::Codex && !p.enabled {
                note = Some("codexOff");
            }
            out.items.push(PluginItem {
                agent: side.agent,
                id: p.id.clone(),
                enabled: p.enabled,
                market,
                checked: note.is_none(),
                note,
            });
        }
    }
    Some(out)
}

// ===== Instruction files =====

/// Where a pack's instruction file goes on this Mac: its `~/…` path in the
/// home folder (Codex's `AGENTS.md` in this Mac's Codex folder), when it is
/// a text file the app may write, outside the app's own folder.
fn file_target(
    home: &Path,
    codex_home: &Path,
    app: &AppDir,
    f: &instructions::Packed,
) -> Option<PathBuf> {
    let rest = f.path.strip_prefix("~/")?;
    let rel = Path::new(rest);
    if rest.is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let ext = rel.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "md" | "mdc" | "txt") {
        return None;
    }
    let target = if f.agent == Some(Agent::Codex) && rest == ".codex/AGENTS.md" {
        codex_home.join("AGENTS.md")
    } else {
        home.join(rel)
    };
    (!target.starts_with(app.path())).then_some(target)
}

/// The agent's own folder is on this Mac.
fn agent_here(home: &Path, codex_home: &Path, agent: Option<Agent>) -> bool {
    match agent {
        Some(Agent::Claude) => home.join(".claude").is_dir(),
        Some(Agent::Codex) => codex_home.is_dir(),
        Some(Agent::Agy) => home.join(".gemini").is_dir(),
        None => true,
    }
}

/// A file's text, each line shaped like a key masked.
fn masked_text(text: &str, known: &[String]) -> String {
    text.lines()
        .map(|l| {
            instructions::key_span(l, known)
                .map(|s| instructions::masked(l, s))
                .unwrap_or_else(|| l.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The file as it is here: `None` when not there.
fn read_here(path: &Path) -> Result<Option<String>, String> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(_) => instructions::read_text(path).map(Some),
    }
}

fn plan_files(
    places: &Places,
    app: &AppDir,
    tools: &Tools,
    pack: &Pack,
    known: &[String],
) -> Option<FilePlan> {
    let list = pack.instructions.as_ref()?;
    let home = &places.home;
    let not_keys = instructions::load(app).not_keys;
    let mut out =
        FilePlan { items: Vec::new(), same: Vec::new(), kept: Vec::new(), bases: BTreeMap::new() };
    for f in &list.files {
        let Some(target) = file_target(home, &tools.codex_home, app, f) else {
            out.kept.push(FileKept { path: f.path.clone(), agent: f.agent, why: "path" });
            continue;
        };
        let shown = runner::display_path(&target, home);
        let kept = |why| FileKept { path: shown.clone(), agent: f.agent, why };
        if !agent_here(home, &tools.codex_home, f.agent) {
            out.kept.push(kept("absent"));
            continue;
        }
        let text = match read_here(&target) {
            Ok(t) => t,
            Err(_) => {
                out.kept.push(kept("unreadable"));
                continue;
            }
        };
        let base = text.as_deref().map(|t| write::sha256(t.as_bytes())).unwrap_or_default();
        match text {
            None => {
                out.bases.insert(shown.clone(), base);
                out.items.push(FileItem {
                    path: shown,
                    agent: f.agent,
                    kind: "add",
                    checked: true,
                    changed: 0,
                    diff: Vec::new(),
                });
            }
            Some(t) if t == f.text => out.same.push(shown),
            Some(t) => {
                if !instructions::key_lines(&shown, &t, known, &not_keys).is_empty() {
                    out.kept.push(kept("key"));
                    continue;
                }
                let diff = batch::diff(&masked_text(&t, known), &masked_text(&f.text, known));
                let count = |op| diff.iter().filter(|l| l.op == op).count();
                out.bases.insert(shown.clone(), base);
                out.items.push(FileItem {
                    path: shown,
                    agent: f.agent,
                    kind: "replace",
                    checked: false,
                    changed: count("add").max(count("del")),
                    diff,
                });
            }
        }
    }
    Some(out)
}

/// A server ticked in the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pick {
    pub name: String,
    /// `add` or `replace`.
    pub kind: String,
}

/// A key pasted for the restore. Its value goes to the Keychain and
/// nowhere else.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pasted {
    pub mcp: String,
    pub slot: String,
    pub value: String,
}

impl std::fmt::Debug for Pasted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} = …", self.mcp, self.slot)
    }
}

/// How one ticked server ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub name: String,
    pub kind: String,
    /// Written, on (or off as the pack had it).
    pub done: Vec<Agent>,
    /// Added turned off by the app, and why: `noKey` or `noCommand`.
    pub kept: Vec<Agent>,
    pub why: Option<&'static str>,
    /// Not written: the write stopped before it, or the plan refused it.
    pub failed: Vec<Agent>,
    /// A replace left as it was, and why: `noKey` or `noCommand`.
    pub skipped: Option<&'static str>,
    pub login: bool,
    pub command: Option<String>,
}

/// What else was ticked in the preview (M12): skills by name, plugins,
/// instruction files by path, and those files as the preview read them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct More {
    pub skills: Vec<String>,
    pub plugins: Vec<PluginRef>,
    pub files: Vec<String>,
    pub file_bases: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRef {
    pub agent: Agent,
    pub id: String,
}

/// How one skill, plugin or instruction file ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartResult {
    /// `skill`, `plugin` or `file`.
    pub part: &'static str,
    /// The skill's name, the plugin's `name@marketplace`, the file's `~/…`.
    pub id: String,
    pub agent: Option<Agent>,
    /// A skill's `github` or `hand`, a file's `add` or `replace`.
    pub kind: String,
    pub ok: bool,
    /// Why not: `noNpx`, `npx`, `cloud`, `damaged`, `taken`, `links`,
    /// `noCli`, `login`, `command`, `changed` or `io`.
    pub why: Option<&'static str>,
    /// "Thử lại" may get it done.
    pub retry: bool,
}

impl PartResult {
    fn new(part: &'static str, id: &str, agent: Option<Agent>, kind: &str) -> Self {
        PartResult {
            part,
            id: id.to_string(),
            agent,
            kind: kind.to_string(),
            ok: true,
            why: None,
            retry: false,
        }
    }

    fn fail(mut self, why: &'static str, retry: bool) -> Self {
        self.ok = false;
        self.why = Some(why);
        self.retry = retry;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub at: i64,
    pub items: Vec<ItemResult>,
    pub failed: Option<batch::Failed>,
    /// The files written, with their backups.
    pub written: Vec<batch::Done>,
    pub keys: usize,
    /// The skills, plugins and instruction files, each on its own.
    pub parts: Vec<PartResult>,
    pub can_undo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum RunError {
    /// A file is not what the preview read: nothing written.
    Changed(Vec<Agent>),
    /// A pasted key did not go to the Keychain: nothing written.
    Keychain(String),
}

/// A server the app keeps off for an agent after a restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptRef {
    pub name: String,
    pub agent: Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotRef {
    pub mcp: String,
    pub slot: String,
}

/// A skill the restore installed: its tree id just after, so that the undo
/// leaves one changed since alone unless the owner says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDone {
    pub name: String,
    /// `github` or `hand`.
    pub kind: String,
    pub hash: String,
}

/// An instruction file the restore wrote: what it wrote, and the old
/// file's backup (none when there was no file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDone {
    /// `~/…`.
    pub path: String,
    pub sha256: String,
    pub backup: Option<String>,
}

/// The last restore, as kept to undo it: names only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub at: i64,
    pub pack: String,
    pub mac: String,
    pub undo: Option<Undo>,
    #[serde(default)]
    pub kept: Vec<KeptRef>,
    /// Keychain items that were not there before the restore.
    #[serde(default)]
    pub created: Vec<SlotRef>,
    #[serde(default)]
    pub skills: Vec<SkillDone>,
    /// Plugins the restore installed.
    #[serde(default)]
    pub plugins: Vec<PluginRef>,
    /// Claude Code's marketplaces the restore added.
    #[serde(default)]
    pub markets: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileDone>,
}

impl Record {
    fn other_parts(&self) -> bool {
        !self.skills.is_empty() || !self.plugins.is_empty() || !self.files.is_empty()
    }
}

pub fn last(app: &AppDir) -> Option<Record> {
    app.read_to_string(&app.path().join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok())
}

fn save_record(app: &AppDir, record: &Record) {
    if let Ok(text) = serde_json::to_string_pretty(record) {
        let _ = app.write_private(&app.path().join(FILE), &text);
    }
}

fn forget_record(app: &AppDir) {
    let _ = app.remove(&app.path().join(FILE));
}

/// Restores what was ticked in the preview of `pack`: the servers in
/// `picks`, the rest in `more`. `bases` are the agents' MCP files as the
/// preview read them: one changed since stops the restore before anything
/// is written. Pasted keys go to the Keychain first; a server still short
/// of one, or of its command, is kept off by the app. Then the skills,
/// plugins and instruction files, each on its own.
#[allow(clippy::too_many_arguments)]
pub fn run(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    tools: &Tools,
    id: &str,
    pack: &Pack,
    picks: &[Pick],
    more: &More,
    pasted: &[Pasted],
    bases: &BTreeMap<Agent, String>,
    now: OffsetDateTime,
) -> Result<Done, RunError> {
    let at = now.unix_timestamp();
    let fresh = plan(places, app, secrets, tools, id, pack);
    let picked: Vec<&Item> = fresh
        .items
        .iter()
        .filter(|i| picks.iter().any(|p| p.name == i.name && p.kind == i.kind))
        .collect();
    let involved: BTreeSet<Agent> = picked.iter().flat_map(|i| i.agents.iter().copied()).collect();
    let changed: Vec<Agent> =
        involved.iter().copied().filter(|a| bases.get(a) != fresh.bases.get(a)).collect();
    if !changed.is_empty() {
        return Err(RunError::Changed(changed));
    }

    // Every secret of the servers restored, and whether its Keychain item is
    // there before: the undo removes the items the restore made.
    let mut slots: BTreeSet<(String, String)> = BTreeSet::new();
    for item in &picked {
        let server = pack.mcp.servers.iter().find(|s| s.name == item.name);
        for e in server.into_iter().flat_map(|s| &s.entries) {
            for slot in pack::holes(&e.def) {
                slots.insert((item.name.clone(), slot));
            }
        }
    }
    let before: BTreeSet<(String, String)> =
        slots.iter().filter(|(m, s)| secrets.has(m, s).unwrap_or(true)).cloned().collect();

    let wanted: BTreeSet<(&str, &str)> = picked
        .iter()
        .flat_map(|i| i.keys.iter().map(move |k| (i.name.as_str(), k.as_str())))
        .collect();
    let mut saved: Vec<SlotRef> = Vec::new();
    for p in pasted {
        if p.value.trim().is_empty() || !wanted.contains(&(p.mcp.as_str(), p.slot.as_str())) {
            continue;
        }
        if let Err(e) = secrets.save(&p.mcp, &p.slot, p.value.trim()) {
            for s in &saved {
                let _ = secrets.delete(&s.mcp, &s.slot);
            }
            return Err(RunError::Keychain(e.to_string()));
        }
        saved.push(SlotRef { mcp: p.mcp.clone(), slot: p.slot.clone() });
    }
    let mut store = kho::load(app);
    let mut hinted = false;
    for n in &pack.secrets.secrets {
        if let Some(b) = n.bitwarden.as_deref().filter(|_| store.hint(&n.mcp, &n.slot).is_none()) {
            store.set_hint(&n.mcp, &n.slot, b);
            hinted = true;
        }
    }
    if hinted {
        let _ = kho::save(app, &store);
    }

    let readings = super::read_all(places);
    let have = Have { readings: &readings, secrets };
    let mut changes: Vec<Change> = Vec::new();
    let mut keep: Vec<(String, Agent, Def, &'static str)> = Vec::new();
    let mut results: Vec<ItemResult> = Vec::new();
    for item in &picked {
        let mut result = ItemResult {
            name: item.name.clone(),
            kind: item.kind.to_string(),
            done: Vec::new(),
            kept: Vec::new(),
            why: None,
            failed: Vec::new(),
            skipped: None,
            login: item.login,
            command: item.missing_command.clone(),
        };
        let Some(server) = pack.mcp.servers.iter().find(|s| s.name == item.name) else {
            continue;
        };
        let keychain: Vec<String> =
            saved.iter().filter(|s| s.mcp == item.name).map(|s| s.slot.clone()).collect();
        for agent in &item.agents {
            let Some(entry) = server.entries.iter().find(|e| e.agent == *agent) else { continue };
            let (def, _) = pack::localize(&entry.def, &places.home);
            let no_key = pack::holes(&def).iter().any(|s| !have.exists(&item.name, s));
            let no_cmd = missing_command(&def, &places.home).is_some_and(|m| m.path);
            let why = if no_key {
                Some("noKey")
            } else if no_cmd {
                Some("noCommand")
            } else {
                None
            };
            if item.kind == "replace" {
                if let Some(w) = why {
                    result.skipped = Some(w);
                    continue;
                }
                let to =
                    To::Def { def, from: Some(*agent), keychain: keychain.clone(), off: false };
                changes.push(Change { name: item.name.clone(), agent: *agent, to });
            } else if why.is_some() || (!entry.enabled && *agent == Agent::Claude) {
                result.why = result.why.or(why);
                keep.push((item.name.clone(), *agent, def, why.unwrap_or("packOff")));
            } else {
                let to =
                    To::Def { def, from: None, keychain: keychain.clone(), off: !entry.enabled };
                changes.push(Change { name: item.name.clone(), agent: *agent, to });
            }
        }
        results.push(result);
    }

    let mut out = None;
    if !changes.is_empty() {
        let p = batch::preview(places, app, secrets, &changes);
        for r in &p.refused {
            if let Some(res) = results.iter_mut().find(|x| x.name == r.name) {
                res.failed.push(r.agent);
            }
        }
        if !p.changes.is_empty() {
            out = Some(batch::write(places, app, secrets, &p.changes, bases, now));
        }
        let done: BTreeSet<Agent> =
            out.iter().flat_map(|o| o.done.iter().map(|d| d.agent)).collect();
        for c in &p.changes {
            if let Some(res) = results.iter_mut().find(|x| x.name == c.name) {
                if done.contains(&c.agent) {
                    res.done.push(c.agent);
                } else {
                    res.failed.push(c.agent);
                }
            }
        }
    }
    let nothing = out.as_ref().is_some_and(|o| o.done.is_empty() && o.failed.is_some());
    let mut kept_refs = Vec::new();
    if nothing {
        // No server written: the keys pasted for them go too, and none is
        // kept off. The other parts still run.
        for s in &saved {
            let _ = secrets.delete(&s.mcp, &s.slot);
        }
        saved.clear();
    } else {
        let mut store = kho::load(app);
        for (name, agent, def, _) in &keep {
            let slots = pack::holes(def);
            let def = mask_holes(def);
            if *agent == Agent::Claude {
                let kept = Kept { def, slots, extra: Map::new(), agents: Vec::new(), at };
                store.parked.insert(name.clone(), kept);
            } else {
                let k = store.waiting.entry(name.clone()).or_insert_with(|| Kept {
                    def,
                    slots,
                    extra: Map::new(),
                    agents: Vec::new(),
                    at,
                });
                if !k.agents.contains(agent) {
                    k.agents.push(*agent);
                    k.agents.sort();
                }
            }
            if let Some(res) = results.iter_mut().find(|x| &x.name == name) {
                res.kept.push(*agent);
            }
            kept_refs.push(KeptRef { name: name.clone(), agent: *agent });
        }
        if !keep.is_empty() {
            let _ = kho::save(app, &store);
        }
    }
    let created: Vec<SlotRef> = if nothing {
        Vec::new()
    } else {
        slots
            .iter()
            .filter(|s| !before.contains(s))
            .map(|(m, s)| SlotRef { mcp: m.clone(), slot: s.clone() })
            .collect()
    };
    let mcp_failed = out.as_ref().and_then(|o| o.failed.clone());
    let written = out.as_ref().map(|o| o.done.clone()).unwrap_or_default();
    let mut undo = out.and_then(|o| o.undo);

    let mut record = Record {
        at,
        pack: id.to_string(),
        mac: pack.manifest.mac.clone(),
        undo: None,
        kept: kept_refs,
        created,
        skills: Vec::new(),
        plugins: Vec::new(),
        markets: Vec::new(),
        files: Vec::new(),
    };
    let mut parts = Vec::new();
    if let Some(plan) = &fresh.skills {
        let items: Vec<&SkillItem> =
            plan.items.iter().filter(|i| more.skills.contains(&i.name)).collect();
        parts.extend(restore_skills(tools, app, id, pack, &items, false, &mut record, now));
    }
    if let Some(plan) = &fresh.plugins {
        let items: Vec<&PluginItem> = plan
            .items
            .iter()
            .filter(|i| more.plugins.iter().any(|p| p.agent == i.agent && p.id == i.id))
            .collect();
        parts.extend(restore_plugins(tools, pack, &items, &mut record));
    }
    if let Some(plan) = &fresh.files {
        let items: Vec<&FileItem> =
            plan.items.iter().filter(|i| more.files.contains(&i.path)).collect();
        parts.extend(restore_files(
            places,
            app,
            tools,
            pack,
            &items,
            &more.file_bases,
            &mut record,
            now,
        ));
    }
    // The skills' boxes and the plugins may have written Codex's file after
    // the servers: the restore's own writes (one write runs at a time), so
    // the undo of the servers starts from the files as they are now.
    if let Some(u) = undo.as_mut() {
        let now_read = super::read_all(places);
        for (agent, base) in u.bases.iter_mut() {
            if let Some(r) = now_read.iter().find(|r| r.agent == *agent && r.ok()) {
                *base = r.base.clone();
            }
        }
    }
    record.undo = undo;
    let can_undo = record.undo.is_some()
        || !record.kept.is_empty()
        || !saved.is_empty()
        || record.other_parts();
    if can_undo {
        save_record(app, &record);
    }
    Ok(Done { at, items: results, failed: mcp_failed, written, keys: saved.len(), parts, can_undo })
}

/// A definition's holes as the store keeps them: [`REDACTED`].
fn mask_holes(def: &Def) -> Def {
    let mut out = def.clone();
    for slot in pack::holes(def) {
        out.set(&slot, REDACTED);
    }
    out
}

// ===== The skills, plugins and files of a run =====

/// Installs or copies each skill of `items`, then sets every agent's box
/// as the pack had it, in one checked batch. With `again`, a skill already
/// in the shared folder (installed by the run that failed later) only has
/// its boxes set.
#[allow(clippy::too_many_arguments)]
fn restore_skills(
    tools: &Tools,
    app: &AppDir,
    id: &str,
    pack: &Pack,
    items: &[&SkillItem],
    again: bool,
    record: &mut Record,
    now: OffsetDateTime,
) -> Vec<PartResult> {
    let places = &tools.skills;
    let look = skills::read(places);
    let mut results: Vec<PartResult> = Vec::new();
    let mut ready: Vec<&SkillItem> = Vec::new();
    let mut fresh: Vec<&SkillItem> = Vec::new();
    for item in items {
        let r = PartResult::new("skill", &item.name, None, item.kind);
        let installed = again && look.shared.contains(&item.name);
        if installed {
            ready.push(item);
        } else if has_skill(places, &look, &item.name) {
            results.push(r.fail("taken", false));
        } else {
            fresh.push(item);
        }
    }
    // From GitHub: one `npx skills add` for each repository.
    let mut repos: BTreeMap<&str, Vec<&SkillItem>> = BTreeMap::new();
    for item in fresh.iter().filter(|i| i.kind == "github") {
        match item.repo.as_deref() {
            Some(repo) => repos.entry(repo).or_default().push(item),
            None => results
                .push(PartResult::new("skill", &item.name, None, "github").fail("npx", false)),
        }
    }
    for (repo, list) in repos {
        let names: Vec<String> = list.iter().map(|i| i.name.clone()).collect();
        let error = match &tools.npx {
            None => Some(NpxError::NoNpx),
            Some(npx) => npx.add(repo, &names).err(),
        };
        let after = skills::read(places);
        for item in list {
            if after.shared.contains(&item.name) {
                ready.push(item);
            } else {
                let why = if matches!(error, Some(NpxError::NoNpx)) { "noNpx" } else { "npx" };
                let retry = why == "npx";
                results.push(PartResult::new("skill", &item.name, None, "github").fail(why, retry));
            }
        }
    }
    // Installed by hand: copied from the pack.
    for item in fresh.iter().filter(|i| i.kind == "hand") {
        let packed = pack.skills.iter().flat_map(|l| &l.skills).find(|s| s.name == item.name);
        match copy_from_pack(tools, id, packed) {
            Ok(()) => ready.push(item),
            Err(why) => results.push(
                PartResult::new("skill", &item.name, None, "hand")
                    .fail(why, why == "cloud" || why == "io"),
            ),
        }
    }
    if ready.is_empty() {
        return results;
    }
    // Each agent's box as in the pack, for the agents here.
    let look = skills::read(places);
    let mut changes = Vec::new();
    for item in &ready {
        for agent in Agent::ALL {
            let r = look.reading(agent);
            if !r.present || !r.writable() {
                continue;
            }
            let on = item.on.contains(&agent);
            if look.on(places, &item.name, agent) != on {
                changes.push(skills::Change { name: item.name.clone(), agent, on });
            }
        }
    }
    let mut failed: BTreeSet<String> = BTreeSet::new();
    if !changes.is_empty() {
        let p = skills::preview(places, &changes);
        for r in &p.refused {
            if r.reason != "nothing" {
                failed.insert(r.name.clone());
            }
        }
        let outcome = skills::write(places, app, &p.changes, &p.bases, now);
        let done: BTreeSet<Agent> = outcome.done.iter().map(|d| d.agent).collect();
        if outcome.failed.is_some() {
            for c in p.changes.iter().filter(|c| !done.contains(&c.agent)) {
                failed.insert(c.name.clone());
            }
        }
    }
    for item in ready {
        let hash = skill_updates::tree_hash(&places.shared.join(&item.name)).unwrap_or_default();
        if !record.skills.iter().any(|s| s.name == item.name) {
            record.skills.push(SkillDone {
                name: item.name.clone(),
                kind: item.kind.to_string(),
                hash,
            });
        }
        let r = PartResult::new("skill", &item.name, None, item.kind);
        results.push(if failed.contains(&item.name) { r.fail("links", true) } else { r });
    }
    results
}

/// Copies a skill installed by hand from the pack into the shared folder:
/// aside first, its tree id checked against the pack's, then put in place.
fn copy_from_pack(
    tools: &Tools,
    id: &str,
    packed: Option<&pack::PackedSkill>,
) -> Result<(), &'static str> {
    let s = packed.filter(|s| s.copied).ok_or("damaged")?;
    let from = pack::skill_dir(&tools.root, id, &s.name);
    if pack::in_cloud_under(&from) {
        return Err("cloud");
    }
    let files = pack::check_skill(&from, &[]).map_err(|_| "damaged")?;
    let files: Vec<(String, bool)> =
        files.into_iter().map(|(rel, _)| (rel.clone(), s.exec.contains(&rel))).collect();
    let shared = &tools.skills.shared;
    std::fs::create_dir_all(shared).map_err(|_| "io")?;
    let dest = shared.join(&s.name);
    if std::fs::symlink_metadata(&dest).is_ok() {
        return Err("taken");
    }
    let tmp = shared.join(format!(".{}.qs-restore-{}", s.name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let result = (|| {
        pack::copy_skill(&from, &tmp, &files).map_err(|_| "io")?;
        if skill_updates::tree_hash(&tmp).ok() != s.hash {
            return Err("damaged");
        }
        std::fs::rename(&tmp, &dest).map_err(|_| "io")
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    result
}

/// A command's error said in a word: a login it needs, or the command.
fn plugin_why(error: &str) -> &'static str {
    let e = error.to_lowercase();
    let login =
        ["auth", "login", "log in", "credential", "permission denied", "username", "401", "403"];
    if login.iter().any(|w| e.contains(w)) {
        "login"
    } else {
        "command"
    }
}

/// Installs each plugin of `items` with its agent's own command, its
/// marketplace added first when this Mac has not got it. One that fails
/// leaves the others to run.
fn restore_plugins(
    tools: &Tools,
    pack: &Pack,
    items: &[&PluginItem],
    record: &mut Record,
) -> Vec<PartResult> {
    let mut results = Vec::new();
    for agent in [Agent::Claude, Agent::Codex] {
        let mine: Vec<&&PluginItem> = items.iter().filter(|i| i.agent == agent).collect();
        if mine.is_empty() {
            continue;
        }
        let side = pack.plugins.iter().flat_map(|l| &l.agents).find(|s| s.agent == agent);
        let (Some(program), Some(side)) = (tools.program(agent), side) else {
            for i in mine {
                results
                    .push(PartResult::new("plugin", &i.id, Some(agent), "").fail("noCli", false));
            }
            continue;
        };
        let here = tools.plugins_here(agent);
        // Each marketplace once: its error, when adding it failed.
        let mut markets: BTreeMap<String, Option<String>> = BTreeMap::new();
        for item in mine {
            let r = PartResult::new("plugin", &item.id, Some(agent), "");
            let Some(p) = side.plugins.iter().find(|p| p.id == item.id) else {
                results.push(r.fail("command", false));
                continue;
            };
            let installed = here.plugins.iter().find(|h| h.id == p.id);
            let mut error: Option<String> = None;
            if installed.is_none() {
                let name = p.marketplace().to_string();
                let has_market = here.marketplaces.iter().any(|m| m.name == name);
                if !has_market {
                    if let Some(step) = plugins::market_step(side, &name) {
                        let added = markets
                            .entry(name.clone())
                            .or_insert_with(|| plugins::run_step(program, &step).err());
                        if added.is_none()
                            && agent == Agent::Claude
                            && !record.markets.contains(&name)
                        {
                            record.markets.push(name.clone());
                        }
                        error = added.clone();
                    }
                }
            }
            let steps = plugins::install_steps(agent, p);
            let mut ran_install = false;
            let mut failed: Option<String> = None;
            for step in &steps {
                let is_install = step.args.get(1).is_some_and(|a| a == "install" || a == "add");
                if is_install && installed.is_some() {
                    continue;
                }
                // Turning off one this Mac already has off needs nothing.
                if !is_install && installed.is_some_and(|h| !h.enabled) {
                    continue;
                }
                match plugins::run_step(program, step) {
                    Ok(()) => ran_install |= is_install,
                    Err(e) => {
                        failed = Some(e);
                        break;
                    }
                }
            }
            if ran_install {
                let r#ref = PluginRef { agent, id: p.id.clone() };
                if !record.plugins.contains(&r#ref) {
                    record.plugins.push(r#ref);
                }
            }
            results.push(match failed {
                None => r,
                Some(e) => {
                    // The marketplace's own error says more than the install's.
                    let e = error.unwrap_or(e);
                    r.fail(plugin_why(&e), true)
                }
            });
        }
    }
    results
}

/// The backup's name for the file `shown`: its path, flattened.
fn file_prefix(shown: &str) -> String {
    let flat: String = shown
        .trim_start_matches("~/")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    format!("{flat}.")
}

/// Writes each instruction file of `items` as the pack has it, when it is
/// still what the preview read: the old one kept first.
#[allow(clippy::too_many_arguments)]
fn restore_files(
    places: &Places,
    app: &AppDir,
    tools: &Tools,
    pack: &Pack,
    items: &[&FileItem],
    bases: &BTreeMap<String, String>,
    record: &mut Record,
    now: OffsetDateTime,
) -> Vec<PartResult> {
    let home = &places.home;
    let list = pack.instructions.iter().flat_map(|l| &l.files);
    let targets: Vec<(PathBuf, &instructions::Packed)> =
        list.filter_map(|f| file_target(home, &tools.codex_home, app, f).map(|t| (t, f))).collect();
    let mut results = Vec::new();
    for item in items {
        let r = PartResult::new("file", &item.path, item.agent, item.kind);
        let Some((target, f)) =
            targets.iter().find(|(t, _)| runner::display_path(t, home) == item.path)
        else {
            results.push(r.fail("io", false));
            continue;
        };
        let now_text = match read_here(target) {
            Ok(t) => t,
            Err(_) => {
                results.push(r.fail("io", true));
                continue;
            }
        };
        let now_base = now_text.as_deref().map(|t| write::sha256(t.as_bytes())).unwrap_or_default();
        if bases.get(&item.path) != Some(&now_base) {
            results.push(r.fail("changed", false));
            continue;
        }
        if let Some(dir) = target.parent() {
            if std::fs::create_dir_all(dir).is_err() {
                results.push(r.fail("io", true));
                continue;
            }
        }
        let prefix = file_prefix(&item.path);
        let dir = app.path().join(FILE_BACKUPS);
        let backup = now_text.as_deref().map(|t| write::Backup {
            dir: &dir,
            prefix: &prefix,
            bytes: t.as_bytes(),
        });
        match write::write(target, now_text.as_deref().map(str::as_bytes), &f.text, backup, now) {
            Ok(kept) => {
                record.files.retain(|d| d.path != item.path);
                record.files.push(FileDone {
                    path: item.path.clone(),
                    sha256: write::sha256(f.text.as_bytes()),
                    backup: kept
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
                });
                results.push(r);
            }
            Err(write::WriteError::Changed) => results.push(r.fail("changed", false)),
            Err(_) => results.push(r.fail("io", true)),
        }
    }
    results
}

// ===== Thử lại =====

/// "Thử lại" of one skill, plugin or instruction file that failed in the
/// last restore of `pack`: read again, run again, and kept in the same
/// record so the undo still covers the whole restore.
#[allow(clippy::too_many_arguments)]
pub fn retry(
    places: &Places,
    app: &AppDir,
    tools: &Tools,
    id: &str,
    pack: &Pack,
    part: &str,
    item: &str,
    agent: Option<Agent>,
    now: OffsetDateTime,
) -> PartResult {
    let mut record = last(app).filter(|r| r.pack == id).unwrap_or_else(|| Record {
        at: now.unix_timestamp(),
        pack: id.to_string(),
        mac: pack.manifest.mac.clone(),
        undo: None,
        kept: Vec::new(),
        created: Vec::new(),
        skills: Vec::new(),
        plugins: Vec::new(),
        markets: Vec::new(),
        files: Vec::new(),
    });
    let gone = |part: &'static str| PartResult::new(part, item, agent, "").fail("changed", false);
    let result = match part {
        "skill" => {
            // Planned again; one the failed run installed is in the shared
            // folder now, and only its boxes are left.
            let look = skills::read(&tools.skills);
            let packed = pack.skills.iter().flat_map(|l| &l.skills).find(|s| s.name == item);
            match packed {
                Some(s) if look.shared.contains(item) || !has_skill(&tools.skills, &look, item) => {
                    let one = SkillItem {
                        name: s.name.clone(),
                        kind: if s.hand { "hand" } else { "github" },
                        repo: s.repo.clone(),
                        on: s.on.iter().copied().filter(|a| tools.skills.present(*a)).collect(),
                        checked: true,
                        blocked: None,
                    };
                    restore_skills(tools, app, id, pack, &[&one], true, &mut record, now)
                        .pop()
                        .unwrap_or_else(|| gone("skill"))
                }
                _ => gone("skill"),
            }
        }
        "plugin" => {
            let side = pack.plugins.iter().flat_map(|l| &l.agents).find(|s| Some(s.agent) == agent);
            match side.and_then(|s| s.plugins.iter().find(|p| p.id == item)) {
                Some(p) => {
                    let one = PluginItem {
                        agent: agent.unwrap_or(Agent::Claude),
                        id: p.id.clone(),
                        enabled: p.enabled,
                        market: None,
                        checked: true,
                        note: None,
                    };
                    restore_plugins(tools, pack, &[&one], &mut record)
                        .pop()
                        .unwrap_or_else(|| gone("plugin"))
                }
                None => gone("plugin"),
            }
        }
        "file" => {
            let readings = super::read_all(places);
            let fresh = plan_files(places, app, tools, pack, &pack::secret_values(&readings));
            match fresh
                .as_ref()
                .and_then(|f| f.items.iter().find(|i| i.path == item).map(|i| (f, i)))
            {
                Some((f, i)) => {
                    restore_files(places, app, tools, pack, &[i], &f.bases, &mut record, now)
                        .pop()
                        .unwrap_or_else(|| gone("file"))
                }
                None => gone("file"),
            }
        }
        _ => gone("file"),
    };
    if record.undo.is_some()
        || !record.kept.is_empty()
        || !record.created.is_empty()
        || record.other_parts()
    {
        save_record(app, &record);
    }
    result
}

// ===== Hoàn tác =====

/// What changed since the last restore wrote it: the agents' MCP files,
/// the instruction files (`~/…`) and the skills (by name).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Changed {
    pub agents: Vec<Agent>,
    pub files: Vec<String>,
    pub skills: Vec<String>,
}

impl Changed {
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty() && self.files.is_empty() && self.skills.is_empty()
    }
}

fn file_now(home: &Path, path: &str) -> Option<Vec<u8>> {
    std::fs::read(runner::expand_path(path, home)).ok()
}

/// What changed since the last restore wrote it.
pub fn changed_since(places: &Places, tools: &Tools, record: &Record) -> Changed {
    let mut out = Changed::default();
    if let Some(undo) = &record.undo {
        let readings = super::read_all(places);
        out.agents = undo
            .bases
            .iter()
            .filter(|(a, base)| {
                readings.iter().find(|r| r.agent == **a).map(|r| &r.base) != Some(*base)
            })
            .map(|(a, _)| *a)
            .collect();
    }
    for f in &record.files {
        if let Some(bytes) = file_now(&places.home, &f.path) {
            if write::sha256(&bytes) != f.sha256 {
                out.files.push(f.path.clone());
            }
        }
    }
    for s in &record.skills {
        let dir = tools.skills.shared.join(&s.name);
        if dir.exists() && skill_updates::tree_hash(&dir).ok().as_deref() != Some(s.hash.as_str()) {
            out.skills.push(s.name.clone());
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum UndoError {
    /// No restore to undo.
    Nothing,
    /// These changed after the restore: the owner says first.
    Changed(Changed),
    /// The files could not be put back.
    Failed(batch::Failed),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Undone {
    pub outcome: Option<batch::Outcome>,
    pub kept: usize,
    pub keys: usize,
    /// Skills removed, plugins uninstalled, instruction files put back.
    pub skills: usize,
    pub plugins: usize,
    pub files: usize,
    /// What stays: changed since (and the owner said go on), or its undo
    /// did not run.
    pub left: usize,
}

/// Undoes the last restore: its batch put back, the servers it kept off
/// dropped, the Keychain items it made removed, the skills and plugins it
/// installed removed, the instruction files it wrote put back. What
/// changed since is touched only with `force`: then the restored servers'
/// entries go, and a skill or file changed since stays as it is.
pub fn undo(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    tools: &Tools,
    force: bool,
    now: OffsetDateTime,
) -> Result<Undone, UndoError> {
    let record = last(app).ok_or(UndoError::Nothing)?;
    let changed = changed_since(places, tools, &record);
    if !changed.is_empty() && !force {
        return Err(UndoError::Changed(changed));
    }
    let mut outcome = None;
    if let Some(u) = &record.undo {
        // An entry already as it was before (removed by hand since) needs
        // nothing.
        let p = batch::preview(places, app, secrets, &u.changes);
        if !p.changes.is_empty() {
            let bases = if changed.agents.is_empty() { &u.bases } else { &p.bases };
            let out = batch::write(places, app, secrets, &p.changes, bases, now);
            if out.done.is_empty() {
                if let Some(f) = out.failed {
                    return Err(UndoError::Failed(f));
                }
            }
            outcome = Some(out);
        }
    }
    let mut store = kho::load(app);
    for k in &record.kept {
        if k.agent == Agent::Claude {
            store.parked.remove(&k.name);
        } else {
            store.unwait(&k.name, k.agent);
        }
    }
    if !record.kept.is_empty() {
        let _ = kho::save(app, &store);
    }
    let mut keys = 0;
    for s in &record.created {
        if secrets.delete(&s.mcp, &s.slot).is_ok() {
            keys += 1;
        }
    }
    let mut done =
        Undone { outcome, kept: record.kept.len(), keys, skills: 0, plugins: 0, files: 0, left: 0 };
    undo_skills(tools, app, &record, &changed, &mut done, now);
    undo_plugins(tools, &record, &mut done);
    undo_files(places, app, &record, &changed, &mut done, now);
    forget_record(app);
    Ok(done)
}

fn undo_skills(
    tools: &Tools,
    app: &AppDir,
    record: &Record,
    changed: &Changed,
    done: &mut Undone,
    now: OffsetDateTime,
) {
    let places = &tools.skills;
    let look = skills::read(places);
    let mut through_npx = Vec::new();
    let mut by_hand = Vec::new();
    for s in &record.skills {
        if !look.shared.contains(&s.name) {
            continue;
        }
        if changed.skills.contains(&s.name) {
            done.left += 1;
        } else if s.kind == "github" && look.locked(&s.name).is_some() {
            through_npx.push(s.name.clone());
        } else {
            by_hand.push(s.name.clone());
        }
    }
    if !through_npx.is_empty() {
        match &tools.npx {
            Some(npx) => {
                let r = skill_ops::remove(places, app, npx, &through_npx, now);
                done.skills += r.names.len();
                done.left += through_npx.len() - r.names.len();
            }
            None => done.left += through_npx.len(),
        }
    }
    let mut removed = Vec::new();
    for name in by_hand {
        let dir = places.shared.join(&name);
        // A copy first, as for any skill folder the app removes.
        if skills::keep_folder(app, &name, &dir, now).is_ok()
            && std::fs::remove_dir_all(&dir).is_ok()
        {
            removed.push(name);
        } else {
            done.left += 1;
        }
    }
    if !removed.is_empty() {
        done.skills += removed.len();
        skill_ops::tidy(places, app, &removed, now);
    }
}

fn undo_plugins(tools: &Tools, record: &Record, done: &mut Undone) {
    for p in &record.plugins {
        let ok = tools.program(p.agent).is_some_and(|prog| {
            plugins::run_step(prog, &plugins::uninstall_step(p.agent, &p.id)).is_ok()
        });
        if ok {
            done.plugins += 1;
        } else {
            done.left += 1;
        }
    }
    // A marketplace the restore added goes when no plugin from it is left:
    // removing one also removes the plugins installed from it.
    if let (Some(claude), false) = (tools.claude.as_deref(), record.markets.is_empty()) {
        let here = tools.plugins_here(Agent::Claude);
        if here.ok() {
            for m in &record.markets {
                if !here.plugins.iter().any(|p| p.marketplace() == m) {
                    let _ = plugins::run_step(claude, &plugins::market_remove_step(m));
                }
            }
        }
    }
}

fn undo_files(
    places: &Places,
    app: &AppDir,
    record: &Record,
    changed: &Changed,
    done: &mut Undone,
    now: OffsetDateTime,
) {
    for f in &record.files {
        let target = runner::expand_path(&f.path, &places.home);
        let Some(bytes) = file_now(&places.home, &f.path) else { continue };
        if changed.files.contains(&f.path) {
            done.left += 1;
            continue;
        }
        let ok = match &f.backup {
            None => std::fs::remove_file(&target).is_ok(),
            Some(name) => {
                let old = std::fs::read(app.path().join(FILE_BACKUPS).join(name))
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok());
                old.is_some_and(|old| write::write(&target, Some(&bytes), &old, None, now).is_ok())
            }
        };
        if ok {
            done.files += 1;
        } else {
            done.left += 1;
        }
    }
}
