//! Gouraud-shaded triangles — a port of Agg's `span_gouraud` and
//! `span_gouraud_rgba`, as used by matplotlib's `draw_gouraud_triangles`.
//!
//! The triangle is dilated by half a pixel (rendered as a bevelled
//! hexagon for numerical stability) while colours are interpolated from
//! the corners of the correspondingly enlarged, mitered triangle.

use plt_core::Point;

use crate::color::Rgba8;
use crate::geometry::stroke_math::calc_intersection;

const INTERSECTION_EPSILON: f64 = 1.0e-30;
const SUBPIXEL_SHIFT: i32 = 4;
const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;

fn cross_product(x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> f64 {
    (x - x2) * (y2 - y1) - (y - y2) * (x2 - x1)
}

/// Agg's `iround`.
fn iround(v: f64) -> i32 {
    (if v < 0.0 { v - 0.5 } else { v + 0.5 }) as i32
}

/// Offset of thickness `d` perpendicular to `1 → 2` (Agg's `calc_orthogonal`).
fn orthogonal(d: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> (f64, f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    (d * dy / len, -d * dx / len)
}

#[derive(Clone, Copy, Debug)]
struct Corner {
    x: f64,
    y: f64,
    color: Rgba8,
}

/// A triangle prepared for rendering: its (dilated) outline and the
/// corners used for colour interpolation.
#[derive(Clone, Debug)]
pub(crate) struct GouraudTriangle {
    outline: Vec<Point>,
    corners: [Corner; 3],
}

impl GouraudTriangle {
    /// Prepares a triangle, dilated by `d` pixels when `d != 0`.
    pub(crate) fn new(points: [Point; 3], colors: [Rgba8; 3], d: f64) -> Self {
        let mut corners = [0, 1, 2].map(|i| Corner {
            x: points[i].x,
            y: points[i].y,
            color: colors[i],
        });
        if d == 0.0 {
            return Self {
                outline: points.to_vec(),
                corners,
            };
        }
        let outline = dilate(points, d);
        let o = |i: usize| (outline[i].x, outline[i].y);
        let pairs = [(4, 0), (0, 2), (2, 4)];
        for (corner, (a, b)) in corners.iter_mut().zip(pairs) {
            let ((ax, ay), (bx, by), (cx, cy), (dx, dy)) = (o(a), o(a + 1), o(b), o(b + 1));
            if let Some((x, y)) = calc_intersection(ax, ay, bx, by, cx, cy, dx, dy) {
                corner.x = x;
                corner.y = y;
            }
        }
        Self {
            outline: outline.to_vec(),
            corners,
        }
    }

    /// The polygon to rasterize.
    pub(crate) fn outline(&self) -> &[Point] {
        &self.outline
    }

    /// The span colour generator (Agg's `prepare()`).
    pub(crate) fn spans(&self) -> GouraudSpans {
        let mut c = self.corners;
        if self.corners[0].y > self.corners[2].y {
            c[0] = self.corners[2];
            c[2] = self.corners[0];
        }
        if c[0].y > c[1].y {
            c.swap(0, 1);
        }
        if c[1].y > c[2].y {
            c.swap(1, 2);
        }
        GouraudSpans {
            y2: c[1].y as i32,
            swap: cross_product(c[0].x, c[0].y, c[2].x, c[2].y, c[1].x, c[1].y) < 0.0,
            edges: [
                Edge::new(&c[0], &c[2]),
                Edge::new(&c[0], &c[1]),
                Edge::new(&c[1], &c[2]),
            ],
        }
    }
}

/// Agg's `dilate_triangle`: the six vertices of the triangle grown by `d`.
fn dilate([p1, p2, p3]: [Point; 3], d: f64) -> [Point; 6] {
    let (x1, y1, x2, y2, x3, y3) = (p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
    let mut offsets = [(0.0, 0.0); 3];
    let loc = cross_product(x1, y1, x2, y2, x3, y3);
    if loc.abs() > INTERSECTION_EPSILON {
        let d = if loc > 0.0 { -d } else { d };
        offsets = [
            orthogonal(d, x1, y1, x2, y2),
            orthogonal(d, x2, y2, x3, y3),
            orthogonal(d, x3, y3, x1, y1),
        ];
    }
    let [(dx1, dy1), (dx2, dy2), (dx3, dy3)] = offsets;
    [
        Point::new(x1 + dx1, y1 + dy1),
        Point::new(x2 + dx1, y2 + dy1),
        Point::new(x2 + dx2, y2 + dy2),
        Point::new(x3 + dx2, y3 + dy2),
        Point::new(x3 + dx3, y3 + dy3),
        Point::new(x1 + dx3, y1 + dy3),
    ]
}

/// Colour and position interpolation along one triangle edge (Agg's
/// `rgba_calc`).
#[derive(Clone, Copy, Debug)]
struct Edge {
    x1: f64,
    y1: f64,
    dx: f64,
    inv_dy: f64,
    start: [i32; 4],
    delta: [i32; 4],
    color: [i32; 4],
    x: i32,
}

impl Edge {
    fn new(c1: &Corner, c2: &Corner) -> Self {
        let dy = c2.y - c1.y;
        let start = [c1.color.r, c1.color.g, c1.color.b, c1.color.a].map(i32::from);
        let end = [c2.color.r, c2.color.g, c2.color.b, c2.color.a].map(i32::from);
        Self {
            x1: c1.x - 0.5,
            y1: c1.y - 0.5,
            dx: c2.x - c1.x,
            inv_dy: if dy < 1e-5 { 1e5 } else { 1.0 / dy },
            start,
            delta: [0, 1, 2, 3].map(|i| end[i] - start[i]),
            color: start,
            x: 0,
        }
    }

    fn calc(&mut self, y: f64) {
        let k = ((y - self.y1) * self.inv_dy).clamp(0.0, 1.0);
        for i in 0..4 {
            self.color[i] = self.start[i].wrapping_add(iround(f64::from(self.delta[i]) * k));
        }
        self.x = iround((self.x1 + self.dx * k) * f64::from(SUBPIXEL_SCALE));
    }
}

/// Agg's `dda_line_interpolator<14>` with its wrap-around arithmetic.
#[derive(Clone, Copy, Debug)]
struct Dda {
    y: i32,
    inc: i32,
    dy: i32,
}

impl Dda {
    fn new(y1: i32, y2: i32, count: i32) -> Self {
        Self {
            y: y1,
            inc: ((y2 - y1) << 14) / count,
            dy: 0,
        }
    }

    fn advance(&mut self, n: i32) {
        self.dy = self.dy.wrapping_add(self.inc.wrapping_mul(n));
    }

    fn value(&self) -> i32 {
        self.y.wrapping_add(self.dy >> 14)
    }
}

/// Generates the colours of horizontal spans inside a prepared triangle.
#[derive(Clone, Debug)]
pub(crate) struct GouraudSpans {
    y2: i32,
    swap: bool,
    /// Edges 0→2 (long), 0→1 (upper part), 1→2 (lower part).
    edges: [Edge; 3],
}

impl GouraudSpans {
    /// Fills `out` with the colours of pixels `x..x + out.len()` on row `y`.
    pub(crate) fn generate(&mut self, x: i32, y: i32, out: &mut [Rgba8]) {
        let yf = f64::from(y);
        self.edges[0].calc(yf);
        let second = if y <= self.y2 {
            self.edges[1].calc(yf + self.edges[1].inv_dy);
            1
        } else {
            self.edges[2].calc(yf - self.edges[2].inv_dy);
            2
        };
        let (mut pc1, mut pc2) = (&self.edges[0], &self.edges[second]);
        if self.swap {
            std::mem::swap(&mut pc1, &mut pc2);
        }

        let mut nlen = pc2.x.wrapping_sub(pc1.x).wrapping_abs();
        if nlen <= 0 {
            nlen = 1;
        }
        let mut dda = [0, 1, 2, 3].map(|i| Dda::new(pc1.color[i], pc2.color[i], nlen));
        // Roll the interpolators back to the span start (this may overflow
        // the colour range, hence the clamped head and tail below).
        let mut start = pc1.x.wrapping_sub(x << SUBPIXEL_SHIFT);
        for d in &mut dda {
            d.advance(start.wrapping_neg());
        }
        nlen = nlen.wrapping_add(start);

        // Agg renders a clamped head (before the left edge), an unclamped
        // middle while the computed length lasts, and a clamped tail.
        for px in out.iter_mut() {
            let clamp = start > 0 || nlen <= 0;
            let channel = |d: &Dda| {
                if clamp {
                    d.value().clamp(0, 255) as u8
                } else {
                    d.value() as u8
                }
            };
            *px = Rgba8::new(
                channel(&dda[0]),
                channel(&dda[1]),
                channel(&dda[2]),
                channel(&dda[3]),
            );
            for d in &mut dda {
                d.advance(SUBPIXEL_SCALE);
            }
            if start > 0 {
                start -= SUBPIXEL_SCALE;
                nlen = nlen.wrapping_sub(SUBPIXEL_SCALE);
            } else if nlen > 0 {
                nlen -= SUBPIXEL_SCALE;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn dilated_outline_has_six_vertices_around_the_triangle() {
        let tri = GouraudTriangle::new(
            [p(0.0, 0.0), p(10.0, 0.0), p(0.0, 10.0)],
            [Rgba8::default(); 3],
            0.5,
        );
        assert_eq!(tri.outline().len(), 6);
        // The mitered corners lie outside the original triangle.
        assert!(tri.corners[0].x < 0.0 && tri.corners[0].y < 0.0);
        let plain = GouraudTriangle::new(
            [p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)],
            [Rgba8::default(); 3],
            0.0,
        );
        assert_eq!(plain.outline().len(), 3);
    }

    #[test]
    fn uniform_colour_is_reproduced_exactly() {
        let c = Rgba8::new(10, 200, 30, 255);
        let tri = GouraudTriangle::new([p(0.0, 0.0), p(20.0, 0.0), p(0.0, 20.0)], [c; 3], 0.5);
        let mut spans = tri.spans();
        let mut out = vec![Rgba8::default(); 30];
        spans.generate(-5, 5, &mut out);
        assert!(out.iter().all(|&px| px == c));
    }

    #[test]
    fn horizontal_gradient_is_monotonic() {
        let black = Rgba8::new(0, 0, 0, 255);
        let white = Rgba8::new(255, 255, 255, 255);
        let tri = GouraudTriangle::new(
            [p(0.0, 0.0), p(64.0, 32.0), p(0.0, 64.0)],
            [black, white, black],
            0.5,
        );
        let mut spans = tri.spans();
        let mut out = vec![Rgba8::default(); 64];
        spans.generate(0, 32, &mut out);
        assert!(out.windows(2).all(|w| w[0].r <= w[1].r));
        assert!(out[0].r < 10 && out[60].r > 230);
    }

    #[test]
    fn degenerate_triangle_does_not_panic() {
        let tri = GouraudTriangle::new([p(1.0, 1.0); 3], [Rgba8::new(1, 2, 3, 4); 3], 0.5);
        let mut out = vec![Rgba8::default(); 4];
        tri.spans().generate(0, 1, &mut out);
        assert_eq!(tri.outline().len(), 6);
    }
}
