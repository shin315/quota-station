// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Newer versions of the skills installed from GitHub (M11, G).
//!
//! Once a day (or on "Kiểm tra bản mới") the app asks GitHub one thing per
//! repository the lock file records: the repository's tree at its default
//! branch (`GET /repos/<owner>/<repo>/git/trees/HEAD?recursive=1`). Nothing
//! else is sent, no token either. A skill whose folder's tree id there is
//! not the one `npx skills` recorded has a newer version. What changed is
//! told by comparing the files' blob ids there with the files on this Mac,
//! so no other call is needed.
//!
//! The app never updates on its own: the owner looks at the preview and
//! presses "Cập nhật", or "Bỏ qua bản này". A skill whose folder is no
//! longer what was installed ([`tree_hash`] against the lock file) was
//! edited on this Mac; the preview says so, and the app keeps a copy first.

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::skills::{self, Locked};
use crate::app_dir::AppDir;

/// In the app's folder.
pub const FILE: &str = "agents/skill-updates.json";
/// How long one question to GitHub may take.
const TIMEOUT: Duration = Duration::from_secs(20);
/// The most of one folder hashed: a skill is a few files.
const MAX_ENTRIES: usize = 5000;
const MAX_BYTES: u64 = 100 * 1024 * 1024;
/// A repository's tree as GitHub sends it is at most a few megabytes.
const MAX_BODY: u64 = 16 * 1024 * 1024;

// ===== Git's object ids =====

fn object(kind: &str, body: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.update(format!("{kind} {}\0", body.len()).as_bytes());
    h.update(body);
    h.finalize().into()
}

/// The tree of an empty folder.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

fn hex(id: &[u8]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// A file's blob id, as `git hash-object` gives it.
pub fn blob_hash(bytes: &[u8]) -> String {
    hex(&object("blob", bytes))
}

/// Files Finder leaves behind, which no repository holds.
fn ignored(name: &str) -> bool {
    name == ".DS_Store" || name == ".git"
}

struct Budget {
    entries: usize,
    bytes: u64,
}

fn tree(
    dir: &Path,
    budget: &mut Budget,
    files: &mut Vec<(String, String)>,
    prefix: &str,
) -> io::Result<[u8; 20]> {
    let mut items: Vec<(String, &'static str, [u8; 20])> = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_string();
        if ignored(&name) {
            continue;
        }
        budget.entries += 1;
        if budget.entries > MAX_ENTRIES {
            return Err(io::Error::other("too many files"));
        }
        let path = e.path();
        let meta = std::fs::symlink_metadata(&path)?;
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&path)?;
            let id = object("blob", target.as_os_str().as_encoded_bytes());
            files.push((rel, hex(&id)));
            items.push((name, "120000", id));
        } else if meta.is_dir() {
            let id = tree(&path, budget, files, &rel)?;
            // Git keeps no empty folder.
            if hex(&id) != EMPTY_TREE {
                items.push((name, "40000", id));
            }
        } else if meta.is_file() {
            budget.bytes += meta.len();
            if budget.bytes > MAX_BYTES {
                return Err(io::Error::other("larger than 100 MB"));
            }
            let mut bytes = Vec::new();
            std::fs::File::open(&path)?.read_to_end(&mut bytes)?;
            let id = object("blob", &bytes);
            files.push((rel, hex(&id)));
            // Git's mode: on Windows, which keeps no execute bit, a script
            // is told by its `#!` (`os::runs`).
            let mode = if crate::os::runs(&meta, &bytes) { "100755" } else { "100644" };
            items.push((name, mode, id));
        }
    }
    // Git sorts a folder's entries by name, a folder's as if it ended in `/`.
    items.sort_by(|a, b| {
        let key = |(n, m, _): &(String, &str, [u8; 20])| {
            let mut k = n.as_bytes().to_vec();
            if *m == "40000" {
                k.push(b'/');
            }
            k
        };
        key(a).cmp(&key(b))
    });
    let mut body = Vec::new();
    for (name, mode, id) in &items {
        body.extend_from_slice(mode.as_bytes());
        body.push(b' ');
        body.extend_from_slice(name.as_bytes());
        body.push(0);
        body.extend_from_slice(id);
    }
    Ok(object("tree", &body))
}

/// A folder's Git tree id: what `npx skills` records as `skillFolderHash`.
pub fn tree_hash(dir: &Path) -> io::Result<String> {
    let mut files = Vec::new();
    tree(dir, &mut Budget { entries: 0, bytes: 0 }, &mut files, "").map(|id| hex(&id))
}

/// Each file in a folder, by its path in it, and its blob id.
pub fn file_hashes(dir: &Path) -> io::Result<BTreeMap<String, String>> {
    let mut files = Vec::new();
    tree(dir, &mut Budget { entries: 0, bytes: 0 }, &mut files, "")?;
    Ok(files.into_iter().collect())
}

// ===== What the checks found =====

/// What changed between the skill on this Mac and the newer version.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Changes {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

/// A newer version of one skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub repo: String,
    /// The tree id the lock file had when this was found.
    pub from: String,
    /// The tree id on GitHub.
    pub to: String,
    /// Unix seconds: when the app first saw this version.
    pub seen_at: i64,
    pub changes: Changes,
}

/// `agents/skill-updates.json`: the last check, what it found, and the
/// versions the owner skipped. No secret, nothing of the skills' text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Updates {
    /// Unix seconds.
    pub checked_at: Option<i64>,
    /// Why the last check did not finish: `offline`, `rateLimited`, or
    /// GitHub's status.
    pub error: Option<String>,
    /// Repositories GitHub did not find, or did not answer for.
    pub failed: Vec<String>,
    pub found: BTreeMap<String, Found>,
    /// "Bỏ qua bản này": the skill's skipped tree id.
    pub skipped: BTreeMap<String, String>,
}

impl Updates {
    /// The newer version of `name` to show: found, not skipped, and still
    /// newer than what is installed.
    pub fn pending(&self, name: &str, locked: &Locked) -> Option<&Found> {
        let f = self.found.get(name)?;
        let installed = locked.skill_folder_hash.as_deref()?;
        (f.to != installed && self.skipped.get(name) != Some(&f.to)).then_some(f)
    }

    /// The names with a newer version to show, given the lock file now.
    pub fn names(&self, lock: &BTreeMap<String, Locked>) -> Vec<String> {
        lock.iter().filter(|(n, l)| self.pending(n, l).is_some()).map(|(n, _)| n.clone()).collect()
    }
}

pub fn load(app: &AppDir) -> Updates {
    skills::load_json(app, FILE)
}

pub fn save(app: &AppDir, u: &Updates) -> io::Result<()> {
    skills::save_json(app, FILE, u)
}

/// What changed, in plain words: "Sửa hướng dẫn, thêm 1 file".
pub fn plain(c: &Changes) -> String {
    let mut parts = Vec::new();
    let guide = c.modified.iter().any(|p| p == skills::SKILL_MD);
    let others = c.modified.len() - usize::from(guide);
    if guide {
        parts.push("Sửa hướng dẫn".to_string());
    }
    if others > 0 {
        parts.push(format!("sửa {others} file"));
    }
    if !c.added.is_empty() {
        parts.push(format!("thêm {} file", c.added.len()));
    }
    if !c.removed.is_empty() {
        parts.push(format!("bỏ {} file", c.removed.len()));
    }
    if parts.is_empty() {
        return "Đổi cách sắp xếp file".into();
    }
    let mut s = parts.join(", ");
    if let Some(first) = s.get(0..1) {
        let up = first.to_uppercase();
        s.replace_range(0..1, &up);
    }
    s
}

/// The skill's history on GitHub, for "Xem thay đổi trên GitHub".
pub fn link(repo: &str, folder: Option<&str>) -> String {
    match folder.filter(|f| !f.is_empty()) {
        Some(f) => format!("https://github.com/{repo}/commits/HEAD/{f}"),
        None => format!("https://github.com/{repo}/commits/HEAD"),
    }
}

// ===== Asking GitHub =====

/// Where the check asks. The app always uses [`GitHub::api`].
#[derive(Debug, Clone)]
pub struct GitHub {
    base: String,
}

impl GitHub {
    pub fn api() -> Self {
        // The tests run a local fake; one that reached for GitHub stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake GitHub");
        }
        GitHub { base: "https://api.github.com".into() }
    }

    #[cfg(feature = "test-support")]
    pub fn fake(base: &str) -> Self {
        GitHub { base: base.trim_end_matches('/').into() }
    }
}

#[derive(Deserialize)]
struct TreeEntry {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
}

#[derive(Deserialize)]
struct TreeBody {
    sha: String,
    #[serde(default)]
    tree: Vec<TreeEntry>,
}

enum Asked {
    Tree(TreeBody),
    /// No such repository, or no access to it.
    Missing,
    RateLimited,
    Offline,
    Status(u16),
}

fn ask(client: &reqwest::blocking::Client, gh: &GitHub, repo: &str) -> Asked {
    let url = format!("{}/repos/{repo}/git/trees/HEAD?recursive=1", gh.base);
    let resp = match client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Quota-Station")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
    {
        Ok(r) => r,
        Err(_) => return Asked::Offline,
    };
    let status = resp.status().as_u16();
    if status == 404 {
        return Asked::Missing;
    }
    if status == 403 || status == 429 {
        return Asked::RateLimited;
    }
    if status != 200 {
        return Asked::Status(status);
    }
    let mut body = Vec::new();
    if resp.take(MAX_BODY).read_to_end(&mut body).is_err() {
        return Asked::Offline;
    }
    match serde_json::from_slice::<TreeBody>(&body) {
        Ok(t) => Asked::Tree(t),
        Err(_) => Asked::Status(status),
    }
}

/// Asks GitHub once for each repository in `lock` and returns what is
/// newer, keeping `before`'s skipped versions and first-seen times.
pub fn check(
    gh: &GitHub,
    lock: &BTreeMap<String, Locked>,
    shared: &Path,
    before: &Updates,
    now: i64,
) -> Updates {
    let mut by_repo: BTreeMap<String, Vec<(&String, &Locked)>> = BTreeMap::new();
    for (name, l) in lock {
        if l.github()
            && l.skill_folder_hash.is_some()
            && l.folder().is_some()
            && skills::valid_name(name)
        {
            by_repo.entry(l.source.clone()).or_default().push((name, l));
        }
    }
    let mut out = Updates {
        checked_at: Some(now),
        error: None,
        failed: Vec::new(),
        found: BTreeMap::new(),
        skipped: before.skipped.clone(),
    };
    if by_repo.is_empty() {
        return out;
    }
    let client = match reqwest::blocking::Client::builder().timeout(TIMEOUT).build() {
        Ok(c) => c,
        Err(_) => {
            out.error = Some("offline".into());
            return out;
        }
    };
    let mut answered = 0;
    for (repo, skills) in &by_repo {
        let tree = match ask(&client, gh, repo) {
            Asked::Tree(t) => t,
            Asked::RateLimited => {
                out.error = Some("rateLimited".into());
                out.failed.push(repo.clone());
                continue;
            }
            Asked::Offline => {
                out.error = Some("offline".into());
                out.failed.push(repo.clone());
                continue;
            }
            Asked::Missing => {
                out.failed.push(repo.clone());
                continue;
            }
            Asked::Status(s) => {
                out.error = Some(format!("status {s}"));
                out.failed.push(repo.clone());
                continue;
            }
        };
        answered += 1;
        for (name, l) in skills {
            let folder = l.folder().unwrap_or_default();
            let id = if folder.is_empty() {
                Some(tree.sha.clone())
            } else {
                tree.tree
                    .iter()
                    .find(|e| e.kind == "tree" && e.path == folder)
                    .map(|e| e.sha.clone())
            };
            let Some(id) = id else { continue };
            let installed = l.skill_folder_hash.clone().unwrap_or_default();
            if id == installed {
                continue;
            }
            let prefix = if folder.is_empty() { String::new() } else { format!("{folder}/") };
            let remote: BTreeMap<String, String> = tree
                .tree
                .iter()
                .filter(|e| e.kind == "blob")
                .filter_map(|e| {
                    e.path.strip_prefix(&prefix).map(|p| (p.to_string(), e.sha.clone()))
                })
                .filter(|(p, _)| !p.split('/').any(ignored))
                .collect();
            let local = file_hashes(&shared.join(name.as_str())).unwrap_or_default();
            let mut changes = Changes::default();
            for (p, sha) in &remote {
                match local.get(p) {
                    None => changes.added.push(p.clone()),
                    Some(s) if s != sha => changes.modified.push(p.clone()),
                    _ => {}
                }
            }
            changes.removed = local.keys().filter(|p| !remote.contains_key(*p)).cloned().collect();
            let seen_at = before
                .found
                .get(name.as_str())
                .filter(|f| f.to == id)
                .map(|f| f.seen_at)
                .unwrap_or(now);
            out.found.insert(
                name.to_string(),
                Found { repo: repo.clone(), from: installed, to: id, seen_at, changes },
            );
        }
    }
    if answered > 0 && out.error.is_some() && out.failed.len() < by_repo.len() {
        // Some answered: what they said stands; the rest is listed.
        out.error = None;
    }
    // A repository that did not answer keeps what was found before.
    for (name, f) in &before.found {
        if out.failed.contains(&f.repo) && !out.found.contains_key(name) {
            out.found.insert(name.clone(), f.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blob_id_is_git_s() {
        // `printf 'hello\n' | git hash-object --stdin`
        assert_eq!(blob_hash(b"hello\n"), "ce013625030ba8dba906f756967f9e9ca394464a");
        // The empty tree.
        assert_eq!(hex(&object("tree", b"")), EMPTY_TREE);
    }

    #[test]
    fn changes_in_plain_words() {
        let c = Changes {
            modified: vec!["SKILL.md".into()],
            added: vec!["x.md".into()],
            removed: vec![],
        };
        assert_eq!(plain(&c), "Sửa hướng dẫn, thêm 1 file");
        let c = Changes {
            modified: vec!["a.py".into(), "b.py".into()],
            added: vec![],
            removed: vec!["c".into()],
        };
        assert_eq!(plain(&c), "Sửa 2 file, bỏ 1 file");
    }
}
