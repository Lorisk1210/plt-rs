//! Arrow polygons in data coordinates (matplotlib's `Arrow` and
//! `FancyArrow`).

use plt_core::{Affine2D, Path, Point};

use super::{PatchShape, Polygon};

/// A fixed-shape arrow from `(x, y)` to `(x + dx, y + dy)` whose width is
/// scaled by `width` (matplotlib's `Arrow`).
#[derive(Clone, Debug, PartialEq)]
pub struct Arrow {
    /// Tail.
    pub tail: Point,
    /// Extent from tail to tip.
    pub delta: Point,
    /// Width scale: the tail is `0.2 * width` wide, the head `0.6 * width`.
    pub width: f64,
}

impl Arrow {
    /// An arrow of unit width from `(x, y)` to `(x + dx, y + dy)`.
    pub fn new(x: f64, y: f64, dx: f64, dy: f64) -> Self {
        Self {
            tail: Point::new(x, y),
            delta: Point::new(dx, dy),
            width: 1.0,
        }
    }

    /// The same arrow with another width scale.
    #[must_use]
    pub fn with_width(self, width: f64) -> Self {
        Self { width, ..self }
    }
}

impl PatchShape for Arrow {
    fn path(&self) -> Path {
        let p = Point::new;
        Path::polygon([
            p(0.0, 0.1),
            p(0.0, -0.1),
            p(0.8, -0.1),
            p(0.8, -0.3),
            p(1.0, 0.0),
            p(0.8, 0.3),
            p(0.8, 0.1),
        ])
    }

    fn patch_transform(&self) -> Affine2D {
        Affine2D::scaling(self.delta.x.hypot(self.delta.y), self.width)
            .rotate(self.delta.y.atan2(self.delta.x))
            .translate(self.tail.x, self.tail.y)
    }
}

/// Which half of a [`FancyArrow`] is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ArrowShape {
    /// Both halves.
    #[default]
    Full,
    /// The half on the left of the direction of travel.
    Left,
    /// The half on the right.
    Right,
}

/// An arrow polygon with independent head and tail sizes (matplotlib's
/// `FancyArrow`); draws an edge by default.
#[derive(Clone, Debug, PartialEq)]
pub struct FancyArrow {
    /// Tail (the middle of the stem's base).
    pub tail: Point,
    /// Extent from tail to tip (or to the head's base, see
    /// `length_includes_head`).
    pub delta: Point,
    /// Width of the stem.
    pub width: f64,
    /// Whether `delta` includes the head.
    pub length_includes_head: bool,
    /// Full width of the head; `None` is three times the stem width.
    pub head_width: Option<f64>,
    /// Length of the head; `None` is 1.5 times the head width.
    pub head_length: Option<f64>,
    /// Which half is drawn.
    pub shape: ArrowShape,
    /// Fraction by which the head's back is swept towards the tip.
    pub overhang: f64,
    /// Whether the head starts at the end of `delta` instead of ending there.
    pub head_starts_at_zero: bool,
}

impl FancyArrow {
    /// matplotlib's default arrow (stem width 0.001) from `(x, y)` along
    /// `(dx, dy)`.
    pub fn new(x: f64, y: f64, dx: f64, dy: f64) -> Self {
        Self {
            tail: Point::new(x, y),
            delta: Point::new(dx, dy),
            width: 0.001,
            length_includes_head: false,
            head_width: None,
            head_length: None,
            shape: ArrowShape::Full,
            overhang: 0.0,
            head_starts_at_zero: false,
        }
    }

    /// The polygon vertices (`FancyArrow._make_verts`).
    pub fn vertices(&self) -> Vec<Point> {
        let head_width = self.head_width.unwrap_or(3.0 * self.width);
        let head_length = self.head_length.unwrap_or(1.5 * head_width);
        let distance = self.delta.x.hypot(self.delta.y);
        let length = if self.length_includes_head {
            distance
        } else {
            distance + head_length
        };
        let (hw, hl, hs, lw) = (head_width, head_length, self.overhang, self.width);
        // Applied one after the other, as matplotlib shifts the vertices.
        let shifts = [
            (!self.length_includes_head).then_some(head_length),
            self.head_starts_at_zero.then_some(head_length / 2.0),
        ];
        let left: Vec<Point> = [
            (0.0, 0.0),
            (-hl, -hw / 2.0),
            (-hl * (1.0 - hs), -lw / 2.0),
            (-length, -lw / 2.0),
            (-length, 0.0),
        ]
        .into_iter()
        .map(|(x, y)| Point::new(shifts.iter().flatten().fold(x, |x, shift| x + shift), y))
        .collect();
        let mirrored = |p: &Point| Point::new(p.x, -p.y);
        let coords: Vec<Point> = match self.shape {
            ArrowShape::Left => left,
            ArrowShape::Right => left.iter().map(mirrored).collect(),
            ArrowShape::Full => {
                let mut full = left[..left.len() - 1].to_vec();
                full.extend(left[..left.len() - 1].iter().rev().map(mirrored));
                full
            }
        };
        let (cx, sx) = if distance == 0.0 {
            (0.0, 1.0)
        } else {
            (self.delta.x / distance, self.delta.y / distance)
        };
        let tip = self.tail + self.delta;
        coords
            .into_iter()
            .map(|p| Point::new(p.x * cx - p.y * sx + tip.x, p.x * sx + p.y * cx + tip.y))
            .collect()
    }
}

impl PatchShape for FancyArrow {
    const EDGE_DEFAULT: bool = true;

    fn path(&self) -> Path {
        Polygon::new(self.vertices(), true).path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        (a - b).length() < 1e-12
    }

    #[test]
    fn arrow_spans_tail_to_tip() {
        let arrow = Arrow::new(1.0, 1.0, 3.0, 4.0).with_width(2.0);
        let placed = arrow.placed_path();
        assert!(close(placed.vertices()[4], Point::new(4.0, 5.0)));
        assert!(close(
            placed.vertices()[0],
            Point::new(1.0 - 0.16, 1.0 + 0.12)
        ));
    }

    #[test]
    fn fancy_arrow_vertices() {
        let arrow = FancyArrow {
            width: 0.3,
            ..FancyArrow::new(1.0, 2.0, 6.0, 0.0)
        };
        let vertices = arrow.vertices();
        assert_eq!(vertices.len(), 8);
        // The head (0.9 wide, 1.35 long) is added past the tip.
        assert!(close(vertices[0], Point::new(8.35, 2.0)));
        assert!(close(vertices[1], Point::new(7.0, 1.55)));
        assert!(close(vertices[3], Point::new(1.0, 1.85)));
        let half = FancyArrow {
            shape: ArrowShape::Left,
            length_includes_head: true,
            ..arrow.clone()
        };
        assert_eq!(half.vertices().len(), 5);
        assert!(close(half.vertices()[0], Point::new(7.0, 2.0)));
        let vertical = FancyArrow::new(0.0, 0.0, 0.0, 1.0).vertices();
        assert!(vertical[0].y > 1.0 && vertical[0].x.abs() < 1e-12);
    }

    #[test]
    fn zero_length_arrows_point_up() {
        let tip = FancyArrow::new(1.0, 1.0, 0.0, 0.0).vertices()[0];
        assert!(close(tip, Point::new(1.0, 1.0045)));
    }
}
