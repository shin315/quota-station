//! Backup packs (M10, the MCP part of C): a folder per backup, by default
//! in iCloud Drive's `Quota Station Agents`, so a Mac set up again (or
//! another Mac) can take its MCP servers back. A pack holds no secret:
//!
//! - `manifest.json`: the Mac, the time, the app's version, what is in it;
//! - `mcp.json`: each server, and for each agent that has it whether it is
//!   on and its definition, every secret [`REDACTED`], paths under the home
//!   folder written `~/` ([`portable`]);
//! - `secrets-needed.json`: each secret's MCP, slot, name and the Bitwarden
//!   item it is in. No value, and no hash of one.
//!
//! The sections after `mcp` (M11), each named in the manifest's `sections`
//! and read only when it is:
//!
//! - `skills.json`: each skill, its GitHub source when it has one, whether
//!   it was installed by hand, and the agents it is on for. No skill's
//!   files yet.
//! - `plugins.json`: Claude Code's and Codex's plugins (`name@marketplace`,
//!   on or off) and the marketplaces' sources. When an agent's list could
//!   not be read, the newest pack's list for it is kept, marked so.
//! - `instructions.json`: the instruction files the owner keeps in, each
//!   with its text. A file with a line shaped like a key is never here.
//!
//! The servers an app brings itself (Codex App's own) are not in it, nor
//! are logins, accounts or Codex's provider. Every file is checked before
//! it is written: no string shaped like a secret, and none of the values in
//! the agents' files ([`write`] refuses otherwise). The newest [`KEPT`]
//! packs of each Mac stay; an older one loses only the files the app wrote.
//!
//! iCloud Drive may list a pack it has not downloaded yet: the folder holds
//! `.manifest.json.icloud` in place of the file, or the file is there with
//! no data. Such a pack is listed as not downloaded and not read: reading
//! one would wait on the network.

use std::collections::BTreeMap;
use std::io::{self, Read, Write as _};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::def::{Def, MASK, REDACTED};
use super::kho::Kho;
use super::{batch, instructions, plugins, redact, Agent, Reading};
use crate::local_time;

/// The folder in iCloud Drive.
pub const FOLDER: &str = "Quota Station Agents";
/// The pack format this app writes and reads.
pub const FORMAT: u32 = 1;
/// Packs kept for each Mac.
pub const KEPT: usize = 10;
const MANIFEST: &str = "manifest.json";
const MCP: &str = "mcp.json";
const SECRETS: &str = "secrets-needed.json";
const FILES: [&str; 3] = [MANIFEST, MCP, SECRETS];
const SKILLS: &str = "skills.json";
const PLUGINS: &str = "plugins.json";
const INSTRUCTIONS: &str = "instructions.json";
/// Every file the app writes in a pack.
const ALL_FILES: [&str; 6] = [MANIFEST, MCP, SECRETS, SKILLS, PLUGINS, INSTRUCTIONS];
/// A pack's files are a few kilobytes.
const MAX_FILE: u64 = 4 * 1024 * 1024;
/// A pack folder's name starts so.
const PREFIX: &str = "agents-";

/// iCloud Drive's folder on this Mac, whether or not iCloud Drive is on.
pub fn icloud_drive(home: &Path) -> PathBuf {
    home.join("Library/Mobile Documents/com~apple~CloudDocs")
}

/// Where the packs go: `iCloud Drive › Quota Station Agents`.
pub fn default_root(home: &Path) -> PathBuf {
    icloud_drive(home).join(FOLDER)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    /// The app's version that wrote it.
    pub app: String,
    /// The name the Mac goes by in the app.
    pub mac: String,
    /// Unix seconds.
    pub at: i64,
    /// What the pack holds: `mcp`, then `skills`, `plugins` and
    /// `instructions` when it has them.
    pub sections: Vec<String>,
    pub mcp: usize,
    /// Secrets a restore asks for.
    pub keys: usize,
    /// SHA-256 of `mcp.json`: two packs with the same hold the same servers.
    pub mcp_hash: String,
}

/// One agent's entry for a server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub agent: Agent,
    pub enabled: bool,
    /// Secrets [`REDACTED`], home paths `~/`.
    pub def: Def,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    /// The secrets its definitions hold, as slot ids.
    #[serde(default)]
    pub slots: Vec<String>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpList {
    pub servers: Vec<Server>,
}

/// One secret a restore asks for: never its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Needed {
    pub mcp: String,
    pub slot: String,
    /// The variable or header name, or `URL`.
    pub label: String,
    #[serde(default)]
    pub bitwarden: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretsNeeded {
    pub secrets: Vec<Needed>,
}

/// One skill in a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackedSkill {
    pub name: String,
    /// `owner/repo` for a skill from GitHub.
    #[serde(default)]
    pub repo: Option<String>,
    /// Its `SKILL.md` in the repository.
    #[serde(default)]
    pub skill_path: Option<String>,
    /// Installed by hand: no source to install it from again.
    pub hand: bool,
    /// The agents it is on for.
    pub on: Vec<Agent>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillList {
    pub skills: Vec<PackedSkill>,
}

/// Each agent's plugins; `state` is `kept` for a list taken from an older
/// pack because the agent's could not be read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginList {
    pub agents: Vec<plugins::Side>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionList {
    pub files: Vec<instructions::Packed>,
}

/// A pack's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub manifest: Manifest,
    pub mcp: McpList,
    pub secrets: SecretsNeeded,
    pub skills: Option<SkillList>,
    pub plugins: Option<PluginList>,
    pub instructions: Option<InstructionList>,
}

impl Pack {
    /// Adds the skills, plugins and instruction files to the pack.
    pub fn extend(
        &mut self,
        skills: Option<SkillList>,
        plugins: Option<PluginList>,
        instructions: Option<InstructionList>,
    ) {
        for (name, here) in [
            ("skills", skills.is_some()),
            ("plugins", plugins.is_some()),
            ("instructions", instructions.is_some()),
        ] {
            if here && !self.manifest.sections.iter().any(|s| s == name) {
                self.manifest.sections.push(name.into());
            }
        }
        self.skills = skills;
        self.plugins = plugins;
        self.instructions = instructions;
    }

    /// The pack's files and their text.
    fn texts(&self) -> Vec<(&'static str, String)> {
        let mut out = vec![
            (MANIFEST, text(&self.manifest)),
            (MCP, text(&self.mcp)),
            (SECRETS, text(&self.secrets)),
        ];
        if let Some(s) = &self.skills {
            out.push((SKILLS, text(s)));
        }
        if let Some(p) = &self.plugins {
            out.push((PLUGINS, text(p)));
        }
        if let Some(i) = &self.instructions {
            out.push((INSTRUCTIONS, text(i)));
        }
        out
    }

    pub fn needed(&self, mcp: &str, slot: &str) -> Option<&Needed> {
        self.secrets.secrets.iter().find(|n| n.mcp == mcp && n.slot == slot)
    }
}

// ===== Paths that follow the Mac =====

/// `text` with the home folder written `~`: the whole text, or at its start,
/// or after the `=` of `--flag=…`.
fn tilde(text: &str, home: &str) -> String {
    let swap = |s: &str| -> Option<String> {
        if s == home {
            Some("~".into())
        } else {
            s.strip_prefix(home).and_then(|r| r.strip_prefix('/')).map(|r| format!("~/{r}"))
        }
    };
    if let Some(s) = swap(text) {
        return s;
    }
    if let Some((flag, value)) = text.split_once('=') {
        if let Some(s) = swap(value) {
            return format!("{flag}={s}");
        }
    }
    text.to_string()
}

/// The reverse of [`tilde`] for this Mac's home folder.
fn untilde(text: &str, home: &str) -> String {
    let swap = |s: &str| -> Option<String> {
        if s == "~" {
            Some(home.to_string())
        } else {
            s.strip_prefix("~/").map(|r| format!("{home}/{r}"))
        }
    };
    if let Some(s) = swap(text) {
        return s;
    }
    if let Some((flag, value)) = text.split_once('=') {
        if let Some(s) = swap(value) {
            return format!("{flag}={s}");
        }
    }
    text.to_string()
}

fn map_def(def: &Def, f: impl Fn(&str) -> String) -> Def {
    match def {
        Def::Stdio { command, args, env, cwd } => Def::Stdio {
            command: f(command),
            args: args.iter().map(|a| f(a)).collect(),
            env: env.iter().map(|p| super::def::Pair::new(p.key.clone(), f(&p.value))).collect(),
            cwd: cwd.as_deref().map(f),
        },
        Def::Http { .. } => def.clone(),
    }
}

/// What a pack keeps of `def`: its secrets [`REDACTED`], and the home
/// folder written `~` so the same server reads on a Mac with another user
/// name.
pub fn portable(def: &Def, home: &Path) -> Def {
    let home = home.to_string_lossy().to_string();
    map_def(&def.stripped(), |s| tilde(s, &home))
}

/// A pack's definition for this Mac: `~` is this home folder again, and a
/// command given by its full path that is not there is looked for by its
/// name where commands usually are (another Node version under `~/.nvm`,
/// Homebrew's folder). Returns the definition and whether its command was
/// found elsewhere.
pub fn localize(def: &Def, home: &Path) -> (Def, bool) {
    let h = home.to_string_lossy().to_string();
    let mut out = map_def(def, |s| untilde(s, &h));
    let mut moved = false;
    if let Def::Stdio { command, .. } = &mut out {
        let p = Path::new(command.as_str());
        if p.is_absolute() && !p.exists() {
            if let Some(name) = p.file_name() {
                let found = super::search_dirs(home).into_iter().map(|d| d.join(name)).find(|c| {
                    c.is_file()
                        // The same command, not one of the system's of the
                        // same name in place of a missing tool.
                        && c.as_path() != p
                });
                if let Some(found) = found {
                    *command = found.to_string_lossy().to_string();
                    moved = true;
                }
            }
        }
    }
    (out, moved)
}

// ===== Building a pack =====

/// A slot's name for the windows: the variable or header, the flag, `URL`.
pub fn slot_label(def: Option<&Def>, slot: &str) -> String {
    if let Some(s) = def.and_then(|d| d.slots().into_iter().find(|s| s.id == slot)) {
        return s.label;
    }
    match slot.split_once(':') {
        Some(("arg", i)) => {
            let i: usize = i.parse().unwrap_or(0);
            match def {
                Some(Def::Stdio { args, .. }) if i > 0 && args[i - 1].starts_with('-') => {
                    args[i - 1].clone()
                }
                Some(Def::Stdio { args, .. }) => {
                    match args.get(i).and_then(|a| a.split_once('=')) {
                        Some((flag, _)) if flag.starts_with('-') => flag.to_string(),
                        _ => format!("tham số {}", i + 1),
                    }
                }
                _ => format!("tham số {}", i + 1),
            }
        }
        Some((_, label)) => label.to_string(),
        None => "URL".into(),
    }
}

/// What a pack of this Mac's servers holds now: every agent that reads, the
/// app's own copies of servers turned off in Claude Code and of restored
/// ones waiting for a key. Built from the readings and the store only.
pub fn content(
    home: &Path,
    readings: &[Reading],
    store: &Kho,
    mac: &str,
    app_version: &str,
    now: OffsetDateTime,
) -> Pack {
    let mut servers: BTreeMap<String, Server> = BTreeMap::new();
    let mut labels: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut add = |name: &str, agent: Agent, enabled: bool, def: &Def, slots: Vec<String>| {
        let s = servers.entry(name.to_string()).or_insert_with(|| Server {
            name: name.to_string(),
            slots: Vec::new(),
            entries: Vec::new(),
        });
        if s.entries.iter().any(|e| e.agent == agent) {
            return;
        }
        for slot in slots {
            labels
                .entry((name.to_string(), slot.clone()))
                .or_insert_with(|| slot_label(Some(def), &slot));
            if !s.slots.contains(&slot) {
                s.slots.push(slot);
            }
        }
        s.entries.push(Entry { agent, enabled, def: portable(def, home) });
    };
    for r in readings.iter().filter(|r| r.ok()) {
        for s in &r.servers {
            let Some(def) = s.def.as_ref().filter(|_| s.owner.is_none()) else { continue };
            let slots = def.slots().into_iter().map(|x| x.id).collect();
            add(&s.name, r.agent, s.enabled, def, slots);
        }
        for (name, k) in &store.waiting {
            if k.agents.contains(&r.agent) && r.server(name).is_none() {
                add(name, r.agent, false, &k.def, k.slots.clone());
            }
        }
        if r.agent == Agent::Claude {
            for (name, k) in &store.parked {
                if r.server(name).is_none() {
                    add(name, Agent::Claude, false, &k.def, k.slots.clone());
                }
            }
        }
    }
    let mut servers: Vec<Server> = servers.into_values().collect();
    for s in &mut servers {
        s.entries.sort_by_key(|e| e.agent);
    }
    servers.sort_by_key(|s| s.name.to_lowercase());
    let secrets: Vec<Needed> = servers
        .iter()
        .flat_map(|s| {
            s.slots.iter().map(|slot| Needed {
                mcp: s.name.clone(),
                slot: slot.clone(),
                label: labels.get(&(s.name.clone(), slot.clone())).cloned().unwrap_or_default(),
                bitwarden: store.hint(&s.name, slot).map(str::to_string),
            })
        })
        .collect();
    let mcp = McpList { servers };
    let manifest = Manifest {
        format: FORMAT,
        app: app_version.to_string(),
        mac: mac.to_string(),
        at: now.unix_timestamp(),
        sections: vec!["mcp".into()],
        mcp: mcp.servers.len(),
        keys: secrets.len(),
        mcp_hash: super::write::sha256(text(&mcp).as_bytes()),
    };
    Pack {
        manifest,
        mcp,
        secrets: SecretsNeeded { secrets },
        skills: None,
        plugins: None,
        instructions: None,
    }
}

fn text<T: Serialize>(v: &T) -> String {
    serde_json::to_string_pretty(v).expect("JSON serializes") + "\n"
}

/// Every secret value in the agents' files now, to look for in a pack (and
/// in the instruction files that go into one).
pub fn secret_values(readings: &[Reading]) -> Vec<String> {
    readings
        .iter()
        .flat_map(|r| r.servers.iter())
        .filter_map(|s| s.def.as_ref())
        .flat_map(|d| {
            d.slots()
                .into_iter()
                .filter_map(|s| d.get(&s.id).map(str::to_string))
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() >= 4 && v != REDACTED && v != MASK)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum PackError {
    /// iCloud Drive is not on on this Mac.
    NoICloud,
    /// A file would hold something shaped like a secret: nothing written.
    Secret,
    Io(String),
}

impl From<io::Error> for PackError {
    fn from(e: io::Error) -> Self {
        PackError::Io(e.to_string())
    }
}

/// The Mac's name in a folder name: lowercase letters, digits and `-`.
fn slug(mac: &str) -> String {
    let mut out = String::new();
    for c in mac.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').chars().take(32).collect::<String>();
    if out.is_empty() {
        "mac".into()
    } else {
        out
    }
}

/// `agents-20261019-143005-mac-mini`: when, on this Mac's clock, and where.
fn folder_name(at: OffsetDateTime, mac: &str) -> String {
    let s = local_time::stamp(at).replace(['-', ':'], "").replace('T', "-");
    format!("{PREFIX}{s}-{}", slug(mac))
}

/// The time in a pack folder's name, for one not downloaded yet.
fn named_at(id: &str) -> Option<i64> {
    let rest = id.strip_prefix(PREFIX)?;
    let (d, t) = (rest.get(0..8)?, rest.get(9..15)?);
    let text =
        format!("{}-{}-{}T{}:{}:{}", &d[0..4], &d[4..6], &d[6..8], &t[0..2], &t[2..4], &t[4..6]);
    local_time::instant(&text).map(|t| t.unix_timestamp())
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()
}

/// Writes `pack` as a new folder in `root`, when none of its files holds a
/// secret, then keeps the newest [`KEPT`] packs of its Mac. `readings` are
/// what it was built from: none of their secrets may be in it.
pub fn write(
    root: &Path,
    home: &Path,
    pack: &Pack,
    readings: &[Reading],
) -> Result<String, PackError> {
    let texts = pack.texts();
    let known = secret_values(readings);
    for (_, t) in &texts {
        let clean = redact::json(t).is_ok_and(|c| c == *t);
        if !clean || known.iter().any(|v| t.contains(v.as_str())) {
            return Err(PackError::Secret);
        }
    }
    if root.starts_with(icloud_drive(home)) && !icloud_drive(home).is_dir() {
        return Err(PackError::NoICloud);
    }
    std::fs::create_dir_all(root)?;
    let at = OffsetDateTime::from_unix_timestamp(pack.manifest.at)
        .unwrap_or_else(|_| OffsetDateTime::now_utc());
    let base = folder_name(at, &pack.manifest.mac);
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    // Written aside, then put in place whole: a pack is never half there.
    let tmp = root.join(format!(".{id}.qs-{}", std::process::id()));
    let result = (|| -> io::Result<()> {
        std::fs::create_dir(&tmp)?;
        for (name, t) in &texts {
            write_file(&tmp.join(name), t)?;
        }
        std::fs::rename(&tmp, root.join(&id))
    })();
    if let Err(e) = result {
        for name in ALL_FILES {
            let _ = std::fs::remove_file(tmp.join(name));
        }
        let _ = std::fs::remove_dir(&tmp);
        return Err(e.into());
    }
    prune(root, &pack.manifest.mac);
    Ok(id)
}

/// Keeps the newest [`KEPT`] packs `mac` wrote. An older one loses the files
/// the app writes; its folder goes only when nothing else is in it.
fn prune(root: &Path, mac: &str) {
    let mine: Vec<Info> = list(root, mac).into_iter().filter(|p| p.mine).collect();
    for old in mine.into_iter().skip(KEPT) {
        let dir = root.join(&old.id);
        for name in ALL_FILES {
            let _ = std::fs::remove_file(dir.join(name));
        }
        let _ = std::fs::remove_dir(&dir);
    }
}

// ===== Listing and reading packs =====

/// A pack in the folder, as the windows list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// The folder's name.
    pub id: String,
    /// `ok`, `cloud` (iCloud Drive has not downloaded it) or `broken`.
    pub state: &'static str,
    /// Unix seconds; for a pack not downloaded, from its name.
    pub at: Option<i64>,
    pub mac: Option<String>,
    /// Written by this Mac.
    pub mine: bool,
    pub mcp: usize,
    pub keys: usize,
    /// A later app's format: listed, not restored.
    pub newer: bool,
    #[serde(skip)]
    pub mcp_hash: Option<String>,
}

/// The file is in iCloud Drive but not on this Mac yet.
fn in_cloud(file: &Path) -> bool {
    let placeholder = file
        .parent()
        .zip(file.file_name())
        .map(|(dir, name)| dir.join(format!(".{}.icloud", name.to_string_lossy())));
    if placeholder.is_some_and(|p| p.exists()) && !file.exists() {
        return true;
    }
    std::fs::symlink_metadata(file).is_ok_and(|m| dataless(&m))
}

/// macOS's `SF_DATALESS`: the file's data is not on this Mac.
#[cfg(target_os = "macos")]
fn dataless(m: &std::fs::Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    m.st_flags() & 0x4000_0000 != 0
}

#[cfg(not(target_os = "macos"))]
fn dataless(_: &std::fs::Metadata) -> bool {
    false
}

fn read_small(file: &Path) -> io::Result<String> {
    let meta = std::fs::symlink_metadata(file)?;
    if !meta.file_type().is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(file)?.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "larger than 4 MB"));
    }
    String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not UTF-8"))
}

/// The packs in `root`, newest first; `mac` is this Mac's name.
pub fn list(root: &Path, mac: &str) -> Vec<Info> {
    let mut out: Vec<Info> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(PREFIX))
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let dir = e.path();
            let mut info = Info {
                id: id.clone(),
                state: "broken",
                at: named_at(&id),
                mac: None,
                mine: false,
                mcp: 0,
                keys: 0,
                newer: false,
                mcp_hash: None,
            };
            if FILES.iter().any(|f| in_cloud(&dir.join(f))) {
                info.state = "cloud";
                return info;
            }
            if let Some(m) = read_small(&dir.join(MANIFEST))
                .ok()
                .and_then(|t| serde_json::from_str::<Manifest>(&t).ok())
            {
                info.state = "ok";
                info.at = Some(m.at);
                info.mine = m.mac == mac;
                info.mac = Some(m.mac);
                info.mcp = m.mcp;
                info.keys = m.keys;
                info.newer = m.format > FORMAT;
                info.mcp_hash = Some(m.mcp_hash);
            }
            info
        })
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at).then(b.id.cmp(&a.id)));
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum LoadError {
    /// Not a pack folder's name.
    BadName,
    Missing,
    /// iCloud Drive has not downloaded it yet.
    Cloud,
    /// Written by a later version of the app.
    Newer,
    Invalid(String),
}

/// A pack's name the app takes: one folder in the root, nothing else.
fn valid_id(id: &str) -> bool {
    id.starts_with(PREFIX)
        && id.len() <= 128
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// Reads the pack `id` in `root`.
pub fn load(root: &Path, id: &str) -> Result<Pack, LoadError> {
    if !valid_id(id) {
        return Err(LoadError::BadName);
    }
    let dir = root.join(id);
    if !dir.is_dir() {
        return Err(LoadError::Missing);
    }
    if FILES.iter().any(|f| in_cloud(&dir.join(f))) {
        return Err(LoadError::Cloud);
    }
    let read = |name: &str| {
        read_small(&dir.join(name)).map_err(|e| LoadError::Invalid(format!("{name}: {e}")))
    };
    let manifest: Manifest = serde_json::from_str(&read(MANIFEST)?)
        .map_err(|e| LoadError::Invalid(format!("{MANIFEST}: {e}")))?;
    if manifest.format > FORMAT {
        return Err(LoadError::Newer);
    }
    let mcp: McpList =
        serde_json::from_str(&read(MCP)?).map_err(|e| LoadError::Invalid(format!("{MCP}: {e}")))?;
    let secrets: SecretsNeeded = serde_json::from_str(&read(SECRETS)?)
        .map_err(|e| LoadError::Invalid(format!("{SECRETS}: {e}")))?;
    for s in &mcp.servers {
        if !super::valid_name(&s.name) {
            return Err(LoadError::Invalid(format!("{MCP}: {}", s.name)));
        }
    }
    let has = |section: &str| manifest.sections.iter().any(|s| s == section);
    fn section<T: for<'de> Deserialize<'de>>(dir: &Path, name: &str) -> Result<T, LoadError> {
        if in_cloud(&dir.join(name)) {
            return Err(LoadError::Cloud);
        }
        let t =
            read_small(&dir.join(name)).map_err(|e| LoadError::Invalid(format!("{name}: {e}")))?;
        serde_json::from_str(&t).map_err(|e| LoadError::Invalid(format!("{name}: {e}")))
    }
    let skills = if has("skills") { Some(section::<SkillList>(&dir, SKILLS)?) } else { None };
    let plugins = if has("plugins") { Some(section::<PluginList>(&dir, PLUGINS)?) } else { None };
    let instructions = if has("instructions") {
        Some(section::<InstructionList>(&dir, INSTRUCTIONS)?)
    } else {
        None
    };
    Ok(Pack { manifest, mcp, secrets, skills, plugins, instructions })
}

/// What "Xem trong gói" says: each server and the agents it is on in,
/// and the keys a restore asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub mac: String,
    pub at: i64,
    pub servers: Vec<SummaryServer>,
    pub keys: Vec<Needed>,
    /// The skills, plugins (`name@marketplace`) and instruction files
    /// (`~/…`) in it, when it has them.
    pub skills: Vec<String>,
    pub plugins: Vec<String>,
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryServer {
    pub name: String,
    pub agents: Vec<Agent>,
    pub on: Vec<Agent>,
}

pub fn summary(id: &str, pack: &Pack) -> Summary {
    Summary {
        id: id.to_string(),
        mac: pack.manifest.mac.clone(),
        at: pack.manifest.at,
        servers: pack
            .mcp
            .servers
            .iter()
            .map(|s| SummaryServer {
                name: s.name.clone(),
                agents: s.entries.iter().map(|e| e.agent).collect(),
                on: s.entries.iter().filter(|e| e.enabled).map(|e| e.agent).collect(),
            })
            .collect(),
        keys: pack.secrets.secrets.clone(),
        skills: pack.skills.iter().flat_map(|s| s.skills.iter().map(|k| k.name.clone())).collect(),
        plugins: pack
            .plugins
            .iter()
            .flat_map(|p| p.agents.iter().flat_map(|a| a.plugins.iter().map(|x| x.id.clone())))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        instructions: pack
            .instructions
            .iter()
            .flat_map(|i| i.files.iter().map(|f| f.path.clone()))
            .collect(),
    }
}

/// The newest pack `mac` wrote, when it holds `mcp_hash`: nothing changed.
pub fn unchanged(root: &Path, mac: &str, mcp_hash: &str) -> bool {
    list(root, mac)
        .into_iter()
        .find(|p| p.mine)
        .is_some_and(|p| p.mcp_hash.as_deref() == Some(mcp_hash))
}

/// The newest pack `mac` wrote holds what `pack` would: its servers and
/// each other section's text.
pub fn same_as_newest(root: &Path, mac: &str, pack: &Pack) -> bool {
    let Some(newest) = list(root, mac).into_iter().find(|p| p.mine) else { return false };
    if newest.mcp_hash.as_deref() != Some(pack.manifest.mcp_hash.as_str()) {
        return false;
    }
    let dir = root.join(&newest.id);
    pack.texts()
        .iter()
        .filter(|(name, _)| ![MANIFEST, MCP].contains(name))
        .all(|(name, t)| read_small(&dir.join(name)).ok().as_deref() == Some(t.as_str()))
        && [SKILLS, PLUGINS, INSTRUCTIONS]
            .iter()
            .filter(|n| !pack.texts().iter().any(|(m, _)| m == *n))
            .all(|n| !dir.join(n).exists())
}

/// The newest pack `mac` wrote, read: for the instruction files' "so với
/// gói" and the plugins an agent could not list.
pub fn newest(root: &Path, mac: &str) -> Option<(i64, Pack)> {
    let info = list(root, mac).into_iter().find(|p| p.mine && p.state == "ok")?;
    let p = load(root, &info.id).ok()?;
    Some((p.manifest.at, p))
}

/// The slots of `def` a restore fills: its [`REDACTED`] places.
pub fn holes(def: &Def) -> Vec<String> {
    batch::holes(def)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::def::Pair;

    #[test]
    fn home_paths_go_and_come_back_as_tilde() {
        let home = Path::new("/Users/someone");
        let def = Def::Stdio {
            command: "/Users/someone/.local/bin/files-mcp".into(),
            args: vec![
                "--root=/Users/someone/work".into(),
                "/Users/someone".into(),
                "/tmp/x".into(),
            ],
            env: vec![
                Pair::new("DATA_DIR", "/Users/someone/data"),
                Pair::new("FILES_TOKEN", "t-123456789"),
            ],
            cwd: Some("/Users/someone/work".into()),
        };
        let p = portable(&def, home);
        let Def::Stdio { command, args, env, cwd } = &p else { unreachable!() };
        assert_eq!(command, "~/.local/bin/files-mcp");
        assert_eq!(args, &["--root=~/work", "~", "/tmp/x"]);
        assert_eq!(env[0].value, "~/data");
        assert_eq!(env[1].value, REDACTED);
        assert_eq!(cwd.as_deref(), Some("~/work"));
        let (back, _) = localize(&p, Path::new("/Users/other"));
        let Def::Stdio { args, cwd, .. } = &back else { unreachable!() };
        assert_eq!(args[0], "--root=/Users/other/work");
        assert_eq!(cwd.as_deref(), Some("/Users/other/work"));
    }

    #[test]
    fn folder_names_are_plain() {
        assert_eq!(slug("Mac mini (M6)"), "mac-mini-m6");
        assert_eq!(slug("Máy của tôi"), "m-y-c-a-t-i");
        assert_eq!(slug("…"), "mac");
        assert!(valid_id("agents-20261019-143005-mac-mini"));
        assert!(!valid_id("agents-../x"));
        assert!(!valid_id("other"));
    }
}
