// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app as a login item through macOS's `SMAppService.mainApp`: the
//! entry in System Settings › General › Login Items, with no plist of our
//! own. Only the switch in settings calls [`MainApp::register`]; nothing
//! registers it on install or at start.
//!
//! On Windows it is the current user's `Run` key, written by `auto-launch`
//! (never the machine's, which needs an administrator): a value named
//! after the product, which the uninstaller removes. The owner can turn it
//! off in Settings › Apps › Startup or Task Manager, which leaves the value
//! and marks it off beside it: the app shows that as waiting for the
//! owner's approval there, as macOS's Login Items do.

use quota_station_core::login_item::{Error, LoginItems, Registered, Status};

/// This app, as macOS's login items know it.
pub struct MainApp;

#[cfg(target_os = "macos")]
mod mac {
    use objc2_service_management::{
        kSMErrorAlreadyRegistered, kSMErrorJobNotFound, kSMErrorLaunchDeniedByUser, SMAppService,
        SMAppServiceStatus,
    };

    use super::{Error, LoginItems, MainApp, Registered, Status};

    impl LoginItems for MainApp {
        fn status(&self) -> Status {
            if !crate::notify::bundled() {
                return Status::Unavailable;
            }
            // SAFETY: plain ServiceManagement calls on the app's own service.
            let status = unsafe { SMAppService::mainAppService().status() };
            match status {
                SMAppServiceStatus::Enabled => Status::On,
                SMAppServiceStatus::RequiresApproval => Status::NeedsApproval,
                // `NotFound` is what macOS reports for an app it has never
                // registered as well.
                _ => Status::Off,
            }
        }

        fn register(&self) -> Result<Registered, Error> {
            if !crate::notify::bundled() {
                return Err(Error::Unavailable);
            }
            // SAFETY: as above.
            match unsafe { SMAppService::mainAppService().registerAndReturnError() } {
                Ok(()) => Ok(Registered::Done),
                Err(e) if e.code() as u32 == kSMErrorLaunchDeniedByUser => {
                    Ok(Registered::NeedsApproval)
                }
                Err(e) if e.code() as u32 == kSMErrorAlreadyRegistered => Ok(Registered::Done),
                Err(e) => Err(Error::Failed(e.localizedDescription().to_string())),
            }
        }

        fn unregister(&self) -> Result<(), Error> {
            if !crate::notify::bundled() {
                return Err(Error::Unavailable);
            }
            // SAFETY: as above.
            match unsafe { SMAppService::mainAppService().unregisterAndReturnError() } {
                Ok(()) => Ok(()),
                Err(e) if e.code() as u32 == kSMErrorJobNotFound => Ok(()),
                Err(e) => Err(Error::Failed(e.localizedDescription().to_string())),
            }
        }
    }

    /// System Settings › General › Login Items, where the owner approves it.
    pub fn open_settings() {
        // SAFETY: opens System Settings; no arguments.
        unsafe { SMAppService::openSystemSettingsLoginItems() }
    }
}

#[cfg(target_os = "macos")]
pub use mac::open_settings;

#[cfg(windows)]
mod win {
    use auto_launch::{AutoLaunch, WindowsEnableMode};

    use super::{Error, LoginItems, MainApp, Registered, Status};

    /// The `Run` value's name: the product's, as the uninstaller removes it.
    const NAME: &str = "Quota Station";
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    /// This copy of the app, quoted: the installed path has a space.
    fn launcher() -> Result<AutoLaunch, Error> {
        let exe = std::env::current_exe().map_err(|e| Error::Failed(e.to_string()))?;
        let path = format!("\"{}\"", exe.display());
        Ok(AutoLaunch::new(NAME, &path, WindowsEnableMode::CurrentUser, &[] as &[&str]))
    }

    /// The value is in the current user's `Run` key, on or off.
    fn registered() -> bool {
        windows_registry::CURRENT_USER.open(RUN).and_then(|k| k.get_string(NAME)).is_ok()
    }

    impl LoginItems for MainApp {
        fn status(&self) -> Status {
            if !crate::notify::bundled() {
                return Status::Unavailable;
            }
            let Ok(launcher) = launcher() else { return Status::Unavailable };
            match launcher.is_enabled() {
                Ok(true) => Status::On,
                // Turned off in Settings › Apps › Startup or Task Manager.
                Ok(false) if registered() => Status::NeedsApproval,
                _ => Status::Off,
            }
        }

        /// Writes the value and marks it on, also when the owner had turned
        /// it off in Settings: the switch in the app asked for it.
        fn register(&self) -> Result<Registered, Error> {
            if !crate::notify::bundled() {
                return Err(Error::Unavailable);
            }
            launcher()?.enable().map_err(|e| Error::Failed(e.to_string()))?;
            Ok(Registered::Done)
        }

        fn unregister(&self) -> Result<(), Error> {
            if !crate::notify::bundled() {
                return Err(Error::Unavailable);
            }
            launcher()?.disable().map_err(|e| Error::Failed(e.to_string()))
        }
    }

    /// Settings › Apps › Startup, where the owner turns it on or off.
    pub fn open_settings(app: &tauri::AppHandle) {
        let _ = crate::open::url(app, "ms-settings:startupapps");
    }
}

#[cfg(windows)]
pub use win::open_settings;

/// Elsewhere there are no login items to offer.
#[cfg(not(any(target_os = "macos", windows)))]
impl LoginItems for MainApp {
    fn status(&self) -> Status {
        Status::Unavailable
    }
    fn register(&self) -> Result<Registered, Error> {
        Err(Error::Unavailable)
    }
    fn unregister(&self) -> Result<(), Error> {
        Err(Error::Unavailable)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn open_settings() {}
