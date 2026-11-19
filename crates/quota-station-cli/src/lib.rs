// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! `quota-station claude …` without the app, which needs a desktop and is
//! built for macOS and Windows only: the same commands, with the advanced
//! part's (`rotate`, `rotation on|off`, `enable N`, `disable N`), the same
//! files and the same JSON, on this system's seams
//! ([`quota_station_core::os`]). On Linux the logins are Claude Code's
//! `~/.claude/.credentials.json` and the app's files of mode 600, and the
//! rotation runs from a `systemd --user` timer.
//!
//! Built with the `antigravity-accounts` feature, it also has the app's
//! `quota-station antigravity …` commands
//! ([`quota_station_core::antigravity::cli`]), for the agy that runs on this
//! machine: on Linux a switch writes the file agy keeps its login in, and
//! `import` takes the accounts another computer's `export` wrote.

use std::io::Write;
use std::path::PathBuf;

use quota_station_core::advanced::hooks::AdvancedHooks;
use quota_station_core::claude::{cli, store::Store, Paths};
use quota_station_core::ext::Hooks;

/// The hooks the commands run with: the advanced part's, as the app's
/// extension gives them, for `rotate`, `rotation on|off`, `enable N` and
/// `disable N`, the rotation as the pool's policy and the accounts held out
/// of it.
pub const HOOKS: AdvancedHooks = AdvancedHooks;

/// What a word other than `claude` or `antigravity` gets.
pub const USAGE: &str = "\
usage: quota-station claude <command>
       quota-station antigravity <command>

  `quota-station claude help` and `quota-station antigravity help` list
  the commands.
";

/// Runs `args` (the words after the program's name); the exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match args.first().map(String::as_str) {
        Some("claude") => {
            let Some(home) = home() else {
                let _ = writeln!(
                    err,
                    "quota-station claude: no home folder (HOME is unset or not an absolute path)"
                );
                return 1;
            };
            claude(&home, &args[1..], &HOOKS, out, err)
        }
        Some("antigravity") => {
            let Some(home) = home() else {
                let _ = writeln!(
                    err,
                    "quota-station antigravity: no home folder (HOME is unset or not an absolute path)"
                );
                return 1;
            };
            antigravity(&home, &args[1..], out, err)
        }
        Some("help" | "--help" | "-h") => {
            let _ = write!(out, "{USAGE}");
            0
        }
        _ => {
            let _ = write!(err, "{USAGE}");
            2
        }
    }
}

/// `quota-station claude …` for `home`, with this system's secret store and
/// the commands `hooks` add.
pub fn claude(
    home: &std::path::Path,
    args: &[String],
    hooks: &dyn Hooks,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let cli = cli::Cli {
        store: Store::new(Paths::for_home(home), quota_station_core::os::secret_store())
            .with_hooks(hooks),
        tools: cli::Tools::system(home),
    };
    cli::run(&cli, hooks, args, out, err)
}

/// `quota-station antigravity …` for `home`: the app's commands, with this
/// system's store, agy's login and no desktop app on Linux.
#[cfg(feature = "antigravity-accounts")]
pub fn antigravity(
    home: &std::path::Path,
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    use quota_station_core::antigravity::cli;
    cli::run(&cli::Cli::system(home), args, out, err)
}

/// A build without the Antigravity accounts has no `antigravity` commands.
#[cfg(not(feature = "antigravity-accounts"))]
pub fn antigravity(
    home: &std::path::Path,
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let _ = (home, args, out);
    let _ = writeln!(
        err,
        "quota-station antigravity: this build has no Antigravity accounts \
         (build it with --features antigravity-accounts)"
    );
    1
}

/// The home folder the commands and the secret store use: on Linux the one
/// [`quota_station_core::linux::home`] gives (never a relative path), else
/// `HOME`.
fn home() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        quota_station_core::linux::home()
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from)
    }
}

/// The program's arguments as text; `None` when one is not valid UTF-8,
/// which the commands cannot carry.
pub fn text_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Vec<String>> {
    args.into_iter().map(|a| a.into_string().ok()).collect()
}
