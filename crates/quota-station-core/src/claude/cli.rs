// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! `quota-station claude …`: the accounts from the command line, for other
//! programs and the owner in a terminal. No command prints a token; only
//! `status --emails` prints emails. An extension may add commands of its own
//! ([`Hooks::claude_command`]) and the usage that names them
//! ([`Hooks::claude_usage`]).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde_json::Map;
use time::OffsetDateTime;

use super::store::{AccountKeys, Store};
use super::usage::Endpoints;
use super::{import, manage, session, state, switch, usage};
use crate::ext::Hooks;

/// The programs the commands run.
#[derive(Debug, Clone)]
pub struct Tools {
    /// `claude`, to log a new account in with and to run as an account.
    pub claude: Option<PathBuf>,
    pub endpoints: Endpoints,
}

impl Tools {
    pub fn system(home: &Path) -> Tools {
        Tools { claude: session::find_claude(home), endpoints: Endpoints::anthropic() }
    }
}

/// The usage of the app's own commands.
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

/// What the commands run with.
pub struct Cli {
    pub store: Store,
    pub tools: Tools,
}

fn slot(arg: Option<&String>) -> Option<u32> {
    arg.and_then(|a| a.parse().ok()).filter(|n| *n > 0)
}

/// Runs one command; the exit code. A command that is not the app's goes to
/// `hooks` ([`Hooks::claude_command`]); `status --refresh` refreshes a token
/// only while they let it ([`Hooks::may_refresh_logins`]), and `switch`
/// switches only when what they do around it refuses nothing
/// ([`Hooks::manual_switch`]).
pub fn run(
    cli: &Cli,
    hooks: &dyn Hooks,
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
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
                usage::read_due(store, &cli.tools.endpoints, false, hooks.may_refresh_logins());
            }
            let now = OffsetDateTime::now_utc();
            let active = switch::active_slot(store);
            let _ = state::write(store, active, now);
            let list = match store.load() {
                Ok(l) => l,
                Err(e) => return fail(err, &e),
            };
            let keys = store.account_keys();
            let mut view = state::build(&list, &usage::load(store), active, keys, now);
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
                print_status(&view, keys, out);
            }
            0
        }
        "switch" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err, hooks) };
            match hooks.manual_switch(&Map::new()) {
                Ok(Some(mut around)) => {
                    if let Some(refusal) = around.before(store) {
                        return fail(err, &refusal.message);
                    }
                }
                Ok(None) => {}
                Err(e) => return fail(err, &e),
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
            let Some(n) = slot(rest.first()) else { return usage_error(err, hooks) };
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
            let Some(n) = slot(rest.first()) else { return usage_error(err, hooks) };
            match session::prepare(store, n) {
                Ok(dir) => {
                    let _ = writeln!(out, "{}", dir.display());
                    0
                }
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
        "remove" | "alias" => {
            let Some(n) = slot(rest.first()) else { return usage_error(err, hooks) };
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
            let _ = write!(out, "{}", usage(hooks));
            0
        }
        _ => hooks.claude_command(cli, args, out, err).unwrap_or_else(|| usage_error(err, hooks)),
    }
}

/// The usage: the extension's when it gives one, else the app's.
fn usage(hooks: &dyn Hooks) -> &'static str {
    hooks.claude_usage().unwrap_or(USAGE)
}

fn usage_error(err: &mut dyn Write, hooks: &dyn Hooks) -> i32 {
    let _ = write!(err, "{}", usage(hooks));
    2
}

/// The accounts' lines, each name after the words `keys` put before it.
fn print_status(view: &state::StateOut, keys: Option<&dyn AccountKeys>, out: &mut dyn Write) {
    let pct = |w: &Option<state::WindowOut>| match w {
        Some(w) => format!("{:>3.0}%", w.used_percent),
        None => "   -".to_string(),
    };
    for a in &view.accounts {
        let name = a.alias.as_deref().unwrap_or("");
        let name = match keys.and_then(|k| k.status_words(&a.ext)) {
            Some(words) => format!("{words}  {name}"),
            None => name.to_string(),
        };
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
