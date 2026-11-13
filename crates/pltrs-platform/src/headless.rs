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
use crate::blocking::Blocking;
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::icon::Icon;
use crate::surface::{ChannelOrder, Surface};
use crate::window::{CursorIcon, WindowId, WindowOptions};

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
    /// Whether the window was made full screen.
    pub fullscreen: bool,
}

#[derive(Debug)]
struct HeadlessWindow {
    snapshot: WindowSnapshot,
    /// The window's contents: opaque black until the first present.
    surface: Surface,
}

impl HeadlessWindow {
    fn resize(&mut self, size: PhysicalSize) -> Result<(), Error> {
        self.surface.resize(size)?;
        self.snapshot.size = size;
        Ok(())
    }
}

#[derive(Debug)]
struct State {
    next_id: u64,
    scale_factor: f64,
    windows: BTreeMap<WindowId, HeadlessWindow>,
    /// The open windows from bottom to top.
    stacking: Vec<WindowId>,
    queue: VecDeque<Event>,
    woken: bool,
    loop_alive: bool,
    frames: Vec<PresentedFrame>,
    /// What was last put on the clipboard.
    clipboard: Option<Clipboard>,
}

/// What the clipboard holds.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Clipboard {
    /// A PNG image.
    Image(Vec<u8>),
    /// Text.
    Text(String),
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

    /// The PNG image on the clipboard, if it holds one.
    pub fn clipboard_image(&self) -> Option<Vec<u8>> {
        match &self.shared.lock().clipboard {
            Some(Clipboard::Image(png)) => Some(png.clone()),
            _ => None,
        }
    }

    /// The text on the clipboard, if it holds text.
    pub fn clipboard_text(&self) -> Option<String> {
        match &self.shared.lock().clipboard {
            Some(Clipboard::Text(text)) => Some(text.clone()),
            _ => None,
        }
    }

    /// Puts `text` on the clipboard, as another application copying it
    /// would.
    pub fn set_clipboard_text(&self, text: impl Into<String>) {
        self.shared.lock().clipboard = Some(Clipboard::Text(text.into()));
    }

    /// Ids of the open windows from bottom to top: new and raised windows
    /// go on top.
    pub fn stacking_order(&self) -> Vec<WindowId> {
        self.shared.lock().stacking.clone()
    }

    /// The state of an open window.
    pub fn window(&self, window: WindowId) -> Option<WindowSnapshot> {
        let state = self.shared.lock();
        state.windows.get(&window).map(|w| w.snapshot.clone())
    }

    /// The current contents of an open window.
    pub fn surface(&self, window: WindowId) -> Option<OwnedFrame> {
        let state = self.shared.lock();
        state.windows.get(&window).map(|w| w.surface.to_rgba())
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
                stacking: Vec::new(),
                queue: VecDeque::new(),
                woken: false,
                loop_alive: true,
                frames: Vec::new(),
                clipboard: None,
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

    /// Makes `window` the owner of the clipboard, holding `content`.
    fn set_clipboard(&mut self, window: WindowId, content: Clipboard) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if !state.windows.contains_key(&window) {
            return Err(Error::UnknownWindow(window));
        }
        state.clipboard = Some(content);
        Ok(())
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
            fullscreen: false,
        };
        let surface = Surface::new(size, ChannelOrder::Rgba)?;
        state
            .windows
            .insert(window, HeadlessWindow { snapshot, surface });
        state.stacking.push(window);
        let event = WindowEvent::RedrawRequested;
        state.queue.push_back(Event::Window { window, event });
        Ok(window)
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error> {
        let mut state = self.shared.lock();
        state.stacking.retain(|&id| id != window);
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
        self.with_window(window, |w| Ok(w.surface.size()))
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

    fn set_fullscreen(&mut self, window: WindowId, fullscreen: bool) -> Result<(), Error> {
        self.with_window(window, |w| {
            w.snapshot.fullscreen = fullscreen;
            Ok(())
        })
    }

    fn set_clipboard_image(&mut self, window: WindowId, png: Vec<u8>) -> Result<(), Error> {
        self.set_clipboard(window, Clipboard::Image(png))
    }

    fn set_clipboard_text(&mut self, window: WindowId, text: String) -> Result<(), Error> {
        self.set_clipboard(window, Clipboard::Text(text))
    }

    fn request_clipboard_text(&mut self, window: WindowId) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if !state.windows.contains_key(&window) {
            return Err(Error::UnknownWindow(window));
        }
        let text = match &state.clipboard {
            Some(Clipboard::Text(text)) => Some(text.clone()),
            _ => None,
        };
        let event = WindowEvent::ClipboardText(text);
        state.queue.push_back(Event::Window { window, event });
        Ok(())
    }

    fn raise_window(&mut self, window: WindowId) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if !state.windows.contains_key(&window) {
            return Err(Error::UnknownWindow(window));
        }
        state.stacking.retain(|&id| id != window);
        state.stacking.push(window);
        Ok(())
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
        let region = target.surface.blit(frame, region);
        let image = target.surface.to_rgba();
        state.frames.push(PresentedFrame {
            window,
            region,
            image,
        });
        Ok(())
    }

    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        self.with_window(window, |w| Ok(w.surface.to_rgba()))
    }

    fn poll(
        &mut self,
        deadline: Option<Instant>,
        events: &mut Vec<Event>,
        blocking: Blocking<'_>,
    ) -> Result<(), Error> {
        let shared = &self.shared;
        let mut state = shared.lock();
        while state.queue.is_empty() && !state.woken {
            let left = match deadline {
                None => None,
                Some(deadline) => match deadline.checked_duration_since(Instant::now()) {
                    Some(left) if !left.is_zero() => Some(left),
                    _ => break,
                },
            };
            state = blocking.run(|| match left {
                None => shared
                    .input
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner),
                Some(left) => {
                    let waited = shared.input.wait_timeout(state, left);
                    waited.unwrap_or_else(PoisonError::into_inner).0
                }
            });
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(surface.pixel(2, 0), Some([0, 0, 0, 255]));
    }

    #[test]
    fn raised_windows_go_on_top() {
        let (mut backend, handle) = HeadlessBackend::new();
        let options = WindowOptions::new("t");
        let first = backend.create_window(&options).unwrap();
        let second = backend.create_window(&options).unwrap();
        assert_eq!(handle.stacking_order(), [first, second]);
        backend.raise_window(first).unwrap();
        assert_eq!(handle.stacking_order(), [second, first]);
        backend.destroy_window(first).unwrap();
        assert_eq!(handle.stacking_order(), [second]);
        assert_eq!(
            backend.raise_window(first),
            Err(Error::UnknownWindow(first))
        );
    }

    #[test]
    fn the_clipboard_holds_an_image_or_text() {
        let (mut backend, handle) = HeadlessBackend::new();
        let window = backend.create_window(&WindowOptions::new("t")).unwrap();
        backend.set_clipboard_image(window, vec![1, 2]).unwrap();
        assert_eq!(handle.clipboard_image(), Some(vec![1, 2]));
        assert_eq!(handle.clipboard_text(), None);
        let mut events = Vec::new();
        backend.poll(None, &mut events, Blocking::DIRECT).unwrap();
        events.clear();
        backend.request_clipboard_text(window).unwrap();
        backend.set_clipboard_text(window, "0.25".into()).unwrap();
        backend.request_clipboard_text(window).unwrap();
        handle.set_clipboard_text("from elsewhere");
        backend.request_clipboard_text(window).unwrap();
        backend.poll(None, &mut events, Blocking::DIRECT).unwrap();
        let texts: Vec<WindowEvent> = events
            .into_iter()
            .filter_map(|event| match event {
                Event::Window { event, .. } => Some(event),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            [
                WindowEvent::ClipboardText(None),
                WindowEvent::ClipboardText(Some("0.25".into())),
                WindowEvent::ClipboardText(Some("from elsewhere".into())),
            ],
            "an image is no text"
        );
        assert_eq!(handle.clipboard_image(), None);
        let gone = WindowId::from_raw(99);
        assert_eq!(
            backend.request_clipboard_text(gone),
            Err(Error::UnknownWindow(gone))
        );
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
