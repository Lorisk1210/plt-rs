//! Glyph outlines flattened exactly as `FreeType`'s smooth rasterizer
//! (`ftgrays.c`) flattens them before matplotlib blits the glyph bitmap.
//!
//! The tagged 26.6 outline is transformed first (`FT_Load_Glyph` under
//! `FT_Set_Transform`), then decomposed (`FT_Outline_Decompose`), so the
//! points a run of conic control points implies lie between transformed
//! points; they are upscaled to the rasterizer's 24.8 units; conic arcs
//! are split into `2^n` segments by integer forward differencing until
//! their deviation is within a quarter pixel (`gray_render_conic`), cubic
//! arcs are bisected until flat enough (`gray_render_cubic`). The contours
//! are returned in those 24.8 units, for `pltrs_raster::render_glyph`,
//! `FreeType`'s rasterizer.
//!
//! `FreeType`'s outline handling and flattening are copyright David
//! Turner, Robert Wilhelm and Werner Lemberg, and ported here under the
//! `FreeType` License (FTL).

use pltrs_core::Point;
use pltrs_font::{ScanOutline, ScanPointKind};

use super::mono::split_cubic as split_coordinate;

/// `FreeType`'s `ONE_PIXEL` in 24.8 units.
const ONE_PIXEL: i64 = 256;

/// A point in 24.8 fixed point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fixed {
    x: i64,
    y: i64,
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
        Self::with_effects(angle, 0.0, 1.0, pen)
    }

    /// matplotlib's `_set_transform` for a glyph of a font slanted by
    /// `slant` and extended by `extend` (`psfonts.map` effects), rotated
    /// by `angle` degrees with its origin at `pen`: the rotation matrix in
    /// 16.16 times the effects, rounded to integers.
    pub(super) fn with_effects(angle: f64, slant: f64, extend: f64, pen: Point) -> Self {
        let (sin, cos) = angle.to_radians().sin_cos();
        let (c, s) = (65536.0 * cos, 65536.0 * -sin);
        let (sc, cc) = (65536.0 * sin, 65536.0 * cos);
        // `(0x10000 * R) @ [[extend, extend * slant], [0, 1]]`.
        let fixed = |v: f64| v.round_ties_even() as i64;
        Self {
            xx: fixed(c * extend + s * 0.0),
            xy: fixed(c * (extend * slant) + s),
            yx: fixed(sc * extend + cc * 0.0),
            yy: fixed(sc * (extend * slant) + cc),
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

/// Accumulates the flattened contours.
#[derive(Default)]
struct Flattener {
    contours: Vec<Vec<(i64, i64)>>,
    current: Option<Fixed>,
}

impl Flattener {
    fn move_to(&mut self, p: Fixed) {
        self.contours.push(vec![(p.x, p.y)]);
        self.current = Some(p);
    }

    fn line_to(&mut self, p: Fixed) {
        match self.contours.last_mut() {
            Some(contour) if self.current.is_some() => contour.push((p.x, p.y)),
            _ => return self.move_to(p),
        }
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

/// A transformed outline point in 26.6 with its role.
#[derive(Clone, Copy)]
struct Tagged {
    x: i64,
    y: i64,
    kind: ScanPointKind,
}

impl Tagged {
    /// The point upscaled to 24.8 (`UPSCALE`).
    fn upscaled(self) -> Fixed {
        Fixed {
            x: left_shift(self.x, 2),
            y: left_shift(self.y, 2),
        }
    }

    /// The point halfway to `other`, rounded down as the rasterizer sees
    /// it: `FT_Outline_Decompose`'s `(a + b) / 2`, taken in the glyph's
    /// bitmap, where no coordinate is negative.
    fn halfway(self, other: Self) -> Self {
        Self {
            x: (self.x + other.x) >> 1,
            y: (self.y + other.y) >> 1,
            kind: ScanPointKind::On,
        }
    }
}

/// The tagged glyph `outline` (26.6, origin at the pen) placed by
/// `transform` (display pixels, y up), flattened as `FreeType` flattens
/// it: one polygon per contour, in 24.8 units. `None` for a malformed
/// outline, which `FreeType` refuses (`Invalid_Outline`).
pub(super) fn freetype_contours(
    outline: &ScanOutline,
    transform: &GlyphTransform,
) -> Option<Vec<Vec<(i64, i64)>>> {
    let points: Vec<Tagged> = outline
        .points
        .iter()
        .map(|point| {
            let (x, y) = transform.transform(point.x, point.y);
            Tagged {
                x,
                y,
                kind: point.kind,
            }
        })
        .collect();
    let mut flattener = Flattener::default();
    let mut start = 0;
    for &end in &outline.contour_ends {
        decompose(points.get(start..end)?, &mut flattener)?;
        start = end;
    }
    Some(flattener.contours)
}

/// `FT_Outline_Decompose` of one contour into `flattener`, closed back to
/// its start; `None` for a malformed contour.
fn decompose(contour: &[Tagged], flattener: &mut Flattener) -> Option<()> {
    let (&first, &last) = (contour.first()?, contour.last()?);
    let mut limit = contour.len() - 1;
    // The index of the next point to read.
    let mut next = 1;
    let start = match first.kind {
        ScanPointKind::Cubic => return None,
        ScanPointKind::Conic => {
            next = 0;
            if last.kind == ScanPointKind::On {
                limit -= 1;
                last
            } else {
                first.halfway(last)
            }
        }
        ScanPointKind::On => first,
    };
    flattener.move_to(start.upscaled());
    while next <= limit {
        let point = contour[next];
        next += 1;
        match point.kind {
            ScanPointKind::On => flattener.line_to(point.upscaled()),
            ScanPointKind::Conic => {
                let mut control = point;
                loop {
                    let Some(&to) = contour.get(next).filter(|_| next <= limit) else {
                        flattener.conic_to(control.upscaled(), start.upscaled());
                        return Some(());
                    };
                    next += 1;
                    match to.kind {
                        ScanPointKind::On => {
                            flattener.conic_to(control.upscaled(), to.upscaled());
                            break;
                        }
                        ScanPointKind::Conic => {
                            flattener.conic_to(control.upscaled(), control.halfway(to).upscaled());
                            control = to;
                        }
                        ScanPointKind::Cubic => return None,
                    }
                }
            }
            ScanPointKind::Cubic => {
                let second = contour.get(next).filter(|p| p.kind == ScanPointKind::Cubic);
                let second = *second.filter(|_| next <= limit)?;
                next += 1;
                let to = if next <= limit { contour[next] } else { start };
                flattener.cubic_to(point.upscaled(), second.upscaled(), to.upscaled());
                if next > limit {
                    return Some(());
                }
                next += 1;
            }
        }
    }
    flattener.line_to(start.upscaled());
    Some(())
}

#[cfg(test)]
mod tests {
    use pltrs_font::{Dropouts, ScanPoint, ScanRules};

    use super::*;

    /// A one-contour outline of `points` (26.6, with their roles).
    fn outline(points: &[(i64, i64, ScanPointKind)]) -> ScanOutline {
        ScanOutline {
            points: points
                .iter()
                .map(|&(x, y, kind)| ScanPoint { x, y, kind })
                .collect(),
            contour_ends: vec![points.len()],
            rules: ScanRules {
                high_precision: false,
                dropouts: Dropouts::Simple { stubs: false },
            },
        }
    }

    fn contours(points: &[(i64, i64, ScanPointKind)], angle: f64) -> Vec<Vec<(i64, i64)>> {
        freetype_contours(&outline(points), &GlyphTransform::new(angle, Point::ORIGIN))
            .expect("a valid outline")
    }

    use ScanPointKind::{Conic, Cubic, On};

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
    fn effects_slant_and_extend_the_rotation() {
        let plain = GlyphTransform::new(30.0, Point::ORIGIN);
        let same = GlyphTransform::with_effects(30.0, 0.0, 1.0, Point::ORIGIN);
        assert_eq!(plain.transform(64, 64), same.transform(64, 64));
        let slanted = GlyphTransform::with_effects(0.0, 0.25, 2.0, Point::new(1.0, 2.0));
        // x' = 2x + 0.5y + 64, y' = y + 128 (26.6).
        assert_eq!(slanted.transform(64, 64), (2 * 64 + 32 + 64, 64 + 128));
    }

    #[test]
    fn flat_conics_are_lines() {
        let contours = contours(&[(0, 0, On), (64, 6, Conic), (128, 0, On)], 0.0);
        assert_eq!(contours, [vec![(0, 0), (512, 0), (0, 0)]]);
    }

    #[test]
    fn curved_conics_end_exactly_on_their_end_point() {
        let contours = contours(&[(0, 0, On), (640, 1280, Conic), (1280, 0, On)], 0.0);
        // A deviation of 40 px (10240 units) falls within a quarter pixel
        // after four four-fold reductions: 2^4 = 16 segments, then the
        // closing line.
        let [contour] = contours.as_slice() else {
            panic!("one contour, got {contours:?}");
        };
        assert_eq!(contour.len(), 1 + 16 + 1);
        assert_eq!(contour[16], (20 * 256, 0));
    }

    #[test]
    fn implied_points_lie_between_transformed_control_points() {
        // Two conic controls imply an on-curve point halfway between them,
        // at y = 1/128 px: rounded down after a half turn (FreeType), not
        // turned after rounding.
        let points = [(0, 0, On), (64, 1, Conic), (128, 0, Conic), (192, 0, On)];
        let turned = contours(&points, 180.0);
        let upright = contours(&points, 0.0);
        let mirrored: Vec<(i64, i64)> = upright[0].iter().map(|&(x, y)| (-x, -y)).collect();
        assert_ne!(turned[0], mirrored);
        assert!(turned[0].contains(&(-96 * 4, -4)), "{:?}", turned[0]);
    }

    #[test]
    fn contours_may_start_at_a_control_point() {
        let starts_off = contours(&[(64, 64, Conic), (128, 0, On), (0, 0, On)], 0.0);
        let starts_on = contours(&[(0, 0, On), (64, 64, Conic), (128, 0, On)], 0.0);
        assert_eq!(starts_off[0][0], (0, 0));
        assert_eq!(starts_off[0].last(), starts_on[0].last());
    }

    #[test]
    fn cubics_are_bisected() {
        let contours = contours(
            &[(0, 0, On), (0, 640, Cubic), (640, 640, Cubic), (640, 0, On)],
            0.0,
        );
        let contour = &contours[0];
        assert!(contour.len() > 8);
        assert_eq!(contour[contour.len() - 2], (10 * 256, 0));
        assert!(
            contour
                .iter()
                .all(|&(_, y)| (0..=7 * 256 + 128).contains(&y))
        );
    }

    #[test]
    fn malformed_contours_are_refused() {
        let transform = GlyphTransform::new(0.0, Point::ORIGIN);
        let lone_cubic = outline(&[(0, 0, On), (64, 64, Cubic), (128, 0, On)]);
        assert!(freetype_contours(&lone_cubic, &transform).is_none());
        let cubic_start = outline(&[(0, 0, Cubic), (64, 64, Cubic), (128, 0, On)]);
        assert!(freetype_contours(&cubic_start, &transform).is_none());
    }
}
