// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! `quota-station claude …`: the accounts from the command line, for other
//! programs and the owner in a terminal. No command prints a token; only
//! `status --emails` prints emails.
// @private:begin
//! Firstmate and the rotation's LaunchAgent run it too.
// @private:end

use std::io::{Read, Write};
#[cfg(not(feature = "author-tools"))]
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use time::OffsetDateTime;

#[cfg(feature = "author-tools")]
use super::rotate;
#[cfg(feature = "author-tools")]
pub use super::rotate::Tools;
use super::store::Store;
#[cfg(not(feature = "author-tools"))]
use super::usage::Endpoints;
use super::{import, manage, session, state, switch, usage};

/// The programs the commands run.
#[cfg(not(feature = "author-tools"))]
#[derive(Debug, Clone)]
pub struct Tools {
    /// `claude`, to log a new account in with and to run as an account.
    pub claude: Option<PathBuf>,
    pub endpoints: Endpoints,
}

#[cfg(not(feature = "author-tools"))]
impl Tools {
    pub fn system(home: &Path) -> Tools {
        Tools { claude: session::find_claude(home), endpoints: Endpoints::anthropic() }
    }
}

#[cfg(not(feature = "author-tools"))]
pub const USAGE: &str = "\
usage: quota-station claude <command>

  status [--json] [--emails] [--refresh]
                        every account, its windows and the account in use
  switch N              make account N Claude Code's login
  run N [--require-session] [--] ARGS...
                        run `claude ARGS...` as account N, in its own folder
  session N             print account N's folder, for CLAUDE_CONFIG_DIR
  import                carry claude-swap's accounts over (claude-swap is left as it is)
  add                   log a new account in with `claude auth login`
  add --current         add the login Claude Code is on now
  add --token [--email E]
                        add a `claude setup-token` token, read from stdin
  remove N | alias N TEXT
";

#[cfg(feature = "author-tools")]
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
  import                carry claude-swap's accounts over (claude-swap is left as it is)
  add                   log a new account in with `claude auth login`
  add --current         add the login Claude Code is on now
  add --token [--email E]
                        add a `claude setup-token` token, read from stdin
  remove N | alias N TEXT | enable N | disable N
";

/// What the commands run with.
pub struct Cli {
    pub store: Store,
    pub tools: Tools,
    /// This program, for the LaunchAgent.
    #[cfg(feature = "author-tools")]
    pub exe: std::path::PathBuf,
}

fn slot(arg: Option<&String>) -> Option<u32> {
    arg.and_then(|a| a.parse().ok()).filter(|n| *n > 0)
}

/// Runs one command; the exit code.
pub fn run(cli: &Cli, args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let store = &cli.store;
    let word = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];
    let has = |flag: &str| rest.iter().any(|a| a == flag);
    let fail = |err: &mut dyn Write, e: &dyn std::fmt::Display| {
        let _ = writeln!(err, "quota-station claude {word}: {e}");
        1
    };
    match word {
        "status" => {
            if has("--refresh") {
                #[cfg(feature = "author-tools")]
                let may_refresh = cli.tools.scheduler.legacy_loaded().is_none();
                #[cfg(not(feature = "author-tools"))]
                let may_refresh = true;
                usage::read_due(store, &cli.tools.endpoints, false, may_refresh);
            }
            let now = OffsetDateTime::now_utc();
            let active = switch::active_slot(store);
            let _ = state::write(store, active, now);
            let list = match store.load() {
                Ok(l) => l,
                Err(e) => return fail(err, &e),
            };
            let mut view = state::build(&list, &usage::load(store), active, now);
            if has("--emails") {
                for a in &mut view.accounts {
                    a.email = store.oauth_account(a.slot).ok().flatten().and_then(|o| {
                        o.get("emailAddress").and_then(|e| e.as_str()).map(String::from)
                    });
                }
            }
            if has("--json") {
                let _ =
                    writeln!(out, "{}", serde_json::to_string_pretty(&view).expect("serializes"));
            } else {
                print_status(&view, out);
            }
            0
        }
        "switch" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err) };
            #[cfg(feature = "author-tools")]
            if let Some(label) = cli.tools.scheduler.legacy_loaded() {
                return fail(
                    err,
                    &format!("{label} is loaded; stop it first (see the README's cutover)"),
                );
            }
            match switch::switch(store, n) {
                Ok(done) => {
                    let _ = state::write(store, Some(n), OffsetDateTime::now_utc());
                    let what = if done.already { "already on" } else { "switched to" };
                    let _ = writeln!(out, "{what} account {n}");
                    0
                }
                Err(e) => fail(err, &format!("{} ({})", e, e.kind())),
            }
        }
        "run" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err) };
            let mut tail = &rest[1..];
            let require = tail.first().is_some_and(|a| a == "--require-session");
            if require {
                tail = &tail[1..];
            }
            if tail.first().is_some_and(|a| a == "--") {
                tail = &tail[1..];
            }
            let Some(claude) = &cli.tools.claude else { return fail(err, &"claude not found") };
            match session::run(store, claude, n, require, tail) {
                Ok(code) => code,
                Err(e) => fail(err, &e),
            }
        }
        "session" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err) };
            match session::prepare(store, n) {
                Ok(dir) => {
                    let _ = writeln!(out, "{}", dir.display());
                    0
                }
                Err(e) => fail(err, &e),
            }
        }
        #[cfg(feature = "author-tools")]
        "rotate" => {
            let decided = if has("--dry-run") {
                rotate::dry_run(
                    store,
                    &cli.tools,
                    rotate::rules(store).as_ref(),
                    OffsetDateTime::now_utc(),
                )
            } else {
                rotate::run(store, &cli.tools)
            };
            let _ = write!(out, "{}", decided.print());
            0
        }
        #[cfg(feature = "author-tools")]
        "rotation" => {
            let home = &store.paths.home;
            let done = match rest.first().map(String::as_str) {
                Some("on") => cli.tools.scheduler.install(home, &cli.exe),
                Some("off") => cli.tools.scheduler.remove(home),
                _ => return usage_error(err),
            };
            match done {
                Ok(()) => 0,
                Err(e) => fail(err, &e),
            }
        }
        "import" => match import::import(store, &import::source(&store.paths.home)) {
            Ok(done) => {
                let _ =
                    writeln!(out, "{}", serde_json::to_string_pretty(&done).expect("serializes"));
                let _ = state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
                0
            }
            Err(e) => fail(err, &e),
        },
        "add" => {
            let added = if has("--current") {
                manage::adopt_live(store)
            } else if has("--token") {
                let email = rest.iter().position(|a| a == "--email").and_then(|i| rest.get(i + 1));
                let mut token = String::new();
                if std::io::stdin().read_to_string(&mut token).is_err() {
                    return fail(err, &"could not read the token from stdin");
                }
                manage::add_token(store, &token, email.map(String::as_str))
            } else {
                let Some(claude) = &cli.tools.claude else { return fail(err, &"claude not found") };
                manage::login(store, claude, true, &AtomicBool::new(false))
            };
            match added {
                Ok(a) => {
                    let _ = writeln!(
                        out,
                        "{} account {}",
                        if a.new { "added" } else { "updated" },
                        a.slot
                    );
                    0
                }
                Err(e) => fail(err, &e),
            }
        }
        #[cfg(feature = "author-tools")]
        "remove" | "enable" | "disable" | "alias" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err) };
            let done = match word {
                "remove" => manage::remove(store, n),
                "enable" => manage::set_enabled(store, n, true),
                "disable" => manage::set_enabled(store, n, false),
                _ => manage::set_alias(store, n, &rest[1..].join(" ")),
            };
            match done {
                Ok(()) => {
                    let _ =
                        state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
                    0
                }
                Err(e) => fail(err, &e),
            }
        }
        #[cfg(not(feature = "author-tools"))]
        "remove" | "alias" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err) };
            let done = match word {
                "remove" => manage::remove(store, n),
                _ => manage::set_alias(store, n, &rest[1..].join(" ")),
            };
            match done {
                Ok(()) => {
                    let _ =
                        state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
                    0
                }
                Err(e) => fail(err, &e),
            }
        }
        "help" | "--help" | "-h" => {
            let _ = write!(out, "{USAGE}");
            0
        }
        _ => usage_error(err),
    }
}

fn usage_error(err: &mut dyn Write) -> i32 {
    let _ = write!(err, "{USAGE}");
    2
}

fn print_status(view: &state::StateOut, out: &mut dyn Write) {
    let pct = |w: &Option<state::WindowOut>| match w {
        Some(w) => format!("{:>3.0}%", w.used_percent),
        None => "   -".to_string(),
    };
    for a in &view.accounts {
        let name = a.alias.as_deref().unwrap_or("");
        #[cfg(feature = "author-tools")]
        let name = if a.enabled { name.to_string() } else { format!("held out  {name}") };
        let _ = writeln!(
            out,
            "{} {:>2}  5h {}  7d {}  {:<8} {}{}",
            if a.active { '*' } else { ' ' },
            a.slot,
            pct(&a.windows.five_hour),
            pct(&a.windows.seven_day),
            a.status,
            name,
            a.email.as_deref().map(|e| format!("  {e}")).unwrap_or_default(),
        );
    }
    if view.accounts.is_empty() {
        let _ = writeln!(out, "no accounts yet: quota-station claude import, or add");
    }
}
