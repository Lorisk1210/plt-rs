//! Client-side decorations on Wayland: a title bar and an invisible
//! resize border, as two subsurfaces of the window's surface, which
//! itself shows only the application's frames.
//!
//! The title bar sits above the content and scales like it; the border, a
//! transparent surface below the content reaching [`RESIZE_MARGIN`]
//! beyond the window on every side, only catches the pointer. The window
//! geometry covers content and title bar. Both subsurfaces are
//! synchronized: their changes show with the window's next commit.

pub(crate) mod layout;
pub(crate) mod plain;

use super::connection::Connection;
use super::protocol::Interface;
use super::protocol::core;
use super::protocol::scale as scaling;
use super::scale::Scale;
use super::shm::{PixelFormat, Pixels, Swapchain};
use super::window::States;
use crate::decoration::{DecorationPainter, TitleBar, TitleBarButton};
use crate::error::Error;
use crate::frame::Frame;
use crate::geometry::{PhysicalRect, PhysicalSize};
use layout::{Hit, RESIZE_MARGIN, TITLE_BAR_HEIGHT, border_hit, title_bar_hit};

/// Which decoration surface the pointer is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    /// The title bar.
    TitleBar,
    /// The resize border.
    Border,
}

/// What the decorations show, from the window's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Look<'a> {
    /// The window title.
    pub(crate) title: &'a str,
    /// The content size in surface coordinates.
    pub(crate) content: (u32, u32),
    /// The window's scale.
    pub(crate) scale: Scale,
    /// The window's states: no decorations full screen, no resize border
    /// maximized or tiled.
    pub(crate) states: States,
}

/// What a title bar was drawn for.
#[derive(Debug, Clone, PartialEq)]
struct Drawn {
    title: String,
    width: u32,
    scale: Scale,
    focused: bool,
    maximized: bool,
}

impl Drawn {
    fn new(look: &Look<'_>) -> Self {
        Self {
            title: look.title.to_owned(),
            width: look.content.0,
            scale: look.scale,
            focused: look.states.activated,
            maximized: look.states.maximized,
        }
    }

    /// Whether the title bar still shows `look`.
    fn shows(&self, look: &Look<'_>) -> bool {
        *self == Self::new(look)
    }
}

/// A decoration surface: a subsurface of the window's.
#[derive(Debug)]
struct DecorationSurface {
    surface: u32,
    subsurface: u32,
    buffers: Swapchain,
    /// The size last attached, if shown.
    shown: Option<PhysicalSize>,
}

impl DecorationSurface {
    fn new(
        conn: &mut Connection,
        (compositor, subcompositor): (u32, u32),
        parent: u32,
        position: (i32, i32),
    ) -> Result<Self, Error> {
        let surface = conn.new_id(Interface::Surface)?;
        conn.send(core::create_surface(compositor, surface))?;
        let subsurface = conn.new_id(Interface::Subsurface)?;
        conn.send(core::get_subsurface(
            subcompositor,
            subsurface,
            surface,
            parent,
        ))?;
        conn.send(core::set_position(subsurface, position.0, position.1))?;
        Ok(Self {
            surface,
            subsurface,
            buffers: Swapchain::new(PixelFormat::Premultiplied),
            shown: None,
        })
    }

    /// Unmaps the surface.
    fn hide(&mut self, conn: &mut Connection) -> Result<(), Error> {
        if self.shown.take().is_some() {
            conn.send(core::attach(self.surface, 0))?;
            conn.send(core::commit(self.surface))?;
        }
        Ok(())
    }

    /// Shows `pixels`; returns `false` when no buffer was free.
    fn show(&mut self, conn: &mut Connection, shm: u32, pixels: &Pixels) -> Result<bool, Error> {
        let full = PhysicalRect::from_size(pixels.size());
        if !self.buffers.attach(conn, shm, self.surface, pixels, full)? {
            return Ok(false);
        }
        self.shown = Some(pixels.size());
        Ok(true)
    }

    fn destroy(&mut self, conn: &mut Connection) -> Result<(), Error> {
        self.buffers.destroy(conn)?;
        conn.send(core::destroy_subsurface(self.subsurface))?;
        conn.send(core::destroy_surface(self.surface))
    }
}

/// The decorations of one window.
#[derive(Debug)]
pub(crate) struct ClientDecorations {
    title_bar: DecorationSurface,
    /// The title bar's viewport, under fractional scaling.
    viewport: Option<u32>,
    border: DecorationSurface,
    /// What the title bar was last drawn for.
    drawn: Option<Drawn>,
    hovered: Option<TitleBarButton>,
    pressed: Option<TitleBarButton>,
    /// Whether the title bar must be drawn again.
    stale: bool,
}

impl ClientDecorations {
    /// Creates the decoration surfaces of the window surface `parent`;
    /// `viewporter` is given under fractional scaling.
    pub(crate) fn new(
        conn: &mut Connection,
        globals: (u32, u32),
        parent: u32,
        viewporter: Option<u32>,
    ) -> Result<Self, Error> {
        let title_height = TITLE_BAR_HEIGHT as i32;
        let margin = RESIZE_MARGIN as i32;
        let border =
            DecorationSurface::new(conn, globals, parent, (-margin, -title_height - margin))?;
        conn.send(core::place_below(border.subsurface, parent))?;
        let title_bar = DecorationSurface::new(conn, globals, parent, (0, -title_height))?;
        let viewport = match viewporter {
            Some(viewporter) => {
                let viewport = conn.new_id(Interface::Viewport)?;
                conn.send(scaling::get_viewport(
                    viewporter,
                    viewport,
                    title_bar.surface,
                ))?;
                Some(viewport)
            }
            None => None,
        };
        Ok(Self {
            title_bar,
            viewport,
            border,
            drawn: None,
            hovered: None,
            pressed: None,
            stale: true,
        })
    }

    /// The title bar's and the border's surfaces.
    #[cfg(test)]
    pub(crate) fn surfaces(&self) -> (u32, u32) {
        (self.title_bar.surface, self.border.surface)
    }

    /// Which part `surface` is, if it is one of these.
    pub(crate) fn part(&self, surface: u32) -> Option<Part> {
        if surface == self.title_bar.surface {
            Some(Part::TitleBar)
        } else if surface == self.border.surface {
            Some(Part::Border)
        } else {
            None
        }
    }

    /// What is at `(x, y)` (surface coordinates) of `part`, for content
    /// of `content` size.
    pub(crate) fn hit(part: Part, (x, y): (f64, f64), (width, height): (u32, u32)) -> Option<Hit> {
        match part {
            Part::TitleBar => Some(title_bar_hit(x, y, width)),
            Part::Border => {
                let frame_width = width + 2 * RESIZE_MARGIN;
                let frame_height = height + TITLE_BAR_HEIGHT + 2 * RESIZE_MARGIN;
                border_hit(x, y, frame_width, frame_height).map(Hit::Resize)
            }
        }
    }

    /// Sets the button under the pointer and the one held down; returns
    /// whether that changed.
    pub(crate) fn set_buttons(
        &mut self,
        hovered: Option<TitleBarButton>,
        pressed: Option<TitleBarButton>,
    ) -> bool {
        let changed = (hovered, pressed) != (self.hovered, self.pressed);
        if changed {
            (self.hovered, self.pressed) = (hovered, pressed);
            self.stale = true;
        }
        changed
    }

    /// Has the title bar drawn again.
    pub(crate) fn invalidate(&mut self) {
        self.stale = true;
    }

    /// The button held down.
    pub(crate) fn pressed(&self) -> Option<TitleBarButton> {
        self.pressed
    }

    /// Whether the decorations need drawing again for `look`.
    pub(crate) fn needs_update(&self, look: &Look<'_>) -> bool {
        self.stale || self.drawn.as_ref().is_none_or(|drawn| !drawn.shows(look))
    }

    /// Brings the surfaces up to date with `look`, without committing the
    /// window. Returns `false` when a buffer was busy, so that the caller
    /// tries again later.
    pub(crate) fn update(
        &mut self,
        conn: &mut Connection,
        shm: u32,
        look: &Look<'_>,
        painter: &dyn DecorationPainter,
    ) -> Result<bool, Error> {
        if look.states.fullscreen {
            self.title_bar.hide(conn)?;
            self.border.hide(conn)?;
            self.drawn = None;
            return Ok(true);
        }
        let (width, height) = look.content;
        let mut done = true;
        if self.needs_update(look) {
            done &= self.draw_title_bar(conn, shm, look, painter)?;
        }
        if look.states.fixed_size() {
            self.border.hide(conn)?;
        } else {
            let size = PhysicalSize::new(
                width + 2 * RESIZE_MARGIN,
                height + TITLE_BAR_HEIGHT + 2 * RESIZE_MARGIN,
            );
            if self.border.shown != Some(size) {
                let blank = Pixels::new(size, PixelFormat::Premultiplied)?;
                if self.border.show(conn, shm, &blank)? {
                    conn.send(core::commit(self.border.surface))?;
                } else {
                    done = false;
                }
            }
        }
        Ok(done)
    }

    fn draw_title_bar(
        &mut self,
        conn: &mut Connection,
        shm: u32,
        look: &Look<'_>,
        painter: &dyn DecorationPainter,
    ) -> Result<bool, Error> {
        let width = look.content.0;
        let size = look.scale.to_physical((width, TITLE_BAR_HEIGHT));
        let bar = TitleBar {
            title: look.title,
            size,
            scale_factor: look.scale.factor(),
            focused: look.states.activated,
            maximized: look.states.maximized,
            hovered: self.hovered,
            pressed: self.pressed,
        };
        let mut rgba = vec![0; crate::frame::rgba_len(size)?];
        painter.paint(&bar, &mut rgba);
        let mut pixels = Pixels::new(size, PixelFormat::Premultiplied)?;
        pixels.blit(
            &Frame::new(size.width, size.height, &rgba)?,
            PhysicalRect::from_size(size),
        );
        if !self.title_bar.show(conn, shm, &pixels)? {
            return Ok(false);
        }
        let surface = self.title_bar.surface;
        match (look.scale, self.viewport) {
            (Scale::Fractional(_), Some(viewport)) => {
                conn.send(core::set_buffer_scale(surface, 1))?;
                let (w, h) = (width as i32, TITLE_BAR_HEIGHT as i32);
                conn.send(scaling::set_destination(viewport, w, h))?;
            }
            // Integer scales stay far below `i32::MAX` (see `ScaleSources`).
            (scale, _) => conn.send(core::set_buffer_scale(surface, scale.factor() as i32))?,
        }
        conn.send(core::commit(surface))?;
        self.drawn = Some(Drawn::new(look));
        self.stale = false;
        Ok(true)
    }

    /// Notes that the compositor released buffer `id`; returns whether it
    /// is one of these surfaces'.
    pub(crate) fn release(&mut self, id: u32) -> bool {
        self.title_bar.buffers.release(id) || self.border.buffers.release(id)
    }

    /// Destroys the surfaces.
    pub(crate) fn destroy(&mut self, conn: &mut Connection) -> Result<(), Error> {
        if let Some(viewport) = self.viewport.take() {
            conn.send(scaling::destroy_viewport(viewport))?;
        }
        self.title_bar.destroy(conn)?;
        self.border.destroy(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::layout::Edges;
    use super::*;

    #[test]
    fn hits_on_each_part() {
        let content = (300, 200);
        let h = f64::from(TITLE_BAR_HEIGHT);
        assert_eq!(
            ClientDecorations::hit(Part::TitleBar, (10.0, 10.0), content),
            Some(Hit::Title)
        );
        assert_eq!(
            ClientDecorations::hit(Part::TitleBar, (295.0, h / 2.0), content),
            Some(Hit::Button(TitleBarButton::Close))
        );
        assert_eq!(
            ClientDecorations::hit(Part::Border, (2.0, 100.0), content),
            Some(Hit::Resize(Edges(Edges::LEFT)))
        );
        let bottom = f64::from(200 + TITLE_BAR_HEIGHT + 2 * RESIZE_MARGIN) - 1.0;
        assert_eq!(
            ClientDecorations::hit(Part::Border, (150.0, bottom), content),
            Some(Hit::Resize(Edges(Edges::BOTTOM)))
        );
        assert_eq!(
            ClientDecorations::hit(Part::Border, (150.0, 100.0), content),
            None
        );
    }
}
