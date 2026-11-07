//! `cvar`: the control value deltas of TrueType hinting at an instance
//! (`tt_face_vary_cvt`). Its tuples refer to the shared tuples of `gvar`,
//! as FreeType reads them.

use super::gvar::{
    Gvar, Points, SHARE_POINT_NUMBERS, TUPLE_COUNT_MASK, TupleHeader, fixed_to_f26dot6,
    read_deltas, read_points,
};
use crate::error::{FontError, Result};
use crate::fixed::mul_fix;
use crate::reader::Reader;
use crate::types::Tag;

const CVAR: Tag = Tag::new(b"cvar");

/// The control values `cvt` (font units) at the normalized coordinates
/// `coords`: each summed delta added in 26.6 and the sum truncated to
/// whole units again, as FreeType scales them. A `cvar` table of an
/// unknown version leaves them alone.
///
/// # Errors
///
/// When the table's header or a tuple's coordinates are malformed.
pub(crate) fn vary_cvt(
    cvar: &[u8],
    gvar: Option<&Gvar<'_>>,
    coords: &[i64],
    cvt: &mut [i64],
) -> Result<()> {
    let invalid = || FontError::malformed(CVAR, "invalid control value variations");
    let mut r = Reader::new(cvar, CVAR);
    if r.u32()? != 0x0001_0000 {
        return Ok(());
    }
    let tuple_field = r.u16()?;
    let mut data_at = usize::from(r.u16()?);
    let tuple_count = usize::from(tuple_field & TUPLE_COUNT_MASK);
    if data_at + tuple_count * 4 > cvar.len() {
        return Err(invalid());
    }
    let shared_points = if tuple_field & SHARE_POINT_NUMBERS != 0 {
        let mut points = Reader::new(&cvar[data_at..], CVAR);
        let read = read_points(&mut points);
        data_at += points.pos();
        read
    } else {
        None
    };
    let mut sums = vec![0i64; cvt.len()];
    let shared = gvar.map_or(&[][..], |gvar| &gvar.shared[..]);
    for _ in 0..tuple_count {
        let tuple = TupleHeader::read(&mut r, shared, coords, CVAR)?;
        let tuple_data = cvar.get(data_at..).unwrap_or_default();
        data_at += tuple.size;
        if tuple.apply == 0 {
            continue;
        }
        let apply = tuple.apply;
        let mut reader = Reader::new(tuple_data, CVAR);
        let points = if tuple.private_points {
            read_points(&mut reader)
        } else {
            shared_points.clone()
        };
        let wanted = match &points {
            Some(Points::Some(list)) => list.len(),
            _ => cvt.len(),
        };
        let (Some(points), Some(deltas)) = (points, read_deltas(&mut reader, wanted)) else {
            continue;
        };
        match points {
            Points::All => {
                for (sum, &delta) in sums.iter_mut().zip(&deltas) {
                    *sum += mul_fix(delta, apply);
                }
            }
            Points::Some(list) => {
                for (&index, &delta) in list.iter().zip(&deltas) {
                    if let Some(sum) = sums.get_mut(usize::from(index)) {
                        *sum += mul_fix(delta, apply);
                    }
                }
            }
        }
    }
    for (value, &sum) in cvt.iter_mut().zip(&sums) {
        *value = (*value * 64 + fixed_to_f26dot6(sum)) / 64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_values_vary() {
        // One tuple with an embedded peak at 1 and its own points (all of
        // them): deltas 10, -3.
        let mut data = vec![0, 1, 0, 0, 0, 1, 0, 14];
        data.extend([0, 4, 0xA0, 0, 0x40, 0]);
        data.extend([0, 0x01, 10, 0xFD]);
        let mut cvt = vec![100, 100];
        vary_cvt(&data, None, &[0x8000], &mut cvt).unwrap();
        assert_eq!(cvt, [105, 98]);
        let mut cvt = vec![100, 100];
        vary_cvt(&data, None, &[0], &mut cvt).unwrap();
        assert_eq!(cvt, [100, 100]);
        assert!(vary_cvt(&data[..9], None, &[0x8000], &mut cvt).is_err());
        let mut other = data.clone();
        other[1] = 2;
        assert!(vary_cvt(&other, None, &[0x8000], &mut cvt).is_ok());
    }
}
