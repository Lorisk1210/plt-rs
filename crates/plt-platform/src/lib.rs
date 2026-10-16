//! Native windowing and input abstraction (X11 first, more platforms pluggable).
//!
//! The crate opens windows, shows pixel buffers in them and turns user input
//! into platform-neutral [`Event`]s, without linking to any C library: the
//! X11 backend speaks the X protocol directly over a socket.
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
//! [`BackendKind`]. The available backends are X11 (also under `XWayland`)
//! and headless. Adding macOS or Windows means implementing that trait,
//! without changing this API.
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

#![forbid(unsafe_code)]

mod backend;
mod click;
mod error;
mod event;
mod event_loop;
mod frame;
mod geometry;
mod headless;
mod keyboard;
mod window;
#[cfg(unix)]
mod x11;

pub use backend::BackendKind;
pub use error::{Error, X11Error};
pub use event::{ButtonState, Event, KeyEvent, MouseButton, ScrollDelta, WindowEvent};
pub use event_loop::{Context, EventLoop, EventLoopProxy, PumpStatus, TimerId};
pub use frame::{Frame, OwnedFrame};
pub use geometry::{LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Size};
pub use headless::{HeadlessHandle, PresentedFrame, WindowSnapshot};
pub use keyboard::{Key, Modifiers};
pub use window::{CursorIcon, Icon, WindowId, WindowOptions};
