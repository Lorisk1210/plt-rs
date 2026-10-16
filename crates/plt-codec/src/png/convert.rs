//! Conversion of decoded pixels in any PNG format to RGBA.

use super::decoded::ColorKey;
use super::types::{BitDepth, ColorType, PixelFormat};

/// Palette entry used for indices beyond the end of the palette.
const MISSING_ENTRY: [u8; 4] = [0, 0, 0, 255];

/// Borrowed pixels plus everything needed to interpret them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pixels<'a> {
    pub(crate) width: usize,
    pub(crate) format: PixelFormat,
    pub(crate) row_bytes: usize,
    pub(crate) data: &'a [u8],
    pub(crate) palette: &'a [[u8; 4]],
    pub(crate) color_key: Option<ColorKey>,
}

impl Pixels<'_> {
    pub(crate) fn to_rgba8(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixel_count() * 4);
        if self.format.depth != BitDepth::Eight {
            self.for_each_rgba16(|pixel| out.extend(pixel.map(to_8_bits)));
            return out;
        }
        // 8-bit fast paths: rows are unpadded, so the data is one run of
        // whole pixels.
        match self.format.color {
            ColorType::Rgba => out.extend_from_slice(self.data),
            ColorType::Rgb => {
                let key = match self.color_key {
                    Some(ColorKey::Rgb(rgb)) => Some(rgb),
                    _ => None,
                };
                let (pixels, _) = self.data.as_chunks::<3>();
                for &[r, g, b] in pixels {
                    let alpha = opaque_unless(key == Some([r, g, b].map(u16::from)));
                    out.extend_from_slice(&[r, g, b, alpha]);
                }
            }
            ColorType::GrayAlpha => {
                let (pixels, _) = self.data.as_chunks::<2>();
                for &[gray, alpha] in pixels {
                    out.extend_from_slice(&[gray, gray, gray, alpha]);
                }
            }
            ColorType::Gray => {
                let key = match self.color_key {
                    Some(ColorKey::Gray(gray)) => Some(gray),
                    _ => None,
                };
                for &gray in self.data {
                    let alpha = opaque_unless(key == Some(u16::from(gray)));
                    out.extend_from_slice(&[gray, gray, gray, alpha]);
                }
            }
            ColorType::Indexed => {
                for &index in self.data {
                    out.extend_from_slice(&self.palette_entry(u16::from(index)));
                }
            }
        }
        out
    }

    pub(crate) fn to_rgba16(self) -> Vec<u16> {
        let mut out = Vec::with_capacity(self.pixel_count() * 4);
        self.for_each_rgba16(|pixel| out.extend_from_slice(&pixel));
        out
    }

    fn pixel_count(self) -> usize {
        self.data.len() / self.row_bytes * self.width
    }

    /// Calls `emit` with every pixel as 16-bit RGBA, row by row.
    fn for_each_rgba16(self, mut emit: impl FnMut([u16; 4])) {
        let depth = self.format.depth;
        let scale = u16::MAX / depth.max_value();
        let channels = self.format.color.channels();
        for row in self.data.chunks_exact(self.row_bytes) {
            for x in 0..self.width {
                let sample = |channel: usize| read_sample(row, x * channels + channel, depth);
                let pixel = match self.format.color {
                    ColorType::Gray => {
                        let gray = sample(0);
                        let alpha = opaque16_unless(self.color_key == Some(ColorKey::Gray(gray)));
                        let gray = gray * scale;
                        [gray, gray, gray, alpha]
                    }
                    ColorType::GrayAlpha => {
                        let gray = sample(0) * scale;
                        [gray, gray, gray, sample(1) * scale]
                    }
                    ColorType::Rgb => {
                        let rgb = [sample(0), sample(1), sample(2)];
                        let alpha = opaque16_unless(self.color_key == Some(ColorKey::Rgb(rgb)));
                        let [r, g, b] = rgb.map(|value| value * scale);
                        [r, g, b, alpha]
                    }
                    ColorType::Rgba => {
                        [sample(0), sample(1), sample(2), sample(3)].map(|v| v * scale)
                    }
                    ColorType::Indexed => self
                        .palette_entry(sample(0))
                        .map(|value| u16::from(value) * 257),
                };
                emit(pixel);
            }
        }
    }

    fn palette_entry(self, index: u16) -> [u8; 4] {
        self.palette
            .get(usize::from(index))
            .copied()
            .unwrap_or(MISSING_ENTRY)
    }
}

/// Sample number `index` of `row` at `depth`.
fn read_sample(row: &[u8], index: usize, depth: BitDepth) -> u16 {
    match depth {
        BitDepth::Sixteen => u16::from_be_bytes([row[2 * index], row[2 * index + 1]]),
        BitDepth::Eight => u16::from(row[index]),
        BitDepth::One | BitDepth::Two | BitDepth::Four => {
            let bits = usize::from(depth.bits());
            let offset = index * bits;
            let shift = 8 - bits - offset % 8;
            u16::from(row[offset / 8] >> shift) & depth.max_value()
        }
    }
}

/// Rounds a 16-bit sample to the nearest 8-bit one (exact for values that
/// came from 8 bits or fewer).
fn to_8_bits(value: u16) -> u8 {
    ((u32::from(value) * 255 + 32_895) >> 16) as u8
}

/// Alpha 0 for a pixel matching the colour key, opaque otherwise.
fn opaque_unless(transparent: bool) -> u8 {
    if transparent { 0 } else { u8::MAX }
}

/// [`opaque_unless`] at 16 bits.
fn opaque16_unless(transparent: bool) -> u16 {
    u16::from(opaque_unless(transparent)) * 257
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_rounding_is_exact_and_nearest() {
        for value in 0..=255u8 {
            assert_eq!(to_8_bits(u16::from(value) * 257), value);
        }
        assert_eq!(to_8_bits(128), 0);
        assert_eq!(to_8_bits(129), 1);
        // 254 and 255 are 65278 and 65535: the midpoint is 65406.5.
        assert_eq!(to_8_bits(65_406), 254);
        assert_eq!(to_8_bits(65_407), 255);
    }

    #[test]
    fn reads_packed_samples() {
        let row = [0b1011_0010, 0x12, 0x34, 0x56];
        assert_eq!(read_sample(&row, 0, BitDepth::One), 1);
        assert_eq!(read_sample(&row, 1, BitDepth::One), 0);
        assert_eq!(read_sample(&row, 3, BitDepth::Two), 0b10);
        assert_eq!(read_sample(&row, 1, BitDepth::Four), 0b0010);
        assert_eq!(read_sample(&row, 4, BitDepth::Four), 0x3);
        assert_eq!(read_sample(&row, 1, BitDepth::Eight), 0x12);
        assert_eq!(read_sample(&row, 1, BitDepth::Sixteen), 0x3456);
    }
}
