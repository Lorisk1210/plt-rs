//! The transforms the encoder applies before entropy coding: a palette
//! for images of at most 256 colours, otherwise green subtraction and
//! spatial prediction with a mode chosen per tile.

use crate::webp::lossless::transform::{
    BLACK, PREDICTOR_MODES, predict, sub_pixels, subsample, subtract_green,
};

/// The side of the predictor tiles, in bits.
pub(super) const TILE_BITS: u32 = 4;

/// The distinct colours of `pixels` in ascending order, or `None` when
/// there are more than 256.
pub(super) fn palette(pixels: &[u32]) -> Option<Vec<u32>> {
    let mut colors: Vec<u32> = Vec::with_capacity(257);
    for &pixel in pixels {
        if let Err(at) = colors.binary_search(&pixel) {
            if colors.len() == 256 {
                return None;
            }
            colors.insert(at, pixel);
        }
    }
    Some(colors)
}

/// How many palette indices are packed into one pixel, in bits:
/// eight of one bit for two colours, down to one per pixel past 16.
pub(super) fn bundle_bits(colors: usize) -> u32 {
    match colors {
        0..=2 => 3,
        3..=4 => 2,
        5..=16 => 1,
        _ => 0,
    }
}

/// The palette as stored: each colour minus the one before, channel by
/// channel.
pub(super) fn palette_deltas(colors: &[u32]) -> Vec<u32> {
    let mut previous = 0;
    colors
        .iter()
        .map(|&color| {
            let delta = sub_pixels(color, previous);
            previous = color;
            delta
        })
        .collect()
}

/// The rows of `width` pixels as palette indices in the green channel,
/// `1 << bits` indices to a pixel, lowest bits first.
pub(super) fn indexed(pixels: &[u32], width: usize, colors: &[u32], bits: u32) -> Vec<u32> {
    let index_bits = 8 >> bits;
    let per_pixel = 1usize << bits;
    let packed_width = subsample(width, bits);
    let mut out = vec![BLACK; packed_width * (pixels.len() / width)];
    for (row, packed_row) in pixels
        .chunks_exact(width)
        .zip(out.chunks_exact_mut(packed_width))
    {
        for (x, pixel) in row.iter().enumerate() {
            let index = colors
                .binary_search(pixel)
                .expect("every colour is in the palette") as u32;
            packed_row[x / per_pixel] |= index << (8 + (x % per_pixel) as u32 * index_bits);
        }
    }
    out
}

/// Subtracts green from red and blue in every pixel.
pub(super) fn subtract_green_all(pixels: &mut [u32]) {
    for pixel in pixels {
        *pixel = subtract_green(*pixel);
    }
}

/// The prediction residual of the pixel at `index` with mode `mode`; the
/// first row and column use the fixed predictors.
fn residual(pixels: &[u32], index: usize, width: usize, mode: u32) -> u32 {
    let prediction = match (index / width, index % width) {
        (0, 0) => BLACK,
        (0, _) => pixels[index - 1],
        (_, 0) => pixels[index - width],
        _ => predict(mode, pixels, index, width),
    };
    sub_pixels(pixels[index], prediction)
}

/// The sum of the magnitudes of a residual's channels, read as signed
/// bytes: a cheap stand-in for its coded size.
fn magnitude(residual: u32) -> u32 {
    residual
        .to_be_bytes()
        .iter()
        .map(|&c| u32::from((c as i8).unsigned_abs()))
        .sum()
}

/// Chooses each tile's predictor mode by the smallest residuals and
/// returns the mode image and the residuals.
pub(super) fn predict_all(pixels: &[u32], width: usize) -> (Vec<u32>, Vec<u32>) {
    let height = pixels.len() / width;
    let tiles_x = subsample(width, TILE_BITS);
    let tiles_y = subsample(height, TILE_BITS);
    let tile = 1usize << TILE_BITS;
    let mut modes = Vec::with_capacity(tiles_x * tiles_y);
    let mut residuals = vec![0u32; pixels.len()];
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let rows = ty * tile..((ty + 1) * tile).min(height);
            let columns = tx * tile..((tx + 1) * tile).min(width);
            let indices = || {
                rows.clone()
                    .flat_map(|y| columns.clone().map(move |x| y * width + x))
            };
            let mode = (0..PREDICTOR_MODES)
                .min_by_key(|&mode| {
                    indices()
                        .map(|index| magnitude(residual(pixels, index, width, mode)))
                        .sum::<u32>()
                })
                .expect("there are predictor modes");
            for index in indices() {
                residuals[index] = residual(pixels, index, width, mode);
            }
            modes.push(BLACK | (mode << 8));
        }
    }
    (modes, residuals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_stop_past_256_colours() {
        let few: Vec<u32> = [9, 3, 9, 3, 7].to_vec();
        assert_eq!(palette(&few), Some(vec![3, 7, 9]));
        let many: Vec<u32> = (0..257).collect();
        assert_eq!(palette(&many), None);
        assert_eq!(palette(&many[..256]).map(|p| p.len()), Some(256));
    }

    #[test]
    fn indices_pack_low_bits_first() {
        let colors = [10, 20, 30];
        let pixels = [20, 30, 10, 20, 30];
        // Two bits per index, four to a pixel.
        let packed = indexed(&pixels, 5, &colors, 2);
        assert_eq!(packed, [BLACK | (0b01_00_10_01 << 8), BLACK | (0b10 << 8)]);
        assert_eq!(palette_deltas(&[5, 7]), [5, 2]);
    }

    #[test]
    fn flat_images_predict_to_zero() {
        let pixels = vec![0xFF12_3456; 40 * 20];
        let (modes, residuals) = predict_all(&pixels, 40);
        assert_eq!(modes.len(), 3 * 2);
        assert_eq!(residuals[0], 0x0012_3456, "the first pixel predicts black");
        assert!(residuals[1..].iter().all(|&r| r == 0));
    }
}
