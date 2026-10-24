// The session folder follows claude-swap's `cswap run` (`session.py`, MIT,
// https://github.com/realiti4/claude-swap): a `CLAUDE_CONFIG_DIR` per
// account with its `oauthAccount` and links to the owner's settings, the
// login variables taken out of the environment, a newer login Claude Code
// wrote there taken back once the folder is quiet, and the folder left
// alone while a session in it runs. Unlike claude-swap, the login is put in
// the Keychain item Claude Code keys to the folder, never in a file.

//! Running `claude` as one account without changing the active login, for
//! Firstmate and the rotation's priming: `CLAUDE_CONFIG_DIR` points at
//! `claude/sessions/<slot>/`, a folder Claude Code treats as its own.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::cc;
use super::keychain::{user_account, KeychainError};
use super::locks::{AppLock, LockError};
use super::store::{Store, StoreError};
use super::switch;

/// The owner's Claude Code files a session shares, as links.
const SHARED: [&str; 6] =
    ["settings.json", "keybindings.json", "CLAUDE.md", "skills", "commands", "agents"];
/// Variables that would make `claude` use another login than the folder's.
pub const SCRUBBED: [&str; 5] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
    "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
];

/// The account's session folder.
pub fn folder(store: &Store, slot: u32) -> PathBuf {
    store.paths.sessions().join(slot.to_string())
}

/// A Claude Code session runs in the folder now: one of its
/// `sessions/*.json` records names a live process.
pub fn live(dir: &Path) -> bool {
    crate::sessions::count(&dir.join("sessions")).map(|s| s.live + s.unreadable > 0).unwrap_or(true)
}

fn profile_credential(dir: &Path, store: &Store) -> Option<String> {
    let service = cc::profile_service(dir)?;
    store.security.get(&service, &user_account()).ok().flatten()
}

/// Takes a newer login Claude Code wrote in the account's quiet session
/// folder back into the app's copy. Best effort; the caller holds the app's
/// lock.
pub fn adopt_newer(store: &Store, slot: u32) {
    let dir = folder(store, slot);
    if !dir.is_dir() || live(&dir) {
        return;
    }
    let Some(theirs) = profile_credential(&dir, store).filter(|c| cc::usable(c)) else { return };
    let ours = store.credential(slot).ok().flatten();
    let newer = match ours.as_deref() {
        None => true,
        Some(ours) if ours == theirs => false,
        Some(ours) => cc::expires_at(&theirs).unwrap_or(0) > cc::expires_at(ours).unwrap_or(0),
    };
    if newer {
        let _ = store.set_credential(slot, &theirs);
    }
}

/// After the app's copy of the account's login changed: puts it in the
/// quiet session folder, so the folder never holds an older copy of the
/// same login. A running session keeps its own until it ends. Best effort;
/// the caller holds the app's lock.
pub fn refresh_profile(store: &Store, slot: u32) {
    let dir = folder(store, slot);
    if !dir.is_dir() || live(&dir) {
        return;
    }
    let (Some(service), Ok(Some(ours))) = (cc::profile_service(&dir), store.credential(slot))
    else {
        return;
    };
    let _ = store.security.set(&service, &user_account(), &ours);
}

#[derive(Debug)]
pub enum SessionError {
    UnknownSlot,
    NoCredential,
    /// The account is Claude Code's default login, and the caller asked for
    /// a separate session (`--require-session`).
    IsActive,
    /// The folder's path is not plain ASCII.
    Path,
    Lock(LockError),
    Keychain(KeychainError),
    Store(StoreError),
    Io(std::io::Error),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::UnknownSlot => write!(f, "no such account"),
            SessionError::NoCredential => write!(f, "the account has no stored login"),
            SessionError::IsActive => write!(f, "the account is Claude Code's default login now"),
            SessionError::Path => write!(f, "the session folder's path is not plain ASCII"),
            SessionError::Lock(e) => write!(f, "{e}"),
            SessionError::Keychain(e) => write!(f, "{e}"),
            SessionError::Store(e) => write!(f, "{e}"),
            SessionError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<LockError> for SessionError {
    fn from(e: LockError) -> Self {
        SessionError::Lock(e)
    }
}

impl From<KeychainError> for SessionError {
    fn from(e: KeychainError) -> Self {
        SessionError::Keychain(e)
    }
}

impl From<StoreError> for SessionError {
    fn from(e: StoreError) -> Self {
        SessionError::Store(e)
    }
}

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        SessionError::Io(e)
    }
}

/// Makes the account's session folder ready and returns it: its
/// `.claude.json` with the account's `oauthAccount`, the links to the
/// owner's settings, and the newer of the app's and the folder's login in
/// the folder's Keychain item (unless a session runs there).
pub fn prepare(store: &Store, slot: u32) -> Result<PathBuf, SessionError> {
    let _app = AppLock::take(&store.paths)?;
    let list = store.load()?;
    list.get(slot).ok_or(SessionError::UnknownSlot)?;
    let dir = folder(store, slot);
    let service = cc::profile_service(&dir).ok_or(SessionError::Path)?;
    super::private_dir(&dir)?;
    let oauth = store.oauth_account(slot)?.ok_or(SessionError::NoCredential)?;
    write_config(&dir, &oauth)?;
    for name in SHARED {
        let (from, to) = (store.paths.claude_home().join(name), dir.join(name));
        if from.exists() && std::fs::symlink_metadata(&to).is_err() {
            std::os::unix::fs::symlink(&from, &to)?;
        }
    }
    if !live(&dir) {
        adopt_newer(store, slot);
        let ours =
            store.credential(slot)?.filter(|c| cc::usable(c)).ok_or(SessionError::NoCredential)?;
        if profile_credential(&dir, store).as_deref() != Some(ours.as_str()) {
            store.security.set(&service, &user_account(), &ours)?;
        }
    }
    Ok(dir)
}

/// The folder's `.claude.json`: the account's `oauthAccount` and the
/// onboarding done, every other member kept.
fn write_config(dir: &Path, oauth: &Value) -> std::io::Result<()> {
    let file = dir.join(".claude.json");
    let mut config = super::read_optional(&file)?
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Default::default()));
    let map = config.as_object_mut().expect("an object");
    map.insert("oauthAccount".into(), oauth.clone());
    map.insert("hasCompletedOnboarding".into(), Value::Bool(true));
    map.entry("theme").or_insert_with(|| Value::String("dark".into()));
    super::write_private(
        &file,
        &(serde_json::to_string_pretty(&config).expect("serializes") + "\n"),
    )
}

/// Where `claude` is usually installed.
pub fn find_claude(home: &Path) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .map(|d| d.join("claude"))
        .chain([
            home.join(".local/bin/claude"),
            PathBuf::from("/opt/homebrew/bin/claude"),
            PathBuf::from("/usr/local/bin/claude"),
        ])
        .find(|p| p.is_file())
}

/// Runs `claude args…` as account `slot` and returns its exit code. When the
/// account is Claude Code's default login, `claude` runs as it is, unless
/// `require_session`, which refuses rather than use the default login.
/// After it ends, a login Claude Code refreshed in the folder is taken back.
pub fn run(
    store: &Store,
    claude: &Path,
    slot: u32,
    require_session: bool,
    args: &[String],
) -> Result<i32, SessionError> {
    run_in(store, claude, slot, require_session, args, None, None)
}

/// [`run`] in the folder `cwd`, stopped after `timeout` (exit 124, as
/// `timeout` reports it).
pub fn run_in(
    store: &Store,
    claude: &Path,
    slot: u32,
    require_session: bool,
    args: &[String],
    cwd: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<i32, SessionError> {
    let mut command = Command::new(claude);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    for name in SCRUBBED {
        command.env_remove(name);
    }
    if switch::active_slot(store) == Some(slot) {
        if require_session {
            return Err(SessionError::IsActive);
        }
        command.env_remove("CLAUDE_CONFIG_DIR");
        return wait(command, timeout);
    }
    let dir = prepare(store, slot)?;
    command.env("CLAUDE_CONFIG_DIR", &dir);
    let code = wait(command, timeout);
    if let Ok(_app) = AppLock::take(&store.paths) {
        adopt_newer(store, slot);
    }
    code
}

fn wait(mut command: Command, timeout: Option<Duration>) -> Result<i32, SessionError> {
    let mut child = command.spawn()?;
    let Some(limit) = timeout else {
        return Ok(child.wait()?.code().unwrap_or(1));
    };
    let deadline = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code().unwrap_or(1));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(124);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
