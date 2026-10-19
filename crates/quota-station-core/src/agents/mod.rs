//! The MCP servers of Claude Code, Codex and Antigravity in one table (M9):
//! each server a row, each agent a column of checkboxes, ticked quickly and
//! written in one batch after one preview ([`batch`]).
//!
//! Where each agent keeps its list, and the only part of it the app edits:
//!
//! - Claude Code: the top-level `mcpServers` of `~/.claude.json`, read only;
//!   `claude mcp add-json`/`remove --scope user` write it ([`claude_mcp`]).
//! - Codex: the `[mcp_servers.<name>]` tables of `config.toml` in the Codex
//!   folder ([`codex_mcp`]); never the provider keys the Codex switch owns.
//! - Antigravity: `mcpServers` in `~/.gemini/config/mcp_config.json`
//!   ([`agy_mcp`]).
//!
//! Each write is checked as the Codex switch's is ([`write`]), and the
//! backup taken before it leaves the secrets out ([`redact`]). Secrets the
//! app must keep itself go to its own Keychain items ([`secrets`]); servers
//! in no agent's file go to its store ([`kho`]). A server whose command
//! lives in an app's bundle (Codex App's own) is shown, never edited.
//!
//! [`look`] builds what the windows see. It reads the files, keeps no
//! secret (only which secrets exist), and drops the text.
//!
//! M11 adds the rest of the Agent group: the skills ([`skills`], written
//! the same way, installed and updated through `npx skills` in [`npx`] and
//! [`skill_ops`], newer versions found by [`skill_updates`]), the plugins
//! ([`plugins`], read only) and the instruction files ([`instructions`],
//! read only).
//!
//! M14 adds [`env_check`]: the environment variables in the shell files and
//! launchd that override an agent's login, found by name, never by value.
//!
//! M15 adds [`sets`]: named sets of MCP servers and skills turned on or off
//! for the whole Mac, applied through the same checked batches after one
//! preview; Codex's provider stays with the Codex switch.

pub mod agy_mcp;
pub mod batch;
pub mod claude_mcp;
pub mod codex_mcp;
pub mod copy;
pub mod def;
pub mod env_check;
pub mod instructions;
pub mod json_text;
pub mod kho;
pub mod npx;
pub mod pack;
pub mod paste;
pub mod plugins;
pub mod redact;
pub mod restore;
pub mod secrets;
pub mod sets;
pub mod skill_ops;
pub mod skill_updates;
pub mod skills;
pub mod write;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};

use crate::antigravity::fs::AppDir;
use crate::codex::files as codex_files;
use crate::runner;
use def::{Def, Place, MASK};
use secrets::Secrets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Agent {
    Claude,
    Codex,
    Agy,
}

impl Agent {
    pub const ALL: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::Agy];

    pub fn label(self) -> &'static str {
        match self {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Agy => "Antigravity",
        }
    }

    /// Its backups' folder in the app's folder, and their names' start.
    /// Codex's are the ones the Codex switch keeps: one file, one history.
    pub fn backups(self) -> (&'static str, &'static str) {
        match self {
            Agent::Claude => ("backups/claude", "mcpServers.json."),
            Agent::Codex => (codex_files::BACKUP_DIR, codex_files::BACKUP_PREFIX),
            Agent::Agy => ("backups/antigravity", "mcp_config.json."),
        }
    }
}

/// Why an edit of one server's entry was not made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum EditError {
    /// The file is not the JSON or TOML the agent reads.
    Parse(String),
    /// The server is not in the file.
    Missing,
    /// A server of that name is there already.
    Exists,
    /// Written in a shape the app does not edit (inline, dotted keys,
    /// twice).
    Shape,
    /// The agent cannot run it (Codex and an SSE server).
    Unsupported,
    /// The edited text changed more than the one server, or does not say
    /// what the edit says: nothing is written.
    Check(&'static str),
}

/// One server in one agent's list. Its `def` holds the secrets: it lives
/// in memory only, while a look, a preview or a write runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub name: String,
    pub enabled: bool,
    /// `None`: neither a command nor an address; shown, not edited.
    pub def: Option<Def>,
    /// Members the app does not read: they stay where they are and are not
    /// copied to another agent.
    pub extra: Vec<String>,
    /// Its entry has a shape the app edits.
    pub editable: bool,
    /// The app whose bundle runs it: that app's own server.
    pub owner: Option<String>,
}

impl Server {
    pub fn new(
        name: &str,
        enabled: bool,
        def: Option<Def>,
        extra: Vec<String>,
        editable: bool,
    ) -> Self {
        let owner = def.as_ref().and_then(owner_of);
        Server { name: name.to_string(), enabled, def, extra, editable, owner }
    }
}

/// The app a command lives in: `/Applications/X.app/Contents/…` is X's own.
fn owner_of(def: &Def) -> Option<String> {
    let Def::Stdio { command, cwd, .. } = def else { return None };
    [Some(command.as_str()), cwd.as_deref()].into_iter().flatten().find_map(|p| {
        let at = p.find(".app/")?;
        let name = p[..at].rsplit('/').next()?;
        (!name.is_empty()).then(|| name.to_string())
    })
}

/// A name the app writes: what all three agents and `claude`'s command
/// line take.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with(['-', '.'])
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Where each agent's list is on this Mac.
#[derive(Debug, Clone)]
pub struct Places {
    pub home: PathBuf,
    pub claude_json: PathBuf,
    /// `claude`, which writes Claude Code's list; without it that column is
    /// read only.
    pub claude_cli: Option<PathBuf>,
    pub codex_config: PathBuf,
    pub agy_config: PathBuf,
}

impl Places {
    pub fn new(home: &Path, codex_home: &Path, claude_cli: Option<PathBuf>) -> Self {
        Places {
            home: home.to_path_buf(),
            claude_json: home.join(claude_mcp::FILE),
            claude_cli,
            codex_config: codex_home.join(crate::codex::CONFIG),
            agy_config: home.join(".gemini/config/mcp_config.json"),
        }
    }

    pub fn file(&self, agent: Agent) -> &Path {
        match agent {
            Agent::Claude => &self.claude_json,
            Agent::Codex => &self.codex_config,
            Agent::Agy => &self.agy_config,
        }
    }

    /// The agent is on this Mac: its file or the folder it keeps it in.
    pub fn present(&self, agent: Agent) -> bool {
        match agent {
            Agent::Claude => self.claude_json.is_file() || self.claude_cli.is_some(),
            Agent::Codex | Agent::Agy => self.file(agent).parent().is_some_and(Path::is_dir),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Missing,
    Ok,
    /// Not what the agent reads: shown, never written.
    Invalid(String),
    Unreadable(String),
}

/// One agent's list as read now. `text` and `entries` hold secrets.
#[derive(Debug, Clone)]
pub struct Reading {
    pub agent: Agent,
    pub present: bool,
    pub state: FileState,
    /// What a write compares: the file's SHA-256 (Codex, Antigravity), the
    /// list's (Claude Code); empty when there is no file.
    pub base: String,
    pub servers: Vec<Server>,
    /// Codex's and Antigravity's file as read.
    pub text: Option<String>,
    /// Claude Code's list as read.
    pub entries: Option<Map<String, Json>>,
}

impl Reading {
    pub fn ok(&self) -> bool {
        matches!(self.state, FileState::Ok | FileState::Missing) && self.present
    }

    pub fn server(&self, name: &str) -> Option<&Server> {
        self.servers.iter().find(|s| s.name == name)
    }
}

/// `config.toml` and `mcp_config.json` are a few kilobytes.
const MAX_FILE: u64 = 4 * 1024 * 1024;

fn read_text(file: &Path) -> Result<Option<String>, String> {
    use std::io::Read;
    let f = match std::fs::File::open(file) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    f.take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("larger than 4 MB".into());
    }
    String::from_utf8(bytes).map(Some).map_err(|_| "not UTF-8".to_string())
}

/// Reads one agent's list.
pub fn read(places: &Places, agent: Agent) -> Reading {
    let present = places.present(agent);
    let mut r = Reading {
        agent,
        present,
        state: FileState::Missing,
        base: String::new(),
        servers: Vec::new(),
        text: None,
        entries: None,
    };
    if !present {
        return r;
    }
    match agent {
        Agent::Claude => match claude_mcp::read(&places.claude_json) {
            Ok(None) => {}
            Ok(Some(entries)) => {
                r.state = FileState::Ok;
                r.base = claude_mcp::hash(&entries);
                r.servers = claude_mcp::servers(&entries);
                r.entries = Some(entries);
            }
            Err(e) => r.state = FileState::Invalid(e),
        },
        Agent::Codex | Agent::Agy => match read_text(places.file(agent)) {
            Ok(None) => {}
            Ok(Some(text)) => {
                r.base = write::sha256(text.as_bytes());
                let servers = if agent == Agent::Codex {
                    codex_mcp::read(&text)
                } else {
                    agy_mcp::read(&text)
                };
                match servers {
                    Ok(s) => {
                        r.state = FileState::Ok;
                        r.servers = s;
                    }
                    Err(e) => r.state = FileState::Invalid(e),
                }
                r.text = Some(text);
            }
            Err(e) => r.state = FileState::Unreadable(e),
        },
    }
    r
}

pub fn read_all(places: &Places) -> Vec<Reading> {
    Agent::ALL.iter().map(|a| read(places, *a)).collect()
}

/// The column can be written: the agent is here, its file reads (or is not
/// there yet), and for Claude Code `claude` is found.
pub fn writable(places: &Places, r: &Reading) -> bool {
    r.ok() && (r.agent != Agent::Claude || places.claude_cli.is_some())
}

// ===== Commands on this Mac =====

/// Folders a command named without a path is usually in.
pub(crate) fn search_dirs(home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![
        home.join(".local/bin"),
        home.join(".cargo/bin"),
        home.join(".bun/bin"),
        home.join(".deno/bin"),
    ];
    out.extend(
        runner::candidates(home).into_iter().filter_map(|p| p.parent().map(Path::to_path_buf)),
    );
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
        out.push(PathBuf::from(d));
    }
    out.dedup();
    out
}

/// A command that is not on this Mac.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingCommand {
    pub command: String,
    /// A full path that is not there: the server cannot start. A bare name
    /// not found in the usual folders may still be on the agent's own PATH.
    pub path: bool,
}

pub fn missing_command(def: &Def, home: &Path) -> Option<MissingCommand> {
    let Def::Stdio { command, cwd, .. } = def else { return None };
    if command.contains('/') {
        let p = Path::new(command);
        let p = match (p.is_absolute(), cwd) {
            (true, _) => p.to_path_buf(),
            (false, Some(cwd)) => Path::new(cwd).join(p),
            (false, None) => return None,
        };
        let p = runner::expand_path(&p.to_string_lossy(), home);
        return (!p.exists()).then(|| MissingCommand { command: command.clone(), path: true });
    }
    let found = search_dirs(home).iter().any(|d| d.join(command).is_file());
    (!found).then(|| MissingCommand { command: command.clone(), path: false })
}

// ===== What the windows see =====

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// Servers of an agent's or app's own: shown, never edited.
    pub owned: Vec<Owned>,
    /// Servers put away from every agent.
    pub stored: Vec<StoredRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub agent: Agent,
    pub present: bool,
    /// `~/.codex/config.toml`
    pub file: String,
    /// `ok`, `missing` (no file yet), `invalid`, `unreadable`, `absent`
    /// (the agent is not on this Mac) or `noCli` (no `claude`).
    pub state: &'static str,
    pub reason: Option<String>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    /// `on`, `off` (in the file and off, or kept by the app for Claude
    /// Code) or `absent`.
    pub state: &'static str,
    /// Why the box cannot be ticked: `readOnly`, `shape`, `missingKey`,
    /// `noCommand` or `sse`.
    pub lock: Option<&'static str>,
    /// Off and kept by the app: Claude Code's server turned off, or one a
    /// restore added off until its key or command is here.
    pub parked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretView {
    pub id: String,
    pub label: String,
    pub place: Place,
    /// In an agent's file or the app's Keychain item.
    pub have: bool,
    pub bitwarden: Option<String>,
}

/// One agent's definition, secrets masked, for the edit form and the
/// side-by-side comparison.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefView {
    pub def: Def,
    pub slots: Vec<String>,
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub name: String,
    /// `stdio`, `http`, `sse` or `other`.
    pub kind: &'static str,
    pub program: Option<String>,
    pub host: Option<String>,
    pub secrets: Vec<SecretView>,
    pub cells: BTreeMap<Agent, Cell>,
    /// Not the same server in every agent that has it.
    pub differs: bool,
    pub missing_command: Option<MissingCommand>,
    pub defs: BTreeMap<Agent, DefView>,
    /// For a secret the agents differ on, only whether they do.
    pub same_secrets: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Owned {
    pub name: String,
    pub owner: String,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRow {
    pub name: String,
    pub kind: &'static str,
    pub program: Option<String>,
    pub host: Option<String>,
    /// Unix seconds.
    pub at: i64,
    pub agents: Vec<Agent>,
    pub def: DefView,
    /// Secrets the Keychain no longer holds.
    pub missing: Vec<String>,
}

fn kind(def: Option<&Def>) -> &'static str {
    match def {
        Some(Def::Stdio { .. }) => "stdio",
        Some(Def::Http { sse: false, .. }) => "http",
        Some(Def::Http { sse: true, .. }) => "sse",
        None => "other",
    }
}

pub(crate) fn def_view(def: &Def, extra: &[String]) -> DefView {
    DefView {
        def: def.masked(MASK),
        slots: def.slots().into_iter().map(|s| s.id).collect(),
        extra: extra.to_vec(),
    }
}

/// A stored definition (secrets [`def::REDACTED`]) for the windows.
pub(crate) fn kept_view(kept: &kho::Kept) -> DefView {
    let mut def = kept.def.clone();
    for slot in &kept.slots {
        def.set(slot, MASK);
    }
    DefView { def, slots: kept.slots.clone(), extra: kept.extra.keys().cloned().collect() }
}

/// What a server's secrets are, wherever the app finds them: an agent's
/// file first, then its Keychain item.
pub(crate) struct Have<'a> {
    pub readings: &'a [Reading],
    pub secrets: &'a Secrets,
}

impl Have<'_> {
    /// The value of `slot` in some agent's entry for `name`.
    pub fn in_files(&self, name: &str, slot: &str, prefer: Option<Agent>) -> Option<String> {
        let mut order: Vec<&Reading> = self.readings.iter().collect();
        order.sort_by_key(|r| Some(r.agent) != prefer);
        order.into_iter().find_map(|r| {
            let v = r.server(name)?.def.as_ref()?.get(slot)?;
            (!v.is_empty() && v != def::REDACTED && v != MASK).then(|| v.to_string())
        })
    }

    pub fn exists(&self, name: &str, slot: &str) -> bool {
        self.in_files(name, slot, None).is_some() || self.secrets.has(name, slot).unwrap_or(false)
    }
}

/// Reads every agent's list and builds what the windows see.
pub fn look(places: &Places, app: &AppDir, secrets: &Secrets) -> View {
    let readings = read_all(places);
    let store = kho::load(app);
    view(places, &readings, &store, secrets)
}

pub fn view(places: &Places, readings: &[Reading], store: &kho::Kho, secrets: &Secrets) -> View {
    let have = Have { readings, secrets };
    let columns = readings
        .iter()
        .map(|r| {
            let (state, reason) = match (&r.state, r.present) {
                (_, false) => ("absent", None),
                (FileState::Invalid(e), _) => ("invalid", Some(e.clone())),
                (FileState::Unreadable(e), _) => ("unreadable", Some(e.clone())),
                _ if r.agent == Agent::Claude && places.claude_cli.is_none() => ("noCli", None),
                (FileState::Missing, _) => ("missing", None),
                (FileState::Ok, _) => ("ok", None),
            };
            Column {
                agent: r.agent,
                present: r.present,
                file: runner::display_path(places.file(r.agent), &places.home),
                state,
                reason,
                writable: writable(places, r),
            }
        })
        .collect();

    let mut owned: BTreeMap<String, Owned> = BTreeMap::new();
    let mut names: BTreeSet<String> = BTreeSet::new();
    for r in readings.iter().filter(|r| r.ok()) {
        for s in &r.servers {
            match &s.owner {
                Some(owner) => {
                    let o = owned.entry(s.name.clone()).or_insert_with(|| Owned {
                        name: s.name.clone(),
                        owner: owner.clone(),
                        agents: Vec::new(),
                    });
                    o.agents.push(r.agent);
                }
                None => {
                    names.insert(s.name.clone());
                }
            }
        }
    }
    let claude_ok = readings.iter().any(|r| r.agent == Agent::Claude && r.ok());
    if claude_ok {
        names.extend(store.parked.keys().cloned());
    }
    // A restored server waiting for its key or command, where it waits.
    for (name, k) in &store.waiting {
        if readings.iter().any(|r| r.ok() && k.agents.contains(&r.agent)) {
            names.insert(name.clone());
        }
    }

    let mut rows: Vec<Row> = names
        .into_iter()
        .filter(|n| !owned.contains_key(n))
        .map(|name| row(places, readings, store, &have, &name))
        .collect();
    rows.sort_by_key(|r| r.name.to_lowercase());

    let stored = store
        .stored
        .iter()
        .map(|(name, k)| StoredRow {
            name: name.clone(),
            kind: kind(Some(&k.def)),
            program: k.def.program(),
            host: k.def.host(),
            at: k.at,
            agents: k.agents.clone(),
            def: kept_view(k),
            missing: k.slots.iter().filter(|s| !have.exists(name, s)).cloned().collect(),
        })
        .collect();

    View { columns, rows, owned: owned.into_values().collect(), stored }
}

fn row(
    places: &Places,
    readings: &[Reading],
    store: &kho::Kho,
    have: &Have<'_>,
    name: &str,
) -> Row {
    let parked = store.parked.get(name);
    let waiting = store.waiting.get(name);
    // Every definition of it, for the comparison and the edit form.
    let mut defs: BTreeMap<Agent, DefView> = BTreeMap::new();
    let mut found: Vec<(Agent, &Def)> = Vec::new();
    for r in readings.iter().filter(|r| r.ok()) {
        if let Some(s) = r.server(name) {
            if let Some(d) = &s.def {
                defs.insert(r.agent, def_view(d, &s.extra));
                found.push((r.agent, d));
            }
        }
    }
    let claude_has = readings.iter().any(|r| r.agent == Agent::Claude && r.server(name).is_some());
    if let (Some(k), false) = (parked, claude_has) {
        defs.insert(Agent::Claude, kept_view(k));
    }
    if let Some(k) = waiting {
        for a in &k.agents {
            defs.entry(*a).or_insert_with(|| kept_view(k));
        }
    }
    let first_def: Option<Def> = found
        .first()
        .map(|(_, d)| (*d).clone())
        .or_else(|| parked.or(waiting).map(|k| k.def.clone()));
    let def = first_def.as_ref();

    // The secrets: every slot any of its definitions has.
    let mut slots: Vec<def::Slot> = Vec::new();
    for (_, d) in &found {
        for s in d.slots() {
            if !slots.iter().any(|x| x.id == s.id) {
                slots.push(s);
            }
        }
    }
    for k in parked.into_iter().chain(waiting) {
        for id in &k.slots {
            if !slots.iter().any(|x| &x.id == id) {
                let place = match id.split_once(':').map(|(p, _)| p) {
                    Some("env") => Place::Env,
                    Some("header") => Place::Header,
                    Some("arg") => Place::Arg,
                    _ => Place::Url,
                };
                let label =
                    id.split_once(':').map(|(_, l)| l.to_string()).unwrap_or_else(|| "URL".into());
                slots.push(def::Slot { id: id.clone(), label, place });
            }
        }
    }
    let secrets: Vec<SecretView> = slots
        .iter()
        .map(|s| SecretView {
            id: s.id.clone(),
            label: s.label.clone(),
            place: s.place.clone(),
            have: have.exists(name, &s.id),
            bitwarden: store.hint(name, &s.id).map(str::to_string),
        })
        .collect();
    let missing_key = secrets.iter().any(|s| !s.have);

    let mut same_secrets = BTreeMap::new();
    for s in &slots {
        let values: Vec<&str> = found.iter().filter_map(|(_, d)| d.get(&s.id)).collect();
        if values.len() > 1 {
            same_secrets.insert(s.id.clone(), values.windows(2).all(|w| w[0] == w[1]));
        }
    }
    let differs = found.windows(2).any(|w| !w[0].1.same(w[1].1))
        || (parked.is_some()
            && !claude_has
            && found.iter().any(|(_, d)| !d.stripped().same(&parked.expect("parked").def)));
    let missing = def.and_then(|d| missing_command(d, &places.home));

    let cells = readings
        .iter()
        .map(|r| {
            let server = r.server(name);
            let is_parked = server.is_none()
                && ((r.agent == Agent::Claude && parked.is_some()) || store.waits(name, r.agent));
            let state = match server {
                Some(s) if s.enabled => "on",
                Some(_) => "off",
                None if is_parked => "off",
                None => "absent",
            };
            let lock = if !writable(places, r) {
                Some("readOnly")
            } else if server.is_some_and(|s| !s.editable || s.def.is_none()) {
                Some("shape")
            } else if state != "on" && missing.as_ref().is_some_and(|m| m.path) {
                Some("noCommand")
            } else if state == "absent" && r.agent == Agent::Codex && def.is_some_and(Def::is_sse) {
                Some("sse")
            } else if (state == "absent" || is_parked) && missing_key {
                Some("missingKey")
            } else if state == "absent" && def.is_none() {
                Some("shape")
            } else {
                None
            };
            (r.agent, Cell { state, lock, parked: is_parked })
        })
        .collect();

    Row {
        name: name.to_string(),
        kind: kind(def),
        program: def.and_then(Def::program),
        host: def.and_then(Def::host),
        secrets,
        cells,
        differs,
        missing_command: missing,
        defs,
        same_secrets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_s_own_server_is_known_by_its_bundle() {
        let d = |command: &str, cwd: Option<&str>| Def::Stdio {
            command: command.into(),
            args: vec![],
            env: vec![],
            cwd: cwd.map(str::to_string),
        };
        assert_eq!(
            owner_of(&d("/Applications/Codex.app/Contents/Resources/node_repl", None)).as_deref(),
            Some("Codex")
        );
        assert_eq!(
            owner_of(&d("./Helper.app/Contents/MacOS/helper", Some("/x/plugins/y"))).as_deref(),
            Some("Helper")
        );
        assert_eq!(owner_of(&d("npx", None)), None);
    }

    #[test]
    fn names_the_app_writes() {
        assert!(valid_name("docs-search"));
        assert!(valid_name("node_repl"));
        assert!(!valid_name("--help"));
        assert!(!valid_name("a b"));
        assert!(!valid_name(""));
    }
}
