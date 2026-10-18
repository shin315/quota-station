//! Every file the Antigravity module reads or writes goes through here, and
//! only paths inside the app's Application Support folder are allowed: a
//! path anywhere else is refused before it is opened. So no bug in this
//! module can reach another tool's login files (SECURITY.md lists them).
//!
//! With the `test-support` feature every path is also recorded, for the spy
//! test that runs every flow and checks where they went.

use std::io;
use std::path::{Component, Path, PathBuf};

/// The app's Application Support folder, the root of everything here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDir(PathBuf);

impl AppDir {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        AppDir(path.into())
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// `path` must be under the folder, written without `..`.
    fn check(&self, path: &Path) -> io::Result<()> {
        record(path);
        let plain =
            path.components().all(|c| matches!(c, Component::Normal(_) | Component::RootDir));
        if plain && !self.0.as_os_str().is_empty() && path.starts_with(&self.0) && path != self.0 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "outside the app's folder"))
        }
    }

    /// The file's own metadata: a symlink is not followed.
    pub fn metadata(&self, path: &Path) -> io::Result<std::fs::Metadata> {
        self.check(path)?;
        std::fs::symlink_metadata(path)
    }

    pub fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.check(path)?;
        if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "a symlink"));
        }
        std::fs::read_to_string(path)
    }

    /// Writes through a temporary file and a rename, readable only by the
    /// owner (600), in a folder only the owner can open (700).
    pub fn write_private(&self, path: &Path, text: &str) -> io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        self.check(path)?;
        let dir = path.parent().ok_or_else(|| io::Error::other("no folder"))?;
        self.check(dir)?;
        std::fs::DirBuilder::new().recursive(true).create(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        let tmp = path.with_extension("json.tmp");
        let _ = std::fs::remove_file(&tmp);
        let mut file =
            std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    }

    /// Removes a file; one that is not there counts as removed.
    pub fn remove(&self, path: &Path) -> io::Result<()> {
        self.check(path)?;
        match std::fs::remove_file(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

#[cfg(feature = "test-support")]
static OPENED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "test-support")]
fn record(path: &Path) {
    OPENED.lock().unwrap_or_else(|e| e.into_inner()).push(path.to_path_buf());
}

#[cfg(not(feature = "test-support"))]
fn record(_: &Path) {}

/// Every path checked so far, allowed or not, and clears the list.
#[cfg(feature = "test-support")]
pub fn take_opened() -> Vec<PathBuf> {
    std::mem::take(&mut *OPENED.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_paths_inside_the_folder_are_allowed() {
        let dir = AppDir::new("/tmp/qs-app");
        assert!(dir.check(Path::new("/tmp/qs-app/antigravity/accounts.json")).is_ok());
        assert!(dir.check(Path::new("/tmp/qs-app/oauth-client.json")).is_ok());
        assert!(dir.check(Path::new("/tmp/qs-app")).is_err(), "the folder itself is not a file");
        assert!(dir.check(Path::new("/tmp/qs-app/../elsewhere/server.json")).is_err());
        assert!(dir.check(Path::new("/tmp/qs-application/x")).is_err());
        assert!(dir.check(Path::new("relative/x")).is_err());
        assert!(AppDir::new("").check(Path::new("x")).is_err());
    }
}
