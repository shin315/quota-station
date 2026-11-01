// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A fake `security` for the tests that keep Keychain items: nothing here
//! reaches the real Keychain.

use std::path::{Path, PathBuf};

use super::script;

/// A `security` that keeps generic passwords as files in `dir/items`, exits
/// 44 for a missing one as macOS's does, and records every run.
pub fn fake_security(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("items")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/security"),
        &format!(
            r#"for a in "$@"; do printf '%s\037' "$a" >> '{d}/calls'; done
printf '\n' >> '{d}/calls'
cmd="$1"; shift
svc=""; acct=""; value=""
while [ $# -gt 0 ]; do
  case "$1" in
    -s) svc="$2"; shift 2 ;;
    -a) acct="$2"; shift 2 ;;
    -w) if [ $# -ge 2 ]; then value="$2"; shift 2; else shift; fi ;;
    *) shift ;;
  esac
done
item='{d}/items/'"$svc--$acct"
missing() {{ echo "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain." >&2; exit 44; }}
case "$cmd" in
  add-generic-password)
    [ -e '{d}/fail-add' ] && {{ echo "security: SecKeychainItemCreateFromContent: User interaction is not allowed." >&2; exit 36; }}
    printf '%s' "$value" > "$item" ;;
  find-generic-password)
    [ -f "$item" ] || missing
    cat "$item"; echo ;;
  delete-generic-password)
    [ -e '{d}/fail-delete' ] && {{ echo "security: SecKeychainItemDelete: User interaction is not allowed." >&2; exit 36; }}
    [ -f "$item" ] || missing
    rm "$item" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}
