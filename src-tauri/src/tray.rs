// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The menu bar item: template glyph, the account in use and its share used
//! as its title, a left click for the popover and a right click for a short
//! menu, where "Thanh menu theo" picks the service the title follows and
//! "Số trên thanh menu" the window it shows.
//!
//! Windows shows no title beside a tray icon: there the icon draws the
//! title's number, white or black as the taskbar is dark or light, and the
//! tooltip leads with the title.

use std::sync::Mutex;

#[cfg(not(windows))]
use quota_station_core::icon::render_glyph;
use quota_station_core::icon::Icon;
use quota_station_core::settings::{Menubar, MenubarWindow};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, Wry};

use crate::commands;
use crate::state::{AppState, Tab};
use crate::windows;

pub(crate) const TRAY_ID: &str = "quota-station";
/// Pixels per point of the glyph: crisp on Retina menu bars.
#[cfg(not(windows))]
const GLYPH_SCALE: u32 = 2;

/// The icon last drawn and how, so an unchanged read does not redraw it.
static LAST: Mutex<Option<(Icon, Look)>> = Mutex::new(None);

/// How the icon is drawn beside what it shows: on Windows, its size in
/// pixels and whether the taskbar is light, either of which the owner can
/// change while the app runs; nothing more on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Look {
    #[cfg(windows)]
    size: u32,
    #[cfg(windows)]
    light: bool,
}

impl Look {
    #[cfg(windows)]
    fn now() -> Look {
        Look { size: crate::winsys::tray_icon_size(), light: crate::winsys::light_taskbar() }
    }

    #[cfg(not(windows))]
    fn now() -> Look {
        Look {}
    }
}

/// The submenus' names: Windows has a tray, not a menu bar.
#[cfg(not(windows))]
const FOLLOW_MENU: &str = "Thanh menu theo";
#[cfg(not(windows))]
const WINDOW_MENU: &str = "Số trên thanh menu";
#[cfg(windows)]
const FOLLOW_MENU: &str = "Icon khay theo";
#[cfg(windows)]
const WINDOW_MENU: &str = "Số trên icon khay";

/// What "Thanh menu theo" shows: the service picked, and which ones this
/// Mac has (the others are greyed out); and what "Số trên thanh menu" shows:
/// the window picked, Fable tuần greyed out unless the title follows Claude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Follow {
    pub picked: Menubar,
    pub gemini: bool,
    pub codex: bool,
    pub window: MenubarWindow,
    pub fable: bool,
}

/// The submenus' items, and what they last showed.
struct FollowMenu {
    items: [(Menubar, CheckMenuItem<Wry>); 3],
    windows: [(MenubarWindow, CheckMenuItem<Wry>); 4],
    last: Option<Follow>,
}

static FOLLOW: Mutex<Option<FollowMenu>> = Mutex::new(None);

const FOLLOW_IDS: [(Menubar, &str, &str); 3] = [
    (Menubar::Claude, "menubar:claude", "Claude"),
    (Menubar::Gemini, "menubar:gemini", "Gemini"),
    (Menubar::Codex, "menubar:codex", "Codex"),
];

const WINDOW_IDS: [(MenubarWindow, &str, &str); 4] = [
    (MenubarWindow::FiveHours, "window:fiveHours", "5 giờ"),
    (MenubarWindow::Week, "window:week", "Tuần"),
    (MenubarWindow::FableWeek, "window:fableWeek", "Fable tuần"),
    (MenubarWindow::Tightest, "window:tightest", "Cao nhất"),
];

pub fn build(app: &App) -> tauri::Result<()> {
    let window = MenuItem::with_id(app, "window", "Mở cửa sổ chính", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Làm mới", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Cài đặt…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Thoát Quota Station", true, None::<&str>)?;
    // The test build's guided session (`crate::session`) only.
    let guide = MenuItem::with_id(app, "guide", "Hướng dẫn phiên thử", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let mut items = Vec::new();
    for (menubar, id, text) in FOLLOW_IDS {
        let on = menubar == Menubar::Claude;
        items.push((menubar, CheckMenuItem::with_id(app, id, text, true, on, None::<&str>)?));
    }
    let follow =
        Submenu::with_items(app, FOLLOW_MENU, true, &[&items[0].1, &items[1].1, &items[2].1])?;
    let mut windows = Vec::new();
    for (window, id, text) in WINDOW_IDS {
        let on = window == MenubarWindow::default();
        windows.push((window, CheckMenuItem::with_id(app, id, text, true, on, None::<&str>)?));
    }
    let shows = Submenu::with_items(
        app,
        WINDOW_MENU,
        true,
        &[&windows[0].1, &windows[1].1, &windows[2].1, &windows[3].1],
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &window,
            &refresh,
            &PredefinedMenuItem::separator(app)?,
            &follow,
            &shows,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    if crate::session::ON {
        menu.insert_items(&[&guide, &separator], 0)?;
    }
    if let (Ok(items), Ok(windows)) = (<[_; 3]>::try_from(items), <[_; 4]>::try_from(windows)) {
        *FOLLOW.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(FollowMenu { items, windows, last: None });
    }
    let blank = Icon {
        title: "–".into(),
        tone: quota_station_core::Tone::Muted,
        used: None,
        claude: None,
        agy: None,
        show_agy: true,
        paused: false,
        label: "Quota Station".into(),
    };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(glyph(&blank, Look::now()))
        .icon_as_template(true)
        .title(&blank.title)
        .tooltip(tooltip(&blank))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            #[cfg(windows)]
            if let TrayIconEvent::Click { rect, .. } = &event {
                windows::tray_clicked(rect);
            }
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_popover(app);
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "refresh" => app.state::<AppState>().request_read(),
            "window" => windows::open_main(app, Tab::Overview),
            "settings" => windows::open_main(app, Tab::Settings),
            "quit" => windows::quit(app),
            "guide" => crate::session::open_guide(app),
            id => {
                if let Some((menubar, _, _)) = FOLLOW_IDS.iter().find(|(_, i, _)| *i == id) {
                    let state = app.state::<AppState>();
                    // The click ticked or unticked the item by itself: set
                    // every tick again on the next publish, which also puts
                    // the old choice back when the file cannot be written.
                    follow_menu(None);
                    if commands::save_menubar(app, &state, *menubar).is_err() {
                        crate::reader::publish(app, &state);
                    }
                } else if let Some((window, _, _)) = WINDOW_IDS.iter().find(|(_, i, _)| *i == id) {
                    let state = app.state::<AppState>();
                    follow_menu(None);
                    if commands::save_menubar_window(app, &state, *window).is_err() {
                        crate::reader::publish(app, &state);
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(not(windows))]
fn glyph(icon: &Icon, _look: Look) -> Image<'static> {
    let (rgba, w, h) = render_glyph(icon, GLYPH_SCALE);
    Image::new_owned(rgba, w, h)
}

/// The number drawn at the size the taskbar shows it, so it stays sharp.
#[cfg(windows)]
fn glyph(icon: &Icon, look: Look) -> Image<'static> {
    let rgba = quota_station_core::icon::render_tray(icon, look.size, look.light);
    Image::new_owned(rgba, look.size, look.size)
}

#[cfg(not(windows))]
fn tooltip(icon: &Icon) -> String {
    icon.label.clone()
}

#[cfg(windows)]
fn tooltip(icon: &Icon) -> String {
    quota_station_core::icon::tray_tip(icon)
}

/// Ticks the service and the window picked and greys out the services this
/// Mac lacks, and Fable tuần unless the title follows Claude. `None`
/// forgets what was shown, so the next call sets every item again (a click
/// ticks an item on its own). The items are set after the lock is let go:
/// from the reader thread each call waits for the main thread, which may be
/// handling a click that needs the lock.
fn follow_menu(follow: Option<Follow>) {
    let (items, windows, follow) = {
        let mut menu = FOLLOW.lock().unwrap_or_else(|e| e.into_inner());
        let Some(menu) = menu.as_mut() else { return };
        let Some(follow) = follow else {
            menu.last = None;
            return;
        };
        if menu.last == Some(follow) {
            return;
        }
        menu.last = Some(follow);
        (menu.items.clone(), menu.windows.clone(), follow)
    };
    for (menubar, item) in &items {
        let _ = item.set_checked(*menubar == follow.picked);
        let _ = item.set_enabled(match menubar {
            Menubar::Claude => true,
            Menubar::Gemini => follow.gemini,
            Menubar::Codex => follow.codex,
        });
    }
    for (window, item) in &windows {
        let _ = item.set_checked(*window == follow.window);
        let _ = item.set_enabled(*window != MenubarWindow::FableWeek || follow.fable);
    }
}

pub fn apply(app: &AppHandle, icon: &Icon, follow: Follow) {
    follow_menu(Some(follow));
    let look = Look::now();
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_ref().is_some_and(|(i, l)| i == icon && *l == look) {
        return;
    }
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let image = glyph(icon, look);
    #[cfg(windows)]
    crate::session::tray_drawn(&icon.title, look.size, look.light, image.rgba());
    let _ = tray.set_icon(Some(image));
    let _ = tray.set_icon_as_template(true);
    let _ = tray.set_title(Some(&icon.title));
    let _ = tray.set_tooltip(Some(tooltip(icon)));
    *last = Some((icon.clone(), look));
}
