//! The colour space of the coded components, as libjpeg guesses it
//! (`default_decompress_parms`), and the conversions to the colours Pillow
//! asks for (`jdcolor.c`): fixed-point YCbCr → RGB through libjpeg's
//! tables, YCCK → CMYK, and pass-through.

/// Fractional bits of the conversion tables.
const SCALE_BITS: u32 = 16;
const ONE_HALF: i32 = 1 << (SCALE_BITS - 1);
/// `FIX(x)`: `x` in 16.16 fixed point, rounded.
const FIX_1_40200: i32 = 91_881;
const FIX_1_77200: i32 = 116_130;
const FIX_0_71414: i32 = 46_802;
const FIX_0_34414: i32 = 22_554;

/// What the components of a frame hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColorSpace {
    /// One grey component.
    Gray,
    /// Luma and two chroma components.
    YCbCr,
    /// Red, green and blue components.
    Rgb,
    /// Four ink components, Adobe-inverted (255 is no ink).
    Cmyk,
    /// YCbCr-coded inverted CMY, plus inverted K.
    Ycck,
}

/// The markers that decide the colour space, as seen before the first
/// scan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ColorHints {
    /// A JFIF APP0 segment was seen.
    pub(super) jfif: bool,
    /// The transform of an Adobe APP14 segment, if one was seen.
    pub(super) adobe: Option<u8>,
}

impl ColorSpace {
    /// libjpeg's guess for `ids.len()` components (1, 3 or 4) with those
    /// identifiers.
    pub(super) fn guess(ids: &[u8], hints: ColorHints) -> Self {
        match ids {
            [_] => Self::Gray,
            [_, _, _] if hints.jfif => Self::YCbCr,
            [_, _, _] => match hints.adobe {
                Some(0) => Self::Rgb,
                None if ids == b"RGB" => Self::Rgb,
                _ => Self::YCbCr,
            },
            _ => match hints.adobe {
                None | Some(0) => Self::Cmyk,
                Some(_) => Self::Ycck,
            },
        }
    }
}

/// libjpeg's YCbCr → RGB tables (`build_ycc_rgb_table`), indexed by the
/// chroma sample.
#[derive(Debug)]
struct YccTables {
    cr_r: [i32; 256],
    cb_b: [i32; 256],
    cr_g: [i32; 256],
    cb_g: [i32; 256],
}

impl YccTables {
    fn new() -> Self {
        let x = |i: usize| i32::try_from(i).expect("the tables have 256 entries") - 128;
        Self {
            cr_r: std::array::from_fn(|i| (FIX_1_40200 * x(i) + ONE_HALF) >> SCALE_BITS),
            cb_b: std::array::from_fn(|i| (FIX_1_77200 * x(i) + ONE_HALF) >> SCALE_BITS),
            cr_g: std::array::from_fn(|i| -FIX_0_71414 * x(i)),
            cb_g: std::array::from_fn(|i| -FIX_0_34414 * x(i) + ONE_HALF),
        }
    }

    /// The RGB of a YCbCr sample, each channel clamped to a byte.
    fn rgb(&self, y: u8, cb: u8, cr: u8) -> [u8; 3] {
        let (y, cb, cr) = (i32::from(y), usize::from(cb), usize::from(cr));
        let green = (self.cb_g[cb] + self.cr_g[cr]) >> SCALE_BITS;
        [y + self.cr_r[cr], y + green, y + self.cb_b[cb]].map(clamp)
    }
}

/// A value clamped to a byte (libjpeg's `range_limit`).
fn clamp(value: i32) -> u8 {
    u8::try_from(value.clamp(0, 255)).expect("clamped to a byte")
}

/// RGB pixels from three full-size YCbCr planes (`ycc_rgb_convert`).
pub(super) fn ycc_to_rgb(y: &[u8], cb: &[u8], cr: &[u8]) -> Vec<[u8; 3]> {
    let tables = YccTables::new();
    y.iter()
        .zip(cb)
        .zip(cr)
        .map(|((&y, &cb), &cr)| tables.rgb(y, cb, cr))
        .collect()
}

/// Adobe-inverted CMYK pixels from four full-size YCCK planes
/// (`ycck_cmyk_convert`): the YCbCr part converts to RGB, inverted.
pub(super) fn ycck_to_cmyk(planes: [&[u8]; 4]) -> Vec<[u8; 4]> {
    let tables = YccTables::new();
    (0..planes[0].len())
        .map(|i| {
            let [r, g, b] = tables.rgb(planes[0][i], planes[1][i], planes[2][i]);
            [255 - r, 255 - g, 255 - b, planes[3][i]]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_spaces_are_guessed_like_libjpeg() {
        let none = ColorHints::default();
        let jfif = ColorHints {
            jfif: true,
            adobe: Some(0),
        };
        let adobe = |t| ColorHints {
            jfif: false,
            adobe: Some(t),
        };
        assert_eq!(ColorSpace::guess(&[1], none), ColorSpace::Gray);
        assert_eq!(ColorSpace::guess(&[1, 2, 3], none), ColorSpace::YCbCr);
        assert_eq!(ColorSpace::guess(b"RGB", none), ColorSpace::Rgb);
        assert_eq!(ColorSpace::guess(b"RGB", jfif), ColorSpace::YCbCr);
        assert_eq!(ColorSpace::guess(&[1, 2, 3], adobe(0)), ColorSpace::Rgb);
        assert_eq!(ColorSpace::guess(&[1, 2, 3], adobe(7)), ColorSpace::YCbCr);
        assert_eq!(ColorSpace::guess(&[1, 2, 3, 4], none), ColorSpace::Cmyk);
        assert_eq!(ColorSpace::guess(&[1, 2, 3, 4], adobe(2)), ColorSpace::Ycck);
        assert_eq!(ColorSpace::guess(&[1, 2, 3, 4], adobe(5)), ColorSpace::Ycck);
    }

    #[test]
    fn ycc_converts_with_libjpeg_tables() {
        assert_eq!(ycc_to_rgb(&[128], &[128], &[128]), [[128, 128, 128]]);
        // The encoder's red, (76, 85, 255), comes back as (254, 0, 0).
        assert_eq!(ycc_to_rgb(&[76], &[85], &[255]), [[254, 0, 0]]);
        assert_eq!(ycc_to_rgb(&[255], &[255], &[255]), [[255, 121, 255]]);
        assert_eq!(
            ycck_to_cmyk([&[128], &[128], &[128], &[9]]),
            [[127, 127, 127, 9]]
        );
    }
}
