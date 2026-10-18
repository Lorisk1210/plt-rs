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

/// The most components one glyph load visits. Components shared between
/// the levels of a composite multiply exponentially with the nesting depth
/// (and empty components add no points to stop at).
const MAX_COMPONENTS: u32 = 0xFFFF;

/// The most points one glyph may have (FreeType's `FT_OUTLINE_POINTS_MAX`).
const MAX_POINTS: usize = 0xFFFF;

/// What one glyph load has used up so far.
#[derive(Default)]
struct LoadBudget {
    /// Current composite nesting depth.
    depth: u8,
    /// Components visited.
    components: u32,
}

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
        self.load_recursive(glyph, scaling, lsb, out, &mut LoadBudget::default())
    }

    fn load_recursive(
        &self,
        glyph: GlyphId,
        scaling: Scaling,
        lsb: &dyn Fn(GlyphId) -> i16,
        out: &mut Outline,
        budget: &mut LoadBudget,
    ) -> Result<i64> {
        if budget.depth > MAX_COMPONENT_DEPTH {
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
            self.load_composite(body, scaling, lsb, out, budget, pp1)
        }
    }

    fn load_composite(
        &self,
        body: &[u8],
        scaling: Scaling,
        lsb: &dyn Fn(GlyphId) -> i16,
        out: &mut Outline,
        budget: &mut LoadBudget,
        mut pp1: i64,
    ) -> Result<i64> {
        let base_points = out.points.len();
        let mut r = Reader::new(body, GLYF);
        loop {
            let component = Component::parse(&mut r)?;
            let start = out.points.len();
            budget.components += 1;
            if budget.components > MAX_COMPONENTS {
                return Err(FontError::malformed(
                    GLYF,
                    "composite glyph has too many components",
                ));
            }
            budget.depth += 1;
            let component_pp1 = self.load_recursive(component.glyph, scaling, lsb, out, budget)?;
            budget.depth -= 1;
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
    if out.points.len() + count > MAX_POINTS {
        return Err(FontError::malformed(GLYF, "glyph has too many points"));
    }
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
mod tests;
