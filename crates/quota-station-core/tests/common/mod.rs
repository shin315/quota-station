//! Shared helpers for the fixture tests; each test file uses a subset.
//! Every fixture is synthetic: made-up slot numbers, percentages and
//! times, generated at 2026-01-16 07:20 UTC.
#![allow(dead_code)]

pub mod accounts;
pub mod claude;
pub mod desktop;
pub mod google;

use std::path::PathBuf;

use quota_station_core::axi::{self, Response};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).expect("fixture exists")
}

pub fn fixture(name: &str) -> Response {
    axi::parse(&fixture_text(name)).expect("fixture parses")
}

static NEXT_DIR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A fresh, empty directory for one test, under the system's temp folder.
pub fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "qs-{tag}-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes an executable `#!/bin/sh` script at `path`.
pub fn script(path: &std::path::Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_path_buf()
}
