//! The X11 backend: the X protocol spoken directly over a socket, without
//! Xlib or XCB.
//!
//! Layers, bottom up:
//!
//! * [`display`], [`auth`], [`transport`]: find the server named by
//!   `$DISPLAY`, its socket and the `MIT-MAGIC-COOKIE-1` credentials.
//! * [`wire`], [`request`], [`reply`], [`server_event`], [`setup`]: the
//!   protocol's message formats.
//! * [`output`], [`connection`]: request numbering and buffering, the
//!   handshake, and routing of replies, errors and events.
//! * [`atoms`], [`keymap`], [`keysym`], [`image`], [`cursor`],
//!   [`resources`], [`window`]: protocol features built on the connection.
//! * [`X11Backend`] (with [`dispatch`], [`input`] and [`waker`]): the
//!   [`Backend`] implementation.

mod atoms;
mod auth;
mod connection;
mod cursor;
mod dispatch;
mod display;
#[cfg(test)]
mod fixtures;
mod image;
mod input;
mod keymap;
mod keysym;
mod keysym_table;
#[cfg(test)]
mod live_tests;
mod output;
mod reply;
mod request;
mod resources;
mod server_event;
mod setup;
mod transport;
mod waker;
mod window;
mod wire;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use crate::backend::{Backend, BackendKind, Waker};
use crate::error::Error;
use crate::event::Event;
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::window::{CursorIcon, WindowId, WindowOptions};
use atoms::Atoms;
use connection::Connection;
use cursor::Cursors;
use display::DisplayName;
use image::PixelFormat;
use input::Input;
use keymap::Keymap;
use request::{ImageTarget, NewWindow, WindowAttribute, WindowClass, event_mask};
use transport::Wait;
use waker::X11Waker;
use window::WindowState;

/// The events every window selects.
const WINDOW_EVENTS: u32 = event_mask::KEY_PRESS
    | event_mask::KEY_RELEASE
    | event_mask::BUTTON_PRESS
    | event_mask::BUTTON_RELEASE
    | event_mask::ENTER_WINDOW
    | event_mask::LEAVE_WINDOW
    | event_mask::POINTER_MOTION
    | event_mask::EXPOSURE
    | event_mask::STRUCTURE_NOTIFY
    | event_mask::FOCUS_CHANGE;

/// Length of a `PutImage` header in BIG-REQUESTS form.
const PUT_IMAGE_HEADER_LEN: usize = 28;

/// Images are placed at 16-bit signed coordinates: only this much of a
/// window can be painted.
const MAX_PAINTABLE: PhysicalSize = PhysicalSize::new(i16::MAX as u32, i16::MAX as u32);

/// The visual windows are created with.
#[derive(Debug)]
struct WindowVisual {
    id: u32,
    /// A colormap for the visual if it is not the root window's.
    colormap: Option<u32>,
    format: PixelFormat,
}

/// The X11 [`Backend`].
#[derive(Debug)]
pub(crate) struct X11Backend {
    conn: Connection,
    atoms: Atoms,
    root: u32,
    visual: WindowVisual,
    windows: BTreeMap<u32, WindowState>,
    input: Input,
    cursors: Cursors,
    scale_factor: f64,
    waker: Arc<X11Waker>,
}

impl X11Backend {
    /// Connects to the display named by `$DISPLAY`.
    pub(crate) fn connect() -> Result<Self, Error> {
        let name = std::env::var("DISPLAY")
            .ok()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| Error::BackendUnavailable(String::from("$DISPLAY is not set")))?;
        Self::open(&DisplayName::parse(&name)?)
    }

    /// Connects to the display `name` and prepares everything windows need.
    fn open(name: &DisplayName) -> Result<Self, Error> {
        let mut conn = Connection::connect(name)?;
        let (root, visual) = WindowVisual::choose(&mut conn, name.screen)?;
        let atoms = Atoms::intern(&mut conn)?;
        let keymap = Keymap::query(&mut conn)?;
        let scale_factor = resources::query_scale_factor(&mut conn, root)?;
        // Report changes of the resource database, which holds `Xft.dpi`.
        let watch_resources = WindowAttribute::EventMask(event_mask::PROPERTY_CHANGE);
        conn.send(&request::change_window_attributes(root, &[watch_resources]))?;
        let wake_window = hidden_window(&mut conn, root)?;
        let waker = Arc::new(X11Waker::new(conn.output(), wake_window, atoms.wake));
        Ok(Self {
            conn,
            atoms,
            root,
            visual,
            windows: BTreeMap::new(),
            input: Input::new(keymap),
            cursors: Cursors::default(),
            scale_factor,
            waker,
        })
    }

    /// The X window id and state of a live window.
    fn window(&self, window: WindowId) -> Result<(u32, &WindowState), Error> {
        u32::try_from(window.to_raw())
            .ok()
            .and_then(|xid| Some((xid, self.windows.get(&xid)?)))
            .ok_or(Error::UnknownWindow(window))
    }
}

impl WindowVisual {
    /// Picks the visual of screen number `screen` (see
    /// [`setup::Screen::true_color_visual`]); returns it with the screen's
    /// root window.
    fn choose(conn: &mut Connection, screen: u16) -> Result<(u32, Self), Error> {
        let setup = conn.setup();
        let missing = || Error::InvalidDisplay(format!("screen {screen} does not exist"));
        let screen = setup.screens.get(usize::from(screen)).ok_or_else(missing)?;
        let (root, root_visual) = (screen.root, screen.root_visual);
        let (depth, visual) = screen
            .true_color_visual()
            .ok_or(Error::Unsupported("no TrueColor visual"))?;
        let format = setup
            .format(depth)
            .ok_or(Error::Malformed("no pixmap format for the visual's depth"))?;
        let format = PixelFormat::new(format, &visual, setup.image_byte_order)?;
        let colormap = if visual.id == root_visual {
            None
        } else {
            let colormap = conn.generate_id()?;
            conn.send(&request::create_colormap(colormap, root, visual.id))?;
            Some(colormap)
        };
        let visual = Self {
            id: visual.id,
            colormap,
            format,
        };
        Ok((root, visual))
    }
}

/// Creates an invisible window, the target of wake-up messages.
fn hidden_window(conn: &mut Connection, root: u32) -> Result<u32, Error> {
    let id = conn.generate_id()?;
    let window = NewWindow {
        id,
        parent: root,
        class: WindowClass::InputOnly,
        depth: 0,
        visual: 0,
        width: 1,
        height: 1,
    };
    conn.send_checked(&request::create_window(&window, &[]))?;
    Ok(id)
}

/// Converts a size to the protocol's 16-bit window dimensions.
fn size_u16(size: PhysicalSize) -> Result<(u16, u16), Error> {
    match (u16::try_from(size.width), u16::try_from(size.height)) {
        (Ok(width), Ok(height)) => Ok((width, height)),
        _ => Err(Error::InvalidArgument(
            "X11 windows are at most 65535 pixels wide and high",
        )),
    }
}

impl Backend for X11Backend {
    fn kind(&self) -> BackendKind {
        BackendKind::X11
    }

    fn create_window(&mut self, options: &WindowOptions) -> Result<WindowId, Error> {
        let size = options.physical_inner_size(self.scale_factor)?;
        let (width, height) = size_u16(size)?;
        let id = self.conn.generate_id()?;
        let gc = self.conn.generate_id()?;
        let mut attributes = vec![
            WindowAttribute::BackgroundNone,
            WindowAttribute::BorderPixel(0),
            WindowAttribute::BitGravityNorthWest,
            WindowAttribute::EventMask(WINDOW_EVENTS),
        ];
        attributes.extend(self.visual.colormap.map(WindowAttribute::Colormap));
        let new_window = NewWindow {
            id,
            parent: self.root,
            class: WindowClass::InputOutput,
            depth: self.visual.format.depth(),
            visual: self.visual.id,
            width,
            height,
        };
        self.conn
            .send_checked(&request::create_window(&new_window, &attributes))?;
        self.conn.send(&request::create_gc(gc, id))?;
        for property in window::initial_properties(&self.atoms, id, options, self.scale_factor) {
            self.conn.send(&property)?;
        }
        self.conn.send(&request::map_window(id))?;
        self.conn.flush()?;
        self.windows.insert(id, WindowState::new(gc, size));
        Ok(WindowId::from_raw(u64::from(id)))
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), Error> {
        let (xid, state) = self.window(window)?;
        let gc = state.gc;
        self.windows.remove(&xid);
        self.conn.send(&request::free_gc(gc))?;
        self.conn.send(&request::destroy_window(xid))?;
        self.conn.flush()
    }

    fn window_ids(&self) -> Vec<WindowId> {
        self.windows
            .keys()
            .map(|&xid| WindowId::from_raw(u64::from(xid)))
            .collect()
    }

    fn has_window(&self, window: WindowId) -> bool {
        self.window(window).is_ok()
    }

    fn inner_size(&self, window: WindowId) -> Result<PhysicalSize, Error> {
        Ok(self.window(window)?.1.size)
    }

    fn scale_factor(&self, window: WindowId) -> Result<f64, Error> {
        self.window(window)?;
        Ok(self.scale_factor)
    }

    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        for request in window::title(&self.atoms, xid, title) {
            self.conn.send(&request)?;
        }
        self.conn.flush()
    }

    fn set_cursor(&mut self, window: WindowId, cursor: CursorIcon) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        let cursor = self.cursors.get(&mut self.conn, cursor)?;
        let attribute = WindowAttribute::Cursor(cursor);
        self.conn
            .send(&request::change_window_attributes(xid, &[attribute]))?;
        self.conn.flush()
    }

    fn raise_window(&mut self, window: WindowId) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        self.conn.send(&request::raise_window(xid))?;
        // EWMH window managers only restack and focus on request.
        let activate = window::activate(&self.atoms, xid);
        self.conn.send(&request::send_event(
            self.root,
            event_mask::SUBSTRUCTURE_REDIRECT | event_mask::SUBSTRUCTURE_NOTIFY,
            &activate,
        ))?;
        self.conn.flush()
    }

    fn set_inner_size(&mut self, window: WindowId, size: Size) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        let (width, height) = size_u16(size.to_physical(self.scale_factor))?;
        self.conn
            .send(&request::resize_window(xid, width, height))?;
        self.conn.flush()
    }

    fn present(
        &mut self,
        window: WindowId,
        frame: &Frame<'_>,
        region: PhysicalRect,
    ) -> Result<(), Error> {
        let (xid, state) = self.window(window)?;
        let region = region.clip_to(state.size).clip_to(MAX_PAINTABLE);
        let target = ImageTarget {
            drawable: xid,
            gc: state.gc,
            depth: self.visual.format.depth(),
        };
        let format = &self.visual.format;
        let max_data = self
            .conn
            .max_request_bytes()
            .saturating_sub(PUT_IMAGE_HEADER_LEN);
        for tile in format.tiles(region, max_data) {
            // The tiles lie within `MAX_PAINTABLE`, so the casts are exact.
            let placement = (
                tile.x as i16,
                tile.y as i16,
                tile.width as u16,
                tile.height as u16,
            );
            let data_len = format.stride(tile.width) * tile.height as usize;
            let request = request::put_image(target, placement, data_len, |data| {
                format.encode(frame, tile, data);
            });
            self.conn.send(&request)?;
        }
        self.conn.flush()
    }

    fn read_pixels(&mut self, window: WindowId) -> Result<OwnedFrame, Error> {
        let (xid, state) = self.window(window)?;
        let size = state.size;
        let (width, height) = size_u16(size)?;
        let reply = self
            .conn
            .round_trip(&request::get_image(xid, width, height))?;
        self.visual.format.decode(reply::image_data(&reply)?, size)
    }

    fn poll(&mut self, deadline: Option<Instant>, events: &mut Vec<Event>) -> Result<(), Error> {
        let wait = match deadline {
            None => Wait::Forever,
            Some(deadline) if deadline <= Instant::now() => Wait::Poll,
            Some(deadline) => Wait::Until(deadline),
        };
        let raw = self.conn.wait_for_events(wait)?;
        self.dispatch(&raw, events)?;
        match self.conn.take_async_error() {
            Some(error) => Err(Error::X11(error)),
            None => Ok(()),
        }
    }

    fn waker(&self) -> Arc<dyn Waker> {
        Arc::clone(&self.waker) as Arc<dyn Waker>
    }
}
