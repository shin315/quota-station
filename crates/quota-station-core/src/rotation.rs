//! The rotation file, `~/.config/cswap-prefer-team/rotation` (F06): which
//! claude-swap slots the switcher moves through, the personal slot and its
//! hours, the two thresholds and when a slot leaves the rotation.
//!
//! `key = value` lines, `#` comments, the last value of a key wins. The
//! switcher reads it on every run and quota-axi on every read, so a change
//! needs no restart. quota-axi wants all six keys and refuses the file
//! otherwise; [`parse`] applies its rules, so the editor can say what is wrong
//! before anything is written.
//!
//! The app edits the file in place: it rewrites the six keys, keeps every
//! other line, and puts one `# updated = <time> by <machine>` line at the
//! top, which quota-axi and the switcher skip as a comment. That line is how
//! two Macs tell which copy is newer. A write goes through a temporary file
//! and a rename, keeps a `rotation.bak-<time>` copy of the file before, and
//! is checked by quota-axi; when quota-axi refuses it the copy goes back.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::axi::Response;
use crate::local_time;
use crate::runner::{self, ProviderId, RunError};
use crate::time_util::parse_ts;

/// Under the home folder, next to the pause flag.
pub const FILE: &str = ".config/cswap-prefer-team/rotation";
/// The keys quota-axi requires, in the order the app writes them.
pub const KEYS: [&str; 6] = ["team", "personal", "personal_hours", "full", "fable_full", "expire"];
/// Backups kept next to the file; older ones are deleted.
pub const BACKUPS_KEPT: usize = 10;
const BACKUP_PREFIX: &str = "rotation.bak-";
const STAMP_KEY: &str = "updated";

pub fn path(home: &Path) -> PathBuf {
    home.join(FILE)
}

/// The six keys, as the editor shows them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    /// Team slots in the switcher's order.
    pub team: Vec<u32>,
    /// The personal slot; `None` writes `personal =` (no personal slot).
    pub personal: Option<u32>,
    /// `personal_hours = 8-23`: from hour 8 through hour 23, both included.
    pub personal_from: u8,
    pub personal_to: u8,
    /// Percent of a window used at which the switcher moves on.
    pub full: f64,
    /// Percent of the weekly Fable window used at which a slot is passed over.
    pub fable_full: f64,
    /// When a slot leaves the rotation, sorted by slot.
    pub expire: Vec<Expire>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expire {
    pub slot: u32,
    /// As written: `2026-10-16T17:45`, local time unless it has an offset.
    pub at: String,
}

impl Rules {
    /// The switcher's built-in values for a Mac with no rotation file, with
    /// no end times.
    pub fn defaults() -> Rules {
        Rules {
            team: vec![2, 3, 4],
            personal: Some(1),
            personal_from: 8,
            personal_to: 23,
            full: 90.0,
            fable_full: 95.0,
            expire: Vec::new(),
        }
    }

    /// The same checks quota-axi makes, plus one of the app's own: the
    /// personal slot is not also a Team slot (`rotation_personal_in_team`).
    /// Otherwise `Err` is quota-axi's reason, `rotation_bad_value:<key>`.
    pub fn check(&self) -> Result<(), String> {
        let bad = |key: &str| Err(format!("rotation_bad_value:{key}"));
        let mut seen = Vec::new();
        for slot in &self.team {
            if *slot == 0 || seen.contains(slot) {
                return bad("team");
            }
            seen.push(*slot);
        }
        if self.team.is_empty() {
            return bad("team");
        }
        if self.personal == Some(0) {
            return bad("personal");
        }
        if self.personal.is_some_and(|p| self.team.contains(&p)) {
            return Err("rotation_personal_in_team".into());
        }
        if self.personal_from > self.personal_to || self.personal_to > 23 {
            return bad("personal_hours");
        }
        if !threshold_ok(self.full) {
            return bad("full");
        }
        if !threshold_ok(self.fable_full) {
            return bad("fable_full");
        }
        let mut slots = Vec::new();
        for e in &self.expire {
            if e.slot == 0 || slots.contains(&e.slot) || !end_time_ok(&e.at) {
                return bad("expire");
            }
            slots.push(e.slot);
        }
        Ok(())
    }

    fn value(&self, key: &str) -> String {
        let slots = |s: &[u32]| s.iter().map(u32::to_string).collect::<Vec<_>>().join(" ");
        match key {
            "team" => slots(&self.team),
            "personal" => self.personal.map(|p| p.to_string()).unwrap_or_default(),
            "personal_hours" => format!("{}-{}", self.personal_from, self.personal_to),
            "full" => number(self.full),
            "fable_full" => number(self.fable_full),
            "expire" => {
                let mut ends = self.expire.clone();
                ends.sort_by_key(|e| e.slot);
                ends.iter().map(|e| format!("{}={}", e.slot, e.at)).collect::<Vec<_>>().join(" ")
            }
            _ => String::new(),
        }
    }
}

fn threshold_ok(v: f64) -> bool {
    v.is_finite() && v > 0.0 && v <= 100.0
}

/// `90`, `92.5`.
fn number(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

/// quota-axi's form: `YYYY-MM-DDTHH:MM`, optional seconds, optional `Z` or
/// `±HH:MM`, and a real date and time.
fn end_time_ok(at: &str) -> bool {
    let b = at.as_bytes();
    let digits =
        |r: std::ops::Range<usize>| b.get(r).is_some_and(|p| p.iter().all(u8::is_ascii_digit));
    if !(b.len() >= 16
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16))
    {
        return false;
    }
    let mut rest = &at[16..];
    if let Some(r) = rest.strip_prefix(':') {
        if r.len() < 2 || !r.as_bytes()[..2].iter().all(u8::is_ascii_digit) {
            return false;
        }
        rest = &r[2..];
    }
    let offset_ok = match rest.as_bytes() {
        [] | [b'Z'] => true,
        [s, h1, h2, b':', m1, m2] => {
            let two = |a: u8, b: u8| {
                (a.is_ascii_digit() && b.is_ascii_digit()).then(|| (a - b'0') * 10 + (b - b'0'))
            };
            (*s == b'+' || *s == b'-')
                && two(*h1, *h2).is_some_and(|h| h <= 23)
                && two(*m1, *m2).is_some_and(|m| m <= 59)
        }
        _ => false,
    };
    offset_ok && local_time::instant(at).is_some()
}

/// The `# updated = <time> by <machine>` line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stamp {
    /// RFC 3339 with this Mac's offset: `2026-10-16T14:30:05+07:00`.
    pub at: String,
    pub machine: String,
}

impl Stamp {
    pub fn new(at: OffsetDateTime, machine: &str) -> Stamp {
        let local = at.to_offset(local_time::offset_at(at));
        let off = local.offset();
        let (h, m, _) = off.as_hms();
        let sign = if off.is_negative() { '-' } else { '+' };
        let at = format!(
            "{}{sign}{:02}:{:02}",
            local_time::stamp(at),
            h.unsigned_abs(),
            m.unsigned_abs()
        );
        Stamp { at, machine: machine.replace(['\n', '\r'], " ").trim().to_string() }
    }

    /// The stamp for a save at `at`, always later than `previous`: two saves
    /// within one second still tell apart which is newer.
    pub fn next(at: OffsetDateTime, machine: &str, previous: Option<&Stamp>) -> Stamp {
        let after = previous.and_then(Stamp::instant).map(|p| p + time::Duration::seconds(1));
        Stamp::new(after.map_or(at, |a| a.max(at)), machine)
    }

    pub fn instant(&self) -> Option<OffsetDateTime> {
        parse_ts(&self.at)
    }

    /// Written later than `other`. A stamp that is not a time is older than
    /// any that is, and two unreadable stamps are not newer than each other.
    pub fn newer_than(&self, other: Option<&Stamp>) -> bool {
        match (self.instant(), other.and_then(Stamp::instant)) {
            (Some(a), Some(b)) => a > b,
            (Some(_), None) => true,
            _ => false,
        }
    }

    fn line(&self) -> String {
        format!("# {STAMP_KEY} = {} by {}", self.at, self.machine)
    }

    fn parse_line(line: &str) -> Option<Stamp> {
        let rest = line.trim().strip_prefix('#')?.trim_start();
        let rest = rest.strip_prefix(STAMP_KEY)?.trim_start().strip_prefix('=')?.trim();
        let (at, machine) = rest.split_once(" by ")?;
        Some(Stamp { at: at.trim().to_string(), machine: machine.trim().to_string() })
    }
}

/// A file quota-axi accepts.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub rules: Rules,
    pub stamp: Option<Stamp>,
    /// Keys other than the six, kept as they are.
    pub ignored_keys: Vec<String>,
}

/// Reads the file as quota-axi does. `Err` is quota-axi's reason:
/// `rotation_malformed_line`, `rotation_missing_key:<key>` or
/// `rotation_bad_value:<key>`.
pub fn parse(text: &str) -> Result<Parsed, String> {
    let mut values: Vec<(&str, String)> = Vec::new();
    let mut ignored = Vec::new();
    let mut stamp = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            if stamp.is_none() {
                stamp = Stamp::parse_line(line);
            }
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or(("", ""));
        let key = key.trim();
        if !key_ok(key) {
            return Err("rotation_malformed_line".into());
        }
        match KEYS.iter().find(|k| **k == key) {
            Some(k) => {
                values.retain(|(x, _)| x != k);
                values.push((k, value.trim().to_string()));
            }
            None if !ignored.iter().any(|x: &String| x == key) => ignored.push(key.to_string()),
            None => {}
        }
    }
    let get = |key: &str| values.iter().find(|(k, _)| *k == key).map(|(_, v)| v.as_str());
    for key in KEYS {
        if get(key).is_none() {
            return Err(format!("rotation_missing_key:{key}"));
        }
    }
    let bad = |key: &str| format!("rotation_bad_value:{key}");
    let team = slot_list(get("team").unwrap_or("")).ok_or_else(|| bad("team"))?;
    let personal = match get("personal").unwrap_or("") {
        "" => None,
        p => Some(slot(p).ok_or_else(|| bad("personal"))?),
    };
    let (from, to) = get("personal_hours")
        .and_then(|h| h.split_once('-'))
        .and_then(|(a, b)| Some((hour(a)?, hour(b)?)))
        .ok_or_else(|| bad("personal_hours"))?;
    let full = threshold(get("full").unwrap_or("")).ok_or_else(|| bad("full"))?;
    let fable_full = threshold(get("fable_full").unwrap_or("")).ok_or_else(|| bad("fable_full"))?;
    let mut expire: Vec<Expire> = Vec::new();
    for item in get("expire").unwrap_or("").split_whitespace() {
        let (s, at) = item.split_once('=').ok_or_else(|| bad("expire"))?;
        let s = slot(s).ok_or_else(|| bad("expire"))?;
        if !end_time_ok(at) {
            return Err(bad("expire"));
        }
        // A repeated slot keeps its last time, as quota-axi keeps it.
        expire.retain(|e| e.slot != s);
        expire.push(Expire { slot: s, at: at.to_string() });
    }
    expire.sort_by_key(|e| e.slot);
    let rules =
        Rules { team, personal, personal_from: from, personal_to: to, full, fable_full, expire };
    if rules.personal_from > rules.personal_to || rules.personal_to > 23 {
        return Err(bad("personal_hours"));
    }
    Ok(Parsed { rules, stamp, ignored_keys: ignored })
}

fn key_ok(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn slot(text: &str) -> Option<u32> {
    let ok = text.starts_with(|c: char| ('1'..='9').contains(&c))
        && text.chars().all(|c| c.is_ascii_digit());
    ok.then(|| text.parse().ok()).flatten()
}

fn slot_list(text: &str) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for token in text.split_whitespace() {
        let s = slot(token)?;
        if out.contains(&s) {
            return None;
        }
        out.push(s);
    }
    (!out.is_empty()).then_some(out)
}

fn hour(text: &str) -> Option<u8> {
    let ok = (1..=2).contains(&text.len()) && text.chars().all(|c| c.is_ascii_digit());
    ok.then(|| text.parse().ok()).flatten()
}

fn threshold(text: &str) -> Option<f64> {
    let (int, frac) = text.split_once('.').unwrap_or((text, "0"));
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if !(digits(int) && digits(frac)) {
        return None;
    }
    text.parse().ok().filter(|v| threshold_ok(*v))
}

/// The file with `rules` and `stamp`: every line of `previous` that is not
/// one of the six keys or a stamp stays where it was, each key takes the
/// place of its first line, a key the file lacked goes at the end, and the
/// stamp goes first. A line quota-axi cannot read is dropped (the backup
/// keeps it).
pub fn render(rules: &Rules, stamp: &Stamp, previous: Option<&str>) -> String {
    let mut out = vec![stamp.line()];
    let mut written: Vec<&str> = Vec::new();
    match previous {
        Some(text) => {
            for raw in text.lines() {
                let line = raw.trim();
                if line.starts_with('#') {
                    if Stamp::parse_line(line).is_none() {
                        out.push(raw.to_string());
                    }
                    continue;
                }
                if line.is_empty() {
                    out.push(raw.to_string());
                    continue;
                }
                let key = line.split_once('=').map(|(k, _)| k.trim()).unwrap_or("");
                if !key_ok(key) {
                    continue;
                }
                match KEYS.iter().find(|k| **k == key) {
                    Some(k) if written.contains(k) => {}
                    Some(k) => {
                        out.push(format!("{k} = {}", rules.value(k)));
                        written.push(k);
                    }
                    None => out.push(raw.to_string()),
                }
            }
        }
        None => {
            out.push("# Luật xoay vòng của cswap-prefer-team, sửa trong Quota Station.".into());
        }
    }
    for k in KEYS {
        if !written.contains(&k) {
            out.push(format!("{k} = {}", rules.value(k)));
        }
    }
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// What is on disk now.
#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Missing,
    Unreadable(String),
    /// quota-axi would refuse it, for `reason`.
    Invalid {
        text: String,
        reason: String,
    },
    Ok {
        text: String,
        parsed: Parsed,
    },
}

impl FileState {
    pub fn text(&self) -> Option<&str> {
        match self {
            FileState::Invalid { text, .. } | FileState::Ok { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Changes whenever the text does; `None` when there is no file. The
    /// editor sends back the version it started from, so a save never
    /// overwrites a file changed meanwhile (by hand, or by another Mac).
    pub fn version(&self) -> Option<String> {
        self.text().map(version)
    }

    pub fn stamp(&self) -> Option<&Stamp> {
        match self {
            FileState::Ok { parsed, .. } => parsed.stamp.as_ref(),
            _ => None,
        }
    }
}

/// FNV-1a of the text, in hex: the same text gives the same version on every run.
pub fn version(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// A file that breaks [`Rules::check`] (a personal slot that is also a Team
/// slot) counts as invalid too, so the editor starts it from the defaults
/// instead of from rules it could never save.
pub fn load(file: &Path) -> FileState {
    match std::fs::read_to_string(file) {
        Ok(text) => match parse(&text).and_then(|p| p.rules.check().map(|()| p)) {
            Ok(parsed) => FileState::Ok { text, parsed },
            Err(reason) => FileState::Invalid { text, reason },
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => FileState::Missing,
        Err(e) => FileState::Unreadable(e.to_string()),
    }
}

static WRITES: AtomicU64 = AtomicU64::new(0);

/// Writes `text` through a temporary file and a rename. A file already there
/// is first copied to `rotation.bak-<local time>`, returned, and only the
/// newest [`BACKUPS_KEPT`] copies stay.
pub fn write(file: &Path, text: &str, now: OffsetDateTime) -> io::Result<Option<PathBuf>> {
    let dir = file.parent().ok_or_else(|| io::Error::other("no folder"))?;
    std::fs::create_dir_all(dir)?;
    let backup = if file.exists() {
        let name = local_time::stamp(now).replace(['-', ':'], "").replace('T', "-");
        let mut backup = dir.join(format!("{BACKUP_PREFIX}{name}"));
        let mut n = 2;
        while backup.exists() {
            backup = dir.join(format!("{BACKUP_PREFIX}{name}-{n}"));
            n += 1;
        }
        std::fs::copy(file, &backup)?;
        Some(backup)
    } else {
        None
    };
    replace(file, text)?;
    prune(dir);
    Ok(backup)
}

fn replace(file: &Path, text: &str) -> io::Result<()> {
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = file.with_file_name(format!(".rotation.tmp-{}-{n}", std::process::id()));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, file).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Backups by name, newest first: the name is the time it was taken.
pub fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(BACKUP_PREFIX))
        .map(|e| e.path())
        .collect();
    found.sort();
    found.reverse();
    found
}

fn prune(dir: &Path) {
    for old in backups(dir).into_iter().skip(BACKUPS_KEPT) {
        let _ = std::fs::remove_file(old);
    }
}

/// Puts the file back as it was before a write: the backup's text, or no
/// file when there was none.
pub fn restore(file: &Path, backup: Option<&Path>) -> io::Result<()> {
    match backup {
        Some(b) => replace(file, &std::fs::read_to_string(b)?),
        None => match std::fs::remove_file(file) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

/// What quota-axi made of the file it just read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    /// quota-axi's reason, `rotation_bad_value:personal_hours`.
    Refused(String),
    /// The report has no rotation block: this quota-axi does not read the file.
    NotRead,
}

/// quota-axi restates the Claude row from the rotation file on every read,
/// a reused one too. A file it refuses leaves `claudePool` on `active_slot`
/// with a `rotation_…` reason; any other state means it read the file.
pub fn verdict(report: &Response) -> Verdict {
    let pool = report.primary("claude").and_then(|row| row.claude_pool.as_ref());
    match pool {
        None => Verdict::NotRead,
        Some(p) => match p.reason.as_deref() {
            Some(r) if p.status == "active_slot" && r.starts_with("rotation_") => {
                Verdict::Refused(r.to_string())
            }
            _ => Verdict::Accepted,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The file is not the one the editor started from.
    Changed,
    /// The text breaks one of quota-axi's rules; nothing was written.
    Invalid(String),
    Write(String),
    /// quota-axi read the new file and refused it.
    Refused {
        reason: String,
        restored: bool,
    },
    /// quota-axi could not check the new file.
    Unchecked {
        error: String,
        restored: bool,
    },
}

impl ApplyError {
    pub fn kind(&self) -> &'static str {
        match self {
            ApplyError::Changed => "changed",
            ApplyError::Invalid(_) => "invalid",
            ApplyError::Write(_) => "write",
            ApplyError::Refused { .. } => "refused",
            ApplyError::Unchecked { .. } => "unchecked",
        }
    }
}

/// A file written and accepted by quota-axi.
#[derive(Debug, Clone)]
pub struct Applied {
    pub text: String,
    pub parsed: Parsed,
    pub backup: Option<PathBuf>,
    /// The quota-axi report that accepted it.
    pub report: Response,
}

/// Writes `text` to `file` when the file is still at `base` (its version, or
/// `None` for no file), then has quota-axi read it. When quota-axi refuses
/// it, or cannot run, the file before goes back: the switcher on this Mac
/// never runs on a file quota-axi did not accept.
pub fn apply(
    file: &Path,
    quota_axi: Option<&Path>,
    text: &str,
    base: Option<&str>,
    now: OffsetDateTime,
) -> Result<Applied, ApplyError> {
    let parsed =
        parse(text).and_then(|p| p.rules.check().map(|()| p)).map_err(ApplyError::Invalid)?;
    let current = load(file);
    if let FileState::Unreadable(e) = &current {
        return Err(ApplyError::Write(e.clone()));
    }
    if current.version().as_deref() != base {
        return Err(ApplyError::Changed);
    }
    let backup = write(file, text, now).map_err(|e| ApplyError::Write(e.to_string()))?;
    let undo = || restore(file, backup.as_deref()).is_ok();
    let report = match quota_axi {
        Some(path) => runner::read(path, ProviderId::Claude),
        None => Err(RunError::NotConfigured),
    };
    match report {
        Ok(report) => match verdict(&report) {
            Verdict::Accepted => Ok(Applied { text: text.to_string(), parsed, backup, report }),
            Verdict::Refused(reason) => Err(ApplyError::Refused { reason, restored: undo() }),
            Verdict::NotRead => Err(ApplyError::Unchecked {
                error: "quota-axi không đọc file luật".into(),
                restored: undo(),
            }),
        },
        Err(e) => Err(ApplyError::Unchecked { error: e.to_string(), restored: undo() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_times_in_quota_axis_forms() {
        for ok in [
            "2026-10-16T17:45",
            "2026-10-16T17:45:30",
            "2026-10-16T17:45Z",
            "2026-10-16T17:45+07:00",
        ] {
            assert!(end_time_ok(ok), "{ok}");
        }
        for bad in [
            "2026-10-16",
            "2026-10-16 17:45",
            "2026-13-16T17:45",
            "2026-10-16T17:45+7",
            "2026-10-16T17:45+99:99",
            "2026-10-16T17:45-07:60",
            "x",
        ] {
            assert!(!end_time_ok(bad), "{bad}");
        }
    }

    #[test]
    fn thresholds_take_decimals_but_not_signs_or_zero() {
        assert_eq!(threshold("90"), Some(90.0));
        assert_eq!(threshold("92.5"), Some(92.5));
        for bad in ["0", "101", "-5", "9e1", "", "90."] {
            assert_eq!(threshold(bad), None, "{bad}");
        }
        assert_eq!(number(90.0), "90");
        assert_eq!(number(92.5), "92.5");
    }
}
