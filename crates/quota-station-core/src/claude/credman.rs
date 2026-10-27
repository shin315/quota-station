// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Claude Code's login items in Windows Credential Manager, the store its
//! Windows build (2.1.311) calls `windows-credman` and uses when
//! `CLAUDE_CODE_FORCE_WINDOWS_CREDMAN=1` or its `tengu_windows_credman`
//! flag is on ([`super::cc`] decides when). Read from its code, not from
//! documentation, which names only the file:
//!
//! - an item's target is `<service>/<account>`, its account
//!   `claude-code-user`, its value the login's JSON as UTF-8;
//! - a value over [`CHUNK`] bytes is kept as base64 in pieces of [`CHUNK`]
//!   characters, `<target>#0` to `#n-1`, with `<target>#m` holding
//!   `{"n":n,"l":length}` written after them, and `<target>#p` holding how
//!   many pieces may be there while a write or a removal is under way;
//! - a read takes the pieces when `#m` is there, else the item itself.
//!
//! [`CcItems`] is that store as a [`SecretStore`], over any [`Credentials`],
//! so the tests run it on every system with credentials kept in memory.

use std::io;
use std::time::Duration;

use base64::Engine;
use serde_json::{json, Value};

use super::keychain::KeychainError;
use crate::os::{Credentials, SecretStore};

/// The longest value kept whole, and the length of each piece.
pub const CHUNK: usize = 2400;
/// The most pieces one value is cut into.
pub const MAX_CHUNKS: usize = 256;

/// Claude Code's item for its default folder, or one for a
/// `CLAUDE_CONFIG_DIR` (`-` and 8 hex digits after it).
pub fn is_claude_code(service: &str) -> bool {
    let base = super::cc::ACTIVE_SERVICE;
    service == base
        || service
            .strip_prefix(base)
            .and_then(|r| r.strip_prefix('-'))
            .is_some_and(|hex| hex.len() == 8 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The target Bun gives a service and an account.
pub fn target(service: &str, account: &str) -> String {
    format!("{service}/{account}")
}

/// Claude Code's items, kept in `creds`.
#[derive(Debug, Clone)]
pub struct CcItems<C> {
    creds: C,
}

/// One item's name in Claude Code's terms, with its pieces.
struct Item<'a> {
    service: &'a str,
    account: &'a str,
}

impl Item<'_> {
    fn base(&self) -> String {
        target(self.service, self.account)
    }

    fn part(&self, suffix: impl std::fmt::Display) -> (String, String) {
        let account = format!("{}#{suffix}", self.account);
        (target(self.service, &account), account)
    }
}

/// `#m`'s pieces and their length, when it reads as Claude Code checks it.
fn meta(value: &[u8]) -> Option<(usize, usize)> {
    let v: Value = serde_json::from_slice(value).ok()?;
    let n = usize::try_from(v.get("n")?.as_u64()?).ok()?;
    let l = usize::try_from(v.get("l")?.as_u64()?).ok()?;
    (n > 0 && n <= MAX_CHUNKS && l > 0 && l <= n * CHUNK).then_some((n, l))
}

fn failed(e: io::Error) -> KeychainError {
    KeychainError::Failed { code: e.raw_os_error(), message: e.to_string() }
}

fn inconsistent(what: &str) -> KeychainError {
    KeychainError::Failed { code: None, message: format!("chunked item inconsistent: {what}") }
}

impl<C: Credentials> CcItems<C> {
    pub fn new(creds: C) -> Self {
        CcItems { creds }
    }

    fn read(&self, target: &str) -> Result<Option<Vec<u8>>, KeychainError> {
        self.creds.read(target).map_err(failed)
    }

    fn write(&self, (target, account): &(String, String), value: &[u8]) -> io::Result<()> {
        self.creds.write(target, account, value)
    }

    fn pieces(&self, item: &Item) -> Result<Option<(usize, usize)>, KeychainError> {
        let (target, _) = item.part("m");
        Ok(self.read(&target)?.as_deref().and_then(meta))
    }

    fn value(&self, item: &Item) -> Result<Option<String>, KeychainError> {
        let Some((n, l)) = self.pieces(item)? else {
            return match self.read(&item.base())? {
                Some(bytes) => String::from_utf8(bytes)
                    .map(Some)
                    .map_err(|_| inconsistent("the value is not text")),
                None => Ok(None),
            };
        };
        let mut joined = String::with_capacity(l);
        for i in 0..n {
            let piece = self.read(&item.part(i).0)?.ok_or_else(|| inconsistent("a piece"))?;
            joined.push_str(&String::from_utf8(piece).map_err(|_| inconsistent("a piece"))?);
        }
        if joined.len() != l {
            return Err(inconsistent("its length"));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(joined.as_bytes())
            .map_err(|_| inconsistent("its base64"))?;
        String::from_utf8(bytes).map(Some).map_err(|_| inconsistent("the value is not text"))
    }
}

impl<C: Credentials> SecretStore for CcItems<C> {
    fn get_within(
        &self,
        service: &str,
        account: &str,
        _wait: Duration,
    ) -> Result<Option<String>, KeychainError> {
        self.value(&Item { service, account })
    }

    fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError> {
        self.value(&Item { service, account })
    }

    fn exists(&self, service: &str, account: &str) -> Result<bool, KeychainError> {
        let item = Item { service, account };
        Ok(self.pieces(&item)?.is_some() || self.read(&item.base())?.is_some())
    }

    /// As Claude Code writes: a value that fits goes whole, and any pieces
    /// of an older one go; a longer one goes in pieces, `#m` after them,
    /// then the whole item and the pieces left over go. `#p` says how many
    /// pieces there may be until the write is done.
    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), KeychainError> {
        let item = Item { service, account };
        let old = self.pieces(&item)?.map_or(0, |(n, _)| n);
        let pending = |n: usize| self.write(&item.part("p"), n.to_string().as_bytes());
        if value.len() <= CHUNK {
            let base = (item.base(), account.to_string());
            self.write(&base, value.as_bytes()).map_err(failed)?;
            if old > 0 {
                pending(old).map_err(failed)?;
                self.creds.delete(&item.part("m").0).map_err(failed)?;
                for i in 0..old {
                    let _ = self.creds.delete(&item.part(i).0);
                }
                let _ = self.creds.delete(&item.part("p").0);
            }
            return Ok(());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(value.as_bytes());
        let n = encoded.len().div_ceil(CHUNK);
        if n > MAX_CHUNKS {
            return Err(KeychainError::Failed {
                code: None,
                message: format!("{} bytes is too large for Credential Manager", value.len()),
            });
        }
        pending(old.max(n)).map_err(failed)?;
        for (i, piece) in encoded.as_bytes().chunks(CHUNK).enumerate() {
            self.write(&item.part(i), piece).map_err(failed)?;
        }
        let m = json!({ "n": n, "l": encoded.len() }).to_string();
        self.write(&item.part("m"), m.as_bytes()).map_err(failed)?;
        let _ = self.creds.delete(&item.base());
        for i in n..old {
            let _ = self.creds.delete(&item.part(i).0);
        }
        let _ = self.creds.delete(&item.part("p").0);
        Ok(())
    }

    /// The item, its pieces and `#m`, then any piece `#p` or a look past
    /// the last says is still there.
    fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError> {
        let item = Item { service, account };
        let known = self.pieces(&item)?.map_or(0, |(n, _)| n);
        self.creds.delete(&item.base()).map_err(failed)?;
        for i in 0..known {
            self.creds.delete(&item.part(i).0).map_err(failed)?;
        }
        self.creds.delete(&item.part("m").0).map_err(failed)?;
        let pending = self
            .read(&item.part("p").0)
            .ok()
            .flatten()
            .and_then(|p| String::from_utf8(p).ok()?.trim().parse::<usize>().ok())
            .map_or(0, |p| p.min(MAX_CHUNKS));
        let mut last = known.max(pending);
        if last == 0 {
            while last < MAX_CHUNKS && self.read(&item.part(last).0).ok().flatten().is_some() {
                last += 1;
            }
        }
        for i in known..last {
            let _ = self.creds.delete(&item.part(i).0);
        }
        let _ = self.creds.delete(&item.part("p").0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    /// Credential Manager in memory: target to (account, value).
    #[derive(Debug, Clone, Default)]
    struct Memory(Arc<Mutex<Kept>>);

    type Kept = BTreeMap<String, (String, Vec<u8>)>;

    impl Credentials for Memory {
        fn read(&self, target: &str) -> io::Result<Option<Vec<u8>>> {
            Ok(self.0.lock().unwrap().get(target).map(|(_, v)| v.clone()))
        }
        fn write(&self, target: &str, account: &str, blob: &[u8]) -> io::Result<()> {
            assert!(blob.len() <= 2560, "Credential Manager keeps 2 560 bytes at most");
            self.0.lock().unwrap().insert(target.into(), (account.into(), blob.to_vec()));
            Ok(())
        }
        fn delete(&self, target: &str) -> io::Result<()> {
            self.0.lock().unwrap().remove(target);
            Ok(())
        }
    }

    impl Memory {
        fn targets(&self) -> Vec<String> {
            self.0.lock().unwrap().keys().cloned().collect()
        }
    }

    const SERVICE: &str = "Claude Code-credentials";
    const USER: &str = "claude-code-user";

    fn long_login() -> String {
        let secrets = "x".repeat(6000);
        format!(r#"{{"claudeAiOauth":{{"accessToken":"a"}},"mcpOAuth":{{"s":"{secrets}"}}}}"#)
    }

    #[test]
    fn claude_codes_services_are_told_from_the_apps() {
        assert!(is_claude_code("Claude Code-credentials"));
        assert!(is_claude_code("Claude Code-credentials-0123abcd"));
        assert!(!is_claude_code("Claude Code-credentials-0123abc"));
        assert!(!is_claude_code("Claude Code-credentials-0123abcz"));
        assert!(!is_claude_code("com.shin315.quota-station.claude"));
    }

    #[test]
    fn a_short_login_is_one_item_named_as_bun_names_it() {
        let mem = Memory::default();
        let items = CcItems::new(mem.clone());
        assert_eq!(items.get(SERVICE, USER).unwrap(), None);
        items.set(SERVICE, USER, r#"{"claudeAiOauth":{"accessToken":"a"}}"#).unwrap();
        assert_eq!(mem.targets(), ["Claude Code-credentials/claude-code-user"]);
        let (account, _) =
            mem.0.lock().unwrap()["Claude Code-credentials/claude-code-user"].clone();
        assert_eq!(account, USER);
        assert!(items.exists(SERVICE, USER).unwrap());
        assert_eq!(
            items.get(SERVICE, USER).unwrap().as_deref(),
            Some(r#"{"claudeAiOauth":{"accessToken":"a"}}"#)
        );
    }

    #[test]
    fn a_long_login_is_cut_into_pieces_and_read_back_whole() {
        let mem = Memory::default();
        let items = CcItems::new(mem.clone());
        items.set(SERVICE, USER, "short").unwrap();
        let login = long_login();
        items.set(SERVICE, USER, &login).unwrap();
        let base = "Claude Code-credentials/claude-code-user";
        let encoded = base64::engine::general_purpose::STANDARD.encode(&login);
        let n = encoded.len().div_ceil(CHUNK);
        let mut expected: Vec<String> = (0..n).map(|i| format!("{base}#{i}")).collect();
        expected.push(format!("{base}#m"));
        expected.sort();
        assert_eq!(mem.targets(), expected, "the whole item and #p are gone");
        let m = mem.read(&format!("{base}#m")).unwrap().unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&m).unwrap(),
            json!({ "n": n, "l": encoded.len() })
        );
        assert_eq!(items.get(SERVICE, USER).unwrap().as_deref(), Some(login.as_str()));
        assert!(items.exists(SERVICE, USER).unwrap());

        // Back to a short one: the pieces go, as Claude Code clears them.
        items.set(SERVICE, USER, "short again").unwrap();
        assert_eq!(mem.targets(), [base]);
        assert_eq!(items.get(SERVICE, USER).unwrap().as_deref(), Some("short again"));
    }

    #[test]
    fn a_piece_missing_is_an_error_not_another_login() {
        let mem = Memory::default();
        let items = CcItems::new(mem.clone());
        items.set(SERVICE, USER, &long_login()).unwrap();
        mem.delete("Claude Code-credentials/claude-code-user#1").unwrap();
        assert!(items.get(SERVICE, USER).is_err());
    }

    #[test]
    fn a_removal_takes_every_piece_even_one_left_over() {
        let mem = Memory::default();
        let items = CcItems::new(mem.clone());
        items.set(SERVICE, USER, &long_login()).unwrap();
        // A write cut short left a piece past the last one #m counts.
        let n = mem.targets().len() - 1;
        let stray = format!("Claude Code-credentials/claude-code-user#{n}");
        mem.write(&stray, "x", b"stray").unwrap();
        mem.write(
            "Claude Code-credentials/claude-code-user#p",
            "x",
            (n + 1).to_string().as_bytes(),
        )
        .unwrap();
        items.delete(SERVICE, USER).unwrap();
        assert!(mem.targets().is_empty(), "{:?}", mem.targets());
        items.delete(SERVICE, USER).unwrap();
        assert!(!items.exists(SERVICE, USER).unwrap());
    }

    #[test]
    fn a_login_too_large_for_256_pieces_is_refused_and_nothing_changes() {
        let mem = Memory::default();
        let items = CcItems::new(mem.clone());
        items.set(SERVICE, USER, "kept").unwrap();
        let huge = "y".repeat(CHUNK * MAX_CHUNKS);
        assert!(items.set(SERVICE, USER, &huge).is_err());
        assert_eq!(items.get(SERVICE, USER).unwrap().as_deref(), Some("kept"));
    }
}
