// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Kun Chen (quota-axi)
// SPDX-License-Identifier: MIT

// Ported from quota-axi's `src/providers/agy.ts` (MIT,
// https://github.com/kunchenguid/quota-axi, 0.1.66): `agy -p /quota
// --output-format json` first, with `open` and `xdg-open` that only fail on
// its PATH so it can open no browser; then the language server of a running
// agy or Antigravity.app, found from the process list (`ps`) and its
// listening ports (`lsof`), asked over loopback for its quota summary (and
// the account's status) with the CSRF token from its command line; the
// group and bucket naming (`gemini_5h`, `claude_gpt_weekly`, Gemini Argon
// apart), percent left from the remaining fraction, the time limits, how
// a failure is weighed and when the last reading may stand in as stale.
// Changes: the account's email is never kept, only a hash of it to tell
// which of the app's accounts a reading is; the plan is not read.

//! The Antigravity account signed in on this Mac: its Gemini and Claude/GPT
//! windows. Nothing here reads a credential: `agy` and the running app
//! answer with their own login, and the CSRF token read from the process
//! list goes no further than the request to that process on 127.0.0.1.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use time::OffsetDateTime;

use super::cache::{fingerprint, Cache};
use super::{epoch_or_iso, number, slugify, text, Provider, ProviderId, Response, Window};
use crate::os::{Process, ProcessList};
use crate::runner::{self, RunError};

const QUOTA_SUMMARY: &str =
    "/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary";
const USER_STATUS: &str = "/exa.language_server_pb.LanguageServerService/GetUserStatus";
const MODEL_CONFIGS: &str = "/exa.language_server_pb.LanguageServerService/GetCommandModelConfigs";
const UNLEASH: &str = "/exa.language_server_pb.LanguageServerService/GetUnleashData";
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);
const PORT_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const CLI_TIMEOUT: Duration = Duration::from_secs(15);
const PROBE_BUDGET: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const FIVE_HOURS: f64 = 18_000.0;
const WEEK: f64 = 604_800.0;

pub const NOT_INSTALLED: &str = "agy CLI is not installed";
pub const NOT_RUNNING: &str = "Antigravity/agy is not running";
pub const DISCOVERY_UNAVAILABLE: &str = "Antigravity process discovery unavailable";
pub const NO_ENDPOINT: &str = "Antigravity/agy has no accessible endpoint";
const CSRF: &str = "Antigravity CLI quota unavailable because its runtime CSRF token is not exposed; use Antigravity /quota";

/// The programs a read runs. The app uses [`Tools::system`].
#[derive(Debug, Clone)]
pub struct Tools {
    /// `agy`, when it is on this Mac.
    pub agy: Option<PathBuf>,
    /// The running processes and their ports: `ps` and `lsof` on macOS.
    pub processes: Arc<dyn ProcessList>,
}

impl Tools {
    /// The system's process list, and `agy` where quota-axi finds it: on
    /// the `PATH` (an app opened from Finder has launchd's, which leaves out
    /// `~/.local/bin`; a read from a shell has the shell's), then in
    /// Homebrew's folders; on Windows `agy.exe`, then in the folder its
    /// installer uses, `%LOCALAPPDATA%\agy\bin`. Windows lists no process
    /// of this user's ([`crate::win::Tasks`]), so a read there is `agy`'s
    /// alone, as quota-axi's is.
    pub fn system() -> Tools {
        let installed = || {
            let local = std::env::var_os("LOCALAPPDATA").filter(|_| cfg!(windows))?;
            runner::program_in(&PathBuf::from(local).join("agy").join("bin"), "agy")
        };
        Tools {
            agy: runner::on_path("agy").or_else(installed),
            processes: Arc::new(crate::os::processes()),
        }
    }
}

/// Why a read failed, weighed as quota-axi weighs it.
#[derive(Debug, Clone, PartialEq)]
enum Fail {
    /// Not there, not reachable, timed out.
    Unavailable(String),
    /// The language server wants a CSRF token its command line does not show.
    Csrf,
    Malformed(String),
    Discovery(String),
    Http(u16),
    Other(String),
}

impl Fail {
    fn message(&self) -> String {
        match self {
            Fail::Unavailable(m) | Fail::Malformed(m) | Fail::Discovery(m) | Fail::Other(m) => {
                m.clone()
            }
            Fail::Csrf => CSRF.into(),
            Fail::Http(401 | 403) => "Antigravity sign-in required".into(),
            Fail::Http(429) => "Antigravity quota endpoint rate limited".into(),
            Fail::Http(s) => format!("Antigravity quota endpoint returned HTTP {s}"),
        }
    }

    fn status(&self) -> &'static str {
        match self {
            Fail::Unavailable(_) | Fail::Csrf => "unavailable",
            other => status_for_error(&other.message()),
        }
    }

    fn rank(&self) -> u8 {
        if *self == Fail::Csrf {
            return 2;
        }
        match self.status() {
            "auth_required" => 4,
            "rate_limited" => 3,
            "error" => 2,
            "unavailable" => 1,
            _ => 0,
        }
    }

    fn stale_eligible(&self) -> bool {
        match self {
            Fail::Csrf => false,
            Fail::Http(s) => *s == 429 || *s >= 500,
            other => other.status() == "unavailable",
        }
    }

    fn definitive_auth(&self) -> bool {
        *self != Fail::Csrf && status_for_error(&self.message()) == "auth_required"
    }
}

fn stronger(current: Option<Fail>, candidate: Fail) -> Fail {
    match current {
        Some(c) if c.rank() >= candidate.rank() => c,
        _ => candidate,
    }
}

/// quota-axi's status for an Antigravity failure's words (`statusForError`):
/// a process or endpoint not there is `unavailable`.
pub fn status_for_error(error: &str) -> &'static str {
    let e = error.to_lowercase();
    let unavailable = [
        "not running",
        "no local",
        "loopback unavailable",
        "loopback access unavailable",
        "loopback timed out",
        "probe timed out",
        "econnrefused",
        "econnreset",
        "econnaborted",
        "enotfound",
        "ehostunreach",
        "etimedout",
        "epipe",
        "eproto",
        "socket hang up",
    ];
    if unavailable.iter().any(|w| e.contains(w)) {
        return "unavailable";
    }
    super::status_for_error(error)
}

/// One reading, before it becomes a row.
struct Reading {
    windows: Vec<Window>,
    /// A hash of the account's email, when the language server named it.
    account: Option<String>,
}

/// Reads the Antigravity account signed in. A reading younger than
/// `max_age` seconds answers without asking; a failed read shows the last
/// one as stale while it can stand.
pub fn read(tools: &Tools, cache: &Cache, max_age: i64, now: OffsetDateTime) -> Response {
    if let Some(reused) = cache.reusable(None, max_age, now) {
        return Response::new(now, vec![reused]);
    }
    let cli = cli(tools);
    let cli = match cli {
        Ok(reading) => return success(reading, cache, now),
        Err(e) => e,
    };
    let cli_skipped = matches!(&cli, Fail::Unavailable(m) if m == NOT_INSTALLED);
    let (final_failure, loop_skipped) = match loopback(tools) {
        Ok(reading) => return success(reading, cache, now),
        Err((e, skipped)) => {
            let f = if cli_skipped { e } else { stronger(Some(cli.clone()), e) };
            (f, skipped)
        }
    };
    let error = final_failure.message();
    let gone = cli_skipped && loop_skipped.as_deref() == Some(NOT_RUNNING);
    if gone {
        cache.retire();
    } else if final_failure.stale_eligible() {
        if let Some(stale) = cache.stale(&error, now) {
            return Response::new(now, vec![stale]);
        }
    } else if final_failure.definitive_auth() {
        cache.retire();
    }
    let mut row = Provider::failed(ProviderId::Agy, final_failure.status(), &error);
    // Neither agy nor a running Antigravity: nothing of it is on this Mac.
    row.not_set_up =
        cli_skipped && matches!(loop_skipped.as_deref(), Some(NOT_RUNNING | DISCOVERY_UNAVAILABLE));
    Response::new(now, vec![row])
}

fn success(reading: Reading, cache: &Cache, now: OffsetDateTime) -> Response {
    let mut row = Provider::fresh(ProviderId::Agy, reading.windows, now);
    row.account_keys = reading.account.into_iter().collect();
    cache.save(&row, None);
    Response::new(now, vec![row])
}

/// The hash a reading names its account by: of the email, trimmed and in
/// lower case, so the app's own list can be matched against it.
pub fn account_hash(email: &str) -> String {
    format!("email:{}", fingerprint(&email.trim().to_lowercase()))
}

/// A folder whose `open` and `xdg-open` only fail, first on agy's PATH, so
/// `/quota` can never send the owner to a browser. On Windows, as quota-axi
/// does there, they are `open.cmd` and `xdg-open.cmd`, and agy runs with
/// `NoDefaultCurrentDirectoryInExePath`, so no program in its working
/// folder is found before them; Windows also opens a browser by other
/// means, which no such folder stops.
struct OpenerGuard(Option<PathBuf>);

impl OpenerGuard {
    #[cfg(not(unix))]
    fn new() -> std::io::Result<OpenerGuard> {
        let dir = std::env::temp_dir().join(format!(
            "quota-station-agy-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir(&dir)?;
        for name in ["open.cmd", "xdg-open.cmd"] {
            std::fs::write(dir.join(name), "@exit /b 1\r\n")?;
        }
        Ok(OpenerGuard(Some(dir)))
    }

    #[cfg(unix)]
    fn new() -> std::io::Result<OpenerGuard> {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "quota-station-agy-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir(&dir)?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        for name in ["open", "xdg-open"] {
            let file = dir.join(name);
            std::fs::write(&file, "#!/bin/sh\nexit 1\n")?;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700))?;
        }
        Ok(OpenerGuard(Some(dir)))
    }
}

impl Drop for OpenerGuard {
    fn drop(&mut self) {
        if let Some(dir) = &self.0 {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn cli(tools: &Tools) -> Result<Reading, Fail> {
    let Some(agy) = tools.agy.as_ref().filter(|p| p.is_file()) else {
        return Err(Fail::Unavailable(NOT_INSTALLED.into()));
    };
    let guard =
        OpenerGuard::new().map_err(|_| Fail::Other("Antigravity CLI /quota failed".into()))?;
    let inherited = runner::child_path(agy, std::env::var_os("PATH"));
    let dirs = guard.0.iter().cloned().chain(std::env::split_paths(&inherited));
    let path = std::env::join_paths(dirs).unwrap_or(inherited);
    let args: Vec<String> =
        ["-p", "/quota", "--output-format", "json"].iter().map(|s| s.to_string()).collect();
    let mut env = vec![("PATH", path.as_os_str())];
    if cfg!(windows) {
        env.push(("NoDefaultCurrentDirectoryInExePath", std::ffi::OsStr::new("1")));
    }
    let out = runner::run_with(agy, &args, &env, None, CLI_TIMEOUT).map_err(|e| match e {
        RunError::Timeout(_) => Fail::Unavailable("Antigravity CLI /quota timed out".into()),
        RunError::NotFound(_) => Fail::Unavailable(NOT_INSTALLED.into()),
        _ => Fail::Other("Antigravity CLI /quota failed".into()),
    })?;
    drop(guard);
    if out.code != Some(0) {
        return Err(Fail::Other("Antigravity CLI /quota failed".into()));
    }
    if out.stdout.len() > 16 * 1024 * 1024 {
        return Err(Fail::Malformed("Antigravity CLI /quota response too large".into()));
    }
    let parsed: Value = serde_json::from_slice(&out.stdout)
        .map_err(|_| Fail::Malformed("agy /quota returned invalid JSON".into()))?;
    let windows = print_usage(&parsed)
        .filter(|w| !w.is_empty())
        .ok_or_else(|| Fail::Malformed("agy /quota quota summary malformed".into()))?;
    Ok(Reading { windows, account: None })
}

/// `agy -p /quota --output-format json`'s answer: the quota summary under
/// `command.data`.
pub fn print_usage(raw: &Value) -> Option<Vec<Window>> {
    let command = raw.get("command").filter(|c| c.is_object())?;
    let name = command.get("name").and_then(Value::as_str)?;
    if !["usage", "/usage", "quota", "/quota"].contains(&name) {
        return None;
    }
    Some(quota_summary(command.get("data")?))
}

/// The quota summary's windows, Gemini first, then Claude/GPT, each
/// five-hour before weekly.
pub fn quota_summary(raw: &Value) -> Vec<Window> {
    let payload = raw
        .get("response")
        .filter(|v| v.is_object())
        .or_else(|| raw.get("summary").filter(|v| v.is_object()))
        .or_else(|| raw.get("groups").filter(|g| g.is_array()).map(|_| raw));
    let Some(groups) = payload.and_then(|p| p.get("groups")).and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut windows: Vec<Window> = groups.iter().flat_map(group_windows).collect();
    windows.sort_by_key(|w| (group_rank(&w.id), kind_rank(w)));
    windows
}

fn group_rank(id: &str) -> u8 {
    if id.starts_with("gemini_") && !id.starts_with("gemini_argon_") {
        0
    } else if id.starts_with("claude_gpt_") {
        1
    } else {
        2
    }
}

fn kind_rank(w: &Window) -> u8 {
    match w.kind.as_deref() {
        Some("session") => 0,
        Some("weekly") => 1,
        _ => 2,
    }
}

fn group_windows(raw: &Value) -> Vec<Window> {
    if !raw.is_object() {
        return Vec::new();
    }
    let name = text(raw.get("displayName"))
        .or_else(|| text(raw.get("name")))
        .unwrap_or_else(|| "Quota".into());
    raw.get("buckets")
        .and_then(Value::as_array)
        .map(|b| b.iter().filter_map(|bucket| bucket_window(&name, bucket)).collect())
        .unwrap_or_default()
}

/// `(id, kind, seconds)` of a bucket's window: five hours, a week or other.
fn bucket_kind(bucket: &Value) -> (&'static str, &'static str, Option<f64>) {
    let values: Vec<String> = ["window", "bucketId", "bucket_id", "displayName", "name"]
        .iter()
        .filter_map(|k| text(bucket.get(*k)))
        .map(|v| v.to_lowercase())
        .collect();
    let is = |cores: &[&str]| {
        values.iter().any(|v| {
            let unlimited = v.strip_suffix(" limit").unwrap_or(v);
            [v.as_str(), unlimited].iter().any(|v| {
                let bare = ["gemini", "3p", "third-party"].iter().find_map(|p| {
                    v.strip_prefix(p)
                        .and_then(|r| r.strip_prefix('-').or_else(|| r.strip_prefix('_')))
                });
                cores.iter().any(|c| *v == *c || bare == Some(*c))
            })
        })
    };
    if is(&["5h", "5-hour", "five hour", "five_hour", "five-hour"]) {
        ("5h", "session", Some(FIVE_HOURS))
    } else if is(&["weekly"]) {
        ("weekly", "weekly", Some(WEEK))
    } else {
        ("unknown", "unknown", None)
    }
}

/// The group a bucket counts in: Gemini Argon on its own, other Gemini,
/// Claude/GPT, or a group of its own name.
fn group_id(group: &str, bucket: &str) -> String {
    let n = format!("{group} {bucket}").to_lowercase();
    if n.contains("gemini") && n.contains("argon") {
        "gemini_argon".into()
    } else if n.contains("gemini") {
        "gemini".into()
    } else if n.contains("claude") || n.contains("gpt") || n.contains("3p") {
        "claude_gpt".into()
    } else {
        let id = slugify(group);
        if id.is_empty() {
            "quota".into()
        } else {
            id
        }
    }
}

fn remaining_fraction(v: &Value) -> Option<f64> {
    number(v.get("remainingFraction")).or_else(|| number(v.get("remaining_fraction"))).or_else(
        || {
            let r = v.get("remaining")?;
            number(Some(r))
                .or_else(|| number(r.get("remainingFraction")))
                .or_else(|| number(r.get("remaining_fraction")))
                .or_else(|| {
                    let case = r.get("case").and_then(Value::as_str);
                    matches!(case, Some("remainingFraction" | "remaining_fraction"))
                        .then(|| number(r.get("value")))
                        .flatten()
                })
        },
    )
}

fn bucket_window(group: &str, bucket: &Value) -> Option<Window> {
    if !bucket.is_object() || bucket.get("disabled").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let id = text(bucket.get("bucketId"))
        .or_else(|| text(bucket.get("bucket_id")))
        .or_else(|| text(bucket.get("id")))?;
    let (kind_id, kind, seconds) = bucket_kind(bucket);
    let resets_at = bucket
        .get("resetTime")
        .and_then(epoch_or_iso)
        .or_else(|| bucket.get("reset_time").and_then(epoch_or_iso));
    let described = text(bucket.get("description")).is_some();
    let window = match remaining_fraction(bucket) {
        Some(left) => {
            let used = (1.0 - if left.is_finite() { left.clamp(0.0, 1.0) } else { 0.0 }) * 100.0;
            Window::used(&format!("{}_{kind_id}", group_id(group, &id)), kind, used, resets_at)
        }
        None if resets_at.is_some() || described => Window {
            id: format!("{}_{kind_id}", group_id(group, &id)),
            kind: Some(kind.into()),
            resets_at,
            ..Window::default()
        },
        None => return None,
    };
    Some(Window { window_seconds: seconds, ..window })
}

/// The account's status: its email (hashed here), and its models' quota
/// as windows when there is no summary.
fn user_status(raw: &Value) -> Option<Reading> {
    let status = raw.get("userStatus").filter(|v| v.is_object()).unwrap_or(raw);
    if !status.is_object() {
        return None;
    }
    let email = text(status.get("email"));
    let configs = status
        .get("cascadeModelConfigData")
        .filter(|v| v.is_object())
        .or_else(|| raw.get("cascadeModelConfigData").filter(|v| v.is_object()))
        .unwrap_or(raw);
    let windows: Vec<Window> = configs
        .get("clientModelConfigs")
        .and_then(Value::as_array)
        .map(|c| c.iter().filter_map(model_window).collect())
        .unwrap_or_default();
    let plan = status
        .pointer("/userTier/name")
        .or_else(|| status.pointer("/planStatus/planInfo/planName"));
    if windows.is_empty() && email.is_none() && text(plan).is_none() {
        return None;
    }
    Some(Reading { windows, account: email.as_deref().map(account_hash) })
}

fn model_window(config: &Value) -> Option<Window> {
    let label = text(config.get("label"));
    let info = config.get("quotaInfo").filter(|v| v.is_object())?;
    let model = config
        .pointer("/modelOrAlias/model")
        .and_then(Value::as_str)
        .or_else(|| config.pointer("/modelOrAlias/alias").and_then(Value::as_str))
        .map(String::from)
        .filter(|m| !m.is_empty())
        .or_else(|| label.as_deref().map(slugify))?;
    label.as_ref()?;
    let id = format!("model:{}", slugify(&model));
    let resets_at = info
        .get("resetTime")
        .and_then(epoch_or_iso)
        .or_else(|| info.get("reset_time").and_then(epoch_or_iso));
    match remaining_fraction(info) {
        Some(left) => {
            let used = (1.0 - if left.is_finite() { left.clamp(0.0, 1.0) } else { 0.0 }) * 100.0;
            Some(Window::used(&id, "model", used, resets_at))
        }
        None if resets_at.is_some() => {
            Some(Window { id, kind: Some("model".into()), resets_at, ..Window::default() })
        }
        None => None,
    }
}

// ===== The running language server, over loopback =====

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    Agy,
    App,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessInfo {
    pid: u32,
    source: Source,
    csrf: Option<String>,
    extension_port: Option<u16>,
    extension_csrf: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Endpoint {
    https: bool,
    port: u16,
    source: Source,
    csrf: Option<String>,
    unleash_first: bool,
}

/// Words of a command line: quoted runs whole, else split on spaces.
fn tokens(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = command.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let mut word = String::new();
        if c == '"' || c == '\'' {
            chars.next();
            for d in chars.by_ref() {
                if d == c {
                    break;
                }
                word.push(d);
            }
        } else {
            while let Some(&d) = chars.peek() {
                if d.is_whitespace() {
                    break;
                }
                word.push(d);
                chars.next();
            }
        }
        out.push(word);
    }
    out
}

fn executable_name(token: &str) -> String {
    let name = token.replace('\\', "/").rsplit('/').next().unwrap_or("").to_lowercase();
    name.strip_suffix(".exe").map(String::from).unwrap_or(name)
}

/// `language_server`, `language-server_macos_arm`.
fn language_server_name(name: &str) -> bool {
    let name = name.strip_suffix(".exe").unwrap_or(name);
    let Some(rest) = name.strip_prefix("language") else { return false };
    let Some(rest) = rest.strip_prefix('-').or_else(|| rest.strip_prefix('_')) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("server") else { return false };
    rest.is_empty()
        || rest.strip_prefix('_').is_some_and(|r| {
            !r.is_empty()
                && r.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

fn app_executable(command: &str) -> bool {
    let normalized = command.replace('\\', "/").to_lowercase();
    let path = match normalized.find(" --") {
        Some(at) => normalized[..at].trim_end(),
        None => normalized.as_str(),
    };
    if let Some(rest) = path.strip_prefix("/applications/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if let Some(app) =
            parts.iter().position(|p| p.ends_with(".app") && p.contains("antigravity"))
        {
            let inside = &parts[app + 1..];
            if inside.first() == Some(&"contents")
                && inside.len() >= 3
                && inside.last().is_some_and(|l| language_server_name(l))
            {
                return true;
            }
        }
    }
    let Some(first) = tokens(command).into_iter().next() else { return false };
    let first = first.replace('\\', "/").to_lowercase();
    first.contains("antigravity") && language_server_name(first.rsplit('/').next().unwrap_or(""))
}

fn process_source(command: &str) -> Option<Source> {
    let words = tokens(command);
    let exe = executable_name(words.first().map(String::as_str).unwrap_or(""));
    if exe == "agy" {
        return Some(Source::Agy);
    }
    if ["node", "nodejs", "bun", "mcp-server", "mcp-server.cjs"].contains(&exe.as_str()) {
        let script = if exe.starts_with("mcp-server") { words.first() } else { words.get(1) };
        if script.is_some_and(|s| {
            let p = s.replace('\\', "/").to_lowercase();
            p.rsplit('/').next() == Some("mcp-server.cjs")
                && p.split('/').any(|c| c == "antigravity-cli")
        }) {
            return Some(Source::Agy);
        }
    }
    app_executable(command).then_some(Source::App)
}

/// `--name=value` or `--name value` in a command line.
fn flag(command: &str, name: &str) -> Option<String> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let long = format!("--{name}");
    for (i, w) in words.iter().enumerate() {
        if let Some(v) = w.strip_prefix(&long) {
            if let Some(v) = v.strip_prefix('=') {
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            } else if v.is_empty() {
                return words.get(i + 1).map(|s| s.to_string());
            }
        }
    }
    None
}

fn processes(listed: &[Process]) -> Vec<ProcessInfo> {
    listed
        .iter()
        .filter_map(|p| {
            let command = p.command.as_str();
            Some(ProcessInfo {
                pid: p.pid,
                source: process_source(command)?,
                csrf: flag(command, "csrf_token"),
                extension_port: flag(command, "extension_server_port").and_then(|p| p.parse().ok()),
                extension_csrf: flag(command, "extension_server_csrf_token"),
            })
        })
        .collect()
}

struct Deadline(Instant);

impl Deadline {
    /// The time one step may take: its own limit, cut to what is left.
    fn step(&self, limit: Duration) -> Result<Duration, Fail> {
        let left = self.0.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Fail::Unavailable("Antigravity probe timed out".into()));
        }
        Ok(limit.min(left))
    }
}

/// The endpoints to ask, agy's first, and how discovery ended when there
/// are none: `Ok` with no endpoints and the skip's words.
fn endpoints(tools: &Tools, deadline: &Deadline) -> Result<(Vec<Endpoint>, &'static str), Fail> {
    let Some(listed) = tools.processes.own(deadline.step(PROCESS_TIMEOUT)?) else {
        return Ok((Vec::new(), DISCOVERY_UNAVAILABLE));
    };
    let found = processes(&listed);
    if found.is_empty() {
        return Ok((Vec::new(), NOT_RUNNING));
    }
    let mut out = Vec::new();
    let mut discovery: Option<Fail> = None;
    for p in &found {
        let listening = match tools.processes.listening(p.pid, deadline.step(PORT_TIMEOUT)?) {
            Some(ports) => ports,
            None => {
                let f = Fail::Discovery("Antigravity port discovery failed".into());
                discovery = Some(stronger(discovery, f));
                continue;
            }
        };
        let at = |https, port, csrf: Option<String>, unleash_first| Endpoint {
            https,
            port,
            source: p.source,
            csrf,
            unleash_first,
        };
        match p.source {
            Source::Agy => {
                for &port in &listening {
                    out.push(at(true, port, None, false));
                    out.push(at(false, port, None, false));
                }
            }
            Source::App => {
                if let Some(csrf) = &p.csrf {
                    for &port in &listening {
                        out.push(at(true, port, Some(csrf.clone()), true));
                        out.push(at(false, port, Some(csrf.clone()), true));
                    }
                }
                if let Some(port) = p.extension_port.filter(|e| listening.contains(e)) {
                    if let Some(token) = p.extension_csrf.clone().or_else(|| p.csrf.clone()) {
                        out.push(at(false, port, Some(token), true));
                    }
                }
            }
        }
    }
    if out.is_empty() {
        if let Some(f) = discovery {
            return Err(f);
        }
    }
    out.sort_by_key(|e| (e.source, e.port, !e.https));
    Ok((out, NO_ENDPOINT))
}

fn request(endpoint: &Endpoint, path: &str, timeout: Duration) -> Result<Value, Fail> {
    let body = if path == QUOTA_SUMMARY {
        json!({ "request": {}, "forceRefresh": false })
    } else {
        json!({ "metadata": { "ideName": "antigravity", "extensionName": "antigravity", "ideVersion": "unknown", "locale": "en" } })
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        // Only ever 127.0.0.1: the language server's own certificate.
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .map_err(|_| Fail::Unavailable("Antigravity loopback unavailable".into()))?;
    let scheme = if endpoint.https { "https" } else { "http" };
    let mut req = client
        .post(format!("{scheme}://127.0.0.1:{}{path}", endpoint.port))
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("connect-protocol-version", "1")
        .body(body.to_string());
    if let Some(token) = &endpoint.csrf {
        req = req.header("x-codeium-csrf-token", token);
    }
    let response = req.send().map_err(|e| {
        if e.is_timeout() {
            Fail::Unavailable("Antigravity loopback timed out".into())
        } else if e.is_connect() {
            Fail::Unavailable("Antigravity loopback unavailable (ECONNREFUSED)".into())
        } else {
            Fail::Unavailable("Antigravity loopback unavailable".into())
        }
    })?;
    let status = response.status().as_u16();
    let bytes = response
        .bytes()
        .map_err(|_| Fail::Unavailable("Antigravity loopback unavailable".into()))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Fail::Malformed("Antigravity loopback response too large".into()));
    }
    let text = String::from_utf8_lossy(&bytes);
    if !(200..300).contains(&status) {
        let csrf = text.to_lowercase().contains("missing csrf token")
            || serde_json::from_str::<Value>(&text).ok().is_some_and(|v| {
                v.get("message").and_then(Value::as_str).is_some_and(|m| {
                    let m = m.to_lowercase();
                    m == "missing csrf token" || m == "invalid csrf token"
                })
            });
        return Err(if (status == 401 || status == 403) && csrf {
            Fail::Csrf
        } else {
            Fail::Http(status)
        });
    }
    serde_json::from_str(&text)
        .map_err(|_| Fail::Malformed("Antigravity loopback returned invalid JSON".into()))
}

fn endpoint_quota(endpoint: &Endpoint, deadline: &Deadline) -> Result<Reading, Fail> {
    let mut failure: Option<Fail> = None;
    match request(endpoint, QUOTA_SUMMARY, deadline.step(REQUEST_TIMEOUT)?) {
        Ok(v) => {
            let windows = quota_summary(&v);
            if !windows.is_empty() {
                let account = deadline
                    .step(REQUEST_TIMEOUT)
                    .ok()
                    .and_then(|t| request(endpoint, USER_STATUS, t).ok())
                    .and_then(|v| user_status(&v))
                    .and_then(|r| r.account);
                return Ok(Reading { windows, account });
            }
            failure = Some(Fail::Malformed("Antigravity quota summary malformed".into()));
        }
        Err(e) => failure = Some(stronger(failure, e)),
    }
    for path in [USER_STATUS, MODEL_CONFIGS] {
        match request(endpoint, path, deadline.step(REQUEST_TIMEOUT)?) {
            Ok(v) => {
                if let Some(r) = user_status(&v).filter(|r| !r.windows.is_empty()) {
                    return Ok(r);
                }
            }
            Err(e) => failure = Some(stronger(failure, e)),
        }
    }
    Err(failure.unwrap_or_else(|| Fail::Other("Antigravity quota unavailable".into())))
}

/// The loopback read. The error carries the skip's words when nothing was
/// there to ask.
fn loopback(tools: &Tools) -> Result<Reading, (Fail, Option<String>)> {
    let deadline = Deadline(Instant::now() + PROBE_BUDGET);
    let (list, skip) = endpoints(tools, &deadline).map_err(|e| (e, None))?;
    if list.is_empty() {
        return Err((Fail::Unavailable(skip.into()), Some(skip.to_string())));
    }
    let mut last: Option<Fail> = None;
    for endpoint in &list {
        let attempt = (|| {
            if endpoint.unleash_first {
                request(endpoint, UNLEASH, deadline.step(REQUEST_TIMEOUT)?)?;
            }
            endpoint_quota(endpoint, &deadline)
        })();
        match attempt {
            Ok(r) => return Ok(r),
            Err(e) => last = Some(stronger(last, e)),
        }
    }
    Err((last.unwrap_or_else(|| Fail::Other("Antigravity quota unavailable".into())), None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_named_by_group_and_length() {
        let summary = json!({"groups": [
            {"displayName": "Claude and GPT", "buckets": [
                {"bucketId": "3p-weekly", "remainingFraction": 1, "resetTime": "2026-01-20T00:00:00Z"},
                {"bucketId": "3p-5h", "remainingFraction": 0.75}
            ]},
            {"displayName": "Gemini", "buckets": [
                {"bucketId": "gemini-weekly", "remainingFraction": 0.555},
                {"bucketId": "gemini-5h", "remaining": {"case": "remainingFraction", "value": 0.2}},
                {"bucketId": "gemini-argon-5h", "remainingFraction": 0.9},
                {"bucketId": "off", "disabled": true, "remainingFraction": 0}
            ]}
        ]});
        let w = quota_summary(&summary);
        let ids: Vec<&str> = w.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(
            ids,
            // quota-axi knows `gemini-5h`, not `gemini-argon-5h`.
            [
                "gemini_5h",
                "gemini_weekly",
                "claude_gpt_5h",
                "claude_gpt_weekly",
                "gemini_argon_unknown"
            ]
        );
        assert_eq!(w[0].percent_remaining, Some(20.0));
        // 0.555 left is a hair under 44.5 % used in floating point: 44 used, 56 left.
        assert_eq!(w[1].percent_remaining, Some(56.0));
        assert_eq!(w[3].resets_at.as_deref(), Some("2026-01-20T00:00:00.000Z"));
        assert_eq!(w[0].window_seconds, Some(FIVE_HOURS));
    }

    #[test]
    fn the_cli_answer_is_the_summary_under_its_command() {
        let out = json!({"command": {"name": "/quota", "data": {"response": {"groups": [
            {"displayName": "Gemini", "buckets": [{"bucketId": "gemini-5h", "remainingFraction": 0.5}]}
        ]}}}});
        assert_eq!(print_usage(&out).unwrap()[0].id, "gemini_5h");
        assert!(print_usage(&json!({"command": {"name": "/help", "data": {}}})).is_none());
    }

    #[test]
    fn the_process_list_finds_agy_and_the_app_and_their_tokens() {
        let ps = "  101 /Users/x/.local/bin/agy --serve\n\
                  202 /Applications/Antigravity.app/Contents/Resources/app/extensions/antigravity/bin/language_server_macos_arm --csrf_token abc --extension_server_port 6000 --extension_server_csrf_token def\n\
                  303 /usr/bin/node /opt/antigravity-cli/dist/mcp-server.cjs\n\
                  404 /bin/zsh\n";
        let p = processes(&crate::os::pid_table(ps));
        assert_eq!(p.len(), 3);
        assert_eq!((p[0].pid, p[0].source), (101, Source::Agy));
        assert_eq!(p[1].source, Source::App);
        assert_eq!(p[1].csrf.as_deref(), Some("abc"));
        assert_eq!(p[1].extension_port, Some(6000));
        assert_eq!(p[1].extension_csrf.as_deref(), Some("def"));
        assert_eq!(p[2].source, Source::Agy);
        assert_eq!(flag("x --csrf_token=q y", "csrf_token").as_deref(), Some("q"));
    }

    #[test]
    fn errors_are_weighed_as_quota_axi_weighs_them() {
        assert_eq!(
            status_for_error("Antigravity loopback unavailable (ECONNREFUSED)"),
            "unavailable"
        );
        assert_eq!(status_for_error("Antigravity sign-in required"), "auth_required");
        assert_eq!(status_for_error("Antigravity quota endpoint rate limited"), "rate_limited");
        assert_eq!(status_for_error("Antigravity CLI /quota failed"), "error");
        assert!(Fail::Http(503).stale_eligible() && !Fail::Http(404).stale_eligible());
        assert!(!Fail::Csrf.stale_eligible());
        assert!(Fail::Http(401).definitive_auth());
        assert_eq!(stronger(Some(Fail::Unavailable("x".into())), Fail::Http(401)), Fail::Http(401));
    }
}
