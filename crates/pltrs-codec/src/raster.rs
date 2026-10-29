//! Decoded images as Pillow presents them: the JPEG, TIFF and GIF decoders
//! all produce a [`Raster`] whose [`Pixels`] variant is the Pillow image
//! mode the same file opens in, holding the same values. [`Raster::to_rgba8`]
//! is Pillow's `convert("RGBA")`.

/// The most pixels a decoded image may have. Pillow refuses larger images
/// as decompression bombs (twice `PIL.Image.MAX_IMAGE_PIXELS`).
pub const MAX_PIXELS: u64 = 178_956_970;

/// The colours of an indexed image, with at most one transparent entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    colors: Vec<[u8; 3]>,
    transparent: Option<u8>,
}

impl Palette {
    /// A palette of at most 256 `colors` (later entries are dropped);
    /// `transparent` is the index drawn fully transparent, if any.
    pub fn new(mut colors: Vec<[u8; 3]>, transparent: Option<u8>) -> Self {
        colors.truncate(256);
        Self {
            colors,
            transparent,
        }
    }

    /// The colours, in index order.
    pub fn colors(&self) -> &[[u8; 3]] {
        &self.colors
    }

    /// The transparent index, if any.
    pub fn transparent(&self) -> Option<u8> {
        self.transparent
    }

    /// The RGBA colour of `index` as Pillow's palettes give it: indices
    /// past the colours are opaque black, the transparent index has alpha
    /// 0 and every other entry is opaque.
    pub fn rgba(&self, index: u8) -> [u8; 4] {
        let [r, g, b] = self
            .colors
            .get(usize::from(index))
            .copied()
            .unwrap_or_default();
        let alpha = if self.transparent == Some(index) {
            0
        } else {
            255
        };
        [r, g, b, alpha]
    }
}

/// The pixels of a decoded image, row by row, in one of Pillow's modes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pixels {
    /// Mode `1`: bilevel pixels, each 0 (black) or 255 (white).
    Bilevel(Vec<u8>),
    /// Mode `L`: 8-bit grey levels.
    Gray(Vec<u8>),
    /// Mode `I;16`: 16-bit grey levels.
    Gray16(Vec<u16>),
    /// Mode `LA`: 8-bit grey levels with alpha.
    GrayAlpha(Vec<[u8; 2]>),
    /// Mode `RGB`: 8-bit colours.
    Rgb(Vec<[u8; 3]>),
    /// Mode `RGBA`: 8-bit colours with straight (not premultiplied) alpha.
    Rgba(Vec<[u8; 4]>),
    /// Mode `CMYK`: 8-bit ink amounts, 255 being full ink.
    Cmyk(Vec<[u8; 4]>),
    /// Mode `P`: palette indices.
    Indexed {
        /// The colours the indices refer to.
        palette: Palette,
        /// One index per pixel.
        indices: Vec<u8>,
    },
    /// Mode `PA`: palette indices with alpha.
    IndexedAlpha {
        /// The colours the indices refer to.
        palette: Palette,
        /// An index and an alpha per pixel.
        pixels: Vec<[u8; 2]>,
    },
}

impl Pixels {
    /// The number of pixels held.
    pub fn len(&self) -> usize {
        match self {
            Self::Bilevel(v) | Self::Gray(v) => v.len(),
            Self::Gray16(v) => v.len(),
            Self::GrayAlpha(v) => v.len(),
            Self::Rgb(v) => v.len(),
            Self::Rgba(v) | Self::Cmyk(v) => v.len(),
            Self::Indexed { indices, .. } => indices.len(),
            Self::IndexedAlpha { pixels, .. } => pixels.len(),
        }
    }

    /// Returns `true` when no pixels are held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every pixel as Pillow's `convert("RGBA")` gives it.
    pub fn to_rgba8(&self) -> Vec<[u8; 4]> {
        let gray = |v: u8| [v, v, v, 255];
        match self {
            Self::Bilevel(v) => v
                .iter()
                .map(|&v| gray(if v == 0 { 0 } else { 255 }))
                .collect(),
            Self::Gray(v) => v.iter().map(|&v| gray(v)).collect(),
            Self::Gray16(v) => v
                .iter()
                .map(|&v| gray(u8::try_from(v).unwrap_or(u8::MAX)))
                .collect(),
            Self::GrayAlpha(v) => v.iter().map(|&[l, a]| [l, l, l, a]).collect(),
            Self::Rgb(v) => v.iter().map(|&[r, g, b]| [r, g, b, 255]).collect(),
            Self::Rgba(v) => v.clone(),
            Self::Cmyk(v) => v.iter().map(|&cmyk| cmyk_to_rgba(cmyk)).collect(),
            Self::Indexed { palette, indices } => {
                indices.iter().map(|&i| palette.rgba(i)).collect()
            }
            Self::IndexedAlpha { palette, pixels } => pixels
                .iter()
                .map(|&[i, a]| {
                    let [r, g, b, _] = palette.rgba(i);
                    [r, g, b, a]
                })
                .collect(),
        }
    }
}

/// Pillow's `cmyk2rgb`: each channel is `(255 − k) · (1 − ink / 255)`,
/// rounded with its divide-by-255 shortcut.
fn cmyk_to_rgba([c, m, y, k]: [u8; 4]) -> [u8; 4] {
    let remaining = 255 - u32::from(k);
    let channel = |ink: u8| {
        let product = u32::from(ink) * remaining + 128;
        let scaled = ((product >> 8) + product) >> 8;
        u8::try_from(remaining - scaled).expect("at most the remaining 255 - k")
    };
    [channel(c), channel(m), channel(y), 255]
}

/// A decoded image: its size and its pixels in a Pillow mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    width: u32,
    height: u32,
    pixels: Pixels,
}

impl Raster {
    /// An image of `width × height` pixels; `pixels` holds exactly that
    /// many.
    pub(crate) fn new(width: u32, height: u32, pixels: Pixels) -> Self {
        debug_assert_eq!(
            pixels.len() as u64,
            u64::from(width) * u64::from(height),
            "the pixels fill the image"
        );
        Self {
            width,
            height,
            pixels,
        }
    }

    /// The width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The pixels, row by row.
    pub fn pixels(&self) -> &Pixels {
        &self.pixels
    }

    /// Takes the pixels.
    pub fn into_pixels(self) -> Pixels {
        self.pixels
    }

    /// Every pixel as Pillow's `convert("RGBA")` gives it.
    pub fn to_rgba8(&self) -> Vec<[u8; 4]> {
        self.pixels.to_rgba8()
    }
}

/// Whether a `width × height` image is small enough to decode: at most
/// [`MAX_PIXELS`] pixels and, as each pixel costs at least `1 / per_byte`
/// bytes of a valid file, at most `per_byte × input_len` pixels. Checking
/// before allocating keeps a forged header from claiming gigabytes.
pub(crate) fn fits(width: u32, height: u32, input_len: usize, per_byte: u64) -> bool {
    let pixels = u64::from(width) * u64::from(height);
    let input = u64::try_from(input_len).unwrap_or(u64::MAX);
    pixels <= MAX_PIXELS && pixels <= input.saturating_mul(per_byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_fill_missing_entries_with_opaque_black() {
        let palette = Palette::new(vec![[1, 2, 3], [4, 5, 6]], Some(1));
        assert_eq!(palette.rgba(0), [1, 2, 3, 255]);
        assert_eq!(palette.rgba(1), [4, 5, 6, 0]);
        assert_eq!(palette.rgba(9), [0, 0, 0, 255]);
        let transparent_past_end = Palette::new(vec![[1, 2, 3]], Some(200));
        assert_eq!(transparent_past_end.rgba(200), [0, 0, 0, 0]);
        assert_eq!(Palette::new(vec![[0; 3]; 300], None).colors().len(), 256);
    }

    #[test]
    fn conversions_follow_pillow() {
        assert_eq!(
            Pixels::Bilevel(vec![0, 255]).to_rgba8(),
            [[0, 0, 0, 255], [255; 4]]
        );
        assert_eq!(
            Pixels::Gray16(vec![7, 300]).to_rgba8(),
            [[7, 7, 7, 255], [255; 4]]
        );
        assert_eq!(Pixels::GrayAlpha(vec![[9, 3]]).to_rgba8(), [[9, 9, 9, 3]]);
        let palette = Palette::new(vec![[10, 20, 30]], None);
        assert_eq!(
            Pixels::IndexedAlpha {
                palette,
                pixels: vec![[0, 77]]
            }
            .to_rgba8(),
            [[10, 20, 30, 77]]
        );
    }

    #[test]
    fn cmyk_converts_like_pillow() {
        assert_eq!(cmyk_to_rgba([0, 0, 0, 0]), [255, 255, 255, 255]);
        assert_eq!(cmyk_to_rgba([255, 0, 0, 0]), [0, 255, 255, 255]);
        assert_eq!(cmyk_to_rgba([0, 0, 0, 255]), [0, 0, 0, 255]);
        // nk = 155, 100 · 155 / 255 ≈ 60.8 rounds to 61: 155 − 61.
        assert_eq!(cmyk_to_rgba([100, 0, 0, 100])[0], 94);
    }

    #[test]
    fn size_limits() {
        assert!(fits(10, 10, 1, 100));
        assert!(!fits(10, 11, 1, 100));
        assert!(!fits(65_535, 65_535, usize::MAX, u64::MAX));
    }
}
