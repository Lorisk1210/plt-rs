//! The colour index map of a palette block (`palette_tokens`): the first
//! index as `NS(n)`, then the others along anti-diagonals, each coded as
//! its rank among the colours of its decoded neighbours.

use super::{MAX_COLORS, Palette};
use crate::av1::cdf::ColorCdfs;
use crate::av1::entropy::SymbolSink;

/// `Palette_Color_Context`: the context of each neighbour hash that can
/// occur.
const HASH_CONTEXT: [usize; 9] = [0, 0, 0, 0, 0, 4, 3, 2, 1];
/// `Palette_Color_Hash_Multipliers`.
const HASH_MULTIPLIERS: [u32; 3] = [1, 2, 2];

/// Writes `value` below `n` as `NS(n)`.
fn write_ns<S: SymbolSink>(sink: &mut S, value: u32, n: u32) {
    let w = n.ilog2() + 1;
    let m = (1 << w) - n;
    if value < m {
        sink.literal(value, w - 1);
    } else {
        let v = value + m;
        sink.literal(v >> 1, w - 1);
        sink.literal(v & 1, 1);
    }
}

/// The colours ranked by how the decoded neighbours of `(row, col)` vote
/// for them (left and above 2, above left 1), and the context of the
/// votes (`get_palette_color_context`).
fn color_context(
    map: &[u8],
    side: usize,
    (row, col): (usize, usize),
    n: usize,
) -> ([u8; MAX_COLORS], usize) {
    let mut scores = [0u32; MAX_COLORS];
    let mut order: [u8; MAX_COLORS] = [0, 1, 2, 3, 4, 5, 6, 7];
    let at = |r: usize, c: usize| usize::from(map[r * side + c]);
    if col > 0 {
        scores[at(row, col - 1)] += 2;
    }
    if row > 0 && col > 0 {
        scores[at(row - 1, col - 1)] += 1;
    }
    if row > 0 {
        scores[at(row - 1, col)] += 2;
    }
    for i in 0..HASH_MULTIPLIERS.len() {
        let mut best = i;
        for j in i + 1..n {
            if scores[j] > scores[best] {
                best = j;
            }
        }
        if best != i {
            let (score, color) = (scores[best], order[best]);
            for k in (i + 1..=best).rev() {
                scores[k] = scores[k - 1];
                order[k] = order[k - 1];
            }
            scores[i] = score;
            order[i] = color;
        }
    }
    let hash: u32 = scores
        .iter()
        .zip(HASH_MULTIPLIERS)
        .map(|(s, m)| s * m)
        .sum();
    (order, HASH_CONTEXT[hash as usize])
}

/// Writes the colour index map of `palette`.
pub(crate) fn write_map<S: SymbolSink>(sink: &mut S, cdfs: &mut ColorCdfs, palette: &Palette) {
    let n = palette.len();
    let side = palette.side;
    let (width, height) = palette.onscreen;
    write_ns(sink, u32::from(palette.map[0]), n as u32);
    for i in 1..width + height - 1 {
        for j in (i.saturating_sub(height - 1)..=i.min(width - 1)).rev() {
            let (row, col) = (i - j, j);
            let (order, ctx) = color_context(&palette.map, side, (row, col), n);
            let index = palette.map[row * side + col];
            let rank = order
                .iter()
                .position(|&color| color == index)
                .expect("every index is ranked");
            cdfs.code(sink, n, ctx, rank);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records literals.
    #[derive(Default)]
    struct Literals(Vec<(u32, u32)>);

    impl SymbolSink for Literals {
        fn symbol<const N: usize>(&mut self, _cdf: &mut [u16; N], _symbol: usize) {}

        fn bool_with(&mut self, _bit: bool, _p0: u16) {}

        fn literal(&mut self, value: u32, bits: u32) {
            self.0.push((value, bits));
        }
    }

    #[test]
    fn non_symmetric_codes() {
        // n = 5: 0 to 2 in two bits, 3 and 4 as 110 and 111.
        let codes: Vec<Vec<(u32, u32)>> = (0..5)
            .map(|v| {
                let mut out = Literals::default();
                write_ns(&mut out, v, 5);
                out.0
            })
            .collect();
        assert_eq!(codes[2], [(2, 2)]);
        assert_eq!(codes[3], [(3, 2), (0, 1)]);
        assert_eq!(codes[4], [(3, 2), (1, 1)]);
        let mut out = Literals::default();
        write_ns(&mut out, 1, 2);
        assert_eq!(out.0, [(1, 1)]);
    }

    #[test]
    fn neighbours_rank_the_colours() {
        // At (1, 1) the left neighbour votes 2 for colour 2, the one
        // above 2 for colour 1 and the one above left 1 for colour 0:
        // ties keep the lower colour first.
        let map = [0, 1, 1, 2, 0, 0, 0, 0, 0];
        let (order, ctx) = color_context(&map, 3, (1, 1), 3);
        assert_eq!(&order[..3], [1, 2, 0]);
        // Scores 2, 2, 1: hash 2 + 4 + 2 = 8.
        assert_eq!(ctx, 1);
        let (order, ctx) = color_context(&map, 3, (0, 1), 3);
        assert_eq!((&order[..3], ctx), (&[0, 1, 2][..], 0));
    }
}
