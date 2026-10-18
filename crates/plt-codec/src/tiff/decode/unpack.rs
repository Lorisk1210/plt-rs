//! Unpacking rows of stored samples into the bytes of the Pillow mode a
//! layout opens as (Pillow's `Unpack.c`): bits and nibbles spread to
//! bytes, 16-bit colour samples cut to their high byte, unused samples
//! dropped and associated alpha divided out.

use super::ifd::ByteOrder;
use super::layout::Layout;

/// Unpacks the first `out.len() / layout.channels()` pixels of a stored
/// row (`row` holds at least that many) into `out`.
pub(super) fn unpack_row(layout: Layout, order: ByteOrder, row: &[u8], out: &mut [u8]) {
    let width = out.len() / layout.channels();
    match layout {
        Layout::Bilevel { white_is_zero } => {
            for (x, pixel) in out.iter_mut().enumerate() {
                // A set bit is white, unless white is zero.
                let white = (packed(row, x, 1) == 1) ^ white_is_zero;
                *pixel = if white { 255 } else { 0 };
            }
        }
        Layout::Gray {
            bits,
            white_is_zero,
        } => {
            let scale = u8::try_from(255 / ((1_u16 << bits) - 1)).expect("at most 255");
            for (x, pixel) in out.iter_mut().enumerate() {
                let level = packed(row, x, bits) * scale;
                *pixel = if white_is_zero { 255 - level } else { level };
            }
        }
        Layout::Palette { bits } => {
            for (x, pixel) in out.iter_mut().enumerate() {
                *pixel = packed(row, x, bits);
            }
        }
        Layout::Gray16 => {
            let samples = row.as_chunks::<2>().0;
            for (pair, sample) in out.as_chunks_mut::<2>().0.iter_mut().zip(samples) {
                let value = u16::try_from(order.read(sample)).expect("two bytes");
                pair.copy_from_slice(&value.to_le_bytes());
            }
        }
        Layout::GrayAlpha | Layout::PaletteAlpha => out.copy_from_slice(&row[..width * 2]),
        Layout::PaletteExtra => {
            for (pixel, [index, _]) in out.iter_mut().zip(row.as_chunks::<2>().0) {
                *pixel = *index;
            }
        }
        Layout::Rgb { wide, .. } | Layout::Rgba { wide, .. } | Layout::Cmyk { wide, .. } => {
            let channels = layout.channels();
            let stored = layout.bits_per_pixel() / 8;
            for (pixel, sample) in out.chunks_exact_mut(channels).zip(row.chunks_exact(stored)) {
                for (c, value) in pixel.iter_mut().enumerate() {
                    *value = if wide {
                        // The high byte of the 16-bit sample.
                        match order {
                            ByteOrder::Little => sample[2 * c + 1],
                            ByteOrder::Big => sample[2 * c],
                        }
                    } else {
                        sample[c]
                    };
                }
                if matches!(
                    layout,
                    Layout::Rgba {
                        associated: true,
                        ..
                    }
                ) {
                    unpremultiply(pixel);
                }
            }
        }
    }
}

/// The `bits`-bit sample `x` of a row packed most significant bits first.
fn packed(row: &[u8], x: usize, bits: u8) -> u8 {
    let bits = usize::from(bits);
    let at = x * bits;
    let shift = 8 - bits - at % 8;
    (row[at / 8] >> shift) & ((1 << bits) - 1) as u8
}

/// Pillow's `unpackRGBa`: colour divided by alpha (rounding down), fully
/// transparent pixels all zero.
fn unpremultiply(pixel: &mut [u8]) {
    let alpha = pixel[3];
    match alpha {
        0 => pixel.fill(0),
        255 => {}
        _ => {
            for channel in &mut pixel[..3] {
                let straight = u16::from(*channel) * 255 / u16::from(alpha);
                *channel = u8::try_from(straight.min(255)).expect("clamped to a byte");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unpack(layout: Layout, order: ByteOrder, row: &[u8], width: usize) -> Vec<u8> {
        let mut out = vec![0; width * layout.channels()];
        unpack_row(layout, order, row, &mut out);
        out
    }

    #[test]
    fn packed_samples_spread_to_bytes() {
        let little = ByteOrder::Little;
        let bilevel = |white_is_zero| Layout::Bilevel { white_is_zero };
        assert_eq!(
            unpack(bilevel(false), little, &[0b1010_0000], 3),
            [255, 0, 255]
        );
        assert_eq!(
            unpack(bilevel(true), little, &[0b1010_0000], 3),
            [0, 255, 0]
        );
        let gray = |bits, white_is_zero| Layout::Gray {
            bits,
            white_is_zero,
        };
        assert_eq!(
            unpack(gray(2, false), little, &[0b1110_0100], 4),
            [255, 170, 85, 0]
        );
        assert_eq!(unpack(gray(4, true), little, &[0x1F], 2), [238, 0]);
        assert_eq!(unpack(gray(8, true), little, &[3], 1), [252]);
        assert_eq!(
            unpack(Layout::Palette { bits: 4 }, little, &[0xA5], 2),
            [10, 5]
        );
    }

    #[test]
    fn wide_samples_keep_their_high_byte() {
        let rgb = Layout::Rgb {
            wide: true,
            extra: 0,
        };
        let row = [1, 2, 3, 4, 5, 6];
        assert_eq!(unpack(rgb, ByteOrder::Little, &row, 1), [2, 4, 6]);
        assert_eq!(unpack(rgb, ByteOrder::Big, &row, 1), [1, 3, 5]);
        assert_eq!(unpack(Layout::Gray16, ByteOrder::Big, &[1, 2], 1), [2, 1]);
        let rgbx = Layout::Rgb {
            wide: false,
            extra: 2,
        };
        assert_eq!(
            unpack(rgbx, ByteOrder::Big, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 2),
            [1, 2, 3, 6, 7, 8]
        );
    }

    #[test]
    fn associated_alpha_is_divided_out() {
        let rgba = Layout::Rgba {
            wide: false,
            associated: true,
            extra: 0,
        };
        let row = [100, 50, 0, 200, 9, 9, 9, 0, 1, 2, 3, 255, 200, 0, 0, 100];
        assert_eq!(
            unpack(rgba, ByteOrder::Little, &row, 4),
            [127, 63, 0, 200, 0, 0, 0, 0, 1, 2, 3, 255, 255, 0, 0, 100]
        );
    }
}
