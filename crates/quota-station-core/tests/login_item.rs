// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Mở khi đăng nhập" through a fake login-item backend that keeps the
//! state macOS would and records every call. No test registers a real
//! login item.

use std::cell::RefCell;

use quota_station_core::login_item::{set, Error, LoginItems, Registered, Status};

struct Fake {
    status: RefCell<Status>,
    /// What macOS does with a registration: take it, or wait for approval.
    approval: bool,
    refuse: Option<String>,
    calls: RefCell<Vec<&'static str>>,
}

impl Fake {
    fn new(status: Status) -> Fake {
        Fake {
            status: RefCell::new(status),
            approval: false,
            refuse: None,
            calls: RefCell::default(),
        }
    }

    fn changes(&self) -> Vec<&'static str> {
        self.calls.borrow().iter().copied().filter(|c| *c != "status").collect()
    }
}

impl LoginItems for Fake {
    fn status(&self) -> Status {
        self.calls.borrow_mut().push("status");
        *self.status.borrow()
    }

    fn register(&self) -> Result<Registered, Error> {
        self.calls.borrow_mut().push("register");
        if let Some(m) = &self.refuse {
            return Err(Error::Failed(m.clone()));
        }
        if self.approval {
            *self.status.borrow_mut() = Status::NeedsApproval;
            return Ok(Registered::NeedsApproval);
        }
        *self.status.borrow_mut() = Status::On;
        Ok(Registered::Done)
    }

    fn unregister(&self) -> Result<(), Error> {
        self.calls.borrow_mut().push("unregister");
        *self.status.borrow_mut() = Status::Off;
        Ok(())
    }
}

#[test]
fn turning_it_on_registers_once_and_off_unregisters() {
    let items = Fake::new(Status::Off);
    assert_eq!(set(&items, true), Ok(Status::On));
    assert_eq!(set(&items, true), Ok(Status::On), "on again changes nothing");
    assert_eq!(set(&items, false), Ok(Status::Off));
    assert_eq!(set(&items, false), Ok(Status::Off));
    assert_eq!(items.changes(), vec!["register", "unregister"]);
}

#[test]
fn reading_the_switch_registers_nothing() {
    // What the app does at start and when the settings open: only asks.
    let items = Fake::new(Status::Off);
    assert_eq!(items.status(), Status::Off);
    assert!(items.changes().is_empty());
}

#[test]
fn a_registration_waiting_for_approval_says_so() {
    let items = Fake { approval: true, ..Fake::new(Status::Off) };
    assert_eq!(set(&items, true), Ok(Status::NeedsApproval));
    // Pressed again while macOS waits: no second registration.
    assert_eq!(set(&items, true), Ok(Status::NeedsApproval));
    // Turning it off withdraws the request.
    assert_eq!(set(&items, false), Ok(Status::Off));
    assert_eq!(items.changes(), vec!["register", "unregister"]);
}

#[test]
fn an_app_outside_a_bundle_cannot_launch_at_login() {
    let items = Fake::new(Status::Unavailable);
    assert_eq!(set(&items, true), Err(Error::Unavailable));
    assert_eq!(set(&items, false), Err(Error::Unavailable));
    assert!(items.changes().is_empty());
}

#[test]
fn a_refusal_keeps_the_switch_off() {
    let items = Fake { refuse: Some("Operation not permitted".into()), ..Fake::new(Status::Off) };
    assert_eq!(set(&items, true), Err(Error::Failed("Operation not permitted".into())));
    assert_eq!(items.status(), Status::Off);
}
