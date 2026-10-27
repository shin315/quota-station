// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the app asks of Windows itself: the taskbar's theme and the size
//! of a tray icon, whether the windows can draw on a material (Transparency
//! effects, High contrast, Windows 11), the popover's rounded corners, the
//! computer's name, and WebView2 without a browser's keys and context menu.

use std::ffi::c_void;
use std::sync::OnceLock;

use ::windows::core::{w, Interface, PCWSTR, PWSTR};
use ::windows::Wdk::System::SystemServices::RtlGetVersion;
use ::windows::Win32::Foundation::ERROR_SUCCESS;
use ::windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use ::windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use ::windows::Win32::System::SystemInformation::{
    ComputerNamePhysicalDnsHostname, GetComputerNameExW, OSVERSIONINFOW,
};
use ::windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use ::windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use ::windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SM_CXSMICON, SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};
use ::windows::UI::ViewManagement::UISettings;
use tauri::plugin::TauriPlugin;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Runtime, Webview, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;

/// The first build of Windows 11: Mica, and corners DWM rounds on request.
const WINDOWS_11: u32 = 22000;

const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

/// The build of Windows that runs, `0` when it cannot be told.
fn build() -> u32 {
    static BUILD: OnceLock<u32> = OnceLock::new();
    *BUILD.get_or_init(|| {
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        // SAFETY: the struct carries its own size, as the call asks.
        let status = unsafe { RtlGetVersion(&mut info) };
        if status.is_ok() {
            info.dwBuildNumber
        } else {
            0
        }
    })
}

/// A DWORD under the owner's `HKEY_CURRENT_USER`.
fn dword(key: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `data` holds the DWORD the flags ask for, `size` says so.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast::<c_void>()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// Whether the taskbar is light (Settings › Personalization › Colors ›
/// "Choose your default Windows mode"): the tray icon is then drawn black,
/// otherwise white. Windows 10 before 1903 has no light taskbar.
pub fn light_taskbar() -> bool {
    dword(PERSONALIZE, w!("SystemUsesLightTheme")) == Some(1)
}

/// The pixels of a small icon at the system's scale: what the taskbar
/// shows a tray icon at (16 at 100 %, 24 at 150 %).
pub fn tray_icon_size() -> u32 {
    // SAFETY: plain metrics getters.
    let px = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) };
    u32::try_from(px).ok().filter(|px| *px >= 16).unwrap_or(16)
}

/// Transparency effects (Settings › Personalization › Colors): when off,
/// Windows draws its own materials solid. Asked of `UISettings`, then of
/// the setting's registry value.
fn transparency_effects() -> bool {
    UISettings::new()
        .and_then(|s| s.AdvancedEffectsEnabled())
        .ok()
        .or_else(|| dword(PERSONALIZE, w!("EnableTransparency")).map(|on| on != 0))
        .unwrap_or(true)
}

/// Whether a High contrast theme is on (Settings › Accessibility ›
/// Contrast themes).
pub fn high_contrast() -> bool {
    let mut contrast =
        HIGHCONTRASTW { cbSize: size_of::<HIGHCONTRASTW>() as u32, ..Default::default() };
    // SAFETY: the struct carries its own size, as SPI_GETHIGHCONTRAST asks.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some((&raw mut contrast).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.is_ok() && contrast.dwFlags.0 & HCF_HIGHCONTRASTON.0 != 0
}

/// Whether the windows draw on a material: Windows 11, with Transparency
/// effects on and no High contrast theme. Otherwise the page draws its
/// solid ground.
pub fn material() -> bool {
    build() >= WINDOWS_11 && transparency_effects() && !high_contrast()
}

/// Puts `effect` behind `window` when the windows draw on a material, and
/// takes it away when they no longer do. Tauri applies a Windows effect
/// without saying whether it took, so the choice is made here first.
pub fn set_material(window: &WebviewWindow, effect: Effect) {
    let effects = material().then(|| EffectsBuilder::new().effect(effect).build());
    let _ = window.set_effects(effects);
}

/// Asks DWM to round the corners of `window`, which has no frame of its
/// own; Windows 10 keeps them square.
pub fn round_corners(window: &WebviewWindow) {
    if build() < WINDOWS_11 {
        return;
    }
    let Ok(hwnd) = window.hwnd() else { return };
    let round = DWMWCP_ROUND;
    // SAFETY: a DWORD-sized attribute on the window's own handle.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const round).cast::<c_void>(),
            size_of_val(&round) as u32,
        )
    };
}

/// The computer's name as Settings › System › About shows it.
pub fn computer_name() -> Option<String> {
    let mut size = 0u32;
    // SAFETY: the first call only says how long the name is.
    let _ = unsafe { GetComputerNameExW(ComputerNamePhysicalDnsHostname, None, &mut size) };
    if size == 0 {
        return None;
    }
    let mut name = vec![0u16; size as usize];
    // SAFETY: `name` holds `size` characters, as the call is told.
    unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            Some(PWSTR(name.as_mut_ptr())),
            &mut size,
        )
    }
    .ok()?;
    let name = String::from_utf16_lossy(&name[..(size as usize).min(name.len())]);
    Some(name.trim().to_string()).filter(|n| !n.is_empty())
}

/// The context menu stays only where text is edited (cut, copy, paste):
/// elsewhere it would offer Back, Reload, Save as and Print.
const CONTEXT_MENU: &str = r#"document.addEventListener("contextmenu", (event) => {
  const target = event.target;
  const editable = target instanceof Element
    && target.closest("input, textarea, [contenteditable]:not([contenteditable=false])");
  if (!editable) event.preventDefault();
}, { capture: true });"#;

/// Every webview without a browser's ways: no context menu outside text
/// fields, and no browser keys (F5 and Ctrl+R reload, Ctrl+F find, Ctrl+P
/// print, F12, Alt+Left back); copy, paste and undo keep working.
pub fn webviews<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("windows-webview")
        .js_init_script(CONTEXT_MENU.to_string())
        .on_webview_ready(|webview| browser_keys_off(&webview))
        .build()
}

fn browser_keys_off<R: Runtime>(webview: &Webview<R>) {
    let _ = webview.with_webview(|platform| {
        // SAFETY: COM calls on the webview's own controller, on the thread
        // that runs it.
        unsafe {
            let Ok(core) = platform.controller().CoreWebView2() else { return };
            let Ok(settings) = core.Settings() else { return };
            if let Ok(settings) = settings.cast::<ICoreWebView2Settings3>() {
                let _ = settings.SetAreBrowserAcceleratorKeysEnabled(false);
            }
        }
    });
}
