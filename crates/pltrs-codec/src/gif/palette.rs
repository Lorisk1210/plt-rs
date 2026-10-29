//! Choosing the at most 256 colours of a GIF: every colour when the frames
//! use few enough, otherwise a median cut of their histogram that keeps
//! dominant colours (backgrounds, line colours) exact.

use std::collections::HashMap;

/// Pixels less opaque than this become the transparent index.
const OPAQUE_FROM: u8 = 128;
/// The most colours a GIF table holds.
const MAX_COLORS: usize = 256;

/// An RGB colour packed as `0xRRGGBB`.
type Packed = u32;

fn pack([r, g, b]: [u8; 3]) -> Packed {
    u32::from_be_bytes([0, r, g, b])
}

fn unpack(color: Packed) -> [u8; 3] {
    let [_, r, g, b] = color.to_be_bytes();
    [r, g, b]
}

/// The opaque colour of an RGBA pixel, or `None` when it counts as
/// transparent.
fn opaque([r, g, b, a]: [u8; 4]) -> Option<Packed> {
    (a >= OPAQUE_FROM).then(|| pack([r, g, b]))
}

/// A colour table and the index of every colour of the frames it was made
/// for.
#[derive(Debug)]
pub(super) struct Palette {
    colors: Vec<[u8; 3]>,
    transparent: Option<u8>,
    lookup: HashMap<Packed, u8>,
}

impl Palette {
    /// The palette of RGBA `frames`, with a transparent entry (the last)
    /// when some pixel is less than half opaque.
    pub(super) fn for_frames(frames: &[&[u8]]) -> Self {
        let mut histogram: HashMap<Packed, u64> = HashMap::new();
        let mut transparent = false;
        for frame in frames {
            for (color, run) in runs(frame) {
                match color {
                    Some(color) => *histogram.entry(color).or_default() += run,
                    None => transparent = true,
                }
            }
        }
        let mut entries: Vec<(Packed, u64)> = histogram.into_iter().collect();
        entries.sort_unstable();
        let capacity = MAX_COLORS - usize::from(transparent);
        let mut colors: Vec<[u8; 3]> = if entries.len() <= capacity {
            entries.iter().map(|&(color, _)| unpack(color)).collect()
        } else {
            median_cut(&mut entries, capacity)
        };
        let lookup = entries
            .iter()
            .map(|&(color, _)| (color, nearest(&colors, unpack(color))))
            .collect();
        let transparent = transparent.then(|| {
            colors.push([0, 0, 0]);
            index_of(colors.len() - 1)
        });
        Self {
            colors,
            transparent,
            lookup,
        }
    }

    /// The colours, transparent entry included.
    pub(super) fn colors(&self) -> &[[u8; 3]] {
        &self.colors
    }

    /// The transparent index, if any.
    pub(super) fn transparent(&self) -> Option<u8> {
        self.transparent
    }

    /// The palette index of every pixel of an RGBA frame.
    pub(super) fn indices(&self, frame: &[u8]) -> Vec<u8> {
        let mut indices = Vec::with_capacity(frame.len() / 4);
        for (color, run) in runs(frame) {
            let index = match color {
                Some(color) => self
                    .lookup
                    .get(&color)
                    .copied()
                    .unwrap_or_else(|| nearest(&self.colors, unpack(color))),
                None => self.transparent.unwrap_or(0),
            };
            let run = usize::try_from(run).expect("a run is no longer than its frame");
            indices.extend(std::iter::repeat_n(index, run));
        }
        indices
    }
}

/// The colours of an RGBA frame as runs of equal colour (plots are mostly
/// flat areas, so this saves most of the hashing).
fn runs(frame: &[u8]) -> impl Iterator<Item = (Option<Packed>, u64)> + '_ {
    let mut pixels = frame
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(opaque)
        .peekable();
    std::iter::from_fn(move || {
        let color = pixels.next()?;
        let mut run = 1;
        while pixels.next_if_eq(&color).is_some() {
            run += 1;
        }
        Some((color, run))
    })
}

/// `index` as a palette index (palettes hold at most 256 colours).
fn index_of(index: usize) -> u8 {
    u8::try_from(index).expect("palettes hold at most 256 colours")
}

/// The index of the colour of `colors` closest to `color`.
fn nearest(colors: &[[u8; 3]], color: [u8; 3]) -> u8 {
    let distance = |other: &[u8; 3]| -> u32 {
        other
            .iter()
            .zip(color)
            .map(|(&a, b)| u32::from(a.abs_diff(b)).pow(2))
            .sum()
    };
    let best = colors
        .iter()
        .enumerate()
        .min_by_key(|(_, other)| distance(other))
        .map_or(0, |(index, _)| index);
    index_of(best)
}

/// A box of the colour cube: a range of the sorted histogram.
#[derive(Debug)]
struct ColorBox {
    start: usize,
    end: usize,
    /// The channel with the widest spread, and that spread.
    channel: usize,
    spread: u8,
    population: u64,
}

impl ColorBox {
    fn new(entries: &[(Packed, u64)], start: usize, end: usize) -> Self {
        let slice = &entries[start..end];
        let (channel, spread) = (0..3)
            .map(|channel| {
                let values = slice.iter().map(|&(color, _)| unpack(color)[channel]);
                let low = values.clone().min().unwrap_or(0);
                let high = values.max().unwrap_or(0);
                (channel, high - low)
            })
            .max_by_key(|&(channel, spread)| (spread, std::cmp::Reverse(channel)))
            .unwrap_or((0, 0));
        Self {
            start,
            end,
            channel,
            spread,
            population: slice.iter().map(|&(_, count)| count).sum(),
        }
    }

    /// How much splitting the box would help: its pixels times its spread.
    fn score(&self) -> u64 {
        self.population * u64::from(self.spread)
    }
}

/// Up to `count` colours for the histogram `entries` by median cut: the
/// box with the most pixels times spread is halved at its weighted median
/// along its widest channel until there are `count` boxes.
fn median_cut(entries: &mut [(Packed, u64)], count: usize) -> Vec<[u8; 3]> {
    let mut boxes = vec![ColorBox::new(entries, 0, entries.len())];
    while boxes.len() < count {
        let Some((at, _)) = boxes
            .iter()
            .enumerate()
            .filter(|(_, colors)| colors.spread > 0)
            .max_by_key(|(at, colors)| (colors.score(), std::cmp::Reverse(*at)))
        else {
            break;
        };
        let split = boxes.swap_remove(at);
        let slice = &mut entries[split.start..split.end];
        slice.sort_unstable_by_key(|&(color, _)| (unpack(color)[split.channel], color));
        let mut seen = 0;
        let half = slice
            .iter()
            .position(|&(_, n)| {
                seen += n;
                seen * 2 >= split.population
            })
            .map_or(1, |median| median + 1)
            .clamp(1, slice.len() - 1);
        boxes.push(ColorBox::new(entries, split.start, split.start + half));
        boxes.push(ColorBox::new(entries, split.start + half, split.end));
    }
    boxes
        .iter()
        .map(|colors| representative(&entries[colors.start..colors.end], colors.population))
        .collect()
}

/// The colour standing for a box: its most common colour when that covers
/// half the box, otherwise the mean weighted by pixel counts.
fn representative(entries: &[(Packed, u64)], population: u64) -> [u8; 3] {
    let dominant = entries.iter().max_by_key(|&&(color, n)| (n, color));
    if let Some(&(color, n)) = dominant
        && n * 2 >= population
    {
        return unpack(color);
    }
    let mut sums = [0_u64; 3];
    for &(color, n) in entries {
        for (sum, channel) in sums.iter_mut().zip(unpack(color)) {
            *sum += u64::from(channel) * n;
        }
    }
    sums.map(|sum| {
        let mean = (sum + population / 2) / population.max(1);
        u8::try_from(mean).expect("a mean of bytes is a byte")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(colors: &[[u8; 4]]) -> Vec<u8> {
        colors.iter().flatten().copied().collect()
    }

    #[test]
    fn few_colours_are_kept_exactly() {
        let frame = rgba(&[
            [255, 255, 255, 255],
            [31, 119, 180, 255],
            [255, 255, 255, 255],
        ]);
        let palette = Palette::for_frames(&[&frame]);
        assert_eq!(palette.colors(), [[31, 119, 180], [255, 255, 255]]);
        assert_eq!(palette.transparent(), None);
        assert_eq!(palette.indices(&frame), [1, 0, 1]);
    }

    #[test]
    fn clear_pixels_share_the_transparent_index() {
        let frame = rgba(&[[9, 9, 9, 0], [200, 0, 0, 255], [1, 2, 3, 127]]);
        let palette = Palette::for_frames(&[&frame]);
        assert_eq!(palette.colors(), [[200, 0, 0], [0, 0, 0]]);
        assert_eq!(palette.transparent(), Some(1));
        assert_eq!(palette.indices(&frame), [1, 0, 1]);
    }

    #[test]
    fn many_colours_are_cut_to_a_full_table() {
        // A white background, a pure line colour and a 4096-colour ramp.
        let mut pixels = vec![[255, 255, 255, 255]; 50_000];
        pixels.extend(vec![[214, 39, 40, 255]; 5_000]);
        pixels.extend((0..4096_u32).map(|i| {
            let [_, _, g, b] = (i * 16).to_be_bytes();
            [u8::try_from(i / 16).unwrap(), g, b, 255]
        }));
        let frame = rgba(&pixels);
        let palette = Palette::for_frames(&[&frame]);
        assert_eq!(palette.colors().len(), MAX_COLORS);
        assert!(palette.colors().contains(&[255, 255, 255]));
        assert!(palette.colors().contains(&[214, 39, 40]));
        let indices = palette.indices(&frame);
        assert_eq!(indices.len(), pixels.len());
        let error: u64 = pixels
            .iter()
            .zip(&indices)
            .map(|(pixel, &index)| {
                let chosen = palette.colors()[usize::from(index)];
                (0..3)
                    .map(|c| u64::from(pixel[c].abs_diff(chosen[c])))
                    .sum::<u64>()
            })
            .sum();
        assert!(error / (pixels.len() as u64) < 4, "mean error {error}");
    }

    #[test]
    fn empty_frames_make_an_empty_palette() {
        let palette = Palette::for_frames(&[&[]]);
        assert_eq!(palette.colors(), Vec::<[u8; 3]>::new());
        assert_eq!(palette.indices(&[]), Vec::<u8>::new());
    }
}
