// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's command that holds a Claude account out of the
//! rotation or puts it back (`quota_station_core::advanced::held`), run
//! through the host as the app runs its own account actions
//! ([`Host::claude_action`]): one at a time, off the main thread, then the
//! accounts taken in again and read.

use quota_station_core::advanced::held;
use tauri::AppHandle;

use crate::ext::{claude_done, Host, Model};

/// Puts the account in the rotation (`enabled`), or holds it out (it can
/// still be switched to by hand).
#[tauri::command]
pub async fn claude_set_enabled(app: AppHandle, slot: u32, enabled: bool) -> Result<Model, String> {
    Host::of(&app)
        .claude_action("enabled", move |store| {
            claude_done("enabled", slot, held::set_enabled(store, slot, enabled))
        })
        .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::ext::{claude_done, ManageError};

    #[test]
    fn holding_an_account_out_ends_as_the_claude_tab_shows_it() {
        assert_eq!(
            serde_json::to_value(claude_done("enabled", 2, Ok(()))).unwrap(),
            json!({
                "action": "enabled", "ok": true, "error": null, "detail": null,
                "slot": 2, "renewed": false, "imported": null,
            })
        );
        assert_eq!(
            serde_json::to_value(claude_done("enabled", 9, Err(ManageError::UnknownSlot))).unwrap(),
            json!({
                "action": "enabled", "ok": false, "error": "unknownSlot", "detail": null,
                "slot": null, "renewed": false, "imported": null,
            })
        );
    }
}
