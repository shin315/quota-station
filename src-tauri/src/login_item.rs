//! The app as a login item through macOS's `SMAppService.mainApp`: the
//! entry in System Settings › General › Login Items, with no plist of our
//! own. Only the switch in settings calls [`MainApp::register`]; nothing
//! registers it on install or at start.

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

/// Elsewhere there are no login items to offer.
#[cfg(not(target_os = "macos"))]
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

#[cfg(not(target_os = "macos"))]
pub fn open_settings() {}
