//! One window: its surface and `xdg_toplevel`, the configure sequence,
//! its scale, and presenting frames in step with the compositor's frame
//! callbacks.

use super::connection::Connection;
use super::decoration::layout::TITLE_BAR_HEIGHT;
use super::decoration::{ClientDecorations, Look};
use super::globals::Globals;
use super::icon::{IconSizes, PendingIcon};
use super::protocol::Interface;
use super::protocol::core;
use super::protocol::scale as scaling;
use super::protocol::shell::{
    self, CAPABILITY_MAXIMIZE, MODE_CLIENT_SIDE, MODE_SERVER_SIDE, STATE_ACTIVATED,
    STATE_FULLSCREEN, STATE_MAXIMIZED, STATES_TILED,
};
use super::scale::{Scale, ScaleSources};
use super::shm::{PixelFormat, Pixels, Swapchain, union};
use crate::decoration::DecorationPainter;
use crate::error::Error;
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize, Size};
use crate::window::{CursorIcon, WindowOptions};

/// The longest side of a window's content, in buffer pixels (an 8K
/// screen is 7680 wide): larger sizes are cut down, so that no
/// configure makes the backend allocate gigabytes.
const MAX_SIDE: u32 = 8192;

/// The window states the backend follows (`xdg_toplevel.state`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the independent xdg_toplevel states"
)]
pub(crate) struct States {
    /// Maximized.
    pub(crate) maximized: bool,
    /// Full screen.
    pub(crate) fullscreen: bool,
    /// Focused.
    pub(crate) activated: bool,
    /// Tiled against an edge.
    pub(crate) tiled: bool,
}

impl States {
    fn from_wire(states: &[u32]) -> Self {
        Self {
            maximized: states.contains(&STATE_MAXIMIZED),
            fullscreen: states.contains(&STATE_FULLSCREEN),
            activated: states.contains(&STATE_ACTIVATED),
            tiled: states.iter().any(|state| STATES_TILED.contains(state)),
        }
    }

    /// Whether the compositor decides the size.
    pub(crate) fn fixed_size(self) -> bool {
        self.maximized || self.fullscreen || self.tiled
    }
}

/// What a configure sequence changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Configured {
    /// Whether it was the first, which shows the window.
    pub(crate) first: bool,
    /// Whether the content's buffer size changed.
    pub(crate) resized: bool,
}

/// One window.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent flags of the configure and commit sequences"
)]
pub(crate) struct Window {
    /// The window's surface, whose id is the [`WindowId`](crate::WindowId).
    pub(crate) surface: u32,
    pub(crate) xdg_surface: u32,
    pub(crate) toplevel: u32,
    viewport: Option<u32>,
    pub(crate) fractional_scale: Option<u32>,
    pub(crate) server_decoration: Option<u32>,
    /// The client-side decorations, while the compositor draws none.
    pub(crate) decorations: Option<ClientDecorations>,
    /// The content size in surface coordinates.
    pub(crate) logical: (u32, u32),
    /// The content size while neither maximized, full screen nor tiled,
    /// restored when the compositor leaves the size to the client.
    floating: (u32, u32),
    pub(crate) scale: Scale,
    pub(crate) scale_sources: ScaleSources,
    pub(crate) states: States,
    pending_size: Option<(i32, i32)>,
    pending_states: States,
    /// The decoration mode last configured.
    pending_mode: Option<u32>,
    pub(crate) configured: bool,
    /// Whether the compositor offers maximizing.
    pub(crate) can_maximize: bool,
    min_size: Option<Size>,
    pub(crate) title: String,
    pub(crate) cursor: CursorIcon,
    pixels: Pixels,
    buffers: Swapchain,
    /// What changed since the last commit.
    damage: PhysicalRect,
    /// Whether the application presented a frame yet (the window maps
    /// with its first frame, not black).
    presented: bool,
    /// The pending frame callback, if any.
    frame_callback: Option<u32>,
    /// Whether a commit waits for the frame callback or a free buffer.
    commit_wanted: bool,
    /// Whether the size changed since the last commit: the new size is
    /// committed without waiting for the frame callback, as the
    /// compositor waits for it to finish a configure.
    resized: bool,
}

impl Window {
    /// Creates a window for `options` (its icon drawn at `icon_sizes`)
    /// and commits its first, empty state; the compositor answers with a
    /// configure.
    pub(crate) fn create(
        conn: &mut Connection,
        (globals, icon_sizes): (&Globals, &IconSizes),
        options: &WindowOptions,
        scale: Scale,
    ) -> Result<Self, Error> {
        let logical = match options.inner_size {
            Size::Logical(size) => (
                (size.width.round() as u32).max(1),
                (size.height.round() as u32).max(1),
            ),
            Size::Physical(size) => (
                scale.to_logical_len(size.width),
                scale.to_logical_len(size.height),
            ),
        };
        options.physical_inner_size(scale.factor())?;
        let compositor = globals.require(Interface::Compositor)?;
        let wm_base = globals.require(Interface::WmBase)?;
        let surface = conn.new_id(Interface::Surface)?;
        conn.send(core::create_surface(compositor, surface))?;
        let (viewport, fractional_scale) = match (
            globals.get(Interface::Viewporter),
            globals.get(Interface::FractionalScaleManager),
        ) {
            (Some(viewporter), Some(manager)) => {
                let viewport = conn.new_id(Interface::Viewport)?;
                conn.send(scaling::get_viewport(viewporter, viewport, surface))?;
                let fractional = conn.new_id(Interface::FractionalScale)?;
                conn.send(scaling::get_fractional_scale(manager, fractional, surface))?;
                (Some(viewport), Some(fractional))
            }
            _ => (None, None),
        };
        let xdg_surface = conn.new_id(Interface::XdgSurface)?;
        conn.send(shell::get_xdg_surface(wm_base, xdg_surface, surface))?;
        let toplevel = conn.new_id(Interface::Toplevel)?;
        conn.send(shell::get_toplevel(xdg_surface, toplevel))?;
        conn.send(shell::set_title(toplevel, &options.title))?;
        conn.send(shell::set_app_id(toplevel, &options.app_id))?;
        let server_decoration = match globals.get(Interface::DecorationManager) {
            Some(manager) => {
                let decoration = conn.new_id(Interface::Decoration)?;
                conn.send(shell::get_toplevel_decoration(
                    manager, decoration, toplevel,
                ))?;
                conn.send(shell::set_decoration_mode(decoration, MODE_SERVER_SIDE))?;
                Some(decoration)
            }
            None => None,
        };
        let icon = match (&options.icon, globals.get(Interface::ToplevelIconManager)) {
            (Some(icon), Some(manager)) => PendingIcon::set(
                conn,
                (manager, globals.require(Interface::Shm)?),
                toplevel,
                (icon, icon_sizes),
                scale.factor().ceil() as u32,
            )?,
            _ => None,
        };
        let mut window = Self {
            surface,
            xdg_surface,
            toplevel,
            viewport,
            fractional_scale,
            server_decoration,
            decorations: None,
            logical,
            floating: logical,
            scale,
            scale_sources: ScaleSources::default(),
            states: States::default(),
            pending_size: None,
            pending_states: States::default(),
            pending_mode: None,
            configured: false,
            can_maximize: true,
            min_size: options.min_inner_size,
            title: options.title.clone(),
            cursor: CursorIcon::Default,
            pixels: Pixels::new(scale.to_physical(logical), PixelFormat::Opaque)?,
            buffers: Swapchain::new(PixelFormat::Opaque),
            damage: PhysicalRect::new(0, 0, 0, 0),
            presented: false,
            frame_callback: None,
            commit_wanted: false,
            resized: false,
        };
        if window.server_decoration.is_none() {
            window.decorate(conn, globals)?;
        }
        window.send_min_size(conn)?;
        conn.send(core::commit(surface))?;
        if let Some(icon) = icon {
            icon.destroy(conn)?;
        }
        Ok(window)
    }

    /// The content size in buffer pixels.
    pub(crate) fn size(&self) -> PhysicalSize {
        self.scale.to_physical(self.logical)
    }

    /// The height the client-side title bar adds to the window geometry.
    fn title_bar_height(&self) -> u32 {
        if self.decorations.is_some() && !self.states.fullscreen {
            TITLE_BAR_HEIGHT
        } else {
            0
        }
    }

    /// Starts drawing client-side decorations.
    fn decorate(&mut self, conn: &mut Connection, globals: &Globals) -> Result<(), Error> {
        if self.decorations.is_none() {
            let compositor = globals.require(Interface::Compositor)?;
            let subcompositor = globals
                .get(Interface::Subcompositor)
                .ok_or(Error::Unsupported(
                    "the compositor offers no wl_subcompositor",
                ))?;
            let viewporter = self.viewport.and(globals.get(Interface::Viewporter));
            self.decorations = Some(ClientDecorations::new(
                conn,
                (compositor, subcompositor),
                self.surface,
                viewporter,
            )?);
        }
        Ok(())
    }

    fn send_min_size(&self, conn: &mut Connection) -> Result<(), Error> {
        let Some(min) = self.min_size else {
            return Ok(());
        };
        let min = min.to_physical(self.scale.factor());
        let width = self.scale.to_logical_len(min.width);
        let height = self.scale.to_logical_len(min.height) + self.title_bar_height();
        conn.send(shell::set_min_size(
            self.toplevel,
            width as i32,
            height as i32,
        ))
    }

    /// Records `xdg_toplevel.configure`, applied by the next
    /// `xdg_surface.configure`.
    pub(crate) fn toplevel_configure(&mut self, width: i32, height: i32, states: &[u32]) {
        self.pending_size = (width > 0 && height > 0).then_some((width, height));
        self.pending_states = States::from_wire(states);
    }

    /// Records the decoration mode the compositor chose.
    pub(crate) fn decoration_mode(&mut self, mode: u32) {
        self.pending_mode = Some(mode);
    }

    /// Records the compositor's capabilities.
    pub(crate) fn wm_capabilities(&mut self, capabilities: &[u32]) {
        self.can_maximize = capabilities.contains(&CAPABILITY_MAXIMIZE);
    }

    /// Applies the configuration and acknowledges `serial`.
    pub(crate) fn configure(
        &mut self,
        conn: &mut Connection,
        globals: &Globals,
        serial: u32,
    ) -> Result<Configured, Error> {
        let old_size = self.size();
        match self.pending_mode.take() {
            Some(MODE_CLIENT_SIDE) => self.decorate(conn, globals)?,
            Some(_) => {
                if let Some(mut decorations) = self.decorations.take() {
                    decorations.destroy(conn)?;
                }
            }
            None => {}
        }
        self.states = self.pending_states;
        match self.pending_size.take() {
            Some((width, height)) => {
                let height = (height as u32)
                    .saturating_sub(self.title_bar_height())
                    .max(1);
                self.logical = (width as u32, height);
            }
            None => self.logical = self.floating,
        }
        if !self.states.fixed_size() {
            self.floating = self.logical;
        }
        conn.send(shell::ack_configure(self.xdg_surface, serial))?;
        self.apply_geometry(conn)?;
        let first = !self.configured;
        self.configured = true;
        let resized = self.size() != old_size;
        Ok(Configured { first, resized })
    }

    /// Sends the window geometry and resizes the content to the current
    /// logical size and scale; the next commit shows it.
    fn apply_geometry(&mut self, conn: &mut Connection) -> Result<(), Error> {
        let longest = self.scale.to_logical_len(MAX_SIDE);
        self.logical = (self.logical.0.min(longest), self.logical.1.min(longest));
        let (width, height) = self.logical;
        let title = self.title_bar_height();
        conn.send(shell::set_window_geometry(
            self.xdg_surface,
            0,
            -(title as i32),
            width as i32,
            (height + title) as i32,
        ))?;
        self.send_min_size(conn)?;
        let size = self.size();
        if self.pixels.size() != size {
            self.pixels.resize(size)?;
            self.damage = PhysicalRect::from_size(size);
            self.commit_wanted = true;
            self.resized = true;
        }
        Ok(())
    }

    /// Resizes the window as the application asks, unless the compositor
    /// decides the size; returns whether the size changed.
    pub(crate) fn request_size(
        &mut self,
        conn: &mut Connection,
        size: Size,
    ) -> Result<bool, Error> {
        if self.states.fixed_size() {
            return Ok(false);
        }
        let size = size.to_physical(self.scale.factor());
        let logical = (
            self.scale.to_logical_len(size.width),
            self.scale.to_logical_len(size.height),
        );
        let old = self.size();
        self.logical = logical;
        self.floating = logical;
        self.apply_geometry(conn)?;
        Ok(self.size() != old)
    }

    /// Changes the scale; returns whether it changed.
    pub(crate) fn set_scale(&mut self, conn: &mut Connection, scale: Scale) -> Result<bool, Error> {
        if scale == self.scale {
            return Ok(false);
        }
        self.scale = scale;
        self.apply_geometry(conn)?;
        Ok(true)
    }

    /// Copies `region` of `frame` into the window's image; the next
    /// commit shows it.
    pub(crate) fn present(&mut self, frame: &Frame<'_>, region: PhysicalRect) {
        let region = region.clip_to(self.pixels.size());
        self.pixels.blit(frame, region);
        self.damage = union(self.damage, region);
        self.presented = true;
        self.commit_wanted = true;
    }

    /// The window's current image.
    pub(crate) fn read_pixels(&self) -> Result<OwnedFrame, Error> {
        self.pixels.to_frame()
    }

    /// Asks for a commit, e.g. to show decorations that changed.
    pub(crate) fn request_commit(&mut self) {
        self.commit_wanted = true;
    }

    /// Has the decorations drawn again (a new painter).
    pub(crate) fn invalidate_decorations(&mut self) {
        if let Some(decorations) = &mut self.decorations {
            decorations.invalidate();
            self.commit_wanted = true;
        }
    }

    /// Commits what changed: the decorations, the content, a frame
    /// callback. Waits while the previous frame is not yet shown (the
    /// frame callback is pending), unless `now` or the size changed;
    /// without a free buffer, tries again when one is released.
    pub(crate) fn commit(
        &mut self,
        conn: &mut Connection,
        shm: u32,
        painter: &dyn DecorationPainter,
        now: bool,
    ) -> Result<(), Error> {
        if !self.commit_wanted || !self.configured || !self.presented {
            return Ok(());
        }
        if self.frame_callback.is_some() && !now && !self.resized {
            return Ok(());
        }
        let look = Look {
            title: &self.title,
            content: self.logical,
            scale: self.scale,
            states: self.states,
        };
        if let Some(decorations) = &mut self.decorations
            && !decorations.update(conn, shm, &look, painter)?
        {
            return Ok(());
        }
        if !self.damage.is_empty() {
            if !self
                .buffers
                .attach(conn, shm, self.surface, &self.pixels, self.damage)?
            {
                return Ok(());
            }
            self.damage = PhysicalRect::new(0, 0, 0, 0);
            self.send_scale(conn)?;
        }
        let callback = conn.new_id(Interface::Callback)?;
        conn.send(core::frame(self.surface, callback))?;
        conn.send(core::commit(self.surface))?;
        self.frame_callback = Some(callback);
        self.commit_wanted = false;
        self.resized = false;
        Ok(())
    }

    /// Tells the compositor how the buffer maps to the surface.
    fn send_scale(&self, conn: &mut Connection) -> Result<(), Error> {
        match (self.scale, self.viewport) {
            (Scale::Fractional(_), Some(viewport)) => {
                conn.send(core::set_buffer_scale(self.surface, 1))?;
                let (width, height) = self.logical;
                conn.send(scaling::set_destination(
                    viewport,
                    width as i32,
                    height as i32,
                ))
            }
            // Integer scales stay far below `i32::MAX` (see `ScaleSources`).
            (scale, _) => conn.send(core::set_buffer_scale(self.surface, scale.factor() as i32)),
        }
    }

    /// Handles a frame callback being done; returns whether it was this
    /// window's.
    pub(crate) fn frame_done(&mut self, callback: u32) -> bool {
        if self.frame_callback == Some(callback) {
            self.frame_callback = None;
            true
        } else {
            false
        }
    }

    /// The pending frame callback.
    #[cfg(test)]
    pub(crate) fn frame_pending(&self) -> Option<u32> {
        self.frame_callback
    }

    /// How many buffers the content has.
    #[cfg(test)]
    pub(crate) fn buffer_count(&self) -> usize {
        self.buffers.len()
    }

    /// Notes that the compositor released buffer `id`; returns whether it
    /// is one of this window's.
    pub(crate) fn release(&mut self, id: u32) -> bool {
        self.buffers.release(id)
            || self
                .decorations
                .as_mut()
                .is_some_and(|decorations| decorations.release(id))
    }

    /// Destroys the window's objects.
    pub(crate) fn destroy(mut self, conn: &mut Connection) -> Result<(), Error> {
        if let Some(mut decorations) = self.decorations.take() {
            decorations.destroy(conn)?;
        }
        if let Some(decoration) = self.server_decoration {
            conn.send(shell::destroy_decoration(decoration))?;
        }
        conn.send(shell::destroy_toplevel(self.toplevel))?;
        conn.send(shell::destroy_xdg_surface(self.xdg_surface))?;
        if let Some(fractional) = self.fractional_scale {
            conn.send(scaling::destroy_fractional_scale(fractional))?;
        }
        if let Some(viewport) = self.viewport {
            conn.send(scaling::destroy_viewport(viewport))?;
        }
        self.buffers.destroy(conn)?;
        conn.send(core::destroy_surface(self.surface))
    }
}
