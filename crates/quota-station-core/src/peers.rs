//! The owner's other Macs (F06). Each Mac keeps its own rotation file, read
//! by its own switcher and quota-axi, and each switches only itself. After a
//! save here the app copies this Mac's file to every other Mac over SSH, so
//! all of them follow the same rules.
//!
//! A copy goes like this: read the other Mac's file; when it already has this
//! text, done; when its `updated` stamp is newer, or it was edited by hand
//! there (the same stamp with other text, no stamp, or text quota-axi cannot
//! read), stop and let the owner choose, since the app never merges two
//! copies; otherwise write it there the way the app writes here (temporary
//! file, rename, `rotation.bak-<time>`), and read it back to compare. The
//! write first checks the file there is still the one just read (its POSIX
//! `cksum`), so a copy saved there in between is not overwritten. A Mac that
//! cannot be reached is tried again every 5 minutes. A Mac's state is kept
//! against the version (the text) of this Mac's file, not its stamp, so two
//! saves within one second are two copies to send.
//!
//! SSH runs with `BatchMode=yes`, so it never asks for a password or a host
//! key: an alias that needs either fails as unreachable. The only commands it
//! runs on the other Mac are the two fixed scripts below, and the copy of the
//! agents' setup (M13, `agents::copy`) runs its own fixed ones the same way.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::rotation::{self, Stamp};
use crate::runner::{self, RunError};
use crate::time_util::parse_ts;

/// One SSH call, connection included.
pub const SSH_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a Mac that could not take the file waits before the next try.
pub const RETRY_MINUTES: i64 = 5;
/// The longest alias kept.
pub const ALIAS_MAX: usize = 64;

/// `ssh` exits 255 when it could not reach the other Mac or log in.
const SSH_FAILED: i32 = 255;
/// The read script's exit when the other Mac has no rotation file.
const NO_FILE: i32 = 3;
/// The write script's exit when the file there is not the one read before.
const CHANGED: i32 = 4;

/// Prints the other Mac's rotation file, or exits 3 when it has none.
const READ_SCRIPT: &str =
    "f=\"$HOME/.config/cswap-prefer-team/rotation\"; [ -e \"$f\" ] || exit 3; cat \"$f\"";

/// Takes the new file on stdin. Exits 4 unless the file there is still the
/// one read before (`expect`: its `cksum`, or `none` for no file); then writes
/// the new one next to it, keeps the old one as `rotation.bak-<time>-<pid>`
/// (the newest 10 stay), renames it into place and prints what is there now,
/// for the read-back.
fn write_script(expect: &str) -> String {
    format!(
        concat!(
            "set -e; d=\"$HOME/.config/cswap-prefer-team\"; f=\"$d/rotation\"; mkdir -p \"$d\"; ",
            "c=none; if [ -e \"$f\" ]; then set -- $(cksum < \"$f\"); c=$1; fi; ",
            "if [ \"$c\" != {expect} ]; then cat > /dev/null; exit 4; fi; ",
            "t=\"$d/.rotation.tmp-qs-$$\"; cat > \"$t\"; ",
            "if [ -e \"$f\" ]; then cp -p \"$f\" \"$d/rotation.bak-$(date +%Y%m%d-%H%M%S)-$$\"; fi; ",
            "mv -f \"$t\" \"$f\"; n=0; ls -1r \"$d\" | while read -r b; do case \"$b\" in ",
            "rotation.bak-*) n=$((n+1)); [ \"$n\" -le 10 ] || rm -f \"$d/$b\";; esac; done; ",
            "cat \"$f\""
        ),
        expect = expect
    )
}

/// The POSIX `cksum` CRC of `text`, the value the write script compares.
pub fn cksum(text: &str) -> u32 {
    let step = |crc: u32, byte: u8| {
        let mut c = crc ^ (u32::from(byte) << 24);
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04C1_1DB7 } else { c << 1 };
        }
        c
    };
    let mut crc = text.bytes().fold(0, step);
    let mut len = text.len();
    while len > 0 {
        crc = step(crc, (len & 0xff) as u8);
        len >>= 8;
    }
    !crc
}

/// An SSH alias as typed in settings: what `~/.ssh/config` or a host name
/// allows, and never something `ssh` would read as an option.
pub fn alias_ok(alias: &str) -> bool {
    let mut chars = alias.chars();
    alias.len() <= ALIAS_MAX
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'))
}

/// The first `ssh` on `path_var` (a `PATH` value), else `/usr/bin/ssh`.
pub fn find_ssh(path_var: Option<&OsStr>) -> Option<PathBuf> {
    path_var
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|d| d.join("ssh"))
        .chain([PathBuf::from("/usr/bin/ssh")])
        .find(|p| p.is_file())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteError {
    /// SSH could not reach the Mac or log in, with its first error line.
    Unreachable(String),
    Timeout,
    /// The command ran there and failed.
    Failed(String),
    /// The file there changed after it was read; nothing was written.
    Changed,
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteError::Unreachable(e) => write!(f, "{e}"),
            RemoteError::Timeout => write!(f, "SSH hết giờ chờ ({} giây)", SSH_TIMEOUT.as_secs()),
            RemoteError::Failed(e) => write!(f, "{e}"),
            RemoteError::Changed => write!(f, "file luật trên máy đó vừa đổi trong lúc gửi"),
        }
    }
}

pub(crate) fn ssh(
    program: &Path,
    alias: &str,
    script: &str,
    input: Option<&str>,
) -> Result<runner::Output, RemoteError> {
    if !alias_ok(alias) {
        return Err(RemoteError::Failed("bad alias".into()));
    }
    let args: Vec<String> = [
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=5",
        "-T",
        alias,
        // ssh hands its remaining words to the other Mac's login shell, so
        // the script runs under /bin/sh whatever that shell is. The scripts
        // hold no single quote.
        &format!("/bin/sh -c '{script}'"),
    ]
    .map(String::from)
    .to_vec();
    runner::run_with(program, &args, &[], input, SSH_TIMEOUT).map_err(|e| match e {
        RunError::Timeout(_) => RemoteError::Timeout,
        other => RemoteError::Failed(other.to_string()),
    })
}

fn failure(out: &runner::Output) -> RemoteError {
    let line = runner::first_line(&out.stderr);
    if out.code == Some(SSH_FAILED) {
        RemoteError::Unreachable(line)
    } else {
        RemoteError::Failed(if line.is_empty() { format!("exit {:?}", out.code) } else { line })
    }
}

/// The other Mac's rotation file; `None` when it has none.
pub fn read(program: &Path, alias: &str) -> Result<Option<String>, RemoteError> {
    let out = ssh(program, alias, READ_SCRIPT, None)?;
    match out.code {
        Some(0) => Ok(Some(String::from_utf8_lossy(&out.stdout).into_owned())),
        Some(NO_FILE) => Ok(None),
        _ => Err(failure(&out)),
    }
}

/// Writes `text` as the other Mac's rotation file, when the file there is
/// still `before` (`None`: no file), and returns what it reads back.
pub fn write(
    program: &Path,
    alias: &str,
    text: &str,
    before: Option<&str>,
) -> Result<String, RemoteError> {
    let expect = before.map(|b| cksum(b).to_string()).unwrap_or_else(|| "none".into());
    let out = ssh(program, alias, &write_script(&expect), Some(text))?;
    match out.code {
        Some(0) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        Some(CHANGED) => Err(RemoteError::Changed),
        _ => Err(failure(&out)),
    }
}

/// What to do with the other Mac's file before writing this Mac's.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// It already has this text.
    Same,
    Write,
    /// Its copy is newer: the owner chooses.
    Newer(Stamp),
    /// Edited by hand there: this Mac's stamp with other text, no stamp at
    /// all (`None`), or text quota-axi cannot read. The owner chooses.
    Edited(Option<Stamp>),
}

pub fn plan(local: &str, stamp: &Stamp, theirs: Option<&str>) -> Plan {
    let Some(theirs) = theirs else { return Plan::Write };
    if theirs == local {
        return Plan::Same;
    }
    let Ok(parsed) = rotation::parse(theirs) else { return Plan::Edited(None) };
    match parsed.stamp {
        Some(s) if s.newer_than(Some(stamp)) => Plan::Newer(s),
        Some(s) if s == *stamp => Plan::Edited(Some(s)),
        Some(_) => Plan::Write,
        None => Plan::Edited(None),
    }
}

/// Where one other Mac stands, against this Mac's file at `against`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PeerState {
    /// It has this Mac's file; `at` is when that was seen, RFC 3339.
    Synced {
        at: String,
    },
    Unreachable {
        error: String,
        retry_at: String,
    },
    /// Written, but what it read back is not what was sent.
    Mismatch {
        retry_at: String,
    },
    /// SSH worked and the command there failed.
    Failed {
        error: String,
        retry_at: String,
    },
    /// Its file is newer; nothing was written there.
    Newer {
        their: Stamp,
    },
    /// Its file was edited by hand (`their`: its stamp, if any); nothing was
    /// written.
    Edited {
        their: Option<Stamp>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerRecord {
    pub state: PeerState,
    /// The version of this Mac's file ([`rotation::version`]) the state is about.
    pub against: String,
    /// The other Mac's text when it holds another copy, for comparing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub their_text: Option<String>,
}

fn rfc(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

/// Copies this Mac's file (`text`, stamped `stamp`) to the Mac at `alias`.
/// With `force` (the owner chose this Mac's copy over the other one), a newer
/// or hand-edited copy there is replaced too; the check that it did not
/// change since it was read still holds.
pub fn push(
    program: &Path,
    alias: &str,
    text: &str,
    stamp: &Stamp,
    now: OffsetDateTime,
    force: bool,
) -> PeerRecord {
    let retry_at = rfc(now + time::Duration::minutes(RETRY_MINUTES));
    let against = rotation::version(text);
    let record = |state, their_text| PeerRecord { state, against: against.clone(), their_text };
    let fail = |e: RemoteError| match e {
        RemoteError::Unreachable(error) => {
            record(PeerState::Unreachable { error, retry_at: retry_at.clone() }, None)
        }
        RemoteError::Timeout => record(
            PeerState::Unreachable {
                error: RemoteError::Timeout.to_string(),
                retry_at: retry_at.clone(),
            },
            None,
        ),
        e @ (RemoteError::Failed(_) | RemoteError::Changed) => {
            record(PeerState::Failed { error: e.to_string(), retry_at: retry_at.clone() }, None)
        }
    };
    // A copy saved there between the read and the write is read again once.
    for _ in 0..2 {
        let theirs = match read(program, alias) {
            Ok(t) => t,
            Err(e) => return fail(e),
        };
        match plan(text, stamp, theirs.as_deref()) {
            Plan::Same => return record(PeerState::Synced { at: rfc(now) }, None),
            Plan::Newer(their) if !force => return record(PeerState::Newer { their }, theirs),
            Plan::Edited(their) if !force => return record(PeerState::Edited { their }, theirs),
            _ => {}
        }
        match write(program, alias, text, theirs.as_deref()) {
            Ok(back) if back == text => return record(PeerState::Synced { at: rfc(now) }, None),
            Ok(_) => return record(PeerState::Mismatch { retry_at: retry_at.clone() }, None),
            Err(RemoteError::Changed) => continue,
            Err(e) => return fail(e),
        }
    }
    fail(RemoteError::Changed)
}

/// Whether the Mac's copy should be tried now: never tried, about another
/// version of this Mac's file (`version`, [`rotation::version`]), or a failed
/// try whose wait is over. A Mac with a newer or hand-edited copy waits for
/// the owner.
pub fn due(record: Option<&PeerRecord>, version: &str, now: OffsetDateTime) -> bool {
    let Some(r) = record else { return true };
    if r.against != version {
        return true;
    }
    match &r.state {
        PeerState::Unreachable { retry_at, .. }
        | PeerState::Mismatch { retry_at }
        | PeerState::Failed { retry_at, .. } => parse_ts(retry_at).is_none_or(|t| t <= now),
        _ => false,
    }
}

/// The records of every other Mac, by alias: `rotation-sync.json` in the
/// app's folder, so a restart keeps trying the Macs that were behind.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SyncLog {
    #[serde(default)]
    pub peers: BTreeMap<String, PeerRecord>,
}

impl SyncLog {
    pub fn load(file: &Path) -> SyncLog {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).map_err(io::Error::other)?)?;
        std::fs::rename(&tmp, file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_that_ssh_would_read_as_options_are_refused() {
        for ok in ["mac-b", "mac.local", "user@mac-c", "m6", "Mac_C"] {
            assert!(alias_ok(ok), "{ok}");
        }
        for bad in ["", "-oProxyCommand=x", "mac b", "mac;rm", "mac'", "a\nb", &"m".repeat(65)] {
            assert!(!alias_ok(bad), "{bad}");
        }
    }

    #[test]
    fn the_scripts_fit_in_single_quotes() {
        assert!(!READ_SCRIPT.contains('\''));
        assert!(!write_script("none").contains('\''));
    }

    #[test]
    fn cksum_matches_the_posix_tool() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        for text in ["", "a", "team = 2 3 4\npersonal = 1\n", &"x".repeat(300)] {
            let mut child = Command::new("cksum")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("cksum runs");
            child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
            let out = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
            assert_eq!(
                out.split_whitespace().next(),
                Some(cksum(text).to_string().as_str()),
                "{text:?}"
            );
        }
    }
}
