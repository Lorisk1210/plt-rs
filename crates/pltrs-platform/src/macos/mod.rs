//! The macOS backend: `AppKit` windows, through the Objective-C runtime.
//!
//! * [`backend`]: the [`Backend`](crate::backend::Backend) implementation.
//!   The application becomes a regular one (a Dock icon, a menu bar with
//!   Quit), each window shows a view of pltrs's own `NSView` subclass,
//!   which is also the window's delegate. Sizes are physical pixels: the
//!   view's points times the window's `backingScaleFactor` (2 on Retina
//!   displays). Frames are kept in a [`Surface`](crate::surface::Surface)
//!   of the view's pixel size, in Core Graphics' native BGRA order, and
//!   drawn as images in `drawRect:` for the regions presented. The loop
//!   waits in `nextEventMatchingMask:untilDate:inMode:dequeue:`, until
//!   the next timer of the generic loop at the latest; proxies wake it by
//!   posting an application-defined event. Every call into `AppKit` runs
//!   within an autorelease pool: without `-[NSApplication run]`, nothing
//!   drains the main thread's own, so what `AppKit` autoreleases outside
//!   one is never released (a window resized outside one was never
//!   deallocated, nor its view and backing store).
//! * [`view`]: what `AppKit`'s calls on the views do, on the per-thread
//!   [`Registry`](crate::registry::Registry) of windows.
//! * [`keys`], [`pointer`](mod@pointer) and [`convert`]: keyboard and
//!   pointer input, and units. They make no `AppKit` calls, so they are
//!   tested on every system.
//! * [`sys`]: the Objective-C, `AppKit` and Core Graphics calls, the
//!   backend's only unsafe code.

#[cfg(target_os = "macos")]
mod backend;
mod convert;
mod keys;
mod pointer;
#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "the FFI to the Objective-C runtime, AppKit and Core Graphics, behind safe wrappers"
)]
mod sys;
#[cfg(target_os = "macos")]
mod view;

#[cfg(target_os = "macos")]
pub(crate) use backend::MacBackend;
