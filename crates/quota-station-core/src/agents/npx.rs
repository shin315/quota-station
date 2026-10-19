//! `npx skills`, the tool that installs skills from GitHub into the shared
//! folder and records them in the lock file (M11, B and G). The app runs it
//! for "Cài từ GitHub", "Đọc SKILL.md", "Cập nhật" and "Gỡ hẳn", so the
//! lock file stays right; it never writes the lock file itself.
//!
//! Every run: the arguments as a list (no shell), in an empty folder of the
//! app's own (no project around it), with the tool's telemetry and colours
//! off, and a time limit. Installing passes `-a claude-code` so the tool
//! writes only the shared folder and Claude Code's link; the app makes the
//! other agents' links itself. Nothing in a skill is run: `npx skills add`
//! copies files.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use super::skills;
use crate::runner::{self, RunError};

/// Listing a repository's skills or reading one fetches it from GitHub.
pub const LIST_TIMEOUT: Duration = Duration::from_secs(120);
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(300);
/// The most of a SKILL.md shown.
const MAX_MD: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum NpxError {
    /// No `npx` on this Mac.
    NoNpx,
    /// Not `owner/repo` or a GitHub link.
    BadRepo,
    /// The repository has no skill, or was not found.
    NoSkills,
    Timeout,
    /// The tool failed: its first line.
    Failed(String),
}

/// One skill a repository holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listed {
    pub name: String,
    pub description: String,
}

/// `owner/repo` from what was typed: `owner/repo`, or a GitHub link to it.
pub fn repo(input: &str) -> Option<String> {
    let t = input.trim().trim_end_matches('/');
    let t = t.strip_suffix(".git").unwrap_or(t);
    let t = ["https://github.com/", "http://github.com/", "github.com/"]
        .iter()
        .find_map(|p| t.strip_prefix(p))
        .unwrap_or(t);
    let mut parts = t.splitn(3, '/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let r = format!("{owner}/{name}");
    skills::repo_ok(&r).then_some(r)
}

/// Escape sequences and the box drawing the tool prints around its lines.
pub fn plain_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c == '\r' {
            continue;
        }
        out.push(c);
    }
    out
}

/// The skills `npx skills add <repo> --list` printed: after "Available
/// Skills", a name indented once, its description indented more.
pub fn parse_list(text: &str) -> Vec<Listed> {
    let mut out: Vec<Listed> = Vec::new();
    let mut started = false;
    for raw in text.lines() {
        if raw.contains("Available Skills") {
            started = true;
            continue;
        }
        if !started {
            continue;
        }
        if raw.contains("--skill") {
            break;
        }
        let line = raw.trim_start_matches(['│', '|', '◇', '└']);
        let indent = line.len() - line.trim_start().len();
        let words = line.trim();
        if words.is_empty() {
            continue;
        }
        if indent <= 4 && !words.contains(' ') {
            out.push(Listed { name: words.to_string(), description: String::new() });
        } else if let Some(last) = out.last_mut() {
            if !last.description.is_empty() {
                last.description.push(' ');
            }
            last.description.push_str(words);
        }
    }
    out
}

/// The SKILL.md `npx skills use <repo>@<skill>` printed between its tags.
pub fn parse_md(text: &str) -> Option<String> {
    let start = text.find("<SKILL.md>")? + "<SKILL.md>".len();
    let end = text[start..].find("</SKILL.md>")? + start;
    let mut md = text[start..end].trim_matches('\n').to_string();
    if md.len() > MAX_MD {
        let mut cut = MAX_MD;
        while !md.is_char_boundary(cut) {
            cut -= 1;
        }
        md.truncate(cut);
        md.push_str("\n…");
    }
    Some(md)
}

/// The `npx` the app runs, and the empty folder it runs in.
#[derive(Debug, Clone)]
pub struct Npx {
    pub program: PathBuf,
    pub cwd: PathBuf,
}

impl Npx {
    pub fn new(program: PathBuf, cwd: PathBuf) -> Self {
        Npx { program, cwd }
    }

    fn run(&self, args: &[String], timeout: Duration) -> Result<String, NpxError> {
        let _ = std::fs::create_dir_all(&self.cwd);
        let mut all = vec!["-y".to_string(), "skills".to_string()];
        all.extend(args.iter().cloned());
        let env: [(&str, &std::ffi::OsStr); 4] = [
            ("DO_NOT_TRACK", "1".as_ref()),
            ("DISABLE_TELEMETRY", "1".as_ref()),
            ("NO_COLOR", "1".as_ref()),
            ("FORCE_COLOR", "0".as_ref()),
        ];
        let out = runner::run_in(&self.program, &all, &env, None, Some(&self.cwd), timeout)
            .map_err(|e| match e {
                RunError::Timeout(_) => NpxError::Timeout,
                RunError::NotFound(_) => NpxError::NoNpx,
                other => NpxError::Failed(other.to_string()),
            })?;
        let text = plain_text(&out.stdout);
        if out.code != Some(0) {
            let err = plain_text(&out.stderr);
            let first = err
                .lines()
                .chain(text.lines())
                .map(|l| l.trim_matches(|c: char| c.is_whitespace() || "│◇■●".contains(c)))
                .find(|l| !l.is_empty())
                .unwrap_or("")
                .chars()
                .take(300)
                .collect();
            return Err(NpxError::Failed(first));
        }
        Ok(text)
    }

    /// The skills in `repo`, without installing any.
    pub fn list(&self, repo: &str) -> Result<Vec<Listed>, NpxError> {
        if !skills::repo_ok(repo) {
            return Err(NpxError::BadRepo);
        }
        let text = self.run(&["add".into(), repo.into(), "--list".into()], LIST_TIMEOUT)?;
        let found = parse_list(&text);
        if found.is_empty() {
            return Err(NpxError::NoSkills);
        }
        Ok(found)
    }

    /// One skill's SKILL.md, to read before installing.
    pub fn read(&self, repo: &str, skill: &str) -> Result<String, NpxError> {
        if !skills::repo_ok(repo) {
            return Err(NpxError::BadRepo);
        }
        let text = self.run(&["use".into(), format!("{repo}@{skill}")], LIST_TIMEOUT)?;
        parse_md(&text).ok_or(NpxError::NoSkills)
    }

    /// Installs `names` from `repo` for the user: the shared folder, the
    /// lock file and Claude Code's link.
    pub fn add(&self, repo: &str, names: &[String]) -> Result<(), NpxError> {
        if !skills::repo_ok(repo) {
            return Err(NpxError::BadRepo);
        }
        let mut args = vec!["add".to_string(), repo.to_string()];
        for n in names {
            args.push("--skill".into());
            args.push(n.clone());
        }
        args.extend(["-g", "-y", "-a", "claude-code"].map(str::to_string));
        self.run(&args, INSTALL_TIMEOUT).map(|_| ())
    }

    /// `npx skills update <name> -g -y`.
    pub fn update(&self, name: &str) -> Result<(), NpxError> {
        self.run(&["update".into(), name.into(), "-g".into(), "-y".into()], INSTALL_TIMEOUT)
            .map(|_| ())
    }

    /// `npx skills remove <names> -g -y`: the folder, the lock entry and the
    /// links the tool made.
    pub fn remove(&self, names: &[String]) -> Result<(), NpxError> {
        let mut args = vec!["remove".to_string()];
        args.extend(names.iter().cloned());
        args.extend(["-g", "-y"].map(str::to_string));
        self.run(&args, INSTALL_TIMEOUT).map(|_| ())
    }
}

/// The folder npx runs in: empty, in the app's folder.
pub fn run_dir(app: &Path) -> PathBuf {
    app.join("agents/npx")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repos_from_what_was_typed() {
        assert_eq!(repo("acme/agent-skills").as_deref(), Some("acme/agent-skills"));
        assert_eq!(
            repo("https://github.com/acme/agent-skills.git").as_deref(),
            Some("acme/agent-skills")
        );
        assert_eq!(
            repo("https://github.com/acme/agent-skills/tree/main/x").as_deref(),
            Some("acme/agent-skills")
        );
        assert_eq!(repo("acme"), None);
        assert_eq!(repo("acme/a b"), None);
    }

    #[test]
    fn the_list_as_the_tool_prints_it() {
        let out = "\u{1b}[?25l│\n◇  Found 2 skills\n│\n◇  Available Skills\n│\n│    changelog-writer\n│\n│      Writes a changelog\n│      from the history.\n│\n│    pdf-tools\n│\n│      Reads PDF files.\n\n│\n└  Use --skill <name> to install specific skills\n";
        let l = parse_list(&plain_text(out.as_bytes()));
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].name, "changelog-writer");
        assert_eq!(l[0].description, "Writes a changelog from the history.");
        assert_eq!(l[1].name, "pdf-tools");
    }

    #[test]
    fn the_skill_md_between_its_tags() {
        let t = "Use the following SKILL.md:\n\n<SKILL.md>\n---\nname: x\n---\n# X\n</SKILL.md>\n";
        assert_eq!(parse_md(t).as_deref(), Some("---\nname: x\n---\n# X"));
    }
}
