// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Copying the agents' setup to or from another Mac over SSH (M13, D): the
//! MCP servers, skills, plugins and instruction files, item by item, after
//! a comparison of the two Macs.
//!
//! - Reading the other Mac: its own Quota Station prints an [`Inventory`],
//!   what a backup pack holds (names, definitions with every secret
//!   [`super::def::REDACTED`], skill sources and tree ids, plugin lists,
//!   instruction files with no line shaped like a key) and the hashes of the
//!   files it would write. Never a key's value.
//! - [`diff`] compares two inventories: what only one Mac has, what differs
//!   in its configuration, its version or its text. A command at another
//!   path on the other Mac is not a difference. Nothing is chosen for the
//!   owner when both Macs have an item: neither side wins on its own.
//! - Copying sends a [`Payload`] with only the items ticked. The Mac that
//!   receives it writes through its own Quota Station: [`apply`] is the
//!   restore of a pack (`restore::run`), so the hash check, the backups
//!   taken first and the undo are the same as for a write made there. A
//!   skill from GitHub that is newer on the sending Mac is brought to its
//!   newest version with `npx skills update`, its folder kept first.
//! - Keys go only when the owner ticks "Gửi cả khoá qua SSH": then their
//!   values travel inside the SSH connection, on stdin, and the receiving
//!   Mac puts them in its Keychain. Never on a command line, never on disk.
//! - Codex's provider, logins and accounts are never part of it.
//!
//! On the other Mac the app is run by one of the fixed [`script`]s, the
//! same for every call but for the word after `agents`; whatever a call
//! carries goes on stdin. A Mac whose app does not say in its `Info.plist`
//! that it speaks [`PROTOCOL`] (version 1 of the app) is never run, so
//! never written to. A write there runs on its own ([`Job`]): SSH waits at
//! most [`peers::SSH_TIMEOUT`] for each call, and the app asks again until
//! the job is done.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use time::OffsetDateTime;

use super::batch::{self, Line};
use super::def::{Def, MASK};
use super::pack::{self, InstructionList, McpList, Pack, PluginList, SecretsNeeded, SkillList};
use super::restore::{self, More, Pasted, Pick, PluginRef, Record, Tools};
use super::secrets::Secrets;
use super::{instructions, skills, write, Agent, Have, Places, Reading};
use crate::app_dir::AppDir;
use crate::peers;
use crate::runner;

/// What both Macs must speak. The other Mac's app says its own in its
/// `Info.plist`, under [`PLIST_KEY`].
pub const PROTOCOL: u32 = 1;
pub const PLIST_KEY: &str = "QuotaStationAgents";
/// Where the other Mac's app is looked for, in this order.
pub const APP_FOLDERS: [&str; 2] = ["/Applications", "$HOME/Applications"];
/// The app's own refusal: its stdout says why, as `{"error": "…"}`.
pub const REFUSED: i32 = 7;
/// The script's exits: no app in the folders, or one of version 1.
const NO_APP: i32 = 5;
const OLD: i32 = 6;
/// The pack a copy restores from, in the receiving Mac's app folder.
pub const INCOMING: &str = "agents/incoming";
/// The write running on its own, and how it ended.
pub const JOB: &str = "agents/copy-job.json";
/// How long the app keeps asking about a write on the other Mac.
pub const WAIT_LIMIT: Duration = Duration::from_secs(15 * 60);

/// What a call asks the other Mac's app: `quota-station agents <word>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    /// Prints its [`Inventory`].
    Read,
    /// Takes an [`ExportAsk`] on stdin, prints the [`Payload`] of what was
    /// asked: a copy to this Mac.
    Export,
    /// Takes a [`JobInput`] on stdin and starts it: a copy from this Mac,
    /// or its undo.
    Start,
    /// Prints the [`Job`] (`null` when there was none).
    Status,
}

impl Word {
    pub fn as_str(self) -> &'static str {
        match self {
            Word::Read => "read",
            Word::Export => "export",
            Word::Start => "start",
            Word::Status => "status",
        }
    }
}

/// The script SSH runs on the other Mac for `word`: find the app in
/// `folders`, check its `Info.plist` says it speaks [`PROTOCOL`], run it.
/// Built from constants only, never from what is copied.
pub fn script(word: Word, folders: &[&str]) -> String {
    let dirs: Vec<String> = folders.iter().map(|f| format!("\"{f}\"")).collect();
    format!(
        concat!(
            "a=; for d in {dirs}; do if [ -d \"$d/Quota Station.app\" ]; then a=\"$d/Quota Station.app\"; break; fi; done; ",
            "[ -n \"$a\" ] || exit {no_app}; ",
            "v=$(/usr/bin/plutil -extract {key} raw -o - \"$a/Contents/Info.plist\" 2>/dev/null) || exit {old}; ",
            "[ \"$v\" -ge {protocol} ] 2>/dev/null || exit {old}; ",
            "exec \"$a/Contents/MacOS/quota-station\" agents {word}"
        ),
        dirs = dirs.join(" "),
        no_app = NO_APP,
        key = PLIST_KEY,
        old = OLD,
        protocol = PROTOCOL,
        word = word.as_str(),
    )
}

// ===== What a Mac has =====

/// A skill's version from GitHub: its tree id and when it was installed or
/// last updated, as `.skill-lock.json` has them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVersion {
    #[serde(default)]
    pub hash: Option<String>,
    /// RFC 3339.
    #[serde(default)]
    pub at: Option<String>,
}

/// An instruction file, by its `~/…` path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    #[serde(default)]
    pub agent: Option<Agent>,
    /// SHA-256 of its text: what a write there checks.
    pub sha256: String,
    /// Its text, when it may leave the Mac: none of its lines is shaped
    /// like a key and the owner keeps it in the backups.
    #[serde(default)]
    pub text: Option<String>,
    /// Why it stays where it is: `key`, `excluded` or `unreadable`.
    #[serde(default)]
    pub held: Option<String>,
}

/// What a Mac tells the other one: no secret, no key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub protocol: u32,
    pub app: String,
    pub mac: String,
    pub at: i64,
    /// The agents on that Mac.
    pub agents: Vec<Agent>,
    pub mcp: McpList,
    pub secrets: SecretsNeeded,
    pub skills: SkillList,
    #[serde(default)]
    pub versions: BTreeMap<String, SkillVersion>,
    pub plugins: PluginList,
    pub files: Vec<FileInfo>,
    /// Each agent's MCP file as read: what a write there checks.
    pub bases: BTreeMap<Agent, String>,
}

impl Inventory {
    /// The file hashes a write there checks: `~/…` → SHA-256.
    pub fn file_bases(&self) -> BTreeMap<String, String> {
        self.files.iter().map(|f| (f.path.clone(), f.sha256.clone())).collect()
    }
}

/// This Mac now, as the app reads it for a pack: the pack with the folders
/// of the skills installed by hand, the MCP readings (which hold secrets:
/// in memory only), the instruction files and the skills' versions.
pub struct Here {
    pub pack: Pack,
    pub readings: Vec<Reading>,
    pub files: Vec<FileInfo>,
    pub versions: BTreeMap<String, SkillVersion>,
    pub agents: Vec<Agent>,
}

impl Here {
    pub fn inventory(&self) -> Inventory {
        let m = &self.pack.manifest;
        Inventory {
            protocol: PROTOCOL,
            app: m.app.clone(),
            mac: m.mac.clone(),
            at: m.at,
            agents: self.agents.clone(),
            mcp: self.pack.mcp.clone(),
            secrets: self.pack.secrets.clone(),
            skills: self.pack.skills.clone().unwrap_or_default(),
            versions: self.versions.clone(),
            plugins: self.pack.plugins.clone().unwrap_or_default(),
            files: self.files.clone(),
            bases: self
                .readings
                .iter()
                .filter(|r| r.ok())
                .map(|r| (r.agent, r.base.clone()))
                .collect(),
        }
    }
}

/// The instruction files as a copy sees them: the text of those a pack
/// would hold, the hash of every one that reads.
pub fn file_infos(read: &[instructions::Read1], choices: &instructions::Choices) -> Vec<FileInfo> {
    let packed = instructions::packed(read, choices);
    read.iter()
        .map(|r| {
            let text = r.text.as_ref().ok();
            let held = if text.is_none() {
                Some("unreadable")
            } else if !r.keys.is_empty() {
                Some("key")
            } else if choices.excluded.contains(&r.shown) {
                Some("excluded")
            } else {
                None
            };
            FileInfo {
                path: r.shown.clone(),
                agent: r.agent,
                sha256: text.map(|t| write::sha256(t.as_bytes())).unwrap_or_default(),
                text: packed.iter().find(|p| p.path == r.shown).map(|p| p.text.clone()),
                held: held.map(str::to_string),
            }
        })
        .collect()
}

/// The version of each skill from GitHub.
pub fn versions(look: &skills::Look) -> BTreeMap<String, SkillVersion> {
    let Ok(lock) = &look.lock else { return BTreeMap::new() };
    lock.iter()
        .filter(|(name, l)| l.github() && look.shared.contains(*name))
        .map(|(name, l)| {
            let at = l.updated_at.clone().or_else(|| l.installed_at.clone());
            (name.clone(), SkillVersion { hash: l.skill_folder_hash.clone(), at })
        })
        .collect()
}

// ===== The comparison =====

/// One agent's definition of a server on each Mac, secrets masked.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefPair {
    pub agent: Agent,
    pub mine: Option<Def>,
    pub theirs: Option<Def>,
}

/// One item that is not the same on both Macs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffItem {
    /// `mcp`, `skill`, `plugin` or `file`.
    pub group: &'static str,
    /// The server's or the skill's name, the plugin's `name@marketplace`,
    /// the file's `~/…`.
    pub id: String,
    pub agent: Option<Agent>,
    /// `mine` (only on this Mac), `theirs` (only on the other), `diff`
    /// (another configuration), `ver` (another version) or `text` (another
    /// text).
    pub state: &'static str,
    /// What differs: `command`, `args`, `env`, `cwd`, `url`, `headers`,
    /// `agents`, `source`.
    pub what: Vec<&'static str>,
    /// Why it cannot go from this Mac to the other one: `leftOut` (a skill
    /// installed by hand whose files stay here, `note` says why), `older`
    /// (the other Mac's version is the newer one), `hand` (two skills
    /// installed by hand that differ), `source`, `key` (a file with a line
    /// shaped like a key), `excluded`, `unreadable`, `absent` (no agent for
    /// it there).
    pub push: Option<&'static str>,
    /// The same, the other way.
    pub pull: Option<&'static str>,
    pub note: Option<String>,
    /// A skill installed by hand: its files go along.
    pub hand: bool,
    /// A server's secrets, by their names.
    pub keys: Vec<String>,
    pub mine_on: Vec<Agent>,
    pub theirs_on: Vec<Agent>,
    pub compare: Vec<DefPair>,
    /// A skill's version on each Mac, RFC 3339.
    pub mine_at: Option<String>,
    pub theirs_at: Option<String>,
    /// A file's lines, this Mac's against the other's.
    pub lines: Vec<Line>,
    pub changed: usize,
}

impl DiffItem {
    fn new(group: &'static str, id: &str, state: &'static str) -> Self {
        DiffItem {
            group,
            id: id.to_string(),
            agent: None,
            state,
            what: Vec::new(),
            push: None,
            pull: None,
            note: None,
            hand: false,
            keys: Vec::new(),
            mine_on: Vec::new(),
            theirs_on: Vec::new(),
            compare: Vec::new(),
            mine_at: None,
            theirs_at: None,
            lines: Vec::new(),
            changed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub items: Vec<DiffItem>,
    /// Items both Macs have alike.
    pub same: Vec<String>,
}

/// A definition with a command given by its full path named by its file
/// name only: another Mac keeps the same tool somewhere else.
fn portable_command(def: &Def) -> Def {
    let mut out = def.clone();
    if let Def::Stdio { command, .. } = &mut out {
        if command.starts_with('/') || command.starts_with("~/") {
            *command = command.rsplit('/').next().unwrap_or(command).to_string();
        }
    }
    out
}

/// What differs between two definitions of a server.
fn def_changes(a: &Def, b: &Def) -> Vec<&'static str> {
    let (a, b) = (portable_command(a), portable_command(b));
    if a.same(&b) {
        return Vec::new();
    }
    let sorted = |pairs: &[super::def::Pair]| {
        let mut v: Vec<(String, String)> =
            pairs.iter().map(|p| (p.key.clone(), p.value.clone())).collect();
        v.sort();
        v
    };
    let mut out = Vec::new();
    match (&a, &b) {
        (
            Def::Stdio { command: c1, args: a1, env: e1, cwd: d1 },
            Def::Stdio { command: c2, args: a2, env: e2, cwd: d2 },
        ) => {
            if c1 != c2 {
                out.push("command");
            }
            if a1 != a2 {
                out.push("args");
            }
            if sorted(e1) != sorted(e2) {
                out.push("env");
            }
            if d1 != d2 {
                out.push("cwd");
            }
        }
        (
            Def::Http { url: u1, headers: h1, sse: s1 },
            Def::Http { url: u2, headers: h2, sse: s2 },
        ) => {
            if u1 != u2 || s1 != s2 {
                out.push("url");
            }
            if sorted(h1) != sorted(h2) {
                out.push("headers");
            }
        }
        _ => out.push("command"),
    }
    out
}

fn on_agents(server: &pack::Server) -> Vec<Agent> {
    server.entries.iter().filter(|e| e.enabled).map(|e| e.agent).collect()
}

fn key_names(inv: &Inventory, server: &pack::Server) -> Vec<String> {
    server
        .slots
        .iter()
        .map(|slot| {
            inv.secrets
                .secrets
                .iter()
                .find(|n| n.mcp == server.name && &n.slot == slot)
                .map(|n| n.label.clone())
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| pack::slot_label(server.entries.first().map(|e| &e.def), slot))
        })
        .collect()
}

fn diff_mcp(mine: &Inventory, theirs: &Inventory, out: &mut Diff) {
    let names: BTreeSet<&str> =
        mine.mcp.servers.iter().chain(&theirs.mcp.servers).map(|s| s.name.as_str()).collect();
    for name in names {
        let m = mine.mcp.servers.iter().find(|s| s.name == name);
        let t = theirs.mcp.servers.iter().find(|s| s.name == name);
        let masked = |s: Option<&pack::Server>, a: Agent| {
            s.and_then(|s| s.entries.iter().find(|e| e.agent == a)).map(|e| e.def.masked(MASK))
        };
        let reaches = |s: &pack::Server, inv: &Inventory| {
            s.entries.iter().any(|e| inv.agents.contains(&e.agent))
        };
        let mut item = match (m, t) {
            (Some(m), None) => {
                let mut i = DiffItem::new("mcp", name, "mine");
                i.push = (!reaches(m, theirs)).then_some("absent");
                i
            }
            (None, Some(t)) => {
                let mut i = DiffItem::new("mcp", name, "theirs");
                i.pull = (!reaches(t, mine)).then_some("absent");
                i
            }
            (Some(m), Some(t)) => {
                let mut what: Vec<&'static str> = Vec::new();
                for agent in Agent::ALL {
                    let a = m.entries.iter().find(|e| e.agent == agent);
                    let b = t.entries.iter().find(|e| e.agent == agent);
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            for w in def_changes(&a.def, &b.def) {
                                if !what.contains(&w) {
                                    what.push(w);
                                }
                            }
                        }
                        // An agent the other Mac has not got is no difference.
                        (Some(_), None) if theirs.agents.contains(&agent) => what.push("agents"),
                        (None, Some(_)) if mine.agents.contains(&agent) => what.push("agents"),
                        _ => {}
                    }
                }
                what.dedup();
                if what.is_empty() {
                    out.same.push(name.to_string());
                    continue;
                }
                let mut i = DiffItem::new("mcp", name, "diff");
                i.what = what;
                i
            }
            (None, None) => continue,
        };
        let any = m.or(t).expect("one side has it");
        item.keys = key_names(if m.is_some() { mine } else { theirs }, any);
        item.mine_on = m.map(on_agents).unwrap_or_default();
        item.theirs_on = t.map(on_agents).unwrap_or_default();
        item.compare = Agent::ALL
            .into_iter()
            .filter_map(|a| {
                let (x, y) = (masked(m, a), masked(t, a));
                (x.is_some() || y.is_some()).then_some(DefPair { agent: a, mine: x, theirs: y })
            })
            .collect();
        out.items.push(item);
    }
}

fn skill_note(s: &pack::PackedSkill) -> Option<String> {
    s.left_out.clone().or_else(|| (s.hand && !s.copied).then(|| "unreadable".to_string()))
}

fn diff_skills(mine: &Inventory, theirs: &Inventory, out: &mut Diff) {
    let names: BTreeSet<&str> =
        mine.skills.skills.iter().chain(&theirs.skills.skills).map(|s| s.name.as_str()).collect();
    for name in names {
        let m = mine.skills.skills.iter().find(|s| s.name == name);
        let t = theirs.skills.skills.iter().find(|s| s.name == name);
        let item = match (m, t) {
            (Some(m), None) => {
                let mut i = DiffItem::new("skill", name, "mine");
                if m.hand && !m.copied {
                    i.push = Some("leftOut");
                    i.note = skill_note(m);
                }
                i.mine_on = m.on.clone();
                i
            }
            (None, Some(t)) => {
                let mut i = DiffItem::new("skill", name, "theirs");
                if t.hand && !t.copied {
                    i.pull = Some("leftOut");
                    i.note = skill_note(t);
                }
                i.theirs_on = t.on.clone();
                i
            }
            (Some(m), Some(t)) => {
                if m.hand != t.hand || m.repo != t.repo {
                    let mut i = DiffItem::new("skill", name, "diff");
                    i.what = vec!["source"];
                    i.push = Some("source");
                    i.pull = Some("source");
                    i
                } else if m.hand {
                    if m.hash.is_none() || t.hash.is_none() || m.hash == t.hash {
                        out.same.push(name.to_string());
                        continue;
                    }
                    let mut i = DiffItem::new("skill", name, "text");
                    i.push = Some("hand");
                    i.pull = Some("hand");
                    i
                } else {
                    let (vm, vt) = (mine.versions.get(name), theirs.versions.get(name));
                    let (hm, ht) =
                        (vm.and_then(|v| v.hash.as_ref()), vt.and_then(|v| v.hash.as_ref()));
                    if hm.is_none() || ht.is_none() || hm == ht {
                        out.same.push(name.to_string());
                        continue;
                    }
                    let mut i = DiffItem::new("skill", name, "ver");
                    i.mine_at = vm.and_then(|v| v.at.clone());
                    i.theirs_at = vt.and_then(|v| v.at.clone());
                    // The update brings the newest version from GitHub: only
                    // to the Mac whose version is the older one.
                    if let (Some(a), Some(b)) = (&i.mine_at, &i.theirs_at) {
                        if a <= b {
                            i.push = Some("older");
                        }
                        if b <= a {
                            i.pull = Some("older");
                        }
                    }
                    i
                }
            }
            (None, None) => continue,
        };
        let mut item = item;
        item.hand = m.or(t).is_some_and(|s| s.hand);
        out.items.push(item);
    }
}

fn diff_plugins(mine: &Inventory, theirs: &Inventory, out: &mut Diff) {
    for agent in [Agent::Claude, Agent::Codex] {
        let side = |inv: &Inventory| {
            inv.plugins
                .agents
                .iter()
                .find(|s| s.agent == agent && (s.state == "ok" || s.state == "kept"))
                .cloned()
        };
        // An agent whose list did not read on one Mac is not compared.
        let (Some(m), Some(t)) = (side(mine), side(theirs)) else { continue };
        let listed = |s: &super::plugins::Side| -> BTreeSet<String> {
            s.plugins
                .iter()
                .filter(|p| !super::plugins::builtin(s, p))
                .map(|p| p.id.clone())
                .collect()
        };
        let (lm, lt) = (listed(&m), listed(&t));
        for id in lm.union(&lt) {
            let state = match (lm.contains(id), lt.contains(id)) {
                (true, false) => "mine",
                (false, true) => "theirs",
                _ => {
                    out.same.push(id.clone());
                    continue;
                }
            };
            let mut i = DiffItem::new("plugin", id, state);
            i.agent = Some(agent);
            out.items.push(i);
        }
    }
}

fn diff_files(mine: &Inventory, theirs: &Inventory, out: &mut Diff) {
    let paths: BTreeSet<&str> =
        mine.files.iter().chain(&theirs.files).map(|f| f.path.as_str()).collect();
    for path in paths {
        let m = mine.files.iter().find(|f| f.path == path);
        let t = theirs.files.iter().find(|f| f.path == path);
        let held = |f: Option<&FileInfo>| -> Option<&'static str> {
            match f.and_then(|f| f.held.as_deref()) {
                Some("key") => Some("key"),
                Some("excluded") => Some("excluded"),
                Some(_) => Some("unreadable"),
                None => None,
            }
        };
        let mut item = match (m, t) {
            (Some(m), Some(t)) if m.sha256 == t.sha256 => {
                out.same.push(path.to_string());
                continue;
            }
            (Some(fm), Some(ft)) => {
                let mut i = DiffItem::new("file", path, "text");
                // Neither side's file is replaced while either holds a key.
                let h = held(m).or(held(t));
                i.push = h;
                i.pull = h;
                if let (Some(a), Some(b)) = (&fm.text, &ft.text) {
                    i.lines = batch::diff(a, b);
                    let count = |op| i.lines.iter().filter(|l| l.op == op).count();
                    i.changed = count("add").max(count("del"));
                }
                i
            }
            (Some(_), None) => {
                let mut i = DiffItem::new("file", path, "mine");
                i.push = held(m);
                i
            }
            (None, Some(_)) => {
                let mut i = DiffItem::new("file", path, "theirs");
                i.pull = held(t);
                i
            }
            (None, None) => continue,
        };
        item.agent = m.or(t).and_then(|f| f.agent);
        out.items.push(item);
    }
}

/// What differs between this Mac (`mine`) and the other (`theirs`).
pub fn diff(mine: &Inventory, theirs: &Inventory) -> Diff {
    let mut out = Diff { items: Vec::new(), same: Vec::new() };
    diff_mcp(mine, theirs, &mut out);
    diff_skills(mine, theirs, &mut out);
    diff_plugins(mine, theirs, &mut out);
    diff_files(mine, theirs, &mut out);
    out
}

// ===== What is sent =====

/// The items ticked, by group. `updates` are skills from GitHub to bring
/// to their newest version.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Selection {
    pub mcp: Vec<String>,
    pub skills: Vec<String>,
    pub updates: Vec<String>,
    pub plugins: Vec<PluginRef>,
    pub files: Vec<String>,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.mcp.is_empty()
            && self.skills.is_empty()
            && self.updates.is_empty()
            && self.plugins.is_empty()
            && self.files.is_empty()
    }
}

/// A key's value on its way to the other Mac's Keychain.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValue {
    pub mcp: String,
    pub slot: String,
    pub value: String,
}

impl std::fmt::Debug for KeyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} = …", self.mcp, self.slot)
    }
}

/// One file of a skill installed by hand, base64.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillFile {
    pub path: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillBundle {
    pub name: String,
    pub files: Vec<SkillFile>,
}

/// The items ticked, as a pack of them: what the receiving Mac restores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub protocol: u32,
    /// The sending Mac's name.
    pub from: String,
    pub mcp: McpList,
    pub secrets: SecretsNeeded,
    pub skills: SkillList,
    #[serde(default)]
    pub skill_files: Vec<SkillBundle>,
    #[serde(default)]
    pub updates: Vec<String>,
    pub plugins: PluginList,
    pub files: InstructionList,
    /// Only when "Gửi cả khoá qua SSH" is ticked.
    #[serde(default)]
    pub keys: Vec<KeyValue>,
    /// The receiving Mac's MCP files and instruction files as the
    /// comparison read them: one changed since is not written.
    #[serde(default)]
    pub bases: BTreeMap<Agent, String>,
    #[serde(default)]
    pub file_bases: BTreeMap<String, String>,
}

/// What the other Mac is asked for a copy to this Mac.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAsk {
    pub protocol: u32,
    pub selection: Selection,
    /// Send the keys too: "Gửi cả khoá qua SSH" is ticked.
    pub keys: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum PayloadError {
    /// A skill installed by hand changed and now has a line shaped like a
    /// key, a link, or is too large: nothing is sent.
    Skill(String),
    Io(String),
}

/// The pack of `sel` from `here`. With `keys`, each server's secrets as
/// this Mac has them, from an agent's file or the app's Keychain item.
pub fn payload(
    here: &Here,
    sel: &Selection,
    keys: Option<&Secrets>,
) -> Result<Payload, PayloadError> {
    let pack = &here.pack;
    let mcp = McpList {
        servers: pack.mcp.servers.iter().filter(|s| sel.mcp.contains(&s.name)).cloned().collect(),
    };
    let secrets = SecretsNeeded {
        secrets: pack
            .secrets
            .secrets
            .iter()
            .filter(|n| sel.mcp.contains(&n.mcp))
            .cloned()
            .collect(),
    };
    let all_skills = pack.skills.clone().unwrap_or_default();
    let skills = SkillList {
        skills: all_skills.skills.into_iter().filter(|s| sel.skills.contains(&s.name)).collect(),
    };
    let known = pack::secret_values(&here.readings);
    let mut skill_files = Vec::new();
    for s in skills.skills.iter().filter(|s| s.hand && s.copied) {
        let Some((_, dir)) = pack.folders.iter().find(|(n, _)| n == &s.name) else {
            return Err(PayloadError::Skill(s.name.clone()));
        };
        let files =
            pack::check_skill(dir, &known).map_err(|_| PayloadError::Skill(s.name.clone()))?;
        let mut out = Vec::new();
        for (rel, _) in files {
            let data =
                std::fs::read(dir.join(&rel)).map_err(|e| PayloadError::Io(e.to_string()))?;
            out.push(SkillFile {
                path: rel,
                data: base64::engine::general_purpose::STANDARD.encode(data),
            });
        }
        skill_files.push(SkillBundle { name: s.name.clone(), files: out });
    }
    let plugins = PluginList {
        agents: pack
            .plugins
            .iter()
            .flat_map(|l| &l.agents)
            .filter_map(|side| {
                let picked: Vec<_> = side
                    .plugins
                    .iter()
                    .filter(|p| sel.plugins.iter().any(|r| r.agent == side.agent && r.id == p.id))
                    .cloned()
                    .collect();
                if picked.is_empty() {
                    return None;
                }
                let markets = side
                    .marketplaces
                    .iter()
                    .filter(|m| picked.iter().any(|p| p.marketplace() == m.name))
                    .cloned()
                    .collect();
                let mut s = side.clone();
                s.plugins = picked;
                s.marketplaces = markets;
                Some(s)
            })
            .collect(),
    };
    let files = InstructionList {
        files: pack
            .instructions
            .iter()
            .flat_map(|l| &l.files)
            .filter(|f| sel.files.contains(&f.path))
            .cloned()
            .collect(),
    };
    let mut values = Vec::new();
    if let Some(store) = keys {
        let have = Have { readings: &here.readings, secrets: store };
        for s in &mcp.servers {
            for slot in &s.slots {
                let value = have
                    .in_files(&s.name, slot, None)
                    .or_else(|| store.read(&s.name, slot).ok())
                    .filter(|v| !v.trim().is_empty());
                if let Some(value) = value {
                    values.push(KeyValue { mcp: s.name.clone(), slot: slot.clone(), value });
                }
            }
        }
    }
    Ok(Payload {
        protocol: PROTOCOL,
        from: pack.manifest.mac.clone(),
        mcp,
        secrets,
        skills,
        skill_files,
        updates: sel.updates.clone(),
        plugins,
        files,
        keys: values,
        bases: BTreeMap::new(),
        file_bases: BTreeMap::new(),
    })
}

// ===== Writing what was sent =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum ApplyError {
    /// Another version of the copy.
    Protocol,
    /// A name or a path the app does not write.
    Invalid(String),
    Io(String),
    /// The restore refused before writing anything.
    Run(restore::RunError),
}

fn plain_path(rel: &str) -> bool {
    let p = Path::new(rel);
    !rel.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_)))
}

/// Puts the files of each skill installed by hand where the restore takes
/// them from: `<root>/<id>/skills/<name>/`, mode 600.
fn write_bundles(root: &Path, id: &str, bundles: &[SkillBundle]) -> Result<(), ApplyError> {
    for b in bundles {
        if !skills::valid_name(&b.name) {
            return Err(ApplyError::Invalid(b.name.clone()));
        }
        let dir = pack::skill_dir(root, id, &b.name);
        for f in &b.files {
            if !plain_path(&f.path) {
                return Err(ApplyError::Invalid(f.path.clone()));
            }
            let data = base64::engine::general_purpose::STANDARD
                .decode(&f.data)
                .map_err(|_| ApplyError::Invalid(f.path.clone()))?;
            let dst = dir.join(&f.path);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|e| ApplyError::Io(e.to_string()))?;
            }
            let mut out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&dst)
                .map_err(|e| ApplyError::Io(e.to_string()))?;
            out.write_all(&data).map_err(|e| ApplyError::Io(e.to_string()))?;
        }
    }
    Ok(())
}

/// Writes `payload` on this Mac, as the restore of a pack of only those
/// items: every server, skill, plugin and file in it, its keys into the
/// Keychain, then the skills to update. Kept as the last restore, so the
/// Sao lưu tab here and the sending Mac can undo it.
pub fn apply(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    tools: &Tools,
    payload: &Payload,
    now: OffsetDateTime,
) -> Result<restore::Done, ApplyError> {
    if payload.protocol != PROTOCOL {
        return Err(ApplyError::Protocol);
    }
    if let Some(bad) = payload.mcp.servers.iter().find(|s| !super::valid_name(&s.name)) {
        return Err(ApplyError::Invalid(bad.name.clone()));
    }
    if let Some(bad) = payload
        .skills
        .skills
        .iter()
        .map(|s| &s.name)
        .chain(&payload.updates)
        .find(|n| !skills::valid_name(n))
    {
        return Err(ApplyError::Invalid(bad.clone()));
    }
    let root = app.path().join(INCOMING);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| ApplyError::Io(e.to_string()))?;
    let _ = std::fs::set_permissions(&root, std::os::unix::fs::PermissionsExt::from_mode(0o700));
    let id = format!("copy-{}", local_stamp(now));
    let result = (|| {
        write_bundles(&root, &id, &payload.skill_files)?;
        let mut sections = vec!["mcp".to_string()];
        sections.extend(["skills", "plugins", "instructions"].map(String::from));
        let pack = Pack {
            manifest: pack::Manifest {
                format: pack::FORMAT,
                app: String::new(),
                mac: payload.from.clone(),
                at: now.unix_timestamp(),
                sections,
                mcp: payload.mcp.servers.len(),
                keys: payload.secrets.secrets.len(),
                mcp_hash: String::new(),
            },
            mcp: own_commands(places, &payload.mcp),
            secrets: payload.secrets.clone(),
            skills: Some(payload.skills.clone()),
            plugins: Some(payload.plugins.clone()),
            instructions: Some(payload.files.clone()),
            folders: Vec::new(),
        };
        let tools = Tools { root: root.clone(), ..tools.clone() };
        let plan = restore::plan(places, app, secrets, &tools, &id, &pack);
        let picks: Vec<Pick> =
            plan.items.iter().map(|i| Pick { name: i.name.clone(), kind: i.kind.into() }).collect();
        let files: Vec<String> =
            plan.files.iter().flat_map(|f| &f.items).map(|i| i.path.clone()).collect();
        let more = More {
            skills: plan.skills.iter().flat_map(|s| &s.items).map(|i| i.name.clone()).collect(),
            plugins: plan
                .plugins
                .iter()
                .flat_map(|p| &p.items)
                .map(|i| PluginRef { agent: i.agent, id: i.id.clone() })
                .collect(),
            file_bases: files
                .iter()
                .map(|f| (f.clone(), payload.file_bases.get(f).cloned().unwrap_or_default()))
                .collect(),
            files,
        };
        let pasted: Vec<Pasted> = payload
            .keys
            .iter()
            .map(|k| Pasted { mcp: k.mcp.clone(), slot: k.slot.clone(), value: k.value.clone() })
            .collect();
        let mut done = restore::run(
            places,
            app,
            secrets,
            &tools,
            &id,
            &pack,
            &picks,
            &more,
            &pasted,
            &payload.bases,
            now,
        )
        .map_err(ApplyError::Run)?;
        if !payload.updates.is_empty() {
            let mut record =
                restore::last(app).filter(|r| r.at == done.at).unwrap_or_else(|| Record {
                    at: done.at,
                    pack: id.clone(),
                    mac: payload.from.clone(),
                    undo: None,
                    kept: Vec::new(),
                    created: Vec::new(),
                    skills: Vec::new(),
                    plugins: Vec::new(),
                    markets: Vec::new(),
                    files: Vec::new(),
                    updated: Vec::new(),
                });
            let parts = restore::update_skills(&tools, app, &payload.updates, &mut record, now);
            if !record.updated.is_empty() {
                restore::save_record(app, &record);
                done.can_undo = true;
            }
            done.parts.extend(parts);
        }
        Ok(done)
    })();
    let _ = std::fs::remove_dir_all(&root);
    result
}

/// The servers sent, each command given by its full path set to where this
/// Mac keeps the same tool for a server of that name: a command at another
/// path is the same command, and the copy keeps this Mac's.
fn own_commands(places: &Places, list: &McpList) -> McpList {
    let readings = super::read_all(places);
    let mut out = list.clone();
    for server in &mut out.servers {
        let here: Vec<&Def> = readings
            .iter()
            .filter(|r| r.ok())
            .filter_map(|r| r.server(&server.name)?.def.as_ref())
            .collect();
        for entry in &mut server.entries {
            let Def::Stdio { command, .. } = &mut entry.def else { continue };
            if !command.contains('/') {
                continue;
            }
            let name = command.rsplit('/').next().unwrap_or_default().to_string();
            let mine = here.iter().find_map(|d| match d {
                Def::Stdio { command: c, .. }
                    if c.contains('/') && c.rsplit('/').next() == Some(&name) =>
                {
                    Some(c.clone())
                }
                _ => None,
            });
            if let Some(c) = mine {
                let probe = Def::Stdio { command: c, args: Vec::new(), env: Vec::new(), cwd: None };
                if let Def::Stdio { command: c, .. } = pack::portable(&probe, &places.home) {
                    *command = c;
                }
            }
        }
    }
    out
}

/// `20261019-143005`, on this Mac's clock.
fn local_stamp(now: OffsetDateTime) -> String {
    crate::local_time::stamp(now).replace(['-', ':'], "").replace('T', "-")
}

/// Which copy an undo is about: the receiving Mac's last restore, by when
/// it ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoAsk {
    pub at: i64,
    pub force: bool,
}

/// Undoes the copy that ran at `ask.at`, when it is still this Mac's last
/// restore. `Err("gone")` when it is not.
pub fn undo(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    tools: &Tools,
    ask: &UndoAsk,
    now: OffsetDateTime,
) -> Result<restore::Undone, Json> {
    if restore::last(app).map(|r| r.at) != Some(ask.at) {
        return Err(Json::String("gone".into()));
    }
    restore::undo(places, app, secrets, tools, ask.force, now)
        .map_err(|e| serde_json::to_value(e).unwrap_or(Json::Null))
}

// ===== The write running on its own =====

/// What a job is given on its stdin: never written to disk, as it may hold
/// keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum JobInput {
    Apply { payload: Box<Payload> },
    Undo { ask: UndoAsk },
}

impl JobInput {
    pub fn kind(&self) -> &'static str {
        match self {
            JobInput::Apply { .. } => "apply",
            JobInput::Undo { .. } => "undo",
        }
    }
}

/// A write on the receiving Mac: `agents/copy-job.json` in its app folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    /// `apply` or `undo`.
    pub kind: String,
    /// `running`, `done`, `failed`, or `lost` (it stopped without a word).
    pub state: String,
    pub started: i64,
    #[serde(default)]
    pub pid: u32,
    /// What the restore or its undo said.
    #[serde(default)]
    pub result: Option<Json>,
    #[serde(default)]
    pub error: Option<Json>,
}

fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 only asks whether the process is there.
    pid > 0 && unsafe { libc::kill(pid as libc::pid_t, 0) } == 0
}

/// The job as it stands; a running one whose process is gone is `lost`.
pub fn job(app: &AppDir) -> Option<Job> {
    let text = app.read_to_string(&app.path().join(JOB)).ok()?;
    let mut job: Job = serde_json::from_str(&text).ok()?;
    if job.state == "running" && !alive(job.pid) {
        job.state = "lost".into();
    }
    Some(job)
}

pub fn save_job(app: &AppDir, job: &Job) -> std::io::Result<()> {
    let file = app.path().join(JOB);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    app.write_private(&file, &serde_json::to_string_pretty(job).map_err(std::io::Error::other)?)
}

/// Whether a job is running now: a second one waits.
pub fn busy(app: &AppDir) -> bool {
    job(app).is_some_and(|j| j.state == "running")
}

/// Runs `input` and keeps how it ended in the job's file.
pub fn run_job(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    tools: &Tools,
    input: &JobInput,
    started: i64,
    now: OffsetDateTime,
) -> Job {
    let (result, error) = match input {
        JobInput::Apply { payload } => match apply(places, app, secrets, tools, payload, now) {
            Ok(done) => (serde_json::to_value(done).ok(), None),
            Err(e) => (None, Some(serde_json::to_value(e).unwrap_or(Json::Null))),
        },
        JobInput::Undo { ask } => match undo(places, app, secrets, tools, ask, now) {
            Ok(u) => (serde_json::to_value(u).ok(), None),
            Err(e) => (None, Some(e)),
        },
    };
    let job = Job {
        kind: input.kind().into(),
        state: if error.is_some() { "failed" } else { "done" }.into(),
        started,
        pid: std::process::id(),
        result,
        error,
    };
    let _ = save_job(app, &job);
    job
}

// ===== Calls to the other Mac =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum CopyError {
    /// SSH could not reach the Mac or log in, with its first error line.
    Unreachable(String),
    Timeout,
    /// No Quota Station in its Applications folders.
    NoApp,
    /// Its Quota Station is version 1: nothing is read or written there.
    Old,
    /// Its app said no, and why: `busy`, `protocol`, …
    Refused(String),
    /// The call failed there, with its first error line.
    Failed(String),
    /// An answer the app cannot read.
    Reply,
    /// The write there is still running after [`WAIT_LIMIT`].
    Waiting,
}

impl CopyError {
    /// The word the window gets: `unreachable:<line>`, `timeout`, `noApp`, …
    pub fn code(&self) -> String {
        match self {
            CopyError::Unreachable(e) => format!("unreachable:{e}"),
            CopyError::Timeout => "timeout".into(),
            CopyError::NoApp => "noApp".into(),
            CopyError::Old => "old".into(),
            CopyError::Refused(w) => format!("refused:{w}"),
            CopyError::Failed(e) => format!("failed:{e}"),
            CopyError::Reply => "reply".into(),
            CopyError::Waiting => "waiting".into(),
        }
    }
}

/// The other Mac, reached through `ssh` by its alias.
#[derive(Debug, Clone)]
pub struct Remote {
    pub ssh: PathBuf,
    pub alias: String,
    /// [`APP_FOLDERS`]; the tests point it at their own folders.
    pub folders: Vec<String>,
}

impl Remote {
    pub fn new(ssh: PathBuf, alias: &str) -> Self {
        Remote { ssh, alias: alias.to_string(), folders: APP_FOLDERS.map(String::from).to_vec() }
    }

    fn call(&self, word: Word, input: Option<&str>) -> Result<String, CopyError> {
        let folders: Vec<&str> = self.folders.iter().map(String::as_str).collect();
        let out = peers::ssh(&self.ssh, &self.alias, &script(word, &folders), input).map_err(
            |e| match e {
                peers::RemoteError::Timeout => CopyError::Timeout,
                peers::RemoteError::Unreachable(l) => CopyError::Unreachable(l),
                other => CopyError::Failed(other.to_string()),
            },
        )?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        match out.code {
            Some(0) => Ok(stdout),
            Some(255) => Err(CopyError::Unreachable(runner::first_line(&out.stderr))),
            Some(NO_APP) => Err(CopyError::NoApp),
            Some(OLD) => Err(CopyError::Old),
            Some(REFUSED) => {
                let why = serde_json::from_str::<Json>(&stdout)
                    .ok()
                    .and_then(|v| v.get("error").and_then(Json::as_str).map(str::to_string))
                    .unwrap_or_default();
                Err(CopyError::Refused(why))
            }
            code => {
                let line = runner::first_line(&out.stderr);
                Err(CopyError::Failed(if line.is_empty() {
                    format!("exit {code:?}")
                } else {
                    line
                }))
            }
        }
    }

    fn json<T: for<'de> Deserialize<'de>>(
        &self,
        word: Word,
        input: Option<&str>,
    ) -> Result<T, CopyError> {
        let text = self.call(word, input)?;
        serde_json::from_str(&text).map_err(|_| CopyError::Reply)
    }

    /// Its inventory: names, configuration and hashes, never a key.
    pub fn read(&self) -> Result<Inventory, CopyError> {
        let inv: Inventory = self.json(Word::Read, None)?;
        if inv.protocol < PROTOCOL {
            return Err(CopyError::Old);
        }
        Ok(inv)
    }

    /// The items of `selection` there, keys only with `keys`.
    pub fn export(&self, selection: &Selection, keys: bool) -> Result<Payload, CopyError> {
        let ask = ExportAsk { protocol: PROTOCOL, selection: selection.clone(), keys };
        let input = serde_json::to_string(&ask).map_err(|_| CopyError::Reply)?;
        self.json(Word::Export, Some(&input))
    }

    /// Starts a copy there, or its undo, and returns the job as it began.
    /// The keys, when the payload has them, are on stdin only.
    pub fn start(&self, input: &JobInput) -> Result<Job, CopyError> {
        let text = serde_json::to_string(input).map_err(|_| CopyError::Reply)?;
        self.json(Word::Start, Some(&text))
    }

    pub fn status(&self) -> Result<Option<Job>, CopyError> {
        self.json(Word::Status, None)
    }

    /// Asks about `job` every `every` until it ends, for at most `limit`.
    /// Its file may still hold the job before it while it starts.
    pub fn wait(
        &self,
        job: &Job,
        every: Duration,
        limit: Duration,
        sleep: &dyn Fn(Duration),
    ) -> Result<Job, CopyError> {
        let mut waited = Duration::ZERO;
        loop {
            if let Some(j) = self.status()? {
                if j.started == job.started && j.pid == job.pid && j.state != "running" {
                    return Ok(j);
                }
            }
            if waited >= limit {
                return Err(CopyError::Waiting);
            }
            sleep(every);
            waited += every;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scripts_fit_in_single_quotes_and_hold_no_data() {
        for w in [Word::Read, Word::Export, Word::Start, Word::Status] {
            let s = script(w, &APP_FOLDERS);
            assert!(!s.contains('\''), "{s}");
            assert!(s.ends_with(&format!("agents {}", w.as_str())), "{s}");
            assert!(s.contains("plutil -extract QuotaStationAgents raw"));
        }
    }

    #[test]
    fn a_command_at_another_path_is_the_same_command() {
        let d = |c: &str| Def::Stdio {
            command: c.into(),
            args: vec!["--x".into()],
            env: vec![],
            cwd: None,
        };
        assert!(def_changes(&d("~/.nvm/versions/node/v20/bin/tool"), &d("/opt/homebrew/bin/tool"))
            .is_empty());
        assert_eq!(def_changes(&d("tool"), &d("other")), vec!["command"]);
    }
}
