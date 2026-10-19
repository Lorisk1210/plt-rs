//! The alpha plane of lossy images (`ALPH` chunks; libwebp's
//! `alpha_dec.c` and `filters.c`): raw or VP8L-compressed levels,
//! unfiltered row by row.

use super::DecodeError;
use super::lossless;

/// How the levels were predicted before coding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    None,
    Horizontal,
    Vertical,
    Gradient,
}

/// Decodes the payload of an `ALPH` chunk of a `width × height` image.
pub(super) fn decode(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, DecodeError> {
    let (&header, payload) = data.split_first().ok_or(DecodeError::InvalidAlpha)?;
    let compression = header & 3;
    let filter = match (header >> 2) & 3 {
        0 => Filter::None,
        1 => Filter::Horizontal,
        2 => Filter::Vertical,
        _ => Filter::Gradient,
    };
    let preprocessing = (header >> 4) & 3;
    let reserved = header >> 6;
    if payload.is_empty() || compression > 1 || preprocessing > 1 || reserved != 0 {
        return Err(DecodeError::InvalidAlpha);
    }
    let size = width as usize * height as usize;
    let mut levels = if compression == 0 {
        payload
            .get(..size)
            .ok_or(DecodeError::InvalidAlpha)?
            .to_vec()
    } else {
        lossless::decode_stream(payload, width, height)?
            .iter()
            .map(|argb| (argb >> 8) as u8)
            .collect()
    };
    unfilter(filter, &mut levels, width as usize);
    Ok(levels)
}

/// Undoes the prediction of the rows of `levels`, in place.
fn unfilter(filter: Filter, levels: &mut [u8], width: usize) {
    for y in 0..levels.len() / width {
        let (done, rest) = levels.split_at_mut(y * width);
        let previous = y.checked_sub(1).map(|above| &done[above * width..]);
        let row = &mut rest[..width];
        match (filter, previous) {
            (Filter::None, _) => {}
            (Filter::Horizontal, previous) | (_, previous @ None) => {
                let mut prediction = previous.map_or(0, |above| above[0]);
                for level in row {
                    *level = level.wrapping_add(prediction);
                    prediction = *level;
                }
            }
            (Filter::Vertical, Some(above)) => {
                for (level, &up) in row.iter_mut().zip(above) {
                    *level = level.wrapping_add(up);
                }
            }
            (Filter::Gradient, Some(above)) => {
                let (mut left, mut top_left) = (above[0], above[0]);
                for (level, &top) in row.iter_mut().zip(above) {
                    let gradient =
                        (i32::from(left) + i32::from(top) - i32::from(top_left)).clamp(0, 255);
                    left = level.wrapping_add(gradient as u8);
                    top_left = top;
                    *level = left;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_levels_are_unfiltered_row_by_row() {
        // Horizontal filter: the first row starts from 0, later rows from
        // the pixel above their first.
        let data = [0b0000_0100, 10, 1, 1, 5, 0, 1];
        assert_eq!(decode(&data, 3, 2), Ok(vec![10, 11, 12, 15, 15, 16]));
        // Vertical: the first row is horizontal, then each adds the above.
        let data = [0b0000_1000, 10, 1, 1, 5, 0, 1];
        assert_eq!(decode(&data, 3, 2), Ok(vec![10, 11, 12, 15, 11, 13]));
    }

    #[test]
    fn gradients_predict_left_plus_top_minus_corner() {
        let mut levels = vec![10, 20, 30, 5, 0, 0];
        unfilter(Filter::Gradient, &mut levels, 3);
        // Row 0 accumulates to 10, 30, 60. Row 1: 5 + 10 = 15, then
        // 15 + 30 − 10 = 35 and 35 + 60 − 30 = 65.
        assert_eq!(levels, [10, 30, 60, 15, 35, 65]);
    }

    #[test]
    fn bad_headers_are_rejected() {
        assert_eq!(decode(&[], 1, 1), Err(DecodeError::InvalidAlpha));
        assert_eq!(
            decode(&[0], 1, 1),
            Err(DecodeError::InvalidAlpha),
            "no levels"
        );
        assert_eq!(
            decode(&[2, 0], 1, 1),
            Err(DecodeError::InvalidAlpha),
            "compression 2"
        );
        assert_eq!(
            decode(&[0x40, 0], 1, 1),
            Err(DecodeError::InvalidAlpha),
            "reserved bits"
        );
        assert_eq!(
            decode(&[0, 1], 2, 1),
            Err(DecodeError::InvalidAlpha),
            "too few levels"
        );
    }
}
