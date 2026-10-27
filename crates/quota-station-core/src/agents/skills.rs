// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The skills of Claude Code, Codex and Antigravity in one table (M11, B),
//! as the MCP tab has their servers: each skill a row, each agent a column
//! of checkboxes, ticked quickly and written in one batch after one preview.
//!
//! There is one store of skills, `~/.agents/skills`, the folder `npx skills`
//! installs into and records in `~/.agents/.skill-lock.json`. The app keeps
//! no second one. What each agent reads, and the only thing the app changes
//! there:
//!
//! - Claude Code: `~/.claude/skills/<name>`. On is a link to the shared
//!   folder's skill; off is no link. The app makes and removes links only.
//! - Antigravity: `~/.gemini/config/skills/<name>`, the same way.
//! - Codex reads the shared folder itself (and its own `skills` folder), so
//!   off is a `[[skills.config]]` entry with `enabled = false` in
//!   `config.toml`, on is no such entry. Nothing else in the file changes.
//!
//! A folder of its own in an agent's folder is a skill installed by hand:
//! shown, turned on or off only once it is gathered into the shared folder
//! ([`import`]), which moves it there and leaves a link behind. Turning a
//! skill off never removes a folder, and nothing here runs a skill's files.
//! Codex's own skills (`skills/.system`) and those plugins bring are shown,
//! never changed.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};

use super::batch::{self, Line};
use super::{redact, skill_updates, write, Agent, FileState};
use crate::app_dir::AppDir;
use crate::codex::files as codex_files;
use crate::{local_time, os, runner};

/// In the home folder: the shared store and the file `npx skills` keeps.
pub const SHARED: &str = ".agents/skills";
pub const LOCK: &str = ".agents/.skill-lock.json";
/// The backups of the agents' links and of skill folders, in the app's folder.
pub const BACKUP_DIR: &str = "backups/skills";
/// A skill's instructions: a folder is a skill when it holds one.
pub const SKILL_MD: &str = "SKILL.md";
/// `config.toml` and the lock file are a few kilobytes.
const MAX_FILE: u64 = 4 * 1024 * 1024;

/// Where the skills are on this Mac.
#[derive(Debug, Clone)]
pub struct SkillPlaces {
    pub home: PathBuf,
    pub shared: PathBuf,
    pub lock: PathBuf,
    pub claude: PathBuf,
    pub codex: PathBuf,
    pub codex_config: PathBuf,
    pub agy: PathBuf,
    /// `npx`, which installs, updates and removes skills from GitHub;
    /// without it the rest still works.
    pub npx: Option<PathBuf>,
}

impl SkillPlaces {
    pub fn new(home: &Path, codex_home: &Path, npx: Option<PathBuf>) -> Self {
        SkillPlaces {
            home: home.to_path_buf(),
            shared: home.join(SHARED),
            lock: home.join(LOCK),
            claude: home.join(".claude/skills"),
            codex: codex_home.join("skills"),
            codex_config: codex_home.join(crate::codex::CONFIG),
            agy: home.join(".gemini/config/skills"),
            npx,
        }
    }

    /// The folder of skills the agent reads.
    pub fn dir(&self, agent: Agent) -> &Path {
        match agent {
            Agent::Claude => &self.claude,
            Agent::Codex => &self.codex,
            Agent::Agy => &self.agy,
        }
    }

    /// The agent is on this Mac: its own folder is.
    pub fn present(&self, agent: Agent) -> bool {
        self.dir(agent).parent().is_some_and(Path::is_dir)
    }

    /// Where the windows say the agent's column is written.
    pub fn shown(&self, agent: Agent) -> String {
        match agent {
            Agent::Codex => runner::display_path(&self.codex_config, &self.home),
            a => runner::display_path(self.dir(a), &self.home) + "/",
        }
    }
}

/// `npx` where Node is usually installed: the usual folders, then the
/// newest Node under `~/.nvm`.
pub fn discover_npx(home: &Path) -> Option<PathBuf> {
    if let Some(p) =
        super::search_dirs(home).into_iter().map(|d| d.join("npx")).find(|p| p.is_file())
    {
        return Some(p);
    }
    let mut nvm: Vec<PathBuf> = std::fs::read_dir(home.join(".nvm/versions/node"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path().join("bin/npx"))
        .filter(|p| p.is_file())
        .collect();
    nvm.sort();
    nvm.pop()
}

/// A skill name the app passes to a command or uses as a folder name.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with(['-', '.'])
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn read_small(file: &Path) -> Result<Option<String>, String> {
    let f = match std::fs::File::open(file) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    f.take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("larger than 4 MB".into());
    }
    String::from_utf8(bytes).map(Some).map_err(|_| "not UTF-8".to_string())
}

// ===== The lock file =====

/// One skill in `~/.agents/.skill-lock.json`: where `npx skills` got it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Locked {
    /// `owner/repo` for a skill from GitHub.
    pub source: String,
    #[serde(default)]
    pub source_type: String,
    /// The skill's `SKILL.md` in the repository.
    #[serde(default)]
    pub skill_path: Option<String>,
    /// The Git tree id of the skill's folder as installed.
    #[serde(default)]
    pub skill_folder_hash: Option<String>,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Locked {
    pub fn github(&self) -> bool {
        self.source_type == "github" && repo_ok(&self.source)
    }

    /// The skill's folder in the repository: `""` for the repository's root.
    pub fn folder(&self) -> Option<String> {
        let p = self.skill_path.as_deref()?;
        let dir = p.strip_suffix(SKILL_MD)?.trim_end_matches('/');
        Some(dir.to_string())
    }
}

/// `owner/repo`, as GitHub names them.
pub fn repo_ok(repo: &str) -> bool {
    let Some((owner, name)) = repo.split_once('/') else { return false };
    let part = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && !s.starts_with('.')
            && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    part(owner) && part(name)
}

#[derive(Deserialize)]
struct LockFile {
    #[serde(default)]
    skills: BTreeMap<String, Locked>,
}

/// The skills the lock file records; none when there is no file.
pub fn read_lock(file: &Path) -> Result<BTreeMap<String, Locked>, String> {
    match read_small(file)? {
        None => Ok(BTreeMap::new()),
        Some(text) => {
            serde_json::from_str::<LockFile>(&text).map(|l| l.skills).map_err(|e| e.to_string())
        }
    }
}

// ===== The folders =====

/// What is at `<agent's folder>/<name>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A link to the shared folder's skill of that name: on.
    Shared,
    /// A skill of its own: a folder, or a link to one outside the shared
    /// folder (`link`). Installed by hand.
    Own { link: Option<PathBuf> },
    /// A link to nothing.
    Broken { target: PathBuf },
}

fn is_skill(dir: &Path) -> bool {
    dir.join(SKILL_MD).is_file()
}

/// The skills in the shared folder, by name.
fn scan_shared(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            (valid_name(&name) && is_skill(&e.path())).then_some(name)
        })
        .collect()
}

/// An agent's folder of skills: each skill, a link to the shared folder, a
/// skill of its own, or a broken link. `Ok(empty)` when there is no folder.
fn scan(dir: &Path, shared: &Path) -> Result<BTreeMap<String, Entry>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e.to_string()),
    };
    let mut out = BTreeMap::new();
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !valid_name(&name) {
            continue;
        }
        let path = e.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&path).unwrap_or_default();
            match std::fs::canonicalize(&path) {
                Err(_) => {
                    out.insert(name, Entry::Broken { target });
                }
                Ok(real) => {
                    let mine = std::fs::canonicalize(shared.join(&name)).ok();
                    if mine.as_deref() == Some(real.as_path()) {
                        out.insert(name, Entry::Shared);
                    } else if is_skill(&real) {
                        out.insert(name, Entry::Own { link: Some(target) });
                    }
                }
            }
        } else if meta.is_dir() && is_skill(&path) {
            out.insert(name, Entry::Own { link: None });
        }
    }
    Ok(out)
}

/// What a folder's links are, for a write to compare against.
fn listing_hash(entries: &BTreeMap<String, Entry>) -> String {
    let text: String = entries
        .iter()
        .map(|(n, e)| match e {
            Entry::Shared => format!("{n}\tshared\n"),
            Entry::Own { link } => {
                format!("{n}\town\t{}\n", link.as_deref().unwrap_or(Path::new("")).display())
            }
            Entry::Broken { target } => format!("{n}\tbroken\t{}\n", target.display()),
        })
        .collect();
    write::sha256(text.as_bytes())
}

// ===== Codex's [[skills.config]] =====

/// One `[[skills.config]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillConfig {
    pub name: Option<String>,
    pub path: Option<String>,
    pub enabled: bool,
}

/// Codex's entries, or why the file is not one the app edits.
pub fn codex_entries(text: &str) -> Result<Vec<SkillConfig>, String> {
    let doc: DocumentMut =
        text.parse().map_err(|e: toml_edit::TomlError| e.message().to_string())?;
    let Some(skills) = doc.get("skills") else { return Ok(Vec::new()) };
    let Some(table) = skills.as_table() else {
        return Err("skills is not a [skills] table".into());
    };
    let Some(config) = table.get("config") else { return Ok(Vec::new()) };
    let Some(list) = config.as_array_of_tables() else {
        return Err("skills.config is not written as [[skills.config]]".into());
    };
    Ok(list
        .iter()
        .map(|t| SkillConfig {
            name: t.get("name").and_then(Item::as_str).map(str::to_string),
            path: t.get("path").and_then(Item::as_str).map(str::to_string),
            enabled: t.get("enabled").and_then(Item::as_bool).unwrap_or(true),
        })
        .collect())
}

/// The paths a `[[skills.config]]` entry may name the skill by.
fn codex_paths(places: &SkillPlaces, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in [&places.shared, &places.codex] {
        out.push(dir.join(name).join(SKILL_MD));
        out.push(dir.join(name));
    }
    out
}

fn names_it(places: &SkillPlaces, c: &SkillConfig, name: &str) -> bool {
    c.name.as_deref() == Some(name)
        || c.path.as_deref().is_some_and(|p| {
            let p = runner::expand_path(p, &places.home);
            codex_paths(places, name).contains(&p)
        })
}

/// Codex has the skill turned off.
pub fn codex_off(places: &SkillPlaces, entries: &[SkillConfig], name: &str) -> bool {
    entries.iter().any(|c| !c.enabled && names_it(places, c, name))
}

/// `config.toml` with each skill of `ops` turned on (no entry turns it off)
/// or off (an entry with `enabled = false`). Nothing else changes; the
/// result is checked for that.
pub fn codex_edit(
    places: &SkillPlaces,
    text: &str,
    ops: &[(String, bool)],
) -> Result<String, String> {
    let mut doc: DocumentMut =
        text.parse().map_err(|e: toml_edit::TomlError| e.message().to_string())?;
    for (name, on) in ops {
        if doc.get("skills").is_none() {
            let mut t = Table::new();
            t.set_implicit(true);
            doc.insert("skills", Item::Table(t));
        }
        let skills = doc
            .get_mut("skills")
            .and_then(Item::as_table_mut)
            .ok_or("skills is not a [skills] table")?;
        if skills.get("config").is_none() {
            skills.insert("config", Item::ArrayOfTables(ArrayOfTables::new()));
        }
        let list = skills
            .get_mut("config")
            .and_then(Item::as_array_of_tables_mut)
            .ok_or("skills.config is not written as [[skills.config]]")?;
        let matches = |t: &Table| {
            let c = SkillConfig {
                name: t.get("name").and_then(Item::as_str).map(str::to_string),
                path: t.get("path").and_then(Item::as_str).map(str::to_string),
                enabled: true,
            };
            names_it(places, &c, name)
        };
        if *on {
            // An entry the app writes (a name and `enabled`) goes; one with
            // more in it stays, turned on.
            let mut i = 0;
            while i < list.len() {
                let t = list.get_mut(i).expect("in range");
                if matches(t) && t.get("enabled").and_then(Item::as_bool) == Some(false) {
                    let plain = t.iter().all(|(k, _)| k == "name" || k == "enabled");
                    if plain {
                        list.remove(i);
                        continue;
                    }
                    t.insert("enabled", toml_edit::value(true));
                }
                i += 1;
            }
        } else {
            let mut found = false;
            for t in list.iter_mut().filter(|t| matches(t)) {
                t.insert("enabled", toml_edit::value(false));
                found = true;
            }
            if !found {
                let mut t = Table::new();
                t.insert("name", toml_edit::value(name.as_str()));
                t.insert("enabled", toml_edit::value(false));
                list.push(t);
            }
        }
        let empty_list = list.is_empty();
        if empty_list {
            skills.remove("config");
        }
        if skills.is_empty() && skills.is_implicit() {
            doc.remove("skills");
        }
    }
    let out = doc.to_string();
    // The check: what is not skills.config reads the same, and each skill
    // is what the edit says.
    let rest = |t: &str| -> Result<String, String> {
        let mut d: DocumentMut =
            t.parse().map_err(|e: toml_edit::TomlError| e.message().to_string())?;
        if let Some(s) = d.get_mut("skills").and_then(Item::as_table_mut) {
            s.remove("config");
            if s.is_empty() {
                d.remove("skills");
            }
        }
        Ok(d.to_string().trim().to_string())
    };
    if rest(text)? != rest(&out)? {
        return Err("the edit changed more than [[skills.config]]".into());
    }
    let entries = codex_entries(&out)?;
    for (name, on) in ops {
        if codex_off(places, &entries, name) == *on {
            return Err(format!("{name} is not {} after the edit", if *on { "on" } else { "off" }));
        }
    }
    Ok(out)
}

// ===== Reading =====

/// One agent's folder and, for Codex, its `config.toml`, as read now.
#[derive(Debug, Clone)]
pub struct Reading {
    pub agent: Agent,
    pub present: bool,
    pub state: FileState,
    /// What a write compares: the folder's links (Claude Code, Antigravity),
    /// `config.toml`'s SHA-256 (Codex; empty with no file).
    pub base: String,
    pub entries: BTreeMap<String, Entry>,
    /// Codex: `config.toml` as read, and its `[[skills.config]]`.
    pub text: Option<String>,
    pub config: Vec<SkillConfig>,
}

impl Reading {
    pub fn writable(&self) -> bool {
        self.present && matches!(self.state, FileState::Ok | FileState::Missing)
    }
}

/// Everything the table is built from.
#[derive(Debug, Clone)]
pub struct Look {
    pub shared: BTreeSet<String>,
    pub lock: Result<BTreeMap<String, Locked>, String>,
    pub readings: Vec<Reading>,
    /// Codex's own skills, in `skills/.system`.
    pub system: Vec<String>,
}

impl Look {
    pub fn reading(&self, agent: Agent) -> &Reading {
        self.readings.iter().find(|r| r.agent == agent).expect("every agent is read")
    }

    pub fn locked(&self, name: &str) -> Option<&Locked> {
        self.lock.as_ref().ok()?.get(name)
    }

    /// The agents whose folder has a skill of its own named `name`.
    pub fn own(&self, name: &str) -> Vec<Agent> {
        self.readings
            .iter()
            .filter(|r| matches!(r.entries.get(name), Some(Entry::Own { .. })))
            .map(|r| r.agent)
            .collect()
    }

    /// Codex reads the skill: in the shared folder or its own.
    pub fn codex_sees(&self, name: &str) -> bool {
        self.shared.contains(name)
            || matches!(
                self.reading(Agent::Codex).entries.get(name),
                Some(Entry::Own { .. } | Entry::Shared)
            )
    }

    /// The skill is on for `agent` now.
    pub fn on(&self, places: &SkillPlaces, name: &str, agent: Agent) -> bool {
        let r = self.reading(agent);
        match agent {
            Agent::Codex => self.codex_sees(name) && !codex_off(places, &r.config, name),
            _ => matches!(r.entries.get(name), Some(Entry::Shared | Entry::Own { .. })),
        }
    }
}

pub fn read(places: &SkillPlaces) -> Look {
    let shared = scan_shared(&places.shared);
    let mut readings = Vec::new();
    for agent in Agent::ALL {
        let present = places.present(agent);
        let mut r = Reading {
            agent,
            present,
            state: FileState::Missing,
            base: String::new(),
            entries: BTreeMap::new(),
            text: None,
            config: Vec::new(),
        };
        if present {
            match scan(places.dir(agent), &places.shared) {
                Ok(e) => {
                    r.state = FileState::Ok;
                    r.entries = e;
                }
                Err(e) => r.state = FileState::Unreadable(e),
            }
            if agent == Agent::Codex {
                match read_small(&places.codex_config) {
                    Ok(None) => {}
                    Ok(Some(text)) => {
                        r.base = write::sha256(text.as_bytes());
                        match codex_entries(&text) {
                            Ok(c) => r.config = c,
                            Err(e) => r.state = FileState::Invalid(e),
                        }
                        r.text = Some(text);
                    }
                    Err(e) => r.state = FileState::Unreadable(e),
                }
            } else {
                r.base = listing_hash(&r.entries);
            }
        }
        readings.push(r);
    }
    let mut system: Vec<String> = std::fs::read_dir(places.codex.join(".system"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| is_skill(&e.path()))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    system.sort();
    Look { shared, lock: read_lock(&places.lock), readings, system }
}

// ===== What the windows see =====

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub agent: Agent,
    pub present: bool,
    /// `~/.claude/skills/`, or Codex's `~/.codex/config.toml`.
    pub file: String,
    /// `ok`, `missing`, `invalid`, `unreadable` or `absent`.
    pub state: &'static str,
    pub reason: Option<String>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    /// `on`, `off`, or `absent` (the agent cannot have it as it is).
    pub state: &'static str,
    /// Why the box cannot be ticked: `readOnly`, `hand` (gather it into the
    /// shared folder first), `taken` (the agent has a skill of its own by
    /// that name) or `broken` (a link to nothing is in the way).
    pub lock: Option<&'static str>,
}

/// A newer version on GitHub (G).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    /// When the installed version came, RFC 3339 from the lock file.
    pub from_at: Option<String>,
    /// When the app first saw the new one, Unix seconds.
    pub seen_at: i64,
    /// What changed, in plain words.
    pub plain: String,
    pub link: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub name: String,
    /// `github`, `other` (recorded in the lock file from elsewhere) or
    /// `hand`.
    pub source: &'static str,
    /// `owner/repo`, or the lock file's source.
    pub repo: Option<String>,
    /// A skill installed by hand: the agent whose folder holds it; `None`
    /// when it is in the shared folder.
    pub home: Option<Agent>,
    pub cells: BTreeMap<Agent, Cell>,
    /// The folder is not what the lock file says was installed.
    pub edited: bool,
    pub update: Option<UpdateView>,
    /// The folder's size in bytes, for gathering.
    pub size: u64,
    /// Can be gathered into the shared folder.
    pub importable: bool,
}

/// Skills an agent or a plugin brings itself: shown, never changed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Owned {
    /// `Codex`, or the plugin (`name@marketplace`).
    pub what: String,
    pub agent: Agent,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Broken {
    pub agent: Agent,
    pub name: String,
    /// Where it points, `~/…`.
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub owned: Vec<Owned>,
    pub broken: Vec<Broken>,
    /// The lock file does not read: sources are not shown.
    pub lock_error: Option<String>,
}

/// The size of a folder's files, not following links.
fn size_of(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    let mut seen = 0;
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            seen += 1;
            if seen > 20_000 {
                return total;
            }
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if m.is_dir() {
                stack.push(e.path());
            } else {
                total += m.len();
            }
        }
    }
    total
}

/// Builds the table. `owned` are the skills plugins bring; `updates` what
/// the last check for newer versions found.
pub fn view(
    places: &SkillPlaces,
    look: &Look,
    owned: Vec<Owned>,
    updates: &skill_updates::Updates,
) -> View {
    let columns = look
        .readings
        .iter()
        .map(|r| {
            let (state, reason) = match (&r.state, r.present) {
                (_, false) => ("absent", None),
                (FileState::Invalid(e), _) => ("invalid", Some(e.clone())),
                (FileState::Unreadable(e), _) => ("unreadable", Some(e.clone())),
                (FileState::Missing, _) => ("missing", None),
                (FileState::Ok, _) => ("ok", None),
            };
            Column {
                agent: r.agent,
                present: r.present,
                file: places.shown(r.agent),
                state,
                reason,
                writable: r.writable(),
            }
        })
        .collect();

    let mut names: BTreeSet<String> = look.shared.clone();
    let mut broken = Vec::new();
    for r in look.readings.iter().filter(|r| r.present) {
        for (name, e) in &r.entries {
            match e {
                Entry::Own { .. } => {
                    names.insert(name.clone());
                }
                Entry::Broken { target } => broken.push(Broken {
                    agent: r.agent,
                    name: name.clone(),
                    target: runner::display_path(&places.dir(r.agent).join(target), &places.home),
                }),
                Entry::Shared => {}
            }
        }
    }
    let mut rows: Vec<Row> = names.iter().map(|n| row(places, look, updates, n)).collect();
    rows.sort_by_key(|r| r.name.to_lowercase());

    let mut all_owned = Vec::new();
    if !look.system.is_empty() {
        all_owned.push(Owned {
            what: "Codex".into(),
            agent: Agent::Codex,
            names: look.system.clone(),
        });
    }
    all_owned.extend(owned);
    View { columns, rows, owned: all_owned, broken, lock_error: look.lock.as_ref().err().cloned() }
}

fn row(places: &SkillPlaces, look: &Look, updates: &skill_updates::Updates, name: &str) -> Row {
    let in_shared = look.shared.contains(name);
    let own = look.own(name);
    let home = if in_shared { None } else { own.first().copied() };
    let locked = look.locked(name);
    let (source, repo) = match locked {
        Some(l) if l.github() => ("github", Some(l.source.clone())),
        Some(l) if in_shared => ("other", Some(l.source.clone())),
        _ => ("hand", None),
    };
    let cells = look
        .readings
        .iter()
        .map(|r| {
            let a = r.agent;
            let entry = r.entries.get(name);
            let mut cell = if !r.present {
                Cell { state: "absent", lock: Some("readOnly") }
            } else if a == Agent::Codex {
                if look.codex_sees(name) {
                    let off = codex_off(places, &r.config, name);
                    Cell { state: if off { "off" } else { "on" }, lock: None }
                } else {
                    Cell { state: "absent", lock: Some("hand") }
                }
            } else {
                match (entry, in_shared) {
                    (Some(Entry::Shared), _) => Cell { state: "on", lock: None },
                    (Some(Entry::Own { .. }), true) => Cell { state: "on", lock: Some("taken") },
                    (Some(Entry::Own { .. }), false) => Cell { state: "on", lock: Some("hand") },
                    (Some(Entry::Broken { .. }), _) => Cell { state: "off", lock: Some("broken") },
                    (None, true) => Cell { state: "off", lock: None },
                    (None, false) => Cell { state: "off", lock: Some("hand") },
                }
            };
            if cell.lock.is_none() && !r.writable() {
                cell.lock = Some("readOnly");
            }
            (a, cell)
        })
        .collect();
    let folder = if in_shared {
        Some(places.shared.join(name))
    } else {
        home.map(|a| places.dir(a).join(name))
    };
    let edited = match (locked.and_then(|l| l.skill_folder_hash.as_deref()), in_shared) {
        (Some(hash), true) => {
            skill_updates::tree_hash(&places.shared.join(name)).is_ok_and(|h| h != hash)
        }
        _ => false,
    };
    let update = locked.filter(|_| in_shared).and_then(|l| {
        let found = updates.pending(name, l)?;
        Some(UpdateView {
            from_at: l.updated_at.clone().or_else(|| l.installed_at.clone()),
            seen_at: found.seen_at,
            plain: skill_updates::plain(&found.changes),
            link: skill_updates::link(&l.source, l.folder().as_deref()),
        })
    });
    let importable = !in_shared
        && own.len() == 1
        && look.reading(own[0]).writable()
        && (own[0] == Agent::Codex
            || look.reading(Agent::Codex).writable()
            || !look.reading(Agent::Codex).present);
    Row {
        name: name.to_string(),
        source,
        repo,
        home,
        cells,
        edited,
        update,
        size: folder.as_deref().map(size_of).unwrap_or(0),
        importable,
    }
}

/// The folder of a skill installed by hand: in the shared folder, or in the
/// one agent's folder that has it (where a link to elsewhere leads).
pub fn hand_folder(places: &SkillPlaces, look: &Look, name: &str) -> Option<PathBuf> {
    if look.shared.contains(name) {
        return Some(places.shared.join(name));
    }
    let agent = *look.own(name).first()?;
    std::fs::canonicalize(places.dir(agent).join(name)).ok()
}

/// What a backup pack keeps of the skills: each one's source and the
/// agents it is on for, and the folders of those installed by hand that
/// can go into it (none holding a line shaped like a key or one of
/// `known`). A skill from GitHub is installed from there again: no files.
pub fn packed(
    places: &SkillPlaces,
    look: &Look,
    known: &[String],
) -> (super::pack::SkillList, Vec<(String, PathBuf)>) {
    let mut names: BTreeSet<String> = look.shared.clone();
    for r in look.readings.iter().filter(|r| r.present) {
        names.extend(
            r.entries
                .iter()
                .filter(|(_, e)| matches!(e, Entry::Own { .. }))
                .map(|(n, _)| n.clone()),
        );
    }
    let mut folders = Vec::new();
    let skills = names
        .into_iter()
        .map(|name| {
            let locked = look.locked(&name).filter(|l| l.github() && look.shared.contains(&name));
            let mut s = super::pack::PackedSkill {
                repo: locked.map(|l| l.source.clone()),
                skill_path: locked.and_then(|l| l.skill_path.clone()),
                hand: locked.is_none(),
                on: Agent::ALL
                    .into_iter()
                    .filter(|a| look.reading(*a).present && look.on(places, &name, *a))
                    .collect(),
                copied: false,
                hash: None,
                exec: Vec::new(),
                left_out: None,
                name,
            };
            if s.hand {
                let checked = hand_folder(places, look, &s.name)
                    .ok_or("unreadable")
                    .and_then(|dir| super::pack::check_skill(&dir, known).map(|f| (dir, f)));
                match checked {
                    Ok((dir, files)) => match skill_updates::tree_hash(&dir) {
                        Ok(hash) => {
                            s.copied = true;
                            s.hash = Some(hash);
                            s.exec = files.iter().filter(|f| f.1).map(|f| f.0.clone()).collect();
                            folders.push((s.name.clone(), dir));
                        }
                        Err(_) => s.left_out = Some("unreadable".into()),
                    },
                    Err(why) => s.left_out = Some(why.into()),
                }
            }
            s
        })
        .collect();
    (super::pack::SkillList { skills }, folders)
}

// ===== The batch =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub name: String,
    pub agent: Agent,
    pub on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refused {
    pub name: String,
    pub agent: Agent,
    /// `readOnly`, `hand`, `taken`, `broken`, `badName`, `unknown` or
    /// `nothing` (it is so already).
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub agent: Agent,
    /// `~/.codex/config.toml`, or the folder of links.
    pub file: String,
    pub count: usize,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub groups: Vec<Group>,
    pub refused: Vec<Refused>,
    /// Sent back with the write.
    pub changes: Vec<Change>,
    pub bases: BTreeMap<Agent, String>,
}

/// Why a change cannot be made now.
fn refusal(places: &SkillPlaces, look: &Look, c: &Change) -> Option<&'static str> {
    if !valid_name(&c.name) {
        return Some("badName");
    }
    let r = look.reading(c.agent);
    if !r.writable() {
        return Some("readOnly");
    }
    if c.agent == Agent::Codex {
        if !look.codex_sees(&c.name) {
            return Some(if look.own(&c.name).is_empty() { "unknown" } else { "hand" });
        }
        let off = codex_off(places, &r.config, &c.name);
        return (off != c.on).then_some("nothing");
    }
    let in_shared = look.shared.contains(&c.name);
    match (r.entries.get(&c.name), c.on) {
        (Some(Entry::Shared), true) | (None, false) => Some("nothing"),
        (Some(Entry::Shared), false) => None,
        (None, true) if in_shared => None,
        (None, true) => Some(if look.own(&c.name).is_empty() { "unknown" } else { "hand" }),
        (Some(Entry::Own { .. }), _) => Some(if in_shared { "taken" } else { "hand" }),
        (Some(Entry::Broken { .. }), _) => Some("broken"),
    }
}

fn plan(places: &SkillPlaces, look: &Look, changes: &[Change]) -> (Vec<Change>, Vec<Refused>) {
    let mut accepted: Vec<Change> = Vec::new();
    let mut refused = Vec::new();
    for c in changes {
        if accepted.iter().any(|a| a.name == c.name && a.agent == c.agent) {
            continue;
        }
        match refusal(places, look, c) {
            None => accepted.push(c.clone()),
            Some(reason) => refused.push(Refused { name: c.name.clone(), agent: c.agent, reason }),
        }
    }
    (accepted, refused)
}

/// The path from `dir` to `to`, both absolute: what a link in `dir` holds,
/// as `npx skills` writes its own (`../../.agents/skills/x`).
pub fn relative(dir: &Path, to: &Path) -> PathBuf {
    let a: Vec<Component> = dir.components().collect();
    let b: Vec<Component> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in common..a.len() {
        out.push("..");
    }
    for c in &b[common..] {
        out.push(c.as_os_str());
    }
    out
}

fn link_line(places: &SkillPlaces, agent: Agent, name: &str) -> String {
    format!(
        "{} → {}",
        runner::display_path(&places.dir(agent).join(name), &places.home),
        runner::display_path(&places.shared.join(name), &places.home)
    )
}

fn line(op: &'static str, text: String) -> Line {
    Line { op, text, secret: false }
}

/// Reads the folders and `config.toml` now and shows what `changes` would
/// write: Codex's lines, the links made and removed.
pub fn preview(places: &SkillPlaces, changes: &[Change]) -> Preview {
    let look = read(places);
    let (accepted, mut refused) = plan(places, &look, changes);
    let mut groups = Vec::new();
    let mut bases = BTreeMap::new();
    let mut late = Vec::new();
    for agent in [Agent::Codex, Agent::Claude, Agent::Agy] {
        let mine: Vec<&Change> = accepted.iter().filter(|c| c.agent == agent).collect();
        if mine.is_empty() {
            continue;
        }
        let r = look.reading(agent);
        bases.insert(agent, r.base.clone());
        let lines = if agent == Agent::Codex {
            let text = r.text.clone().unwrap_or_default();
            let ops: Vec<(String, bool)> = mine.iter().map(|c| (c.name.clone(), c.on)).collect();
            match codex_edit(places, &text, &ops) {
                Ok(next) => {
                    let before = if text.trim().is_empty() {
                        Ok(String::new())
                    } else {
                        redact::toml(&text)
                    };
                    match (before, redact::toml(&next)) {
                        (Ok(a), Ok(b)) => batch::diff(&a, &b),
                        _ => Vec::new(),
                    }
                }
                Err(_) => {
                    late.extend(mine.iter().map(|c| (c.name.clone(), agent)));
                    continue;
                }
            }
        } else {
            mine.iter()
                .map(|c| line(if c.on { "add" } else { "del" }, link_line(places, agent, &c.name)))
                .collect()
        };
        groups.push(Group { agent, file: places.shown(agent), count: mine.len(), lines });
    }
    let mut accepted = accepted;
    for (name, agent) in late {
        accepted.retain(|c| !(c.name == name && c.agent == agent));
        refused.push(Refused { name, agent, reason: "readOnly" });
    }
    Preview { groups, refused, changes: accepted, bases }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Failure {
    /// The folder or the file is not what the preview read.
    Changed,
    /// The batch is not the one the preview showed.
    Refused {
        refused: Vec<Refused>,
    },
    /// `config.toml` could not be edited as planned.
    Edit {
        detail: String,
    },
    Write {
        detail: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub agent: Agent,
    pub changes: usize,
    pub backup: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failed {
    pub agent: Agent,
    pub error: Failure,
}

/// The batch that puts back what a write did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Undo {
    pub changes: Vec<Change>,
    pub bases: BTreeMap<Agent, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub done: Vec<Done>,
    pub failed: Option<Failed>,
    pub undo: Option<Undo>,
    pub at: i64,
}

impl Outcome {
    pub fn empty(at: i64) -> Self {
        Outcome { done: Vec::new(), failed: None, undo: None, at }
    }
}

/// Keeps what the folder's links are before a write, in the app's backups.
fn keep_links(
    app: &AppDir,
    places: &SkillPlaces,
    agent: Agent,
    entries: &BTreeMap<String, Entry>,
    now: OffsetDateTime,
) -> io::Result<PathBuf> {
    let list: BTreeMap<&String, String> = entries
        .iter()
        .map(|(n, e)| {
            let v = match e {
                Entry::Shared => runner::display_path(&places.shared.join(n), &places.home),
                Entry::Own { link: Some(t) } | Entry::Broken { target: t } => {
                    t.display().to_string()
                }
                Entry::Own { link: None } => "(folder)".to_string(),
            };
            (n, v)
        })
        .collect();
    let text = serde_json::to_string_pretty(&list).map_err(io::Error::other)? + "\n";
    let prefix = format!("{}-links.json.", agent_slug(agent));
    write::keep(text.as_bytes(), &app.path().join(BACKUP_DIR), &prefix, now)
}

fn agent_slug(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude",
        Agent::Codex => "codex",
        Agent::Agy => "antigravity",
    }
}

/// Makes the link `<dir>/<name>` to the shared folder's skill: only where
/// nothing is.
pub fn link(places: &SkillPlaces, agent: Agent, name: &str) -> io::Result<()> {
    let dir = places.dir(agent);
    std::fs::create_dir_all(dir)?;
    let at = dir.join(name);
    if std::fs::symlink_metadata(&at).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "something is there"));
    }
    os::link_dir(&relative(dir, &places.shared.join(name)), &at)
}

/// Removes the link `<dir>/<name>`, only when it is a link to the shared
/// folder's skill: never a folder.
pub fn unlink(places: &SkillPlaces, agent: Agent, name: &str) -> io::Result<()> {
    let at = places.dir(agent).join(name);
    let meta = std::fs::symlink_metadata(&at)?;
    let ours = meta.file_type().is_symlink()
        && std::fs::canonicalize(&at).ok() == std::fs::canonicalize(places.shared.join(name)).ok();
    if !ours {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a link to the shared skill"));
    }
    os::remove_link(&at)
}

/// Removes a link that points to nothing (`Bỏ liên kết`): never a folder.
pub fn drop_broken(places: &SkillPlaces, agent: Agent, name: &str) -> io::Result<()> {
    if !valid_name(name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad name"));
    }
    let at = places.dir(agent).join(name);
    let meta = std::fs::symlink_metadata(&at)?;
    if !meta.file_type().is_symlink() || std::fs::metadata(&at).is_ok() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a broken link"));
    }
    os::remove_link(&at)
}

/// Writes Codex's part of a batch: `config.toml` once, after its backup.
pub(crate) fn write_codex(
    places: &SkillPlaces,
    app: &AppDir,
    r: &Reading,
    ops: &[(String, bool)],
    now: OffsetDateTime,
) -> Result<Option<String>, Failure> {
    let base = r.text.clone();
    let text = codex_edit(places, base.as_deref().unwrap_or(""), ops)
        .map_err(|detail| Failure::Edit { detail })?;
    let (dir, prefix) = Agent::Codex.backups();
    let backups = app.path().join(dir);
    let clean = match &base {
        Some(b) => Some(redact::toml(b).map_err(|detail| Failure::Edit { detail })?),
        None => None,
    };
    let backup =
        clean.as_ref().map(|c| write::Backup { dir: &backups, prefix, bytes: c.as_bytes() });
    match write::write(&places.codex_config, base.as_deref().map(str::as_bytes), &text, backup, now)
    {
        Ok(kept) => {
            // The Codex switch's record follows the app's own write.
            let mut record = codex_files::load(app);
            let mcp = crate::codex::config::parse(&text, None)
                .map(|p| crate::codex::config::mcp_fingerprint(&p.mcp))
                .unwrap_or_default();
            record.follow(&r.base, &write::sha256(text.as_bytes()), mcp);
            let _ = codex_files::save(app, &record);
            Ok(kept.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())))
        }
        Err(write::WriteError::Changed) => Err(Failure::Changed),
        Err(e) => Err(Failure::Write { detail: format!("{e:?}") }),
    }
}

/// Writes `changes` when every folder and file they touch is still what
/// `bases` says. Codex's file first, then the links; the first failure
/// stops the batch, and what was written before it is in the undo.
pub fn write(
    places: &SkillPlaces,
    app: &AppDir,
    changes: &[Change],
    bases: &BTreeMap<Agent, String>,
    now: OffsetDateTime,
) -> Outcome {
    let at = now.unix_timestamp();
    let look = read(places);
    let (accepted, refused) = plan(places, &look, changes);
    let mut out = Outcome::empty(at);
    if !refused.is_empty() {
        out.failed = Some(Failed { agent: refused[0].agent, error: Failure::Refused { refused } });
        return out;
    }
    let agents: BTreeSet<Agent> = accepted.iter().map(|c| c.agent).collect();
    for a in &agents {
        if bases.get(a) != Some(&look.reading(*a).base) {
            out.failed = Some(Failed { agent: *a, error: Failure::Changed });
            return out;
        }
    }
    let mut undo: Vec<Change> = Vec::new();
    for agent in [Agent::Codex, Agent::Claude, Agent::Agy] {
        let mine: Vec<&Change> = accepted.iter().filter(|c| c.agent == agent).collect();
        if mine.is_empty() {
            continue;
        }
        let r = look.reading(agent);
        let result = if agent == Agent::Codex {
            let ops: Vec<(String, bool)> = mine.iter().map(|c| (c.name.clone(), c.on)).collect();
            write_codex(places, app, r, &ops, now).map(|b| (b, mine.len()))
        } else {
            (|| {
                let kept = keep_links(app, places, agent, &r.entries, now)
                    .map_err(|e| Failure::Write { detail: e.to_string() })?;
                // Read again just before: a folder changed since the plan is not touched.
                let again = scan(places.dir(agent), &places.shared)
                    .map_err(|e| Failure::Write { detail: e })?;
                if listing_hash(&again) != r.base {
                    return Err(Failure::Changed);
                }
                let mut n = 0;
                for c in &mine {
                    let done = if c.on {
                        link(places, agent, &c.name)
                    } else {
                        unlink(places, agent, &c.name)
                    };
                    if let Err(e) = done {
                        if n > 0 {
                            undo.extend(
                                mine[..n].iter().map(|c| Change { on: !c.on, ..(*c).clone() }),
                            );
                        }
                        return Err(Failure::Write { detail: e.to_string() });
                    }
                    n += 1;
                }
                Ok((kept.file_name().map(|n| n.to_string_lossy().to_string()), n))
            })()
        };
        match result {
            Ok((backup, n)) => {
                undo.extend(mine.iter().map(|c| Change { on: !c.on, ..(*c).clone() }));
                out.done.push(Done { agent, changes: n, backup });
            }
            Err(error) => {
                out.failed = Some(Failed { agent, error });
                break;
            }
        }
    }
    if !undo.is_empty() {
        let after = read(places);
        let bases = undo.iter().map(|c| (c.agent, after.reading(c.agent).base.clone())).collect();
        undo.reverse();
        out.undo = Some(Undo { changes: undo, bases });
    }
    out
}

// ===== Copies of skill folders =====

/// The most a skill folder the app copies may hold.
const COPY_MAX_BYTES: u64 = 100 * 1024 * 1024;
const COPY_MAX_ENTRIES: usize = 5000;

/// Copies the folder `from` to `to` (which must not be there): files with
/// their modes, links as links. Nothing in it is run or followed.
pub fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    let mut bytes = 0u64;
    let mut entries = 0usize;
    fn go(from: &Path, to: &Path, bytes: &mut u64, entries: &mut usize) -> io::Result<()> {
        std::fs::create_dir(to)?;
        for e in std::fs::read_dir(from)? {
            let e = e?;
            *entries += 1;
            if *entries > COPY_MAX_ENTRIES {
                return Err(io::Error::other("too many files"));
            }
            let src = e.path();
            let dst = to.join(e.file_name());
            let meta = std::fs::symlink_metadata(&src)?;
            if meta.file_type().is_symlink() {
                os::copy_link(&src, &dst)?;
            } else if meta.is_dir() {
                go(&src, &dst, bytes, entries)?;
            } else if meta.is_file() {
                *bytes += meta.len();
                if *bytes > COPY_MAX_BYTES {
                    return Err(io::Error::other("larger than 100 MB"));
                }
                let mut input = std::fs::File::open(&src)?;
                let mut output = os::with_mode(
                    std::fs::OpenOptions::new().write(true).create_new(true),
                    os::mode(&meta) & 0o777,
                )
                .open(&dst)?;
                io::copy(&mut input, &mut output)?;
                output.sync_all()?;
            }
        }
        Ok(())
    }
    go(from, to, &mut bytes, &mut entries)
}

/// Keeps a copy of a skill's folder in the app's backups before the app
/// moves, updates or removes it; the newest 10 copies of each skill stay.
pub fn keep_folder(
    app: &AppDir,
    name: &str,
    folder: &Path,
    now: OffsetDateTime,
) -> io::Result<String> {
    let root = app.path().join(BACKUP_DIR);
    std::fs::create_dir_all(&root)?;
    os::set_mode(&root, 0o700)?;
    let stamp = local_time::stamp(now).replace(['-', ':'], "").replace('T', "-");
    let prefix = format!("{name}.folder.");
    let mut id = format!("{prefix}{stamp}");
    let mut n = 2;
    while root.join(&id).exists() {
        id = format!("{prefix}{stamp}-{n}");
        n += 1;
    }
    let real = std::fs::canonicalize(folder)?;
    copy_tree(&real, &root.join(&id))?;
    for old in write::list(&root, &prefix).into_iter().skip(write::BACKUPS_KEPT) {
        // Only copies the app made, by their name.
        if old.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefix)) {
            let _ = std::fs::remove_dir_all(old);
        }
    }
    Ok(id)
}

// ===== Gathering skills installed by hand =====

/// One skill gathered into the shared folder, for the undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Moved {
    pub name: String,
    pub home: Agent,
    /// The link it was, when it was a link to a folder elsewhere: the
    /// folder was copied, the link replaced.
    pub link: Option<PathBuf>,
    /// The tree hash of what was put in the shared folder.
    pub hash: String,
    /// Codex was given an entry turning it off, since it did not read it before.
    pub codex_off: bool,
    pub backup: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gathered {
    pub moved: Vec<Moved>,
    /// The skill that stopped the rest, and why.
    pub failed: Option<(String, String)>,
    pub at: i64,
}

/// Gathers each skill of `names`, installed by hand in one agent's folder,
/// into the shared folder: a copy is kept first, the folder is moved (or,
/// for a link to a folder elsewhere, copied), and the old place becomes a
/// link to it. Agents that had it still have it; Codex, which reads the
/// shared folder, gets an entry turning it off when it did not read it.
pub fn import(
    places: &SkillPlaces,
    app: &AppDir,
    names: &[String],
    now: OffsetDateTime,
) -> Gathered {
    let mut out = Gathered { moved: Vec::new(), failed: None, at: now.unix_timestamp() };
    for name in names {
        match import_one(places, app, name, now) {
            Ok(m) => out.moved.push(m),
            Err(e) => {
                out.failed = Some((name.clone(), e));
                break;
            }
        }
    }
    out
}

fn import_one(
    places: &SkillPlaces,
    app: &AppDir,
    name: &str,
    now: OffsetDateTime,
) -> Result<Moved, String> {
    let look = read(places);
    if !valid_name(name) || look.shared.contains(name) || places.shared.join(name).exists() {
        return Err("already in the shared folder".into());
    }
    let own = look.own(name);
    let [home] = own.as_slice() else { return Err("not in exactly one agent's folder".into()) };
    let home = *home;
    let r = look.reading(home);
    let codex = look.reading(Agent::Codex);
    if !r.writable() || (home != Agent::Codex && codex.present && !codex.writable()) {
        return Err("read only".into());
    }
    let Some(Entry::Own { link: was_link }) = r.entries.get(name).cloned() else {
        return Err("not a skill of its own".into());
    };
    let at = places.dir(home).join(name);
    let backup = keep_folder(app, name, &at, now).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&places.shared).map_err(|e| e.to_string())?;
    let dest = places.shared.join(name);
    match &was_link {
        None => std::fs::rename(&at, &dest).map_err(|e| e.to_string())?,
        Some(_) => {
            let real = std::fs::canonicalize(&at).map_err(|e| e.to_string())?;
            copy_tree(&real, &dest).map_err(|e| e.to_string())?;
            std::fs::remove_file(&at).map_err(|e| e.to_string())?;
        }
    }
    let hash = skill_updates::tree_hash(&dest).map_err(|e| e.to_string())?;
    if home != Agent::Codex {
        link(places, home, name).map_err(|e| e.to_string())?;
    }
    // Codex now reads it from the shared folder; it stays as it was.
    let mut codex_off = false;
    if home != Agent::Codex && codex.present {
        let text = codex.text.clone();
        let fresh = read_small(&places.codex_config).ok().flatten();
        if fresh != text {
            return Err("config.toml changed".into());
        }
        write_codex(places, app, codex, &[(name.to_string(), false)], now)
            .map_err(|e| format!("{e:?}"))?;
        codex_off = true;
    }
    Ok(Moved { name: name.to_string(), home, link: was_link, hash, codex_off, backup })
}

/// Puts gathered skills back where they were, when the shared folder still
/// holds what was put there.
pub fn unimport(
    places: &SkillPlaces,
    app: &AppDir,
    moved: &[Moved],
    now: OffsetDateTime,
) -> Result<usize, String> {
    let mut n = 0;
    for m in moved.iter().rev() {
        let dest = places.shared.join(&m.name);
        if skill_updates::tree_hash(&dest).ok().as_deref() != Some(m.hash.as_str()) {
            return Err(format!("{} changed in the shared folder", m.name));
        }
        let at = places.dir(m.home).join(&m.name);
        if m.home != Agent::Codex {
            unlink(places, m.home, &m.name).map_err(|e| e.to_string())?;
        }
        if std::fs::symlink_metadata(&at).is_ok() {
            return Err(format!("{} is in the agent's folder again", m.name));
        }
        match &m.link {
            None => std::fs::rename(&dest, &at).map_err(|e| e.to_string())?,
            Some(target) => {
                os::link_dir(target, &at).map_err(|e| e.to_string())?;
                // The copy the gathering made, unchanged: nothing else is lost.
                std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
            }
        }
        if m.codex_off {
            let look = read(places);
            let r = look.reading(Agent::Codex);
            write_codex(places, app, r, &[(m.name.clone(), true)], now)
                .map_err(|e| format!("{e:?}"))?;
        }
        n += 1;
    }
    Ok(n)
}

/// Writes a small JSON file of the app's, mode 600.
pub(crate) fn save_json<T: Serialize>(app: &AppDir, rel: &str, v: &T) -> io::Result<()> {
    let text = serde_json::to_string_pretty(v).map_err(io::Error::other)?;
    app.write_private(&app.path().join(rel), &text)
}

pub(crate) fn load_json<T: for<'de> Deserialize<'de> + Default>(app: &AppDir, rel: &str) -> T {
    app.read_to_string(&app.path().join(rel))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_relative_as_npx_skills_writes_them() {
        assert_eq!(
            relative(Path::new("/Users/a/.claude/skills"), Path::new("/Users/a/.agents/skills/x")),
            PathBuf::from("../../.agents/skills/x")
        );
        assert_eq!(
            relative(
                Path::new("/Users/a/.gemini/config/skills"),
                Path::new("/Users/a/.agents/skills/x")
            ),
            PathBuf::from("../../../.agents/skills/x")
        );
    }

    #[test]
    fn repos_and_names() {
        assert!(repo_ok("acme/agent-skills"));
        assert!(!repo_ok("acme"));
        assert!(!repo_ok("acme/../x"));
        assert!(valid_name("pdf-tools"));
        assert!(!valid_name("../x"));
        assert!(!valid_name("-rf"));
    }
}
