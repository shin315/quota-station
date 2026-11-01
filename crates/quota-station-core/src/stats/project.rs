// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Theo dự án's rows: the folders the sessions ran in, put together by the
//! repository they belong to. An agent often works in a git worktree at
//! another path (`~/.treehouse/<repo>-<hash>/<n>/<repo>`), in a second
//! clone, or in a subfolder; each is the same project as the repository.
//!
//! A folder is looked up the way git finds its repository: the nearest
//! `.git` above it, a worktree's `.git` file led to the repository's own
//! folder by its `commondir`. Clones are one project when their `origin`
//! is the same remote. A folder that is gone (a worktree removed after the
//! session) is found in a repository's `worktrees/` while git still lists
//! it, and otherwise goes by its own name, with the one repository of that
//! name when there is just one. Two repositories that share a name and not a
//! remote stay two rows. Only git's own small files are read; git is never
//! run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::report::ProjectRow;

/// The repository a folder is in.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Repo {
    /// The remote, or the repository's folder when it has none.
    key: String,
    /// The repository's own folder (`.git` or a bare one).
    common: PathBuf,
    /// Where the repository is checked out: the main worktree's folder.
    root: PathBuf,
}

/// One row being added up.
#[derive(Default)]
struct Group {
    /// Each folder the row stands for, with its tokens.
    members: BTreeMap<PathBuf, u64>,
    /// The repositories' own folders, for a row of clones.
    commons: BTreeSet<PathBuf>,
}

impl Group {
    fn add(&mut self, folder: PathBuf, tokens: u64) {
        *self.members.entry(folder).or_default() += tokens;
    }

    fn tokens(&self) -> u64 {
        self.members.values().sum()
    }

    /// The folder that names the row: of those still there, the one with
    /// the most tokens.
    fn main(&self) -> &Path {
        self.members
            .iter()
            .map(|(p, n)| (p.is_dir(), *n, p))
            .max_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then_with(|| b.2.cmp(a.2)))
            .map(|(_, _, p)| p.as_path())
            .unwrap_or(Path::new(""))
    }
}

/// The rows for `projects`, a session folder's path and its tokens, each
/// row's path from the home as `~`.
pub fn rows(projects: BTreeMap<String, u64>, home: &Path) -> Vec<ProjectRow> {
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    let mut gone: Vec<(PathBuf, u64)> = Vec::new();
    for (path, tokens) in projects {
        if path.is_empty() {
            groups.entry(String::new()).or_default().add(PathBuf::new(), tokens);
            continue;
        }
        let folder = PathBuf::from(&path);
        if !folder.is_dir() {
            gone.push((folder, tokens));
            continue;
        }
        match repo_of(&folder, home) {
            Some(repo) => {
                let group = groups.entry(repo.key).or_default();
                group.add(repo.root, tokens);
                group.commons.insert(repo.common);
            }
            // A worktree whose repository is gone: no longer git.
            None if in_git(&folder, home) => gone.push((folder, tokens)),
            None => groups.entry(format!("dir:{path}")).or_default().add(folder, tokens),
        }
    }

    // A folder that is gone: a worktree git still lists, or the one
    // repository of its name.
    let listed = listed_worktrees(&groups);
    let gone: Vec<(String, PathBuf, u64)> = gone
        .into_iter()
        .map(|(folder, tokens)| {
            let key = real(&folder)
                .ancestors()
                .find_map(|a| listed.get(a).cloned())
                .or_else(|| same_name(&groups, name_of(&folder)))
                .unwrap_or_else(|| format!("name:{}", name_of(&folder)));
            (key, folder, tokens)
        })
        .collect();
    for (key, folder, tokens) in gone {
        groups.entry(key).or_default().add(folder, tokens);
    }

    groups
        .into_values()
        .map(|g| {
            let main = g.main();
            ProjectRow {
                name: name_of(main).to_string(),
                path: if main.as_os_str().is_empty() { String::new() } else { show(main, home) },
                tokens: g.tokens(),
            }
        })
        .collect()
}

/// A folder's own name; a bare repository's without its `.git`.
fn name_of(path: &Path) -> &str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.strip_suffix(".git").filter(|n| !n.is_empty()).unwrap_or(name)
}

/// The key of the one repository row named `name`, if just one is.
fn same_name(groups: &BTreeMap<String, Group>, name: &str) -> Option<String> {
    let mut found = groups
        .iter()
        .filter(|(k, _)| !k.is_empty() && !k.starts_with("dir:") && !k.starts_with("name:"))
        .filter(|(_, g)| g.members.keys().any(|p| name_of(p) == name));
    let (key, _) = found.next()?;
    found.next().is_none().then(|| key.clone())
}

/// Every worktree the repositories of `groups` list (`worktrees/*/gitdir`),
/// there or not, with its row's key.
fn listed_worktrees(groups: &BTreeMap<String, Group>) -> BTreeMap<PathBuf, String> {
    let mut listed = BTreeMap::new();
    for (key, group) in groups {
        for common in &group.commons {
            let Ok(entries) = std::fs::read_dir(common.join("worktrees")) else { continue };
            for e in entries.flatten() {
                let Some(dot_git) = read_path(&e.path().join("gitdir"), &e.path()) else {
                    continue;
                };
                if let Some(folder) = dot_git.parent() {
                    listed.insert(real(folder), key.clone());
                }
            }
        }
    }
    listed
}

/// `path` with its links resolved as far as it is there: `/var/x/gone` is
/// `/private/var/x/gone` on macOS.
fn real(path: &Path) -> PathBuf {
    for at in path.ancestors() {
        if let Ok(real) = at.canonicalize() {
            return real.join(path.strip_prefix(at).unwrap_or(Path::new("")));
        }
    }
    path.to_path_buf()
}

/// The folders from `folder` up to the home, which is left out: a home kept
/// in git is not every folder's project.
fn above<'a>(folder: &'a Path, home: &'a Path) -> impl Iterator<Item = &'a Path> {
    folder.ancestors().take_while(move |a| *a != home)
}

/// Whether a `.git` is above `folder`.
fn in_git(folder: &Path, home: &Path) -> bool {
    above(folder, home).any(|a| a.join(".git").exists())
}

/// The repository `folder` is in, from the nearest `.git` above it.
fn repo_of(folder: &Path, home: &Path) -> Option<Repo> {
    let at = above(folder, home).find(|a| a.join(".git").exists())?;
    let dot_git = at.join(".git");
    let (common, root) = if dot_git.is_dir() {
        let common = dot_git.canonicalize().unwrap_or(dot_git);
        let root = common.parent()?.to_path_buf();
        (common, root)
    } else {
        // A worktree, a submodule or a separate git folder: `gitdir: <path>`.
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let git_dir = resolve(text.trim().strip_prefix("gitdir:")?.trim(), at);
        let common = match read_path(&git_dir.join("commondir"), &git_dir) {
            Some(c) => c,
            None if git_dir.is_dir() => git_dir.clone(),
            None => return None,
        };
        let root = if common.file_name().is_some_and(|n| n == ".git") {
            common.parent()?.to_path_buf()
        } else if common == git_dir {
            real(at)
        } else {
            // A worktree of a bare repository.
            common.clone()
        };
        (common, root)
    };
    let key = remote(&common).unwrap_or_else(|| format!("git:{}", common.display()));
    Some(Repo { key, common, root })
}

/// A path a git file holds, relative to `base` unless it is absolute.
fn read_path(file: &Path, base: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(file).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| resolve(text, base))
}

fn resolve(path: &str, base: &Path) -> PathBuf {
    let path = Path::new(path);
    let joined = if path.is_absolute() { path.to_path_buf() } else { base.join(path) };
    joined.canonicalize().unwrap_or(joined)
}

/// The repository's `origin`, or its first remote, as `host/owner/repo`.
fn remote(common: &Path) -> Option<String> {
    let text = std::fs::read_to_string(common.join("config")).ok()?;
    let mut section = String::new();
    let mut urls: Vec<(String, String)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line.trim_matches(|c| c == '[' || c == ']').to_string();
        } else if let Some(name) = section.strip_prefix("remote ") {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case("url") {
                    urls.push((name.trim_matches('"').to_string(), v.trim().to_string()));
                }
            }
        }
    }
    let url = urls.iter().find(|(n, _)| n == "origin").or(urls.first())?;
    Some(format!("remote:{}", same_remote(&url.1)))
}

/// `git@github.com:Demo/web.git`, `https://github.com/demo/web` and
/// `ssh://git@github.com/demo/web.git/` are all `github.com/demo/web`.
fn same_remote(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let rest = match url.split_once("://") {
        Some((_, rest)) => rest.to_string(),
        // `user@host:path`, git's short form for ssh; a local path stays.
        None => match url.split_once(':') {
            Some((host, path)) if !host.contains('/') => format!("{host}/{path}"),
            _ => url.to_string(),
        },
    };
    let rest = match rest.split_once('/') {
        Some((host, path)) => {
            let host = host.rsplit('@').next().unwrap_or(host);
            format!("{host}/{}", path.trim_start_matches('/'))
        }
        None => rest,
    };
    rest.to_lowercase()
}

/// `/Users/x/Projects/web` → `~/Projects/web` for a home of `/Users/x`,
/// the home's links resolved or not.
fn show(path: &Path, home: &Path) -> String {
    let real_home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let home = if path.starts_with(home) { home } else { &real_home };
    crate::runner::display_path(path, home)
}

#[cfg(test)]
mod tests {
    use super::same_remote;

    #[test]
    fn one_remote_however_written() {
        for url in [
            "git@github.com:Demo/web-shop.git",
            "https://github.com/demo/web-shop",
            "https://github.com/demo/web-shop.git/",
            "ssh://git@github.com/demo/web-shop.git",
            "https://token@github.com/demo/web-shop.git",
        ] {
            assert_eq!(same_remote(url), "github.com/demo/web-shop", "{url}");
        }
        assert_eq!(same_remote("/Users/demo/src/web-shop.git"), "/users/demo/src/web-shop");
        assert_ne!(
            same_remote("git@github.com:acme/web-shop"),
            same_remote("git@github.com:demo/web-shop")
        );
    }
}
