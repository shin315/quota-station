// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The popover under the menu bar item and the main window.
//!
//! On Windows the popover opens above the tray icon (or beside it, where
//! the taskbar sits), inside the work area, and the main window keeps
//! Windows' own title bar (decision 2 A): Mica behind it on Windows 11,
//! Acrylic behind the popover, a solid page where neither can be drawn.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use tauri::window::EffectState;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, TitleBarStyle};
#[cfg(not(windows))]
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
/// The test build's pictures (`crate::session`) wait this long for the
/// popover to take its place and its material, and the main window to draw.
const POPOVER_SETTLES: Duration = Duration::from_millis(700);
const MAIN_SETTLES: Duration = Duration::from_millis(900);
/// The traffic lights sit inside the sidebar, where the prototype draws them.
#[cfg(target_os = "macos")]
const TRAFFIC_LIGHTS: (f64, f64) = (22.0, 26.0);
/// The room Windows leaves between a flyout and the taskbar, and between
/// the popover and the edges of the work area.
#[cfg(windows)]
const POPOVER_MARGIN: f64 = 12.0;

fn popover(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(POPOVER)
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// macOS's Reduce Transparency and Increase Contrast as the windows last
/// heard them; on Windows, whether the windows go without a material, and
/// High contrast.
static REDUCE_TRANSPARENCY: AtomicBool = AtomicBool::new(false);
static INCREASE_CONTRAST: AtomicBool = AtomicBool::new(false);

/// Whether macOS reduces transparency (System Settings › Accessibility ›
/// Display). Its own materials then draw solid, and the page shows the
/// solid ground instead of the glass tint.
#[cfg(target_os = "macos")]
pub fn reduce_transparency() -> bool {
    use objc2::{msg_send, runtime::Bool};
    let on = workspace().is_some_and(|w| {
        // SAFETY: a plain BOOL getter on the shared NSWorkspace.
        let on: Bool = unsafe { msg_send![w, accessibilityDisplayShouldReduceTransparency] };
        on.as_bool()
    });
    REDUCE_TRANSPARENCY.store(on, Ordering::Relaxed);
    on
}

/// Whether macOS increases contrast (the same pane): the page draws the
/// edges of its glass and its panels stronger.
#[cfg(target_os = "macos")]
pub fn increase_contrast() -> bool {
    use objc2::{msg_send, runtime::Bool};
    let on = workspace().is_some_and(|w| {
        // SAFETY: a plain BOOL getter on the shared NSWorkspace.
        let on: Bool = unsafe { msg_send![w, accessibilityDisplayShouldIncreaseContrast] };
        on.as_bool()
    });
    INCREASE_CONTRAST.store(on, Ordering::Relaxed);
    on
}

/// The shared NSWorkspace.
#[cfg(target_os = "macos")]
fn workspace() -> Option<&'static objc2::runtime::AnyObject> {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    let class = AnyClass::get(c"NSWorkspace")?;
    // SAFETY: `+[NSWorkspace sharedWorkspace]` returns the shared instance,
    // which lives as long as the app; null is checked.
    unsafe {
        let workspace: *mut AnyObject = msg_send![class, sharedWorkspace];
        workspace.as_ref()
    }
}

/// Whether the windows go without a material on Windows: before Windows
/// 11, with Transparency effects off or a High contrast theme on. The page
/// then shows the solid ground, as under macOS's Reduce Transparency.
#[cfg(windows)]
pub fn reduce_transparency() -> bool {
    let on = !crate::winsys::material();
    REDUCE_TRANSPARENCY.store(on, Ordering::Relaxed);
    on
}

/// Whether a High contrast theme is on: the page draws stronger edges.
#[cfg(windows)]
pub fn increase_contrast() -> bool {
    let on = crate::winsys::high_contrast();
    INCREASE_CONTRAST.store(on, Ordering::Relaxed);
    on
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn reduce_transparency() -> bool {
    false
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn increase_contrast() -> bool {
    false
}

/// Windows: Acrylic behind the popover and Mica behind the main window
/// while the windows draw on a material, none otherwise.
#[cfg(windows)]
fn set_materials(app: &AppHandle) {
    if let Some(window) = popover(app) {
        crate::winsys::set_material(&window, Effect::Acrylic);
    }
    if let Some(window) = main_window(app) {
        crate::winsys::set_material(&window, Effect::Mica);
    }
}

/// Windows: the popover from `tauri.windows.conf.json` gets its corners
/// and its material once the app starts.
#[cfg(windows)]
pub fn prepare_popover(app: &AppHandle) {
    reduce_transparency();
    increase_contrast();
    if let Some(window) = popover(app) {
        crate::winsys::round_corners(&window);
    }
    set_materials(app);
}

/// A window came forward: the owner may have changed Reduce Transparency or
/// Increase Contrast while it was behind, so both windows hear a setting
/// that changed.
fn check_material(app: &AppHandle) {
    let before = REDUCE_TRANSPARENCY.load(Ordering::Relaxed);
    let now = reduce_transparency();
    if now != before {
        #[cfg(windows)]
        set_materials(app);
        let _ = app.emit("reduceTransparency", now);
        crate::session::note("material", serde_json::json!({ "reduceTransparency": now }));
    }
    let before = INCREASE_CONTRAST.load(Ordering::Relaxed);
    let now = increase_contrast();
    if now != before {
        let _ = app.emit("increaseContrast", now);
        crate::session::note("material", serde_json::json!({ "increaseContrast": now }));
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
    #[cfg(not(windows))]
    let _ = window.move_window_constrained(Position::TrayBottomCenter);
    #[cfg(windows)]
    place_popover(app, &window, None);
    let _ = window.show();
    let _ = window.set_focus();
    update_visible(app);
    crate::session::shoot(app, "popover", POPOVER_SETTLES);
}

/// Shows the popover under the menu bar item, for a button on a macOS card
/// that opens a switch sheet there, and on Windows for a second launch.
pub fn show_popover(app: &AppHandle) {
    let Some(window) = popover(app) else { return };
    if !window.is_visible().unwrap_or(false) {
        #[cfg(not(windows))]
        let _ = window.move_window_constrained(Position::TrayBottomCenter);
        #[cfg(windows)]
        place_popover(app, &window, None);
        let _ = window.show();
        crate::session::shoot(app, "popover", POPOVER_SETTLES);
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
#[cfg(not(windows))]
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

/// Fits the popover to its content, up to `--pop-max` and the work area,
/// and keeps it against the taskbar as it grows or shrinks.
#[cfg(windows)]
pub fn resize_popover(app: &AppHandle, height: f64) {
    let Some(window) = popover(app) else { return };
    place_popover(app, &window, Some(height));
}

/// Where the tray icon was last clicked, in physical pixels.
#[cfg(windows)]
static TRAY_AREA: std::sync::Mutex<Option<place::Area>> = std::sync::Mutex::new(None);

/// Keeps where a click on the tray icon landed: the popover opens there.
#[cfg(windows)]
pub fn tray_clicked(rect: &tauri::Rect) {
    let position = rect.position.to_physical::<i32>(1.0);
    let size = rect.size.to_physical::<u32>(1.0);
    let area = place::Area::new(position.x, position.y, size.width, size.height);
    *TRAY_AREA.lock().unwrap_or_else(|e| e.into_inner()) = area;
}

/// The tray icon's rectangle in physical pixels (left, top, width,
/// height), for the test build's pictures of the tray corner.
#[cfg(windows)]
pub(crate) fn tray_rect(app: &AppHandle) -> Option<(i32, i32, i32, i32)> {
    tray_area(app).map(|a| (a.x, a.y, a.w, a.h))
}

/// Where the tray icon is: where it was last clicked, else what Windows
/// says now (a second launch opens the popover without a click).
#[cfg(windows)]
fn tray_area(app: &AppHandle) -> Option<place::Area> {
    if let Some(area) = *TRAY_AREA.lock().unwrap_or_else(|e| e.into_inner()) {
        return Some(area);
    }
    let rect = app.tray_by_id(crate::tray::TRAY_ID)?.rect().ok().flatten()?;
    let position = rect.position.to_physical::<i32>(1.0);
    let size = rect.size.to_physical::<u32>(1.0);
    place::Area::new(position.x, position.y, size.width, size.height)
}

/// Sizes the popover to `height` (its height now when `None`) and puts it
/// against the taskbar beside the tray icon, inside the work area of the
/// screen the icon is on. Sizes are logical, so 125 % and 150 % scale the
/// page as the design draws it.
#[cfg(windows)]
fn place_popover(app: &AppHandle, window: &WebviewWindow, height: Option<f64>) {
    let tray = tray_area(app);
    let monitor = tray
        .and_then(|t| {
            let (x, y) = t.centre();
            app.monitor_from_point(f64::from(x), f64::from(y)).ok().flatten()
        })
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let height = height
        .or_else(|| {
            let size = window.inner_size().ok()?;
            Some(f64::from(size.height) / window.scale_factor().ok()?)
        })
        .unwrap_or(POPOVER_MAX_HEIGHT);
    let room = f64::from(work.size.height) / scale - 2.0 * POPOVER_MARGIN;
    let height = height.clamp(120.0, POPOVER_MAX_HEIGHT.min(room));
    let screen = place::Area {
        x: monitor.position().x,
        y: monitor.position().y,
        w: monitor.size().width as i32,
        h: monitor.size().height as i32,
    };
    let work = place::Area {
        x: work.position.x,
        y: work.position.y,
        w: work.size.width as i32,
        h: work.size.height as i32,
    };
    let size = ((POPOVER_WIDTH * scale).round() as i32, (height * scale).round() as i32);
    let margin = (POPOVER_MARGIN * scale).round() as i32;
    let (x, y) = place::popover(tray, screen, work, size, margin);
    note_placed(tray, work, (x, y), size, margin, scale);
    let at = tauri::PhysicalPosition::new(x, y);
    // Moved first, so a screen of another scale has already rescaled the
    // window when it takes its logical size; moved again in case that
    // rescale shifted it.
    let _ = window.set_position(at);
    let _ = window.set_size(LogicalSize::new(POPOVER_WIDTH, height));
    let _ = window.set_position(at);
}

/// The test build's record of where the popover went (`crate::session`):
/// the icon, the work area, the scale, and whether the popover stayed
/// `margin` inside the work area. Only a place that changed is noted.
#[cfg(windows)]
fn note_placed(
    tray: Option<place::Area>,
    work: place::Area,
    at: (i32, i32),
    size: (i32, i32),
    margin: i32,
    scale: f64,
) {
    static LAST: std::sync::Mutex<Option<[i32; 4]>> = std::sync::Mutex::new(None);
    if !crate::session::ON {
        return;
    }
    {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        let placed = [at.0, at.1, size.0, size.1];
        if *last == Some(placed) {
            return;
        }
        *last = Some(placed);
    }
    let inside = at.0 >= work.x + margin
        && at.1 >= work.y + margin
        && at.0 + size.0 <= work.x + work.w - margin
        && at.1 + size.1 <= work.y + work.h - margin;
    crate::session::note(
        "popover",
        serde_json::json!({
            "tray": tray.map(|t| [t.x, t.y, t.w, t.h]),
            "work": [work.x, work.y, work.w, work.h],
            "at": [at.0, at.1],
            "size": [size.0, size.1],
            "scale": scale,
            "insideWithMargin": inside,
            "gapBelow": work.y + work.h - (at.1 + size.1),
            "margin": margin,
        }),
    );
}

/// Where the popover goes on Windows, worked out in physical pixels.
#[cfg(any(windows, test))]
mod place {
    /// A rectangle on the desktop, in physical pixels.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Area {
        pub x: i32,
        pub y: i32,
        pub w: i32,
        pub h: i32,
    }

    impl Area {
        /// `None` for an empty rectangle: Windows gives one when it cannot
        /// tell where the icon is.
        pub fn new(x: i32, y: i32, w: u32, h: u32) -> Option<Area> {
            let (w, h) = (i32::try_from(w).ok()?, i32::try_from(h).ok()?);
            (w > 0 && h > 0).then_some(Area { x, y, w, h })
        }

        pub fn centre(&self) -> (i32, i32) {
            (self.x + self.w / 2, self.y + self.h / 2)
        }

        fn right(&self) -> i32 {
            self.x + self.w
        }

        fn bottom(&self) -> i32 {
            self.y + self.h
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Edge {
        Bottom,
        Top,
        Left,
        Right,
    }

    /// The edge the taskbar sits on: where the work area leaves the most of
    /// the screen out; with the taskbar hidden, the edge nearest the icon;
    /// knowing neither, the bottom.
    fn taskbar(tray: Option<Area>, screen: Area, work: Area) -> Edge {
        let insets = [
            (Edge::Bottom, screen.bottom() - work.bottom()),
            (Edge::Top, work.y - screen.y),
            (Edge::Left, work.x - screen.x),
            (Edge::Right, screen.right() - work.right()),
        ];
        let (edge, inset) =
            insets.into_iter().fold((Edge::Bottom, 0), |a, b| if b.1 > a.1 { b } else { a });
        if inset > 0 {
            return edge;
        }
        let Some(tray) = tray else { return Edge::Bottom };
        let (x, y) = tray.centre();
        let distances = [
            (Edge::Bottom, screen.bottom() - y),
            (Edge::Top, y - screen.y),
            (Edge::Left, x - screen.x),
            (Edge::Right, screen.right() - x),
        ];
        distances.into_iter().fold((Edge::Bottom, i32::MAX), |a, b| if b.1 < a.1 { b } else { a }).0
    }

    /// The popover's top left corner: against the taskbar's side of the
    /// work area, centred on the tray icon along it (at the far end, where
    /// the tray is, when the icon cannot be told), and kept `margin` inside
    /// the work area. An icon in the hidden icons' flyout lies inside the
    /// work area; the popover still opens against the taskbar, beside it.
    pub fn popover(
        tray: Option<Area>,
        screen: Area,
        work: Area,
        size: (i32, i32),
        margin: i32,
    ) -> (i32, i32) {
        let (w, h) = size;
        let (cx, cy) = tray.map_or((work.right(), work.bottom()), |t| t.centre());
        let (x, y) = match taskbar(tray, screen, work) {
            Edge::Bottom => (cx - w / 2, work.bottom() - h - margin),
            Edge::Top => (cx - w / 2, work.y + margin),
            Edge::Left => (work.x + margin, cy - h / 2),
            Edge::Right => (work.right() - w - margin, cy - h / 2),
        };
        let clamp = |v: i32, low: i32, high: i32| if high < low { low } else { v.clamp(low, high) };
        (
            clamp(x, work.x + margin, work.right() - w - margin),
            clamp(y, work.y + margin, work.bottom() - h - margin),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const SCREEN: Area = Area { x: 0, y: 0, w: 1920, h: 1080 };
        const POPOVER: (i32, i32) = (360, 560);

        fn icon(x: i32, y: i32) -> Option<Area> {
            Area::new(x, y, 24, 40)
        }

        #[test]
        fn above_a_bottom_taskbar_centred_on_the_icon() {
            let work = Area { x: 0, y: 0, w: 1920, h: 1032 };
            let at = popover(icon(1500, 1036), SCREEN, work, POPOVER, 12);
            assert_eq!(at, (1512 - 180, 1032 - 560 - 12));
        }

        #[test]
        fn an_icon_near_the_corner_keeps_the_popover_on_screen() {
            let work = Area { x: 0, y: 0, w: 1920, h: 1032 };
            let at = popover(icon(1890, 1036), SCREEN, work, POPOVER, 12);
            assert_eq!(at, (1920 - 360 - 12, 1032 - 560 - 12));
        }

        #[test]
        fn at_150_percent_the_sizes_are_physical() {
            // 2880 x 1620 at 150 %: a 48 px taskbar is 72 px.
            let screen = Area { x: 0, y: 0, w: 2880, h: 1620 };
            let work = Area { x: 0, y: 0, w: 2880, h: 1548 };
            let at = popover(icon(2400, 1560), screen, work, (540, 840), 18);
            assert_eq!(at, (2412 - 270, 1548 - 840 - 18));
        }

        #[test]
        fn below_a_top_taskbar_and_beside_a_side_one() {
            let top = Area { x: 0, y: 48, w: 1920, h: 1032 };
            assert_eq!(popover(icon(1500, 4), SCREEN, top, POPOVER, 12), (1332, 60));
            let left = Area { x: 62, y: 0, w: 1858, h: 1080 };
            assert_eq!(popover(icon(19, 900), SCREEN, left, POPOVER, 12), (74, 1080 - 560 - 12));
            let right = Area { x: 0, y: 0, w: 1858, h: 1080 };
            assert_eq!(popover(icon(1878, 300), SCREEN, right, POPOVER, 12), (1858 - 372, 40));
        }

        #[test]
        fn a_second_screen_left_of_the_first() {
            let screen = Area { x: -1280, y: 0, w: 1280, h: 1024 };
            let work = Area { x: -1280, y: 0, w: 1280, h: 984 };
            let at = popover(icon(-200, 990), screen, work, POPOVER, 12);
            assert_eq!(at, (-1280 + 1280 - 360 - 12, 984 - 560 - 12));
        }

        #[test]
        fn a_hidden_taskbar_goes_by_the_icon() {
            let at = popover(icon(1500, 1050), SCREEN, SCREEN, POPOVER, 12);
            assert_eq!(at, (1332, 1080 - 560 - 12));
            let at = popover(icon(1500, 2), SCREEN, SCREEN, POPOVER, 12);
            assert_eq!(at, (1332, 12));
        }

        #[test]
        fn an_icon_in_the_hidden_icons_flyout_still_opens_against_the_taskbar() {
            let work = Area { x: 0, y: 0, w: 1920, h: 1032 };
            let at = popover(icon(1400, 960), SCREEN, work, POPOVER, 12);
            assert_eq!(at, (1232, 1032 - 560 - 12));
        }

        #[test]
        fn without_the_icon_the_popover_goes_to_the_trays_corner() {
            let work = Area { x: 0, y: 0, w: 1920, h: 1032 };
            assert_eq!(popover(None, SCREEN, work, POPOVER, 12), (1548, 460));
            assert_eq!(Area::new(0, 0, 0, 0), None);
        }

        #[test]
        fn a_popover_taller_than_the_work_area_starts_at_its_top() {
            let work = Area { x: 0, y: 0, w: 1366, h: 728 };
            let screen = Area { x: 0, y: 0, w: 1366, h: 768 };
            assert_eq!(popover(icon(1200, 740), screen, work, (360, 820), 12), (994, 12));
        }
    }
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
    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title("Quota Station")
        .min_inner_size(MAIN_MIN_SIZE.0, MAIN_MIN_SIZE.1);
    #[cfg(not(windows))]
    let builder = builder.inner_size(MAIN_SIZE.0, MAIN_SIZE.1);
    #[cfg(target_os = "macos")]
    let builder = builder
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
        );
    // Windows' own title bar (decision 2 A), dark or light as `set_theme`
    // asks, with Mica behind it and the page on Windows 11; the window
    // stays transparent so Mica can come and go with Transparency effects.
    #[cfg(windows)]
    let builder = {
        let (width, height) = main_size(app);
        let builder = builder.inner_size(width, height).transparent(true);
        if crate::winsys::material() {
            builder.effects(EffectsBuilder::new().effect(Effect::Mica).build())
        } else {
            builder
        }
    };
    let built = builder.center().build();
    if let Ok(window) = built {
        let handle = app.clone();
        window.on_window_event(move |event| match event {
            // Unsaved rules: the window asks first, and closes itself through
            // `close_main` once the owner answered.
            #[cfg(feature = "author-tools")]
            WindowEvent::CloseRequested { api, .. }
                if handle.state::<AppState>().lock().rules_dirty =>
            {
                api.prevent_close();
                let _ = handle.emit_to(MAIN, "closeRequested", ());
            }
            WindowEvent::Focused(true) => {
                check_material(&handle);
                crate::session::shoot(&handle, "main", MAIN_SETTLES);
            }
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

/// The design's size, within the work area of the main screen less
/// Windows' title bar: a 1366 by 768 laptop has less room than the design.
#[cfg(windows)]
fn main_size(app: &AppHandle) -> (f64, f64) {
    let Some(monitor) = app.primary_monitor().ok().flatten() else { return MAIN_SIZE };
    let scale = monitor.scale_factor();
    let work = monitor.work_area().size;
    let width = f64::from(work.width) / scale - 32.0;
    let height = f64::from(work.height) / scale - 72.0;
    (MAIN_SIZE.0.min(width).max(MAIN_MIN_SIZE.0), MAIN_SIZE.1.min(height).max(MAIN_MIN_SIZE.1))
}

/// Opens the main window on Cài đặt's `group` (as the page names it).
pub fn open_settings(app: &AppHandle, group: &str) {
    app.state::<AppState>().lock().settings_group = group.into();
    open_main(app, Tab::Settings);
}

/// The owner's look for the system's own parts of the windows (the
/// popover's material, the title bar; on Windows also Mica and Acrylic):
/// `None` follows the system.
pub fn apply_theme(app: &AppHandle, theme: quota_station_core::settings::Theme) {
    use quota_station_core::settings::Theme;
    app.set_theme(match theme {
        Theme::Auto => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    });
}

/// Closes the main window without asking again.
pub fn close_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.destroy();
    }
}

/// `Thoát Quota Station`: quits.
#[cfg(not(feature = "author-tools"))]
pub fn quit(app: &AppHandle) {
    app.exit(0);
}

/// `Thoát Quota Station`: quits, unless the rules editor holds unsaved
/// changes; then the main window asks first and quits through the
/// `close_main` command with `quit` set. The request is kept until the page
/// takes it (`take_quit`), whether it was listening already or opens now.
#[cfg(feature = "author-tools")]
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

    #[test]
    fn reads_increase_contrast_from_macos() {
        let on = increase_contrast();
        assert_eq!(INCREASE_CONTRAST.load(Ordering::Relaxed), on);
    }
}
