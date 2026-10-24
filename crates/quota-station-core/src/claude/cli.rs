//! `quota-station claude …`: the accounts from the command line, for
//! Firstmate, the rotation's LaunchAgent and the owner in a terminal. No
//! command prints a token; only `status --emails` prints emails.

use std::io::{Read, Write};
use std::sync::atomic::AtomicBool;

use time::OffsetDateTime;

use super::rotate::{self, Tools};
use super::store::Store;
use super::{agent, import, manage, session, state, switch, usage};

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
                let legacy = agent::legacy_loaded(&cli.tools.launchctl);
                usage::read_due(store, &cli.tools.endpoints, false, legacy.is_none());
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
                    a.email = store
                        .oauth_account(a.slot)
                        .ok()
                        .flatten()
                        .and_then(|o| o.get("emailAddress").and_then(|e| e.as_str()).map(String::from));
                }
            }
            if has("--json") {
                let _ = writeln!(out, "{}", serde_json::to_string_pretty(&view).expect("serializes"));
            } else {
                print_status(&view, out);
            }
            0
        }
        "switch" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err) };
            if let Some(label) = agent::legacy_loaded(&cli.tools.launchctl) {
                return fail(err, &format!("{label} is loaded; stop it first (see the README's cutover)"));
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
        "rotate" => {
            let decided = if has("--dry-run") {
                rotate::dry_run(store, &cli.tools, &rotate::rules(store), OffsetDateTime::now_utc())
            } else {
                rotate::run(store, &cli.tools)
            };
            let _ = write!(out, "{}", decided.print());
            0
        }
        "rotation" => {
            let home = &store.paths.home;
            let done = match rest.first().map(String::as_str) {
                Some("on") => agent::install(&cli.tools.launchctl, home, &cli.exe),
                Some("off") => agent::remove(&cli.tools.launchctl, home),
                _ => return usage_error(err),
            };
            match done {
                Ok(()) => 0,
                Err(e) => fail(err, &e),
            }
        }
        "import" => match import::import(store, &import::source(&store.paths.home)) {
            Ok(done) => {
                let _ = writeln!(out, "{}", serde_json::to_string_pretty(&done).expect("serializes"));
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
                    let _ = writeln!(out, "{} account {}", if a.new { "added" } else { "updated" }, a.slot);
                    0
                }
                Err(e) => fail(err, &e),
            }
        }
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
                    let _ = state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
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
        let _ = writeln!(
            out,
            "{} {:>2}  5h {}  7d {}  {:<8} {}{}{}",
            if a.active { '*' } else { ' ' },
            a.slot,
            pct(&a.windows.five_hour),
            pct(&a.windows.seven_day),
            a.status,
            if a.enabled { "" } else { "held out  " },
            a.alias.as_deref().unwrap_or(""),
            a.email.as_deref().map(|e| format!("  {e}")).unwrap_or_default(),
        );
    }
    if view.accounts.is_empty() {
        let _ = writeln!(out, "no accounts yet: quota-station claude import, or add");
    }
}
