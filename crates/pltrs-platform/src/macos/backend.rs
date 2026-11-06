//! [`MacBackend`]: the [`Backend`] on `AppKit` windows.

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use super::convert::{
    DOCK_ICON_SIDES, PointRect, icon_representations, pixel_size, points, seconds_until,
};
use super::sys::{
    self, App, Content, Id, Image, NsPoint, View, WakeTarget, Window, autoreleasepool,
};
use super::view::{ViewState, with_views};
use crate::backend::{Backend, BackendKind, Wait, Waker};
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::icon::{Icon, IconLayout};
use crate::registry::{LoopId, Registry};
use crate::surface::{ChannelOrder, Surface};
use crate::wake::WakeFlags;
use crate::window::{CursorIcon, WindowId, WindowOptions};

thread_local! {
    /// The application delegate pltrs installed, which the application
    /// does not retain: it lives as long as the (main) thread.
    static DELEGATE: OnceCell<Id> = const { OnceCell::new() };
}

/// A window and its view.
#[derive(Debug)]
struct MacWindow {
    window: Window,
    view: View,
}

/// The macOS [`Backend`].
#[derive(Debug)]
pub(crate) struct MacBackend {
    /// This loop among those of the thread.
    id: LoopId,
    app: App,
    /// The address of pltrs's view class.
    view_class: usize,
    windows: BTreeMap<WindowId, MacWindow>,
    waker: Arc<MacWaker>,
    /// Where the next window's top-left corner goes, once one was placed.
    cascade: Option<NsPoint>,
    /// The icon last put in the Dock.
    icon: Option<Icon>,
}

impl MacBackend {
    /// Sets up the application: a regular one (with a Dock icon and a
    /// menu bar whose Quit asks every window to close), launched.
    pub(crate) fn connect() -> Result<Self, Error> {
        autoreleasepool(|| {
            if !sys::is_main_thread() {
                return Err(Error::BackendUnavailable(String::from(
                    "macOS windows can only be opened on the main thread",
                )));
            }
            let app = App::shared()?;
            app.make_regular();
            DELEGATE.with(|installed| {
                if installed.get().is_none() {
                    let delegate = sys::app_delegate()?;
                    app.adopt_delegate(&delegate);
                    installed.get_or_init(|| delegate);
                }
                Ok::<_, Error>(())
            })?;
            app.install_menu(&format!("Quit {}", sys::process_name()))?;
            app.finish_launching();
            Ok(Self {
                id: with_views(Registry::new_loop),
                app,
                view_class: sys::view_class()?,
                windows: BTreeMap::new(),
                waker: Arc::new(MacWaker {
                    flags: WakeFlags::new(),
                    target: app.wake_target(),
                }),
                cascade: None,
                icon: None,
            })
        })
    }

    /// The window and view of `window`, one of this loop's windows.
    fn window(&self, window: WindowId) -> Result<&MacWindow, Error> {
        self.windows
            .get(&window)
            .ok_or(Error::UnknownWindow(window))
    }

    /// Runs `f` on the state of `window`, one of this loop's windows.
    fn with_state<T>(
        &self,
        window: WindowId,
        f: impl FnOnce(&mut ViewState) -> T,
    ) -> Result<T, Error> {
        with_views(|views| views.get_mut(self.id, window).map(f))
    }

    /// Places a new window: the first one centered, the next ones
    /// cascading from it.
    fn place(&mut self, window: &Window) {
        let from = self.cascade.unwrap_or_else(|| {
            window.center();
            NsPoint::default()
        });
        self.cascade = Some(window.cascade(from));
    }

    /// Shows `icon` in the Dock, on Apple's icon grid, drawn at each of
    /// Apple's icon sizes (unless it is there already).
    fn set_icon(&mut self, icon: &Icon) -> Result<(), Error> {
        if self.icon.as_ref() == Some(icon) {
            return Ok(());
        }
        let images = icon
            .images(&DOCK_ICON_SIDES, IconLayout::AppleGrid)
            .into_iter()
            .map(|image| {
                let size = image.size();
                Image::straight_rgba(size.width, size.height, image.rgba().to_vec())
                    .map(|cg_image| (cg_image, size))
                    .ok_or(Error::InvalidArgument("icon too large for macOS"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let representations: Vec<_> = images
            .iter()
            .flat_map(|(cg_image, size)| {
                icon_representations(*size)
                    .into_iter()
                    .map(move |points| (cg_image, points))
            })
            .collect();
        if representations.is_empty() {
            return Ok(());
        }
        self.app.set_icon(&sys::ns_image(&representations)?);
        self.icon = Some(icon.clone());
        Ok(())
    }
}

impl Drop for MacBackend {
    fn drop(&mut self) {
        for window in self.window_ids() {
            // Errors cannot be reported from `drop`; the window is no
            // longer used either way.
            let _ = self.destroy_window(window);
        }
        self.waker.flags.close();
    }
}

impl Backend for MacBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::MacOs
    }

    fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error> {
        autoreleasepool(|| {
            // The window's screen, and so its scale factor, is known once
            // it exists: sizes start from the main screen's.
            let estimate = sys::main_screen_scale_factor();
            let size = options.physical_inner_size(estimate)?.to_logical(estimate);
            let view = View::new(self.view_class, size)?;
            let window = Window::new(size, &options.title, &view)?;
            if let Some(min_size) = options.min_inner_size {
                window.set_content_min_size(points(min_size, estimate));
            }
            if let Some(icon) = &options.icon {
                self.set_icon(icon)?;
            }
            let id = view.get().id();
            let scale = view.get().scale_factor();
            let bounds = view.get().bounds();
            let surface = Surface::new(
                pixel_size(bounds.width, bounds.height, scale),
                ChannelOrder::Bgra,
            )?;
            with_views(|views| views.insert(self.id, id, ViewState::new(surface, scale)));
            self.place(&window);
            window.show();
            self.app.activate();
            with_views(|views| views.push(id, WindowEvent::RedrawRequested));
            self.windows.insert(id, MacWindow { window, view });
            Ok(id)
        })
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error> {
        let closed = self
            .windows
            .remove(&window)
            .ok_or(Error::UnknownWindow(window))?;
        with_views(|views| views.remove(window));
        // Releasing the window and its view deallocates them, which
        // autoreleases.
        autoreleasepool(|| {
            closed.window.close();
            drop(closed);
        });
        Ok(())
    }

    fn window_ids(&self) -> Vec<WindowId> {
        with_views(|views| views.windows(self.id))
    }

    fn has_window(&self, window: WindowId) -> bool {
        with_views(|views| views.get(self.id, window).is_ok())
    }

    fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error> {
        self.with_state(window, |state| state.surface.size())
    }

    fn scale_factor(&self, window: WindowId) -> Result<f64, Error> {
        self.with_state(window, |state| state.scale)
    }

    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error> {
        autoreleasepool(|| self.window(window)?.window.set_title(title))
    }

    fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error> {
        let shown = self.window(window)?;
        let inside = self.with_state(window, |state| {
            state.cursor = cursor;
            state.pointer_inside
        })?;
        autoreleasepool(|| {
            shown.window.invalidate_cursor_rects(&shown.view);
            if inside {
                sys::set_cursor(cursor);
            }
        });
        Ok(())
    }

    fn set_inner_size(&mut self, window: WindowId, size: Size) -> Result<(), Error> {
        let scale = self.with_state(window, |state| state.scale)?;
        let shown = &self.window(window)?.window;
        autoreleasepool(|| shown.set_content_size(points(size, scale)));
        Ok(())
    }

    fn raise_window(&mut self, window: WindowId) -> Result<(), Error> {
        let shown = self.window(window)?;
        autoreleasepool(|| {
            self.app.activate();
            shown.window.show();
        });
        Ok(())
    }

    fn set_clipboard_image(&mut self, window: WindowId, png: Vec<u8>) -> Result<(), Error> {
        self.window(window)?;
        sys::set_pasteboard(Content::Png(&png))
    }

    fn set_clipboard_text(&mut self, window: WindowId, text: String) -> Result<(), Error> {
        self.window(window)?;
        sys::set_pasteboard(Content::Text(&text))
    }

    fn request_clipboard_text(&mut self, window: WindowId) -> Result<(), Error> {
        self.window(window)?;
        let text = sys::pasteboard_text()?;
        with_views(|views| views.push(window, WindowEvent::ClipboardText(text)));
        Ok(())
    }

    fn set_fullscreen(&mut self, window: WindowId, fullscreen: bool) -> Result<(), Error> {
        let shown = &self.window(window)?.window;
        autoreleasepool(|| {
            if shown.is_fullscreen() != fullscreen {
                shown.toggle_fullscreen();
            }
        });
        Ok(())
    }

    fn present(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error> {
        let view = self.window(window)?.view.get();
        let (copied, scale) = self.with_state(window, |state| {
            (state.surface.blit(frame, region), state.scale)
        })?;
        if !copied.is_empty() {
            autoreleasepool(|| view.redraw(PointRect::from_pixels(copied, scale)));
        }
        Ok(())
    }

    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        self.with_state(window, |state| state.surface.to_rgba())
    }

    fn poll(&mut self, deadline: Option<Instant>, events: &mut Vec<Event>) -> Result<(), Error> {
        let wait = Wait::until(deadline);
        loop {
            let seconds = seconds_until(wait, Instant::now());
            self.app.pump_events(seconds);
            events.extend(with_views(|views| views.take_events(self.id)));
            if self.waker.flags.take() {
                events.push(Event::WakeUp);
            }
            if !events.is_empty() || seconds.is_none() {
                return Ok(());
            }
        }
    }

    fn waker(&self) -> Arc<dyn Waker> {
        Arc::clone(&self.waker) as Arc<dyn Waker>
    }
}

/// Wakes a macOS event loop by posting an application-defined event, from
/// any thread.
#[derive(Debug)]
struct MacWaker {
    flags: WakeFlags,
    target: WakeTarget,
}

impl Waker for MacWaker {
    fn wake(&self) -> Result<(), Error> {
        self.flags.wake(|| self.target.post())
    }
}
