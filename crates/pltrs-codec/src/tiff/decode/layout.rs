//! The pixel layouts Pillow reads, from the tags that describe them
//! (`OPEN_INFO` in `TiffImagePlugin`), and how each is unpacked into the
//! Pillow mode it opens as (`Unpack.c`).

use super::ifd::ByteOrder;

/// Photometric interpretations.
const WHITE_IS_ZERO: u64 = 0;
const BLACK_IS_ZERO: u64 = 1;
const RGB: u64 = 2;
const PALETTE: u64 = 3;
const SEPARATED: u64 = 5;
const YCBCR: u64 = 6;

/// How the samples of a pixel are stored: one of Pillow's raw modes, up
/// to the byte order and fill order, which are handled separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layout {
    /// One bit per pixel (`1`, `1;I`): mode `1`.
    Bilevel {
        /// Whether a set bit is black.
        white_is_zero: bool,
    },
    /// Grey levels of 2, 4 or 8 bits, scaled to bytes (`L;2`, `L;4I`,
    /// `L` ...): mode `L`.
    Gray {
        /// Bits per sample.
        bits: u8,
        /// Whether 0 is white.
        white_is_zero: bool,
    },
    /// 16-bit grey levels (`I;16`, `I;16B`): mode `I;16`.
    Gray16,
    /// 8-bit grey and alpha (`LA`): mode `LA`.
    GrayAlpha,
    /// Red, green and blue of 8 or 16 bits, then `extra` ignored samples
    /// (`RGB`, `RGBX`, `RGB;16L` ...): mode `RGB`.
    Rgb {
        /// Whether samples have 16 bits, of which the high byte is kept.
        wide: bool,
        /// Extra samples skipped.
        extra: usize,
    },
    /// RGB and alpha, then `extra` ignored samples (`RGBA`, `RGBa`,
    /// `RGBAX`, `RGBa;16L` ...): mode `RGBA`.
    Rgba {
        /// Whether samples have 16 bits, of which the high byte is kept.
        wide: bool,
        /// Whether the colour is premultiplied by alpha.
        associated: bool,
        /// Extra samples skipped.
        extra: usize,
    },
    /// Palette indices of 1, 2, 4 or 8 bits (`P;1` ... `P`): mode `P`.
    Palette {
        /// Bits per index.
        bits: u8,
    },
    /// An 8-bit index and an ignored sample (`PX`): mode `P`.
    PaletteExtra,
    /// An 8-bit index and alpha (`PA`): mode `PA`.
    PaletteAlpha,
    /// Four inks of 8 or 16 bits, then `extra` ignored samples (`CMYK`,
    /// `CMYKX`, `CMYK;16L` ...): mode `CMYK`.
    Cmyk {
        /// Whether samples have 16 bits, of which the high byte is kept.
        wide: bool,
        /// Extra samples skipped.
        extra: usize,
    },
}

/// What describes a layout: the tags as Pillow normalises them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Description<'a> {
    /// The byte order of the file.
    pub(super) order: ByteOrder,
    /// `PhotometricInterpretation`.
    pub(super) photometric: u64,
    /// `SampleFormat`, a single value when all were equal.
    pub(super) sample_format: &'a [u64],
    /// `FillOrder`.
    pub(super) fill_order: u64,
    /// `BitsPerSample`, one per sample.
    pub(super) bits: &'a [u64],
    /// `ExtraSamples`.
    pub(super) extra: &'a [u64],
}

impl Layout {
    /// The layout Pillow reads for `description`, if any.
    pub(super) fn find(description: &Description<'_>) -> Option<Self> {
        let Description {
            order,
            photometric,
            sample_format,
            fill_order,
            bits,
            extra,
        } = *description;
        let little = order == ByteOrder::Little;
        let reversible = matches!(fill_order, 1 | 2);
        let plain = fill_order == 1;
        let unsigned = sample_format == [1];
        let all =
            |width: u64, count: usize| bits.len() == count && bits.iter().all(|&b| b == width);
        let low_bits = |b: u64| u8::try_from(b).ok().filter(|b| matches!(b, 1 | 2 | 4 | 8));
        Some(match (photometric, extra) {
            (WHITE_IS_ZERO | BLACK_IS_ZERO, []) if unsigned && bits.len() == 1 => {
                let white_is_zero = photometric == WHITE_IS_ZERO;
                match bits[0] {
                    1 if reversible => Self::Bilevel { white_is_zero },
                    2 | 4 | 8 if reversible => Self::Gray {
                        bits: low_bits(bits[0])?,
                        white_is_zero,
                    },
                    16 if little && (plain || photometric == BLACK_IS_ZERO && fill_order == 2) => {
                        Self::Gray16
                    }
                    16 if photometric == BLACK_IS_ZERO && plain => Self::Gray16,
                    _ => return None,
                }
            }
            (BLACK_IS_ZERO, []) if sample_format == [2] && all(8, 1) && plain => Self::Gray {
                bits: 8,
                white_is_zero: false,
            },
            (BLACK_IS_ZERO, [2]) if unsigned && all(8, 2) && plain => Self::GrayAlpha,
            (RGB, []) if unsigned && all(8, 3) && reversible => Self::Rgb {
                wide: false,
                extra: 0,
            },
            (RGB, []) if unsigned && (all(8, 4) || all(16, 4)) && plain => Self::Rgba {
                wide: bits[0] == 16,
                associated: false,
                extra: 0,
            },
            (RGB, []) if unsigned && all(16, 3) && plain => Self::Rgb {
                wide: true,
                extra: 0,
            },
            (RGB, [_, ..]) if unsigned && all(8, 3 + extra.len()) && plain => rgb(false, extra)?,
            (RGB, [_]) if unsigned && all(16, 4) && plain => rgb(true, extra)?,
            (PALETTE, []) if unsigned && bits.len() == 1 && reversible => Self::Palette {
                bits: low_bits(bits[0])?,
            },
            (PALETTE, [0]) if unsigned && all(8, 2) && plain => Self::PaletteExtra,
            (PALETTE, [2]) if unsigned && all(8, 2) && plain => Self::PaletteAlpha,
            (SEPARATED, [] | [0] | [0, 0]) if unsigned && all(8, 4 + extra.len()) && plain => {
                Self::Cmyk {
                    wide: false,
                    extra: extra.len(),
                }
            }
            (SEPARATED, []) if unsigned && all(16, 4) && plain => Self::Cmyk {
                wide: true,
                extra: 0,
            },
            (YCBCR, []) if unsigned && all(8, 1) && plain => Self::Gray {
                bits: 8,
                white_is_zero: false,
            },
            _ => return None,
        })
    }

    /// Bits per pixel in the file.
    pub(super) fn bits_per_pixel(self) -> usize {
        match self {
            Self::Bilevel { .. } => 1,
            Self::Gray { bits, .. } | Self::Palette { bits } => usize::from(bits),
            Self::Gray16 | Self::GrayAlpha | Self::PaletteExtra | Self::PaletteAlpha => 16,
            Self::Rgb { wide, extra } => (3 + extra) * sample_bits(wide),
            Self::Rgba { wide, extra, .. } | Self::Cmyk { wide, extra } => {
                (4 + extra) * sample_bits(wide)
            }
        }
    }

    /// Bytes per pixel of the decoded image (a 16-bit grey level takes
    /// two, least significant first).
    pub(super) fn channels(self) -> usize {
        match self {
            Self::Bilevel { .. }
            | Self::Gray { .. }
            | Self::Palette { .. }
            | Self::PaletteExtra => 1,
            Self::Gray16 | Self::GrayAlpha | Self::PaletteAlpha => 2,
            Self::Rgb { .. } => 3,
            Self::Rgba { .. } | Self::Cmyk { .. } => 4,
        }
    }

    /// Whether the samples are bytes or 16-bit numbers, as horizontal
    /// differencing needs; `None` for packed samples.
    pub(super) fn sample_bytes(self) -> Option<usize> {
        match self {
            Self::Bilevel { .. }
            | Self::Palette { bits: 1 | 2 | 4 }
            | Self::Gray { bits: 2 | 4, .. } => None,
            Self::Gray16 => Some(2),
            Self::Rgb { wide, .. } | Self::Rgba { wide, .. } | Self::Cmyk { wide, .. } => {
                Some(if wide { 2 } else { 1 })
            }
            _ => Some(1),
        }
    }

    /// Samples per pixel in the file.
    pub(super) fn samples(self) -> usize {
        match self.sample_bytes() {
            Some(bytes) => self.bits_per_pixel() / (8 * bytes),
            None => 1,
        }
    }
}

/// Bits per sample.
fn sample_bits(wide: bool) -> usize {
    if wide { 16 } else { 8 }
}

/// The RGB(A) layout of three colour samples followed by the `extra`
/// ones `ExtraSamples` lists, following Pillow's table: unspecified extras
/// are ignored, a first extra sample of 1 is associated alpha, of 2 (or
/// 999, from Corel Draw) straight alpha, and later ones must be
/// unspecified.
fn rgb(wide: bool, extra: &[u64]) -> Option<Layout> {
    let max_extra = if wide { 1 } else { 3 };
    if extra.len() > max_extra {
        return None;
    }
    Some(match extra {
        [first, rest @ ..] if rest.iter().all(|&e| e == 0) => match first {
            0 => Layout::Rgb {
                wide,
                extra: extra.len(),
            },
            1 => Layout::Rgba {
                wide,
                associated: true,
                extra: rest.len(),
            },
            2 => Layout::Rgba {
                wide,
                associated: false,
                extra: rest.len(),
            },
            999 if extra.len() == 1 && !wide => Layout::Rgba {
                wide,
                associated: false,
                extra: 0,
            },
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(
        photometric: u64,
        format: &[u64],
        fill: u64,
        bits: &[u64],
        extra: &[u64],
    ) -> Option<Layout> {
        Layout::find(&Description {
            order: ByteOrder::Little,
            photometric,
            sample_format: format,
            fill_order: fill,
            bits,
            extra,
        })
    }

    #[test]
    fn pillow_s_table() {
        assert_eq!(
            find(0, &[1], 2, &[1], &[]),
            Some(Layout::Bilevel {
                white_is_zero: true
            })
        );
        assert_eq!(
            find(1, &[1], 1, &[4], &[]),
            Some(Layout::Gray {
                bits: 4,
                white_is_zero: false
            })
        );
        assert_eq!(find(1, &[1], 1, &[16], &[]), Some(Layout::Gray16));
        assert_eq!(
            find(1, &[2], 1, &[16], &[]),
            None,
            "signed 16-bit is mode I"
        );
        assert_eq!(find(1, &[1], 1, &[8, 8], &[2]), Some(Layout::GrayAlpha));
        assert_eq!(find(1, &[1], 1, &[8, 8], &[1]), None);
        assert_eq!(
            find(2, &[1], 2, &[8, 8, 8], &[]),
            Some(Layout::Rgb {
                wide: false,
                extra: 0
            })
        );
        assert_eq!(
            find(2, &[1], 1, &[8; 4], &[]),
            Some(Layout::Rgba {
                wide: false,
                associated: false,
                extra: 0
            })
        );
        assert_eq!(
            find(2, &[1], 1, &[8; 5], &[1, 0]),
            Some(Layout::Rgba {
                wide: false,
                associated: true,
                extra: 1
            })
        );
        assert_eq!(
            find(2, &[1], 1, &[8; 6], &[0, 0, 0]),
            Some(Layout::Rgb {
                wide: false,
                extra: 3
            })
        );
        assert_eq!(
            find(2, &[1], 1, &[16; 4], &[1]),
            Some(Layout::Rgba {
                wide: true,
                associated: true,
                extra: 0
            })
        );
        assert_eq!(find(2, &[1], 1, &[16; 5], &[2, 0]), None);
        assert_eq!(
            find(3, &[1], 2, &[4], &[]),
            Some(Layout::Palette { bits: 4 })
        );
        assert_eq!(find(3, &[1], 1, &[8, 8], &[2]), Some(Layout::PaletteAlpha));
        assert_eq!(
            find(5, &[1], 1, &[8; 5], &[0]),
            Some(Layout::Cmyk {
                wide: false,
                extra: 1
            })
        );
        assert_eq!(find(8, &[1], 1, &[8; 3], &[]), None, "LAB");
        let big = Description {
            order: ByteOrder::Big,
            photometric: 0,
            sample_format: &[1],
            fill_order: 1,
            bits: &[16],
            extra: &[],
        };
        assert_eq!(Layout::find(&big), None, "big-endian white-is-zero 16-bit");
    }

    #[test]
    fn sizes() {
        let rgba16 = Layout::Rgba {
            wide: true,
            associated: false,
            extra: 0,
        };
        assert_eq!(
            (rgba16.bits_per_pixel(), rgba16.channels(), rgba16.samples()),
            (64, 4, 4)
        );
        let cmykx = Layout::Cmyk {
            wide: false,
            extra: 1,
        };
        assert_eq!((cmykx.bits_per_pixel(), cmykx.samples()), (40, 5));
        assert_eq!(Layout::Palette { bits: 2 }.sample_bytes(), None);
    }
}
