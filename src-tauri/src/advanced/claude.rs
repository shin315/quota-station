// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's command that turns the rotation's LaunchAgent on or
//! off (on Windows the app's own timer), run through the host as the app
//! runs its own actions on the Claude accounts
//! ([`Host::claude_action`]): one at a time, off the main thread, then the
//! accounts taken in again and read.

use quota_station_core::claude::agent::AgentError;
use quota_station_core::os::Scheduler;
use tauri::AppHandle;

use crate::ext::{Done, Host, Model};

/// Loads the rotation's LaunchAgent (`on`), which runs `quota-station
/// claude rotate` every three minutes whether the app is open or not, or
/// unloads it; on Windows turns the app's own timer on or off (`timer`).
/// Refused while an older switcher is loaded.
#[tauri::command]
pub async fn claude_set_rotation(app: AppHandle, on: bool) -> Result<Model, String> {
    let host = Host::of(&app);
    let home = host.home();
    host.claude_action("rotation", move |_| {
        let scheduler = quota_station_core::os::scheduler();
        rotation_done(match (on, std::env::current_exe()) {
            (true, Ok(exe)) => scheduler.install(&home, &exe),
            (true, Err(e)) => Err(AgentError::Io(e.to_string())),
            (false, _) => scheduler.remove(&home),
        })
    })
    .await
}

/// How turning the rotation on or off ended, as the Claude tab shows it:
/// refused for an older switcher (`legacy`, with its label), or failed
/// otherwise (`launchctl`, with why).
fn rotation_done(result: Result<(), AgentError>) -> Done {
    let (error, detail) = match result {
        Ok(()) => (None, None),
        Err(AgentError::Legacy(label)) => (Some("legacy".to_string()), Some(label)),
        Err(e) => (Some("launchctl".to_string()), Some(e.to_string())),
    };
    Done {
        action: "rotation",
        ok: error.is_none(),
        error,
        detail,
        slot: None,
        renewed: false,
        imported: None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn turning_the_rotation_on_or_off_ends_as_the_claude_tab_shows_it() {
        let shown = |result| serde_json::to_value(rotation_done(result)).unwrap();
        assert_eq!(
            shown(Ok(())),
            json!({
                "action": "rotation", "ok": true, "error": null, "detail": null,
                "slot": null, "renewed": false, "imported": null,
            })
        );
        assert_eq!(
            shown(Err(AgentError::Legacy("com.example.older".into()))),
            json!({
                "action": "rotation", "ok": false, "error": "legacy",
                "detail": "com.example.older", "slot": null, "renewed": false, "imported": null,
            })
        );
        let failed = AgentError::Io("no such file".into());
        let why = failed.to_string();
        assert_eq!(
            shown(Err(failed)),
            json!({
                "action": "rotation", "ok": false, "error": "launchctl", "detail": why,
                "slot": null, "renewed": false, "imported": null,
            })
        );
    }
}
