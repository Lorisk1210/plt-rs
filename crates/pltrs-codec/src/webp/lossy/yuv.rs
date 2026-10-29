//! Converting decoded Y, U and V planes to RGB as libwebp does by default
//! (`dsp/upsampling.c`, `dsp/yuv.h`, `EmitFancyRGB`): chroma upsampled by
//! interpolating the four nearest samples ("fancy" upsampling), then the
//! BT.601 conversion in 14-bit fixed point.

use super::filter::Plane;
use super::frame::Frame;

/// `v · coeff / 256` (an emulated `_mm_mulhi_epu16`).
fn mult_hi(v: i32, coeff: i32) -> i32 {
    (v * coeff) >> 8
}

/// A fixed-point value with six fractional bits, rounded down to a byte.
fn clip8(v: i32) -> u8 {
    if v & !((256 << 6) - 1) == 0 {
        (v >> 6) as u8
    } else if v < 0 {
        0
    } else {
        255
    }
}

/// The RGB colour of a Y, U, V sample.
fn yuv_to_rgb(y: u8, u: u8, v: u8) -> [u8; 3] {
    let (y, u, v) = (i32::from(y), i32::from(u), i32::from(v));
    let luma = mult_hi(y, 19077);
    [
        clip8(luma + mult_hi(v, 26149) - 14234),
        clip8(luma - mult_hi(u, 6419) - mult_hi(v, 13320) + 8708),
        clip8(luma + mult_hi(u, 33050) - 17685),
    ]
}

/// U in the low half and V in the high half of a word, so that both are
/// interpolated at once.
fn pack(u: u8, v: u8) -> u32 {
    u32::from(u) | (u32::from(v) << 16)
}

/// The U and V of a packed pair.
fn unpack(uv: u32) -> (u8, u8) {
    ((uv & 0xFF) as u8, (uv >> 16) as u8)
}

/// Writes the RGB of `y` with the packed chroma `uv` to `out`.
fn put(out: &mut [u8; 3], y: u8, uv: u32) {
    let (u, v) = unpack(uv);
    *out = yuv_to_rgb(y, u, v);
}

/// The chroma rows (U and V) above and below a pair of luma rows.
#[derive(Clone, Copy, Debug)]
struct ChromaRows<'a> {
    top: (&'a [u8], &'a [u8]),
    current: (&'a [u8], &'a [u8]),
}

/// A luma row and the pixels it converts to.
#[derive(Debug)]
struct OutputRow<'a> {
    luma: &'a [u8],
    out: &'a mut [[u8; 3]],
}

/// `(3a + b + 2) / 4` for both halves of packed chroma.
fn three_quarters(a: u32, b: u32) -> u32 {
    (3 * a + b + 0x0002_0002) >> 2
}

/// Converts a pair of luma rows (`top`, and `bottom` if any) whose chroma
/// lies between two chroma rows (libwebp's `UPSAMPLE_FUNC`).
fn upsample_pair(
    chroma: ChromaRows<'_>,
    top: &mut OutputRow<'_>,
    mut bottom: Option<OutputRow<'_>>,
) {
    let len = top.luma.len();
    let ((top_u, top_v), (cur_u, cur_v)) = (chroma.top, chroma.current);
    let mut top_left = pack(top_u[0], top_v[0]);
    let mut left = pack(cur_u[0], cur_v[0]);
    put(&mut top.out[0], top.luma[0], three_quarters(top_left, left));
    if let Some(row) = &mut bottom {
        put(&mut row.out[0], row.luma[0], three_quarters(left, top_left));
    }
    for x in 1..=(len - 1) >> 1 {
        let t = pack(top_u[x], top_v[x]);
        let cur = pack(cur_u[x], cur_v[x]);
        let average = top_left + t + left + cur + 0x0008_0008;
        let diag_12 = (average + 2 * (t + left)) >> 3;
        let diag_03 = (average + 2 * (top_left + cur)) >> 3;
        put(
            &mut top.out[2 * x - 1],
            top.luma[2 * x - 1],
            u32::midpoint(diag_12, top_left),
        );
        put(
            &mut top.out[2 * x],
            top.luma[2 * x],
            u32::midpoint(diag_03, t),
        );
        if let Some(row) = &mut bottom {
            put(
                &mut row.out[2 * x - 1],
                row.luma[2 * x - 1],
                u32::midpoint(diag_03, left),
            );
            put(
                &mut row.out[2 * x],
                row.luma[2 * x],
                u32::midpoint(diag_12, cur),
            );
        }
        top_left = t;
        left = cur;
    }
    if len.is_multiple_of(2) {
        put(
            &mut top.out[len - 1],
            top.luma[len - 1],
            three_quarters(top_left, left),
        );
        if let Some(row) = bottom {
            put(
                &mut row.out[len - 1],
                row.luma[len - 1],
                three_quarters(left, top_left),
            );
        }
    }
}

/// The first `width` samples of row `row` of `plane`.
fn row(plane: &Plane, row: usize, width: usize) -> &[u8] {
    &plane.data[row * plane.stride..][..width]
}

/// The RGB pixels of a frame, row by row.
pub(super) fn to_rgb(frame: &Frame) -> Vec<[u8; 3]> {
    let (width, height) = (frame.width, frame.height);
    let [y, u, v] = &frame.planes;
    let half = width.div_ceil(2);
    let chroma = |k: usize| (row(u, k, half), row(v, k, half));
    let alone = |k: usize| ChromaRows {
        top: chroma(k),
        current: chroma(k),
    };
    let mut out = vec![[0u8; 3]; width * height];
    let (first, rest) = out.split_at_mut(width);
    upsample_pair(
        alone(0),
        &mut OutputRow {
            luma: row(y, 0, width),
            out: first,
        },
        None,
    );
    // Rows 2k − 1 and 2k lie between chroma rows k − 1 and k.
    for (pair, k) in rest.chunks_exact_mut(2 * width).zip(1..) {
        let (top_out, bottom_out) = pair.split_at_mut(width);
        let rows = ChromaRows {
            top: chroma(k - 1),
            current: chroma(k),
        };
        let mut top = OutputRow {
            luma: row(y, 2 * k - 1, width),
            out: top_out,
        };
        let bottom = OutputRow {
            luma: row(y, 2 * k, width),
            out: bottom_out,
        };
        upsample_pair(rows, &mut top, Some(bottom));
    }
    if height.is_multiple_of(2) {
        let mut last = OutputRow {
            luma: row(y, height - 1, width),
            out: &mut out[(height - 1) * width..],
        };
        upsample_pair(alone(height / 2 - 1), &mut last, None);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_matches_libwebp_fixed_point() {
        assert_eq!(yuv_to_rgb(16, 128, 128), [0, 0, 0]);
        assert_eq!(yuv_to_rgb(235, 128, 128), [255, 255, 255]);
        // Red reaches 16316 / 64 = 254.9: libwebp rounds down.
        assert_eq!(yuv_to_rgb(81, 90, 240), [254, 0, 0]);
        assert_eq!(clip8(-1), 0);
        assert_eq!(clip8(256 << 6), 255);
        assert_eq!(clip8(100 << 6 | 63), 100);
    }

    #[test]
    fn upsampling_interpolates_between_chroma_samples() {
        // One row of luma 128 over chroma U 0, 100 (V 128): the middle
        // pixels take three quarters of their nearer sample.
        let top = [128u8; 4];
        let u = [0u8, 100];
        let v = [128u8, 128];
        let mut out = [[0u8; 3]; 4];
        let chroma = ChromaRows {
            top: (&u, &v),
            current: (&u, &v),
        };
        upsample_pair(
            chroma,
            &mut OutputRow {
                luma: &top,
                out: &mut out,
            },
            None,
        );
        let expected = [0u8, 25, 75, 100].map(|u| yuv_to_rgb(128, u, 128));
        assert_eq!(out, expected);
    }
}
