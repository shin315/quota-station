// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the Codex accounts need from the operating system, behind one seam
//! ([`Platform`]), so the account store ([`super::accounts`]) holds no
//! macOS-only step and the Windows build only has to give its own answers:
//!
//! - a folder only the owner can open, and a file only the owner can read
//!   (700 and 600 here; on Windows an ACL for the current user alone);
//! - moving a sign-in file without ever overwriting one, and without a
//!   second copy left behind (`renamex_np` with `RENAME_EXCL` here; on
//!   Windows `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, retried
//!   while Codex holds the file open);
//! - keeping the account folders out of backups (Time Machine here).
//!
//! Finding the `codex` command and restarting Codex's background process
//! stay in [`super::procs`], which already answers them per system.

use std::io;
use std::path::Path;

/// The operating system's part of the Codex accounts.
pub trait Platform: Send + Sync {
    /// Makes `dir` (and its parents) and leaves it openable by the owner only.
    fn private_dir(&self, dir: &Path) -> io::Result<()>;
    /// Leaves `file` readable and writable by the owner only.
    fn private_file(&self, file: &Path) -> io::Result<()>;
    /// Moves `from` to `to`, refusing when something is at `to` already.
    /// Afterwards there is one copy, at `to`.
    fn move_file(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// Moves `from` to `to`, replacing what is there in one step: a sign-in
    /// renewed over its old one.
    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// Asks the system's backup to leave `dir` out. Best effort: a failure
    /// is not a reason to stop.
    fn exclude_from_backup(&self, dir: &Path);
}

/// macOS (and the Unix systems the tests run on).
#[derive(Debug, Clone, Copy, Default)]
pub struct Unix {
    /// Runs `tmutil addexclusion`; off in tests, which use temporary folders.
    pub time_machine: bool,
}

impl Unix {
    /// The app's own: Time Machine leaves the account folders out.
    pub fn system() -> Self {
        Unix { time_machine: cfg!(target_os = "macos") }
    }
}

#[cfg(unix)]
impl Platform for Unix {
    fn private_dir(&self, dir: &Path) -> io::Result<()> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
    }

    fn private_file(&self, file: &Path) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))
    }

    fn move_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        match rename_no_replace(from, to) {
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => copy_then_remove(from, to),
            other => other,
        }
    }

    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        match std::fs::rename(from, to) {
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
                // Across disks: in next to `to` first, then over it.
                let near = to.with_extension(format!("json.{}.tmp", std::process::id()));
                copy_then_remove(from, &near)?;
                std::fs::rename(&near, to)
            }
            other => other,
        }
    }

    fn exclude_from_backup(&self, dir: &Path) {
        if !self.time_machine {
            return;
        }
        // A sticky exclusion: it stays with the folder, and needs no admin.
        let _ = std::process::Command::new("/usr/bin/tmutil")
            .arg("addexclusion")
            .arg(dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// Windows: a folder whose ACL grants the current user alone and inherits
/// nothing, which the files made in it inherit; `MoveFileExW` without
/// `MOVEFILE_REPLACE_EXISTING` for a move (across volumes too, written
/// through), with it for a replace, both tried again for two seconds while
/// Codex holds the file open ([`crate::win::move_file`]). Windows's File
/// History leaves `AppData` out, where the account folders are (see
/// [`super::accounts`]), and Windows Search is told to leave them out too.
#[derive(Debug, Clone, Copy, Default)]
pub struct Windows;

impl Windows {
    pub fn system() -> Self {
        Windows
    }
}

#[cfg(windows)]
impl Platform for Windows {
    fn private_dir(&self, dir: &Path) -> io::Result<()> {
        crate::os::owner_dir(dir)
    }

    fn private_file(&self, _file: &Path) -> io::Result<()> {
        // The folder's ACL, inherited.
        Ok(())
    }

    fn move_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        crate::win::move_file(from, to, false)
    }

    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        crate::win::move_file(from, to, true)
    }

    fn exclude_from_backup(&self, dir: &Path) {
        crate::win::not_indexed(dir);
    }
}

/// This system's: [`Unix`] or [`Windows`].
#[cfg(unix)]
pub type System = Unix;
#[cfg(windows)]
pub type System = Windows;

/// `rename` that fails with `AlreadyExists` instead of replacing `to`.
#[cfg(target_os = "macos")]
fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = |p: &Path| CString::new(p.as_os_str().as_bytes()).map_err(io::Error::other);
    let (from, to) = (c(from)?, c(to)?);
    // SAFETY: both are valid NUL-terminated paths that outlive the call.
    let done = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if done == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Elsewhere (the Linux the tests may run on): a look first, then `rename`.
#[cfg(all(unix, not(target_os = "macos")))]
fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    std::fs::rename(from, to)
}

/// Across disks: a new file at `to` (owner only, never over one), synced,
/// then `from` removed, so a second copy lives only for that moment.
#[cfg(unix)]
fn copy_then_remove(from: &Path, to: &Path) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut bytes = std::fs::read(from)?;
    let written =
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(to).and_then(
            |mut f| {
                f.write_all(&bytes)?;
                f.sync_all()
            },
        );
    // A sign-in: the bytes go no further than this function.
    bytes.iter_mut().for_each(|b| *b = 0);
    if let Err(e) = written {
        if e.kind() != io::ErrorKind::AlreadyExists {
            let _ = std::fs::remove_file(to);
        }
        return Err(e);
    }
    std::fs::remove_file(from)
}
