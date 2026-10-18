//! The menu bar item: template glyph, the whole-rotation number as its
//! title, a left click for the popover and a right click for a short menu.

use std::sync::Mutex;

use quota_station_core::icon::{render_glyph, Icon};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

use crate::state::{AppState, Tab};
use crate::windows;

const TRAY_ID: &str = "quota-station";
/// Pixels per point of the glyph: crisp on Retina menu bars.
const GLYPH_SCALE: u32 = 2;

/// The icon last drawn, so an unchanged read does not redraw it.
static LAST: Mutex<Option<Icon>> = Mutex::new(None);

pub fn build(app: &App) -> tauri::Result<()> {
    let window = MenuItem::with_id(app, "window", "Mở cửa sổ chính", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Làm mới", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Cài đặt…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Thoát Quota Station", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&window, &refresh, &settings, &PredefinedMenuItem::separator(app)?, &quit],
    )?;
    let blank = Icon {
        title: "–".into(),
        tone: quota_station_core::Tone::Muted,
        claude: None,
        agy: None,
        show_agy: true,
        paused: false,
        label: "Quota Station".into(),
    };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(glyph(&blank))
        .icon_as_template(true)
        .title(&blank.title)
        .tooltip(&blank.label)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
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
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn glyph(icon: &Icon) -> Image<'static> {
    let (rgba, w, h) = render_glyph(icon, GLYPH_SCALE);
    Image::new_owned(rgba, w, h)
}

pub fn apply(app: &AppHandle, icon: &Icon) {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_ref() == Some(icon) {
        return;
    }
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let _ = tray.set_icon(Some(glyph(icon)));
    let _ = tray.set_icon_as_template(true);
    let _ = tray.set_title(Some(&icon.title));
    let _ = tray.set_tooltip(Some(&icon.label));
    *last = Some(icon.clone());
}
