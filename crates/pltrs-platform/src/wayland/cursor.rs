//! Pointer cursors: named shapes through `wp_cursor_shape_v1` when the
//! compositor offers it, else simple cursors drawn into cursor surfaces.

use std::collections::HashMap;

use super::connection::Connection;
use super::decoration::layout::Edges;
use super::draw::{Canvas, Point};
use super::protocol::Interface;
use super::protocol::core;
use super::protocol::input;
use super::shm::{PixelFormat, Pixels, Swapchain};
use crate::error::Error;
use crate::frame::Frame;
use crate::geometry::{PhysicalRect, PhysicalSize};
use crate::window::CursorIcon;

/// The size of drawn cursors, in logical pixels.
const SIZE: u32 = 24;

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

    /// The outline of the drawn cursor, in [`SIZE`] units, and its hot
    /// spot.
    fn outline(self) -> (Vec<Vec<Point>>, (i32, i32)) {
        const CENTRE: (i32, i32) = (12, 12);
        match self {
            Self::Icon(CursorIcon::Text) => (
                vec![
                    rect((11.0, 4.0), (13.0, 20.0)),
                    rect((8.0, 3.0), (16.0, 5.0)),
                    rect((8.0, 19.0), (16.0, 21.0)),
                ],
                CENTRE,
            ),
            Self::Icon(CursorIcon::Crosshair) => (
                vec![
                    rect((11.0, 3.0), (13.0, 21.0)),
                    rect((3.0, 11.0), (21.0, 13.0)),
                ],
                CENTRE,
            ),
            Self::Icon(CursorIcon::Move) => {
                let mut parts = double_arrow(0.0);
                parts.extend(double_arrow(90.0));
                (parts, CENTRE)
            }
            Self::Icon(CursorIcon::ResizeHorizontal) => (double_arrow(0.0), CENTRE),
            Self::Icon(CursorIcon::ResizeVertical) => (double_arrow(90.0), CENTRE),
            Self::Resize(Edges(edges)) => {
                let angle = match edges {
                    Edges::LEFT | Edges::RIGHT => 0.0,
                    Edges::TOP | Edges::BOTTOM => 90.0,
                    e if e == Edges::TOP | Edges::LEFT || e == Edges::BOTTOM | Edges::RIGHT => 45.0,
                    _ => 135.0,
                };
                (double_arrow(angle), CENTRE)
            }
            Self::Icon(_) => {
                let arrow = vec![
                    (1.0, 1.0),
                    (1.0, 18.0),
                    (5.0, 14.0),
                    (8.0, 21.0),
                    (11.0, 20.0),
                    (8.0, 13.0),
                    (13.0, 13.0),
                ];
                (vec![arrow], (1, 1))
            }
        }
    }

    /// The cursor drawn at an integer `scale`: black with a white border,
    /// premultiplied RGBA.
    fn draw(self, scale: u32) -> (PhysicalSize, Vec<u8>) {
        let size = PhysicalSize::new(SIZE * scale, SIZE * scale);
        let mut pixels = vec![0; size.area() as usize * 4];
        let mut canvas = Canvas::new(&mut pixels, size.width);
        let s = f64::from(scale);
        let (parts, _) = self.outline();
        let scaled = |part: &[Point], (dx, dy): Point| -> Vec<Point> {
            part.iter()
                .map(|&(x, y)| (x * s + dx, y * s + dy))
                .collect()
        };
        for dx in [-s, 0.0, s] {
            for dy in [-s, 0.0, s] {
                for part in &parts {
                    canvas.fill(&scaled(part, (dx, dy)), [255; 4]);
                }
            }
        }
        for part in &parts {
            canvas.fill(&scaled(part, (0.0, 0.0)), [0, 0, 0, 255]);
        }
        (size, pixels)
    }
}

/// A rectangle as a polygon.
fn rect((x0, y0): Point, (x1, y1): Point) -> Vec<Point> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

/// A double-headed arrow through the centre, turned by `degrees`.
fn double_arrow(degrees: f64) -> Vec<Vec<Point>> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let turn = |(x, y): Point| {
        let (x, y) = (x - 12.0, y - 12.0);
        (12.0 + x * cos - y * sin, 12.0 + x * sin + y * cos)
    };
    let shaft = rect((6.0, 11.0), (18.0, 13.0));
    let left = vec![(2.0, 12.0), (8.0, 7.0), (8.0, 17.0)];
    let right = vec![(22.0, 12.0), (16.0, 17.0), (16.0, 7.0)];
    [shaft, left, right]
        .into_iter()
        .map(|part| part.into_iter().map(turn).collect())
        .collect()
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
        hotspot: shape.outline().1,
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
        let (_, hotspot) = Shape::Icon(CursorIcon::Default).outline();
        assert_eq!(hotspot, (1, 1));
    }
}
