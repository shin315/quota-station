//! Quota Station: AI coding quota in the macOS menu bar.
//!
//! Each Mac runs its own copy and switches only its own machine. It reads
//! through quota-axi; since M3 it also switches Claude through claude-swap
//! when the owner confirms, and pauses the `cswap-prefer-team` switcher.
//! Since M5 it edits the rotation file and copies it to the owner's other
//! Macs over SSH, so every Mac's switcher follows the same rules.
//! Since M4a it logs Antigravity accounts in itself, and since M4b it reads
//! their quota itself and leaves it where quota-axi reads it.

mod agy_quota;
mod antigravity;
mod commands;
mod reader;
mod rules;
mod state;
mod tray;
mod windows;

use std::process::Command;

use tauri::Manager;

use crate::state::{Inner, Shared, Tab};

fn main() {
    tauri::Builder::default()
        // A second launch shows this copy's main window instead of a second icon.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::open_main(app, Tab::Overview);
        }))
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let home = app.path().home_dir()?;
            let settings_file = app.path().app_config_dir()?.join("settings.json");
            let state = Shared::new(Inner::new(home, settings_file, machine_name()));
            app.manage(state.clone());

            tray::build(app)?;
            if let Some(popover) = app.get_webview_window(windows::POPOVER) {
                let handle = app.handle().clone();
                popover.on_window_event(move |event| windows::on_popover_event(&handle, event));
            }
            reader::spawn(app.handle().clone(), state.clone());
            agy_quota::spawn(app.handle().clone(), state.clone());
            rules::spawn(app.handle().clone(), state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_model,
            commands::refresh,
            commands::open_window,
            commands::main_tab,
            commands::hide_popover,
            commands::popover_resize,
            commands::set_quota_axi_path,
            commands::set_machine_name,
            commands::grant_keychain,
            commands::claude_sessions,
            commands::switch_claude,
            commands::pause_switching,
            commands::extend_pause,
            commands::resume_switching,
            rules::save_rotation,
            rules::adopt_rotation,
            rules::keep_rotation,
            rules::retry_rotation,
            rules::set_peers,
            rules::test_peer,
            rules::preview_rotation,
            rules::rules_dirty,
            rules::take_quit,
            rules::close_main,
            antigravity::agy_login,
            antigravity::agy_login_callback,
            antigravity::agy_login_cancel,
            antigravity::agy_logout,
            antigravity::agy_open_verification,
            antigravity::agy_tab_opened,
            antigravity::set_show_email,
        ])
        .run(tauri::generate_context!())
        .expect("Quota Station failed to start");
}

/// The name macOS shows for this Mac (System Settings › General › About).
fn machine_name() -> String {
    Command::new("/usr/sbin/scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Mac này".to_string())
}
