// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Pictures for the guided session of the test build (`crate::session`):
//! a part of the screen as DWM composed it (GDI's `BitBlt` with
//! `CAPTUREBLT`, so the tooltip, the popover's Acrylic and the hidden
//! icons' flyout are in it), saved as a JPEG.
//!
//! Only the app's own parts are taken: the popover with the tray icon
//! under it, the main window, the tray corner, the corner where toasts
//! show. Never the whole screen, where a browser, a terminal or Windows
//! Settings could show a name or an email. The guide's own window is left
//! out of every picture.

use ::windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    ROP_CODE, SRCCOPY,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SetWindowDisplayAffinity, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WDA_EXCLUDEFROMCAPTURE,
};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::windows;

/// JPEG quality: text stays readable, a picture stays a few hundred KB.
const QUALITY: u8 = 85;

/// Leaves `window` out of every picture of the screen (Windows 10 2004
/// and later; older ones show it).
pub fn exclude(window: &WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        // SAFETY: the window's own handle.
        let _ = unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) };
    }
}

/// A rectangle in physical pixels: left, top, width, height.
type Rect = (i32, i32, i32, i32);

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.0.min(b.0), a.1.min(b.1));
    let (r, bottom) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
    (x, y, r - x, bottom - y)
}

fn grow(a: Rect, by: i32) -> Rect {
    (a.0 - by, a.1 - by, a.2 + 2 * by, a.3 + 2 * by)
}

/// The part of the screen `what` names, or `None` when it is not shown.
pub fn area(app: &AppHandle, what: &str) -> Option<Rect> {
    let monitor = || app.primary_monitor().ok().flatten();
    match what {
        "popover" => {
            let window = app.get_webview_window(windows::POPOVER)?;
            if !window.is_visible().unwrap_or(false) {
                return None;
            }
            let at = window.outer_position().ok()?;
            let size = window.outer_size().ok()?;
            let scale = window.scale_factor().unwrap_or(1.0);
            let mut rect = (at.x, at.y, size.width as i32, size.height as i32);
            if let Some(tray) = windows::tray_rect(app) {
                rect = union(rect, tray);
            }
            Some(grow(rect, (24.0 * scale) as i32))
        }
        "main" => {
            let window = app.get_webview_window(windows::MAIN)?;
            if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
                return None;
            }
            let at = window.outer_position().ok()?;
            let size = window.outer_size().ok()?;
            Some((at.x, at.y, size.width as i32, size.height as i32))
        }
        "tray" => {
            let tray = windows::tray_rect(app);
            let m = tray
                .and_then(|t| {
                    let (x, y) = (t.0 + t.2 / 2, t.1 + t.3 / 2);
                    app.monitor_from_point(f64::from(x), f64::from(y)).ok().flatten()
                })
                .or_else(monitor)?;
            let half = (420.0 * m.scale_factor()) as i32;
            let (x, y) = match tray {
                Some(t) => (t.0 + t.2 / 2, t.1 + t.3 / 2),
                None => (
                    m.position().x + m.size().width as i32,
                    m.position().y + m.size().height as i32,
                ),
            };
            Some((x - half, y - half, 2 * half, 2 * half))
        }
        "toast" => {
            let m = monitor()?;
            let (w, h) = ((520.0 * m.scale_factor()) as i32, (560.0 * m.scale_factor()) as i32);
            let right = m.position().x + m.size().width as i32;
            let bottom = m.position().y + m.size().height as i32;
            Some((right - w, bottom - h, w, h))
        }
        _ => None,
    }
}

/// A JPEG of `x, y, w, h` on the desktop, clipped to it.
pub fn take(x: i32, y: i32, w: i32, h: i32) -> Option<Vec<u8>> {
    // SAFETY: plain metrics getters.
    let desk = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    let (left, top) = (x.max(desk.0), y.max(desk.1));
    let right = (x + w).min(desk.0 + desk.2);
    let bottom = (y + h).min(desk.1 + desk.3);
    let (w, h) = (right - left, bottom - top);
    if w <= 0 || h <= 0 || w > 65535 || h > 65535 {
        return None;
    }
    let mut pixels = vec![0u8; w as usize * h as usize * 4];
    // SAFETY: a memory DC and bitmap of our own, the screen DC released
    // and every GDI object deleted before returning; `pixels` holds the
    // `w * h` 32-bit rows `GetDIBits` is told to write.
    let lines = unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(memory, bitmap.into());
        let copied =
            BitBlt(memory, 0, 0, w, h, Some(screen), left, top, ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0))
                .is_ok();
        SelectObject(memory, old);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative: the rows top down.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = if copied {
            GetDIBits(
                memory,
                bitmap,
                0,
                h as u32,
                Some(pixels.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        lines
    };
    if lines <= 0 {
        return None;
    }
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, QUALITY)
        .encode(&pixels, w as u16, h as u16, jpeg_encoder::ColorType::Bgra)
        .ok()?;
    Some(jpeg)
}
