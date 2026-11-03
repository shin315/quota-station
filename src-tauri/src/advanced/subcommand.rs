// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's subcommands from a terminal
//! ([`crate::ext::Extension::subcommand`]): on a Mac, `quota-station agents
//! <word>`, which another Mac's Quota Station runs here over SSH (M13),
//! answered without a window while nothing else starts ([`crate::copy`]),
//! with the app's state opened with the hooks the app runs with. Windows
//! has no copy to another Mac, so there it answers nothing and the app
//! refuses `agents` as a build without the advanced part does.

use crate::ext::Hooks;

/// The exit code of the subcommand `args` name, when it is the private
/// part's; `None` leaves the words to the app.
#[cfg(unix)]
pub fn answer(args: &[String], hooks: &'static dyn Hooks) -> Option<i32> {
    (args.first()? == "agents").then(|| crate::copy::cli(&args[1..], hooks))
}

/// Windows: nothing is the advanced part's.
#[cfg(not(unix))]
pub fn answer(args: &[String], hooks: &'static dyn Hooks) -> Option<i32> {
    let _ = (args, hooks);
    None
}

#[cfg(test)]
mod tests {
    use super::answer;
    use crate::advanced::hooks::AdvancedHooks;

    fn words(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn the_apps_subcommands_and_a_plain_launch_are_left_to_the_app() {
        for args in [
            words(&[]),
            words(&["claude", "status"]),
            words(&["quota", "--json"]),
            words(&["nonsense"]),
            words(&["--agents"]),
        ] {
            assert_eq!(answer(&args, &AdvancedHooks), None, "{args:?}");
        }
    }

    /// Windows has no copy to another Mac: the app refuses `agents` there.
    #[cfg(not(unix))]
    #[test]
    fn on_windows_agents_is_left_to_the_apps_refusal() {
        assert_eq!(answer(&words(&["agents", "read"]), &AdvancedHooks), None);
    }
}
