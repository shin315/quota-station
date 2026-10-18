//! Every file the Antigravity module reads or writes goes through here, and
//! only paths inside the app's Application Support folder are allowed: a
//! path anywhere else, or one that passes through a symlinked folder, is
//! refused before it is opened. So no bug in this module can reach another
//! tool's login files (SECURITY.md lists them). The one other place read is
//! an installed app's bundle ([`Bundle`]), only for its version, and never
//! written.
//!
//! With the `test-support` feature every path is also recorded, for the spy
//! test that runs every flow and checks where they went.

use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

    /// `path` must be under the folder, written without `..`, and neither
    /// the folder nor any folder between it and `path` may be a symlink.
    fn check(&self, path: &Path) -> io::Result<()> {
        record(path);
        let plain =
            path.components().all(|c| matches!(c, Component::Normal(_) | Component::RootDir));
        if !(plain && !self.0.as_os_str().is_empty() && path.starts_with(&self.0) && path != self.0)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "outside the app's folder",
            ));
        }
        // A folder not made yet is fine: `write_private` makes it, as a folder.
        let linked =
            path.ancestors().skip(1).take_while(|dir| dir.starts_with(&self.0)).any(|dir| {
                std::fs::symlink_metadata(dir).is_ok_and(|m| m.file_type().is_symlink())
            });
        if linked {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "a symlinked folder"));
        }
        Ok(())
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
    /// owner (600), in a folder only the owner can open (700). Each write has
    /// its own temporary file, so two writes at once cannot take each other's.
    pub fn write_private(&self, path: &Path, text: &str) -> io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        self.check(path)?;
        let dir = path.parent().ok_or_else(|| io::Error::other("no folder"))?;
        self.check(dir)?;
        std::fs::DirBuilder::new().recursive(true).create(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        let ext =
            format!("json.{}.{}.tmp", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
        let tmp = path.with_extension(ext);
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .and_then(|mut file| {
                file.write_all(text.as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| std::fs::rename(&tmp, path));
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        written
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

/// An installed app's bundle, such as Antigravity's, read only for its
/// version: files are read by their place inside the bundle, never written,
/// and nothing outside the bundle can be named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle(PathBuf);

impl Bundle {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Bundle(path.into())
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// `parts` joined under the bundle; `None` for an empty or `..` part.
    fn file(&self, parts: &[&str]) -> Option<PathBuf> {
        let plain = |p: &&str| {
            let mut c = Path::new(p).components();
            matches!((c.next(), c.next()), (Some(Component::Normal(_)), None))
        };
        parts.iter().all(plain).then(|| parts.iter().fold(self.0.clone(), |p, part| p.join(part)))
    }

    /// The bundle is there (a symlink to one counts, as `Path::exists`).
    pub fn present(&self) -> bool {
        record(&self.0);
        std::fs::metadata(&self.0).is_ok()
    }

    /// Something is at `parts` inside the bundle.
    pub fn has(&self, parts: &[&str]) -> bool {
        self.file(parts).is_some_and(|path| {
            record(&path);
            std::fs::metadata(path).is_ok()
        })
    }

    pub fn read_to_string(&self, parts: &[&str]) -> io::Result<String> {
        let path = self
            .file(parts)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not inside the bundle"))?;
        record(&path);
        std::fs::read_to_string(path)
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

    #[test]
    fn a_bundle_reads_only_inside_itself() {
        let base = std::env::temp_dir().join(format!("qs-bundle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("App.app/Contents")).unwrap();
        std::fs::write(base.join("App.app/Contents/product.json"), "{}").unwrap();
        std::fs::write(base.join("outside.json"), "{}").unwrap();
        let bundle = Bundle::new(base.join("App.app"));
        assert!(bundle.present());
        assert!(bundle.has(&["Contents", "product.json"]));
        assert_eq!(bundle.read_to_string(&["Contents", "product.json"]).unwrap(), "{}");
        assert!(!bundle.has(&["..", "outside.json"]));
        assert!(bundle.read_to_string(&["..", "outside.json"]).is_err());
        assert!(bundle.read_to_string(&["Contents/../../outside.json"]).is_err());
        assert!(bundle.read_to_string(&["/etc/hosts"]).is_err());
        assert!(!Bundle::new(base.join("Missing.app")).present());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_symlinked_folder_inside_is_refused() {
        let base = std::env::temp_dir().join(format!("qs-fs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (root, elsewhere) = (base.join("app"), base.join("elsewhere"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, root.join("antigravity")).unwrap();
        let dir = AppDir::new(&root);

        let err = dir.write_private(&root.join("antigravity/accounts.json"), "{}").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(dir.read_to_string(&root.join("antigravity/accounts.json")).is_err());
        assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0, "nothing written there");

        // A real folder works, and leaves no temporary file behind.
        dir.write_private(&root.join("real/accounts.json"), "{}").unwrap();
        let names: Vec<_> = std::fs::read_dir(root.join("real")).unwrap().flatten().collect();
        assert_eq!(names.len(), 1);
        assert_eq!(dir.read_to_string(&root.join("real/accounts.json")).unwrap(), "{}");
        std::fs::remove_dir_all(&base).unwrap();
    }
}
