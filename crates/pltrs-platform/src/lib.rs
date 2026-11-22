//! Native windows and input for Wayland, X11, macOS and Windows.
//!
//! The crate opens windows, shows pixel buffers in them and changes user
//! input into platform-neutral [`Event`]s. It links to no windowing
//! library. The X11 backend speaks the X protocol over a socket. The
//! Wayland backend speaks the Wayland protocol, keymaps included. The
//! macOS and Windows backends call the operating system directly (`AppKit`
//! through the Objective-C runtime, and Win32).
//!
//! # Overview
//!
//! * [`EventLoop`] owns the connection to the windowing system. Run it
//!   with [`EventLoop::run`] (blocking, `plt.show()`), or with
//!   [`EventLoop::pump_events`] or [`EventLoop::run_until`] (bounded,
//!   `plt.pause()`).
//! * Handlers get a [`Context`] with each [`Event`]. Use it to create and
//!   close windows ([`WindowOptions`]), present [`Frame`]s, set titles and
//!   cursors, and start timers.
//! * The loop blocks until input comes, a timer is due or something wakes
//!   it. An [`EventLoopProxy`] on another thread or a write to a
//!   [`WakePipe`] (Python `signal.set_wakeup_fd`) wakes it. An embedder can
//!   run the blocking waits through a [`WaitHook`]
//!   ([`EventLoop::set_wait_hook`]), so that its other threads can work.
//! * [`EventLoop::headless`] makes an in-memory loop for tests. Its
//!   [`HeadlessHandle`] injects input and reads the presented frames.
//!
//! # Backends
//!
//! The platform code is behind an internal backend trait. Callers see only
//! [`BackendKind`]. The backends are Wayland (Linux), X11 (also under
//! `XWayland`, and on macOS under `XQuartz`), macOS (`AppKit`), Windows
//! (Win32) and headless. [`BackendKind::detect`] selects the native
//! backend on macOS and Windows. On Linux, it selects Wayland if
//! `$WAYLAND_DISPLAY` is set, or else X11 if `$DISPLAY` is set.
//! `$PLTRS_PLATFORM` (`wayland`, `x11`, `macos`, `windows`, `headless`)
//! overrides the selection.
//!
//! The macOS and Windows backends keep the last frame of each window. They
//! paint it again when the system asks, for example on an expose or a live
//! resize. The system calls back into the process with input. These
//! callbacks put events in a registry of windows for each thread, and the
//! loop takes them in turn. On macOS, the loop must run on the main
//! thread, because `AppKit` requires it.
//!
//! Some desktops leave window decorations to applications, for example
//! Wayland compositors without server-side decorations, such as GNOME.
//! There, the platform adds a title bar and resize borders and handles
//! them. The buttons and click actions come from the desktop settings,
//! which the crate reads from the XDG desktop portal over D-Bus. A
//! [`DecorationPainter`] draws the title bar in the look of the
//! application.
//!
//! Keys type text through the dead keys and compose sequences of the
//! system. On X11 and Wayland, they use the Compose files of libxkbcommon
//! (´ then e types é, Compose o c types ©). On macOS, they use
//! `UCKeyTranslate` with the keyboard layout. On Windows, they use
//! `TranslateMessage`. A dead key is [`Key::Dead`], and the key that
//! completes a sequence carries its text.
//!
//! # Unsafe code
//!
//! Unsafe code is denied everywhere except in three private `sys` modules,
//! the FFI to the operating system. These modules wrap it in safe
//! functions and justify each `unsafe` block:
//!
//! * `wayland::sys`: Wayland passes file descriptors (shared memory,
//!   keymaps, clipboard pipes) over its socket. This needs
//!   `sendmsg`/`recvmsg` with `SCM_RIGHTS`, and pixel buffers need
//!   `memfd_create`. `std` has neither on this toolchain.
//! * `macos::sys`: the Objective-C runtime (`objc_msgSend`, class
//!   definitions), `AppKit`, Foundation and Core Graphics.
//! * `windows::sys`: Win32 (`user32`, `gdi32`, `kernel32`, and `advapi32`
//!   for `registry_string_values`). With MSVC, it links them with
//!   `raw-dylib` and needs no import libraries. With the GNU toolchains, it
//!   uses the MinGW-w64 import libraries.
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
mod blocking;
#[cfg(unix)]
mod canvas;
mod click;
#[cfg(unix)]
mod compose;
#[cfg(unix)]
mod cursor_art;
#[cfg(target_os = "linux")]
mod dbus;
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
#[cfg(target_os = "linux")]
mod portal;
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
mod wake_pipe;
#[cfg(target_os = "linux")]
mod wayland;
mod window;
#[cfg(any(windows, test))]
mod windows;
#[cfg(unix)]
mod x11;

/// The harness of pltrs's fuzz tests (`pltrs_core::fuzz`), driving the
/// unit tests that fuzz what an X server sends.
#[cfg(test)]
use pltrs_core::fuzz;

pub use backend::BackendKind;
pub use blocking::WaitHook;
pub use decoration::{ButtonLayout, DecorationPainter, TitleBar, TitleBarButton};
pub use error::{Error, WaylandError, X11Error};
pub use event::{ButtonState, Event, KeyEvent, MouseButton, ScrollDelta, WindowEvent};
pub use event_loop::{Context, EventLoop, EventLoopProxy, PumpStatus, TimerId};
pub use frame::{Frame, OwnedFrame};
pub use geometry::{LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Size};
pub use headless::{HeadlessHandle, PresentedFrame, WindowSnapshot};
pub use icon::{Icon, IconImage, IconLayout};
pub use keyboard::{Key, Modifiers};
pub use wake_pipe::WakePipe;
pub use window::{CursorIcon, WindowId, WindowOptions};
#[cfg(windows)]
pub use windows::registry_string_values;
