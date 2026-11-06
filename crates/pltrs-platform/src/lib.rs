//! Native windowing and input abstraction (Wayland, X11, macOS and
//! Windows).
//!
//! The crate opens windows, shows pixel buffers in them and turns user input
//! into platform-neutral [`Event`]s, without linking to any windowing
//! library: the X11 backend speaks the X protocol directly over a socket,
//! the Wayland backend the Wayland protocol, keymaps included; the macOS
//! and Windows backends call the operating system (`AppKit` through the
//! Objective-C runtime, Win32) without any binding crate.
//!
//! # Overview
//!
//! * [`EventLoop`] owns the connection to the windowing system. Drive it
//!   with [`EventLoop::run`] (blocking, `plt.show()`) or
//!   [`EventLoop::pump_events`] / [`EventLoop::run_until`] (bounded,
//!   `plt.pause()`).
//! * Handlers receive a [`Context`] with every [`Event`]; use it to create
//!   and close windows ([`WindowOptions`]), present [`Frame`]s, set titles
//!   and cursors, and start timers.
//! * [`EventLoopProxy`] wakes the loop from other threads.
//! * [`EventLoop::headless`] creates an in-memory loop for tests, with a
//!   [`HeadlessHandle`] to inject input and inspect presented frames.
//!
//! # Backends
//!
//! Platform code sits behind an internal backend trait; callers only see
//! [`BackendKind`]. The available backends are Wayland (Linux), X11 (also
//! under `XWayland`, and on macOS under `XQuartz`), macOS (`AppKit`),
//! Windows (Win32) and headless. [`BackendKind::detect`] picks the native
//! one on macOS and Windows; on Linux, Wayland when `$WAYLAND_DISPLAY` is
//! set, else X11 when `$DISPLAY` is. `$PLTRS_PLATFORM` (`wayland`, `x11`,
//! `macos`, `windows`, `headless`) overrides the choice.
//!
//! The macOS and Windows backends keep what was presented in each window,
//! so that they repaint it whenever the system asks (an expose, a live
//! resize), and the system calls back into the process with input; such
//! callbacks queue events in a per-thread registry of windows, which the
//! loop takes in turn. On macOS, the loop must run on the main thread, as
//! `AppKit` requires.
//!
//! Where the desktop leaves window decorations to applications (Wayland
//! compositors without server-side decorations, such as GNOME's), the
//! platform adds a title bar and resize borders and handles them; a
//! [`DecorationPainter`] draws the title bar in the application's look.
//!
//! # Unsafe code
//!
//! Unsafe code is denied everywhere but in three private `sys` modules,
//! the FFI to the operating system, which wrap it safely, each `unsafe`
//! block justified:
//!
//! * `wayland::sys`: Wayland passes file descriptors (shared memory,
//!   keymaps, clipboard pipes) over its socket, which needs
//!   `sendmsg`/`recvmsg` with `SCM_RIGHTS`, and pixel buffers need
//!   `memfd_create`; `std` offers neither on this toolchain.
//! * `macos::sys`: the Objective-C runtime (`objc_msgSend`, class
//!   definitions), `AppKit`, Foundation and Core Graphics.
//! * `windows::sys`: Win32 (`user32`, `gdi32` and `kernel32`, linked with
//!   `raw-dylib`, so no import libraries are needed).
//!
//! ```
//! use pltrs_platform::{Event, EventLoop, Frame, Key, WindowEvent, WindowOptions};
//!
//! let (mut event_loop, _handle) = EventLoop::headless();
//! event_loop.run(|cx, event| match event {
//!     Event::Started => {
//!         cx.create_window(&WindowOptions::new("Figure 1")).unwrap();
//!     }
//!     Event::Window { window, event: WindowEvent::RedrawRequested } => {
//!         let size = cx.inner_size(window).unwrap();
//!         let pixels = vec![255; size.area() as usize * 4];
//!         cx.present(window, &Frame::new(size.width, size.height, &pixels).unwrap()).unwrap();
//!         cx.close_window(window).unwrap();
//!     }
//!     Event::Window { window, event: WindowEvent::Key(key) } if key.key == Key::Escape => {
//!         cx.close_window(window).unwrap();
//!     }
//!     _ => {}
//! })?;
//! # Ok::<(), pltrs_platform::Error>(())
//! ```

// Unsafe code is denied everywhere but in `wayland::sys`, `macos::sys` and
// `windows::sys`, the FFI to the operating system.
#![deny(unsafe_code)]

mod backend;
#[cfg(unix)]
mod canvas;
mod click;
#[cfg(unix)]
mod cursor_art;
mod decoration;
mod error;
mod event;
mod event_loop;
mod frame;
mod geometry;
mod headless;
mod icon;
mod keyboard;
#[cfg(unix)]
mod keysym;
// The native backends' pure parts (key tables, event decoding, unit
// conversions) are compiled for tests everywhere, so that they are
// tested on Linux too.
#[cfg(any(target_os = "macos", test))]
mod macos;
#[cfg(any(target_os = "macos", windows, test))]
mod registry;
mod surface;
#[cfg(any(target_os = "macos", windows, test))]
mod wake;
#[cfg(target_os = "linux")]
mod wayland;
mod window;
#[cfg(any(windows, test))]
mod windows;
#[cfg(unix)]
mod x11;

/// The harness of pltrs's fuzz tests (`crates/pltrs/tests/fuzz`), driving the
/// unit tests that fuzz what an X server sends.
#[cfg(test)]
#[allow(
    dead_code,
    reason = "pltrs's fuzz tests use every generator, these unit tests only some"
)]
#[path = "../../pltrs/tests/fuzz/harness.rs"]
mod fuzz;

pub use backend::BackendKind;
pub use decoration::{DecorationPainter, TitleBar, TitleBarButton};
pub use error::{Error, WaylandError, X11Error};
pub use event::{ButtonState, Event, KeyEvent, MouseButton, ScrollDelta, WindowEvent};
pub use event_loop::{Context, EventLoop, EventLoopProxy, PumpStatus, TimerId};
pub use frame::{Frame, OwnedFrame};
pub use geometry::{LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Size};
pub use headless::{HeadlessHandle, PresentedFrame, WindowSnapshot};
pub use icon::{Icon, IconImage, IconLayout};
pub use keyboard::{Key, Modifiers};
pub use window::{CursorIcon, WindowId, WindowOptions};
