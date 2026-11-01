// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Quota Station: AI coding quota in the macOS menu bar.
//!
//! Each Mac runs its own copy and switches only its own machine. It reads
//! the quota of Claude, Codex and Antigravity itself (it read them through
//! quota-axi until it stood alone); since M3 it also switches Claude when the owner
//! confirms. Since the Claude accounts moved in (`docs/claude-accounts.md`)
//! it keeps their logins itself, reads their usage and runs a process as a
//! chosen account (`quota-station claude …`), without claude-swap.
//! Since M4a it logs Antigravity accounts in itself, since M4b it reads
//! their quota itself and lists it on the Antigravity row, and since M4c
//! it switches the Antigravity account without Cockpit Tools. Since M6 it
//! keeps a history of the switches and sends alerts as macOS cards with
//! buttons. Since M7 it can launch at login (a switch in settings, off by
//! default). Since M8 it switches Codex between the ChatGPT login and a
//! custom API provider, keeping the ChatGPT login. Since M9 it shows the MCP
//! servers of Claude Code, Codex and Antigravity in one table and turns them
//! on, off, adds and removes them, after a preview of the whole batch. Since
//! M10 it backs them up to iCloud Drive and restores them. Since M11 it does
//! the same for the skills, shows the plugins and the instruction files, and
//! says when a skill from GitHub has a newer version. Since M16 it counts the
//! tokens in this Mac's own session logs (Thống kê), keeping the counts only.

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
mod cli_tools;
mod codex;
mod codex_accounts;
mod codex_import;
mod codex_wake;
mod commands;
mod diag;
mod env;
pub mod ext;
mod install;
mod login_item;
mod notify;
mod open;
mod reader;
mod release;
mod session;
mod sets;
#[cfg(windows)]
mod shot;
mod skills;
mod state;
mod stats;
mod tray;
mod windows;
#[cfg(windows)]
mod winsys;

#[cfg(not(windows))]
use std::process::Command;

use tauri::Manager;

use crate::ext::Extension;
use crate::state::{Inner, Shared, Tab};

/// The app: a subcommand from a terminal (`quota-station claude …`,
/// `quota-station quota …`), else the menu bar app, with what `extension`
/// adds ([`ext`]). The binary (`main.rs`) only calls it, with the Tauri
/// context of its own `tauri.conf.json` and the extension it is built with.
pub fn run(context: tauri::Context<tauri::Wry>, extension: impl Extension) {
    // The extension lasts as long as the app: the state keeps its hooks.
    let extension = Box::leak(Box::new(extension));
    let hooks = extension.hooks();
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if matches!(args.first().map(String::as_str), Some("claude" | "quota")) {
        attach_console();
    }
    if args.first().map(String::as_str) == Some("agents") {
        // `quota-station agents …` is refused, as by a copy of another
        // version, without a window.
        {
            println!("{}", serde_json::json!({ "error": "protocol" }));
            std::process::exit(quota_station_core::agents::COPY_REFUSED);
        }
    }
    // The Claude accounts from a terminal and the programs that use them:
    // answered without a window.
    if args.first().map(String::as_str) == Some("claude") {
        std::process::exit(claude_cli(&args[1..]));
    }
    // The quota as the app reads it, from a terminal.
    if args.first().map(String::as_str) == Some("quota") {
        std::process::exit(quota_cli(&args[1..]));
    }
    let builder = tauri::Builder::default()
        // A second launch shows this copy's main window instead of a second
        // icon; on Windows, where the Start menu is how the app is found
        // again, the popover beside the tray icon.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            session::note("second-launch", serde_json::json!({}));
            #[cfg(not(windows))]
            windows::open_main(app, Tab::Overview);
            #[cfg(windows)]
            windows::show_popover(app);
        }))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());
    #[cfg(windows)]
    let builder = builder.plugin(winsys::webviews());
    // Windows's toasts with buttons (`notify`); macOS posts its own cards.
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_notification::init());
    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let home = app.path().home_dir()?;
            let settings_file = app.path().app_config_dir()?.join("settings.json");
            let mut inner = Inner::new(home, settings_file, machine_name(), hooks);
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
            #[cfg(windows)]
            windows::prepare_popover(app.handle());
            reader::spawn(app.handle().clone(), state.clone());
            #[cfg(feature = "antigravity-accounts")]
            agy_quota::spawn(app.handle().clone(), state.clone());
            codex_wake::spawn(app.handle().clone(), state.clone());
            codex::spawn(app.handle().clone(), state);
            // A test build's guided session (plan 4.3); nothing in any other.
            session::start(app.handle());
            Ok(())
        })
        .invoke_handler(route(handler(), extension.command_handler()))
        .run(context)
        .expect("Quota Station failed to start");
}

/// The windows' calls: a command the app has goes to the app's own handler,
/// any other to the extension's ([`ext::Extension::command_handler`]),
/// which refuses those it does not have either.
fn route(
    app: impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static,
    extension: ext::Handler,
) -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    let own: std::collections::HashSet<&'static str> = command_names().into_iter().collect();
    move |invoke| {
        if own.contains(invoke.message.command()) {
            app(invoke)
        } else {
            extension(invoke)
        }
    }
}

/// Every command of the app's own: [`handler`] for Tauri, and their names
/// for [`route`] and the tests. An entry with a `cfg` is built only with
/// that feature, so a build without it has no such command at all. The
/// extension's commands are its own ([`ext::Extension::command_names`]).
macro_rules! commands {
    ($( $(#[cfg($c:meta)])? $module:ident :: $name:ident ),* $(,)?) => {
        fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
            tauri::generate_handler![$( $(#[cfg($c)])? $module::$name ),*]
        }

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
    commands::grant_keychain,
    commands::claude_sessions,
    commands::switch_claude,
    claude::claude_import,
    claude::claude_add_login,
    claude::claude_cancel_login,
    claude::claude_add_current,
    claude::claude_add_token,
    claude::claude_remove,
    claude::claude_set_alias,
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
    codex_accounts::codex_use,
    codex_accounts::codex_login,
    codex_accounts::codex_cancel_login,
    codex_accounts::codex_reopen_login,
    codex_accounts::codex_check_provider,
    codex_accounts::codex_provider_preview,
    codex_accounts::codex_add_provider,
    codex_accounts::codex_rename,
    codex_accounts::codex_remove,
    codex_import::codex_import_pick,
    codex_import::codex_import_text,
    codex_import::codex_import,
    codex_wake::codex_trigger_set,
    codex_wake::codex_trigger_now,
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
    cli_tools::cli_tools_check,
    cli_tools::cli_tools_opened,
    cli_tools::cli_tool_update,
    cli_tools::set_cli_checks,
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
    alerting::alert_action,
    alerting::alerts_read,
    alerting::test_alert,
    alerting::set_alert_switch,
    alerting::set_pool_alert,
    alerting::get_history,
    alerting::notify_refresh,
    alerting::open_notification_settings,
    install::settings_opened,
    install::set_launch_at_login,
    install::open_login_items,
    session::session_get,
    session::session_step,
    session::session_done,
    session::session_shot,
    session::session_note,
    session::session_open,
    session::session_zip,
    session::session_reveal,
];

/// The home folder for the commands a terminal runs: `HOME`, which Windows
/// rarely sets, else the profile folder there.
fn cli_home() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME");
    #[cfg(windows)]
    let home = home.or_else(|| std::env::var_os("USERPROFILE"));
    home.map(std::path::PathBuf::from)
}

/// A release build on Windows has no console of its own: `quota-station
/// claude …` and `quota-station quota …` write to the terminal they were
/// started from, unless their output already goes somewhere (a pipe).
#[cfg(windows)]
fn attach_console() {
    use ::windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };
    // SAFETY: plain console calls on this process.
    unsafe {
        let has_output = GetStdHandle(STD_OUTPUT_HANDLE).is_ok_and(|h| !h.is_invalid());
        if !has_output {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

/// `quota-station claude …`, with the owner's home and the Keychain.
fn claude_cli(args: &[String]) -> i32 {
    use quota_station_core::claude::{cli, store::Store, Paths};
    let Some(home) = cli_home() else {
        eprintln!("quota-station claude: HOME is not set");
        return 1;
    };
    let cli = cli::Cli {
        store: Store::new(Paths::for_home(&home), quota_station_core::os::secret_store()),
        tools: cli::Tools::system(&home),
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
    use quota_station_core::claude::{self, store::Store, usage, Paths};
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
    let Some(home) = cli_home() else {
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
                let store =
                    Store::new(Paths::for_home(&home), quota_station_core::os::secret_store());
                quota::claude::own_now(&store, now).unwrap_or_else(|| {
                    let live = quota::claude::Live {
                        paths: &store.paths,
                        security: &*store.security,
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
#[cfg(not(windows))]
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

/// The name Windows shows for this computer (Settings › System › About).
#[cfg(windows)]
pub(crate) fn machine_name() -> String {
    winsys::computer_name().unwrap_or_else(|| "Máy này".to_string())
}

#[cfg(test)]
mod tests {
    use crate::ext::Extension;

    /// The extension the binary passes in this build (`main.rs`).
    fn extension() -> crate::ext::NoExtension {
        crate::ext::NoExtension
    }

    /// The commands the windows can call in this build: the app's own and
    /// those of the extension its binary passes.
    fn command_names() -> Vec<&'static str> {
        let mut names = super::command_names();
        names.extend(extension().command_names());
        names
    }

    #[test]
    fn every_build_has_the_shared_commands() {
        let names = command_names();
        for command in [
            "get_model",
            "switch_claude",
            "claude_set_alias",
            "set_alert_switch",
            "set_pool_alert",
            "set_launch_at_login",
            "finish_setup",
        ] {
            assert!(names.contains(&command), "{command}");
        }
    }
}
