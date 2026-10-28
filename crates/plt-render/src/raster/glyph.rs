//! Glyph outlines flattened exactly as `FreeType`'s smooth rasterizer
//! (`ftgrays.c`) flattens them before matplotlib blits the glyph bitmap.
//!
//! Control points are taken to 26.6 fixed point and upscaled to the
//! rasterizer's 24.8 units; conic arcs are split into `2^n` segments by
//! integer forward differencing until their deviation is within a quarter
//! pixel (`gray_render_conic`), cubic arcs are bisected until flat enough
//! (`gray_render_cubic`). The polygon is returned in pixels; every vertex
//! is a multiple of 1/256 pixel, so the Agg rasterizer sees the same
//! integer coordinates `FreeType` would.

use plt_core::{Path, Point, Segment};

use super::mono::split_cubic as split_coordinate;

/// `FreeType`'s `ONE_PIXEL` in 24.8 units.
const ONE_PIXEL: i64 = 256;

/// A point in 24.8 fixed point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fixed {
    x: i64,
    y: i64,
}

impl Fixed {
    fn to_point(self) -> Point {
        Point::new(self.x as f64 / 256.0, self.y as f64 / 256.0)
    }
}

/// `FT_MulFix`: `a * b / 0x10000`, rounded half away from zero.
fn mul_fix(a: i64, b: i64) -> i64 {
    let ab = a * b;
    (ab + 0x8000 - i64::from(ab < 0)) >> 16
}

/// The transform `FreeType` applies to a glyph outline loaded after
/// `FT_Set_Transform`: a 16.16 rotation matrix, then a 26.6 translation.
#[derive(Clone, Copy, Debug)]
pub(super) struct GlyphTransform {
    xx: i64,
    xy: i64,
    yx: i64,
    yy: i64,
    dx: i64,
    dy: i64,
}

impl GlyphTransform {
    /// matplotlib's `_set_transform` for a glyph rotated by `angle` degrees
    /// with its origin at `pen` (display pixels, already on the 26.6 grid).
    pub(super) fn new(angle: f64, pen: Point) -> Self {
        let (sin, cos) = angle.to_radians().sin_cos();
        let fixed = |v: f64| (65536.0 * v).round_ties_even() as i64;
        Self {
            xx: fixed(cos),
            xy: fixed(-sin),
            yx: fixed(sin),
            yy: fixed(cos),
            dx: (pen.x * 64.0).round() as i64,
            dy: (pen.y * 64.0).round() as i64,
        }
    }

    /// An outline point in 26.6 transformed and translated
    /// (`FT_Vector_Transform`, `FT_Outline_Translate`).
    pub(super) fn transform(&self, x: i64, y: i64) -> (i64, i64) {
        (
            mul_fix(x, self.xx) + mul_fix(y, self.xy) + self.dx,
            mul_fix(x, self.yx) + mul_fix(y, self.yy) + self.dy,
        )
    }

    /// An outline point (pixels, a multiple of 1/64) transformed and
    /// upscaled to 24.8 (`UPSCALE`).
    fn apply(&self, p: Point) -> Fixed {
        let (x, y) = self.transform((p.x * 64.0).round() as i64, (p.y * 64.0).round() as i64);
        Fixed { x: x * 4, y: y * 4 }
    }
}

/// Whether the glyph matrix matplotlib sets for `angle` degrees is, in
/// `FreeType`'s eyes, a rotation by a multiple of 90 degrees (`yx` zero
/// with `xx` not, or the reverse): the only transforms it autohints.
pub(super) fn is_axis_aligned(angle: f64) -> bool {
    let transform = GlyphTransform::new(angle, Point::ORIGIN);
    (transform.yx == 0 && transform.xx != 0) || (transform.xx == 0 && transform.yx != 0)
}

/// `LEFT_SHIFT`: a shift of the two's complement bits.
fn left_shift(a: i64, b: u32) -> i64 {
    ((a as u64) << b) as i64
}

/// Accumulates the flattened polygon.
#[derive(Default)]
struct Flattener {
    vertices: Vec<Point>,
    starts: Vec<usize>,
    current: Option<Fixed>,
}

impl Flattener {
    fn move_to(&mut self, p: Fixed) {
        self.starts.push(self.vertices.len());
        self.vertices.push(p.to_point());
        self.current = Some(p);
    }

    fn line_to(&mut self, p: Fixed) {
        if self.current.is_none() {
            return self.move_to(p);
        }
        self.vertices.push(p.to_point());
        self.current = Some(p);
    }

    /// `gray_render_conic` (the 64-bit forward-differencing variant).
    fn conic_to(&mut self, control: Fixed, to: Fixed) {
        let Some(p0) = self.current else {
            return self.move_to(to);
        };
        let (bx, by) = (control.x - p0.x, control.y - p0.y);
        let (ax, ay) = (to.x - control.x - bx, to.y - control.y - by);
        let mut d = ax.abs().max(ay.abs());
        if d <= ONE_PIXEL / 4 {
            return self.line_to(to);
        }
        let mut shift: u32 = 16;
        loop {
            d >>= 2;
            shift -= 1;
            if d <= ONE_PIXEL / 4 || shift == 0 {
                break;
            }
        }
        let count = 0x10000_u32 >> shift;
        let (mut rx, mut ry) = (left_shift(ax, 2 * shift), left_shift(ay, 2 * shift));
        let mut qx = left_shift(bx, shift + 17).wrapping_add(rx);
        let mut qy = left_shift(by, shift + 17).wrapping_add(ry);
        rx = rx.wrapping_mul(2);
        ry = ry.wrapping_mul(2);
        let (mut px, mut py) = (left_shift(p0.x, 32), left_shift(p0.y, 32));
        for _ in 0..count {
            px = px.wrapping_add(qx);
            py = py.wrapping_add(qy);
            qx = qx.wrapping_add(rx);
            qy = qy.wrapping_add(ry);
            self.line_to(Fixed {
                x: px >> 32,
                y: py >> 32,
            });
        }
    }

    /// `gray_render_cubic`: bisection until the control points are within
    /// half a pixel of the chord's trisection points.
    fn cubic_to(&mut self, c1: Fixed, c2: Fixed, to: Fixed) {
        let Some(p0) = self.current else {
            return self.move_to(to);
        };
        // The arcs to draw, the last one first, as FreeType stacks them.
        let mut stack: Vec<[Fixed; 4]> = vec![[to, c2, c1, p0]];
        while let Some(arc) = stack.pop() {
            let flat = |a: i64, b: i64, c: i64| (a - 3 * b + c).abs() <= ONE_PIXEL / 2;
            let is_flat = flat(2 * arc[0].x, arc[1].x, arc[3].x)
                && flat(2 * arc[0].y, arc[1].y, arc[3].y)
                && flat(arc[0].x, arc[2].x, 2 * arc[3].x)
                && flat(arc[0].y, arc[2].y, 2 * arc[3].y);
            if is_flat || stack.len() >= 16 {
                self.line_to(arc[0]);
            } else {
                let (first, second) = split_cubic(arc);
                stack.push(first);
                stack.push(second);
            }
        }
    }

    fn finish(self) -> Path {
        let mut builder = Path::builder();
        let ends = self
            .starts
            .iter()
            .skip(1)
            .copied()
            .chain([self.vertices.len()]);
        for (&start, end) in self.starts.iter().zip(ends) {
            builder = builder.move_to(self.vertices[start]);
            for &p in &self.vertices[start + 1..end] {
                builder = builder.line_to(p);
            }
            builder = builder.close();
        }
        builder.build()
    }
}

/// `gray_split_cubic` on an arc stored end first: returns the half nearer
/// the end and the half nearer the start (drawn first).
fn split_cubic(arc: [Fixed; 4]) -> ([Fixed; 4], [Fixed; 4]) {
    let split = |v: [i64; 4]| -> [i64; 7] {
        let mut base = [v[0], v[1], v[2], v[3], 0, 0, 0];
        split_coordinate(&mut base);
        base
    };
    let xs = split(arc.map(|p| p.x));
    let ys = split(arc.map(|p| p.y));
    let at = |i: usize| Fixed { x: xs[i], y: ys[i] };
    ([at(0), at(1), at(2), at(3)], [at(3), at(4), at(5), at(6)])
}

/// The glyph `outline` (pixels, origin at the pen) rotated by `angle`
/// degrees and moved to `pen` (display pixels, y up, on the 26.6 grid),
/// flattened as `FreeType` flattens it.
pub(super) fn freetype_polygon(outline: &Path, angle: f64, pen: Point) -> Path {
    let transform = GlyphTransform::new(angle, pen);
    let place = |p: Point| transform.apply(p);
    let mut flattener = Flattener::default();
    for segment in outline.segments() {
        match segment {
            Segment::MoveTo(p) => flattener.move_to(place(p)),
            Segment::LineTo(p) => flattener.line_to(place(p)),
            Segment::QuadTo(c, p) => flattener.conic_to(place(c), place(p)),
            Segment::CubicTo(c1, c2, p) => flattener.cubic_to(place(c1), place(c2), place(p)),
            Segment::Close => {}
        }
    }
    flattener.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conic(p0: Point, c: Point, p2: Point) -> Path {
        Path::builder().move_to(p0).quad_to(c, p2).build()
    }

    #[test]
    fn quarter_turns_are_axis_aligned() {
        for angle in [0.0, 90.0, 180.0, 270.0, -90.0, 360.0] {
            assert!(is_axis_aligned(angle), "{angle}");
        }
        for angle in [30.0, 45.0, 135.0, 89.0] {
            assert!(!is_axis_aligned(angle), "{angle}");
        }
    }

    #[test]
    fn flat_conics_are_lines() {
        let path = freetype_polygon(
            &conic(
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.1),
                Point::new(2.0, 0.0),
            ),
            0.0,
            Point::ORIGIN,
        );
        assert_eq!(path.vertices().len(), 3, "move, line, close");
    }

    #[test]
    fn curved_conics_end_exactly_on_their_end_point() {
        let path = freetype_polygon(
            &conic(
                Point::new(0.0, 0.0),
                Point::new(10.0, 20.0),
                Point::new(20.0, 0.0),
            ),
            0.0,
            Point::ORIGIN,
        );
        // A deviation of 40 px (10240 units) falls within a quarter pixel
        // after four four-fold reductions: 2^4 = 16 segments.
        let vertices = path.vertices();
        assert_eq!(vertices.len(), 1 + 16 + 1);
        assert_eq!(vertices[16], Point::new(20.0, 0.0));
        assert!(vertices.iter().all(|p| (p.x * 256.0).fract() == 0.0));
    }

    #[test]
    fn cubics_are_bisected() {
        let path = Path::builder()
            .move_to(Point::new(0.0, 0.0))
            .cubic_to(
                Point::new(0.0, 10.0),
                Point::new(10.0, 10.0),
                Point::new(10.0, 0.0),
            )
            .build();
        let flat = freetype_polygon(&path, 0.0, Point::ORIGIN);
        let vertices = flat.vertices();
        assert!(vertices.len() > 8);
        assert_eq!(vertices[vertices.len() - 2], Point::new(10.0, 0.0));
        assert!(vertices.iter().all(|p| p.y >= 0.0 && p.y <= 7.5 + 1e-9));
    }
}
