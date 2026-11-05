// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Antigravity's OAuth client, found in the copy of Antigravity installed on
//! this computer, so that adding an account needs no setup.
//!
//! Antigravity's own programs carry the client among their strings: the
//! desktop app's `language_server` (`language_server.exe` on Windows) and
//! the `agy` CLI. The app reads those files, and only reads them: it never
//! runs, changes or signs them. A program packs its strings with nothing
//! between them, so the app does not guess where a string ends: around each
//! marker (`.apps.googleusercontent.com` after an id, `GOCSPX-` before a
//! secret) it hashes every string that could be the id or the secret and
//! keeps the one whose SHA-256 is the pin ([`Pins::antigravity`]). The
//! repository holds only those two hashes, never the id or the secret.
//!
//! What it finds stays in this process's memory: it is never written,
//! logged or shown, and `OAuthClient`'s `Debug` hides it. As in any OAuth
//! login, the id goes in Google's consent URL and the secret only to
//! Google's token endpoint. A scan is kept while
//! the programs found are the same files, of the same length and time, so
//! the windows can ask as often as they like.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use super::client::OAuthClient;

/// SHA-256 of Antigravity's OAuth client id, lowercase hex.
const ID_SHA256: &str = "bf00c418024ba6bf606ccdc37120976e41bc429dd1d46ecf16a729aa532626ea";
/// SHA-256 of its client secret, lowercase hex.
const SECRET_SHA256: &str = "1d2f041093fd95aa8995a038c711d50a7960da09a505381c09a745d6ad0ecc60";

/// What ends a Google OAuth client id.
const ID_MARK: &[u8] = b".apps.googleusercontent.com";
/// What starts a Google OAuth client secret.
const SECRET_MARK: &[u8] = b"GOCSPX-";
/// The longest part of an id before its marker that is tried.
const ID_HEAD_MAX: usize = 96;
/// The longest part of a secret after its marker that is tried.
const SECRET_TAIL_MAX: usize = 64;
/// What a scan reads at once, and what it keeps of the read before so a
/// string cut by a read is whole in the next one.
const CHUNK: usize = 8 << 20;
const OVERLAP: usize = 256;

/// A file smaller or larger than this is not one of Antigravity's programs.
pub const MIN_SIZE: u64 = 1 << 20;
pub const MAX_SIZE: u64 = 1 << 30;
/// How deep and how wide the search of an install folder goes.
const MAX_DEPTH: usize = 8;
const MAX_ENTRIES: usize = 50_000;

/// The SHA-256 of the client's id and secret.
#[derive(Clone, PartialEq, Eq)]
pub struct Pins {
    id: [u8; 32],
    secret: [u8; 32],
}

impl std::fmt::Debug for Pins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pins(…)")
    }
}

impl Pins {
    /// Antigravity's client.
    pub fn antigravity() -> Self {
        Pins::from_hex(ID_SHA256, SECRET_SHA256).expect("the pins are 64 hex digits")
    }

    /// Pins written as 64 lowercase or uppercase hex digits each.
    pub fn from_hex(id: &str, secret: &str) -> Option<Self> {
        Some(Pins { id: hex32(id)?, secret: hex32(secret)? })
    }

    /// The pins of a given client: for the tests' fake programs.
    pub fn of(id: &str, secret: &str) -> Self {
        Pins { id: sha256(id.as_bytes()), secret: sha256(secret.as_bytes()) }
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex32(text: &str) -> Option<[u8; 32]> {
    let text = text.as_bytes();
    if text.len() != 64 {
        return None;
    }
    let digit = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = [0u8; 32];
    for (i, pair) in text.chunks(2).enumerate() {
        out[i] = digit(pair[0])? << 4 | digit(pair[1])?;
    }
    Some(out)
}

fn id_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-'
}

fn secret_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
}

/// The pinned id in `buf`: of the strings that end at a marker, the one
/// whose SHA-256 is the pin.
fn find_id(buf: &[u8], pins: &Pins) -> Option<String> {
    for at in memchr::memmem::find_iter(buf, ID_MARK) {
        let end = at + ID_MARK.len();
        let mut start = at;
        while start > 0 && at - start < ID_HEAD_MAX && id_byte(buf[start - 1]) {
            start -= 1;
            if sha256(&buf[start..end]) == pins.id {
                return String::from_utf8(buf[start..end].to_vec()).ok();
            }
        }
    }
    None
}

/// The pinned secret in `buf`: of the strings that start at a marker, the
/// one whose SHA-256 is the pin.
fn find_secret(buf: &[u8], pins: &Pins) -> Option<String> {
    for start in memchr::memmem::find_iter(buf, SECRET_MARK) {
        let mut end = start + SECRET_MARK.len();
        while end < buf.len() && end - start - SECRET_MARK.len() < SECRET_TAIL_MAX {
            if !secret_byte(buf[end]) {
                break;
            }
            end += 1;
            if sha256(&buf[start..end]) == pins.secret {
                return String::from_utf8(buf[start..end].to_vec()).ok();
            }
        }
    }
    None
}

/// Fills `buf` from `reader`, short only at the end of the file.
fn read_full(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// The pinned client in `reader`'s bytes, read `chunk` bytes at a time;
/// `None` unless both the id and the secret are there.
pub fn scan(
    mut reader: impl Read,
    pins: &Pins,
    chunk: usize,
) -> std::io::Result<Option<OAuthClient>> {
    let chunk = chunk.max(OVERLAP);
    let mut buf: Vec<u8> = Vec::with_capacity(chunk + OVERLAP);
    let (mut id, mut secret) = (None, None);
    loop {
        let drop = buf.len().saturating_sub(OVERLAP);
        buf.drain(..drop);
        let kept = buf.len();
        buf.resize(kept + chunk, 0);
        let n = read_full(&mut reader, &mut buf[kept..])?;
        buf.truncate(kept + n);
        if id.is_none() {
            id = find_id(&buf, pins);
        }
        if secret.is_none() {
            secret = find_secret(&buf, pins);
        }
        if let (Some(id), Some(secret)) = (&id, &secret) {
            return Ok(OAuthClient::new(id, secret));
        }
        if n < chunk {
            return Ok(None);
        }
    }
}

/// Why no client was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Miss {
    /// No program of Antigravity's is on this computer.
    NotInstalled,
    /// Antigravity is there, but none of its programs holds the pinned
    /// client: a build newer (or older) than this app knows.
    Unrecognized,
}

/// One program found, as it was when it was looked at: a change to it
/// makes the next look scan again.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
}

type Found = Result<OAuthClient, Miss>;

/// Where Antigravity's programs may be: install folders, searched for them,
/// and the CLI's own places.
pub struct Sources {
    folders: Vec<PathBuf>,
    programs: Vec<PathBuf>,
    pins: Pins,
    last: Mutex<Option<(Vec<Seen>, Found)>>,
}

impl std::fmt::Debug for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sources")
            .field("folders", &self.folders)
            .field("programs", &self.programs)
            .finish_non_exhaustive()
    }
}

/// Whether a file's name is one of Antigravity's programs that carry the
/// client: `language_server` or `language-server` (with or without a
/// platform suffix or `.exe`) or `agy`/`agy.exe`.
pub fn is_program(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_lowercase();
    name.starts_with("language_server")
        || name.starts_with("language-server")
        || name == "agy"
        || name == "agy.exe"
}

fn skipped(name: &str) -> bool {
    name == "node_modules" || name == "locales" || name.ends_with(".lproj")
}

/// The programs under `dir`, symlinked folders inside it not followed (an
/// install folder that is itself a symlink is searched, as its own app).
fn walk(dir: &Path, depth: usize, budget: &mut usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let Ok(kind) = entry.file_type() else { continue };
        let name = entry.file_name();
        if kind.is_dir() {
            if depth < MAX_DEPTH && !skipped(&name.to_string_lossy().to_ascii_lowercase()) {
                walk(&entry.path(), depth + 1, budget, out);
            }
        } else if kind.is_file() && is_program(&name) {
            out.push(entry.path());
        }
    }
}

fn seen(path: &Path) -> Option<Seen> {
    // A CLI on the PATH is often a symlink; the file it points to is read.
    let meta = std::fs::metadata(path).ok()?;
    (meta.is_file() && (MIN_SIZE..=MAX_SIZE).contains(&meta.len())).then(|| Seen {
        path: path.to_path_buf(),
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

impl Sources {
    pub fn new(folders: Vec<PathBuf>, programs: Vec<PathBuf>, pins: Pins) -> Self {
        Sources { folders, programs, pins, last: Mutex::new(None) }
    }

    /// Antigravity's places on this computer ([`mac_places`],
    /// [`windows_places`]), for the pinned client.
    pub fn system(home: &Path) -> Self {
        let path: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        #[cfg(windows)]
        let (folders, programs) = {
            let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
            windows_places(
                var("LOCALAPPDATA").as_deref(),
                var("PROGRAMFILES").as_deref(),
                var("PROGRAMFILES(X86)").as_deref(),
                home,
                &path,
            )
        };
        #[cfg(not(windows))]
        let (folders, programs) = mac_places(home, &path);
        Sources::new(folders, programs, Pins::antigravity())
    }

    /// The programs there now: those in the install folders first, then
    /// the CLI, each file once.
    fn look(&self) -> Vec<Seen> {
        let mut found = Vec::new();
        let mut budget = MAX_ENTRIES;
        for folder in &self.folders {
            walk(folder, 0, &mut budget, &mut found);
        }
        found.extend(self.programs.iter().cloned());
        let mut out: Vec<Seen> = Vec::new();
        let mut real = std::collections::HashSet::new();
        for path in found {
            let Some(s) = seen(&path) else { continue };
            let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if real.insert(canonical) {
                out.push(s);
            }
        }
        out
    }

    /// The pinned client from the first program that holds it.
    pub fn find(&self) -> Result<OAuthClient, Miss> {
        let now = self.look();
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((before, found)) = last.as_ref() {
            if *before == now {
                return found.clone();
            }
        }
        if now.is_empty() {
            *last = Some((now, Err(Miss::NotInstalled)));
            return Err(Miss::NotInstalled);
        }
        let mut failed = false;
        for program in &now {
            crate::app_dir::record(&program.path);
            match std::fs::File::open(&program.path).and_then(|f| scan(f, &self.pins, CHUNK)) {
                Ok(Some(client)) => {
                    *last = Some((now, Ok(client.clone())));
                    return Ok(client);
                }
                Ok(None) => {}
                Err(_) => failed = true,
            }
        }
        // A file that could not be read now (Antigravity updating itself)
        // is read again next time.
        *last = if failed { None } else { Some((now, Err(Miss::Unrecognized))) };
        Err(Miss::Unrecognized)
    }
}

/// On macOS: the desktop app and the IDE, in `/Applications` and in
/// `~/Applications`; the `agy` CLI on the `PATH`, in `~/.local/bin` (where
/// its installer puts it) and in Homebrew's folders.
pub fn mac_places(home: &Path, path: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let apps = ["Antigravity.app", "Antigravity IDE.app"];
    let folders = [PathBuf::from("/Applications"), home.join("Applications")]
        .iter()
        .flat_map(|dir| apps.iter().map(move |app| dir.join(app)))
        .collect();
    let mut programs: Vec<PathBuf> = path.iter().map(|dir| dir.join("agy")).collect();
    for dir in [home.join(".local/bin"), "/opt/homebrew/bin".into(), "/usr/local/bin".into()] {
        programs.push(dir.join("agy"));
    }
    (folders, programs)
}

/// On Windows: the desktop app and the IDE where their installers put them
/// (`%LOCALAPPDATA%\Programs`, `%PROGRAMFILES%`, `%PROGRAMFILES(X86)%`);
/// `agy.exe` on the `PATH`, in `%LOCALAPPDATA%\agy\bin` (where its
/// installer puts it) and in `~\.local\bin`. A variable that is not set is
/// left out.
pub fn windows_places(
    local_app_data: Option<&Path>,
    program_files: Option<&Path>,
    program_files_x86: Option<&Path>,
    home: &Path,
    path: &[PathBuf],
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let apps = ["Antigravity", "Antigravity IDE"];
    let bases = [
        local_app_data.map(|d| d.join("Programs")),
        program_files.map(Path::to_path_buf),
        program_files_x86.map(Path::to_path_buf),
    ];
    let folders =
        bases.iter().flatten().flat_map(|dir| apps.iter().map(move |app| dir.join(app))).collect();
    let mut programs: Vec<PathBuf> = path.iter().map(|dir| dir.join("agy.exe")).collect();
    if let Some(local) = local_app_data {
        programs.push(local.join("agy").join("bin").join("agy.exe"));
    }
    programs.push(home.join(".local").join("bin").join("agy.exe"));
    (folders, programs)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fake clients, put together at run time so that no literal here looks
    // like a client to a secret scanner.
    const DOMAIN: [&str; 2] = [".apps.", "googleusercontent.com"];

    fn id() -> String {
        ["123456789012-fakeclientfakeclient", "fakeclient00", DOMAIN[0], DOMAIN[1]].concat()
    }

    fn secret() -> String {
        ["GOC", "SPX-fakeSecretForTests_only-0"].concat()
    }

    /// Strings packed as a program packs them: no separator, digits and
    /// letters right before the id and right after the secret, and other
    /// clients beside the pinned one.
    fn packed() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"\x00\x01runtime error 4242");
        let other = ["999999999999-otherclientotherclient", "otherclie", DOMAIN[0], DOMAIN[1]];
        out.extend_from_slice(other.concat().as_bytes());
        out.extend_from_slice(["GOC", "SPX-anotherSecretThatIsNotPinned"].concat().as_bytes());
        out.extend_from_slice(b"value 77");
        out.extend_from_slice(id().as_bytes());
        out.extend_from_slice(b"http://x");
        out.extend_from_slice(secret().as_bytes());
        out.extend_from_slice(b"Abc_def-ghi more text");
        out
    }

    #[test]
    fn the_pinned_client_is_found_among_packed_strings() {
        let pins = Pins::of(&id(), &secret());
        let client = scan(&packed()[..], &pins, CHUNK).unwrap().unwrap();
        assert_eq!(client.id(), id());
        assert_eq!(client.secret(), secret());
    }

    #[test]
    fn other_pins_find_nothing_and_nothing_is_found_without_both_halves() {
        assert!(scan(&packed()[..], &Pins::of(&id(), "x"), CHUNK).unwrap().is_none());
        let no_secret: Vec<u8> = packed()
            .windows(secret().len())
            .position(|w| w == secret().as_bytes())
            .map(|at| [&packed()[..at], &packed()[at + secret().len()..]].concat())
            .unwrap();
        assert!(scan(&no_secret[..], &Pins::of(&id(), &secret()), CHUNK).unwrap().is_none());
    }

    #[test]
    fn a_string_cut_between_two_reads_is_still_found() {
        let pins = Pins::of(&id(), &secret());
        let data = packed();
        // Every cut point, with reads as small as the overlap allows.
        for pad in 0..OVERLAP {
            let mut padded = vec![0u8; pad];
            padded.extend_from_slice(&data);
            let client = scan(&padded[..], &pins, OVERLAP).unwrap();
            assert_eq!(
                client.map(|c| c.id().to_string()).as_deref(),
                Some(id().as_str()),
                "pad {pad}"
            );
        }
    }

    #[test]
    fn the_pins_are_sha256_in_hex_and_nothing_else() {
        assert_eq!(Pins::from_hex(&"ab".repeat(32), &"CD".repeat(32)).unwrap().id, [0xab; 32]);
        assert!(Pins::from_hex("abc", &"cd".repeat(32)).is_none());
        assert!(Pins::from_hex(&"zz".repeat(32), &"cd".repeat(32)).is_none());
        let _ = Pins::antigravity();
        assert_eq!(format!("{:?}", Pins::of(&id(), &secret())), "Pins(…)");
    }

    #[test]
    fn the_program_names() {
        for yes in [
            "language_server",
            "language-server_macos_arm",
            "language_server.exe",
            "language_server_macos_arm",
            "Language_Server_Windows_x64.exe",
            "agy",
            "AGY.EXE",
        ] {
            assert!(is_program(OsStr::new(yes)), "{yes}");
        }
        for no in ["agy.js", "language.exe", "Antigravity.exe", "agyx"] {
            assert!(!is_program(OsStr::new(no)), "{no}");
        }
    }
}
