//! Palette mode (screen content coding): a block predicted from a few
//! colours and the colour index of each sample, as flat-coloured plots,
//! lines and text suit. This module holds the palettes and codes their
//! colours (`palette_mode_info`, with the cache of the neighbours'
//! colours); `choose` picks palettes for a block and `tokens` codes the
//! colour index maps.

mod choose;
mod tokens;

pub(crate) use choose::{Area, chroma_candidates, luma_candidates};
pub(crate) use tokens::write_map;

use super::cdf::PaletteCdfs;
use super::entropy::SymbolSink;

/// The most colours of a palette (`PALETTE_COLORS`).
pub(crate) const MAX_COLORS: usize = 8;

/// The colours of one plane's palette as the neighbours remember them
/// (`PaletteSizes` and `PaletteColors`), for the cache of later blocks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Colors {
    len: u8,
    values: [u8; MAX_COLORS],
}

impl Colors {
    /// The colours `values`, at most eight.
    pub(crate) fn new(values: &[u8]) -> Self {
        let mut colors = Self {
            len: values.len() as u8,
            ..Self::default()
        };
        colors.values[..values.len()].copy_from_slice(values);
        colors
    }

    /// The colours, ascending.
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.values[..usize::from(self.len)]
    }

    /// Whether there is a palette at all.
    pub(crate) fn is_some(&self) -> bool {
        self.len > 0
    }
}

/// A palette of a block and the colour index of each of its samples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Palette {
    /// The colours: Y for a luma palette; U, ascending, then the V paired
    /// with each for a chroma palette.
    pub(crate) colors: Vec<Vec<u8>>,
    /// The colour index of each sample of the block, row by row, `side`
    /// to a row; past the frame's edge, the indices of the last samples
    /// on screen repeat as a decoder repeats them.
    pub(crate) map: Vec<u8>,
    /// The block's side in the palette's plane.
    pub(crate) side: usize,
    /// The columns and rows of the block on screen (`onscreenWidth`,
    /// `onscreenHeight`).
    pub(crate) onscreen: (usize, usize),
}

impl Palette {
    /// The number of colours (`PaletteSizeY` or `PaletteSizeUV`).
    pub(crate) fn len(&self) -> usize {
        self.colors[0].len()
    }

    /// The prediction of the `size × size` transform block at `(x, y)`
    /// in the block, from colour list `list` (0 for Y or U, 1 for V).
    pub(crate) fn predict(&self, list: usize, (x, y): (usize, usize), size: usize) -> Vec<u8> {
        let colors = &self.colors[list];
        (y..y + size)
            .flat_map(|row| &self.map[row * self.side + x..][..size])
            .map(|&index| colors[usize::from(index)])
            .collect()
    }
}

/// `CeilLog2`: the bits of values below `x`, 0 for `x < 2`.
fn ceil_log2(x: u32) -> u32 {
    if x < 2 { 0 } else { (x - 1).ilog2() + 1 }
}

/// The cache of a block's palette: the colours of the palettes above
/// (within the superblock row) and to the left, merged in ascending order
/// without repeats (`get_palette_cache`).
pub(crate) fn cache(above: &[u8], left: &[u8]) -> Vec<u8> {
    let mut merged: Vec<u8> = above.iter().chain(left).copied().collect();
    merged.sort_unstable();
    merged.dedup();
    merged
}

/// Writes the colours of one ascending list: which of `cache` it uses,
/// then the others as a first colour and increments, `offset` being 1
/// for luma (whose colours differ) and 0 for U (`palette_colors_y` and
/// `palette_colors_u` of `palette_mode_info`).
fn write_ascending<S: SymbolSink>(sink: &mut S, colors: &[u8], cache: &[u8], offset: u32) {
    let mut used = 0;
    let mut rest: Vec<u8> = colors.to_vec();
    for &entry in cache {
        if used == colors.len() {
            break;
        }
        let hit = rest.iter().position(|&c| c == entry);
        sink.literal(u32::from(hit.is_some()), 1);
        if let Some(at) = hit {
            rest.remove(at);
            used += 1;
        }
    }
    let Some((&first, others)) = rest.split_first() else {
        return;
    };
    sink.literal(u32::from(first), 8);
    if others.is_empty() {
        return;
    }
    let deltas: Vec<u32> = rest
        .windows(2)
        .map(|pair| u32::from(pair[1] - pair[0]) - offset)
        .collect();
    let needed = deltas.iter().map(|&d| d.bit_width()).max().unwrap_or(0);
    let extra = needed.saturating_sub(5).min(3);
    sink.literal(extra, 2);
    let mut bits = 5 + extra;
    for (&delta, &color) in deltas.iter().zip(&rest[1..]) {
        sink.literal(delta, bits);
        let range = 256 - u32::from(color) - offset;
        bits = bits.min(ceil_log2(range));
    }
}

/// The V deltas of a chroma palette as `palette_delta_v` codes them, each
/// taken the short way round 256; `None` when one needs more than 7 bits.
fn v_deltas(v: &[u8]) -> Option<Vec<i32>> {
    v.windows(2)
        .map(|pair| {
            let d = i32::from(pair[1]) - i32::from(pair[0]);
            let d = if d > 128 {
                d - 256
            } else if d < -128 {
                d + 256
            } else {
                d
            };
            (d.abs() < 128).then_some(d)
        })
        .collect()
}

/// Writes the V colours of a chroma palette: delta coded when that is
/// shorter, else as they are.
fn write_v<S: SymbolSink>(sink: &mut S, v: &[u8]) {
    let raw_bits = 8 * v.len() as u32;
    let coded = v_deltas(v).map(|deltas| {
        let bits = deltas
            .iter()
            .map(|d| d.unsigned_abs().bit_width())
            .max()
            .unwrap_or(0)
            .max(4);
        let signs = deltas.iter().filter(|&&d| d != 0).count() as u32;
        let cost = 2 + 8 + bits * deltas.len() as u32 + signs;
        (deltas, bits, cost)
    });
    match coded {
        Some((deltas, bits, cost)) if cost < raw_bits => {
            sink.literal(1, 1);
            sink.literal(bits - 4, 2);
            sink.literal(u32::from(v[0]), 8);
            for d in deltas {
                sink.literal(d.unsigned_abs(), bits);
                if d != 0 {
                    sink.literal(u32::from(d < 0), 1);
                }
            }
        }
        _ => {
            sink.literal(0, 1);
            for &value in v {
                sink.literal(u32::from(value), 8);
            }
        }
    }
}

/// What the palette syntax of a block depends on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PaletteContext<'a> {
    /// `bsizeCtx`: the block's size class.
    pub(crate) size_ctx: usize,
    /// The colours of the palettes above and to the left, for the cache.
    pub(crate) neighbours: (&'a [u8], &'a [u8]),
}

/// Writes the luma part of `palette_mode_info` for a block predicted in
/// DC mode: whether it has a palette (`ctx` counting the neighbours that
/// do), then its size and colours.
pub(crate) fn write_luma<S: SymbolSink>(
    sink: &mut S,
    cdfs: &mut PaletteCdfs,
    (context, ctx): (&PaletteContext<'_>, usize),
    palette: Option<&Palette>,
) {
    sink.symbol(
        &mut cdfs.y_mode[context.size_ctx][ctx],
        usize::from(palette.is_some()),
    );
    if let Some(palette) = palette {
        sink.symbol(&mut cdfs.y_size[context.size_ctx], palette.len() - 2);
        let (above, left) = context.neighbours;
        write_ascending(sink, &palette.colors[0], &cache(above, left), 1);
    }
}

/// Writes the chroma part of `palette_mode_info` for a block whose chroma
/// is predicted in DC mode.
pub(crate) fn write_chroma<S: SymbolSink>(
    sink: &mut S,
    cdfs: &mut PaletteCdfs,
    (context, has_luma): (&PaletteContext<'_>, bool),
    palette: Option<&Palette>,
) {
    sink.symbol(
        &mut cdfs.uv_mode[usize::from(has_luma)],
        usize::from(palette.is_some()),
    );
    if let Some(palette) = palette {
        sink.symbol(&mut cdfs.uv_size[context.size_ctx], palette.len() - 2);
        let (above, left) = context.neighbours;
        write_ascending(sink, &palette.colors[0], &cache(above, left), 0);
        write_v(sink, &palette.colors[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::av1::entropy::RateEstimate;

    /// Records literals, to check what is written.
    #[derive(Default)]
    struct Bits(Vec<(u32, u32)>);

    impl SymbolSink for Bits {
        fn symbol<const N: usize>(&mut self, _cdf: &mut [u16; N], symbol: usize) {
            self.0.push((symbol as u32, 0));
        }

        fn bool_with(&mut self, bit: bool, _p0: u16) {
            self.0.push((u32::from(bit), 1));
        }

        fn literal(&mut self, value: u32, bits: u32) {
            self.0.push((value, bits));
        }
    }

    #[test]
    fn caches_merge_without_repeats() {
        assert_eq!(cache(&[3, 9, 200], &[1, 9, 250]), [1, 3, 9, 200, 250]);
        assert_eq!(cache(&[], &[]), []);
        assert_eq!(ceil_log2(1), 0);
        assert_eq!(ceil_log2(5), 3);
        assert_eq!(ceil_log2(8), 3);
    }

    #[test]
    fn luma_colours_use_the_cache_then_increments() {
        let mut bits = Bits::default();
        // Of the cache, 9 and 200 are used, 1 and 250 not; 20 and 40
        // follow as a colour and an increment.
        write_ascending(&mut bits, &[9, 20, 40, 200], &[1, 9, 200, 250], 1);
        assert_eq!(
            bits.0,
            [
                (0, 1),
                (1, 1),
                (1, 1),
                (0, 1),
                (20, 8),
                // 40 - 20 - 1 = 19 needs 5 bits: no extra.
                (0, 2),
                (19, 5),
            ]
        );
    }

    #[test]
    fn v_colours_take_the_short_way_round() {
        assert_eq!(v_deltas(&[250, 4, 0]), Some(vec![10, -4]));
        assert_eq!(v_deltas(&[0, 128]), None);
        let mut delta = Bits::default();
        write_v(&mut delta, &[100, 104, 99, 101]);
        assert_eq!(delta.0[..4], [(1, 1), (0, 2), (100, 8), (4, 4)]);
        let mut raw = Bits::default();
        write_v(&mut raw, &[0, 128]);
        assert_eq!(raw.0, [(0, 1), (0, 8), (128, 8)]);
        let mut estimate = RateEstimate::default();
        write_v(&mut estimate, &[7, 7]);
        assert!(estimate.bits <= 17.0);
    }

    #[test]
    fn predictions_read_the_map() {
        let palette = Palette {
            colors: vec![vec![10, 200], vec![5, 6]],
            map: vec![0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0],
            side: 4,
            onscreen: (4, 4),
        };
        assert_eq!(palette.len(), 2);
        assert_eq!(palette.predict(0, (2, 2), 2), [200, 200, 10, 10]);
        assert_eq!(palette.predict(1, (0, 0), 2), [5, 6, 6, 6]);
        assert_eq!(Colors::new(&[3, 4]).as_slice(), [3, 4]);
        assert!(!Colors::default().is_some());
    }
}
