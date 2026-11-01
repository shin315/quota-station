// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Mở khi đăng nhập" (M7): the app as a login item, through macOS's own
//! mechanism (`SMAppService.mainApp` in the app). Off until the owner turns
//! it on in settings; nothing registers it on install or at start, and
//! macOS keeps the answer, so the switch shows what System Settings shows.

use serde::Serialize;

/// The app as a login item, as macOS reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Off,
    On,
    /// Registered, but the owner has to allow it in System Settings ›
    /// General › Login Items first.
    NeedsApproval,
    /// Not a packaged, signed `.app` (as `tauri dev` runs it): macOS has no
    /// login item to offer.
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Unavailable,
    /// macOS refused, with its message.
    Failed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unavailable => write!(f, "unavailable"),
            Error::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

/// What registering asked of macOS answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Registered {
    Done,
    /// macOS took it but waits for the owner's approval.
    NeedsApproval,
}

/// macOS's login items for this app; a fake in the tests.
pub trait LoginItems {
    fn status(&self) -> Status;
    fn register(&self) -> Result<Registered, Error>;
    /// Unregistering what is not registered is not an error.
    fn unregister(&self) -> Result<(), Error>;
}

/// Turns launching at login on or off, and returns what macOS reports
/// after. Asks macOS only when the switch would change something.
pub fn set(items: &dyn LoginItems, on: bool) -> Result<Status, Error> {
    let now = items.status();
    match (now, on) {
        (Status::Unavailable, _) => return Err(Error::Unavailable),
        (Status::On | Status::NeedsApproval, true) | (Status::Off, false) => return Ok(now),
        (_, true) => {
            if items.register()? == Registered::NeedsApproval {
                return Ok(match items.status() {
                    Status::On => Status::On,
                    _ => Status::NeedsApproval,
                });
            }
        }
        (_, false) => items.unregister()?,
    }
    Ok(items.status())
}
