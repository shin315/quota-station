//! The agents' shared instruction files (M11, F): `~/.claude/CLAUDE.md`,
//! `~/.claude/rules/*.md`, Codex's `AGENTS.md`, `~/.gemini/GEMINI.md`, and
//! any other file the owner adds. Shown with where each is, when it was
//! changed, how it compares with the newest backup pack, and whether it
//! goes into the pack.
//!
//! The app never writes these files. A file with a line shaped like a key
//! ([`key_lines`]) never goes into a pack, and its value is masked wherever
//! the windows show it. "Không phải khoá" remembers that one line (by a
//! hash of the line, in the app's folder) is not a key.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::def::{self, MASK};
use super::{skills, write, Agent};
use crate::antigravity::fs::AppDir;
use crate::runner;

/// The owner's choices, in the app's folder: the files added, the ones
/// kept out of the pack, the lines that are not keys.
pub const FILE: &str = "agents/instructions.json";
/// The largest file read.
pub const MAX_FILE: u64 = 1024 * 1024;
/// How deep `~/.claude/rules` is looked into.
const RULES_DEPTH: usize = 3;
const MAX_FILES: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Choices {
    /// Files added with "Thêm file…", as `~/…`.
    pub extra: Vec<String>,
    /// Files kept out of the pack, as `~/…`.
    pub excluded: Vec<String>,
    /// SHA-256 of `<file>\n<line>` for each line marked "Không phải khoá".
    pub not_keys: Vec<String>,
}

pub fn load(app: &AppDir) -> Choices {
    skills::load_json(app, FILE)
}

pub fn save(app: &AppDir, c: &Choices) -> std::io::Result<()> {
    skills::save_json(app, FILE, c)
}

/// The hash a "Không phải khoá" keeps for a line of a file.
pub fn line_id(shown: &str, line: &str) -> String {
    write::sha256(format!("{shown}\n{line}").as_bytes())
}

// ===== Lines shaped like a key =====

/// A word in prose that only looks like a value: a placeholder.
fn placeholder(v: &str) -> bool {
    let l = v.to_ascii_lowercase();
    v.len() < 8
        || v.contains(['<', '>', '{', '}', '…'])
        || v.contains("...")
        || l.contains("xxxx")
        || l.starts_with("your")
        || l.contains("example")
        || l.contains("placeholder")
        || l.contains("changeme")
        || def::only_references(v)
        || v.chars().all(|c| c == v.chars().next().unwrap_or(' '))
}

/// Where in `line` a key's value is, if the line holds one: a value after
/// a secret's name (`DEPLOY_TOKEN=…`, `api_key: …`), a word shaped like a
/// token (`sk-…`, `ghp_…`), a password in an address, a private key, or
/// one of `known` (the values in the agents' MCP entries).
pub fn key_span(line: &str, known: &[String]) -> Option<(usize, usize)> {
    if line.contains("PRIVATE KEY-----") {
        let start = line.find("-----").unwrap_or(0);
        return Some((start, line.len()));
    }
    for k in known.iter().filter(|k| k.len() >= 6) {
        if let Some(at) = line.find(k.as_str()) {
            return Some((at, at + k.len()));
        }
    }
    // Words: a run of characters a token is made of.
    let is_tok = |c: char| c.is_ascii_alphanumeric() || "-_./+=:@%~".contains(c);
    let bytes: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0;
    while i < bytes.len() {
        if !is_tok(bytes[i].1) {
            i += 1;
            continue;
        }
        let start = bytes[i].0;
        while i < bytes.len() && is_tok(bytes[i].1) {
            i += 1;
        }
        let end = if i < bytes.len() { bytes[i].0 } else { line.len() };
        let word = &line[start..end];
        if def::token_shaped(word) {
            return Some((start, end));
        }
        if word.contains("://") && def::secret_url(word) {
            return Some((start, end));
        }
        // NAME=value inside the word.
        if let Some((name, value)) = word.split_once('=') {
            let value = value.trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty() && def::secret_name(name) && !placeholder(value) {
                let at = start + word.find('=').unwrap_or(0) + 1;
                return Some((at, end));
            }
        }
    }
    // `name: value`, `name = "value"`: the name, then the value after it.
    let t = line.trim_start().trim_start_matches(['-', '*', ' ']).trim_start_matches("export ");
    let (name, rest) = t.split_once([':', '='])?;
    let name = name.trim().trim_matches(['`', '"', '\'', '*']);
    if name.is_empty() || name.contains(' ') || !def::secret_name(name) {
        return None;
    }
    let value = rest.trim().trim_matches(['`', '"', '\'', ',', ';']);
    if value.contains(' ') || placeholder(value) {
        return None;
    }
    let at = line.find(value)?;
    Some((at, at + value.len()))
}

/// The line with the key's value masked.
pub fn masked(line: &str, span: (usize, usize)) -> String {
    format!("{}{MASK}{}", &line[..span.0], &line[span.1..])
}

/// The numbers (from 1) of the lines shaped like a key, those marked not
/// keys left out.
pub fn key_lines(shown: &str, text: &str, known: &[String], not_keys: &[String]) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| key_span(l, known).is_some() && !not_keys.contains(&line_id(shown, l)))
        .map(|(i, _)| i + 1)
        .collect()
}

// ===== The files =====

/// Each file the tab lists, with the agent that reads it.
pub fn files(home: &Path, codex_home: &Path, choices: &Choices) -> Vec<(PathBuf, Option<Agent>)> {
    let mut out: Vec<(PathBuf, Option<Agent>)> =
        vec![(home.join(".claude/CLAUDE.md"), Some(Agent::Claude))];
    let mut rules = Vec::new();
    let mut stack = vec![(home.join(".claude/rules"), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if m.is_dir() && depth + 1 < RULES_DEPTH {
                stack.push((p, depth + 1));
            } else if m.is_file() && p.extension().is_some_and(|x| x == "md") {
                rules.push(p);
            }
        }
    }
    rules.sort();
    out.extend(rules.into_iter().map(|p| (p, Some(Agent::Claude))));
    out.push((codex_home.join("AGENTS.md"), Some(Agent::Codex)));
    out.push((home.join(".gemini/GEMINI.md"), Some(Agent::Agy)));
    for e in &choices.extra {
        let p = runner::expand_path(e, home);
        if !out.iter().any(|(q, _)| *q == p) {
            out.push((p, None));
        }
    }
    out.retain(|(p, _)| {
        std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file() || m.file_type().is_symlink())
    });
    out.truncate(MAX_FILES);
    out
}

/// A file added with "Thêm file…": a text file of 1 MB or less in the home
/// folder. Returns it as `~/…`.
pub fn check_extra(home: &Path, app: &Path, input: &str) -> Result<String, &'static str> {
    let p = runner::expand_path(input.trim(), home);
    if !p.is_absolute() || !p.starts_with(home) || p.starts_with(app) || input.contains("..") {
        return Err("outside");
    }
    let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "md" | "mdc" | "txt") {
        return Err("type");
    }
    let meta = std::fs::metadata(&p).map_err(|_| "missing")?;
    if !meta.is_file() {
        return Err("missing");
    }
    if meta.len() > MAX_FILE {
        return Err("large");
    }
    Ok(runner::display_path(&p, home))
}

pub(crate) fn read_text(p: &Path) -> Result<String, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(p)
        .and_then(|f| f.take(MAX_FILE + 1).read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("larger than 1 MB".into());
    }
    String::from_utf8(bytes).map_err(|_| "not UTF-8".to_string())
}

/// One file in the pack: its text, which holds no key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Packed {
    /// `~/…`.
    pub path: String,
    #[serde(default)]
    pub agent: Option<Agent>,
    pub sha256: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    /// `~/…`.
    pub path: String,
    pub agent: Option<Agent>,
    /// RFC 3339.
    pub modified_at: Option<String>,
    pub size: u64,
    pub lines: usize,
    /// Lines shaped like a key: the file stays out of the pack.
    pub key_lines: Vec<usize>,
    /// Not readable as text: the reason.
    pub error: Option<String>,
    /// The owner keeps it in the pack (unless it has a key).
    pub include: bool,
    /// Added with "Thêm file…".
    pub extra: bool,
    /// Against the newest pack: `same`, `changed`, `new` (not in it) or
    /// `noPack`.
    pub vs_pack: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub files: Vec<FileView>,
    /// When the newest pack was made, Unix seconds.
    pub pack_at: Option<i64>,
}

/// One file read: its text when readable, and its key lines.
pub struct Read1 {
    pub path: PathBuf,
    pub shown: String,
    pub agent: Option<Agent>,
    pub text: Result<String, String>,
    pub keys: Vec<usize>,
}

pub fn read_all(home: &Path, codex_home: &Path, choices: &Choices, known: &[String]) -> Vec<Read1> {
    files(home, codex_home, choices)
        .into_iter()
        .map(|(path, agent)| {
            let shown = runner::display_path(&path, home);
            let text = read_text(&path);
            let keys = text
                .as_ref()
                .map(|t| key_lines(&shown, t, known, &choices.not_keys))
                .unwrap_or_default();
            Read1 { path, shown, agent, text, keys }
        })
        .collect()
}

/// What goes into a pack: each file the owner keeps in, readable, with no
/// key.
pub fn packed(read: &[Read1], choices: &Choices) -> Vec<Packed> {
    read.iter()
        .filter(|r| r.keys.is_empty() && !choices.excluded.contains(&r.shown))
        .filter_map(|r| {
            let text = r.text.as_ref().ok()?;
            Some(Packed {
                path: r.shown.clone(),
                agent: r.agent,
                sha256: write::sha256(text.as_bytes()),
                text: text.clone(),
            })
        })
        .collect()
}

pub fn view(read: &[Read1], choices: &Choices, pack: Option<(i64, &[Packed])>) -> View {
    let files = read
        .iter()
        .map(|r| {
            let meta = std::fs::metadata(&r.path).ok();
            let modified_at = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| OffsetDateTime::from(t).format(&Rfc3339).ok());
            let vs_pack = match (pack, &r.text) {
                (None, _) => "noPack",
                (Some((_, files)), Ok(text)) => match files.iter().find(|p| p.path == r.shown) {
                    None => "new",
                    Some(p) if p.sha256 == write::sha256(text.as_bytes()) => "same",
                    Some(_) => "changed",
                },
                (Some(_), Err(_)) => "new",
            };
            FileView {
                path: r.shown.clone(),
                agent: r.agent,
                modified_at,
                size: meta.map(|m| m.len()).unwrap_or(0),
                lines: r.text.as_ref().map(|t| t.lines().count()).unwrap_or(0),
                key_lines: r.keys.clone(),
                error: r.text.as_ref().err().cloned(),
                include: !choices.excluded.contains(&r.shown),
                extra: choices.extra.contains(&r.shown),
                vs_pack,
            }
        })
        .collect();
    View { files, pack_at: pack.map(|(at, _)| at) }
}

/// "Xem": the file's lines, each key's value masked.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    pub path: String,
    pub lines: Vec<ShownLine>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownLine {
    pub text: String,
    /// Shaped like a key, masked.
    pub key: bool,
    /// Shaped like a key and marked "Không phải khoá".
    pub not_key: bool,
}

pub fn show(r: &Read1, known: &[String], not_keys: &[String]) -> Result<Shown, String> {
    let text = r.text.clone()?;
    let lines = text
        .lines()
        .map(|l| match key_span(l, known) {
            Some(span) if not_keys.contains(&line_id(&r.shown, l)) => {
                // Marked not a key: shown, still masked here.
                ShownLine { text: masked(l, span), key: false, not_key: true }
            }
            Some(span) => ShownLine { text: masked(l, span), key: true, not_key: false },
            None => ShownLine { text: l.to_string(), key: false, not_key: false },
        })
        .collect();
    Ok(Shown { path: r.shown.clone(), lines })
}

/// "So sánh": the file against its copy in the newest pack, line by line.
pub fn compare(
    r: &Read1,
    packed: &Packed,
    known: &[String],
) -> Result<Vec<super::batch::Line>, String> {
    let now = r.text.clone()?;
    let mask = |t: &str| -> String {
        t.lines()
            .map(|l| key_span(l, known).map(|s| masked(l, s)).unwrap_or_else(|| l.to_string()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(super::batch::diff(&mask(&packed.text), &mask(&now)))
}

/// "Không phải khoá": the line `n` of the file is not a key.
pub fn mark_not_key(choices: &mut Choices, r: &Read1, n: usize) -> bool {
    let Ok(text) = &r.text else { return false };
    let Some(line) = text.lines().nth(n.saturating_sub(1)) else { return false };
    let id = line_id(&r.shown, line);
    if !choices.not_keys.contains(&id) {
        choices.not_keys.push(id);
    }
    true
}

/// The newest pack's files, by path.
pub fn by_path(files: &[Packed]) -> BTreeMap<&str, &Packed> {
    files.iter().map(|p| (p.path.as_str(), p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_shaped_like_a_key() {
        let k = |l: &str| key_span(l, &[]).map(|s| masked(l, s));
        assert_eq!(k("DEPLOY_TOKEN=abcd1234efgh5678").as_deref(), Some("DEPLOY_TOKEN=••••••"));
        assert_eq!(
            k("export OPENAI_API_KEY=\"sk-test-not-real-123456\"").as_deref(),
            Some("export OPENAI_API_KEY=\"••••••\"")
        );
        assert_eq!(k("- api_key: q8w7e6r5t4y3u2i1").as_deref(), Some("- api_key: ••••••"));
        assert_eq!(
            k("Use ghp_testtokennotreal1234 to push").as_deref(),
            Some("Use •••••• to push")
        );
        assert!(k("-----BEGIN OPENSSH PRIVATE KEY-----").is_some());
        assert_eq!(k("Set GITHUB_TOKEN=<your token> first"), None);
        assert_eq!(k("API_KEY=${API_KEY}"), None);
        assert_eq!(k("The token is read from the Keychain."), None);
        assert_eq!(k("password: see Bitwarden"), None);
        assert_eq!(k("Keys go in KEY_FILE=~/keys/app.pem"), None);
        assert_eq!(
            key_span("value trap-value-123 here", &["trap-value-123".into()]).map(|s| s.0),
            Some(6)
        );
    }
}
