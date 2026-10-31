// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Onur Cetinkol (claude-swap)
// SPDX-License-Identifier: MIT

// Where claude-swap keeps each account is read as claude-swap reads it
// (`switcher.py`, `credentials.py`, MIT, https://github.com/realiti4/claude-swap):
// `sequence.json` for the slots, `configs/.claude-config-N-<email>.json` for
// `oauthAccount`, and the credential from `credentials/.creds-N-<email>.enc`
// when that file is there and decodes, else the Keychain item
// `claude-swap` / `account-N-<email>`.

//! `Nhập từ claude-swap`: carries every claude-swap account over to the app,
//! under the same slot number, without a new login. It only reads
//! claude-swap's folder and Keychain items, never writes them.
//!
//! Running it again changes nothing, except that a credential claude-swap
//! holds a newer copy of (a later `expiresAt`) replaces the app's: an
//! account the owner renamed in the app stays as the owner left it, and an
//! account already in the app under another slot is not added twice. It
//! also copies claude-swap's last readings, only when the app has none yet.
// @private:begin
//!
//! Each account comes in the rotation: claude-swap's own `disabled` is not
//! carried over, as the old switcher never read it, and one the owner held
//! out of the rotation in the app stays held out. It also copies the
//! rotation file, the `prime` flag and its times, each only when the app
//! has none yet.
// @private:end

use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::cc;
use super::locks::{AppLock, LockError};
use super::session;
use super::store::{self, Account, Kind, Store, StoreError};
use super::usage::{self, Row, Scoped, Win};
#[cfg(feature = "author-tools")]
use super::Paths;

/// claude-swap's folder on macOS.
pub const SOURCE: &str = ".claude-swap-backup";
/// claude-swap's Keychain service for its accounts.
const SWAP_SERVICE: &str = "claude-swap";
/// The old switcher's priming times.
#[cfg(feature = "author-tools")]
const PRIME_STATE: &str = ".local/state/cswap-prefer-team";
#[cfg(feature = "author-tools")]
const PRIME_FLAG: &str = ".config/cswap-prefer-team/prime";

pub fn source(home: &Path) -> PathBuf {
    home.join(SOURCE)
}

/// What an import did, by slot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub added: Vec<u32>,
    /// A newer credential replaced the app's.
    pub updated: Vec<u32>,
    pub unchanged: Vec<u32>,
    /// Not carried over, and why: `slotTaken` (another account has that
    /// slot in the app), `elsewhere` (the account is in the app under
    /// another slot), `apiKey`, `noConfig`, `noCredential`, `keychain`.
    pub skipped: Vec<(u32, String)>,
    #[cfg(feature = "author-tools")]
    pub rotation: bool,
    #[cfg(feature = "author-tools")]
    pub prime: bool,
}

#[derive(Debug)]
pub enum ImportError {
    /// No `sequence.json`: claude-swap has no accounts here.
    NotFound,
    /// `sequence.json` is there but does not read.
    Corrupt,
    Lock(LockError),
    Store(StoreError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::NotFound => write!(f, "claude-swap has no accounts on this Mac"),
            ImportError::Corrupt => write!(f, "claude-swap's sequence.json does not read"),
            ImportError::Lock(e) => write!(f, "{e}"),
            ImportError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<LockError> for ImportError {
    fn from(e: LockError) -> Self {
        ImportError::Lock(e)
    }
}

impl From<StoreError> for ImportError {
    fn from(e: StoreError) -> Self {
        ImportError::Store(e)
    }
}

/// claude-swap has accounts on this Mac to import.
pub fn available(home: &Path) -> bool {
    source(home).join("sequence.json").is_file()
}

/// One claude-swap account as found.
struct Found {
    slot: u32,
    email: String,
    record: Value,
}

fn read_sequence(dir: &Path) -> Result<Vec<Found>, ImportError> {
    let text = match std::fs::read_to_string(dir.join("sequence.json")) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(ImportError::NotFound),
        Err(_) => return Err(ImportError::Corrupt),
    };
    let data: Value = serde_json::from_str(&text).map_err(|_| ImportError::Corrupt)?;
    let accounts = data.get("accounts").and_then(Value::as_object).ok_or(ImportError::Corrupt)?;
    let mut found: Vec<Found> = accounts
        .iter()
        .filter_map(|(key, record)| {
            let slot = key.parse::<u32>().ok().filter(|s| *s > 0)?;
            let email = record.get("email").and_then(Value::as_str).unwrap_or("").to_string();
            Some(Found { slot, email, record: record.clone() })
        })
        .collect();
    found.sort_by_key(|f| f.slot);
    Ok(found)
}

/// The account's credential: the `.enc` file when it decodes to something,
/// else claude-swap's Keychain item.
fn read_credential(dir: &Path, store: &Store, f: &Found) -> Result<Option<String>, ()> {
    let enc = dir.join("credentials").join(format!(".creds-{}-{}.enc", f.slot, f.email));
    if let Ok(text) = std::fs::read_to_string(&enc) {
        if let Ok(bytes) = STANDARD.decode(text.trim()) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                if !decoded.is_empty() {
                    return Ok(Some(decoded));
                }
            }
        }
    }
    store
        .security
        .get(SWAP_SERVICE, &format!("account-{}-{}", f.slot, f.email))
        .map(|c| c.filter(|c| !c.is_empty()))
        .map_err(|_| ())
}

fn read_oauth(dir: &Path, f: &Found) -> Option<Value> {
    let file = dir.join("configs").join(format!(".claude-config-{}-{}.json", f.slot, f.email));
    let text = std::fs::read_to_string(file).ok()?;
    let config: Value = serde_json::from_str(&text).ok()?;
    config.get("oauthAccount").filter(|o| o.is_object()).cloned()
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Imports from claude-swap's folder at `dir` (normally `~/.claude-swap-backup`).
pub fn import(store: &Store, dir: &Path) -> Result<Imported, ImportError> {
    let paths = &store.paths;
    let _app = AppLock::take(paths)?;
    let found = read_sequence(dir)?;
    let mut list = store.load()?;
    let mut done = Imported::default();
    let now = OffsetDateTime::now_utc();
    let mut skip = |slot: u32, why: &str| done.skipped.push((slot, why.to_string()));

    let mut imported: Vec<(u32, String)> = Vec::new();
    for f in &found {
        if f.record.get("kind").and_then(Value::as_str) == Some("api_key") {
            skip(f.slot, "apiKey");
            continue;
        }
        let credential = match read_credential(dir, store, f) {
            Ok(Some(c)) if cc::usable(&c) => c,
            Ok(_) => {
                skip(f.slot, "noCredential");
                continue;
            }
            Err(()) => {
                skip(f.slot, "keychain");
                continue;
            }
        };
        let token = cc::refresh_token(&credential).is_none_or(|r| r.is_empty());
        let oauth = match read_oauth(dir, f) {
            Some(o) => o,
            // claude-swap's own record for a setup token.
            None if token && !f.email.is_empty() => json!({
                "emailAddress": f.email,
                "accountUuid": "",
                "organizationUuid": null,
                "organizationName": null,
            }),
            None => {
                skip(f.slot, "noConfig");
                continue;
            }
        };
        let org = match text(&f.record, "organizationUuid") {
            "" => text(&oauth, "organizationUuid").to_string(),
            o => o.to_string(),
        };
        let identity = store::identity(&f.email, &org);
        match list.get(f.slot) {
            Some(a) if a.identity != identity => skip(f.slot, "slotTaken"),
            Some(_) => {
                let ours = store.credential(f.slot).ok().flatten();
                let newer = match ours.as_deref() {
                    None => true,
                    Some(o) if o == credential => false,
                    Some(o) => {
                        cc::expires_at(&credential).unwrap_or(0) > cc::expires_at(o).unwrap_or(0)
                    }
                };
                if newer {
                    if store.set_credential(f.slot, &credential).is_err() {
                        skip(f.slot, "keychain");
                        continue;
                    }
                    session::refresh_profile(store, f.slot);
                    done.updated.push(f.slot);
                } else {
                    done.unchanged.push(f.slot);
                }
                imported.push((f.slot, identity));
            }
            None if list.find(&identity).is_some() => skip(f.slot, "elsewhere"),
            None => {
                if store.set_credential(f.slot, &credential).is_err()
                    || store.set_oauth_account(f.slot, &oauth).is_err()
                {
                    let _ = store.forget(f.slot);
                    skip(f.slot, "keychain");
                    continue;
                }
                let alias = f
                    .record
                    .get("alias")
                    .and_then(Value::as_str)
                    .and_then(|a| store::clean_alias(a).ok().flatten());
                list.put(Account {
                    slot: f.slot,
                    alias,
                    #[cfg(feature = "author-tools")]
                    disabled: false,
                    kind: if token { Kind::SetupToken } else { Kind::Login },
                    account_uuid: text(&oauth, "accountUuid").to_string(),
                    organization_uuid: org,
                    identity: identity.clone(),
                    added: now.format(&Rfc3339).unwrap_or_default(),
                });
                done.added.push(f.slot);
                imported.push((f.slot, identity));
            }
        }
    }
    store.save(&list)?;

    seed_usage(store, dir, &found, &imported, &done.updated);
    #[cfg(feature = "author-tools")]
    {
        done.rotation = copy_rotation(paths);
        done.prime = copy_prime(paths);
    }
    Ok(done)
}

/// claude-swap's last readings for the imported accounts the app has no
/// reading of yet, and a fresh start for those whose login was replaced.
fn seed_usage(
    store: &Store,
    dir: &Path,
    found: &[Found],
    imported: &[(u32, String)],
    updated: &[u32],
) {
    let theirs: Value = std::fs::read_to_string(dir.join("cache/usage.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|v: &Value| v.get("schemaVersion").and_then(Value::as_u64) == Some(2))
        .unwrap_or(Value::Null);
    let mut cache = usage::load(store);
    for (slot, identity) in imported {
        let key = slot.to_string();
        if updated.contains(slot) {
            if let Some(row) = cache.accounts.get_mut(&key) {
                row.new_login();
            }
        }
        if cache
            .accounts
            .get(&key)
            .is_some_and(|r| r.identity == *identity && r.last_good.is_some())
        {
            continue;
        }
        let Some(f) = found.iter().find(|f| f.slot == *slot) else { continue };
        let Some(row) = theirs.pointer(&format!("/accounts/{key}")) else { continue };
        // claude-swap's own guard: a row written for another account is not used.
        if text(row, "email") != f.email {
            continue;
        }
        let win = |v: Option<&Value>| -> Option<Win> {
            let v = v?;
            Some(Win {
                pct: v.get("pct")?.as_f64()?,
                resets_at: v.get("resets_at").and_then(Value::as_str).map(String::from),
            })
        };
        let good = row.get("lastGood");
        let reading = good.map(|g| usage::Reading {
            five_hour: win(g.get("five_hour")),
            seven_day: win(g.get("seven_day")),
            scoped: g
                .get("scoped")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|s| {
                            Some(Scoped {
                                name: s.get("name")?.as_str()?.to_string(),
                                pct: s.get("pct")?.as_f64()?,
                                resets_at: s
                                    .get("resets_at")
                                    .and_then(Value::as_str)
                                    .map(String::from),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        });
        cache.accounts.insert(
            key,
            Row {
                identity: identity.clone(),
                last_good: reading,
                fetched_at: row.get("fetchedAt").and_then(Value::as_f64),
                ..Row::default()
            },
        );
    }
    let _ = usage::save(store, &cache);
}

/// The old switcher's rotation file, when the app has none.
#[cfg(feature = "author-tools")]
fn copy_rotation(paths: &Paths) -> bool {
    let own = paths.rotation();
    let legacy = crate::rotation::path(&paths.home);
    if own.exists() {
        return false;
    }
    match std::fs::read_to_string(&legacy) {
        Ok(text) => super::write_private(&own, &text).is_ok(),
        Err(_) => false,
    }
}

/// The old switcher's priming flag and times, when the app has none.
#[cfg(feature = "author-tools")]
fn copy_prime(paths: &Paths) -> bool {
    if paths.prime_flag().exists() || !paths.home.join(PRIME_FLAG).exists() {
        return false;
    }
    let state = paths.prime_state();
    if super::private_dir(&state).is_ok() {
        for entry in std::fs::read_dir(paths.home.join(PRIME_STATE)).into_iter().flatten().flatten()
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("prime-") {
                let _ = std::fs::copy(entry.path(), state.join(&name));
            }
        }
    }
    super::write_private(&paths.prime_flag(), "").is_ok()
}
