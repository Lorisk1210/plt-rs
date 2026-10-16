//! An in-memory backend for tests.
//!
//! Windows are plain pixel surfaces. Presented frames are composited onto
//! them and recorded; tests drive input through a [`HeadlessHandle`], which
//! is `Send + Sync` so events can also be injected from another thread
//! while the loop blocks in [`EventLoop::run`](crate::EventLoop::run).
//!
//! The backend behaves like a cooperative window manager: a new window
//! gets a `RedrawRequested`, and
//! [`Context::set_inner_size`](crate::Context::set_inner_size) is applied
//! at once and reported as `Resized`.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use crate::backend::{Backend, BackendKind, Waker};
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::window::{CursorIcon, Icon, WindowId, WindowOptions};

/// What a new window's surface is filled with before the first present.
const INITIAL_PIXEL: [u8; 4] = [0, 0, 0, 255];

/// A frame recorded by the headless backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedFrame {
    /// The window presented to.
    pub window: WindowId,
    /// The updated region (clipped to the window).
    pub region: PhysicalRect,
    /// The window's full contents after the present (opaque RGBA8).
    pub image: OwnedFrame,
}

/// The observable state of a headless window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowSnapshot {
    /// Current title.
    pub title: String,
    /// Application id from the creation options.
    pub app_id: String,
    /// Content size in physical pixels.
    pub size: PhysicalSize,
    /// Minimum content size in physical pixels.
    pub min_size: Option<PhysicalSize>,
    /// Scale factor.
    pub scale_factor: f64,
    /// Current cursor.
    pub cursor: CursorIcon,
    /// Icon from the creation options.
    pub icon: Option<Icon>,
}

#[derive(Debug)]
struct HeadlessWindow {
    snapshot: WindowSnapshot,
    surface: OwnedFrame,
}

impl HeadlessWindow {
    fn resize(&mut self, size: PhysicalSize) -> Result<(), Error> {
        let mut surface = OwnedFrame::filled(size, INITIAL_PIXEL)?;
        let overlap = PhysicalRect::from_size(self.surface.size()).clip_to(size);
        copy_opaque(&self.surface.as_frame(), overlap, &mut surface);
        self.surface = surface;
        self.snapshot.size = size;
        Ok(())
    }
}

#[derive(Debug)]
struct State {
    next_id: u64,
    scale_factor: f64,
    windows: BTreeMap<WindowId, HeadlessWindow>,
    queue: VecDeque<Event>,
    woken: bool,
    loop_alive: bool,
    frames: Vec<PresentedFrame>,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    input: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        // The state stays consistent even if a test thread panicked while
        // holding the lock: every mutation is a single field update.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Test-side handle of a headless event loop: injects input, simulates
/// window-manager actions and inspects windows and presented frames.
#[derive(Debug, Clone)]
pub struct HeadlessHandle {
    shared: Arc<Shared>,
}

impl HeadlessHandle {
    /// Queues an event for `window`, waking the loop. `Resized` and
    /// `ScaleFactorChanged` also update the window's state, like a window
    /// manager would.
    pub fn inject(&self, window: WindowId, event: WindowEvent) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if let Some(target) = state.windows.get_mut(&window) {
            match event {
                WindowEvent::Resized(size) => target.resize(size)?,
                WindowEvent::ScaleFactorChanged(scale) => target.snapshot.scale_factor = scale,
                _ => {}
            }
        }
        state.queue.push_back(Event::Window { window, event });
        self.shared.input.notify_all();
        Ok(())
    }

    /// Changes the scale factor of every window (and of windows created
    /// later), queuing `ScaleFactorChanged` for each open window.
    pub fn set_scale_factor(&self, scale_factor: f64) -> Result<(), Error> {
        if !(scale_factor.is_finite() && scale_factor > 0.0) {
            return Err(Error::InvalidArgument("scale factor must be positive"));
        }
        let mut state = self.shared.lock();
        state.scale_factor = scale_factor;
        let ids: Vec<WindowId> = state.windows.keys().copied().collect();
        for window in ids {
            if let Some(target) = state.windows.get_mut(&window) {
                target.snapshot.scale_factor = scale_factor;
            }
            let event = WindowEvent::ScaleFactorChanged(scale_factor);
            state.queue.push_back(Event::Window { window, event });
        }
        self.shared.input.notify_all();
        Ok(())
    }

    /// Ids of the open windows, in creation order.
    pub fn windows(&self) -> Vec<WindowId> {
        self.shared.lock().windows.keys().copied().collect()
    }

    /// The state of an open window.
    pub fn window(&self, window: WindowId) -> Option<WindowSnapshot> {
        let state = self.shared.lock();
        state.windows.get(&window).map(|w| w.snapshot.clone())
    }

    /// The current contents of an open window.
    pub fn surface(&self, window: WindowId) -> Option<OwnedFrame> {
        let state = self.shared.lock();
        state.windows.get(&window).map(|w| w.surface.clone())
    }

    /// Removes and returns every frame presented so far, oldest first.
    pub fn take_frames(&self) -> Vec<PresentedFrame> {
        std::mem::take(&mut self.shared.lock().frames)
    }
}

/// The headless [`Backend`].
#[derive(Debug)]
pub(crate) struct HeadlessBackend {
    shared: Arc<Shared>,
}

impl HeadlessBackend {
    pub(crate) fn new() -> (Self, HeadlessHandle) {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                next_id: 1,
                scale_factor: 1.0,
                windows: BTreeMap::new(),
                queue: VecDeque::new(),
                woken: false,
                loop_alive: true,
                frames: Vec::new(),
            }),
            input: Condvar::new(),
        });
        let handle = HeadlessHandle {
            shared: Arc::clone(&shared),
        };
        (Self { shared }, handle)
    }

    fn with_window<T>(
        &self,
        window: WindowId,
        f: impl FnOnce(&mut HeadlessWindow) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut state = self.shared.lock();
        let target = state
            .windows
            .get_mut(&window)
            .ok_or(Error::UnknownWindow(window))?;
        f(target)
    }
}

impl Drop for HeadlessBackend {
    fn drop(&mut self) {
        self.shared.lock().loop_alive = false;
    }
}

impl Backend for HeadlessBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Headless
    }

    fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error> {
        let mut state = self.shared.lock();
        let scale_factor = state.scale_factor;
        let size = options.physical_inner_size(scale_factor)?;
        let window = WindowId::from_raw(state.next_id);
        state.next_id += 1;
        let snapshot = WindowSnapshot {
            title: options.title.clone(),
            app_id: options.app_id.clone(),
            size,
            min_size: options.min_inner_size.map(|s| s.to_physical(scale_factor)),
            scale_factor,
            cursor: CursorIcon::Default,
            icon: options.icon.clone(),
        };
        let surface = OwnedFrame::filled(size, INITIAL_PIXEL)?;
        state
            .windows
            .insert(window, HeadlessWindow { snapshot, surface });
        let event = WindowEvent::RedrawRequested;
        state.queue.push_back(Event::Window { window, event });
        Ok(window)
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error> {
        let mut state = self.shared.lock();
        state
            .windows
            .remove(&window)
            .map(drop)
            .ok_or(Error::UnknownWindow(window))
    }

    fn window_ids(&self) -> Vec<WindowId> {
        self.shared.lock().windows.keys().copied().collect()
    }

    fn has_window(&self, window: WindowId) -> bool {
        self.shared.lock().windows.contains_key(&window)
    }

    fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error> {
        self.with_window(window, |w| Ok(w.snapshot.size))
    }

    fn scale_factor(&self, window: WindowId) -> Result<f64, Error> {
        self.with_window(window, |w| Ok(w.snapshot.scale_factor))
    }

    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error> {
        self.with_window(window, |w| {
            title.clone_into(&mut w.snapshot.title);
            Ok(())
        })
    }

    fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error> {
        self.with_window(window, |w| {
            w.snapshot.cursor = cursor;
            Ok(())
        })
    }

    fn set_inner_size(&mut self, window: WindowId, size: Size) -> Result<(), Error> {
        let mut state = self.shared.lock();
        let target = state
            .windows
            .get_mut(&window)
            .ok_or(Error::UnknownWindow(window))?;
        let size = size.to_physical(target.snapshot.scale_factor);
        target.resize(size)?;
        state.queue.push_back(Event::Window {
            window,
            event: WindowEvent::Resized(size),
        });
        Ok(())
    }

    fn present(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error> {
        let mut state = self.shared.lock();
        let target = state
            .windows
            .get_mut(&window)
            .ok_or(Error::UnknownWindow(window))?;
        let region = region.clip_to(target.surface.size());
        copy_opaque(frame, region, &mut target.surface);
        let image = target.surface.clone();
        state.frames.push(PresentedFrame {
            window,
            region,
            image,
        });
        Ok(())
    }

    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        self.with_window(window, |w| Ok(w.surface.clone()))
    }

    fn poll(&mut self, deadline: Option<Instant>, events: &mut Vec<Event>) -> Result<(), Error> {
        let mut state = self.shared.lock();
        while state.queue.is_empty() && !state.woken {
            state = match deadline {
                None => self
                    .shared
                    .input
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner),
                Some(deadline) => {
                    let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                        break;
                    };
                    if left.is_zero() {
                        break;
                    }
                    let waited = self.shared.input.wait_timeout(state, left);
                    waited.unwrap_or_else(PoisonError::into_inner).0
                }
            };
        }
        events.extend(state.queue.drain(..));
        if std::mem::take(&mut state.woken) {
            events.push(Event::WakeUp);
        }
        Ok(())
    }

    fn waker(&self) -> Arc<dyn Waker> {
        Arc::new(HeadlessWaker {
            shared: Arc::clone(&self.shared),
        })
    }
}

#[derive(Debug)]
struct HeadlessWaker {
    shared: Arc<Shared>,
}

impl Waker for HeadlessWaker {
    fn wake(&self) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if !state.loop_alive {
            return Err(Error::EventLoopClosed);
        }
        state.woken = true;
        self.shared.input.notify_all();
        Ok(())
    }
}

/// Copies `region` (inside both images) of `src` into `dst` at the same
/// position, forcing alpha to 255 as an opaque window would.
fn copy_opaque(src: &Frame<'_>, region: PhysicalRect, dst: &mut OwnedFrame) {
    let region = region.clip_to(src.size()).clip_to(dst.size());
    let dst_stride = dst.size().width as usize * 4;
    let (x0, x1) = (
        region.x as usize * 4,
        (region.x + region.width) as usize * 4,
    );
    for y in region.y..region.y + region.height {
        let src_row = &src.row(y)[x0..x1];
        let start = y as usize * dst_stride;
        let dst_row = &mut dst.pixels_mut()[start + x0..start + x1];
        for (d, s) in dst_row
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(src_row.as_chunks::<4>().0)
        {
            d.copy_from_slice(&[s[0], s[1], s[2], 255]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_opaque_respects_region_and_forces_alpha() {
        let src = OwnedFrame::filled(PhysicalSize::new(4, 4), [10, 20, 30, 0]).unwrap();
        let mut dst = OwnedFrame::filled(PhysicalSize::new(3, 3), [0, 0, 0, 255]).unwrap();
        copy_opaque(&src.as_frame(), PhysicalRect::new(1, 1, 10, 10), &mut dst);
        assert_eq!(dst.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(dst.pixel(1, 0), Some([0, 0, 0, 255]));
        assert_eq!(dst.pixel(1, 1), Some([10, 20, 30, 255]));
        assert_eq!(dst.pixel(2, 2), Some([10, 20, 30, 255]));
    }

    #[test]
    fn resize_keeps_overlapping_content() {
        let (mut backend, _handle) = HeadlessBackend::new();
        let options = WindowOptions::new("t").with_inner_size(PhysicalSize::new(2, 2));
        let window = backend.create_window(&options).unwrap();
        let white = OwnedFrame::filled(PhysicalSize::new(2, 2), [255; 4]).unwrap();
        backend
            .present(window, &white.as_frame(), PhysicalRect::new(0, 0, 2, 2))
            .unwrap();
        backend
            .set_inner_size(window, PhysicalSize::new(3, 1).into())
            .unwrap();
        let surface = backend.read_pixels(window).unwrap();
        assert_eq!(surface.size(), PhysicalSize::new(3, 1));
        assert_eq!(surface.pixel(1, 0), Some([255; 4]));
        assert_eq!(surface.pixel(2, 0), Some(INITIAL_PIXEL));
    }

    #[test]
    fn waker_fails_after_backend_drop() {
        let (backend, _handle) = HeadlessBackend::new();
        let waker = backend.waker();
        assert_eq!(waker.wake(), Ok(()));
        drop(backend);
        assert_eq!(waker.wake(), Err(Error::EventLoopClosed));
    }
}
