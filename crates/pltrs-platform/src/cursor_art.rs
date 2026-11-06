//! The cursors the platform draws itself, at any size, where the display
//! server offers none at the screen's scale: Wayland compositors without
//! `wp_cursor_shape_v1`, and X servers, whose cursor font has 16-pixel
//! cursors whatever the scale. Each is black with a white border, drawn
//! from vector outlines on a [`GRID`]-unit square.

use crate::canvas::{Canvas, Point};
use crate::window::CursorIcon;

/// The side of the square the outlines are designed on, and the size of
/// a cursor at scale 1.
pub(crate) const GRID: u32 = 24;

/// The centre of the design square, the hot spot of symmetric cursors.
const CENTRE: (i32, i32) = (12, 12);

/// A cursor's shape: filled polygons and the hot spot, in grid units.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Outline {
    /// The polygons, filled with the non-zero rule.
    pub(crate) parts: Vec<Vec<Point>>,
    /// The point the cursor points with.
    pub(crate) hotspot: (i32, i32),
}

impl Outline {
    /// The outline of `icon`.
    pub(crate) fn of_icon(icon: CursorIcon) -> Self {
        let centred = |parts| Self {
            parts,
            hotspot: CENTRE,
        };
        match icon {
            CursorIcon::Default => Self {
                parts: vec![vec![
                    (1.0, 1.0),
                    (1.0, 18.0),
                    (5.0, 14.0),
                    (8.0, 21.0),
                    (11.0, 20.0),
                    (8.0, 13.0),
                    (13.0, 13.0),
                ]],
                hotspot: (1, 1),
            },
            CursorIcon::Pointer => Self {
                parts: vec![
                    // The raised index finger, then the fist.
                    rect((8.0, 2.0), (11.0, 13.0)),
                    vec![
                        (5.0, 12.0),
                        (8.0, 11.0),
                        (8.0, 15.0),
                        (11.0, 10.0),
                        (18.0, 11.0),
                        (19.0, 17.0),
                        (16.0, 22.0),
                        (9.0, 22.0),
                    ],
                ],
                hotspot: (9, 2),
            },
            CursorIcon::Wait => centred(vec![
                rect((5.0, 2.0), (19.0, 4.0)),
                rect((5.0, 20.0), (19.0, 22.0)),
                vec![(7.0, 4.0), (17.0, 4.0), (13.0, 12.0), (11.0, 12.0)],
                vec![(11.0, 12.0), (13.0, 12.0), (17.0, 20.0), (7.0, 20.0)],
            ]),
            CursorIcon::Text => centred(vec![
                rect((11.0, 4.0), (13.0, 20.0)),
                rect((8.0, 3.0), (16.0, 5.0)),
                rect((8.0, 19.0), (16.0, 21.0)),
            ]),
            CursorIcon::Crosshair => centred(vec![
                rect((11.0, 3.0), (13.0, 21.0)),
                rect((3.0, 11.0), (21.0, 13.0)),
            ]),
            CursorIcon::Move => {
                let mut parts = Self::double_arrow(0.0).parts;
                parts.extend(Self::double_arrow(90.0).parts);
                centred(parts)
            }
            CursorIcon::ResizeHorizontal => Self::double_arrow(0.0),
            CursorIcon::ResizeVertical => Self::double_arrow(90.0),
        }
    }

    /// A double-headed arrow through the centre, turned by `degrees`
    /// (0 points left and right).
    pub(crate) fn double_arrow(degrees: f64) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let turn = |(x, y): Point| {
            let (x, y) = (x - 12.0, y - 12.0);
            (12.0 + x * cos - y * sin, 12.0 + x * sin + y * cos)
        };
        let shaft = rect((6.0, 11.0), (18.0, 13.0));
        let left = vec![(2.0, 12.0), (8.0, 7.0), (8.0, 17.0)];
        let right = vec![(22.0, 12.0), (16.0, 17.0), (16.0, 7.0)];
        Self {
            parts: [shaft, left, right]
                .into_iter()
                .map(|part| part.into_iter().map(turn).collect())
                .collect(),
            hotspot: CENTRE,
        }
    }

    /// The cursor drawn on a `size` x `size` square: premultiplied RGBA,
    /// black with a white border one grid unit (and at least a pixel)
    /// wide.
    pub(crate) fn draw(&self, size: u32) -> Vec<u8> {
        let mut pixels = vec![0; size as usize * size as usize * 4];
        let mut canvas = Canvas::new(&mut pixels, size);
        let s = f64::from(size) / f64::from(GRID);
        let border = s.max(1.0);
        let scaled = |part: &[Point], (dx, dy): Point| -> Vec<Point> {
            part.iter()
                .map(|&(x, y)| (x * s + dx, y * s + dy))
                .collect()
        };
        for dx in [-border, 0.0, border] {
            for dy in [-border, 0.0, border] {
                for part in &self.parts {
                    canvas.fill(&scaled(part, (dx, dy)), [255; 4]);
                }
            }
        }
        for part in &self.parts {
            canvas.fill(&scaled(part, (0.0, 0.0)), [0, 0, 0, 255]);
        }
        pixels
    }

    /// The hot spot on a cursor drawn `size` pixels wide.
    pub(crate) fn hotspot_at(&self, size: u32) -> (u32, u32) {
        let at = |unit: i32| {
            let scaled = f64::from(unit) * f64::from(size) / f64::from(GRID);
            // Within 0 ..= size: the cast is exact.
            scaled.round().clamp(0.0, f64::from(size)) as u32
        };
        (at(self.hotspot.0), at(self.hotspot.1))
    }
}

/// A rectangle as a polygon.
fn rect((x0, y0): Point, (x1, y1): Point) -> Vec<Point> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICONS: [CursorIcon; 8] = [
        CursorIcon::Default,
        CursorIcon::Pointer,
        CursorIcon::Crosshair,
        CursorIcon::Move,
        CursorIcon::Wait,
        CursorIcon::Text,
        CursorIcon::ResizeHorizontal,
        CursorIcon::ResizeVertical,
    ];

    #[test]
    fn every_icon_is_black_on_white_at_any_size() {
        for icon in ICONS {
            let outline = Outline::of_icon(icon);
            for size in [16, 24, 36, 48, 72] {
                let pixels = outline.draw(size);
                assert_eq!(pixels.len(), (size * size * 4) as usize);
                assert!(pixels.chunks(4).any(|px| px[3] == 0), "{icon:?}");
                // Off the grid's multiples, strokes and borders less than
                // two pixels wide may cover no pixel whole.
                let (black, white) = if size % GRID == 0 {
                    (0, 255)
                } else {
                    (128, 128)
                };
                let opaque = |px: &&[u8]| px[3] == 255;
                let mut shown = pixels.chunks(4).filter(opaque);
                assert!(shown.clone().any(|px| px[0] <= black), "{icon:?} at {size}");
                assert!(shown.any(|px| px[0] >= white), "{icon:?} at {size}");
            }
            let (x, y) = outline.hotspot;
            assert!((0..24).contains(&x) && (0..24).contains(&y), "{icon:?}");
        }
    }

    #[test]
    fn hot_spots_scale_with_the_cursor() {
        let arrow = Outline::of_icon(CursorIcon::Default);
        assert_eq!(arrow.hotspot_at(24), (1, 1));
        assert_eq!(arrow.hotspot_at(48), (2, 2));
        let cross = Outline::of_icon(CursorIcon::Crosshair);
        assert_eq!(cross.hotspot_at(36), (18, 18));
        assert_eq!(cross.hotspot_at(0), (0, 0));
        assert_eq!(Outline::of_icon(CursorIcon::Pointer).draw(0).len(), 0);
    }

    #[test]
    fn the_hand_points_up_with_its_finger() {
        let hand = Outline::of_icon(CursorIcon::Pointer);
        let pixels = hand.draw(24);
        let (x, y) = hand.hotspot;
        let below_tip = ((y as usize + 1) * 24 + x as usize + 1) * 4;
        assert_eq!(&pixels[below_tip..below_tip + 4], [0, 0, 0, 255]);
    }
}
