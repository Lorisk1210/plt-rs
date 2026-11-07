//! The Windows backend: Win32 windows, painted with GDI.
//!
//! * [`backend`]: the [`Backend`](crate::backend::Backend) implementation.
//!   Windows are per-monitor DPI aware (version 2): sizes are physical
//!   pixels and the scale factor is the monitor's DPI over 96. Frames are
//!   kept in a [`Surface`](crate::surface::Surface) of the window's size,
//!   in the BGRA order of device-independent bitmaps, and drawn with
//!   `StretchDIBits` on `WM_PAINT` only: presenting a frame has its
//!   region painted at once (`RedrawWindow`). The loop waits
//!   with `MsgWaitForMultipleObjectsEx`, until the next timer of the
//!   generic loop at the latest; proxies wake it by posting a message to
//!   a message-only window.
//! * [`procedure`]: what window messages do, on the per-thread
//!   [`Registry`](crate::registry::Registry) of windows; [`icon`]: each
//!   window's icon, drawn at the sizes of its DPI.
//! * [`message`], [`keys`] and [`convert`]: decoding messages, keyboard
//!   input (virtual keys, left and right modifiers, `AltGr`) and units.
//!   They hold no Win32 calls, so they are tested, and fuzzed
//!   (`fuzz_tests`), on every system.
//! * [`sys`]: the Win32 calls, the backend's only unsafe code, and the
//!   registry reader the crate also offers (`registry_string_values`).

#[cfg(windows)]
mod backend;
mod convert;
#[cfg(test)]
mod fuzz_tests;
#[cfg(windows)]
mod icon;
mod keys;
mod message;
#[cfg(windows)]
mod procedure;
#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "the Win32 FFI (user32, gdi32, kernel32, advapi32), behind safe wrappers"
)]
mod sys;

#[cfg(windows)]
pub(crate) use backend::WindowsBackend;
#[cfg(windows)]
pub use sys::string_values as registry_string_values;
