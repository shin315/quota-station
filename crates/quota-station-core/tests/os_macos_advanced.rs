// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The macOS side of the advanced part's scheduler, against a fake
//! `launchctl` shell script (the core's `os_macos` has the app's own
//! commands): the commands it runs and how it reads their answers. The
//! other tests use the fake kept in memory (`common::advanced::FakeScheduler`).
#![cfg(target_os = "macos")]

mod common;

use std::path::{Path, PathBuf};

use common::{script, temp_dir};
use quota_station_core::claude::agent::{self, AgentError, Launchd};
use quota_station_core::os::Scheduler;

/// A `launchctl` that keeps loaded labels as files in `dir/loaded/`:
/// `list LABEL` exits 0 or 113, `bootstrap DOMAIN PLIST` loads the plist's
/// label, `bootout DOMAIN/LABEL` unloads it. Every run is recorded.
fn fake_launchctl(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("loaded")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/launchctl"),
        &format!(
            r#"printf '%s\n' "$*" >> '{d}/launchctl-calls'
case "$1" in
  list) [ -e '{d}/loaded/'"$2" ] && exit 0; exit 113 ;;
  bootstrap) label=$(basename "$3" .plist); : > '{d}/loaded/'"$label" ;;
  bootout) label=${{2##*/}}; [ -e '{d}/loaded/'"$label" ] || exit 3; rm '{d}/loaded/'"$label" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}

#[test]
fn launchd_loads_the_rotation_agent_and_unloads_it() {
    let dir = temp_dir("os-launchd");
    let home = dir.join("home");
    let exe = dir.join("Quota Station.app/Contents/MacOS/quota-station");
    let launchd = Launchd { launchctl: fake_launchctl(&dir) };
    let plist = agent::plist_path(&home);

    assert_eq!(launchd.loaded(), Some(false));
    assert_eq!(launchd.legacy_loaded(), None);
    launchd.install(&home, &exe).unwrap();
    assert_eq!(launchd.loaded(), Some(true));
    let text = std::fs::read_to_string(&plist).unwrap();
    assert!(text.contains(&format!("<string>{}</string>", exe.display())), "{text}");
    let calls = std::fs::read_to_string(dir.join("launchctl-calls")).unwrap();
    assert!(calls.lines().any(|l| l.starts_with("bootstrap gui/")), "{calls}");

    launchd.remove(&home).unwrap();
    assert_eq!(launchd.loaded(), Some(false));
    assert!(!plist.exists());

    // An older switcher loaded: nothing is installed beside it.
    std::fs::write(dir.join("loaded/com.cswap.prefer-team"), "").unwrap();
    assert_eq!(launchd.legacy_loaded(), Some("com.cswap.prefer-team"));
    assert_eq!(
        launchd.install(&home, &exe),
        Err(AgentError::Legacy("com.cswap.prefer-team".into()))
    );

    // `launchctl` that cannot tell counts as an older switcher running.
    let unknown = Launchd { launchctl: script(&dir.join("bin/launchctl-unknown"), "exit 1\n") };
    assert_eq!(unknown.loaded(), None);
    assert_eq!(unknown.legacy_loaded(), Some("com.cswap.prefer-team"));
}
