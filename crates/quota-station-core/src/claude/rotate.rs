//! One run of the rotation, as one run of the old `cswap-prefer-team`
//! script, in the same order: the pause guard, the readings, the decision
//! ([`super::engine`]), the switch, then priming. The decisions and
//! switches go to `claude/rotation.log` in the script's log format, which
//! the history and the window read.
//!
//! Two switchers must never run at once: while `com.cswap.prefer-team` or
//! `com.cswap.auto` is loaded, a run switches nothing, primes nothing and
//! refreshes no token (the older switcher keeps its own copies of the same
//! logins), and says so as its decision.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use time::OffsetDateTime;

use super::engine::{self, Input, Plan};
use super::session;
use super::store::Store;
use super::usage::{self, Endpoints};
use super::{agent, switch};
use crate::local_time;
use crate::pause::{self, Flag};
use crate::rotation::{self, FileState, Rules};

/// The model a priming message goes to, as the script sent it.
pub const PRIME_MODEL: &str = "claude-haiku-5-5";
/// How long one priming may run.
pub const PRIME_TIMEOUT: Duration = Duration::from_secs(120);

/// The programs and files a run uses.
#[derive(Debug, Clone)]
pub struct Tools {
    pub launchctl: PathBuf,
    /// `claude`, for priming; none skips priming.
    pub claude: Option<PathBuf>,
    pub pause_file: PathBuf,
    pub endpoints: Endpoints,
}

impl Tools {
    pub fn system(home: &Path) -> Tools {
        Tools {
            launchctl: PathBuf::from(crate::switcher::LAUNCHCTL),
            claude: session::find_claude(home),
            pause_file: pause::path(home),
            endpoints: Endpoints::anthropic(),
        }
    }
}

/// The rules the rotation follows: the app's rotation file, the old one on a
/// Mac that has not imported yet, or the script's built-in values.
pub fn rules(store: &Store) -> Rules {
    match rotation::load(&store.paths.rotation_file()) {
        FileState::Ok { parsed, .. } => parsed.rules,
        _ => Rules::defaults(),
    }
}

/// The readings as the decision takes them.
pub fn readings(store: &Store, now: OffsetDateTime) -> Vec<engine::Reading> {
    let Ok(list) = store.load() else { return Vec::new() };
    let cache = usage::load(store);
    let at = super::epoch(now);
    let win = |w: Option<&usage::Win>| engine::Window {
        used: w.map(|w| w.pct),
        resets_at: w.and_then(|w| w.resets_at.as_deref()).and_then(crate::time_util::parse_ts),
    };
    list.accounts
        .iter()
        .map(|a| {
            let row = cache.row(a.slot).filter(|r| r.identity == a.identity);
            let reading = row.and_then(|r| r.last_good.as_ref());
            engine::Reading {
                number: a.slot,
                fresh: row.is_some_and(|r| r.fresh(at)),
                five_hour: win(reading.and_then(|r| r.five_hour.as_ref())),
                seven_day: win(reading.and_then(|r| r.seven_day.as_ref())),
                fable: win(reading.and_then(|r| r.fable()).map(|f| usage::Win { pct: f.pct, resets_at: f.resets_at.clone() }).as_ref()),
                disabled: a.disabled,
            }
        })
        .collect()
}

fn primed(store: &Store) -> Option<BTreeMap<u32, f64>> {
    if !store.paths.prime_flag().exists() {
        return None;
    }
    let mut last = BTreeMap::new();
    for entry in std::fs::read_dir(store.paths.prime_state()).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(slot) = name.strip_prefix("prime-").and_then(|n| n.parse::<u32>().ok()) else { continue };
        if let Some(t) = std::fs::read_to_string(entry.path()).ok().and_then(|t| t.trim().parse::<f64>().ok()) {
            last.insert(slot, t);
        }
    }
    Some(last)
}

/// The decision now from the readings on disk, with `rules`. Reads nothing
/// over the network.
pub fn plan(store: &Store, rules: &Rules, now: OffsetDateTime) -> Plan {
    let accounts = readings(store, now);
    let last = primed(store);
    let hour = local_time::stamp(now)[11..13].parse().unwrap_or(0);
    engine::decide(&Input {
        rules,
        accounts: &accounts,
        active: switch::active_slot(store),
        now,
        hour,
        prime: last.as_ref(),
    })
}

/// The pause flag as the script's guard reads it.
#[derive(Debug, Clone, PartialEq)]
pub enum Guard {
    None,
    /// The pause holds: the guard's line (`paused until …: reason`).
    Paused(String),
    /// The pause ended at `until`: a real run deletes the flag.
    Ended { until: String, text: String },
}

pub fn guard(pause_file: &Path, now: OffsetDateTime) -> Guard {
    let Ok(text) = std::fs::read_to_string(pause_file) else { return Guard::None };
    let flag = Flag::parse(&text);
    if !flag.holds(&local_time::stamp(now)) {
        return Guard::Ended { until: flag.until.unwrap_or_default(), text };
    }
    let mut line = match &flag.until {
        Some(until) => format!("paused until {until}"),
        None => "paused indefinitely".to_string(),
    };
    if let Some(reason) = &flag.reason {
        line.push_str(&format!(": {reason}"));
    }
    Guard::Paused(line)
}

/// What a run decided, as the script prints it with `--dry-run`.
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    /// The decision line.
    pub line: String,
    /// `would prime slot(s) 3`, when any.
    pub primes: Vec<u32>,
}

impl Decided {
    pub fn print(&self) -> String {
        let mut out = format!("{}\n", self.line);
        if !self.primes.is_empty() {
            let list: Vec<String> = self.primes.iter().map(u32::to_string).collect();
            out.push_str(&format!("would prime slot(s) {}\n", list.join(" ")));
        }
        out
    }
}

/// The line a run gives while an older switcher is loaded.
pub fn blocked_line(label: &str) -> String {
    format!("blocked ({label} is loaded)")
}

/// What the rotation would do now, reading nothing over the network: the
/// window's "why" line and the rules editor's preview (`rules` there is
/// the draft).
pub fn dry_run(store: &Store, tools: &Tools, rules: &Rules, now: OffsetDateTime) -> Decided {
    match guard(&tools.pause_file, now) {
        Guard::Paused(line) => return Decided { line, primes: Vec::new() },
        Guard::None | Guard::Ended { .. } => {}
    }
    if let Some(label) = agent::legacy_loaded(&tools.launchctl) {
        return Decided { line: blocked_line(label), primes: Vec::new() };
    }
    let plan = plan(store, rules, now);
    Decided { line: plan.decision, primes: plan.primes }
}

/// Appends `text` to the rotation's log, stamped with this Mac's time.
pub fn log(store: &Store, text: &str, now: OffsetDateTime) {
    let line = format!("{} {text}\n", local_time::stamp(now).replace('T', " "));
    if super::private_dir(&store.paths.dir).is_ok() {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(store.paths.log()) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// One real run, launchd's every three minutes.
pub fn run(store: &Store, tools: &Tools) -> Decided {
    let now = OffsetDateTime::now_utc();
    match guard(&tools.pause_file, now) {
        Guard::Paused(line) => return Decided { line, primes: Vec::new() },
        Guard::Ended { until, text } => {
            // Set the flag aside, and put it back if it is not the one just
            // read: a pause written in between is kept for the next run.
            let aside = tools.pause_file.with_file_name("pause.ended");
            if std::fs::rename(&tools.pause_file, &aside).is_err() {
                return Decided { line: "paused".into(), primes: Vec::new() };
            }
            if std::fs::read_to_string(&aside).ok().as_deref() != Some(text.as_str()) {
                if !tools.pause_file.exists() {
                    let _ = std::fs::rename(&aside, &tools.pause_file);
                }
                let _ = std::fs::remove_file(&aside);
                return Decided { line: "paused".into(), primes: Vec::new() };
            }
            let _ = std::fs::remove_file(&aside);
            log(store, &format!("pause ended (until {until})"), now);
        }
        Guard::None => {}
    }
    let legacy = agent::legacy_loaded(&tools.launchctl);
    usage::read_due(store, &tools.endpoints, false, legacy.is_none());
    let now = OffsetDateTime::now_utc();
    let _ = super::state::write(store, switch::active_slot(store), now);
    if let Some(label) = legacy {
        return Decided { line: blocked_line(label), primes: Vec::new() };
    }
    let plan = plan(store, &rules(store), now);
    if let Some(slot) = plan.target() {
        let rest = plan.decision.splitn(3, ' ').nth(2).unwrap_or("");
        match switch::switch(store, slot) {
            Ok(_) => log(store, &format!("switch to slot {slot} {rest}"), OffsetDateTime::now_utc()),
            Err(_) => log(store, &format!("switch to slot {slot} failed"), OffsetDateTime::now_utc()),
        }
        let _ = super::state::write(store, switch::active_slot(store), OffsetDateTime::now_utc());
    }
    if let Some(claude) = &tools.claude {
        for &slot in &plan.primes {
            prime(store, claude, slot);
        }
    }
    Decided { line: plan.decision.clone(), primes: plan.primes }
}

/// One tiny message as account `slot`, in its own session folder, so its
/// five-hour window starts now. Never as Claude Code's default login.
fn prime(store: &Store, claude: &Path, slot: u32) {
    let dir = store.paths.prime_state();
    if super::private_dir(&dir).is_err() {
        return;
    }
    let now = OffsetDateTime::now_utc();
    let _ = std::fs::write(dir.join(format!("prime-{slot}")), format!("{}\n", now.unix_timestamp()));
    let args: Vec<String> = [
        "-p",
        "ok",
        "--model",
        PRIME_MODEL,
        "--setting-sources",
        "project",
        "--strict-mcp-config",
        "--no-session-persistence",
    ]
    .map(String::from)
    .to_vec();
    let code = session::run_in(store, claude, slot, true, &args, Some(&dir), Some(PRIME_TIMEOUT))
        .map(|c| c.to_string())
        .unwrap_or_else(|_| "refused".to_string());
    log(store, &format!("primed slot {slot} (exit {code})"), OffsetDateTime::now_utc());
}
