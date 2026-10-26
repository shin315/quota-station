// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Quota Station: AI coding quota in the macOS menu bar.
//!
//! Each Mac runs its own copy and switches only its own machine. It reads
//! the quota of Claude, Codex and Antigravity itself (it read them through
//! quota-axi until it stood alone); since M3 it also switches Claude when the owner
//! confirms, and pauses the rotation. Since the Claude accounts moved in
//! (`docs/claude-accounts.md`) it keeps their logins itself, reads their
//! usage, runs the rotation from its own LaunchAgent and runs a process as a
//! chosen account (`quota-station claude …`), without claude-swap.
//! Since M5 it edits the rotation file and copies it to the owner's other
//! Macs over SSH, so every Mac's switcher follows the same rules.
//! Since M4a it logs Antigravity accounts in itself, since M4b it reads
//! their quota itself and lists it on the Antigravity row, and since M4c
//! it switches the Antigravity account without Cockpit Tools. Since M6 it
//! keeps a history of the switches, sends five alerts as macOS cards with
//! buttons, and shows what Firstmate sees. Since M7 it can launch at login
//! (a switch in settings, off by default) and update itself from a clone of
//! its source with `scripts/install.sh`. Since M8 it switches Codex between
//! the ChatGPT login and a custom API provider, keeping the ChatGPT login.
//! Since M9 it shows the MCP servers of Claude Code, Codex and Antigravity
//! in one table and turns them on, off, adds and removes them, after a
//! preview of the whole batch. Since M10 it backs them up to iCloud Drive
//! and restores them. Since M11 it does the same for the skills, shows the
//! plugins and the instruction files, and says when a skill from GitHub has
//! a newer version. Since M13 it copies them, item by item, to or from
//! another Mac over SSH, where that Mac's own Quota Station writes them.
//! Since M16 it counts the tokens in this Mac's own session logs (Thống kê),
//! keeping the counts only.

mod agents;
#[cfg(feature = "antigravity-accounts")]
mod agy_quota;
#[cfg(feature = "antigravity-accounts")]
mod agy_switch;
mod alerting;
#[cfg(feature = "antigravity-accounts")]
mod antigravity;
mod backup;
mod claude;
mod codex;
mod commands;
#[cfg(feature = "author-tools")]
mod copy;
mod env;
mod install;
mod login_item;
mod notify;
mod reader;
mod release;
#[cfg(feature = "author-tools")]
mod rules;
mod sets;
mod skills;
mod state;
mod stats;
mod tray;
mod windows;

use std::process::Command;

use tauri::Manager;

use crate::state::{Inner, Shared, Tab};

fn main() {
    // Another Mac's Quota Station runs `quota-station agents <word>` here
    // over SSH (M13): answered without a window, and nothing else starts.
    // Only a build with the author's tools copies; any other refuses as
    // a copy of another version would, and never opens a window.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("agents") {
        #[cfg(feature = "author-tools")]
        std::process::exit(copy::cli(&args[1..]));
        #[cfg(not(feature = "author-tools"))]
        {
            println!("{}", serde_json::json!({ "error": "protocol" }));
            std::process::exit(quota_station_core::agents::COPY_REFUSED);
        }
    }
    // The Claude accounts from a terminal, Firstmate and the rotation's
    // LaunchAgent: answered without a window.
    if args.first().map(String::as_str) == Some("claude") {
        std::process::exit(claude_cli(&args[1..]));
    }
    // The quota as the app reads it, from a terminal.
    if args.first().map(String::as_str) == Some("quota") {
        std::process::exit(quota_cli(&args[1..]));
    }
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
            let first_run = !state.lock().settings.setup_seen;
            windows::apply_theme(app.handle(), theme);
            tray::build(app)?;
            // A new install opens on the first run's steps, once.
            if first_run {
                windows::open_main(app.handle(), Tab::Overview);
            }
            if let Some(popover) = app.get_webview_window(windows::POPOVER) {
                let handle = app.handle().clone();
                popover.on_window_event(move |event| windows::on_popover_event(&handle, event));
            }
            reader::spawn(app.handle().clone(), state.clone());
            #[cfg(feature = "antigravity-accounts")]
            agy_quota::spawn(app.handle().clone(), state.clone());
            #[cfg(feature = "author-tools")]
            rules::spawn(app.handle().clone(), state.clone());
            codex::spawn(app.handle().clone(), state);
            Ok(())
        })
        .invoke_handler(handler())
        .run(tauri::generate_context!())
        .expect("Quota Station failed to start");
}

/// Every command the windows can call: [`handler`] for Tauri, and their
/// names for the tests. An entry with a `cfg` is built only with that
/// feature, so a build without it has no such command at all.
macro_rules! commands {
    ($( $(#[cfg($c:meta)])? $module:ident :: $name:ident ),* $(,)?) => {
        fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
            tauri::generate_handler![$( $(#[cfg($c)])? $module::$name ),*]
        }

        #[cfg(test)]
        fn command_names() -> Vec<&'static str> {
            let mut names = Vec::new();
            $( $(#[cfg($c)])? names.push(stringify!($name)); )*
            names
        }
    };
}

commands![
    commands::get_model,
    commands::refresh,
    commands::open_window,
    commands::main_tab,
    commands::settings_group,
    commands::set_settings_group,
    commands::hide_popover,
    commands::popover_resize,
    commands::set_machine_name,
    commands::set_show_names,
    commands::set_theme,
    commands::set_theme_pack,
    commands::set_language,
    commands::finish_setup,
    commands::reduce_transparency,
    commands::increase_contrast,
    commands::set_menubar,
    commands::set_menubar_window,
    #[cfg(feature = "author-tools")]
    commands::set_advanced,
    #[cfg(feature = "author-tools")]
    commands::set_firstmate,
    commands::grant_keychain,
    commands::claude_sessions,
    commands::switch_claude,
    commands::pause_switching,
    commands::extend_pause,
    commands::resume_switching,
    claude::claude_import,
    claude::claude_add_login,
    claude::claude_cancel_login,
    claude::claude_add_current,
    claude::claude_add_token,
    claude::claude_remove,
    claude::claude_set_alias,
    claude::claude_set_enabled,
    claude::claude_set_rotation,
    #[cfg(feature = "author-tools")]
    rules::save_rotation,
    #[cfg(feature = "author-tools")]
    rules::adopt_rotation,
    #[cfg(feature = "author-tools")]
    rules::keep_rotation,
    #[cfg(feature = "author-tools")]
    rules::retry_rotation,
    #[cfg(feature = "author-tools")]
    rules::set_peers,
    #[cfg(feature = "author-tools")]
    rules::test_peer,
    #[cfg(feature = "author-tools")]
    rules::preview_rotation,
    commands::rules_dirty,
    commands::take_quit,
    commands::close_main,
    #[cfg(feature = "antigravity-accounts")]
    antigravity::agy_login,
    #[cfg(feature = "antigravity-accounts")]
    antigravity::agy_login_callback,
    #[cfg(feature = "antigravity-accounts")]
    antigravity::agy_login_cancel,
    #[cfg(feature = "antigravity-accounts")]
    antigravity::agy_logout,
    #[cfg(feature = "antigravity-accounts")]
    antigravity::agy_open_verification,
    #[cfg(feature = "antigravity-accounts")]
    antigravity::agy_tab_opened,
    #[cfg(feature = "antigravity-accounts")]
    agy_switch::agy_app_state,
    #[cfg(feature = "antigravity-accounts")]
    agy_switch::switch_agy,
    #[cfg(feature = "antigravity-accounts")]
    agy_switch::agy_dismiss_overwrite,
    codex::codex_switch,
    codex::codex_key_preview,
    codex::codex_set_key,
    codex::codex_undo_key,
    codex::set_codex_plugins,
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
    skills::skills_read,
    skills::skills_preview,
    skills::skills_write,
    skills::skills_undo,
    skills::skills_drop_link,
    skills::skills_list_repo,
    skills::skills_read_md,
    skills::skills_install,
    skills::skills_update,
    skills::skills_update_many,
    skills::skills_skip,
    skills::skills_remove,
    skills::skills_import,
    skills::skills_open_link,
    release::release_check,
    release::release_skip,
    release::release_later,
    release::set_release_checks,
    release::release_open,
    release::release_diagnostics,
    skills::skills_check,
    skills::skills_set_checks,
    env::env_check,
    env::env_open,
    env::env_ignore,
    env::env_unignore,
    sets::sets_save,
    sets::sets_delete,
    sets::sets_undelete,
    sets::sets_preview,
    sets::sets_apply,
    sets::sets_undo,
    stats::stats_read,
    stats::stats_report,
    stats::stats_ack,
    skills::plugins_read,
    skills::instr_read,
    skills::instr_show,
    skills::instr_compare,
    skills::instr_include,
    skills::instr_not_key,
    skills::instr_add,
    skills::instr_forget,
    skills::instr_open,
    backup::backup_read,
    backup::backup_now,
    backup::backup_set_auto,
    backup::backup_set_folder,
    backup::backup_open_folder,
    backup::backup_delete_key,
    backup::pack_summary,
    backup::restore_plan,
    backup::restore_run,
    backup::restore_retry,
    backup::restore_undo,
    #[cfg(feature = "author-tools")]
    copy::copy_compare,
    #[cfg(feature = "author-tools")]
    copy::copy_run,
    #[cfg(feature = "author-tools")]
    copy::copy_undo,
    alerting::alert_action,
    alerting::alerts_read,
    alerting::test_alert,
    alerting::set_alert_switch,
    alerting::set_pool_alert,
    #[cfg(feature = "author-tools")]
    alerting::set_firstmate_file,
    alerting::get_history,
    alerting::notify_refresh,
    alerting::open_notification_settings,
    install::settings_opened,
    install::set_launch_at_login,
    install::open_login_items,
    #[cfg(feature = "author-tools")]
    install::set_source_dir,
    #[cfg(feature = "author-tools")]
    install::update_from_source,
    #[cfg(feature = "author-tools")]
    install::relaunch,
];

/// `quota-station claude …`, with the owner's home and the Keychain.
fn claude_cli(args: &[String]) -> i32 {
    use quota_station_core::claude::{cli, keychain::Security, rotate, store::Store, Paths};
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        eprintln!("quota-station claude: HOME is not set");
        return 1;
    };
    let cli = cli::Cli {
        store: Store::new(Paths::for_home(&home), Security::system()),
        tools: rotate::Tools::system(&home),
        exe: std::env::current_exe().unwrap_or_else(|_| "quota-station".into()),
    };
    cli::run(&cli, args, &mut std::io::stdout(), &mut std::io::stderr())
}

const QUOTA_USAGE: &str = "\
usage: quota-station quota [--provider claude|agy|codex]... [--json]

  Reads the quota now, as the app reads it, read-only: no login is
  refreshed, the app's own files and caches are left as they are, and
  nothing printed holds a token or an email.
";

/// `quota-station quota …`: one fresh read of each provider asked for (all
/// three by default), printed as the app's rows. The readings go to a
/// cache folder of their own, removed after, so the app's are untouched;
/// the Claude row of a Mac where the app keeps the accounts is built from
/// its files, without a read.
fn quota_cli(args: &[String]) -> i32 {
    use quota_station_core::app_dir::AppDir;
    use quota_station_core::claude::{self, keychain::Security, store::Store, usage, Paths};
    use quota_station_core::quota::{self, cache::Cache, ProviderId, Response};
    use quota_station_core::settings::Settings;
    use std::io::Write;

    let mut wanted: Vec<ProviderId> = Vec::new();
    let mut json = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => json = true,
            "--provider" => match it.next().map(String::as_str) {
                Some("claude") => wanted.push(ProviderId::Claude),
                Some("agy") => wanted.push(ProviderId::Agy),
                Some("codex") => wanted.push(ProviderId::Codex),
                _ => {
                    eprint!("{QUOTA_USAGE}");
                    return 2;
                }
            },
            _ => {
                eprint!("{QUOTA_USAGE}");
                return if a == "help" || a == "--help" { 0 } else { 2 };
            }
        }
    }
    if wanted.is_empty() {
        wanted = vec![ProviderId::Claude, ProviderId::Agy, ProviderId::Codex];
    }
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        eprintln!("quota-station quota: HOME is not set");
        return 1;
    };
    let app = AppDir::new(claude::app_dir(&home));
    let scratch = std::env::temp_dir().join(format!("quota-station-quota-{}", std::process::id()));
    if std::fs::create_dir(&scratch).is_err() {
        eprintln!("quota-station quota: no folder for the readings");
        return 1;
    }
    let own = AppDir::new(&scratch);
    let now = time::OffsetDateTime::now_utc();
    let mut rows = Vec::new();
    for provider in wanted {
        let report = match provider {
            ProviderId::Claude => {
                let store = Store::new(Paths::for_home(&home), Security::system());
                quota::claude::own_now(&store, now).unwrap_or_else(|| {
                    let live = quota::claude::Live {
                        paths: &store.paths,
                        security: &store.security,
                        endpoints: &usage::Endpoints::anthropic(),
                        app: &own,
                    };
                    let cache = Cache::new(&own, ProviderId::Claude);
                    quota::claude::read(&live, quota::claude::Keychain::Quiet, &cache, 0, now)
                })
            }
            ProviderId::Agy => {
                let cache = Cache::new(&own, ProviderId::Agy);
                #[allow(unused_mut)]
                let mut report = quota::agy::read(&quota::agy::Tools::system(), &cache, 0, now);
                #[cfg(feature = "antigravity-accounts")]
                if let Some(row) = report.primary_mut("agy") {
                    quota_station_core::antigravity::listing::attach(row, &app, now);
                }
                report
            }
            ProviderId::Codex => {
                let settings = Settings::load(&app.path().join("settings.json"));
                let codex_home =
                    quota_station_core::codex::home(&home, settings.codex_home.as_deref());
                let cache = Cache::new(&own, ProviderId::Codex);
                let endpoints = quota::codex::Endpoints::chatgpt();
                quota::codex::read(&codex_home, &endpoints, &cache, 0, now)
            }
        };
        rows.extend(report.providers);
    }
    let _ = std::fs::remove_dir_all(&scratch);
    let report = Response::new(now, rows);
    let mut out = std::io::stdout();
    if json {
        let text = serde_json::to_string_pretty(&report).expect("serializes");
        let _ = writeln!(out, "{text}");
        return 0;
    }
    for row in &report.providers {
        let at = row.state.refreshed_at.as_deref().unwrap_or("-");
        let _ = writeln!(out, "{} {} {at}", row.provider, row.state.status);
        if let Some(e) = &row.state.error {
            let _ = writeln!(out, "  {e}");
        }
        for w in &row.windows {
            let left = w.percent_remaining.map_or("-".to_string(), |p| format!("{p}%"));
            let _ = writeln!(
                out,
                "  {:<20} {left:>5} left  {}",
                w.id,
                w.resets_at.as_deref().unwrap_or("-")
            );
        }
    }
    0
}

/// The name macOS shows for this Mac (System Settings › General › About).
pub(crate) fn machine_name() -> String {
    Command::new("/usr/sbin/scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Mac này".to_string())
}

#[cfg(test)]
mod tests {
    use super::command_names;

    /// The commands of the author's tools: Nâng cao and its Firstmate
    /// switch, the rules editor and the other Macs, the copy to another
    /// Mac, Firstmate's state file, and the update from source.
    const AUTHOR_COMMANDS: [&str; 16] = [
        "set_advanced",
        "set_firstmate",
        "save_rotation",
        "adopt_rotation",
        "keep_rotation",
        "retry_rotation",
        "set_peers",
        "test_peer",
        "preview_rotation",
        "copy_compare",
        "copy_run",
        "copy_undo",
        "set_firstmate_file",
        "set_source_dir",
        "update_from_source",
        "relaunch",
    ];

    #[test]
    fn the_author_tools_commands_come_only_with_the_feature() {
        let names = command_names();
        let tools = cfg!(feature = "author-tools");
        for command in AUTHOR_COMMANDS {
            assert_eq!(names.contains(&command), tools, "{command}");
        }
        // Every build keeps the rest: the quit guard, the pause, the alerts,
        // the Claude accounts' own rotation switch.
        for command in [
            "get_model",
            "rules_dirty",
            "take_quit",
            "close_main",
            "pause_switching",
            "resume_switching",
            "set_alert_switch",
            "set_pool_alert",
            "claude_set_rotation",
            "set_launch_at_login",
            "finish_setup",
        ] {
            assert!(names.contains(&command), "{command}");
        }
    }
}
