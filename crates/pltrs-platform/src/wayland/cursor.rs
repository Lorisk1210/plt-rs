//! Pointer cursors: named shapes through `wp_cursor_shape_v1` when the
//! compositor offers it, else simple cursors drawn into cursor surfaces.

use std::collections::HashMap;

use super::connection::Connection;
use super::decoration::layout::Edges;
use super::protocol::Interface;
use super::protocol::core;
use super::protocol::input;
use super::shm::{PixelFormat, Pixels, Swapchain};
use crate::cursor_art::{GRID, Outline};
use crate::error::Error;
use crate::frame::Frame;
use crate::geometry::{PhysicalRect, PhysicalSize};
use crate::window::CursorIcon;

/// A cursor shape: the crate's icons and the resize cursors of the
/// decorations' borders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Shape {
    /// An icon applications ask for.
    Icon(CursorIcon),
    /// Resizing these edges.
    Resize(Edges),
}

impl Shape {
    /// The `wp_cursor_shape_device_v1.shape` code.
    fn code(self) -> u32 {
        match self {
            Self::Icon(CursorIcon::Pointer) => 4,
            Self::Icon(CursorIcon::Wait) => 6,
            Self::Icon(CursorIcon::Crosshair) => 8,
            Self::Icon(CursorIcon::Text) => 9,
            Self::Icon(CursorIcon::Move) => 13,
            Self::Icon(CursorIcon::ResizeHorizontal) => 26,
            Self::Icon(CursorIcon::ResizeVertical) => 27,
            Self::Resize(Edges(edges)) => match edges {
                Edges::TOP => 19,
                Edges::BOTTOM => 22,
                Edges::LEFT => 25,
                Edges::RIGHT => 18,
                e if e == Edges::TOP | Edges::LEFT => 21,
                e if e == Edges::TOP | Edges::RIGHT => 20,
                e if e == Edges::BOTTOM | Edges::LEFT => 24,
                _ => 23,
            },
            Self::Icon(_) => 1,
        }
    }

    /// The drawn cursor's outline.
    fn outline(self) -> Outline {
        match self {
            Self::Icon(icon) => Outline::of_icon(icon),
            Self::Resize(Edges(edges)) => Outline::double_arrow(match edges {
                Edges::LEFT | Edges::RIGHT => 0.0,
                Edges::TOP | Edges::BOTTOM => 90.0,
                e if e == Edges::TOP | Edges::LEFT || e == Edges::BOTTOM | Edges::RIGHT => 45.0,
                _ => 135.0,
            }),
        }
    }

    /// The cursor drawn at an integer `scale`: black with a white border,
    /// premultiplied RGBA.
    fn draw(self, scale: u32) -> (PhysicalSize, Vec<u8>) {
        let side = GRID * scale;
        (PhysicalSize::new(side, side), self.outline().draw(side))
    }
}

/// A drawn cursor's surface; its buffer stays attached for good.
#[derive(Debug, Clone, Copy)]
struct Image {
    surface: u32,
    hotspot: (i32, i32),
}

/// The cursors shown so far.
#[derive(Debug, Default)]
pub(crate) struct Cursors {
    /// `wp_cursor_shape_device_v1` of the pointer, if offered.
    pub(crate) shape_device: Option<u32>,
    images: HashMap<(Shape, u32), Image>,
}

impl Cursors {
    /// Shows `shape` for the pointer entered with `serial`, drawn at
    /// `scale` when it must be drawn.
    pub(crate) fn show(
        &mut self,
        conn: &mut Connection,
        globals: (u32, u32),
        pointer: u32,
        serial: u32,
        shape: Shape,
        scale: u32,
    ) -> Result<(), Error> {
        if let Some(device) = self.shape_device {
            return conn.send(input::set_shape(device, serial, shape.code()));
        }
        let scale = scale.clamp(1, 4);
        let image = match self.images.entry((shape, scale)) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(draw_image(conn, globals, shape, scale)?)
            }
        };
        conn.send(input::set_cursor(
            pointer,
            serial,
            image.surface,
            image.hotspot,
        ))
    }
}

/// Creates a surface showing `shape` at `scale`; `globals` are the
/// compositor and `wl_shm`.
fn draw_image(
    conn: &mut Connection,
    (compositor, shm): (u32, u32),
    shape: Shape,
    scale: u32,
) -> Result<Image, Error> {
    let (size, rgba) = shape.draw(scale);
    let frame = Frame::new(size.width, size.height, &rgba)?;
    let mut pixels = Pixels::new(size, PixelFormat::Premultiplied)?;
    let full = PhysicalRect::from_size(size);
    pixels.blit(&frame, full);
    let surface = conn.new_id(Interface::Surface)?;
    conn.send(core::create_surface(compositor, surface))?;
    let mut buffers = Swapchain::new(PixelFormat::Premultiplied);
    buffers.attach(conn, shm, surface, &pixels, full)?;
    // Cursor scales are clamped to 1 ..= 4.
    conn.send(core::set_buffer_scale(surface, scale as i32))?;
    conn.send(core::commit(surface))?;
    Ok(Image {
        surface,
        hotspot: shape.outline().hotspot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_have_cursor_shape_codes() {
        assert_eq!(Shape::Icon(CursorIcon::Default).code(), 1);
        assert_eq!(Shape::Icon(CursorIcon::Text).code(), 9);
        assert_eq!(Shape::Resize(Edges(Edges::TOP | Edges::LEFT)).code(), 21);
        assert_eq!(
            Shape::Resize(Edges(Edges::BOTTOM | Edges::RIGHT)).code(),
            23
        );
        assert_eq!(Shape::Resize(Edges(Edges::RIGHT)).code(), 18);
    }

    #[test]
    fn drawn_cursors_are_black_on_white() {
        for shape in [
            Shape::Icon(CursorIcon::Default),
            Shape::Icon(CursorIcon::Text),
            Shape::Icon(CursorIcon::Move),
            Shape::Resize(Edges(Edges::TOP | Edges::RIGHT)),
        ] {
            let (size, pixels) = shape.draw(2);
            assert_eq!(size, PhysicalSize::new(48, 48));
            assert!(pixels.chunks(4).any(|px| px == [0, 0, 0, 255]), "{shape:?}");
            assert!(pixels.chunks(4).any(|px| px == [255; 4]), "{shape:?}");
            assert!(pixels.chunks(4).any(|px| px[3] == 0), "{shape:?}");
        }
        assert_eq!(Shape::Icon(CursorIcon::Default).outline().hotspot, (1, 1));
        assert_eq!(
            Shape::Resize(Edges(Edges::LEFT)).outline(),
            Outline::double_arrow(0.0)
        );
    }
}
