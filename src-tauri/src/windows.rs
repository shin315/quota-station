// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The popover under the menu bar item and the main window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, TitleBarStyle, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::{AppState, Tab};

pub const POPOVER: &str = "popover";
pub const MAIN: &str = "main";
/// `--pop-w` in the design tokens.
const POPOVER_WIDTH: f64 = 360.0;
/// `--pop-max-h`: never taller than a 13" MacBook Air screen allows.
const POPOVER_MAX_HEIGHT: f64 = 820.0;
/// A click on the menu bar item first takes focus from an open popover,
/// which hides it; the click that follows must not open it again.
const REOPEN_GUARD: Duration = Duration::from_millis(300);
/// `--win-w` and `--win-h` in the design tokens: the page draws under the
/// title bar, so the window is the size the design was drawn at.
const MAIN_SIZE: (f64, f64) = (1152.0, 864.0);
const MAIN_MIN_SIZE: (f64, f64) = (760.0, 480.0);
/// The traffic lights sit inside the sidebar, where the prototype draws them.
const TRAFFIC_LIGHTS: (f64, f64) = (22.0, 26.0);

fn popover(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(POPOVER)
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// macOS's Reduce Transparency as the windows last heard it.
static REDUCE_TRANSPARENCY: AtomicBool = AtomicBool::new(false);

/// Whether macOS reduces transparency (System Settings › Accessibility ›
/// Display). Its own materials then draw solid, and the page shows the
/// solid ground instead of the glass tint.
#[cfg(target_os = "macos")]
pub fn reduce_transparency() -> bool {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    let Some(class) = AnyClass::get(c"NSWorkspace") else { return false };
    // SAFETY: `+[NSWorkspace sharedWorkspace]` returns the shared instance,
    // and the property is a plain BOOL getter on it.
    unsafe {
        let workspace: *mut AnyObject = msg_send![class, sharedWorkspace];
        if workspace.is_null() {
            return false;
        }
        let on: Bool = msg_send![workspace, accessibilityDisplayShouldReduceTransparency];
        let on = on.as_bool();
        REDUCE_TRANSPARENCY.store(on, Ordering::Relaxed);
        on
    }
}

#[cfg(not(target_os = "macos"))]
pub fn reduce_transparency() -> bool {
    false
}

/// A window came forward: the owner may have changed Reduce Transparency
/// while it was behind, so both windows hear the setting when it changed.
fn check_material(app: &AppHandle) {
    let before = REDUCE_TRANSPARENCY.load(Ordering::Relaxed);
    let now = reduce_transparency();
    if now != before {
        let _ = app.emit("reduceTransparency", now);
    }
}

/// Tells the schedule what is on screen, and reads when a window just opened.
fn update_visible(app: &AppHandle) {
    let popover = popover(app).is_some_and(|w| w.is_visible().unwrap_or(false));
    let main = main_window(app).is_some_and(|w| w.is_visible().unwrap_or(false));
    let state = app.state::<AppState>();
    let read = state.lock().set_visible(popover, main);
    if read {
        state.request_read();
    }
}

pub fn toggle_popover(app: &AppHandle) {
    let Some(window) = popover(app) else { return };
    let state = app.state::<AppState>();
    if window.is_visible().unwrap_or(false) {
        hide_popover(app);
        return;
    }
    let just_hidden = state.lock().popover_hidden_at.is_some_and(|at| at.elapsed() < REOPEN_GUARD);
    if just_hidden {
        return;
    }
    let _ = window.move_window_constrained(Position::TrayBottomCenter);
    let _ = window.show();
    let _ = window.set_focus();
    update_visible(app);
}

/// Shows the popover under the menu bar item, for a button on a macOS card
/// that opens a switch sheet there.
pub fn show_popover(app: &AppHandle) {
    let Some(window) = popover(app) else { return };
    if !window.is_visible().unwrap_or(false) {
        let _ = window.move_window_constrained(Position::TrayBottomCenter);
        let _ = window.show();
    }
    let _ = window.set_focus();
    update_visible(app);
}

pub fn hide_popover(app: &AppHandle) {
    if let Some(window) = popover(app) {
        let _ = window.hide();
    }
    app.state::<AppState>().lock().popover_hidden_at = Some(Instant::now());
    update_visible(app);
}

/// The popover closes when it loses focus, like a system popover.
pub fn on_popover_event(app: &AppHandle, event: &WindowEvent) {
    match event {
        WindowEvent::Focused(false) => {
            if popover(app).is_some_and(|w| w.is_visible().unwrap_or(false)) {
                hide_popover(app);
            }
        }
        WindowEvent::Focused(true) => check_material(app),
        _ => {}
    }
}

/// Fits the popover to its content, up to `--pop-max` and the screen.
pub fn resize_popover(app: &AppHandle, height: f64) {
    let Some(window) = popover(app) else { return };
    let screen = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| f64::from(m.size().height) / m.scale_factor() - 40.0)
        .unwrap_or(POPOVER_MAX_HEIGHT);
    let height = height.clamp(120.0, POPOVER_MAX_HEIGHT.min(screen));
    let _ = window.set_size(LogicalSize::new(POPOVER_WIDTH, height));
}

/// Opens the main window on `tab`, or brings it forward and switches tab.
/// A new window asks for its tab once it has loaded (`main_tab`).
pub fn open_main(app: &AppHandle, tab: Tab) {
    hide_popover(app);
    app.state::<AppState>().lock().main_tab = tab;
    if let Some(window) = main_window(app) {
        let _ = window.emit_to(MAIN, "tab", tab);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        update_visible(app);
        return;
    }
    let built = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title("Quota Station")
        .inner_size(MAIN_SIZE.0, MAIN_SIZE.1)
        .min_inner_size(MAIN_MIN_SIZE.0, MAIN_MIN_SIZE.1)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(LogicalPosition::new(TRAFFIC_LIGHTS.0, TRAFFIC_LIGHTS.1))
        // The clear glass of style Q13: macOS draws the sidebar's material
        // behind the whole window and blurs the desktop; the page lays the
        // measured glass tint of the sidebar and the page on top of it.
        .transparent(true)
        .effects(
            EffectsBuilder::new()
                .effect(Effect::Sidebar)
                .state(EffectState::FollowsWindowActiveState)
                .build(),
        )
        .center()
        .build();
    if let Ok(window) = built {
        let handle = app.clone();
        window.on_window_event(move |event| match event {
            // Unsaved rules: the window asks first, and closes itself through
            // `close_main` once the owner answered.
            WindowEvent::CloseRequested { api, .. }
                if handle.state::<AppState>().lock().rules_dirty =>
            {
                api.prevent_close();
                let _ = handle.emit_to(MAIN, "closeRequested", ());
            }
            WindowEvent::Focused(true) => check_material(&handle),
            WindowEvent::Destroyed => {
                let popover = popover(&handle).is_some_and(|w| w.is_visible().unwrap_or(false));
                handle.state::<AppState>().lock().set_visible(popover, false);
            }
            _ => {}
        });
        let _ = window.set_focus();
        update_visible(app);
    }
}

/// Closes the main window without asking again.
/// The owner's look for macOS's own parts of the windows (the popover's
/// material, the title bar): `None` follows macOS.
pub fn apply_theme(app: &AppHandle, theme: quota_station_core::settings::Theme) {
    use quota_station_core::settings::Theme;
    app.set_theme(match theme {
        Theme::Auto => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    });
}

pub fn close_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.destroy();
    }
}

/// `Thoát Quota Station`: quits, unless the rules editor holds unsaved
/// changes; then the main window asks first and quits through the
/// `close_main` command with `quit` set. The request is kept until the page
/// takes it (`take_quit`), whether it was listening already or opens now.
pub fn quit(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut inner = state.lock();
        if !inner.rules_dirty {
            drop(inner);
            app.exit(0);
            return;
        }
        inner.quit_pending = true;
    }
    open_main(app, Tab::Rules);
    if let Some(window) = main_window(app) {
        let _ = window.emit_to(MAIN, "quitRequested", ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The call reaches AppKit's real property, whose type objc2 checks in a
    /// debug build, and keeps what it read for `check_material`.
    #[test]
    fn reads_reduce_transparency_from_macos() {
        let on = reduce_transparency();
        assert_eq!(REDUCE_TRANSPARENCY.load(Ordering::Relaxed), on);
    }
}
