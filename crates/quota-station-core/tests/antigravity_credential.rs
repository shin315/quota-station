// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

#![cfg(feature = "antigravity-accounts")]
// The two tests are ported from Cockpit Tools
// `src-tauri/src/modules/antigravity_credential_tests.rs` (1.3.70, commit
// 244b957, CC BY-NC-SA 4.0); changes: they write through the app's
// `SharedLogin` with a token instead of Cockpit's private payload function,
// and the fixture folder is named by a counter instead of the clock.

//! The write of the shared Antigravity login, against a fake `security`:
//! a failed write must leave the old login as it was, and a write that
//! hangs must be stopped before it can change anything late.
// Unix only: the fake `security` is a shell script.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use quota_station_core::antigravity::credential::SharedLogin;
use quota_station_core::antigravity::token::TokenData;

struct Fixture(PathBuf);

impl Fixture {
    fn new(script: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "antigravity-keychain-write-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("credential"), "old-credential").unwrap();
        std::fs::write(dir.join("security"), script).unwrap();
        std::fs::set_permissions(dir.join("security"), std::fs::Permissions::from_mode(0o700))
            .unwrap();
        Self(dir)
    }

    fn old_credential(&self) -> String {
        std::fs::read_to_string(self.0.join("credential")).unwrap()
    }

    fn writer(&self, timeout: Duration) -> SharedLogin {
        SharedLogin::fake(self.0.join("security"), timeout)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn token() -> TokenData {
    TokenData::new("fixture-access".into(), "fixture-refresh".into(), 3599, None, None, None, 0)
}

#[test]
fn failed_keychain_upsert_does_not_delete_existing_credential() {
    let fixture = Fixture::new(
        r#"#!/bin/sh
fixture_dir="${0%/*}"
case "$1" in
  delete-generic-password) /bin/rm -f "$fixture_dir/credential"; exit 0 ;;
  add-generic-password) echo 'injected write failure' >&2; exit 1 ;;
  *) exit 2 ;;
esac
"#,
    );
    // Inject only the fixture executable; never invoke the real Keychain or change PATH.
    let error = fixture.writer(Duration::from_secs(2)).write(&token()).unwrap_err();
    assert!(error.to_string().contains("injected write failure"), "{error}");
    assert_eq!(fixture.old_credential(), "old-credential");
}

#[test]
fn keychain_timeout_kills_writer_before_late_credential_overwrite() {
    let fixture = Fixture::new(
        r#"#!/bin/sh
/bin/sleep 0.5
printf 'late-credential' > "${0%/*}/credential"
"#,
    );
    let start = std::time::Instant::now();
    let error = fixture.writer(Duration::from_millis(100)).write(&token()).unwrap_err();
    assert!(error.to_string().contains("timed out"), "{error}");
    assert!(start.elapsed() < Duration::from_secs(2));
    // Wait beyond the fixture write delay to catch a dropped-but-still-running writer.
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(fixture.old_credential(), "old-credential");
}
