//! "Cập nhật từ mã nguồn" against temporary git repositories: a bare
//! `origin`, the clone the app updates from, and a second clone that pushes
//! new commits. The clone's `scripts/install.sh` is a fake that records its
//! arguments, so nothing is built or installed. git runs with a temporary
//! HOME and no global or system config.

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use common::{script, temp_dir};
use quota_station_core::update::{
    self, check_source, install_dest, Outcome, SourceError, Step, Tools, UpdateError,
};

struct Repos {
    root: PathBuf,
    /// The clone the app updates from.
    source: PathBuf,
    /// Another clone, where new commits come from.
    upstream: PathBuf,
    dest: PathBuf,
    log: PathBuf,
}

const FAKE_INSTALL: &str = r#"echo "install.sh $*" >> "$FAKE_LOG"
echo "==> Installed Quota Station"
if [ -f "$(dirname "$0")/fail" ]; then
  echo "   Compiling quota-station" >&2
  echo "install.sh: error: cargo is not installed: install Rust with rustup" >&2
  exit 1
fi
"#;

fn env(root: &Path) -> Vec<(String, OsString)> {
    vec![
        ("HOME".into(), root.join("home").into()),
        ("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("FAKE_LOG".into(), root.join("calls.log").into()),
    ]
}

fn git(root: &Path, dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.invalid"])
        .args(["-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main"])
        .args(args)
        .current_dir(dir)
        .envs(env(root))
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

impl Repos {
    fn new(tag: &str) -> Repos {
        let root = temp_dir(tag);
        std::fs::create_dir_all(root.join("home")).unwrap();
        git(&root, &root, &["init", "-q", "--bare", "origin.git"]);
        git(&root, &root, &["clone", "-q", "origin.git", "upstream"]);
        let upstream = root.join("upstream");
        script(&upstream.join("scripts/install.sh"), FAKE_INSTALL);
        std::fs::create_dir_all(upstream.join("src-tauri")).unwrap();
        std::fs::write(upstream.join("src-tauri/tauri.conf.json"), r#"{ "version": "0.1.0" }"#)
            .unwrap();
        git(&root, &upstream, &["add", "-A"]);
        git(&root, &upstream, &["commit", "-q", "-m", "first"]);
        git(&root, &upstream, &["push", "-q", "origin", "HEAD:main"]);
        git(&root, &root, &["clone", "-q", "origin.git", "source"]);
        std::fs::write(root.join("calls.log"), "").unwrap();
        Repos {
            source: root.join("source"),
            upstream,
            dest: root.join("Applications"),
            log: root.join("calls.log"),
            root,
        }
    }

    /// A new commit on origin, with the version it builds.
    fn push(&self, version: &str) -> String {
        let conf = format!(r#"{{ "version": "{version}" }}"#);
        std::fs::write(self.upstream.join("src-tauri/tauri.conf.json"), conf).unwrap();
        git(&self.root, &self.upstream, &["commit", "-q", "-am", version]);
        git(&self.root, &self.upstream, &["push", "-q", "origin", "HEAD:main"]);
        self.head(&self.upstream)
    }

    fn head(&self, dir: &Path) -> String {
        git(&self.root, dir, &["rev-parse", "HEAD"])
    }

    fn tools(&self) -> Tools {
        Tools { git: which_git(), bash: PathBuf::from("/bin/bash"), env: env(&self.root) }
    }

    fn run(&self, running: Option<&str>) -> (Result<Outcome, UpdateError>, Vec<Step>) {
        let steps = Mutex::new(Vec::new());
        let log = self.root.join("app/update/install.log");
        let result =
            update::update(&self.tools(), &self.source, &self.dest, running, Some(&log), &|s| {
                steps.lock().unwrap().push(s)
            });
        (result, steps.into_inner().unwrap())
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log).unwrap().lines().map(String::from).collect()
    }
}

fn which_git() -> PathBuf {
    let out = Command::new("/usr/bin/which").arg("git").output().unwrap();
    PathBuf::from(String::from_utf8_lossy(&out.stdout).trim())
}

#[test]
fn pulls_then_installs_into_the_folder_of_the_running_app() {
    let r = Repos::new("update-new");
    let old = r.head(&r.source);
    let new = r.push("0.2.0");
    let (result, steps) = r.run(Some(&old));
    assert_eq!(
        result.unwrap(),
        Outcome::Installed { commit: new.clone(), version: Some("0.2.0".into()) }
    );
    assert_eq!(steps, vec![Step::Pulling, Step::Installing]);
    assert_eq!(r.head(&r.source), new, "the clone moved forward");
    let script = r.source.join("scripts/install.sh");
    assert_eq!(
        r.calls(),
        vec![format!("install.sh --dest {} --no-open", r.dest.display())],
        "{}",
        script.display()
    );
    // The script's output is kept for the owner only.
    let log = r.root.join("app/update/install.log");
    assert!(std::fs::read_to_string(&log).unwrap().contains("==> Installed Quota Station"));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&log).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn nothing_new_and_built_from_it_installs_nothing() {
    let r = Repos::new("update-same");
    let head = r.head(&r.source);
    let (result, steps) = r.run(Some(&head));
    assert_eq!(result.unwrap(), Outcome::UpToDate { commit: head });
    assert_eq!(steps, vec![Step::Pulling]);
    assert!(r.calls().is_empty());
}

#[test]
fn a_copy_built_from_a_changed_tree_or_unknown_installs_again() {
    let r = Repos::new("update-dirty");
    let head = r.head(&r.source);
    let (result, _) = r.run(Some(&format!("{head}-dirty")));
    assert!(matches!(result.unwrap(), Outcome::Installed { .. }));
    let (result, _) = r.run(None);
    assert!(matches!(result.unwrap(), Outcome::Installed { .. }));
    assert_eq!(r.calls().len(), 2);
}

#[test]
fn a_clone_that_cannot_fast_forward_is_left_as_it_is() {
    let r = Repos::new("update-diverged");
    std::fs::write(r.source.join("local.txt"), "a local change").unwrap();
    git(&r.root, &r.source, &["add", "local.txt"]);
    git(&r.root, &r.source, &["commit", "-q", "-m", "local"]);
    let local = r.head(&r.source);
    r.push("0.2.0");
    let (result, steps) = r.run(None);
    match result.unwrap_err() {
        UpdateError::Pull { detail } => {
            assert!(detail.to_lowercase().contains("fast-forward"), "{detail}")
        }
        other => panic!("expected a pull error, got {other:?}"),
    }
    assert_eq!(steps, vec![Step::Pulling]);
    assert_eq!(r.head(&r.source), local);
    assert!(r.calls().is_empty(), "nothing is installed");
}

#[test]
fn an_unreachable_origin_installs_nothing() {
    let r = Repos::new("update-unreachable");
    std::fs::rename(r.root.join("origin.git"), r.root.join("gone.git")).unwrap();
    let (result, _) = r.run(None);
    assert!(matches!(result.unwrap_err(), UpdateError::Pull { .. }));
    assert!(r.calls().is_empty());
}

#[test]
fn a_failed_install_says_the_script_error() {
    let r = Repos::new("update-fail");
    let old = r.head(&r.source);
    r.push("0.2.0");
    std::fs::write(r.source.join("scripts/fail"), "").unwrap();
    let (result, steps) = r.run(Some(&old));
    assert_eq!(
        result.unwrap_err(),
        UpdateError::Install { detail: "cargo is not installed: install Rust with rustup".into() }
    );
    assert_eq!(steps, vec![Step::Pulling, Step::Installing]);
    let log = std::fs::read_to_string(r.root.join("app/update/install.log")).unwrap();
    assert!(log.contains("Compiling quota-station"));
}

#[test]
fn git_never_waits_for_a_password() {
    let r = Repos::new("update-prompt");
    let fake = script(
        &r.root.join("bin/git"),
        r#"echo "git $* prompt=$GIT_TERMINAL_PROMPT ssh=$GIT_SSH_COMMAND" >> "$FAKE_LOG"
[ "$3" = "rev-parse" ] && echo 0123456789abcdef0123456789abcdef01234567
exit 0
"#,
    );
    let tools = Tools { git: fake, ..r.tools() };
    let result = update::update(&tools, &r.source, &r.dest, None, None, &|_| {});
    assert!(result.is_ok());
    let calls = r.calls();
    let source = r.source.display();
    assert_eq!(
        calls[0],
        format!("git -C {source} pull --ff-only prompt=0 ssh=ssh -o BatchMode=yes")
    );
}

#[test]
fn only_a_clone_with_the_install_script_is_a_source() {
    let r = Repos::new("update-source");
    assert_eq!(check_source(&r.source), Ok(()));
    assert_eq!(check_source(&r.root.join("nowhere")), Err(SourceError::Missing));
    let plain = r.root.join("plain");
    script(&plain.join("scripts/install.sh"), "exit 0\n");
    assert_eq!(check_source(&plain), Err(SourceError::NotSource), "no .git");
    let bare_clone = r.root.join("other");
    git(&r.root, &r.root, &["init", "-q", "other"]);
    assert_eq!(check_source(&bare_clone), Err(SourceError::NotSource), "no install script");
    // A linked work tree has a .git file, not a folder.
    git(&r.root, &r.source, &["worktree", "add", "-q", "--detach", "../linked"]);
    assert_eq!(check_source(&r.root.join("linked")), Ok(()));

    let (result, _) = {
        let steps = Mutex::new(Vec::new());
        let res = update::update(&r.tools(), &plain, &r.dest, None, None, &|s| {
            steps.lock().unwrap().push(s)
        });
        (res, steps.into_inner().unwrap())
    };
    assert_eq!(result.unwrap_err(), UpdateError::Source { reason: SourceError::NotSource });
    assert!(r.calls().is_empty());
}

#[test]
fn installs_next_to_the_app_that_runs() {
    let exe = Path::new("/Applications/Quota Station.app/Contents/MacOS/quota-station");
    assert_eq!(install_dest(exe), Some(PathBuf::from("/Applications")));
    let exe = Path::new("/Users/example/Apps/Quota Station.app/Contents/MacOS/quota-station");
    assert_eq!(install_dest(exe), Some(PathBuf::from("/Users/example/Apps")));
    assert_eq!(install_dest(Path::new("/repo/target/debug/quota-station")), None);
    assert_eq!(install_dest(Path::new("/x/Contents/MacOS/quota-station")), None);
}
