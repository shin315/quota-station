//! The popover under the menu bar item and the settings window.

use std::time::{Duration, Instant};

use tauri::{
    AppHandle, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::state::AppState;

pub const POPOVER: &str = "popover";
pub const SETTINGS: &str = "settings";
/// `--pop-w` in the design tokens.
const POPOVER_WIDTH: f64 = 348.0;
/// `--pop-max`: never taller than a 13" MacBook Air screen allows.
const POPOVER_MAX_HEIGHT: f64 = 820.0;
/// A click on the menu bar item first takes focus from an open popover,
/// which hides it; the click that follows must not open it again.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

fn popover(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(POPOVER)
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
    let read = state.lock().schedule.set_visible(true, Instant::now());
    if read {
        state.request_read();
    }
}

pub fn hide_popover(app: &AppHandle) {
    if let Some(window) = popover(app) {
        let _ = window.hide();
    }
    let state = app.state::<AppState>();
    let mut inner = state.lock();
    inner.schedule.set_visible(false, Instant::now());
    inner.popover_hidden_at = Some(Instant::now());
}

/// The popover closes when it loses focus, like a system popover.
pub fn on_popover_event(app: &AppHandle, event: &WindowEvent) {
    if let WindowEvent::Focused(false) = event {
        if popover(app).is_some_and(|w| w.is_visible().unwrap_or(false)) {
            hide_popover(app);
        }
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

pub fn open_settings(app: &AppHandle) {
    hide_popover(app);
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title("Cài đặt · Quota Station")
        .inner_size(600.0, 330.0)
        .min_inner_size(520.0, 300.0)
        .center()
        .build();
    if let Ok(window) = built {
        let _ = window.set_focus();
    }
}
