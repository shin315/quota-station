// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's commands of `quota-station claude …` ([`CliHooks`],
//! through [`crate::claude::cli::run`]): one run of the rotation
//! (`rotate [--dry-run]`, what its LaunchAgent runs), the LaunchAgent
//! loaded or unloaded (`rotation on|off`), and an account put back in the
//! rotation or held out of it (`enable N`, `disable N`); with the usage that
//! names them.

use std::io::Write;
use std::path::PathBuf;

use serde_json::{Map, Value};
use time::OffsetDateTime;

use crate::advanced::switch::SwitchHooks;
use crate::claude::cli::Cli;
use crate::claude::{rotate, state, switch};
use crate::ext::Hooks;
use crate::pause;
use crate::switching::AroundSwitch;

/// The usage of `quota-station claude` with the advanced part's commands.
pub const USAGE: &str = "\
usage: quota-station claude <command>

  status [--json] [--emails] [--refresh]
                        every account, its windows and the account in use
  switch N              make account N Claude Code's login
  run N [--require-session] [--] ARGS...
                        run `claude ARGS...` as account N, in its own folder
  session N             print account N's folder, for CLAUDE_CONFIG_DIR
  rotate [--dry-run]    one run of the rotation (the LaunchAgent's)
  rotation on|off       load or unload the rotation's LaunchAgent
  import                carry the other account switcher's accounts over (it is left as it is)
  add                   log a new account in with `claude auth login`
  add --current         add the login Claude Code is on now
  add --token [--email E]
                        add a `claude setup-token` token, read from stdin
  remove N | alias N TEXT | enable N | disable N
";

/// The advanced part's hooks for `quota-station claude …`: the app's
/// extension gives the core these, with this
/// program as `exe` and this Mac's launchd and pause flag as `switch`.
#[derive(Debug, Clone)]
pub struct CliHooks {
    /// This program, for the LaunchAgent.
    pub exe: PathBuf,
    /// The rotation's job and the older switchers, and the pause flag: what
    /// the rotation's commands run with, and what refuses a switch and a
    /// token's refresh while an older switcher is loaded.
    pub switch: SwitchHooks,
}

impl CliHooks {
    /// The hooks of the program running now.
    pub fn this_program() -> CliHooks {
        CliHooks {
            exe: std::env::current_exe().unwrap_or_else(|_| "quota-station".into()),
            switch: SwitchHooks::system(),
        }
    }

    /// What a run of the rotation uses: `claude` and the endpoints the
    /// command line has, with the job and the pause flag of these hooks.
    fn rotation_tools(&self, cli: &Cli) -> rotate::Tools {
        rotate::Tools {
            scheduler: self.switch.scheduler.clone(),
            claude: cli.tools.claude.clone(),
            pause_file: (self.switch.pause_file.clone())
                .unwrap_or_else(|| pause::file(&cli.store.paths.home)),
            endpoints: cli.tools.endpoints.clone(),
        }
    }
}

impl Hooks for CliHooks {
    fn claude_command(
        &self,
        cli: &Cli,
        args: &[String],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Option<i32> {
        let store = &cli.store;
        let word = args.first()?.as_str();
        let rest = &args[1..];
        let has = |flag: &str| rest.iter().any(|a| a == flag);
        let fail = |err: &mut dyn Write, e: &dyn std::fmt::Display| {
            let _ = writeln!(err, "quota-station claude {word}: {e}");
            1
        };
        let code = match word {
            "rotate" => {
                let tools = self.rotation_tools(cli);
                let decided = if has("--dry-run") {
                    rotate::dry_run(
                        store,
                        &tools,
                        &rotate::policy(store),
                        OffsetDateTime::now_utc(),
                    )
                } else {
                    rotate::run(store, &tools)
                };
                let _ = write!(out, "{}", decided.print());
                0
            }
            "rotation" => {
                let home = &store.paths.home;
                let done = match rest.first().map(String::as_str) {
                    Some("on") => self.switch.scheduler.install(home, &self.exe),
                    Some("off") => self.switch.scheduler.remove(home),
                    _ => return Some(usage_error(err)),
                };
                match done {
                    Ok(()) => 0,
                    Err(e) => fail(err, &e),
                }
            }
            "enable" | "disable" => {
                let Some(n) = slot(rest.first()) else { return Some(usage_error(err)) };
                match super::held::set_enabled(store, n, word == "enable") {
                    Ok(()) => {
                        let now = OffsetDateTime::now_utc();
                        let _ = state::write(store, switch::active_slot(store), now);
                        0
                    }
                    Err(e) => fail(err, &e),
                }
            }
            _ => return None,
        };
        Some(code)
    }

    fn claude_usage(&self) -> Option<&'static str> {
        Some(USAGE)
    }

    fn manual_switch(
        &self,
        asked: &Map<String, Value>,
    ) -> Result<Option<Box<dyn AroundSwitch>>, String> {
        self.switch.manual_switch(asked)
    }

    fn may_refresh_logins(&self) -> bool {
        self.switch.may_refresh_logins()
    }
}

/// The account number an argument names, as the app's commands read it.
fn slot(arg: Option<&String>) -> Option<u32> {
    arg.and_then(|a| a.parse().ok()).filter(|n| *n > 0)
}

fn usage_error(err: &mut dyn Write) -> i32 {
    let _ = write!(err, "{USAGE}");
    2
}
