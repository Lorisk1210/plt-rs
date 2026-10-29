//! The outlines of the built-in box shapes around a padded rectangle
//! (the `__call__` of matplotlib's `BoxStyle` subclasses).

use pltrs_core::{Affine2D, Path, PathCode, Point};

use super::ArrowHead;
use crate::numeric::linspace;

/// The most teeth along one side of a sawtooth or roundtooth box.
const MAX_TEETH: f64 = 10_000.0;

/// The padded rectangle every outline is built from.
pub(super) struct Padded {
    pub(super) x0: f64,
    pub(super) y0: f64,
    pub(super) dx: f64,
    pub(super) dy: f64,
}

/// A closed polygon through `points` (matplotlib's `Path._create_closed`).
fn closed(points: &[(f64, f64)]) -> Path {
    Path::polygon(points.iter().map(|&(x, y)| Point::new(x, y)))
}

/// A path from `(code, x, y)` triples.
fn coded(segments: &[(PathCode, f64, f64)]) -> Path {
    let (vertices, codes) = segments
        .iter()
        .map(|&(code, x, y)| (Point::new(x, y), code))
        .unzip();
    Path::new(vertices, codes).expect("box outlines start with a move")
}

impl Padded {
    fn x1(&self) -> f64 {
        self.x0 + self.dx
    }

    fn y1(&self) -> f64 {
        self.y0 + self.dy
    }

    pub(super) fn square(&self) -> Path {
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1(), self.y1());
        closed(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    pub(super) fn circle(&self) -> Path {
        let center = Point::new(self.x0 + self.dx / 2.0, self.y0 + self.dy / 2.0);
        Path::circle(center, self.dx.max(self.dy) / 2.0)
    }

    pub(super) fn ellipse(&self) -> Path {
        let a = self.dx / std::f64::consts::SQRT_2;
        let b = self.dy / std::f64::consts::SQRT_2;
        Path::unit_circle().transformed(
            &Affine2D::scaling(a, b).translate(self.x0 + self.dx / 2.0, self.y0 + self.dy / 2.0),
        )
    }

    /// Mid-height, shaft edges and cotangent of the half tip angle.
    fn arrow_geometry(&self, head: ArrowHead) -> (f64, f64, f64, f64, f64) {
        let head_dy = head.head_width * self.dy;
        let mid_y = f64::midpoint(self.y0, self.y1());
        let cot = 1.0 / (head.head_angle / 2.0).to_radians().tan();
        (
            head_dy,
            mid_y,
            mid_y - head_dy / 2.0,
            mid_y + head_dy / 2.0,
            cot,
        )
    }

    pub(super) fn right_arrow(&self, head: ArrowHead) -> Path {
        let (x0, y0, x1, y1, dy) = (self.x0, self.y0, self.x1(), self.y1(), self.dy);
        let (head_dy, mid_y, shaft_y0, shaft_y1, cot) = self.arrow_geometry(head);
        if cot > 0.0 {
            let tip_x = x1 + cot * dy.min(head_dy) / 2.0;
            let shaft_x = tip_x - cot * head_dy / 2.0;
            closed(&[
                (x0, y0),
                (shaft_x, y0),
                (shaft_x, shaft_y0),
                (tip_x, mid_y),
                (shaft_x, shaft_y1),
                (shaft_x, y1),
                (x0, y1),
            ])
        } else {
            let back = (-cot * (head_dy - dy).max(0.0) / 2.0).min(self.dx);
            let mid_y0 = shaft_y0.min(y0) - back / cot;
            let mid_y1 = shaft_y1.max(y1) + back / cot;
            closed(&[
                (x0, y0),
                (x1 - back, mid_y0),
                (x1, shaft_y0),
                (x1, shaft_y1),
                (x1 - back, mid_y1),
                (x0, y1),
            ])
        }
    }

    pub(super) fn double_arrow(&self, head: ArrowHead) -> Path {
        let (x0, y0, x1, y1, dy) = (self.x0, self.y0, self.x1(), self.y1(), self.dy);
        let (head_dy, mid_y, shaft_y0, shaft_y1, cot) = self.arrow_geometry(head);
        if cot > 0.0 {
            let tip_x0 = x0 - cot * dy.min(head_dy) / 2.0;
            let shaft_x0 = tip_x0 + cot * head_dy / 2.0;
            let tip_x1 = x1 + cot * dy.min(head_dy) / 2.0;
            let shaft_x1 = tip_x1 - cot * head_dy / 2.0;
            closed(&[
                (shaft_x0, y1),
                (shaft_x0, shaft_y1),
                (tip_x0, mid_y),
                (shaft_x0, shaft_y0),
                (shaft_x0, y0),
                (shaft_x1, y0),
                (shaft_x1, shaft_y0),
                (tip_x1, mid_y),
                (shaft_x1, shaft_y1),
                (shaft_x1, y1),
            ])
        } else {
            let back = (-cot * (head_dy - dy).max(0.0) / 2.0).min(self.dx / 2.0);
            let mid_y0 = shaft_y0.min(y0) - back / cot;
            let mid_y1 = shaft_y1.max(y1) + back / cot;
            closed(&[
                (x0, shaft_y0),
                (x0 + back, mid_y0),
                (x1 - back, mid_y0),
                (x1, shaft_y0),
                (x1, shaft_y1),
                (x1 - back, mid_y1),
                (x0 + back, mid_y1),
                (x0, shaft_y1),
            ])
        }
    }

    /// Corners rounded by quadratic Béziers of radius `dr`.
    pub(super) fn round(&self, dr: f64) -> Path {
        use PathCode::{ClosePoly, Curve3, LineTo, MoveTo};
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1(), self.y1());
        coded(&[
            (MoveTo, x0 + dr, y0),
            (LineTo, x1 - dr, y0),
            (Curve3, x1, y0),
            (Curve3, x1, y0 + dr),
            (LineTo, x1, y1 - dr),
            (Curve3, x1, y1),
            (Curve3, x1 - dr, y1),
            (LineTo, x0 + dr, y1),
            (Curve3, x0, y1),
            (Curve3, x0, y1 - dr),
            (LineTo, x0, y0 + dr),
            (Curve3, x0, y0),
            (Curve3, x0 + dr, y0),
            (ClosePoly, x0 + dr, y0),
        ])
    }

    /// Edges bulging by cubic Béziers of rounding `dr`.
    pub(super) fn round4(&self, dr: f64) -> Path {
        use PathCode::{ClosePoly, Curve4, MoveTo};
        let (x0, y0) = (self.x0 + dr, self.y0 + dr);
        let (x1, y1) = (x0 + self.dx - 2.0 * dr, y0 + self.dy - 2.0 * dr);
        coded(&[
            (MoveTo, x0, y0),
            (Curve4, x0 + dr, y0 - dr),
            (Curve4, x1 - dr, y0 - dr),
            (Curve4, x1, y0),
            (Curve4, x1 + dr, y0 + dr),
            (Curve4, x1 + dr, y1 - dr),
            (Curve4, x1, y1),
            (Curve4, x1 - dr, y1 + dr),
            (Curve4, x0 + dr, y1 + dr),
            (Curve4, x0, y1),
            (Curve4, x0 - dr, y1 - dr),
            (Curve4, x0 - dr, y0 + dr),
            (Curve4, x0, y0),
            (ClosePoly, x0, y0),
        ])
    }

    /// The sawtooth vertices, ending with a copy of the first
    /// (`Sawtooth._get_sawtooth_vertices`). Degenerate teeth (a zero or
    /// negative count) leave plain edges; teeth so small that a side would
    /// take more than [`MAX_TEETH`] are widened to that many (matplotlib
    /// allocates them all, until memory runs out).
    pub(super) fn sawtooth(&self, tooth_size: f64) -> Vec<Point> {
        let hsz = tooth_size / 2.0;
        let width = self.dx - tooth_size;
        let height = self.dy - tooth_size;
        let count = |extent: f64| {
            let n = ((extent - tooth_size) / (tooth_size * 2.0)).round_ties_even();
            if n.is_finite() && n > 0.0 {
                n.min(MAX_TEETH) as usize * 2
            } else {
                0
            }
        };
        let (dsx_n, dsy_n) = (count(width), count(height));
        let (x0, y0) = (self.x0 + hsz, self.y0 + hsz);
        let (x1, y1) = (x0 + width, y0 + height);
        let zigzag = |a: f64, n: usize, sign: f64| -> Vec<f64> {
            [a, a - sign * hsz, a, a + sign * hsz]
                .into_iter()
                .cycle()
                .take(4 * n)
                .take(2 * n + 2)
                .collect()
        };
        let mut xs = vec![x0];
        xs.extend(linspace(x0 + hsz, x1 - hsz, 2 * dsx_n + 1));
        xs.extend(zigzag(x1, dsy_n, -1.0));
        xs.push(x1);
        xs.extend(linspace(x1 - hsz, x0 + hsz, 2 * dsx_n + 1));
        xs.extend(zigzag(x0, dsy_n, 1.0));
        let mut ys = zigzag(y0, dsx_n, 1.0);
        ys.push(y0);
        ys.extend(linspace(y0 + hsz, y1 - hsz, 2 * dsy_n + 1));
        ys.extend(zigzag(y1, dsx_n, -1.0));
        ys.push(y1);
        ys.extend(linspace(y1 - hsz, y0 + hsz, 2 * dsy_n + 1));
        let mut points: Vec<Point> = xs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| Point::new(x, y))
            .collect();
        points.push(points[0]);
        points
    }

    /// The sawtooth vertices joined by quadratic Béziers.
    pub(super) fn roundtooth(&self, tooth_size: f64) -> Path {
        let mut vertices = self.sawtooth(tooth_size);
        let first = vertices[0];
        vertices.push(first);
        let curves = (vertices.len() - 1) / 2;
        let mut codes = vec![PathCode::MoveTo];
        codes.extend(std::iter::repeat_n(PathCode::Curve3, 2 * curves));
        codes.push(PathCode::ClosePoly);
        vertices.resize(codes.len(), first);
        Path::new(vertices, codes).expect("roundtooth codes match its vertices")
    }
}
