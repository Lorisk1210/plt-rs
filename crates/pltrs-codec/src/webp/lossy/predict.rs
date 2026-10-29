//! VP8's intra predictors (libwebp's `dsp/dec.c`; RFC 6386 section 12),
//! drawn in a work buffer whose rows are [`STRIDE`] bytes apart: each
//! block reads the row above it (including four pixels above-right for
//! 4×4 blocks) and the column to its left, already in the buffer.

/// The distance between rows of the work buffer.
pub(super) const STRIDE: usize = 32;

/// The prediction of a 16×16 luma or 8×8 chroma block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BlockMode {
    /// The average of the top and left neighbours.
    Dc,
    /// True motion: left + top − top-left.
    TrueMotion,
    /// The row above, repeated.
    Vertical,
    /// The column to the left, repeated.
    Horizontal,
}

impl BlockMode {
    /// The mode of a 16×16 or chroma mode number (0 DC, 1 TM, 2 V, 3 H).
    pub(super) fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Dc,
            1 => Self::TrueMotion,
            2 => Self::Vertical,
            _ => Self::Horizontal,
        }
    }
}

/// Which neighbours of a block lie inside the frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct Edges {
    pub(super) top: bool,
    pub(super) left: bool,
}

/// `(a + 2b + c + 2) / 4`.
fn avg3(a: u8, b: u8, c: u8) -> u8 {
    ((u16::from(a) + 2 * u16::from(b) + u16::from(c) + 2) >> 2) as u8
}

/// `(a + b + 1) / 2`.
fn avg2(a: u8, b: u8) -> u8 {
    ((u16::from(a) + u16::from(b) + 1) >> 1) as u8
}

/// Fills the `size`-square block at `at` with `value`.
fn fill(buf: &mut [u8], at: usize, size: usize, value: u8) {
    for row in 0..size {
        buf[at + row * STRIDE..][..size].fill(value);
    }
}

/// Predicts a `size`-square block (16 luma, 8 chroma) at `at`.
pub(super) fn predict_block(buf: &mut [u8], at: usize, size: usize, mode: BlockMode, edges: Edges) {
    let shift = size.trailing_zeros();
    let top_sum = || {
        (0..size)
            .map(|i| u32::from(buf[at - STRIDE + i]))
            .sum::<u32>()
    };
    let left_sum = || {
        (0..size)
            .map(|j| u32::from(buf[at + j * STRIDE - 1]))
            .sum::<u32>()
    };
    match mode {
        BlockMode::Dc => {
            let value = match (edges.top, edges.left) {
                (true, true) => (top_sum() + left_sum() + size as u32) >> (shift + 1),
                (true, false) => (top_sum() + (size as u32 >> 1)) >> shift,
                (false, true) => (left_sum() + (size as u32 >> 1)) >> shift,
                (false, false) => 0x80,
            };
            fill(buf, at, size, value as u8);
        }
        BlockMode::TrueMotion => true_motion(buf, at, size),
        BlockMode::Vertical => {
            for row in 1..=size {
                buf.copy_within(at - STRIDE..at - STRIDE + size, at + (row - 1) * STRIDE);
            }
        }
        BlockMode::Horizontal => {
            for row in 0..size {
                let left = buf[at + row * STRIDE - 1];
                buf[at + row * STRIDE..][..size].fill(left);
            }
        }
    }
}

/// `left + top − top-left`, clamped, for every pixel of the block.
fn true_motion(buf: &mut [u8], at: usize, size: usize) {
    let corner = i32::from(buf[at - STRIDE - 1]);
    for row in 0..size {
        let left = i32::from(buf[at + row * STRIDE - 1]);
        for col in 0..size {
            let top = i32::from(buf[at - STRIDE + col]);
            buf[at + row * STRIDE + col] = (left + top - corner).clamp(0, 255) as u8;
        }
    }
}

/// Predicts the 4×4 luma block at `at` with subblock mode `mode` (0 DC,
/// 1 TM, 2 VE, 3 HE, 4 RD, 5 VR, 6 LD, 7 VL, 8 HD, 9 HU).
pub(super) fn predict_subblock(buf: &mut [u8], at: usize, mode: u8) {
    let top = |i: isize| buf[(at as isize - STRIDE as isize + i) as usize];
    let left = |j: usize| buf[at + j * STRIDE - 1];
    let corner = top(-1);
    let [a, b, c, d, e, f, g, h] = std::array::from_fn(|i| top(i as isize));
    let [i, j, k, l] = std::array::from_fn(left);
    let x = corner;
    let mut out = [[0u8; 4]; 4];
    match mode {
        0 => {
            let sum: u32 = [a, b, c, d, i, j, k, l].iter().map(|&v| u32::from(v)).sum();
            out = [[((sum + 4) >> 3) as u8; 4]; 4];
        }
        1 => {
            true_motion(buf, at, 4);
            return;
        }
        2 => out = [[avg3(x, a, b), avg3(a, b, c), avg3(b, c, d), avg3(c, d, e)]; 4],
        3 => {
            out = [
                [avg3(x, i, j); 4],
                [avg3(i, j, k); 4],
                [avg3(j, k, l); 4],
                [avg3(k, l, l); 4],
            ];
        }
        4 => {
            // Down-right: diagonals from the bottom-left.
            let diagonals = [
                avg3(j, k, l),
                avg3(i, j, k),
                avg3(x, i, j),
                avg3(a, x, i),
                avg3(b, a, x),
                avg3(c, b, a),
                avg3(d, c, b),
            ];
            for (row, line) in out.iter_mut().enumerate() {
                for (col, pixel) in line.iter_mut().enumerate() {
                    *pixel = diagonals[3 - row + col];
                }
            }
        }
        5 => {
            out = [
                [avg2(x, a), avg2(a, b), avg2(b, c), avg2(c, d)],
                [avg3(i, x, a), avg3(x, a, b), avg3(a, b, c), avg3(b, c, d)],
                [avg3(j, i, x), avg2(x, a), avg2(a, b), avg2(b, c)],
                [avg3(k, j, i), avg3(i, x, a), avg3(x, a, b), avg3(a, b, c)],
            ];
        }
        6 => {
            // Down-left: diagonals from the top-left.
            let diagonals = [
                avg3(a, b, c),
                avg3(b, c, d),
                avg3(c, d, e),
                avg3(d, e, f),
                avg3(e, f, g),
                avg3(f, g, h),
                avg3(g, h, h),
            ];
            for (row, line) in out.iter_mut().enumerate() {
                for (col, pixel) in line.iter_mut().enumerate() {
                    *pixel = diagonals[row + col];
                }
            }
        }
        7 => {
            out = [
                [avg2(a, b), avg2(b, c), avg2(c, d), avg2(d, e)],
                [avg3(a, b, c), avg3(b, c, d), avg3(c, d, e), avg3(d, e, f)],
                [avg2(b, c), avg2(c, d), avg2(d, e), avg3(e, f, g)],
                [avg3(b, c, d), avg3(c, d, e), avg3(d, e, f), avg3(f, g, h)],
            ];
        }
        8 => {
            out = [
                [avg2(i, x), avg3(i, x, a), avg3(x, a, b), avg3(a, b, c)],
                [avg2(j, i), avg3(j, i, x), avg2(i, x), avg3(i, x, a)],
                [avg2(k, j), avg3(k, j, i), avg2(j, i), avg3(j, i, x)],
                [avg2(l, k), avg3(l, k, j), avg2(k, j), avg3(k, j, i)],
            ];
        }
        _ => {
            out = [
                [avg2(i, j), avg3(i, j, k), avg2(j, k), avg3(j, k, l)],
                [avg2(j, k), avg3(j, k, l), avg2(k, l), avg3(k, l, l)],
                [avg2(k, l), avg3(k, l, l), l, l],
                [l; 4],
            ];
        }
    }
    for (row, line) in out.iter().enumerate() {
        buf[at + row * STRIDE..][..4].copy_from_slice(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer with the block at (1, 1): top row 10, 20, ..., 80 (with
    /// the corner 5 before it) and left column 1, 2, 3, 4.
    fn buffer() -> (Vec<u8>, usize) {
        let mut buf = vec![0u8; STRIDE * 6];
        let at = STRIDE + 1;
        buf[at - STRIDE - 1] = 5;
        for i in 0..8 {
            buf[at - STRIDE + i] = 10 * (i as u8 + 1);
        }
        for j in 0..4 {
            buf[at + j * STRIDE - 1] = j as u8 + 1;
        }
        (buf, at)
    }

    fn block(buf: &[u8], at: usize) -> [[u8; 4]; 4] {
        std::array::from_fn(|row| buf[at + row * STRIDE..][..4].try_into().unwrap())
    }

    #[test]
    fn subblock_modes_match_libwebp() {
        let (mut buf, at) = buffer();
        predict_subblock(&mut buf, at, 0);
        // (10 + 20 + 30 + 40 + 1 + 2 + 3 + 4 + 4) / 8 = 14.
        assert_eq!(block(&buf, at), [[14; 4]; 4]);
        let (mut buf, at) = buffer();
        predict_subblock(&mut buf, at, 9);
        assert_eq!(
            block(&buf, at)[3],
            [4; 4],
            "horizontal-up ends on the last left pixel"
        );
        let (mut buf, at) = buffer();
        predict_subblock(&mut buf, at, 6);
        let down_left = block(&buf, at);
        assert_eq!(down_left[0][3], down_left[3][0], "down-left diagonals");
        assert_eq!(down_left[3][3], avg3(70, 80, 80));
        let (mut buf, at) = buffer();
        predict_subblock(&mut buf, at, 1);
        // left + top − corner: row 0 is 1 + 10 − 5 = 6, 16, 26, 36.
        assert_eq!(block(&buf, at)[0], [6, 16, 26, 36]);
    }

    #[test]
    fn dc_predictions_use_the_edges_inside_the_frame() {
        let mut buf = vec![0u8; STRIDE * 10];
        let at = STRIDE + 8;
        buf[at - STRIDE..at - STRIDE + 8].fill(200);
        for j in 0..8 {
            buf[at + j * STRIDE - 1] = 100;
        }
        let edges = |top, left| Edges { top, left };
        predict_block(&mut buf, at, 8, BlockMode::Dc, edges(true, true));
        assert_eq!(buf[at], 150);
        predict_block(&mut buf, at, 8, BlockMode::Dc, edges(true, false));
        assert_eq!(buf[at + 7 * STRIDE + 7], 200);
        predict_block(&mut buf, at, 8, BlockMode::Dc, edges(false, false));
        assert_eq!(buf[at], 0x80);
    }
}
