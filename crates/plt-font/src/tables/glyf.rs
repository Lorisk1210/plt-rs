//! `loca` + `glyf`: TrueType glyph outlines, simple and composite.

use crate::error::{FontError, Result};
use crate::fixed::{hypot, mul_fix};
use crate::outline::{Outline, PointKind};
use crate::reader::{Array, Reader, slice, tail};
use crate::types::{GlyphId, Rect, Tag};

const GLYF: Tag = Tag::new(b"glyf");
const LOCA: Tag = Tag::new(b"loca");

/// Deepest composite nesting accepted (guards against reference cycles).
const MAX_COMPONENT_DEPTH: u8 = 16;

const ON_CURVE: u8 = 0x01;
const X_SHORT: u8 = 0x02;
const Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME_OR_POSITIVE: u8 = 0x10;
const Y_SAME_OR_POSITIVE: u8 = 0x20;

const ARGS_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const HAVE_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const HAVE_XY_SCALE: u16 = 0x0040;
const HAVE_TWO_BY_TWO: u16 = 0x0080;
const USE_MY_METRICS: u16 = 0x0200;
const SCALED_COMPONENT_OFFSET: u16 = 0x0800;

/// Coordinate space in which glyph points are produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Scaling {
    /// Raw font units (composite transforms applied in floating point).
    Units,
    /// FreeType 26.6 pixels: every coordinate scaled with `FT_MulFix` by
    /// the given 16.16 scales, composites transformed in fixed point.
    Fixed { x_scale: i64, y_scale: i64 },
}

impl Scaling {
    fn x(self, value: i64) -> f64 {
        match self {
            Self::Units => value as f64,
            Self::Fixed { x_scale, .. } => mul_fix(value, x_scale) as f64,
        }
    }

    fn y(self, value: i64) -> f64 {
        match self {
            Self::Units => value as f64,
            Self::Fixed { y_scale, .. } => mul_fix(value, y_scale) as f64,
        }
    }
}

/// A component's 2×2 matrix as 2.14 values: `[xx, yx, xy, yy]` where
/// `x' = xx*x + xy*y` and `y' = yx*x + yy*y`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Matrix([i16; 4]);

impl Matrix {
    const IDENTITY: Self = Self([0x4000, 0, 0, 0x4000]);

    fn apply(self, scaling: Scaling, x: f64, y: f64) -> (f64, f64) {
        let [xx, yx, xy, yy] = self.0;
        match scaling {
            Scaling::Units => {
                let f = |v: i16| f64::from(v) / 16384.0;
                (f(xx) * x + f(xy) * y, f(yx) * x + f(yy) * y)
            }
            Scaling::Fixed { .. } => {
                // FT_Vector_Transform with the matrix widened to 16.16.
                let f = |v: i16| i64::from(v) << 2;
                let (x, y) = (x as i64, y as i64);
                (
                    (mul_fix(x, f(xx)) + mul_fix(y, f(xy))) as f64,
                    (mul_fix(x, f(yx)) + mul_fix(y, f(yy))) as f64,
                )
            }
        }
    }

    /// Scale factors applied to offsets of `SCALED_COMPONENT_OFFSET`
    /// components, as 16.16 values.
    fn offset_scales(self) -> (i64, i64) {
        let [xx, yx, xy, yy] = self.0.map(|v| i64::from(v) << 2);
        (hypot(xx, xy), hypot(yy, yx))
    }
}

/// Glyph locations plus glyph data.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Glyf<'a> {
    short_offsets: Option<Array<'a, u16>>,
    long_offsets: Option<Array<'a, u32>>,
    data: &'a [u8],
}

/// Header values of one glyph record.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct GlyphHeader {
    pub(crate) contours: i16,
    pub(crate) bbox: Rect,
}

impl<'a> Glyf<'a> {
    /// Combines `loca` (in the given format) with `glyf`.
    pub(crate) fn new(loca: &'a [u8], glyf: &'a [u8], long_format: bool) -> Self {
        if long_format {
            Self {
                short_offsets: None,
                long_offsets: Some(Array::new(loca)),
                data: glyf,
            }
        } else {
            Self {
                short_offsets: Some(Array::new(loca)),
                long_offsets: None,
                data: glyf,
            }
        }
    }

    fn offset(&self, index: usize) -> Option<usize> {
        match (self.short_offsets, self.long_offsets) {
            (Some(short), _) => short.get(index).map(|v| usize::from(v) * 2),
            (_, Some(long)) => long.get(index).map(|v| v as usize),
            _ => None,
        }
    }

    /// Raw record of `glyph`; empty for glyphs without outline. Like
    /// FreeType, a record running past the end of `glyf` is clamped.
    pub(crate) fn data(&self, glyph: GlyphId) -> Result<&'a [u8]> {
        let index = usize::from(glyph.0);
        let start = self.offset(index).ok_or(FontError::Truncated(LOCA))?;
        let end = self
            .offset(index + 1)
            .ok_or(FontError::Truncated(LOCA))?
            .min(self.data.len());
        if end <= start {
            return Ok(&[]);
        }
        slice(self.data, start, end - start, GLYF)
    }

    /// The record header and the body that follows it.
    fn record(&self, glyph: GlyphId) -> Result<Option<(GlyphHeader, &'a [u8])>> {
        let data = self.data(glyph)?;
        if data.is_empty() {
            return Ok(None);
        }
        let mut r = Reader::new(data, GLYF);
        let contours = r.i16()?;
        let bbox = Rect {
            x_min: i32::from(r.i16()?),
            y_min: i32::from(r.i16()?),
            x_max: i32::from(r.i16()?),
            y_max: i32::from(r.i16()?),
        };
        Ok(Some((
            GlyphHeader { contours, bbox },
            tail(data, r.pos(), GLYF)?,
        )))
    }

    /// Loads `glyph` into `out` and returns the x position of its first
    /// phantom point relative to the outline (FreeType's `pp1.x`, in font
    /// units), which the caller uses to align the origin.
    pub(crate) fn load(
        &self,
        glyph: GlyphId,
        scaling: Scaling,
        lsb: &dyn Fn(GlyphId) -> i16,
        out: &mut Outline,
    ) -> Result<i64> {
        self.load_recursive(glyph, scaling, lsb, out, 0)
    }

    fn load_recursive(
        &self,
        glyph: GlyphId,
        scaling: Scaling,
        lsb: &dyn Fn(GlyphId) -> i16,
        out: &mut Outline,
        depth: u8,
    ) -> Result<i64> {
        if depth > MAX_COMPONENT_DEPTH {
            return Err(FontError::malformed(
                GLYF,
                "composite glyphs nested too deeply",
            ));
        }
        let Some((header, body)) = self.record(glyph)? else {
            return Ok(-i64::from(lsb(glyph)));
        };
        let pp1 = i64::from(header.bbox.x_min) - i64::from(lsb(glyph));
        if header.contours >= 0 {
            load_simple(body, header.contours as usize, scaling, out)?;
            Ok(pp1)
        } else {
            self.load_composite(body, scaling, lsb, out, depth, pp1)
        }
    }

    fn load_composite(
        &self,
        body: &[u8],
        scaling: Scaling,
        lsb: &dyn Fn(GlyphId) -> i16,
        out: &mut Outline,
        depth: u8,
        mut pp1: i64,
    ) -> Result<i64> {
        let base_points = out.points.len();
        let mut r = Reader::new(body, GLYF);
        loop {
            let component = Component::parse(&mut r)?;
            let start = out.points.len();
            let component_pp1 =
                self.load_recursive(component.glyph, scaling, lsb, out, depth + 1)?;
            if component.flags & USE_MY_METRICS != 0 {
                pp1 = component_pp1;
            }
            if component.matrix != Matrix::IDENTITY {
                for point in &mut out.points[start..] {
                    (point.x, point.y) = component.matrix.apply(scaling, point.x, point.y);
                }
            }
            let (dx, dy) = component.offset(scaling, out, base_points, start)?;
            if dx != 0.0 || dy != 0.0 {
                for point in &mut out.points[start..] {
                    point.x += dx;
                    point.y += dy;
                }
            }
            if component.flags & MORE_COMPONENTS == 0 {
                return Ok(pp1);
            }
        }
    }
}

/// How a component is positioned.
#[derive(Clone, Copy, Debug)]
enum Placement {
    /// Offset in font units.
    Offset(i64, i64),
    /// Align child point `.1` with parent point `.0`.
    Anchor(usize, usize),
}

#[derive(Clone, Copy, Debug)]
struct Component {
    flags: u16,
    glyph: GlyphId,
    placement: Placement,
    matrix: Matrix,
}

impl Component {
    fn parse(r: &mut Reader<'_>) -> Result<Self> {
        let flags = r.u16()?;
        let glyph = r.glyph()?;
        let (a, b) = if flags & ARGS_ARE_WORDS != 0 {
            let (a, b) = (r.u16()?, r.u16()?);
            if flags & ARGS_ARE_XY_VALUES != 0 {
                (i64::from(a as i16), i64::from(b as i16))
            } else {
                (i64::from(a), i64::from(b))
            }
        } else {
            let (a, b) = (r.u8()?, r.u8()?);
            if flags & ARGS_ARE_XY_VALUES != 0 {
                (i64::from(a as i8), i64::from(b as i8))
            } else {
                (i64::from(a), i64::from(b))
            }
        };
        let placement = if flags & ARGS_ARE_XY_VALUES != 0 {
            Placement::Offset(a, b)
        } else {
            Placement::Anchor(a as usize, b as usize)
        };
        let matrix = if flags & HAVE_SCALE != 0 {
            let s = r.f2dot14()?;
            Matrix([s, 0, 0, s])
        } else if flags & HAVE_XY_SCALE != 0 {
            let (sx, sy) = (r.f2dot14()?, r.f2dot14()?);
            Matrix([sx, 0, 0, sy])
        } else if flags & HAVE_TWO_BY_TWO != 0 {
            Matrix([r.f2dot14()?, r.f2dot14()?, r.f2dot14()?, r.f2dot14()?])
        } else {
            Matrix::IDENTITY
        };
        Ok(Self {
            flags,
            glyph,
            placement,
            matrix,
        })
    }

    /// Translation to apply to the component points (already transformed)
    /// that start at index `start` in `out`.
    fn offset(
        &self,
        scaling: Scaling,
        out: &Outline,
        base: usize,
        start: usize,
    ) -> Result<(f64, f64)> {
        match self.placement {
            Placement::Offset(mut x, mut y) => {
                if x == 0 && y == 0 {
                    return Ok((0.0, 0.0));
                }
                let have_scale = self.flags & (HAVE_SCALE | HAVE_XY_SCALE | HAVE_TWO_BY_TWO) != 0;
                if have_scale && self.flags & SCALED_COMPONENT_OFFSET != 0 {
                    let (sx, sy) = self.matrix.offset_scales();
                    x = mul_fix(x, sx);
                    y = mul_fix(y, sy);
                }
                Ok((scaling.x(x), scaling.y(y)))
            }
            Placement::Anchor(parent, child) => {
                let parent = out
                    .points
                    .get(base..start)
                    .and_then(|points| points.get(parent));
                let child = out.points.get(start..).and_then(|points| points.get(child));
                match (parent, child) {
                    (Some(p), Some(c)) => Ok((p.x - c.x, p.y - c.y)),
                    _ => Err(FontError::malformed(
                        GLYF,
                        "component anchor point out of range",
                    )),
                }
            }
        }
    }
}

/// Decodes a simple glyph body (after the 10-byte header).
fn load_simple(body: &[u8], contours: usize, scaling: Scaling, out: &mut Outline) -> Result<()> {
    let mut r = Reader::new(body, GLYF);
    let ends: Array<'_, u16> = r.array(contours)?;
    let Some(last) = ends.last() else {
        return Ok(());
    };
    let mut previous: Option<u16> = None;
    for end in ends.iter() {
        if previous.is_some_and(|p| end <= p) {
            return Err(FontError::malformed(
                GLYF,
                "contour end points not increasing",
            ));
        }
        previous = Some(end);
    }
    let count = usize::from(last) + 1;
    let instructions = usize::from(r.u16()?);
    r.skip(instructions)?;

    let mut flags = Vec::with_capacity(count);
    while flags.len() < count {
        let flag = r.u8()?;
        flags.push(flag);
        if flag & REPEAT != 0 {
            let repeat = usize::from(r.u8()?);
            flags.extend(std::iter::repeat_n(flag, repeat.min(count - flags.len())));
        }
    }
    let xs = read_coordinates(&mut r, &flags, X_SHORT, X_SAME_OR_POSITIVE)?;
    let ys = read_coordinates(&mut r, &flags, Y_SHORT, Y_SAME_OR_POSITIVE)?;

    let mut contour_start = 0;
    for end in ends.iter() {
        let end = usize::from(end) + 1;
        for index in contour_start..end {
            let kind = if flags[index] & ON_CURVE != 0 {
                PointKind::OnCurve
            } else {
                PointKind::Quadratic
            };
            out.push(scaling.x(xs[index]), scaling.y(ys[index]), kind);
        }
        out.end_contour();
        contour_start = end;
    }
    Ok(())
}

fn read_coordinates(
    r: &mut Reader<'_>,
    flags: &[u8],
    short: u8,
    same_or_positive: u8,
) -> Result<Vec<i64>> {
    let mut value = 0i64;
    flags
        .iter()
        .map(|&flag| {
            let delta = if flag & short != 0 {
                let magnitude = i64::from(r.u8()?);
                if flag & same_or_positive != 0 {
                    magnitude
                } else {
                    -magnitude
                }
            } else if flag & same_or_positive != 0 {
                0
            } else {
                i64::from(r.i16()?)
            };
            value += delta;
            Ok(value)
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::outline::OutlinePoint;

    /// Encodes a simple glyph with uncompressed coordinates.
    pub(crate) fn simple_glyph(contours: &[&[(i16, i16, bool)]]) -> Vec<u8> {
        let points: Vec<_> = contours.iter().flat_map(|c| c.iter().copied()).collect();
        let mut out = (contours.len() as i16).to_be_bytes().to_vec();
        let xs = points.iter().map(|p| p.0);
        let ys = points.iter().map(|p| p.1);
        for v in [xs.clone().min(), ys.clone().min(), xs.max(), ys.max()] {
            out.extend_from_slice(&v.unwrap_or(0).to_be_bytes());
        }
        let mut end = 0u16;
        for contour in contours {
            end += contour.len() as u16;
            out.extend_from_slice(&(end - 1).to_be_bytes());
        }
        out.extend_from_slice(&[0, 0]);
        out.extend(points.iter().map(|p| u8::from(p.2)));
        let mut previous = (0i16, 0i16);
        let mut ys = Vec::new();
        for p in &points {
            out.extend_from_slice(&(p.0 - previous.0).to_be_bytes());
            ys.extend_from_slice(&(p.1 - previous.1).to_be_bytes());
            previous = (p.0, p.1);
        }
        out.extend_from_slice(&ys);
        out
    }

    /// Builds `(loca, glyf)` in long format from glyph records.
    pub(crate) fn tables(glyphs: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
        let mut loca = Vec::new();
        let mut glyf = Vec::new();
        for glyph in glyphs {
            loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
            glyf.extend_from_slice(glyph);
        }
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        (loca, glyf)
    }

    fn load(glyf: &Glyf<'_>, glyph: u16, scaling: Scaling) -> Result<Outline> {
        let mut out = Outline::default();
        glyf.load(GlyphId(glyph), scaling, &|_| 0, &mut out)?;
        Ok(out)
    }

    fn square() -> Vec<u8> {
        simple_glyph(&[&[
            (0, 0, true),
            (100, 0, true),
            (100, 100, false),
            (0, 100, true),
        ]])
    }

    #[test]
    fn simple_glyph_points() {
        let (loca, data) = tables(&[Vec::new(), square()]);
        let glyf = Glyf::new(&loca, &data, true);
        assert_eq!(load(&glyf, 0, Scaling::Units).unwrap().points, Vec::new());
        let outline = load(&glyf, 1, Scaling::Units).unwrap();
        assert_eq!(outline.contour_ends, vec![4]);
        assert_eq!(
            outline.points[2],
            OutlinePoint {
                x: 100.0,
                y: 100.0,
                kind: PointKind::Quadratic
            }
        );
        let (header, _) = glyf.record(GlyphId(1)).unwrap().unwrap();
        assert_eq!(
            header.bbox,
            Rect {
                x_min: 0,
                y_min: 0,
                x_max: 100,
                y_max: 100
            }
        );
        let scaled = load(
            &glyf,
            1,
            Scaling::Fixed {
                x_scale: 0x8000,
                y_scale: 0x1_0000,
            },
        )
        .unwrap();
        assert_eq!((scaled.points[1].x, scaled.points[2].y), (50.0, 100.0));
    }

    #[test]
    fn compressed_flags_and_coordinates() {
        // One contour, three points; flags use repeat, short and same bits.
        let mut glyph = vec![0, 1, 0, 0, 0, 0, 0, 10, 0, 10, 0, 2, 0, 0];
        glyph.extend_from_slice(&[
            ON_CURVE | X_SHORT | Y_SHORT | X_SAME_OR_POSITIVE | Y_SAME_OR_POSITIVE | REPEAT,
            1,
        ]);
        glyph.push(ON_CURVE | X_SAME_OR_POSITIVE | Y_SHORT);
        glyph.extend_from_slice(&[5, 5, 5, 5, 3]);
        let (loca, data) = tables(&[glyph]);
        let glyf = Glyf::new(&loca, &data, true);
        let outline = load(&glyf, 0, Scaling::Units).unwrap();
        let xy: Vec<_> = outline.points.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(xy, vec![(5.0, 5.0), (10.0, 10.0), (10.0, 7.0)]);
    }

    fn composite(components: &[(u16, u16, &[u8])]) -> Vec<u8> {
        let mut out = vec![0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0];
        for (index, (flags, glyph, args)) in components.iter().enumerate() {
            let more = if index + 1 < components.len() {
                MORE_COMPONENTS
            } else {
                0
            };
            out.extend_from_slice(&(flags | more).to_be_bytes());
            out.extend_from_slice(&glyph.to_be_bytes());
            out.extend_from_slice(args);
        }
        out
    }

    #[test]
    fn composite_offsets_and_transforms() {
        let comp = composite(&[
            (ARGS_ARE_XY_VALUES, 0, &[10, 0xFE]),
            (
                ARGS_ARE_XY_VALUES | ARGS_ARE_WORDS | HAVE_SCALE,
                0,
                &[0x01, 0x00, 0, 0, 0x20, 0x00],
            ),
        ]);
        let (loca, data) = tables(&[square(), comp]);
        let glyf = Glyf::new(&loca, &data, true);
        let outline = load(&glyf, 1, Scaling::Units).unwrap();
        assert_eq!(outline.contour_ends, vec![4, 8]);
        assert_eq!((outline.points[1].x, outline.points[1].y), (110.0, -2.0));
        assert_eq!((outline.points[5].x, outline.points[5].y), (306.0, 0.0));
    }

    #[test]
    fn composite_two_by_two_and_anchor() {
        let comp = composite(&[
            (
                ARGS_ARE_XY_VALUES | HAVE_TWO_BY_TWO,
                0,
                &[0, 0, 0, 0, 0x40, 0, 0xC0, 0, 0, 0],
            ),
            (0, 0, &[1, 0]),
        ]);
        let (loca, data) = tables(&[square(), comp]);
        let glyf = Glyf::new(&loca, &data, true);
        let outline = load(&glyf, 1, Scaling::Units).unwrap();
        // Rotation by 90°: (100, 0) -> (0, 100) under [xx=0, yx=1, xy=-1, yy=0].
        assert_eq!((outline.points[1].x, outline.points[1].y), (0.0, 100.0));
        // Second component's point 0 aligned with the first's point 1.
        assert_eq!((outline.points[4].x, outline.points[4].y), (0.0, 100.0));
    }

    #[test]
    fn malformed_glyphs_error() {
        let recursive = composite(&[(ARGS_ARE_XY_VALUES, 0, &[0, 0])]);
        let (loca, data) = tables(&[recursive]);
        let glyf = Glyf::new(&loca, &data, true);
        assert!(load(&glyf, 0, Scaling::Units).is_err());

        let bad_anchor = composite(&[(0, 0, &[9, 9])]);
        let (loca, data) = tables(&[square(), bad_anchor]);
        let glyf = Glyf::new(&loca, &data, true);
        assert!(load(&glyf, 1, Scaling::Units).is_err());

        let full = square();
        for len in 1..full.len() {
            let (loca, data) = tables(&[full[..len].to_vec()]);
            let glyf = Glyf::new(&loca, &data, true);
            assert!(load(&glyf, 0, Scaling::Units).is_err(), "len {len}");
        }
        let (loca, data) = tables(&[square()]);
        let glyf = Glyf::new(&loca[..4], &data, true);
        assert!(load(&glyf, 0, Scaling::Units).is_err());
    }

    #[test]
    fn decreasing_contour_ends_are_rejected() {
        let mut glyph = simple_glyph(&[&[(0, 0, true), (1, 1, true)], &[(2, 2, true)]]);
        glyph[10..14].copy_from_slice(&[0, 2, 0, 1]);
        let (loca, data) = tables(&[glyph]);
        let glyf = Glyf::new(&loca, &data, true);
        assert!(load(&glyf, 0, Scaling::Units).is_err());
    }

    #[test]
    fn short_loca_format() {
        let glyph = square();
        let loca = [0, 0, 0, (glyph.len() / 2) as u8];
        let glyf = Glyf::new(&loca, &glyph, false);
        assert_eq!(load(&glyf, 0, Scaling::Units).unwrap().points.len(), 4);
    }
}
