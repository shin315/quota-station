// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The command line's words: `claude …` goes to the app's commands, with
//! the advanced part's; anything else gets the usage.

fn call(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = quota_station_cli::run(&args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

#[test]
fn a_word_that_is_not_claude_gets_the_usage() {
    assert_eq!(call(&[]), (2, String::new(), quota_station_cli::USAGE.to_string()));
    assert_eq!(call(&["quota"]).0, 2);
    assert_eq!(call(&["--help"]), (0, quota_station_cli::USAGE.to_string(), String::new()));
}

/// `claude help` is the advanced part's usage. (Run here it would reach for
/// this system's secret store; the core's tests run the commands on fakes.)
#[test]
fn the_commands_are_the_advanced_parts() {
    use quota_station_core::ext::Hooks;
    let usage = quota_station_cli::HOOKS.claude_usage().unwrap();
    assert_eq!(usage, quota_station_core::advanced::cli::USAGE);
    assert!(usage.contains("  rotate [--dry-run]"), "{usage}");
    assert!(usage.contains("  rotation on|off"), "{usage}");
}
