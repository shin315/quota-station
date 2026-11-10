// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! `quota-station claude …` without the app, which needs a desktop and is
//! built for macOS and Windows only: the same commands, with the advanced
//! part's (`rotate`, `rotation on|off`, `enable N`, `disable N`), the same
//! files and the same JSON, on this system's seams
//! ([`quota_station_core::os`]). On Linux the logins are Claude Code's
//! `~/.claude/.credentials.json` and the app's files of mode 600, and the
//! rotation runs from a `systemd --user` timer.

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

/// What a word other than `claude` gets.
pub const USAGE: &str = "\
usage: quota-station claude <command>

  `quota-station claude help` lists the commands.
";

/// Runs `args` (the words after the program's name); the exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match args.first().map(String::as_str) {
        Some("claude") => {
            let Some(home) = home() else {
                let _ = writeln!(err, "quota-station claude: HOME is not set");
                return 1;
            };
            claude(&home, &args[1..], &HOOKS, out, err)
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

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from)
}
