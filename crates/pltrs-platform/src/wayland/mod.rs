//! The Wayland backend: the Wayland protocol spoken directly over the
//! compositor's socket, without libwayland or libxkbcommon.
//!
//! Layers, bottom up:
//!
//! * [`sys`]: the crate's only unsafe code, passing file descriptors over
//!   the socket and creating shared memory.
//! * [`transport`], [`wire`], [`connection`]: the socket, the message
//!   format, object ids and event parsing.
//! * [`protocol`]: the interfaces spoken, their requests and events.
//! * [`globals`], [`shm`], [`scale`], [`window`], [`decoration`],
//!   [`cursor`], [`icon`], [`xkb`]: protocol features built on the
//!   connection.
//! * [`WaylandBackend`] (with [`dispatch`], [`seat`], [`pointer`](mod@pointer),
//!   [`keyboard`], [`clipboard`] and [`waker`]): the [`Backend`]
//!   implementation.

mod clipboard;
mod connection;
mod cursor;
mod decoration;
mod dispatch;
#[cfg(test)]
mod fake;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod fuzz_tests;
mod globals;
mod icon;
mod keyboard;
#[cfg(test)]
mod live_tests;
mod pointer;
mod protocol;
mod scale;
#[cfg(test)]
mod scripted_tests;
mod seat;
mod shm;
#[allow(
    unsafe_code,
    reason = "the FFI to sendmsg, recvmsg and memfd_create, behind safe wrappers"
)]
mod sys;
mod transport;
mod waker;
mod window;
mod wire;
mod xkb;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

use crate::backend::{Backend, BackendKind, Wait, Waker};
use crate::decoration::DecorationPainter;
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::window::{CursorIcon, WindowId, WindowOptions};
use clipboard::{Clipboard, Content};
use connection::Connection;
use cursor::{Cursors, Shape};
use decoration::plain::PlainPainter;
use globals::Globals;
use icon::IconSizes;
use pointer::Target;
use protocol::{Interface, core, shell};
use scale::ScaleSources;
use seat::Seat;
use waker::WaylandWaker;
use window::Window;

/// How many round trips a new window waits for its first configure.
const CONFIGURE_ROUND_TRIPS: usize = 4;

/// An output the compositor announced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Output {
    /// The global's numeric name.
    name: u32,
    /// The version bound.
    version: u32,
    /// Its integer scale.
    scale: u32,
}

/// The Wayland [`Backend`].
#[derive(Debug)]
pub(crate) struct WaylandBackend {
    conn: Connection,
    registry: u32,
    globals: Globals,
    outputs: BTreeMap<u32, Output>,
    seat: Option<Seat>,
    /// The windows, by surface id.
    windows: BTreeMap<u32, Window>,
    cursors: Cursors,
    clipboard: Clipboard,
    /// Activation tokens asked for, with the surface each focuses.
    activations: HashMap<u32, u32>,
    painter: Box<dyn DecorationPainter>,
    /// The window icon sizes the compositor prefers.
    icon_sizes: IconSizes,
    /// Events produced outside [`Backend::poll`], delivered by the next.
    pending: Vec<Event>,
    waker: Arc<WaylandWaker>,
}

impl WaylandBackend {
    /// Connects to the compositor named by `$WAYLAND_DISPLAY`.
    pub(crate) fn connect() -> Result<Self, Error> {
        Self::with_connection(Connection::new(transport::connect()?)?)
    }

    /// Binds the globals over an open connection and learns the seat and
    /// outputs.
    fn with_connection(mut conn: Connection) -> Result<Self, Error> {
        let registry = conn.new_id(Interface::Registry)?;
        conn.send(core::get_registry(registry))?;
        let waker = Arc::new(WaylandWaker::new(conn.output()));
        let mut backend = Self {
            conn,
            registry,
            globals: Globals::default(),
            outputs: BTreeMap::new(),
            seat: None,
            windows: BTreeMap::new(),
            cursors: Cursors::default(),
            clipboard: Clipboard::default(),
            activations: HashMap::new(),
            painter: Box::new(PlainPainter),
            icon_sizes: IconSizes::default(),
            pending: Vec::new(),
            waker,
        };
        backend.round_trip()?;
        for interface in [Interface::Compositor, Interface::Shm, Interface::WmBase] {
            backend.globals.require(interface)?;
        }
        // The second round trip brings the seat's devices, the outputs'
        // scales and the keymap.
        backend.round_trip()?;
        Ok(backend)
    }

    /// Waits for the compositor to handle every request, dispatching
    /// what arrives meanwhile into the pending events.
    fn round_trip(&mut self) -> Result<(), Error> {
        let events = self.conn.round_trip()?;
        let mut out = std::mem::take(&mut self.pending);
        let result = self.dispatch(events, &mut out);
        self.pending = out;
        result
    }

    /// The surface id and state of a live window.
    fn window(&self, window: WindowId) -> Result<(u32, &Window), Error> {
        u32::try_from(window.to_raw())
            .ok()
            .and_then(|id| Some((id, self.windows.get(&id)?)))
            .ok_or(Error::UnknownWindow(window))
    }

    /// The surface id and state of a live window, mutably.
    fn window_mut(
        windows: &mut BTreeMap<u32, Window>,
        window: WindowId,
    ) -> Result<(u32, &mut Window), Error> {
        u32::try_from(window.to_raw())
            .ok()
            .and_then(|id| Some((id, windows.get_mut(&id)?)))
            .ok_or(Error::UnknownWindow(window))
    }

    /// Whether windows scale fractionally (the compositor offers both
    /// `wp_fractional_scale_v1` and `wp_viewporter`).
    fn fractional_scaling(&self) -> bool {
        self.globals
            .get(Interface::FractionalScaleManager)
            .is_some()
            && self.globals.get(Interface::Viewporter).is_some()
    }

    /// The largest output scale, the guess for windows not yet shown.
    fn largest_output_scale(&self) -> u32 {
        self.outputs
            .values()
            .map(|output| output.scale)
            .max()
            .unwrap_or(1)
    }

    /// Commits what changed in a window (see [`Window::commit`]).
    fn commit_window(&mut self, id: u32, now: bool) -> Result<(), Error> {
        let shm = self.globals.require(Interface::Shm)?;
        match self.windows.get_mut(&id) {
            Some(window) => window.commit(&mut self.conn, shm, &*self.painter, now),
            None => Ok(()),
        }
    }

    /// Forgets the pointer and keyboard focus on a closed window.
    fn unfocus(&mut self, id: u32) {
        if let Some(seat) = &mut self.seat
            && let Some(pointer) = &mut seat.pointer
            && pointer.focus.is_some_and(|(window, _)| window == id)
        {
            pointer.focus = None;
        }
    }
}

/// Appends an event for the window with surface `window`.
fn emit(out: &mut Vec<Event>, window: u32, event: WindowEvent) {
    let window = WindowId::from_raw(u64::from(window));
    out.push(Event::Window { window, event });
}

impl Backend for WaylandBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Wayland
    }

    fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error> {
        let fractional = self.fractional_scaling();
        let guess =
            ScaleSources::default().scale(fractional, |_| None, self.largest_output_scale());
        let window = Window::create(
            &mut self.conn,
            (&self.globals, &self.icon_sizes),
            options,
            guess,
        )?;
        let id = window.surface;
        self.windows.insert(id, window);
        for _ in 0..CONFIGURE_ROUND_TRIPS {
            self.round_trip()?;
            if self.windows.get(&id).is_none_or(|window| window.configured) {
                break;
            }
        }
        Ok(WindowId::from_raw(u64::from(id)))
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error> {
        let (id, _) = self.window(window)?;
        self.unfocus(id);
        self.activations.retain(|_, surface| *surface != id);
        if let Some(state) = self.windows.remove(&id) {
            state.destroy(&mut self.conn)?;
        }
        self.conn.flush()
    }

    fn window_ids(&self) -> Vec<WindowId> {
        self.windows
            .keys()
            .map(|&id| WindowId::from_raw(u64::from(id)))
            .collect()
    }

    fn has_window(&self, window: WindowId) -> bool {
        self.window(window).is_ok()
    }

    fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error> {
        Ok(self.window(window)?.1.size())
    }

    fn scale_factor(&self, window: WindowId) -> Result<f64, Error> {
        Ok(self.window(window)?.1.scale.factor())
    }

    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error> {
        let (id, state) = Self::window_mut(&mut self.windows, window)?;
        title.clone_into(&mut state.title);
        state.request_commit();
        let toplevel = state.toplevel;
        self.conn.send(shell::set_title(toplevel, title))?;
        self.commit_window(id, false)?;
        self.conn.flush()
    }

    fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error> {
        let (id, state) = Self::window_mut(&mut self.windows, window)?;
        state.cursor = cursor;
        let over = self
            .seat
            .as_ref()
            .and_then(|seat| seat.pointer.as_ref())
            .and_then(|pointer| pointer.focus)
            == Some((id, Target::Content));
        if over {
            self.show_cursor(id, Shape::Icon(cursor))?;
        }
        self.conn.flush()
    }

    fn set_inner_size(&mut self, window: WindowId, size: Size) -> Result<(), Error> {
        let (id, state) = Self::window_mut(&mut self.windows, window)?;
        if state.request_size(&mut self.conn, size)? {
            let size = state.size();
            emit(&mut self.pending, id, WindowEvent::Resized(size));
        }
        self.conn.flush()
    }

    fn raise_window(&mut self, window: WindowId) -> Result<(), Error> {
        let (id, _) = self.window(window)?;
        let (Some(activation), Some(seat)) = (self.globals.get(Interface::Activation), &self.seat)
        else {
            return Ok(());
        };
        let (seat, serial) = (seat.id, seat.serial);
        let token = self.conn.new_id(Interface::ActivationToken)?;
        self.conn
            .send(shell::get_activation_token(activation, token))?;
        if serial != 0 {
            self.conn
                .send(shell::token_set_serial(token, serial, seat))?;
        }
        self.conn.send(shell::token_set_surface(token, id))?;
        self.conn.send(shell::token_commit(token))?;
        self.activations.insert(token, id);
        self.conn.flush()
    }

    fn set_clipboard_image(&mut self, window: WindowId, png: Vec<u8>) -> Result<(), Error> {
        self.offer_clipboard(window, Content::Image(Arc::from(png)))
    }

    fn set_clipboard_text(&mut self, window: WindowId, text: String) -> Result<(), Error> {
        self.offer_clipboard(window, Content::Text(Arc::from(text)))
    }

    fn request_clipboard_text(&mut self, window: WindowId) -> Result<(), Error> {
        self.request_clipboard(window)
    }

    fn set_fullscreen(&mut self, window: WindowId, fullscreen: bool) -> Result<(), Error> {
        let toplevel = self.window(window)?.1.toplevel;
        self.conn
            .send(shell::set_fullscreen(toplevel, fullscreen))?;
        self.conn.flush()
    }

    fn present(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error> {
        let (id, state) = Self::window_mut(&mut self.windows, window)?;
        state.present(frame, region);
        self.commit_window(id, false)?;
        self.conn.flush()
    }

    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        self.window(window)?.1.read_pixels()
    }

    fn poll(&mut self, deadline: Option<Instant>, events: &mut Vec<Event>) -> Result<(), Error> {
        events.append(&mut self.pending);
        let deadline = match (deadline, self.next_repeat()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let wait = if events.is_empty() {
            Wait::until(deadline)
        } else {
            Wait::Poll
        };
        let received = self.conn.read_events(wait)?;
        self.dispatch(received, events)?;
        self.fire_repeats(Instant::now(), events);
        self.finished_pastes(events);
        events.append(&mut self.pending);
        self.conn.flush()
    }

    fn waker(&self) -> Arc<dyn Waker> {
        Arc::clone(&self.waker) as Arc<dyn Waker>
    }

    fn set_decoration_painter(&mut self, painter: Box<dyn DecorationPainter>) -> Result<(), Error> {
        self.painter = painter;
        let windows: Vec<u32> = self.windows.keys().copied().collect();
        for id in windows {
            if let Some(window) = self.windows.get_mut(&id) {
                window.invalidate_decorations();
            }
            self.commit_window(id, false)?;
        }
        self.conn.flush()
    }
}
