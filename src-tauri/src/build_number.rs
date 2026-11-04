// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The build number shown next to the version: `build.rs` includes this
//! file to pass it to the app (`QS_BUILD_NUMBER`), and the crate includes it
//! for its tests only.
//!
//! It counts the commits reachable from the commit being built
//! (`git rev-list --count HEAD`). A commit added on top, as `git pull
//! --ff-only` brings, makes it one or more larger, so it only grows along
//! the history a clone follows.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What [`build_number`] asks git: its trimmed output, `None` when git is
/// missing or fails.
pub type Git<'a> = &'a dyn Fn(&[&str]) -> Option<String>;

/// `QS_BUILD` (`env`) when the build script sets it to a number, as
/// `scripts/install.sh` does; else the number of commits reachable from
/// `HEAD`, from `git`. `None` without git, outside a repository, or in a
/// shallow clone, whose count is only how deep it was fetched.
pub fn build_number(env: Option<&str>, git: Git<'_>) -> Option<String> {
    if let Some(n) = env.map(str::trim).filter(|n| is_number(n)) {
        return Some(n.to_string());
    }
    if git(&["rev-parse", "--is-shallow-repository"])?.as_str() != "false" {
        return None;
    }
    git(&["rev-list", "--count", "HEAD"]).filter(|n| is_number(n))
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// The files that change when `HEAD` moves to another commit: `HEAD`, its
/// reflog (written on every move, also when the branch is only in the
/// packed refs), the branch it names and the packed refs, those that exist
/// (a missing one would make cargo run the build script every time). Paths
/// git gives relative are taken from `dir`, where it runs.
pub fn watched(dir: &Path, git: Git<'_>) -> Vec<PathBuf> {
    let mut names = vec!["HEAD".to_string(), "logs/HEAD".to_string(), "packed-refs".to_string()];
    if let Some(branch) = git(&["rev-parse", "--symbolic-full-name", "HEAD"]) {
        if branch.starts_with("refs/") {
            names.push(branch);
        }
    }
    names
        .iter()
        .filter_map(|name| git(&["rev-parse", "--git-path", name]))
        .map(|p| dir.join(p))
        .filter(|p| p.is_file())
        .collect()
}

/// Runs `git -C dir …` from the PATH.
pub fn git_in(dir: &Path) -> impl Fn(&[&str]) -> Option<String> + '_ {
    move |args| {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::{build_number, git_in, watched};

    /// A git that answers from a table, and says nothing to the rest.
    fn fake<'a>(answers: &'a [(&'a str, &'a str)]) -> impl Fn(&[&str]) -> Option<String> + 'a {
        move |args: &[&str]| {
            let asked = args.join(" ");
            answers.iter().find(|(q, _)| *q == asked).map(|(_, a)| a.to_string())
        }
    }

    const FULL: &[(&str, &str)] =
        &[("rev-parse --is-shallow-repository", "false"), ("rev-list --count HEAD", "412")];

    #[test]
    fn the_build_scripts_number_comes_first() {
        assert_eq!(build_number(Some("57"), &fake(FULL)), Some("57".into()));
        assert_eq!(build_number(Some(" 57\n"), &fake(FULL)), Some("57".into()));
        // Not a number: git counts.
        assert_eq!(build_number(Some(""), &fake(FULL)), Some("412".into()));
        assert_eq!(build_number(Some("abc"), &fake(FULL)), Some("412".into()));
        assert_eq!(build_number(None, &fake(FULL)), Some("412".into()));
    }

    #[test]
    fn no_git_or_a_shallow_clone_has_no_number() {
        assert_eq!(build_number(None, &|_: &[&str]| None), None);
        let shallow =
            [("rev-parse --is-shallow-repository", "true"), ("rev-list --count HEAD", "1")];
        assert_eq!(build_number(None, &fake(&shallow)), None);
        let odd = [("rev-parse --is-shallow-repository", "false"), ("rev-list --count HEAD", "x")];
        assert_eq!(build_number(None, &fake(&odd)), None);
        // The build script's number needs no git.
        assert_eq!(build_number(Some("9"), &|_: &[&str]| None), Some("9".into()));
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qs-build-number-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.invalid"])
            .args(["-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main"])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn a_repository_counts_its_commits_and_a_plain_folder_has_none() {
        let dir = temp_dir("repo");
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        for n in ["1", "2", "3"] {
            std::fs::write(repo.join("file"), n).unwrap();
            git(&repo, &["add", "file"]);
            git(&repo, &["commit", "-q", "-m", n]);
        }
        assert_eq!(build_number(None, &git_in(&repo)), Some("3".into()));
        // From a folder inside it, as build.rs runs in src-tauri.
        let inner = repo.join("src-tauri");
        std::fs::create_dir_all(&inner).unwrap();
        assert_eq!(build_number(None, &git_in(&inner)), Some("3".into()));
        let watched = watched(&inner, &git_in(&inner));
        assert!(watched.iter().any(|p| p.ends_with("HEAD")), "{watched:?}");
        assert!(watched.iter().any(|p| p.ends_with("refs/heads/main")), "{watched:?}");
        assert!(watched.iter().all(|p| p.is_file()), "{watched:?}");

        // With the branch only in the packed refs, the next commit writes a
        // loose ref no one watched: a watched file must still change.
        git(&repo, &["pack-refs", "--all", "--prune"]);
        let watched = super::watched(&inner, &git_in(&inner));
        let read = |paths: &[PathBuf]| -> Vec<Vec<u8>> {
            paths.iter().map(|p| std::fs::read(p).unwrap_or_default()).collect()
        };
        let before = read(&watched);
        std::fs::write(repo.join("file"), "4").unwrap();
        git(&repo, &["commit", "-q", "-am", "4"]);
        assert_ne!(read(&watched), before, "a commit after pack-refs changes none of {watched:?}");
        assert_eq!(build_number(None, &git_in(&inner)), Some("4".into()));

        // A shallow clone fetched one commit deep.
        let path = repo.display().to_string().replace('\\', "/");
        let url = if path.starts_with('/') {
            format!("file://{path}")
        } else {
            format!("file:///{path}")
        };
        git(&dir, &["clone", "-q", "--depth", "1", &url, "shallow"]);
        assert_eq!(build_number(None, &git_in(&dir.join("shallow"))), None);

        let plain = dir.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        // A folder outside any repository: the temporary folder may sit in
        // one on some machines, so only when git says it is not.
        if git_in(&plain)(&["rev-parse", "--git-dir"]).is_none() {
            assert_eq!(build_number(None, &git_in(&plain)), None);
            assert!(super::watched(&plain, &git_in(&plain)).is_empty());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
