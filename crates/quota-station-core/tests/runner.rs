//! quota-axi is replaced by a fake shell script: these tests never run the
//! real quota-axi, never touch the Keychain, and never read real quota.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use common::fixture_path;
use quota_station_core::runner::{
    self, candidates, child_path, discover, display_path, expand_path, ProviderId, RunError,
};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A fresh, empty directory for one test.
fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "qs-runner-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A fake quota-axi that records its arguments next to itself, prints
/// `stdout_file`, and exits with `code`.
fn fake(dir: &Path, stdout_file: Option<&Path>, code: i32) -> PathBuf {
    let path = dir.join("quota-axi");
    let print = stdout_file.map_or(String::new(), |f| format!("cat '{}'\n", f.display()));
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nif [ \"$1\" = --version ]; then echo 0.1.61+cswap.4; exit 0; fi\n{print}echo 'something went wrong' >&2\nexit {code}\n",
        dir.join("args").display()
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn recorded_args(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("args")).unwrap().lines().map(String::from).collect()
}

#[test]
fn a_read_reuses_the_cache_and_never_refreshes_credentials() {
    let dir = temp_dir();
    let axi = fake(&dir, Some(&fixture_path("claude-pool-schema5.json")), 0);
    let report = runner::read(&axi, ProviderId::Claude).unwrap();
    assert_eq!(report.schema_version, 5);
    assert_eq!(
        recorded_args(&dir),
        ["--provider", "claude", "--json", "--max-age", "120s", "--no-credential-refresh"]
    );
}

#[test]
fn a_report_on_a_failing_exit_still_counts() {
    // quota-axi exits 1 when a provider needs attention (Keychain) and still prints the report.
    let dir = temp_dir();
    let axi = fake(&dir, Some(&fixture_path("claude-keychain-required.json")), 1);
    assert!(runner::read(&axi, ProviderId::Claude).is_ok());
}

#[test]
fn an_exit_without_a_report_is_an_error_with_stderr() {
    let dir = temp_dir();
    let axi = fake(&dir, None, 2);
    let err = runner::read(&axi, ProviderId::Agy).unwrap_err();
    assert_eq!(err, RunError::Exit { code: Some(2), stderr: "something went wrong".to_string() });
}

#[test]
fn a_missing_binary_is_not_found() {
    let dir = temp_dir();
    let err = runner::read(&dir.join("nope"), ProviderId::Claude).unwrap_err();
    assert!(matches!(err, RunError::NotFound(_)));
}

#[test]
fn an_unsupported_schema_is_a_parse_error() {
    let dir = temp_dir();
    let axi = fake(&dir, Some(&fixture_path("schema-unsupported.json")), 0);
    let err = runner::read(&axi, ProviderId::Claude).unwrap_err();
    assert!(matches!(err, RunError::Parse(_)));
}

#[test]
fn only_the_grant_button_allows_the_keychain_prompt() {
    assert!(!runner::read_args(ProviderId::Claude).contains(&"--allow-keychain-prompt".to_string()));
    assert!(!runner::read_args(ProviderId::Agy).contains(&"--allow-keychain-prompt".to_string()));

    let dir = temp_dir();
    let axi = fake(&dir, Some(&fixture_path("claude-pool-schema5.json")), 0);
    runner::grant_keychain(&axi).unwrap();
    assert_eq!(
        recorded_args(&dir),
        ["--provider", "claude", "--json", "--allow-keychain-prompt", "--no-credential-refresh"]
    );
}

#[test]
fn version_names_the_fork() {
    let dir = temp_dir();
    let axi = fake(&dir, None, 0);
    let v = runner::version(&axi).unwrap();
    assert_eq!(v, "0.1.61+cswap.4");
    assert!(runner::is_fork(&v));
    assert!(!runner::is_fork("0.1.61"));
}

#[test]
fn the_child_finds_node_next_to_quota_axi() {
    let p = child_path(Path::new("/opt/tools/bin/quota-axi"), Some("/usr/bin:/bin".into()));
    let dirs: Vec<PathBuf> = std::env::split_paths(&p).collect();
    assert_eq!(dirs[0], PathBuf::from("/opt/tools/bin"));
    assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
    assert_eq!(dirs.last(), Some(&PathBuf::from("/bin")));
}

#[test]
fn discovery_prefers_local_bin_then_the_newest_nvm_node() {
    let home = temp_dir();
    for v in ["v20.1.0", "v24.18.0", "v24.9.0"] {
        let bin = home.join(".nvm/versions/node").join(v).join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        fake(&bin, None, 0);
    }
    let list = candidates(&home);
    assert_eq!(list[0], home.join(".local/bin/quota-axi"));
    assert_eq!(list[1], home.join(".nvm/versions/node/v24.18.0/bin/quota-axi"));
    assert_eq!(list[2], home.join(".nvm/versions/node/v24.9.0/bin/quota-axi"));
    assert_eq!(discover(&home), Some(home.join(".nvm/versions/node/v24.18.0/bin/quota-axi")));
}

#[test]
fn paths_show_home_as_a_tilde() {
    let home = Path::new("/home/someone");
    assert_eq!(display_path(&home.join(".local/bin/quota-axi"), home), "~/.local/bin/quota-axi");
    assert_eq!(
        display_path(Path::new("/opt/homebrew/bin/quota-axi"), home),
        "/opt/homebrew/bin/quota-axi"
    );
    assert_eq!(expand_path(" ~/.local/bin/quota-axi ", home), home.join(".local/bin/quota-axi"));
}
