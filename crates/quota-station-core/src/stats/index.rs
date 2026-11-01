// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The index, `stats/index.json` in the app's folder: how far each log was
//! read, what each log told so far (its version, Codex's running totals),
//! the counts per quarter hour, agent, model and project, and the keys of
//! the Claude responses already counted. Nothing else: no line, no text.
//!
//! A log is known by its device and inode, and by a fingerprint of its
//! first bytes, so a log that is moved (Codex archives a session that way)
//! keeps its place, and one that is replaced under the same name is read
//! as a new log. A read takes only the complete lines after the place it
//! stopped; a line still being written waits for the next read. A log
//! shorter than where the read stopped was truncated: it is read again from
//! its start as a new log, and the Claude responses already counted are
//! known by their keys. A deleted log keeps what it added.
//!
//! Counts older than [`KEEP_DAYS`] are dropped.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{claude_log, codex_log, Agent, Line, Tokens, Usage};
use crate::app_dir::AppDir;
use crate::os;

pub const FILE: &str = "stats/index.json";
/// Bumped when the index changes shape: an index of another version is
/// read again from the logs.
pub const VERSION: u32 = 1;
/// The longest range the tab shows is 30 days; a month more is kept.
pub const KEEP_DAYS: i64 = 62;
/// The counts' grain: a quarter hour, so a switch of Claude account in the
/// middle of an hour splits it.
pub const BUCKET: i64 = 15 * 60;
/// How much of a log's start makes its fingerprint.
const HEAD: u64 = 256;
/// The deepest folder looked into under a root.
const DEPTH: usize = 6;

/// Where the logs are on this Mac.
#[derive(Debug, Clone, Default)]
pub struct Roots {
    pub claude: Vec<PathBuf>,
    pub codex: Vec<PathBuf>,
}

impl Roots {
    /// Claude Code's under the home, Codex's under its folder.
    pub fn new(home: &Path, codex_home: &Path) -> Self {
        Roots {
            claude: vec![home.join(claude_log::DIR)],
            codex: vec![codex_home.join(codex_log::SESSIONS), codex_home.join(codex_log::ARCHIVED)],
        }
    }

    fn of(&self, agent: Agent) -> &[PathBuf] {
        match agent {
            Agent::Claude => &self.claude,
            Agent::Codex => &self.codex,
            Agent::Agy => &[],
        }
    }
}

/// What a log has told so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "agent", rename_all = "camelCase")]
pub enum Reader {
    Claude(claude_log::File),
    Codex(codex_log::File),
}

impl Reader {
    fn new(agent: Agent) -> Self {
        match agent {
            Agent::Codex => Reader::Codex(codex_log::File::default()),
            _ => Reader::Claude(claude_log::File::default()),
        }
    }

    pub fn agent(&self) -> Agent {
        match self {
            Reader::Claude(_) => Agent::Claude,
            Reader::Codex(_) => Agent::Codex,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            Reader::Claude(f) => f.version.as_deref(),
            Reader::Codex(f) => f.version.as_deref(),
        }
    }

    fn line(&mut self, bytes: &[u8]) -> Line {
        match self {
            Reader::Claude(f) => f.line(bytes),
            Reader::Codex(f) => f.line(bytes),
        }
    }
}

/// One log as far as it was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    pub path: PathBuf,
    pub dev: u64,
    pub ino: u64,
    /// Bytes read: up to the end of the last complete line.
    pub offset: u64,
    /// SHA-256 of the first `head_len` bytes, in hex.
    pub head: String,
    pub head_len: u64,
    /// Lines in a format the app does not know.
    #[serde(default)]
    pub skipped: u32,
    /// When the first of them was found (Unix seconds): the newest line
    /// read before it, else the log's change time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_since: Option<i64>,
    /// The newest line with tokens (Unix seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_at: Option<i64>,
    /// No longer on disk; kept only to keep saying what it skipped.
    #[serde(default)]
    pub gone: bool,
    pub reader: Reader,
}

/// Counts of one quarter hour, agent, model and project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// Its start, Unix seconds, a multiple of [`BUCKET`].
    pub at: i64,
    pub agent: Agent,
    pub model: String,
    /// The folder; empty when the log does not say.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub project: String,
    #[serde(flatten)]
    pub tokens: Tokens,
}

type BucketKey = (i64, Agent, String, String);

/// The keys of the responses counted, by the day of the response: kept as
/// long as the counts of that day are.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen(BTreeMap<i64, HashSet<u64>>);

impl Seen {
    /// True when `key` was not seen yet; it is now.
    fn first(&mut self, key: u64, at: i64) -> bool {
        if self.0.values().any(|s| s.contains(&key)) {
            return false;
        }
        self.0.entry(at.div_euclid(86_400)).or_default().insert(key)
    }

    fn drop_before(&mut self, at: i64) {
        let day = at.div_euclid(86_400);
        self.0.retain(|d, _| *d >= day);
    }

    pub fn len(&self) -> usize {
        self.0.values().map(HashSet::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Each day's keys as one base64 string of little-endian `u64`s.
impl Serialize for Seen {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let days: BTreeMap<String, String> = self
            .0
            .iter()
            .map(|(day, keys)| {
                let mut sorted: Vec<u64> = keys.iter().copied().collect();
                sorted.sort_unstable();
                let bytes: Vec<u8> = sorted.iter().flat_map(|k| k.to_le_bytes()).collect();
                (day.to_string(), b64.encode(bytes))
            })
            .collect();
        days.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Seen {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let days = BTreeMap::<String, String>::deserialize(d)?;
        let mut out = BTreeMap::new();
        for (day, text) in days {
            let day: i64 = day.parse().map_err(serde::de::Error::custom)?;
            let bytes = b64.decode(text).map_err(serde::de::Error::custom)?;
            let keys = bytes
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().expect("eight bytes")))
                .collect();
            out.insert(day, keys);
        }
        Ok(Seen(out))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub logs: Vec<Log>,
    #[serde(default)]
    pub buckets: Vec<Bucket>,
    #[serde(default)]
    pub seen: Seen,
    /// When the last read ended (Unix seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<i64>,
    /// "Đã biết": the skipped logs counted when the owner pressed it.
    #[serde(default)]
    pub acknowledged: BTreeMap<Agent, u32>,
}

/// How a read goes, for the progress bars: logs read of the logs found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub agent: Agent,
    pub done: u32,
    pub total: u32,
}

/// The logs a format the app does not know, per agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub agent: Agent,
    /// Sessions (logs) with at least one line skipped.
    pub sessions: u32,
    pub lines: u32,
    /// The agent's versions that wrote them, when the logs say.
    pub versions: Vec<String>,
    /// The earliest of them (Unix seconds).
    pub since: Option<i64>,
    /// Sessions counted when "Đã biết" was pressed.
    pub acknowledged: u32,
}

pub fn path(app: &AppDir) -> PathBuf {
    app.path().join(FILE)
}

/// The index on disk; a missing, unreadable or older one is empty, so the
/// logs are read again from their start.
pub fn load(app: &AppDir) -> Index {
    app.read_to_string(&path(app))
        .ok()
        .and_then(|t| serde_json::from_str::<Index>(&t).ok())
        .filter(|i| i.version == VERSION)
        .unwrap_or_else(|| Index { version: VERSION, ..Index::default() })
}

pub fn save(app: &AppDir, index: &Index) -> io::Result<()> {
    let text = serde_json::to_string(index).map_err(io::Error::other)?;
    app.write_private(&path(app), &text)
}

impl Index {
    /// No log was ever read for `agent`: its read is a first read.
    pub fn first_read(&self, agent: Agent) -> bool {
        !self.logs.iter().any(|l| l.reader.agent() == agent)
    }

    /// Logs on disk now, per agent.
    pub fn logs_of(&self, agent: Agent) -> u32 {
        self.logs.iter().filter(|l| l.reader.agent() == agent && !l.gone).count() as u32
    }

    pub fn skipped(&self) -> Vec<Skipped> {
        [Agent::Claude, Agent::Codex]
            .into_iter()
            .map(|agent| {
                let logs: Vec<&Log> = self
                    .logs
                    .iter()
                    .filter(|l| l.reader.agent() == agent && l.skipped > 0)
                    .collect();
                let mut versions: Vec<String> =
                    logs.iter().filter_map(|l| l.reader.version().map(str::to_string)).collect();
                versions.sort();
                versions.dedup();
                Skipped {
                    agent,
                    sessions: logs.len() as u32,
                    lines: logs.iter().map(|l| l.skipped).sum(),
                    versions,
                    since: logs.iter().filter_map(|l| l.skipped_since).min(),
                    acknowledged: self.acknowledged.get(&agent).copied().unwrap_or(0),
                }
            })
            .collect()
    }

    /// "Đã biết": the skipped sessions counted now no longer show.
    pub fn acknowledge(&mut self) {
        for s in self.skipped() {
            self.acknowledged.insert(s.agent, s.sessions);
        }
    }

    /// Reads what was appended to every log since the last read, `now` in
    /// Unix seconds. `progress` hears after each log.
    pub fn update(&mut self, roots: &Roots, now: i64, progress: &mut dyn FnMut(Progress)) {
        let keep_from = now - KEEP_DAYS * 86_400;
        let mut buckets: BTreeMap<BucketKey, Tokens> = std::mem::take(&mut self.buckets)
            .into_iter()
            .map(|b| ((b.at, b.agent, b.model, b.project), b.tokens))
            .collect();
        let mut found: HashSet<(u64, u64)> = HashSet::new();
        for agent in [Agent::Claude, Agent::Codex] {
            let files = list(roots.of(agent));
            let total = files.len() as u32;
            progress(Progress { agent, done: 0, total });
            for (i, (file, meta)) in files.into_iter().enumerate() {
                let id = os::file_id(&file, &meta);
                if id.is_ok_and(|id| found.insert(id)) {
                    self.read_log(agent, &file, &meta, keep_from, &mut buckets);
                }
                progress(Progress { agent, done: i as u32 + 1, total });
            }
        }
        // A deleted log keeps its counts; its entry stays only while it
        // has skipped lines to tell about.
        for log in &mut self.logs {
            log.gone = !found.contains(&(log.dev, log.ino));
        }
        self.logs.retain(|l| {
            !l.gone
                || (l.skipped > 0 && l.last_at.or(l.skipped_since).is_some_and(|t| t >= keep_from))
        });
        self.buckets = buckets
            .into_iter()
            .filter(|((at, ..), _)| *at + BUCKET > keep_from)
            .map(|((at, agent, model, project), tokens)| Bucket {
                at,
                agent,
                model,
                project,
                tokens,
            })
            .collect();
        self.seen.drop_before(keep_from);
        self.read_at = Some(now);
    }

    fn read_log(
        &mut self,
        agent: Agent,
        file: &Path,
        meta: &std::fs::Metadata,
        keep_from: i64,
        buckets: &mut BTreeMap<BucketKey, Tokens>,
    ) {
        let Ok((dev, ino)) = os::file_id(file, meta) else { return };
        let len = meta.len();
        let at = self.logs.iter().position(|l| l.dev == dev && l.ino == ino);
        // Nothing appended, same place: nothing to open.
        if let Some(i) = at {
            let log = &mut self.logs[i];
            if log.offset == len && log.path == file && log.reader.agent() == agent {
                return;
            }
        }
        let Ok(mut f) = std::fs::File::open(file) else { return };
        let head_len = len.min(HEAD);
        let Ok(head) = fingerprint(&mut f, head_len) else { return };
        let same = at.and_then(|i| {
            let log = &self.logs[i];
            let still = log.reader.agent() == agent
                && log.offset <= len
                && log.head_len <= len
                && ((log.head_len == head_len && log.head == head)
                    || fingerprint(&mut f, log.head_len).is_ok_and(|h| h == log.head));
            still.then_some(i)
        });
        let i = match (same, at) {
            (Some(i), _) => i,
            (None, Some(i)) => {
                self.logs[i] = fresh(agent, file, dev, ino);
                i
            }
            (None, None) => {
                self.logs.push(fresh(agent, file, dev, ino));
                self.logs.len() - 1
            }
        };
        let log = &mut self.logs[i];
        log.path = file.to_path_buf();
        log.gone = false;
        if log.head_len < head_len {
            log.head = head;
            log.head_len = head_len;
        }
        let changed = os::changed(meta);
        if f.seek(SeekFrom::Start(log.offset)).is_err() {
            return;
        }
        let mut reader = BufReader::with_capacity(256 * 1024, f);
        let mut line: Vec<u8> = Vec::new();
        loop {
            line.clear();
            let Ok(n) = reader.read_until(b'\n', &mut line) else { break };
            // A line not finished yet is read next time.
            if n == 0 || line.last() != Some(&b'\n') {
                break;
            }
            log.offset += n as u64;
            let text = trim(&line);
            let outcome = if text.is_empty() { Line::Quiet } else { log.reader.line(text) };
            match outcome {
                Line::Usage(u) => add(&mut self.seen, buckets, agent, u, keep_from, log),
                Line::Quiet => {}
                Line::Unknown => {
                    log.skipped += 1;
                    if log.skipped_since.is_none() {
                        log.skipped_since = Some(log.last_at.unwrap_or(changed));
                    }
                }
            }
            // The line held the conversation: it does not stay in memory.
            line.fill(0);
        }
        line.fill(0);
    }
}

fn fresh(agent: Agent, file: &Path, dev: u64, ino: u64) -> Log {
    Log {
        path: file.to_path_buf(),
        dev,
        ino,
        offset: 0,
        head: String::new(),
        head_len: 0,
        skipped: 0,
        skipped_since: None,
        last_at: None,
        gone: false,
        reader: Reader::new(agent),
    }
}

fn add(
    seen: &mut Seen,
    buckets: &mut BTreeMap<BucketKey, Tokens>,
    agent: Agent,
    u: Usage,
    keep_from: i64,
    log: &mut Log,
) {
    log.last_at = Some(log.last_at.map_or(u.at, |t| t.max(u.at)));
    if u.at < keep_from {
        return;
    }
    if let Some(k) = u.key {
        if !seen.first(k, u.at) {
            return;
        }
    }
    let at = u.at.div_euclid(BUCKET) * BUCKET;
    buckets.entry((at, agent, u.model, u.project.unwrap_or_default())).or_default().add(&u.tokens);
}

fn trim(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && matches!(line[end - 1], b'\n' | b'\r' | b' ' | b'\t') {
        end -= 1;
    }
    &line[..end]
}

/// SHA-256 of the first `len` bytes, in hex.
fn fingerprint(f: &mut std::fs::File, len: u64) -> io::Result<String> {
    f.seek(SeekFrom::Start(0))?;
    let mut buf = Vec::with_capacity(len as usize);
    f.by_ref().take(len).read_to_end(&mut buf)?;
    if (buf.len() as u64) < len {
        return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
    }
    let digest = Sha256::digest(&buf);
    buf.fill(0);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Every `.jsonl` file under the roots, sorted, without following a
/// symlink; with its metadata.
fn list(roots: &[PathBuf]) -> Vec<(PathBuf, std::fs::Metadata)> {
    let mut out = Vec::new();
    for root in roots {
        walk(root, 0, &mut out);
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<(PathBuf, std::fs::Metadata)>) {
    let Ok(meta) = std::fs::symlink_metadata(dir) else { return };
    if !meta.is_dir() || depth > DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if meta.is_dir() {
            walk(&path, depth + 1, out);
        } else if meta.is_file() && path.extension().is_some_and(|e| e == "jsonl") {
            out.push((path, meta));
        }
    }
}

/// The counts whose quarter hour starts in `[from, to)` (Unix seconds).
/// Every time zone's hours start on a quarter hour, so a local hour or day
/// is whole buckets.
pub fn between(index: &Index, from: i64, to: i64) -> impl Iterator<Item = &Bucket> {
    index.buckets.iter().filter(move |b| b.at >= from && b.at < to)
}

/// Totals per agent, for tests and the summary line.
pub fn totals(index: &Index) -> HashMap<Agent, Tokens> {
    let mut out: HashMap<Agent, Tokens> = HashMap::new();
    for b in &index.buckets {
        out.entry(b.agent).or_default().add(&b.tokens);
    }
    out
}
