// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The commands and the subcommands the advanced part adds to the app, as
//! the app's extension (`Advanced`, which the app's binary passes). The app
//! crate's own test checks that none of them is one of the app's own.

use quota_station::advanced::Advanced;
use quota_station::ext::Extension;

/// The advanced part's commands on every system: Nâng cao, the pause, the
/// rotation and its rules editor, and the quit guard for unsaved rules.
const ADVANCED_COMMANDS: [&str; 11] = [
    "set_advanced",
    "pause_switching",
    "extend_pause",
    "resume_switching",
    "claude_set_enabled",
    "claude_set_rotation",
    "save_rotation",
    "preview_rotation",
    "rules_dirty",
    "take_quit",
    "close_main",
];

/// Those only a Mac has: the rules' copy to the other Macs, the copy to
/// another Mac, and the update from source.
const MAC_COMMANDS: [&str; 11] = [
    "adopt_rotation",
    "keep_rotation",
    "retry_rotation",
    "set_peers",
    "test_peer",
    "copy_compare",
    "copy_run",
    "copy_undo",
    "set_source_dir",
    "update_from_source",
    "relaunch",
];

#[test]
fn the_advanced_parts_commands_come_with_it() {
    let names = Advanced.command_names();
    for command in ADVANCED_COMMANDS {
        assert!(names.contains(&command), "{command}");
    }
    for command in MAC_COMMANDS {
        assert_eq!(names.contains(&command), cfg!(unix), "{command}");
    }
}

/// Windows has no copy to another Mac: the advanced part leaves `agents`
/// unanswered there, so the app refuses it as it does without an extension.
#[cfg(not(unix))]
#[test]
fn on_windows_the_advanced_part_leaves_agents_to_the_apps_refusal() {
    let args: Vec<String> = ["agents", "read"].iter().map(|w| w.to_string()).collect();
    assert_eq!(Advanced.subcommand(&args, quota_station_core::ext::none()), None);
}
