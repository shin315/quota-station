//! Quota Station: AI coding quota in the macOS menu bar.
//!
//! Each Mac runs its own copy and switches only its own machine. It reads
//! through quota-axi; since M3 it also switches Claude through claude-swap
//! when the owner confirms, and pauses the `cswap-prefer-team` switcher.
//! Since M5 it edits the rotation file and copies it to the owner's other
//! Macs over SSH, so every Mac's switcher follows the same rules.
//! Since M4a it logs Antigravity accounts in itself, since M4b it reads
//! their quota itself and leaves it where quota-axi reads it, and since M4c
//! it switches the Antigravity account without Cockpit Tools. Since M6 it
//! keeps a history of the switches, sends five alerts as macOS cards with
//! buttons, and shows what Firstmate sees. Since M7 it can launch at login
//! (a switch in settings, off by default) and update itself from a clone of
//! its source with `scripts/install.sh`. Since M8 it switches Codex between
//! the ChatGPT login and a custom API provider, keeping the ChatGPT login.
//! Since M9 it shows the MCP servers of Claude Code, Codex and Antigravity
//! in one table and turns them on, off, adds and removes them, after a
//! preview of the whole batch.

mod agents;
mod agy_quota;
mod agy_switch;
mod alerting;
mod antigravity;
mod backup;
mod codex;
mod commands;
mod install;
mod login_item;
mod notify;
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
            let mut inner = Inner::new(home, settings_file, machine_name());
            inner.app_version = app.package_info().version.to_string();
            let state = Shared::new(inner);
            // Only asks macOS: the app never registers itself.
            install::read_login(&state);
            app.manage(state.clone());
            state.lock().center = Some(std::sync::Arc::new(alerting::start_center(app.handle())));

            let theme = state.lock().settings.theme;
            windows::apply_theme(app.handle(), theme);
            tray::build(app)?;
            if let Some(popover) = app.get_webview_window(windows::POPOVER) {
                let handle = app.handle().clone();
                popover.on_window_event(move |event| windows::on_popover_event(&handle, event));
            }
            reader::spawn(app.handle().clone(), state.clone());
            agy_quota::spawn(app.handle().clone(), state.clone());
            codex::spawn(app.handle().clone(), state.clone());
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
            commands::set_show_names,
            commands::set_theme,
            commands::set_menubar,
            commands::set_menubar_window,
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
            agy_switch::agy_app_state,
            agy_switch::switch_agy,
            agy_switch::agy_dismiss_overwrite,
            codex::codex_switch,
            codex::codex_restart,
            codex::codex_keep,
            codex::codex_check,
            codex::set_codex_home,
            agents::mcp_read,
            agents::mcp_preview,
            agents::mcp_write,
            agents::mcp_undo,
            agents::mcp_save_secret,
            agents::mcp_paste,
            agents::mcp_forget,
            agents::mcp_open_backups,
            backup::backup_read,
            backup::backup_now,
            backup::backup_set_auto,
            backup::backup_open_folder,
            backup::backup_delete_key,
            backup::pack_summary,
            backup::restore_plan,
            backup::restore_run,
            backup::restore_undo,
            alerting::alert_action,
            alerting::test_alert,
            alerting::set_alert_switch,
            alerting::set_firstmate_file,
            alerting::get_history,
            alerting::notify_refresh,
            alerting::open_notification_settings,
            install::settings_opened,
            install::set_launch_at_login,
            install::open_login_items,
            install::set_source_dir,
            install::update_from_source,
            install::relaunch,
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
