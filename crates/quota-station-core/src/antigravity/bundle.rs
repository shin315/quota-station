// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The Antigravity accounts the app logs in, carried to another computer:
//! `quota-station antigravity export` writes them as a bundle, and
//! `quota-station antigravity import` puts a bundle into the app's store
//! there, so an account logged in once on a Mac is switched to on a Linux
//! machine (or Linux in WSL) without a new login. The format is in
//! `docs/linux.md`:
//!
//! - `manifest.json`: `{"format": "quota-station-antigravity-accounts", "version": 1}`;
//! - `accounts.json`: the account index ([`super::index`]), in the order
//!   whose numbers `status` and `switch N` use;
//! - `oauth-client.json`: the OAuth client the tokens were got with
//!   ([`super::client`]), which the other computer refreshes them with;
//! - `tokens/<id>.json`: each account's [`TokenData`], as the app keeps it
//!   in its Keychain item.
//!
//! A bundle is a folder (mode 700, its files 600) or the same files in a tar
//! stream. It holds every account's refresh token: whoever has it can use
//! the accounts, so it is kept like a password and removed once imported.
//! Nothing here prints a token or the client; the commands print account
//! numbers and emails only.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::client::{self, OAuthClient};
use super::index::{self, AccountIndex, AccountSummary};
use super::store::Keychain;
use super::token::TokenData;
use crate::app_dir::AppDir;

pub const FORMAT: &str = "quota-station-antigravity-accounts";
pub const VERSION: u32 = 1;
pub const MANIFEST: &str = "manifest.json";
pub const TOKENS: &str = "tokens";
/// What one file of a bundle may hold, and all of them together.
const FILE_LIMIT: u64 = 1 << 20;
const TOTAL_LIMIT: u64 = 16 << 20;

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

/// The accounts, their tokens and the OAuth client, checked.
pub struct Bundle {
    pub index: AccountIndex,
    client: OAuthClient,
    tokens: Vec<TokenData>,
}

/// Never prints a token or the client.
impl std::fmt::Debug for Bundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bundle").field("accounts", &self.index.accounts.len()).finish()
    }
}

/// Why a bundle could not be made, read or put in.
#[derive(Debug)]
pub enum BundleError {
    Io(io::Error),
    /// Not a bundle this app reads: why. Never a token or the client.
    Invalid(String),
    /// The app's own files or store: why.
    Store(String),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundleError::Io(e) => write!(f, "{e}"),
            BundleError::Invalid(e) => write!(f, "not an account bundle: {e}"),
            BundleError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for BundleError {}

impl From<io::Error> for BundleError {
    fn from(e: io::Error) -> Self {
        BundleError::Io(e)
    }
}

fn invalid(why: impl Into<String>) -> BundleError {
    BundleError::Invalid(why.into())
}

fn token_name(id: &str) -> String {
    format!("{TOKENS}/{id}.json")
}

impl Bundle {
    /// Every account the app logs in here, with its token, and the client
    /// ([`client::load`]). The index's current account is left out: which
    /// account agy is on is the other computer's own.
    pub fn collect(app: &AppDir, keychain: &Keychain) -> Result<Bundle, BundleError> {
        let mut index = index::load(app).map_err(|e| BundleError::Store(e.to_string()))?;
        if index.accounts.is_empty() {
            return Err(BundleError::Store("no Antigravity accounts to export".into()));
        }
        index.current_account_id = None;
        let client = client::load(app)
            .map_err(|e| BundleError::Store(format!("no OAuth client to export ({e:?})")))?;
        let tokens = (1..)
            .zip(&index.accounts)
            .map(|(n, a)| {
                keychain.read(&a.id).map_err(|e| {
                    BundleError::Store(format!("account {n}: its token cannot be read: {e}"))
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Bundle { index, client, tokens })
    }

    /// The bundle's files by name.
    fn files(&self) -> BTreeMap<String, Vec<u8>> {
        let manifest = Manifest { format: FORMAT.into(), version: VERSION };
        let mut files = BTreeMap::new();
        files.insert(MANIFEST.to_string(), pretty(&manifest));
        files.insert(index::INDEX_FILE.to_string(), pretty(&self.index));
        files.insert(client::FILE.to_string(), self.client.file_text().into_bytes());
        for (a, token) in self.index.accounts.iter().zip(&self.tokens) {
            files.insert(token_name(&a.id), pretty(token));
        }
        files
    }

    /// Checks `files`: the manifest, an index of version 2.0 whose ids are
    /// its emails', a usable client and a token with a refresh token for
    /// each account, of that account's email. Any other file is refused.
    fn from_files(mut files: BTreeMap<String, Vec<u8>>) -> Result<Bundle, BundleError> {
        let mut take = |name: &str| files.remove(name).ok_or_else(|| invalid(format!("no {name}")));
        let manifest: Manifest = serde_json::from_slice(&take(MANIFEST)?)
            .map_err(|_| invalid(format!("{MANIFEST} is not readable")))?;
        if manifest.format != FORMAT || manifest.version != VERSION {
            return Err(invalid(format!(
                "{MANIFEST} names {} version {}, not {FORMAT} version {VERSION}",
                manifest.format, manifest.version
            )));
        }
        let index: AccountIndex = serde_json::from_slice(&take(index::INDEX_FILE)?)
            .map_err(|_| invalid(format!("{} is not readable", index::INDEX_FILE)))?;
        if index.version != index::INDEX_VERSION {
            return Err(invalid(format!("{} is version {}", index::INDEX_FILE, index.version)));
        }
        if index.accounts.is_empty() {
            return Err(invalid("it holds no accounts"));
        }
        let text = String::from_utf8(take(client::FILE)?)
            .map_err(|_| invalid(format!("{} is not text", client::FILE)))?;
        let client =
            client::parse(&text).map_err(|_| invalid(format!("{} is not usable", client::FILE)))?;
        let mut tokens = Vec::new();
        for (n, a) in (1..).zip(&index.accounts) {
            if a.id != index::account_id(&a.email) {
                return Err(invalid(format!("account {n}: its id is not its email's")));
            }
            if index.accounts[..n - 1].iter().any(|b| b.id == a.id) {
                return Err(invalid(format!("account {n} is there twice")));
            }
            let token: TokenData = serde_json::from_slice(&take(&token_name(&a.id))?)
                .map_err(|_| invalid(format!("account {n}: its token is not readable")))?;
            if token.refresh_token.trim().is_empty() {
                return Err(invalid(format!("account {n}: its token has no refresh token")));
            }
            if token
                .email
                .as_deref()
                .is_some_and(|e| !e.trim().eq_ignore_ascii_case(a.email.trim()))
            {
                return Err(invalid(format!("account {n}: its token is another account's")));
            }
            tokens.push(token);
        }
        if let Some(name) = files.keys().next() {
            return Err(invalid(format!("unexpected file {name}")));
        }
        Ok(Bundle { index, client, tokens })
    }

    /// The accounts by number and email.
    pub fn accounts(&self) -> Vec<(usize, String)> {
        (1..).zip(&self.index.accounts).map(|(n, a)| (n, a.email.clone())).collect()
    }

    /// Writes the bundle where `export` was told: a tar stream on `out`
    /// (`-`), a new tar file of mode 600 (a name ending in `.tar`), else
    /// the folder [`Bundle::write_dir`] makes.
    pub fn write_to(&self, to: &str, out: &mut dyn Write) -> Result<(), BundleError> {
        if to == "-" {
            return self.write_tar(out);
        }
        if to.ends_with(".tar") {
            let mut file = crate::os::with_mode(
                std::fs::OpenOptions::new().write(true).create_new(true),
                0o600,
            )
            .open(to)?;
            return self.write_tar(&mut file);
        }
        self.write_dir(Path::new(to))
    }

    /// Reads the bundle `import` was told: a tar stream on `input` (`-`), a
    /// folder, else a tar file.
    pub fn read_from(from: &str, input: &mut dyn Read) -> Result<Bundle, BundleError> {
        let path = Path::new(from);
        if from == "-" {
            Bundle::read_tar(input)
        } else if path.is_dir() {
            Bundle::read_dir(path)
        } else {
            Bundle::read_tar(&mut std::fs::File::open(path)?)
        }
    }

    /// Writes the bundle as the folder `dir`, which must not be there yet
    /// or be empty: mode 700, its files 600, none replaced.
    pub fn write_dir(&self, dir: &Path) -> Result<(), BundleError> {
        if dir.exists() && std::fs::read_dir(dir)?.next().is_some() {
            return Err(BundleError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} is there and not empty", dir.display()),
            )));
        }
        crate::os::owner_dir(&dir.join(TOKENS))?;
        crate::os::set_mode(dir, 0o700)?;
        for (name, bytes) in self.files() {
            let mut file = crate::os::with_mode(
                std::fs::OpenOptions::new().write(true).create_new(true),
                0o600,
            )
            .open(dir.join(&name))?;
            file.write_all(&bytes)?;
            crate::os::sync_all(&file)?;
        }
        Ok(())
    }

    /// Writes the bundle as a tar stream (ustar, regular files of mode 600).
    pub fn write_tar(&self, out: &mut dyn Write) -> Result<(), BundleError> {
        for (name, bytes) in self.files() {
            out.write_all(&tar_header(&name, bytes.len() as u64)?)?;
            out.write_all(&bytes)?;
            out.write_all(&vec![0; padding(bytes.len() as u64)])?;
        }
        out.write_all(&[0; 1024])?;
        out.flush()?;
        Ok(())
    }

    /// Reads and checks the bundle in the folder `dir`.
    pub fn read_dir(dir: &Path) -> Result<Bundle, BundleError> {
        let mut files = BTreeMap::new();
        let mut total = 0;
        for (sub, prefix) in [(dir.to_path_buf(), ""), (dir.join(TOKENS), "tokens/")] {
            for entry in std::fs::read_dir(&sub)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if (prefix.is_empty() && name == TOKENS && kind.is_dir()) || skipped(&name) {
                    continue;
                }
                if !kind.is_file() {
                    return Err(invalid(format!("{prefix}{name} is not a plain file")));
                }
                let bytes = read_capped(std::fs::File::open(entry.path())?, &mut total)?;
                files.insert(format!("{prefix}{name}"), bytes);
            }
        }
        Bundle::from_files(files)
    }

    /// Reads and checks a bundle from a tar stream: regular files and
    /// folders only (a link is refused), names as [`Bundle::write_tar`]
    /// writes them or under `./`.
    pub fn read_tar(input: &mut dyn Read) -> Result<Bundle, BundleError> {
        let mut files = BTreeMap::new();
        let mut total = 0;
        loop {
            let mut header = [0u8; 512];
            input.read_exact(&mut header).map_err(|_| invalid("the tar stream ends early"))?;
            if header.iter().all(|&b| b == 0) {
                break;
            }
            let size = octal(&header[124..136]).ok_or_else(|| invalid("a tar header is bad"))?;
            let name = tar_name(&header)?;
            let kind = header[156];
            let mut body = (&mut *input).take(size + padding(size) as u64);
            match kind {
                b'0' | 0 if !skipped(name.rsplit('/').next().unwrap_or("")) => {
                    let mut part = (&mut body).take(size);
                    let bytes = read_capped(&mut part, &mut total)?;
                    if bytes.len() as u64 != size {
                        return Err(invalid("the tar stream ends early"));
                    }
                    files.insert(name, bytes);
                }
                // Folders, pax headers, and files the bundle never holds.
                b'0' | 0 | b'5' | b'x' | b'g' => {}
                _ => return Err(invalid(format!("{name} is not a plain file"))),
            }
            io::copy(&mut body, &mut io::sink())?;
        }
        Bundle::from_files(files)
    }

    /// Puts the bundle into the app's store here: each account's token in
    /// [`Keychain`], then the index, then `oauth-client.json`. The accounts
    /// already here keep their numbers and their current account; one the
    /// bundle also holds gets its token and name, and loses a standing that
    /// asked for a new login. The others are added in the bundle's order.
    /// Gives each account of the bundle by its number here and email.
    pub fn install(
        &self,
        app: &AppDir,
        keychain: &Keychain,
    ) -> Result<Vec<(usize, String)>, BundleError> {
        let mut here = index::load(app).map_err(|e| BundleError::Store(e.to_string()))?;
        for (a, token) in self.index.accounts.iter().zip(&self.tokens) {
            keychain.save(&a.id, token).map_err(|e| {
                BundleError::Store(format!("the token of {} cannot be kept: {e}", a.email))
            })?;
        }
        for a in &self.index.accounts {
            match here.accounts.iter_mut().find(|h| h.id == a.id) {
                Some(h) => h.name = a.name.clone().or(h.name.take()),
                None => here.accounts.push(AccountSummary { last_used: 0, ..a.clone() }),
            }
        }
        index::save(app, &here).map_err(|e| BundleError::Store(e.to_string()))?;
        for a in &self.index.accounts {
            let _ = index::set_standing(app, &a.id, None);
        }
        app.write_private(&client::path(app), &self.client.file_text())
            .map_err(|e| BundleError::Store(format!("cannot write {}: {e}", client::FILE)))?;
        Ok(self
            .index
            .accounts
            .iter()
            .filter_map(|a| {
                let n = here.accounts.iter().position(|h| h.id == a.id)? + 1;
                Some((n, a.email.clone()))
            })
            .collect())
    }
}

/// Files a copy of the folder may pick up on the way that are no part of it.
fn skipped(name: &str) -> bool {
    name.starts_with("._") || name == ".DS_Store"
}

fn read_capped(input: impl Read, total: &mut u64) -> Result<Vec<u8>, BundleError> {
    let mut bytes = Vec::new();
    input.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
    *total += bytes.len() as u64;
    if bytes.len() as u64 > FILE_LIMIT || *total > TOTAL_LIMIT {
        return Err(invalid("a file is too large"));
    }
    Ok(bytes)
}

fn padding(size: u64) -> usize {
    ((512 - size % 512) % 512) as usize
}

fn octal(field: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(field).ok()?;
    let text = text.trim_matches(|c: char| c == '\0' || c == ' ');
    if text.is_empty() {
        return Some(0);
    }
    u64::from_str_radix(text, 8).ok()
}

fn tar_name(header: &[u8; 512]) -> Result<String, BundleError> {
    let field = |range: std::ops::Range<usize>| {
        let raw = &header[range];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        String::from_utf8(raw[..end].to_vec()).map_err(|_| invalid("a tar name is not text"))
    };
    let (prefix, name) = (field(345..500)?, field(0..100)?);
    let full = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
    Ok(full.trim_start_matches("./").trim_end_matches('/').to_string())
}

fn tar_header(name: &str, size: u64) -> io::Result<[u8; 512]> {
    let mut h = [0u8; 512];
    if name.len() >= 100 {
        return Err(io::Error::other(format!("{name}: too long a name")));
    }
    let mut put = |at: usize, text: &str| h[at..at + text.len()].copy_from_slice(text.as_bytes());
    put(0, name);
    put(100, "0000600\0");
    put(108, "0000000\0");
    put(116, "0000000\0");
    put(124, &format!("{size:011o}\0"));
    put(136, "00000000000\0");
    put(148, "        ");
    put(156, "0");
    put(257, "ustar\0");
    put(263, "00");
    let sum: u32 = h.iter().map(|&b| u32::from(b)).sum();
    h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    Ok(h)
}

/// A part of the bundle as pretty JSON, ending in a newline.
fn pretty(value: &impl Serialize) -> Vec<u8> {
    let mut text = serde_json::to_vec_pretty(value).expect("serializes");
    text.push(b'\n');
    text
}
