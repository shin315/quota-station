// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the advanced part brings over from the old switcher when the app
//! imports the other account switcher's accounts, which the app knows only
//! through the hooks ([`crate::ext::Hooks::claude_imported`]): the rotation
//! file, and the `prime` flag with its times, each only when the app has none
//! yet. The import's outcome says whether each came over (`rotation`, `prime`,
//! at its end, both always written).

use serde_json::Value;

use crate::claude::{private_dir, write_private, Paths};
use crate::ext::{Fields, Hooks};

/// The old switcher's priming times (its own paths, read only).
const PRIME_STATE: &str = ".local/state/cswap-prefer-team";
const PRIME_FLAG: &str = ".config/cswap-prefer-team/prime";

/// The advanced part's hooks for an import. The app's extension
/// gives the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImportHooks;

impl Hooks for ImportHooks {
    fn claude_imported(&self, paths: &Paths) -> Fields {
        let mut ext = Fields::new();
        ext.insert("rotation", Value::Bool(copy_rotation(paths)));
        ext.insert("prime", Value::Bool(copy_prime(paths)));
        ext
    }
}

/// The old switcher's rotation file, when the app has none.
fn copy_rotation(paths: &Paths) -> bool {
    let own = paths.rotation();
    let legacy = crate::rotation::path(&paths.home);
    if own.exists() {
        return false;
    }
    match std::fs::read_to_string(&legacy) {
        Ok(text) => write_private(&own, &text).is_ok(),
        Err(_) => false,
    }
}

/// The old switcher's priming flag and times, when the app has none.
fn copy_prime(paths: &Paths) -> bool {
    if paths.prime_flag().exists() || !paths.home.join(PRIME_FLAG).exists() {
        return false;
    }
    let state = paths.prime_state();
    if private_dir(&state).is_ok() {
        for entry in std::fs::read_dir(paths.home.join(PRIME_STATE)).into_iter().flatten().flatten()
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("prime-") {
                let _ = std::fs::copy(entry.path(), state.join(&name));
            }
        }
    }
    write_private(&paths.prime_flag(), "").is_ok()
}
