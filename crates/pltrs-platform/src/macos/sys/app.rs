//! The application (`NSApplication`): its setup, its menu bar, its event
//! queue, and waking it from other threads.

use std::ptr::{self, NonNull};

use super::foundation::{NsPoint, date, string};
use super::graphics::Image;
use super::runtime::{Id, NO, Object, Sel, YES, autoreleasepool, class, sel, send};
use super::{failed, new_object};
use crate::error::Error;

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    /// `NSDefaultRunLoopMode`.
    static NSDefaultRunLoopMode: *mut Object;
}

/// `NSApplicationActivationPolicyRegular`: a Dock icon and a menu bar.
const ACTIVATION_POLICY_REGULAR: i64 = 0;
/// `NSEventTypeApplicationDefined`.
const APPLICATION_DEFINED: u64 = 15;
/// The subtype of the application-defined events that wake the loop.
const WAKE_SUBTYPE: i16 = 0x504C;
/// `NSEventMaskAny`.
const ANY_EVENT: u64 = u64::MAX;

/// The application (`NSApp`), which lives as long as the process.
#[derive(Debug, Clone, Copy)]
pub(crate) struct App(NonNull<Object>);

impl App {
    /// The shared application. Main thread only.
    pub(crate) fn shared() -> Result<Self, Error> {
        let class = class(c"NSApplication").ok_or_else(|| failed("finding NSApplication"))?;
        // SAFETY: a class method returning the (immortal) shared
        // application.
        let app = unsafe { send(class.as_ptr(), c"sharedApplication", ()) };
        NonNull::new(app)
            .map(Self)
            .ok_or_else(|| failed("creating the application"))
    }

    /// Makes the application a regular one, with a Dock icon and a menu
    /// bar, as a process started from a terminal is not by default.
    pub(crate) fn make_regular(self) {
        // SAFETY: an instance method taking an NSInteger policy.
        let _: i8 = unsafe {
            send(
                self.0.as_ptr(),
                c"setActivationPolicy:",
                (ACTIVATION_POLICY_REGULAR,),
            )
        };
    }

    /// Makes `delegate` the application's delegate unless it has one
    /// (when pltrs runs inside another application). The application does
    /// not retain its delegate.
    pub(crate) fn adopt_delegate(self, delegate: &Id) {
        // SAFETY: the delegate accessors; `delegate` is live.
        unsafe {
            let current: *mut Object = send(self.0.as_ptr(), c"delegate", ());
            if current.is_null() {
                send::<_, ()>(self.0.as_ptr(), c"setDelegate:", (delegate.as_ptr(),));
            }
        }
    }

    /// Gives the application a menu bar with a Quit item (⌘Q) unless it
    /// has one.
    pub(crate) fn install_menu(self, quit_title: &str) -> Result<(), Error> {
        autoreleasepool(|| {
            // SAFETY: menu accessors and constructors; every object is
            // live while used, and the menus retain their items.
            unsafe {
                let current: *mut Object = send(self.0.as_ptr(), c"mainMenu", ());
                if !current.is_null() {
                    return Ok(());
                }
                let bar = menu()?;
                let app_item = menu_item("", None, "")?;
                let app_menu = menu()?;
                let quit = menu_item(quit_title, Some(c"terminate:"), "q")?;
                send::<_, ()>(app_menu.as_ptr(), c"addItem:", (quit.as_ptr(),));
                send::<_, ()>(app_item.as_ptr(), c"setSubmenu:", (app_menu.as_ptr(),));
                send::<_, ()>(bar.as_ptr(), c"addItem:", (app_item.as_ptr(),));
                send::<_, ()>(self.0.as_ptr(), c"setMainMenu:", (bar.as_ptr(),));
                Ok(())
            }
        })
    }

    /// Finishes launching the application unless it is launched already:
    /// needed once before handling events without `-[NSApplication run]`.
    pub(crate) fn finish_launching(self) {
        autoreleasepool(|| {
            // SAFETY: class and instance methods without arguments.
            unsafe {
                let launched = class(c"NSRunningApplication").is_some_and(|class| {
                    let current: *mut Object = send(class.as_ptr(), c"currentApplication", ());
                    send::<_, i8>(current, c"isFinishedLaunching", ()) == YES
                });
                if !launched {
                    send::<_, ()>(self.0.as_ptr(), c"finishLaunching", ());
                }
            }
        });
    }

    /// Brings the application to the front, even when another one is
    /// active (as a program started from a terminal needs).
    pub(crate) fn activate(self) {
        // SAFETY: an instance method taking a BOOL.
        unsafe { send::<_, ()>(self.0.as_ptr(), c"activateIgnoringOtherApps:", (YES,)) };
    }

    /// Shows `icon` in the Dock: macOS windows have no icons of their own.
    pub(crate) fn set_icon(self, icon: &Image) -> Result<(), Error> {
        let image = icon.ns_image()?;
        // SAFETY: an instance method taking a live NSImage, which it
        // retains.
        unsafe {
            send::<_, ()>(
                self.0.as_ptr(),
                c"setApplicationIconImage:",
                (image.as_ptr(),),
            );
        };
        Ok(())
    }

    /// Waits for an event at most `seconds` (`None`: not at all,
    /// infinity: no limit), then dispatches it and every other queued
    /// event. Wake-up events only end the wait: the waker's flags say
    /// that the loop was woken.
    pub(crate) fn pump_events(self, seconds: Option<f64>) {
        let mut wait = seconds;
        // Each event is handled in a pool of its own.
        while autoreleasepool(|| self.dispatch_next(wait)) {
            wait = None;
        }
    }

    /// Dispatches the next event (but wake-ups), waiting at most `seconds`
    /// for it; returns whether there was one.
    fn dispatch_next(self, seconds: Option<f64>) -> bool {
        // SAFETY: within the caller's pool: an autoreleased date, the
        // default mode, and the autoreleased event dequeued, inspected and
        // dispatched.
        unsafe {
            let until = date(seconds);
            let event: *mut Object = send(
                self.0.as_ptr(),
                c"nextEventMatchingMask:untilDate:inMode:dequeue:",
                (ANY_EVENT, until, NSDefaultRunLoopMode, YES),
            );
            if event.is_null() {
                return false;
            }
            let kind: u64 = send(event, c"type", ());
            let wake = kind == APPLICATION_DEFINED
                && send::<_, i16>(event, c"subtype", ()) == WAKE_SUBTYPE;
            if !wake {
                send::<_, ()>(self.0.as_ptr(), c"sendEvent:", (event,));
            }
            true
        }
    }

    /// What wakes this application's loop from other threads.
    pub(crate) fn wake_target(self) -> WakeTarget {
        WakeTarget(self.0.as_ptr() as usize)
    }
}

/// Posts wake-up events to the application from any thread
/// (`-[NSApplication postEvent:atStart:]` may be called from any thread).
#[derive(Debug, Clone, Copy)]
pub(crate) struct WakeTarget(usize);

impl WakeTarget {
    /// Posts a wake-up event.
    pub(crate) fn post(self) -> Result<(), Error> {
        autoreleasepool(|| {
            let class = class(c"NSEvent").ok_or_else(|| failed("finding NSEvent"))?;
            // SAFETY: an autoreleased application-defined event, posted to
            // the immortal application this target was made from.
            unsafe {
                let event: *mut Object = send(
                    class.as_ptr(),
                    c"otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:",
                    (
                        APPLICATION_DEFINED,
                        NsPoint::default(),
                        0_u64,
                        0.0_f64,
                        0_i64,
                        ptr::null_mut::<Object>(),
                        WAKE_SUBTYPE,
                        0_i64,
                        0_i64,
                    ),
                );
                if event.is_null() {
                    return Err(failed("creating a wake-up event"));
                }
                send::<_, ()>(self.0 as *mut Object, c"postEvent:atStart:", (event, NO));
            }
            Ok(())
        })
    }
}

/// A new, empty menu.
fn menu() -> Result<Id, Error> {
    let title = string("").ok_or_else(|| failed("creating a menu"))?;
    // SAFETY: `new_object` gives an allocated NSMenu, initialized here
    // with a live title.
    unsafe {
        let menu = new_object(c"NSMenu")?;
        Id::owned(send(menu, c"initWithTitle:", (title.as_ptr(),)))
            .ok_or_else(|| failed("creating a menu"))
    }
}

/// A new menu item titled `title`, sending `action` to the responder
/// chain, with the key equivalent `key` (with ⌘).
fn menu_item(title: &str, action: Option<&std::ffi::CStr>, key: &str) -> Result<Id, Error> {
    let error = || failed("creating a menu item");
    let (title, key) = (
        string(title).ok_or_else(error)?,
        string(key).ok_or_else(error)?,
    );
    let action = action.map_or(Sel::NONE, sel);
    // SAFETY: `new_object` gives an allocated NSMenuItem, initialized here
    // with live strings and a selector (or none).
    unsafe {
        let item = new_object(c"NSMenuItem")?;
        Id::owned(send(
            item,
            c"initWithTitle:action:keyEquivalent:",
            (title.as_ptr(), action, key.as_ptr()),
        ))
        .ok_or_else(error)
    }
}
