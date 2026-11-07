//! `gvar`: the point deltas of TrueType glyphs at an instance
//! (`ft_var_load_gvar`, `TT_Vary_Apply_Glyph_Deltas`), with the points
//! without deltas interpolated as the `IUP` instruction does
//! (`tt_interpolate_deltas`). Deltas are summed in 16.16 and rounded last.

use super::store::f2dot14_to_fixed;
use crate::error::{FontError, Result};
use crate::fixed::{div_fix, mul_div, mul_fix};
use crate::reader::Reader;
use crate::types::{GlyphId, Tag};

const GVAR: Tag = Tag::new(b"gvar");

pub(super) const SHARE_POINT_NUMBERS: u16 = 0x8000;
pub(super) const TUPLE_COUNT_MASK: u16 = 0x0FFF;
const EMBEDDED_TUPLE: u16 = 0x8000;
const INTERMEDIATE_TUPLE: u16 = 0x4000;
const PRIVATE_POINT_NUMBERS: u16 = 0x2000;
const TUPLE_INDEX_MASK: u16 = 0x0FFF;

/// A point delta in 16.16 font units.
pub(crate) type Delta = (i64, i64);

/// A `gvar` table.
#[derive(Clone, Debug)]
pub(crate) struct Gvar<'a> {
    data: &'a [u8],
    /// The shared tuples' peaks, in 16.16 (which `cvar` tuples refer to
    /// as well).
    pub(super) shared: Vec<Vec<i64>>,
    /// Where each glyph's data starts in the table, and the end of the
    /// last.
    offsets: Vec<usize>,
}

/// `FT_fixedToInt`: a 16.16 delta rounded to whole units.
pub(crate) fn fixed_to_int(value: i64) -> i64 {
    i64::from(((value + 0x8000) >> 16) as i16)
}

/// `FT_fixedToFdot6`: a 16.16 delta in 26.6.
pub(crate) fn fixed_to_f26dot6(value: i64) -> i64 {
    (value + 0x200) >> 10
}

impl<'a> Gvar<'a> {
    /// Parses a `gvar` table of a font of `axis_count` axes.
    pub(crate) fn parse(data: &'a [u8], axis_count: usize) -> Result<Self> {
        let invalid = |why| FontError::malformed(GVAR, why);
        let mut r = Reader::new(data, GVAR);
        if r.u32()? != 0x0001_0000 {
            return Err(invalid("unknown gvar version"));
        }
        if usize::from(r.u16()?) != axis_count {
            return Err(invalid("gvar of other axes than fvar"));
        }
        let shared_count = usize::from(r.u16()?);
        let shared_at = r.u32()? as usize;
        let glyph_count = usize::from(r.u16()?);
        let long = r.u16()? & 1 != 0;
        let data_at = r.u32()? as usize;
        if shared_count * axis_count > data.len() / 2 {
            return Err(invalid("too many shared tuples"));
        }
        let mut max = 0;
        let offsets = (0..=glyph_count)
            .map(|_| {
                let offset = if long {
                    r.u32()? as usize
                } else {
                    usize::from(r.u16()?) * 2
                };
                // FreeType keeps offsets monotonic and within the table.
                max = max.max(data_at.saturating_add(offset));
                Ok(max.min(data.len()))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut r = Reader::at(data, shared_at, GVAR)?;
        let shared = (0..shared_count)
            .map(|_| {
                (0..axis_count)
                    .map(|_| r.i16().map(f2dot14_to_fixed))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            data,
            shared,
            offsets,
        })
    }

    /// The summed deltas of `glyph`'s `points` (in font units: the
    /// outline's, or a composite's component offsets, then the four
    /// phantom points) whose contours end at `ends`, at the normalized
    /// coordinates `coords`; `None` when the glyph has no variation data.
    ///
    /// # Errors
    ///
    /// When the glyph's variation data are malformed.
    pub(crate) fn deltas(
        &self,
        glyph: GlyphId,
        points: &[(i64, i64)],
        ends: &[usize],
        coords: &[i64],
    ) -> Result<Option<Vec<Delta>>> {
        let index = usize::from(glyph.0);
        let (Some(&start), Some(&end)) = (self.offsets.get(index), self.offsets.get(index + 1))
        else {
            return Ok(None);
        };
        if start >= end {
            return Ok(None);
        }
        let data = &self.data[start..end];
        let invalid = || FontError::malformed(GVAR, "invalid glyph variation data");
        let mut r = Reader::new(data, GVAR);
        let tuple_field = r.u16()?;
        let mut data_at = usize::from(r.u16()?);
        let tuple_count = usize::from(tuple_field & TUPLE_COUNT_MASK);
        if data_at > data.len() || tuple_count * 4 > data.len() {
            return Err(invalid());
        }
        let shared_points = if tuple_field & SHARE_POINT_NUMBERS != 0 {
            let mut points = Reader::new(&data[data_at..], GVAR);
            let read = read_points(&mut points);
            data_at += points.pos();
            read
        } else {
            None
        };
        let count = points.len();
        let mut sums = vec![(0i64, 0i64); count];
        let origin: Vec<(i64, i64)> = points.iter().map(|&(x, y)| (x << 16, y << 16)).collect();
        for _ in 0..tuple_count {
            let tuple = TupleHeader::read(&mut r, &self.shared, coords, GVAR)?;
            let tuple_data = data.get(data_at..).unwrap_or_default();
            data_at += tuple.size;
            if tuple.apply == 0 {
                continue;
            }
            let mut tuple_reader = Reader::new(tuple_data, GVAR);
            let points_read = if tuple.private_points {
                read_points(&mut tuple_reader)
            } else {
                shared_points.clone()
            };
            let wanted = match &points_read {
                Some(Points::Some(list)) => list.len(),
                _ => count,
            };
            let deltas_x = read_deltas(&mut tuple_reader, wanted);
            let deltas_y = read_deltas(&mut tuple_reader, wanted);
            if let (Some(points_read), Some(dx), Some(dy)) = (points_read, deltas_x, deltas_y) {
                accumulate(
                    &points_read,
                    (&dx, &dy),
                    tuple.apply,
                    ends,
                    &origin,
                    &mut sums,
                );
            }
            // FreeType ignores a tuple it cannot read.
        }
        Ok(Some(sums))
    }
}

/// The header of a tuple variation record (`gvar` and `cvar`): the size
/// of its data, whether they start with points of their own, and its
/// weight at the coordinates.
pub(super) struct TupleHeader {
    pub(super) size: usize,
    pub(super) private_points: bool,
    pub(super) apply: i64,
}

impl TupleHeader {
    /// Reads the header at `r`, its peak embedded or one of the `shared`
    /// tuples, and weighs it at `coords`.
    pub(super) fn read(
        r: &mut Reader<'_>,
        shared: &[Vec<i64>],
        coords: &[i64],
        tag: Tag,
    ) -> Result<Self> {
        let invalid = || FontError::malformed(tag, "invalid tuple variation header");
        let size = usize::from(r.u16()?);
        let index = r.u16()?;
        let peaks = |r: &mut Reader<'_>| {
            (0..coords.len())
                .map(|_| r.i16().map(f2dot14_to_fixed))
                .collect::<Result<Vec<_>>>()
                .map_err(|_| invalid())
        };
        let peak = if index & EMBEDDED_TUPLE != 0 {
            peaks(r)?
        } else {
            shared
                .get(usize::from(index & TUPLE_INDEX_MASK))
                .cloned()
                .ok_or_else(invalid)?
        };
        let intermediate = if index & INTERMEDIATE_TUPLE != 0 {
            Some((peaks(r)?, peaks(r)?))
        } else {
            None
        };
        Ok(Self {
            size,
            private_points: index & PRIVATE_POINT_NUMBERS != 0,
            apply: tuple_scalar(&peak, intermediate.as_ref(), coords),
        })
    }
}

/// Adds one tuple's deltas, weighed by `apply`, to `sums`: to every point,
/// or to the points listed and, as `IUP` would move them, to the others of
/// their contours.
fn accumulate(
    points: &Points,
    (dx, dy): (&[i64], &[i64]),
    apply: i64,
    ends: &[usize],
    origin: &[(i64, i64)],
    sums: &mut [(i64, i64)],
) {
    match points {
        Points::All => {
            for (sum, (&x, &y)) in sums.iter_mut().zip(dx.iter().zip(dy)) {
                sum.0 += mul_fix(x, apply);
                sum.1 += mul_fix(y, apply);
            }
        }
        Points::Some(list) => {
            let count = origin.len();
            let mut out = origin.to_vec();
            let mut has_delta = vec![false; count];
            for (&index, (&x, &y)) in list.iter().zip(dx.iter().zip(dy)) {
                let index = usize::from(index);
                if index < count {
                    has_delta[index] = true;
                    out[index].0 += mul_fix(x, apply);
                    out[index].1 += mul_fix(y, apply);
                }
            }
            interpolate(ends, &mut out, origin, &has_delta);
            for ((sum, out), origin) in sums.iter_mut().zip(&out).zip(origin) {
                sum.0 += out.0 - origin.0;
                sum.1 += out.1 - origin.1;
            }
        }
    }
}

/// `ft_var_apply_tuple`: the weight of a tuple at `coords`.
fn tuple_scalar(peak: &[i64], intermediate: Option<&(Vec<i64>, Vec<i64>)>, coords: &[i64]) -> i64 {
    let mut apply: i64 = 0x10000;
    for (axis, (&peak, &coord)) in peak.iter().zip(coords).enumerate() {
        if peak == 0 {
            continue;
        }
        if coord == 0 {
            return 0;
        }
        if peak == coord {
            continue;
        }
        match intermediate {
            None => {
                if (peak > coord && coord > 0) || (peak < coord && coord < 0) {
                    apply = mul_div(apply, coord, peak);
                } else {
                    return 0;
                }
            }
            Some((starts, ends)) => {
                let (start, end) = (starts[axis], ends[axis]);
                if coord <= start || coord >= end {
                    return 0;
                }
                apply = if coord < peak {
                    mul_div(apply, coord - start, peak - start)
                } else {
                    mul_div(apply, end - coord, end - peak)
                };
            }
        }
    }
    apply
}

/// Which points a tuple's deltas are for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Points {
    All,
    Some(Vec<u16>),
}

/// `ft_var_readpackedpoints`: `None` when the data end inside the list.
pub(super) fn read_points(r: &mut Reader<'_>) -> Option<Points> {
    let mut count = usize::from(r.u8().ok()?);
    if count == 0 {
        return Some(Points::All);
    }
    if count & 0x80 != 0 {
        count = (count & 0x7F) << 8 | usize::from(r.u8().ok()?);
    }
    let mut points = Vec::with_capacity(count);
    let mut current: u16 = 0;
    while points.len() < count {
        let run = r.u8().ok()?;
        let length = (usize::from(run & 0x7F) + 1).min(count - points.len());
        for _ in 0..length {
            let step = if run & 0x80 != 0 {
                r.u16().ok()?
            } else {
                u16::from(r.u8().ok()?)
            };
            current = current.wrapping_add(step);
            points.push(current);
        }
    }
    Some(Points::Some(points))
}

/// `ft_var_readpackeddeltas`: `count` deltas in 16.16, `None` when the
/// data end inside them.
pub(super) fn read_deltas(r: &mut Reader<'_>, count: usize) -> Option<Vec<i64>> {
    let mut deltas = Vec::with_capacity(count);
    while deltas.len() < count {
        let run = r.u8().ok()?;
        let length = (usize::from(run & 0x3F) + 1).min(count - deltas.len());
        for _ in 0..length {
            let delta = if run & 0x80 != 0 {
                0
            } else if run & 0x40 != 0 {
                i64::from(r.i16().ok()?)
            } else {
                i64::from(r.u8().ok()? as i8)
            };
            deltas.push(delta << 16);
        }
    }
    Some(deltas)
}

/// `tt_interpolate_deltas`: the points of each contour without a delta
/// moved as the `IUP` instruction moves them, from the original points
/// `origin` and the moved ones `out`.
fn interpolate(ends: &[usize], out: &mut [(i64, i64)], origin: &[(i64, i64)], has_delta: &[bool]) {
    let mut first = 0;
    for &end in ends {
        if end >= out.len() {
            break;
        }
        let Some(first_delta) = (first..=end).find(|&point| has_delta[point]) else {
            first = end + 1;
            continue;
        };
        let mut current = first_delta;
        let touched = (first_delta + 1..=end).filter(|&point| has_delta[point]);
        for point in touched.collect::<Vec<_>>() {
            interpolate_range(current + 1, point - 1, current, point, origin, out);
            current = point;
        }
        if current == first_delta {
            shift(first, end, current, origin, out);
        } else {
            interpolate_range(current + 1, end, current, first_delta, origin, out);
            if first_delta > first {
                interpolate_range(first, first_delta - 1, current, first_delta, origin, out);
            }
        }
        first = end + 1;
    }
}

/// `tt_delta_shift`: the points of a contour but `reference` moved as it
/// moved.
fn shift(
    first: usize,
    end: usize,
    reference: usize,
    origin: &[(i64, i64)],
    out: &mut [(i64, i64)],
) {
    let delta = (
        out[reference].0 - origin[reference].0,
        out[reference].1 - origin[reference].1,
    );
    if delta == (0, 0) {
        return;
    }
    for point in (first..=end).filter(|&point| point != reference) {
        out[point].0 += delta.0;
        out[point].1 += delta.1;
    }
}

/// `tt_delta_interpolate`: points `p1` to `p2` interpolated between the
/// references, each coordinate on its own.
fn interpolate_range(
    p1: usize,
    p2: usize,
    ref1: usize,
    ref2: usize,
    origin: &[(i64, i64)],
    out: &mut [(i64, i64)],
) {
    if p1 > p2 {
        return;
    }
    let coordinate = |point: (i64, i64), axis: usize| if axis == 0 { point.0 } else { point.1 };
    let (mut ref1, mut ref2) = (ref1, ref2);
    for axis in 0..2 {
        if coordinate(origin[ref1], axis) > coordinate(origin[ref2], axis) {
            std::mem::swap(&mut ref1, &mut ref2);
        }
        let (in1, in2) = (
            coordinate(origin[ref1], axis),
            coordinate(origin[ref2], axis),
        );
        let (out1, out2) = (coordinate(out[ref1], axis), coordinate(out[ref2], axis));
        let (d1, d2) = (out1 - in1, out2 - in2);
        if in1 == in2 && out1 != out2 {
            continue;
        }
        let scale = if in1 == in2 {
            0
        } else {
            div_fix(out2 - out1, in2 - in1)
        };
        for point in p1..=p2 {
            let value = coordinate(origin[point], axis);
            let moved = if value <= in1 {
                value + d1
            } else if value >= in2 {
                value + d2
            } else {
                out1 + mul_fix(value - in1, scale)
            };
            if axis == 0 {
                out[point].0 = moved;
            } else {
                out[point].1 = moved;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_points_and_deltas() {
        let mut r = Reader::new(&[0], GVAR);
        assert_eq!(read_points(&mut r), Some(Points::All));
        // Three points as bytes (1, +2, +3), then one as a word.
        let data = [4, 2, 1, 2, 3, 0x80, 0x01, 0x00];
        let mut r = Reader::new(&data, GVAR);
        assert_eq!(read_points(&mut r), Some(Points::Some(vec![1, 3, 6, 262])));
        assert_eq!(read_points(&mut Reader::new(&[2, 1, 1], GVAR)), None);
        // Two zero deltas, a byte delta and a word delta.
        let data = [0x81, 0x00, 0xFF, 0x40, 0x01, 0x00];
        let deltas = read_deltas(&mut Reader::new(&data, GVAR), 4).unwrap();
        assert_eq!(deltas, [0, 0, -1 << 16, 256 << 16]);
        assert_eq!(read_deltas(&mut Reader::new(&[0x01, 5], GVAR), 2), None);
        assert_eq!(fixed_to_int(-0x18000), -1);
        assert_eq!(fixed_to_f26dot6(0x18000), 96);
    }

    #[test]
    fn tuples_weigh_and_untouched_points_follow() {
        assert_eq!(tuple_scalar(&[0x10000], None, &[0x8000]), 0x8000);
        assert_eq!(tuple_scalar(&[0x10000], None, &[-0x8000]), 0);
        assert_eq!(tuple_scalar(&[0], None, &[0x8000]), 0x10000);
        let intermediate = (vec![0], vec![0x10000]);
        assert_eq!(
            tuple_scalar(&[0x8000], Some(&intermediate), &[0xC000]),
            0x8000
        );
        // A square whose corner 0 moves right by 10 and corner 2 by 20:
        // corner 1, between them in x, is interpolated; corner 3 shifts
        // by the nearer reference.
        let origin: Vec<(i64, i64)> = [(0, 0), (100, 0), (100, 100), (0, 100)]
            .iter()
            .map(|&(x, y)| (x << 16, y << 16))
            .collect();
        let mut out = origin.clone();
        out[0].0 += 10 << 16;
        out[2].0 += 20 << 16;
        interpolate(&[3], &mut out, &origin, &[true, false, true, false]);
        assert_eq!(out[1].0, 120 << 16);
        assert_eq!(out[3].0, 10 << 16);
        // A single delta shifts its contour.
        let mut out = origin.clone();
        out[1].1 += 5 << 16;
        interpolate(&[3], &mut out, &origin, &[false, true, false, false]);
        assert!(out.iter().all(|point| point.1 % (100 << 16) == 5 << 16));
    }
}
