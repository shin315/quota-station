// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Windows's side of the seams in [`crate::os`] and
//! [`crate::codex::platform`], called straight through `windows-sys`:
//!
//! - [`Dpapi`]: the seal of the app's own secrets ([`crate::sealed`]),
//!   `CryptProtectData` for this user (never the machine), with an entropy
//!   of the app's own, so another program of the same user that calls DPAPI
//!   without it cannot open them;
//! - [`WinCred`]: generic credentials in Credential Manager, read and
//!   written as bytes (`CredReadW`, `CredWriteW`), for Claude Code's items
//!   ([`crate::claude::credman`]);
//! - [`Secrets`]: the two as the app's [`crate::os::SecretStore`];
//! - [`Tasks`]: the processes, from `sysinfo`, with their command lines;
//! - [`owner_only`]: a folder whose ACL grants its user alone and inherits
//!   nothing, as 700 is on a Mac;
//! - [`move_file`]: a move that never replaces (or, asked, one that does),
//!   tried again while another program holds the file;
//! - [`env_set`]: whether a variable is set in the registry's environment,
//!   without reading its value.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS,
    ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_LOCK_VIOLATION, ERROR_NOT_FOUND,
    ERROR_SHARING_VIOLATION, ERROR_SUCCESS, GENERIC_ALL, HANDLE, STILL_ACTIVE,
};

use crate::claude::credman::CcItems;
use crate::os::{Credentials, Process, ProcessList, Split};
use crate::sealed::{Seal, SealedFiles};

/// `text` as a NUL-terminated UTF-16 string.
fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

fn last_error() -> io::Error {
    io::Error::last_os_error()
}

// ===== The app's own secrets =====

/// The app's secrets and Claude Code's login items, each in its place.
pub type Secrets = Split<CcItems<WinCred>, SealedFiles<Dpapi>>;

/// The folder of the app's sealed secrets: `%LOCALAPPDATA%`, which never
/// roams, as Tauri's `app_local_data_dir` names it.
pub fn secrets_dir() -> std::path::PathBuf {
    let local = match std::env::var_os("LOCALAPPDATA") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
            .join("AppData")
            .join("Local"),
    };
    local.join(crate::app_dir::ID).join("secrets")
}

impl Secrets {
    pub fn system() -> Secrets {
        // The tests keep their secrets in memory; one that reached for the
        // real stores stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake secret store");
        }
        Split { claude_code: CcItems::new(WinCred), own: SealedFiles::new(secrets_dir(), Dpapi) }
    }
}

/// DPAPI for the current user, with the app's own entropy.
#[derive(Debug, Clone, Copy, Default)]
pub struct Dpapi;

/// Mixed into every seal: a program of the same user that calls DPAPI
/// without it opens none of the app's files.
const ENTROPY: &[u8] = b"com.shin315.quota-station secrets v1";

impl Dpapi {
    fn call(&self, data: &[u8], seal: bool) -> io::Result<Vec<u8>> {
        use windows_sys::Win32::Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };
        let len = u32::try_from(data.len()).map_err(|_| io::Error::other("too large"))?;
        let input = CRYPT_INTEGER_BLOB { cbData: len, pbData: data.as_ptr().cast_mut() };
        let entropy = CRYPT_INTEGER_BLOB {
            cbData: ENTROPY.len() as u32,
            pbData: ENTROPY.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        // SAFETY: the blobs point at live buffers for the call; DPAPI only
        // reads them and allocates `output`, freed below with `LocalFree`.
        let done = unsafe {
            if seal {
                CryptProtectData(
                    &input,
                    std::ptr::null(),
                    &entropy,
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    std::ptr::null_mut(),
                    &entropy,
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
        };
        if done == 0 {
            return Err(last_error());
        }
        // SAFETY: DPAPI set `output` to a buffer of `cbData` bytes.
        let bytes =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        // SAFETY: the buffer is DPAPI's and still ours to free; an opened
        // secret is zeroed first.
        unsafe {
            if !seal {
                std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
            }
            LocalFree(output.pbData.cast());
        }
        Ok(bytes)
    }
}

impl Seal for Dpapi {
    fn seal(&self, plain: &[u8]) -> io::Result<Vec<u8>> {
        self.call(plain, true)
    }

    fn open(&self, sealed: &[u8]) -> io::Result<Vec<u8>> {
        self.call(sealed, false)
    }
}

// ===== Credential Manager =====

/// Generic credentials, as Bun's `Bun.secrets` keeps Claude Code's: the
/// value as bytes, kept for the user in the enterprise scope.
#[derive(Debug, Clone, Copy, Default)]
pub struct WinCred;

impl Credentials for WinCred {
    fn read(&self, target: &str) -> io::Result<Option<Vec<u8>>> {
        use windows_sys::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };
        let name = wide(target);
        let mut found: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: `name` is NUL-terminated; a credential read is freed below.
        if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut found) } == 0 {
            // SAFETY: reads the calling thread's last error.
            return match unsafe { GetLastError() } {
                ERROR_NOT_FOUND => Ok(None),
                _ => Err(last_error()),
            };
        }
        // SAFETY: CredReadW returned a credential with a blob of its size.
        let bytes = unsafe {
            let c = &*found;
            let blob = if c.CredentialBlob.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec()
            };
            CredFree(found.cast());
            blob
        };
        Ok(Some(bytes))
    }

    fn write(&self, target: &str, account: &str, blob: &[u8]) -> io::Result<()> {
        use windows_sys::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_ENTERPRISE, CRED_TYPE_GENERIC,
        };
        let mut name = wide(target);
        let mut user = wide(account);
        let size = u32::try_from(blob.len()).map_err(|_| io::Error::other("too large"))?;
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            CredentialBlobSize: size,
            CredentialBlob: blob.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_ENTERPRISE,
            UserName: user.as_mut_ptr(),
            ..Default::default()
        };
        // SAFETY: every pointer in `credential` is to a live buffer, which
        // CredWriteW only reads.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            return Err(last_error());
        }
        Ok(())
    }

    fn delete(&self, target: &str) -> io::Result<()> {
        use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
        let name = wide(target);
        // SAFETY: `name` is NUL-terminated.
        if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
            // SAFETY: reads the calling thread's last error.
            if unsafe { GetLastError() } != ERROR_NOT_FOUND {
                return Err(last_error());
            }
        }
        Ok(())
    }
}

// ===== Processes =====

/// Windows's processes: whether a pid runs through `OpenProcess`, and the
/// table with each command line from `sysinfo`. None has a terminal to
/// tell. This user's processes and their ports are not listed: as
/// quota-axi does on Windows, the Antigravity read asks `agy` only.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tasks;

impl ProcessList for Tasks {
    fn alive(&self, pid: i32) -> bool {
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let Ok(pid) = u32::try_from(pid) else { return false };
        if pid <= 4 {
            // The idle process and System, never a session.
            return false;
        }
        // SAFETY: the handle is checked, read once and closed.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                // Denied means it runs, as another user's.
                return GetLastError() == ERROR_ACCESS_DENIED;
            }
            let mut code = 0u32;
            let read = GetExitCodeProcess(handle, &mut code);
            CloseHandle(handle);
            read != 0 && code == STILL_ACTIVE as u32
        }
    }

    fn all(&self) -> Option<Vec<Process>> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
        let mut system = System::new();
        let kind =
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_exe(UpdateKind::Always);
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        let mut out: Vec<Process> = system
            .processes()
            .iter()
            .map(|(pid, p)| Process { pid: pid.as_u32(), tty: None, command: command_line(p) })
            .filter(|p| !p.command.is_empty())
            .collect();
        out.sort_by_key(|p| p.pid);
        Some(out)
    }

    fn own(&self, _wait: Duration) -> Option<Vec<Process>> {
        None
    }

    fn listening(&self, _pid: u32, _wait: Duration) -> Option<Vec<u16>> {
        None
    }
}

/// The process's words joined by spaces, as `ps` shows a command line; for
/// one whose words cannot be read (another user's, an elevated one), its
/// program's path, else its name.
fn command_line(p: &sysinfo::Process) -> String {
    let words: Vec<String> = p.cmd().iter().map(|w| w.to_string_lossy().into_owned()).collect();
    if !words.is_empty() {
        return words.join(" ");
    }
    match p.exe() {
        Some(exe) => exe.display().to_string(),
        None => p.name().to_string_lossy().into_owned(),
    }
}

// ===== Files =====

/// The current user's SID, copied out of the process's token.
fn user_sid() -> io::Result<Vec<u8>> {
    use windows_sys::Win32::Security::{
        GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: the pseudo-handle needs no closing; the token is closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(last_error());
    }
    let mut size = 0u32;
    // SAFETY: the first call only asks for the size.
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size) };
    // u64s, so the buffer is aligned for TOKEN_USER.
    let mut buf = vec![0u64; (size as usize).div_ceil(8).max(1)];
    // SAFETY: `buf` holds `size` bytes; the token is ours until closed.
    let read = unsafe {
        let read = GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), size, &mut size);
        CloseHandle(token);
        read
    };
    if read == 0 {
        return Err(last_error());
    }
    // SAFETY: the buffer now holds a TOKEN_USER whose SID points inside it.
    unsafe {
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let len = GetLengthSid(user.User.Sid) as usize;
        Ok(std::slice::from_raw_parts(user.User.Sid as *const u8, len).to_vec())
    }
}

/// Leaves `dir` to its owner alone: an ACL that grants the current user
/// full control and nobody else anything, inherited by what is made in it,
/// and none inherited from above (`PROTECTED_DACL_SECURITY_INFORMATION`).
pub fn owner_only(dir: &Path) -> io::Result<()> {
    use windows_sys::Win32::Security::Authorization::{
        SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, NO_MULTIPLE_TRUSTEE,
        SET_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_USER, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        ACL, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        SUB_CONTAINERS_AND_OBJECTS_INHERIT,
    };
    let mut sid = user_sid()?;
    let access = EXPLICIT_ACCESS_W {
        grfAccessPermissions: GENERIC_ALL,
        grfAccessMode: SET_ACCESS,
        grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_USER,
            ptstrName: sid.as_mut_ptr().cast(),
        },
    };
    let mut acl: *mut ACL = std::ptr::null_mut();
    // SAFETY: one entry, whose SID lives in `sid`; the new ACL is freed below.
    let made = unsafe { SetEntriesInAclW(1, &access, std::ptr::null(), &mut acl) };
    if made != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(made as i32));
    }
    let path = wide(dir);
    // SAFETY: `path` is NUL-terminated and `acl` is the one just made.
    let set = unsafe {
        let set = SetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            acl,
            std::ptr::null(),
        );
        LocalFree(acl.cast());
        set
    };
    if set != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(set as i32));
    }
    Ok(())
}

/// What the move of a sign-in says when Codex keeps holding the file.
pub const HELD: &str = "Codex is using the sign-in; quit Codex and try again";

/// Moves `from` to `to` (`MoveFileExW`), across volumes too, written
/// through to the disk. Without `replace` a file at `to` stops it as
/// `AlreadyExists`. A file another program holds open (Codex App, an
/// indexer, an antivirus) is tried again for two seconds, then the move
/// fails with [`HELD`].
pub fn move_file(from: &Path, to: &Path, replace: bool) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let (src, dst) = (wide(from), wide(to));
    let mut flags = MOVEFILE_COPY_ALLOWED | MOVEFILE_WRITE_THROUGH;
    if replace {
        flags |= MOVEFILE_REPLACE_EXISTING;
    }
    for _ in 0..10 {
        // SAFETY: both paths are NUL-terminated.
        if unsafe { MoveFileExW(src.as_ptr(), dst.as_ptr(), flags) } != 0 {
            return Ok(());
        }
        // SAFETY: reads the calling thread's last error.
        match unsafe { GetLastError() } {
            ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => {
                return Err(io::ErrorKind::AlreadyExists.into());
            }
            ERROR_SHARING_VIOLATION | ERROR_ACCESS_DENIED | ERROR_LOCK_VIOLATION => {
                std::thread::sleep(Duration::from_millis(200));
            }
            code => return Err(io::Error::from_raw_os_error(code as i32)),
        }
    }
    Err(io::Error::new(io::ErrorKind::ResourceBusy, HELD))
}

/// Marks `dir` for Windows Search to leave out
/// (`FILE_ATTRIBUTE_NOT_CONTENT_INDEXED`). Best effort.
pub fn not_indexed(dir: &Path) {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED,
        INVALID_FILE_ATTRIBUTES,
    };
    let path = wide(dir);
    // SAFETY: `path` is NUL-terminated.
    unsafe {
        let now = GetFileAttributesW(path.as_ptr());
        if now != INVALID_FILE_ATTRIBUTES {
            SetFileAttributesW(path.as_ptr(), now | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED);
        }
    }
}

// ===== The environment in the registry =====

/// Whether `name` is set, not empty, in the user's environment
/// (`HKCU\Environment`) or the computer's (`HKLM\SYSTEM\…\Session
/// Manager\Environment`). Only the value's size is asked for: its text is
/// never read.
pub fn env_set(machine: bool, name: &str) -> io::Result<bool> {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_NOEXPAND, RRF_RT_REG_EXPAND_SZ,
        RRF_RT_REG_SZ,
    };
    let (root, key) = if machine {
        (HKEY_LOCAL_MACHINE, r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment")
    } else {
        (HKEY_CURRENT_USER, "Environment")
    };
    let (key, value) = (wide(key), wide(name));
    let mut size = 0u32;
    // SAFETY: both strings are NUL-terminated; with no data buffer the call
    // only reports the size.
    let found = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    match found {
        // A string of more than its terminating NUL.
        ERROR_SUCCESS => Ok(size > 2),
        ERROR_FILE_NOT_FOUND => Ok(false),
        code => Err(io::Error::from_raw_os_error(code as i32)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_sealed_by_dpapi_opens_again_and_is_not_its_text() {
        let text = br#"{"claudeAiOauth":{"accessToken":"synthetic-not-a-token"}}"#;
        let sealed = Dpapi.seal(text).unwrap();
        assert_ne!(&sealed[..], &text[..]);
        assert!(!sealed.windows(9).any(|w| w == b"synthetic"));
        assert_eq!(Dpapi.open(&sealed).unwrap(), text);
        let mut tampered = sealed.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xff;
        assert!(Dpapi.open(&tampered).is_err());
    }

    #[test]
    fn the_sealed_files_round_trip_through_dpapi() {
        use crate::os::SecretStore;
        let dir = std::env::temp_dir().join(format!("qs-dpapi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = SealedFiles::new(&dir, Dpapi);
        store.set("com.shin315.quota-station.mcp-secret", "github:env:TOKEN", "synthetic").unwrap();
        assert_eq!(
            store.get("com.shin315.quota-station.mcp-secret", "github:env:TOKEN").unwrap(),
            Some("synthetic".into())
        );
        // A new store over the same folder, as after a restart.
        let again = SealedFiles::new(&dir, Dpapi);
        assert!(again.exists("com.shin315.quota-station.mcp-secret", "github:env:TOKEN").unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn this_process_runs_and_a_finished_one_does_not() {
        assert!(Tasks.alive(std::process::id() as i32));
        let mut child = std::process::Command::new("cmd").args(["/C", "exit 0"]).spawn().unwrap();
        let pid = child.id() as i32;
        child.wait().unwrap();
        assert!(!Tasks.alive(pid));
        assert!(!Tasks.alive(4));
    }

    #[test]
    fn the_table_lists_this_process_with_its_command_line() {
        let table = Tasks.all().unwrap();
        let me = table.iter().find(|p| p.pid == std::process::id()).unwrap();
        assert!(me.command.to_ascii_lowercase().contains(".exe"), "{}", me.command);
        assert_eq!(me.tty, None);
    }

    #[test]
    fn a_folder_left_to_its_owner_still_opens_for_them() {
        let dir = std::env::temp_dir().join(format!("qs-owner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        owner_only(&dir).unwrap();
        std::fs::write(dir.join("auth.json"), "{}").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("auth.json")).unwrap(), "{}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_move_never_replaces_unless_asked() {
        let dir = std::env::temp_dir().join(format!("qs-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.json"), dir.join("b.json"));
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();
        let err = move_file(&a, &b, false).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "b");
        move_file(&a, &b, true).unwrap();
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "a");
        assert!(!a.exists());
        move_file(&b, &a, false).unwrap();
        assert!(a.exists() && !b.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_variable_nobody_set_is_not_set() {
        assert!(!env_set(false, "QUOTA_STATION_TEST_NEVER_SET_7f3a").unwrap());
        assert!(!env_set(true, "QUOTA_STATION_TEST_NEVER_SET_7f3a").unwrap());
        // Windows sets this one for every computer.
        assert!(env_set(true, "OS").unwrap());
    }
}
