//! The four VP8L image transforms, undone after decoding (libwebp's
//! `lossless.c`): spatial prediction, cross-colour decorrelation, green
//! subtraction and colour indexing.
//!
//! Pixels are ARGB words: alpha in the top byte, then red, green, blue.

/// Opaque black, the prediction of the first pixel.
const BLACK: u32 = 0xFF00_0000;

/// A transform read from the bitstream, with what undoing it needs.
#[derive(Clone, Debug)]
pub(super) enum Transform {
    /// Each pixel was predicted from its decoded neighbours by the mode of
    /// its tile; the residuals were coded.
    Predictor(TileImage),
    /// Red and blue were decorrelated from green (and blue from red) by the
    /// multipliers of each tile.
    CrossColor(TileImage),
    /// Green was subtracted from red and blue.
    SubtractGreen,
    /// Pixels are indices into a palette, several packed per coded pixel.
    ColorIndexing(Palette),
}

/// Per-tile parameters: one ARGB word per square tile.
#[derive(Clone, Debug)]
pub(super) struct TileImage {
    /// The width of the image the transform applies to.
    pub(super) width: usize,
    /// Tiles are `1 << bits` pixels square.
    pub(super) bits: u32,
    /// The tiles, row by row.
    pub(super) tiles: Vec<u32>,
}

impl TileImage {
    /// The tile parameter of pixel `(x, y)`.
    fn at(&self, x: usize, y: usize) -> u32 {
        let per_row = subsample(self.width, self.bits);
        self.tiles[(y >> self.bits) * per_row + (x >> self.bits)]
    }
}

/// A colour-indexing transform.
#[derive(Clone, Debug)]
pub(super) struct Palette {
    /// The width of the image before packing.
    pub(super) width: usize,
    /// `1 << bits` indices are packed per coded pixel.
    pub(super) bits: u32,
    /// The colours; indices past them are transparent black.
    pub(super) colors: Vec<u32>,
}

impl Palette {
    /// The palette whose colours are coded as differences from the colour
    /// before (each channel modulo 256), padded with transparent black to
    /// every index a packed pixel can hold.
    pub(super) fn from_deltas(width: usize, bits: u32, deltas: &[u32]) -> Self {
        let mut colors = Vec::with_capacity(1 << (8 >> bits));
        let mut previous = 0;
        for &delta in deltas {
            previous = add_pixels(previous, delta);
            colors.push(previous);
        }
        colors.resize(1 << (8 >> bits), 0);
        Self {
            width,
            bits,
            colors,
        }
    }
}

/// `ceil(size / 2^bits)`: how many tiles (or packed pixels) cover `size`.
pub(super) fn subsample(size: usize, bits: u32) -> usize {
    (size + (1 << bits) - 1) >> bits
}

impl Transform {
    /// Undoes the transform on the `width`-pixel rows of `pixels` (at least
    /// one), returning the pixels and their width (colour indexing widens
    /// the rows).
    pub(super) fn invert(&self, mut pixels: Vec<u32>, width: usize) -> (Vec<u32>, usize) {
        match self {
            Self::Predictor(tiles) => {
                unpredict(tiles, &mut pixels);
                (pixels, width)
            }
            Self::CrossColor(tiles) => {
                recorrelate(tiles, &mut pixels);
                (pixels, width)
            }
            Self::SubtractGreen => {
                for pixel in &mut pixels {
                    *pixel = add_green(*pixel);
                }
                (pixels, width)
            }
            Self::ColorIndexing(palette) => (unpack(palette, &pixels, width), palette.width),
        }
    }
}

/// Adds two pixels channel by channel, modulo 256.
fn add_pixels(a: u32, b: u32) -> u32 {
    let alpha_green = (a & 0xFF00_FF00).wrapping_add(b & 0xFF00_FF00);
    let red_blue = (a & 0x00FF_00FF).wrapping_add(b & 0x00FF_00FF);
    (alpha_green & 0xFF00_FF00) | (red_blue & 0x00FF_00FF)
}

/// The channel average of two pixels, rounded down.
fn average2(a: u32, b: u32) -> u32 {
    (((a ^ b) & 0xFEFE_FEFE) >> 1) + (a & b)
}

/// The channels of a pixel, alpha first.
fn channels(pixel: u32) -> [i32; 4] {
    pixel.to_be_bytes().map(i32::from)
}

/// A pixel from channels already in 0–255, alpha first.
fn from_channels(channels: [i32; 4]) -> u32 {
    u32::from_be_bytes(channels.map(|c| c.clamp(0, 255) as u8))
}

/// Whichever of `top` and `left` is closer to the gradient estimate
/// `top + left - top_left` (libwebp's `Select`).
fn select(top: u32, left: u32, top_left: u32) -> u32 {
    let [t, l, tl] = [top, left, top_left].map(channels);
    let distance: i32 = (0..4)
        .map(|i| (l[i] - tl[i]).abs() - (t[i] - tl[i]).abs())
        .sum();
    if distance <= 0 { top } else { left }
}

/// `a + b - c` per channel, clamped.
fn clamped_add_subtract_full(a: u32, b: u32, c: u32) -> u32 {
    let [a, b, c] = [a, b, c].map(channels);
    from_channels(std::array::from_fn(|i| a[i] + b[i] - c[i]))
}

/// `avg + (avg - c) / 2` per channel with `avg` the average of `a` and
/// `b`, clamped.
fn clamped_add_subtract_half(a: u32, b: u32, c: u32) -> u32 {
    let [average, c] = [average2(a, b), c].map(channels);
    from_channels(std::array::from_fn(|i| {
        average[i] + (average[i] - c[i]) / 2
    }))
}

/// The prediction of mode `mode` for the pixel at `index` of rows of
/// `width`, from its decoded neighbours. The top-right neighbour of a
/// row's last pixel is the first pixel of its own row, as in libwebp.
fn predict(mode: u32, pixels: &[u32], index: usize, width: usize) -> u32 {
    let left = pixels[index - 1];
    let top = pixels[index - width];
    let top_left = pixels[index - width - 1];
    let top_right = pixels[index - width + 1];
    match mode {
        1 => left,
        2 => top,
        3 => top_right,
        4 => top_left,
        5 => average2(average2(left, top_right), top),
        6 => average2(left, top_left),
        7 => average2(left, top),
        8 => average2(top_left, top),
        9 => average2(top, top_right),
        10 => average2(average2(left, top_left), average2(top, top_right)),
        11 => select(top, left, top_left),
        12 => clamped_add_subtract_full(left, top, top_left),
        13 => clamped_add_subtract_half(left, top, top_left),
        // Mode 0, and the unused 14 and 15.
        _ => BLACK,
    }
}

/// Adds back the predictions: the first pixel predicts black, the rest of
/// the first row its left neighbour, the first column its top neighbour,
/// and every other pixel the mode of its tile. Images have at least one
/// pixel.
fn unpredict(tiles: &TileImage, pixels: &mut [u32]) {
    let width = tiles.width;
    pixels[0] = add_pixels(pixels[0], BLACK);
    for x in 1..width {
        pixels[x] = add_pixels(pixels[x], pixels[x - 1]);
    }
    for y in 1..pixels.len() / width {
        let row = y * width;
        pixels[row] = add_pixels(pixels[row], pixels[row - width]);
        for x in 1..width {
            let mode = (tiles.at(x, y) >> 8) & 0xF;
            let index = row + x;
            pixels[index] = add_pixels(pixels[index], predict(mode, pixels, index, width));
        }
    }
}

/// The colour transform's delta: `multiplier · channel / 32`, both signed.
fn color_delta(multiplier: u8, channel: u8) -> u8 {
    ((i32::from(multiplier as i8) * i32::from(channel as i8)) >> 5) as u8
}

/// Restores red and blue from green and red with each tile's multipliers
/// (green-to-red in the low byte, green-to-blue next, red-to-blue above).
fn recorrelate(tiles: &TileImage, pixels: &mut [u32]) {
    let width = tiles.width;
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let [_, red_to_blue, green_to_blue, green_to_red] =
            tiles.at(index % width, index / width).to_be_bytes();
        let [alpha, red, green, blue] = pixel.to_be_bytes();
        let red = red.wrapping_add(color_delta(green_to_red, green));
        let blue = blue
            .wrapping_add(color_delta(green_to_blue, green))
            .wrapping_add(color_delta(red_to_blue, red));
        *pixel = u32::from_be_bytes([alpha, red, green, blue]);
    }
}

/// Adds green back to red and blue.
fn add_green(pixel: u32) -> u32 {
    let green = (pixel >> 8) & 0xFF;
    let red_blue = ((pixel & 0x00FF_00FF) + ((green << 16) | green)) & 0x00FF_00FF;
    (pixel & 0xFF00_FF00) | red_blue
}

/// Looks the palette indices of the green channel up, unpacking `1 <<
/// palette.bits` indices from each coded pixel (lowest bits first).
fn unpack(palette: &Palette, coded: &[u32], coded_width: usize) -> Vec<u32> {
    let width = palette.width;
    let index_bits = 8 >> palette.bits;
    let mask = (1u32 << index_bits) - 1;
    let per_coded = 1usize << palette.bits;
    let mut pixels = Vec::with_capacity(width * (coded.len() / coded_width));
    for row in coded.chunks_exact(coded_width) {
        for x in 0..width {
            let packed = (row[x / per_coded] >> 8) & 0xFF;
            let index = (packed >> ((x % per_coded) as u32 * index_bits)) & mask;
            pixels.push(palette.colors[index as usize]);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
        u32::from_be_bytes([a, r, g, b])
    }

    #[test]
    fn channel_arithmetic_wraps_and_rounds_down() {
        assert_eq!(
            add_pixels(argb(200, 1, 2, 255), argb(100, 3, 4, 1)),
            argb(44, 4, 6, 0)
        );
        assert_eq!(
            average2(argb(0, 1, 255, 10), argb(255, 2, 254, 11)),
            argb(127, 1, 254, 10)
        );
        assert_eq!(add_green(argb(9, 250, 10, 1)), argb(9, 4, 10, 11));
    }

    #[test]
    fn predictors_follow_libwebp() {
        // 3×2: the predicted pixel is (1, 1); its row holds 0x50 first.
        let top_left = argb(255, 10, 20, 30);
        let top = argb(255, 40, 50, 60);
        let top_right = argb(255, 70, 80, 90);
        let left = argb(255, 100, 110, 120);
        let pixels = [top_left, top, top_right, left, 0, 0];
        assert_eq!(predict(0, &pixels, 4, 3), BLACK);
        assert_eq!(predict(3, &pixels, 4, 3), top_right);
        assert_eq!(
            predict(11, &pixels, 4, 3),
            left,
            "left is nearer the gradient"
        );
        assert_eq!(
            predict(12, &pixels, 4, 3),
            argb(255, 130, 140, 150),
            "left + top - top-left"
        );
        assert_eq!(predict(13, &pixels, 4, 3), argb(255, 100, 110, 120));
        assert_eq!(
            predict(15, &pixels, 4, 3),
            BLACK,
            "unused modes predict black"
        );
        // The last pixel of a row takes its row's first pixel as top-right.
        let row_end = [1, 2, 3, 4, 5, 6];
        assert_eq!(predict(3, &row_end, 5, 3), 4);
    }

    #[test]
    fn half_steps_truncate_towards_zero() {
        // avg(0, 0) = 0, c = 5: 0 + (0 - 5) / 2 = -2 (C division), clamped.
        let pixel = clamped_add_subtract_half(0, 0, argb(5, 5, 5, 5));
        assert_eq!(pixel, 0);
        let pixel = clamped_add_subtract_half(argb(0, 9, 0, 0), argb(0, 9, 0, 0), argb(0, 4, 0, 0));
        assert_eq!(pixel, argb(0, 11, 0, 0), "9 + 5 / 2");
    }

    #[test]
    fn colour_transforms_use_signed_multipliers() {
        // Green 0x80 (−128) times −32 / 32 adds 128 to red.
        let tiles = TileImage {
            width: 1,
            bits: 2,
            tiles: vec![u32::from_be_bytes([0, 0, 0, 0xE0])],
        };
        let mut pixels = vec![argb(255, 0, 0x80, 0)];
        recorrelate(&tiles, &mut pixels);
        assert_eq!(pixels, [argb(255, 128, 0x80, 0)]);
    }

    #[test]
    fn palettes_unpack_low_bits_first_and_pad_with_transparency() {
        let palette = Palette::from_deltas(5, 2, &[argb(255, 1, 1, 1), argb(0, 1, 1, 1)]);
        assert_eq!(palette.colors.len(), 4);
        assert_eq!(palette.colors[1], argb(255, 2, 2, 2));
        assert_eq!(palette.colors[2], 0);
        // Indices 1, 0, 2, 1 in the first coded pixel, 1 in the second.
        let coded = [0b01_10_00_01 << 8, 1 << 8];
        let pixels = unpack(&palette, &coded, 2);
        let [one, zero, two] = [1, 0, 2].map(|i| palette.colors[i]);
        assert_eq!(pixels, [one, zero, two, one, one]);
    }

    #[test]
    fn prediction_round_trips_a_ramp() {
        // Residuals of mode 1 (left) everywhere decode a horizontal ramp.
        let tiles = TileImage {
            width: 4,
            bits: 2,
            tiles: vec![1 << 8],
        };
        let mut pixels = vec![argb(0, 0, 0, 1); 8];
        unpredict(&tiles, &mut pixels);
        let blue: Vec<u8> = pixels.iter().map(|p| p.to_be_bytes()[3]).collect();
        assert_eq!(blue, [1, 2, 3, 4, 2, 3, 4, 5]);
    }
}
