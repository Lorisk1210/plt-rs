//! Native windowing and input abstraction (X11 and Wayland, more platforms
//! pluggable).
//!
//! The crate opens windows, shows pixel buffers in them and turns user input
//! into platform-neutral [`Event`]s, without linking to any windowing
//! library: the X11 backend speaks the X protocol directly over a socket,
//! the Wayland backend the Wayland protocol, keymaps included.
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
//! under `XWayland`) and headless; [`BackendKind::detect`] picks Wayland
//! when `$WAYLAND_DISPLAY` is set, else X11 when `$DISPLAY` is, and
//! `$PLT_PLATFORM` (`wayland`, `x11`, `headless`) overrides the choice.
//! Adding macOS or Windows means implementing that trait, without changing
//! this API.
//!
//! Where the desktop leaves window decorations to applications (Wayland
//! compositors without server-side decorations, such as GNOME's), the
//! platform adds a title bar and resize borders and handles them; a
//! [`DecorationPainter`] draws the title bar in the application's look.
//!
//! # Unsafe code
//!
//! Wayland passes file descriptors (shared memory, keymaps, clipboard
//! pipes) over its socket, which needs `sendmsg`/`recvmsg` with
//! `SCM_RIGHTS`, and pixel buffers need `memfd_create`; `std` offers
//! neither on this toolchain. Those three C library calls are the crate's
//! only unsafe code: one private module wraps them, each `unsafe` block
//! justified, and denies unsafe code everywhere else.
//!
//! ```
//! use plt_platform::{Event, EventLoop, Frame, Key, WindowEvent, WindowOptions};
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
//! # Ok::<(), plt_platform::Error>(())
//! ```

// Unsafe code is denied everywhere but in `wayland::sys`, the FFI that
// passes file descriptors over the compositor's socket.
#![deny(unsafe_code)]

mod backend;
mod click;
mod decoration;
mod error;
mod event;
mod event_loop;
mod frame;
mod geometry;
mod headless;
mod keyboard;
#[cfg(unix)]
mod keysym;
#[cfg(target_os = "linux")]
mod wayland;
mod window;
#[cfg(unix)]
mod x11;

/// The harness of plt's fuzz tests (`crates/plt/tests/fuzz`), driving the
/// unit tests that fuzz what an X server sends.
#[cfg(test)]
#[allow(
    dead_code,
    reason = "plt's fuzz tests use every generator, these unit tests only some"
)]
#[path = "../../plt/tests/fuzz/harness.rs"]
mod fuzz;

pub use backend::BackendKind;
pub use decoration::{DecorationPainter, TitleBar, TitleBarButton};
pub use error::{Error, WaylandError, X11Error};
pub use event::{ButtonState, Event, KeyEvent, MouseButton, ScrollDelta, WindowEvent};
pub use event_loop::{Context, EventLoop, EventLoopProxy, PumpStatus, TimerId};
pub use frame::{Frame, OwnedFrame};
pub use geometry::{LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Size};
pub use headless::{HeadlessHandle, PresentedFrame, WindowSnapshot};
pub use keyboard::{Key, Modifiers};
pub use window::{CursorIcon, Icon, WindowId, WindowOptions};
