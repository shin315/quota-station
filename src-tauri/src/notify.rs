//! macOS notification cards with buttons (F09), through the system's
//! `UNUserNotificationCenter`. Each set of buttons is a notification
//! category of its own, registered before the card is posted; a press comes
//! back through the center's delegate with the card's id and the button's.
//!
//! Only a packaged `.app` has a notification center: a bare binary, as
//! `tauri dev` runs it, posts nothing, and its alerts stay in the app.

use std::sync::{Arc, Mutex};

use quota_station_core::alerts::{Card, Notifier};
use serde::Serialize;

/// What macOS lets the app do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    /// Not asked yet, or not known yet.
    Unknown,
    Allowed,
    /// Turned off in System Settings › Notifications.
    Denied,
    /// Not running as a packaged `.app`.
    Unbundled,
}

/// A press on a card: its id, and the button's (`None` for a click on the
/// card itself).
pub type Handler = Box<dyn Fn(String, Option<String>) + Send + Sync>;

/// A card macOS refused after taking it: its id.
pub type Refused = Arc<dyn Fn(String) + Send + Sync>;

/// A category's id and its buttons, each an id and a label.
type Category = (String, Vec<(String, String)>);

pub struct Center {
    permission: Arc<Mutex<Permission>>,
    /// The button sets registered so far, by category id.
    categories: Mutex<Vec<Category>>,
    refused: Refused,
}

impl Center {
    /// Sets the delegate that takes the presses and asks macOS, once, for
    /// leave to show cards. `refused` hears of a card macOS did not post.
    pub fn start(handler: Handler, refused: Refused) -> Center {
        let center = Center {
            permission: Arc::new(Mutex::new(Permission::Unknown)),
            categories: Mutex::new(Vec::new()),
            refused,
        };
        if mac::bundled() {
            mac::start(handler, center.permission.clone());
        } else {
            *center.permission.lock().unwrap_or_else(|e| e.into_inner()) = Permission::Unbundled;
        }
        center
    }

    pub fn permission(&self) -> Permission {
        *self.permission.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Asks macOS again whether cards may show; the answer comes later.
    pub fn refresh(&self) {
        if self.permission() != Permission::Unbundled {
            mac::refresh(self.permission.clone());
        }
    }

    /// The category for this set of buttons, registered with macOS when new.
    fn category(&self, card: &Card) -> String {
        let buttons: Vec<(String, String)> =
            card.actions.iter().map(|a| (a.id.clone(), a.label.clone())).collect();
        let mut known = self.categories.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((id, _)) = known.iter().find(|(_, b)| *b == buttons) {
            return id.clone();
        }
        let id = format!("qs.{}.{}", card.kind.number(), known.len());
        known.push((id.clone(), buttons));
        // Re-registering replaces every category, so all of them go each time.
        mac::set_categories(&known);
        id
    }
}

impl Notifier for Center {
    fn show(&self, card: &Card) -> Result<(), String> {
        match self.permission() {
            Permission::Unbundled => return Err("unbundled".into()),
            Permission::Denied => return Err("denied".into()),
            _ => {}
        }
        let category = self.category(card);
        mac::post(card, &category, self.refused.clone());
        Ok(())
    }

    fn withdraw(&self, id: &str) {
        if self.permission() != Permission::Unbundled {
            mac::withdraw(id);
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::sync::{Arc, Mutex};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{Bool, NSObject, ProtocolObject};
    use objc2::{define_class, msg_send, AnyThread, DefinedClass};
    use objc2_foundation::{NSArray, NSBundle, NSError, NSObjectProtocol, NSSet, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent,
        UNNotification, UNNotificationAction, UNNotificationActionOptions, UNNotificationCategory,
        UNNotificationCategoryOptions, UNNotificationDefaultActionIdentifier,
        UNNotificationDismissActionIdentifier, UNNotificationPresentationOptions,
        UNNotificationRequest, UNNotificationResponse, UNNotificationSettings, UNNotificationSound,
        UNUserNotificationCenter, UNUserNotificationCenterDelegate,
    };
    use quota_station_core::alerts::Card;

    use super::{Handler, Permission, Refused};

    pub struct Ivars {
        handler: Handler,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and `Delegate`
        // does not implement `Drop`.
        #[unsafe(super(NSObject))]
        #[ivars = Ivars]
        pub struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl UNUserNotificationCenterDelegate for Delegate {
            // A card while the popover or the window has focus still shows.
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                done: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                done.call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Sound,));
            }

            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                done: &block2::DynBlock<dyn Fn()>,
            ) {
                let id = response.notification().request().identifier().to_string();
                let action = response.actionIdentifier();
                // SAFETY: the two constants are NSStrings UserNotifications defines.
                let (tap, dismiss) = unsafe {
                    (UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier)
                };
                if *action != *dismiss {
                    let pressed = (*action != *tap).then(|| action.to_string());
                    (self.ivars().handler)(id, pressed);
                }
                done.call(());
            }
        }
    );

    impl Delegate {
        fn new(handler: Handler) -> Retained<Self> {
            let this = Self::alloc().set_ivars(Ivars { handler });
            // SAFETY: NSObject's `init` on a freshly allocated object.
            unsafe { msg_send![super(this), init] }
        }
    }

    /// The process is a packaged `.app` with a bundle id: without one, the
    /// notification center throws.
    pub fn bundled() -> bool {
        let bundle = NSBundle::mainBundle();
        bundle.bundleIdentifier().is_some() && bundle.bundlePath().to_string().ends_with(".app")
    }

    fn center() -> Retained<UNUserNotificationCenter> {
        UNUserNotificationCenter::currentNotificationCenter()
    }

    pub fn start(handler: Handler, permission: Arc<Mutex<Permission>>) {
        let delegate = Delegate::new(handler);
        center().setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        // The center keeps the delegate weakly: it lives as long as the app.
        std::mem::forget(delegate);
        let asked = permission.clone();
        let done = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
            let mut p = asked.lock().unwrap_or_else(|e| e.into_inner());
            *p = if granted.as_bool() { Permission::Allowed } else { Permission::Denied };
        });
        center().requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &done,
        );
    }

    pub fn refresh(permission: Arc<Mutex<Permission>>) {
        let done = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
            // SAFETY: the center passes a valid settings object for the call.
            let status = unsafe { settings.as_ref() }.authorizationStatus();
            let mut p = permission.lock().unwrap_or_else(|e| e.into_inner());
            *p = if status == UNAuthorizationStatus::Denied {
                Permission::Denied
            } else if status == UNAuthorizationStatus::NotDetermined {
                Permission::Unknown
            } else {
                Permission::Allowed
            };
        });
        center().getNotificationSettingsWithCompletionHandler(&done);
    }

    pub fn set_categories(all: &[super::Category]) {
        let categories: Vec<Retained<UNNotificationCategory>> = all
            .iter()
            .map(|(id, buttons)| {
                let actions: Vec<Retained<UNNotificationAction>> = buttons
                    .iter()
                    .map(|(id, label)| {
                        UNNotificationAction::actionWithIdentifier_title_options(
                            &NSString::from_str(id),
                            &NSString::from_str(label),
                            UNNotificationActionOptions::empty(),
                        )
                    })
                    .collect();
                UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                    &NSString::from_str(id),
                    &NSArray::from_retained_slice(&actions),
                    &NSArray::from_slice(&[]),
                    UNNotificationCategoryOptions::empty(),
                )
            })
            .collect();
        center().setNotificationCategories(&NSSet::from_retained_slice(&categories));
    }

    /// Posts the card; macOS answers later, and a refusal goes to `refused`.
    pub fn post(card: &Card, category: &str, refused: Refused) {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&card.title));
        content.setBody(&NSString::from_str(&card.body));
        content.setCategoryIdentifier(&NSString::from_str(category));
        content.setThreadIdentifier(&NSString::from_str(&format!("qs.{}", card.kind.number())));
        content.setSound(Some(&UNNotificationSound::defaultSound()));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&card.id),
            &content,
            None,
        );
        let id = card.id.clone();
        let done = RcBlock::new(move |error: *mut NSError| {
            if !error.is_null() {
                refused(id.clone());
            }
        });
        center().addNotificationRequest_withCompletionHandler(&request, Some(&done));
    }

    /// Takes the card away, shown or still waiting to show.
    pub fn withdraw(id: &str) {
        let ids = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        center().removePendingNotificationRequestsWithIdentifiers(&ids);
        center().removeDeliveredNotificationsWithIdentifiers(&ids);
    }
}

/// Elsewhere there is no notification center to talk to.
#[cfg(not(target_os = "macos"))]
mod mac {
    use std::sync::{Arc, Mutex};

    use quota_station_core::alerts::Card;

    use super::{Handler, Permission, Refused};

    pub fn bundled() -> bool {
        false
    }
    pub fn start(_: Handler, _: Arc<Mutex<Permission>>) {}
    pub fn refresh(_: Arc<Mutex<Permission>>) {}
    pub fn set_categories(_: &[super::Category]) {}
    pub fn post(_: &Card, _: &str, _: Refused) {}
    pub fn withdraw(_: &str) {}
}
