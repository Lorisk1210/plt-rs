//! Pixel formats: colour types, bit depths and the row geometry they imply.

use std::fmt;

/// How a PNG pixel is made up of samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// One grey sample.
    Gray,
    /// A grey sample followed by an alpha sample.
    GrayAlpha,
    /// Red, green and blue samples.
    Rgb,
    /// Red, green, blue and alpha samples.
    Rgba,
    /// One index into the palette.
    Indexed,
}

impl ColorType {
    /// Number of samples per pixel.
    pub const fn channels(self) -> usize {
        match self {
            Self::Gray | Self::Indexed => 1,
            Self::GrayAlpha => 2,
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }

    /// Whether pixels carry their own alpha sample.
    pub const fn has_alpha(self) -> bool {
        matches!(self, Self::GrayAlpha | Self::Rgba)
    }

    /// The `IHDR` colour type code.
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Gray => 0,
            Self::Rgb => 2,
            Self::Indexed => 3,
            Self::GrayAlpha => 4,
            Self::Rgba => 6,
        }
    }

    /// The colour type with `IHDR` code `code`.
    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Gray),
            2 => Some(Self::Rgb),
            3 => Some(Self::Indexed),
            4 => Some(Self::GrayAlpha),
            6 => Some(Self::Rgba),
            _ => None,
        }
    }
}

impl fmt::Display for ColorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Gray => "grey",
            Self::GrayAlpha => "grey + alpha",
            Self::Rgb => "RGB",
            Self::Rgba => "RGBA",
            Self::Indexed => "indexed",
        })
    }
}

/// Bits per sample (per palette index for indexed images).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitDepth {
    /// 1 bit per sample.
    One,
    /// 2 bits per sample.
    Two,
    /// 4 bits per sample.
    Four,
    /// 8 bits per sample.
    Eight,
    /// 16 bits per sample, stored big-endian.
    Sixteen,
}

impl BitDepth {
    /// Number of bits.
    pub const fn bits(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
            Self::Eight => 8,
            Self::Sixteen => 16,
        }
    }

    /// The depth with `bits` bits per sample.
    pub const fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            4 => Some(Self::Four),
            8 => Some(Self::Eight),
            16 => Some(Self::Sixteen),
            _ => None,
        }
    }

    /// The largest sample value, `2^bits - 1`.
    pub const fn max_value(self) -> u16 {
        match self {
            Self::Sixteen => u16::MAX,
            _ => (1 << self.bits()) - 1,
        }
    }
}

impl fmt::Display for BitDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-bit", self.bits())
    }
}

/// A colour type with a bit depth the PNG specification allows for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixelFormat {
    pub(crate) color: ColorType,
    pub(crate) depth: BitDepth,
}

impl PixelFormat {
    /// The format, if PNG allows `depth` for `color` (PNG spec §11.2.2).
    pub(crate) fn new(color: ColorType, depth: BitDepth) -> Option<Self> {
        let allowed = match color {
            ColorType::Gray => true,
            ColorType::Indexed => depth != BitDepth::Sixteen,
            ColorType::GrayAlpha | ColorType::Rgb | ColorType::Rgba => depth >= BitDepth::Eight,
        };
        allowed.then_some(Self { color, depth })
    }

    pub(crate) fn bits_per_pixel(self) -> usize {
        self.color.channels() * usize::from(self.depth.bits())
    }

    /// Byte distance to the corresponding byte of the previous pixel, as used
    /// by the row filters (at least 1).
    pub(crate) fn filter_stride(self) -> usize {
        self.bits_per_pixel().div_ceil(8)
    }

    /// Bytes per row of `width` pixels (rows are padded to whole bytes), or
    /// `None` on overflow.
    pub(crate) fn row_bytes(self, width: u32) -> Option<usize> {
        usize::try_from(width)
            .ok()?
            .checked_mul(self.bits_per_pixel())
            .map(|bits| bits.div_ceil(8))
    }

    /// Bytes for `width` × `height` pixels, or `None` on overflow.
    pub(crate) fn image_bytes(self, width: u32, height: u32) -> Option<usize> {
        self.row_bytes(width)?
            .checked_mul(usize::try_from(height).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: [ColorType; 5] = [
        ColorType::Gray,
        ColorType::GrayAlpha,
        ColorType::Rgb,
        ColorType::Rgba,
        ColorType::Indexed,
    ];
    const DEPTHS: [BitDepth; 5] = [
        BitDepth::One,
        BitDepth::Two,
        BitDepth::Four,
        BitDepth::Eight,
        BitDepth::Sixteen,
    ];

    #[test]
    fn exactly_the_fifteen_standard_formats_are_allowed() {
        let allowed: Vec<(u8, u8)> = COLORS
            .iter()
            .flat_map(|&c| DEPTHS.iter().map(move |&d| (c, d)))
            .filter_map(|(c, d)| PixelFormat::new(c, d))
            .map(|f| (f.color.code(), f.depth.bits()))
            .collect();
        assert_eq!(allowed.len(), 15);
        for (code, bits) in [
            (0, 1),
            (0, 16),
            (2, 8),
            (2, 16),
            (3, 1),
            (3, 8),
            (4, 16),
            (6, 8),
        ] {
            assert!(allowed.contains(&(code, bits)), "{code}/{bits}");
        }
        assert!(!allowed.contains(&(3, 16)));
        assert!(!allowed.contains(&(2, 4)));
    }

    #[test]
    fn codes_and_bits_round_trip() {
        for color in COLORS {
            assert_eq!(ColorType::from_code(color.code()), Some(color));
        }
        assert_eq!(ColorType::from_code(1), None);
        for depth in DEPTHS {
            assert_eq!(BitDepth::from_bits(depth.bits()), Some(depth));
        }
        assert_eq!(BitDepth::from_bits(3), None);
        assert_eq!(BitDepth::Four.max_value(), 15);
        assert_eq!(BitDepth::Sixteen.max_value(), 65_535);
    }

    #[test]
    fn row_geometry() {
        let gray1 = PixelFormat::new(ColorType::Gray, BitDepth::One).unwrap();
        assert_eq!(gray1.row_bytes(9), Some(2));
        assert_eq!(gray1.filter_stride(), 1);
        let rgba16 = PixelFormat::new(ColorType::Rgba, BitDepth::Sixteen).unwrap();
        assert_eq!(rgba16.row_bytes(3), Some(24));
        assert_eq!(rgba16.filter_stride(), 8);
        let rgb8 = PixelFormat::new(ColorType::Rgb, BitDepth::Eight).unwrap();
        assert_eq!(rgb8.image_bytes(4, 5), Some(60));
        assert_eq!(rgb8.filter_stride(), 3);
    }
}
