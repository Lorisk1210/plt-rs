//! Encoding VP8L bitstreams: a palette or green subtraction and
//! prediction, then the entropy-coded pixels.

mod backward;
mod codes;
mod image;
mod transforms;

use self::image::write_image;
use self::transforms::{
    TILE_BITS, bundle_bits, indexed, palette, palette_deltas, predict_all, subtract_green_all,
};
use super::format::SIGNATURE;
use super::transform::subsample;
use crate::deflate::bit_writer::BitWriter;

/// The transform kinds as the bitstream numbers them.
const PREDICTOR: u32 = 0;
const SUBTRACT_GREEN: u32 = 2;
const COLOR_INDEXING: u32 = 3;

/// Encodes rows of `width` ARGB pixels (each side 1 to 16 384) as a VP8L
/// bitstream; `uses_alpha` sets the header's alpha hint.
pub(crate) fn encode(argb: &[u32], width: usize, uses_alpha: bool) -> Vec<u8> {
    let height = argb.len() / width;
    let mut out = Vec::new();
    let mut writer = BitWriter::new(&mut out);
    writer.write_bits(u32::from(SIGNATURE), 8);
    writer.write_bits(width as u32 - 1, 14);
    writer.write_bits(height as u32 - 1, 14);
    writer.write_bits(u32::from(uses_alpha), 1);
    writer.write_bits(0, 3);
    if let Some(colors) = palette(argb) {
        let bits = bundle_bits(colors.len());
        writer.write_bits(1, 1);
        writer.write_bits(COLOR_INDEXING, 2);
        writer.write_bits(colors.len() as u32 - 1, 8);
        write_image(&mut writer, &palette_deltas(&colors), colors.len(), false);
        writer.write_bits(0, 1);
        let packed = indexed(argb, width, &colors, bits);
        write_image(&mut writer, &packed, subsample(width, bits), true);
    } else {
        let mut pixels = argb.to_vec();
        subtract_green_all(&mut pixels);
        writer.write_bits(1, 1);
        writer.write_bits(SUBTRACT_GREEN, 2);
        let (modes, residuals) = predict_all(&pixels, width);
        writer.write_bits(1, 1);
        writer.write_bits(PREDICTOR, 2);
        writer.write_bits(TILE_BITS - 2, 3);
        write_image(&mut writer, &modes, subsample(width, TILE_BITS), false);
        writer.write_bits(0, 1);
        write_image(&mut writer, &residuals, width, true);
    }
    writer.align_to_byte();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webp::lossless::decode;

    /// Smooth colour ramps with some noise and alpha.
    fn picture(width: usize, height: usize, alpha: bool) -> Vec<u32> {
        let mut state = 0x2545_F491_u32;
        (0..width * height)
            .map(|i| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let (x, y) = ((i % width) as u32, (i / width) as u32);
                let noise = state & 0x0F;
                let a = if alpha {
                    (x * 255 / width as u32) & 0xFF
                } else {
                    255
                };
                (a << 24)
                    | (((x * 7 + noise) & 0xFF) << 16)
                    | (((y * 5) & 0xFF) << 8)
                    | ((x + y) & 0xFF)
            })
            .collect()
    }

    #[test]
    fn photographic_images_round_trip() {
        for (width, height) in [(1, 1), (1, 9), (9, 1), (37, 23), (64, 64), (100, 3)] {
            for alpha in [false, true] {
                let pixels = picture(width, height, alpha);
                let stream = encode(&pixels, width, alpha);
                assert_eq!(
                    decode(&stream).unwrap(),
                    pixels,
                    "{width}x{height} alpha {alpha}"
                );
            }
        }
    }

    #[test]
    fn palette_images_round_trip_at_every_packing() {
        for colors in [1, 2, 3, 5, 16, 17, 200, 256] {
            let pixels: Vec<u32> = (0..31 * 7)
                .map(|i| 0xFF00_0000 | (((i * 7 + i / 31) % colors) as u32 * 0x0001_0203))
                .collect();
            let stream = encode(&pixels, 31, false);
            assert_eq!(decode(&stream).unwrap(), pixels, "{colors} colours");
        }
    }

    #[test]
    fn flat_images_compress_to_almost_nothing() {
        let pixels = vec![0xFFFF_FFFF; 640 * 480];
        let stream = encode(&pixels, 640, false);
        assert!(stream.len() < 100, "{} bytes", stream.len());
        assert_eq!(decode(&stream).unwrap(), pixels);
    }
}
