// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Where the programs the app runs are looked for, and the PATH they get.
//! Nothing here runs a real program.

mod common;

use std::path::{Path, PathBuf};

use common::{script, temp_dir};
use quota_station_core::runner::{bin_dirs, child_path, display_path, expand_path, on_path};

#[test]
fn the_child_finds_node_next_to_the_program() {
    let p = child_path(Path::new("/opt/tools/bin/codex"), Some("/usr/bin:/bin".into()));
    let dirs: Vec<PathBuf> = std::env::split_paths(&p).collect();
    assert_eq!(dirs[0], PathBuf::from("/opt/tools/bin"));
    assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
    assert_eq!(dirs.last(), Some(&PathBuf::from("/bin")));
}

#[test]
fn installed_commands_are_looked_for_in_local_bin_then_the_newest_nvm_node() {
    let home = temp_dir("runner-bins");
    for v in ["v20.1.0", "v24.18.0", "v24.9.0", "system"] {
        script(&home.join(".nvm/versions/node").join(v).join("bin/claude"), "exit 0\n");
    }
    let dirs = bin_dirs(&home);
    assert_eq!(
        dirs,
        [
            home.join(".local/bin"),
            home.join(".nvm/versions/node/v24.18.0/bin"),
            home.join(".nvm/versions/node/v24.9.0/bin"),
            home.join(".nvm/versions/node/v20.1.0/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ]
    );
}

#[test]
fn a_program_nowhere_on_the_path_is_not_found() {
    assert_eq!(on_path("quota-station-test-no-such-program"), None);
}

#[test]
fn paths_show_home_as_a_tilde() {
    let home = Path::new("/home/someone");
    assert_eq!(display_path(&home.join(".codex"), home), "~/.codex");
    assert_eq!(display_path(home, home), "~");
    assert_eq!(display_path(Path::new("/opt/homebrew/bin/agy"), home), "/opt/homebrew/bin/agy");
    assert_eq!(expand_path(" ~/.codex ", home), home.join(".codex"));
    assert_eq!(expand_path("~", home), home.to_path_buf());
    assert_eq!(expand_path("/srv/codex", home), PathBuf::from("/srv/codex"));
}
