//! The bitmap file formats that figures and image arrays are written in:
//! PNG, raw RGBA buffers (Agg's `print_raw`), and JPEG, TIFF, GIF and
//! WebP, which matplotlib writes through Pillow (`print_jpg`, `print_tif`,
//! `print_gif`, `print_webp` and `image.imsave`). JPEG and TIFF files
//! match Pillow's byte for byte; WebP files are lossless, where Pillow
//! writes lossy ones by default.

use std::fmt;

use plt_codec::gif::{GifEncoder, GifError, GifFrame};
use plt_codec::jpeg::{JpegError, JpegOptions, encode_rgb8};
use plt_codec::png::{self, EncodeError, ImageRef};
use plt_codec::tiff::{TiffError, encode_rgba8};
use plt_codec::webp;

/// A bitmap file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BitmapFormat {
    /// Portable Network Graphics, with transparency.
    Png,
    /// Baseline JPEG; transparency is flattened onto a background.
    Jpeg,
    /// Uncompressed TIFF, with transparency.
    Tiff,
    /// A single-frame GIF of at most 256 colours.
    Gif,
    /// Lossless WebP, with transparency.
    Webp,
    /// The bare RGBA bytes, row by row, with no header.
    Raw,
}

/// Settings of the formats that need more than the pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BitmapSettings {
    /// The resolution recorded in the file, in dots per inch.
    pub dpi: f64,
    /// JPEG quality from 1 to 100 (Pillow's `quality`).
    pub jpeg_quality: u8,
    /// The colour JPEG pixels are flattened onto.
    pub background: [u8; 3],
}

impl Default for BitmapSettings {
    /// 100 dpi, Pillow's JPEG quality of 75, and a white background.
    fn default() -> Self {
        Self {
            dpi: 100.0,
            jpeg_quality: 75,
            background: [255; 3],
        }
    }
}

/// Why pixels could not be encoded.
#[derive(Clone, Debug, PartialEq)]
pub enum BitmapError {
    /// As a PNG.
    Png(EncodeError),
    /// As a JPEG.
    Jpeg(JpegError),
    /// As a TIFF.
    Tiff(TiffError),
    /// As a GIF.
    Gif(GifError),
    /// As a WebP.
    Webp(webp::EncodeError),
    /// The pixel data does not have `width × height × 4` bytes.
    DataLengthMismatch,
}

impl fmt::Display for BitmapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Png(err) => write!(f, "cannot encode the PNG: {err}"),
            Self::Jpeg(err) => write!(f, "cannot encode the JPEG: {err}"),
            Self::Tiff(err) => write!(f, "cannot encode the TIFF: {err}"),
            Self::Gif(err) => write!(f, "cannot encode the GIF: {err}"),
            Self::Webp(err) => write!(f, "cannot encode the WebP: {err}"),
            Self::DataLengthMismatch => f.write_str("the pixel data does not match the image size"),
        }
    }
}

impl std::error::Error for BitmapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Png(err) => Some(err),
            Self::Jpeg(err) => Some(err),
            Self::Tiff(err) => Some(err),
            Self::Gif(err) => Some(err),
            Self::Webp(err) => Some(err),
            Self::DataLengthMismatch => None,
        }
    }
}

impl BitmapFormat {
    /// Encodes `width × height` straight RGBA pixels, row by row.
    ///
    /// # Errors
    ///
    /// When the data does not match the size, or the size is out of the
    /// format's range.
    pub fn encode(
        self,
        width: usize,
        height: usize,
        rgba: &[u8],
        settings: &BitmapSettings,
    ) -> Result<Vec<u8>, BitmapError> {
        if width.checked_mul(height).and_then(|n| n.checked_mul(4)) != Some(rgba.len()) {
            return Err(BitmapError::DataLengthMismatch);
        }
        let side = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let (columns, rows) = (side(width), side(height));
        match self {
            Self::Png => ImageRef::rgba8(columns, rows, rgba)
                .and_then(|image| png::Encoder::new().dpi(settings.dpi).encode(image))
                .map_err(BitmapError::Png),
            Self::Jpeg => {
                let options = JpegOptions {
                    quality: settings.jpeg_quality,
                    dpi: jfif_density(settings.dpi),
                };
                encode_rgb8(columns, rows, &flatten(rgba, settings.background), &options)
                    .map_err(BitmapError::Jpeg)
            }
            Self::Tiff => {
                encode_rgba8(columns, rows, rgba, settings.dpi).map_err(BitmapError::Tiff)
            }
            Self::Gif => {
                let frame = GifFrame {
                    pixels: rgba,
                    delay: 0,
                };
                GifEncoder::new()
                    .plays(1)
                    .encode(columns, rows, &[frame])
                    .map_err(BitmapError::Gif)
            }
            Self::Webp => webp::encode_rgba8(columns, rows, rgba).map_err(BitmapError::Webp),
            Self::Raw => Ok(rgba.to_vec()),
        }
    }
}

/// The JFIF density for `dpi`: rounded, as Pillow passes it to libjpeg.
fn jfif_density(dpi: f64) -> u16 {
    let rounded = dpi.round().clamp(0.0, f64::from(u16::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rounded and clamped to u16's range"
    )]
    let density = rounded as u16;
    density
}

/// RGBA pixels flattened onto `background` as Pillow's
/// `background.paste(image, image)` blends them: `(bg × (255 − a) + c × a)
/// / 255` in Pillow's rounded integer division.
fn flatten(rgba: &[u8], background: [u8; 3]) -> Vec<u8> {
    let blend = |under: u8, over: u8, alpha: u8| {
        let sum = u32::from(under) * u32::from(255 - alpha) + u32::from(over) * u32::from(alpha);
        let tmp = sum + 128;
        u8::try_from(((tmp >> 8) + tmp) >> 8).expect("a blend of bytes")
    };
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| {
            [r, g, b]
                .into_iter()
                .zip(background)
                .map(move |(over, under)| blend(under, over, a))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparency_flattens_like_pillow() {
        let pixels = [10, 20, 30, 0, 10, 20, 30, 255, 200, 100, 0, 128];
        let flat = flatten(&pixels, [255, 255, 255]);
        assert_eq!(&flat[..6], [255, 255, 255, 10, 20, 30]);
        // (255 × 127 + 200 × 128) / 255 and so on, rounded.
        assert_eq!(&flat[6..], [227, 177, 127]);
    }

    #[test]
    fn each_format_has_its_signature() {
        let pixels = [255, 0, 0, 255].repeat(6);
        let settings = BitmapSettings::default();
        let encode = |format: BitmapFormat| format.encode(3, 2, &pixels, &settings).unwrap();
        assert!(encode(BitmapFormat::Png).starts_with(b"\x89PNG"));
        assert!(encode(BitmapFormat::Jpeg).starts_with(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(encode(BitmapFormat::Tiff).starts_with(b"II*\0"));
        assert!(encode(BitmapFormat::Gif).starts_with(b"GIF89a"));
        let webp = encode(BitmapFormat::Webp);
        assert!(webp.starts_with(b"RIFF") && &webp[8..16] == b"WEBPVP8L");
        assert_eq!(encode(BitmapFormat::Raw), pixels);
    }

    #[test]
    fn densities_round_and_saturate() {
        assert_eq!(jfif_density(72.4), 72);
        assert_eq!(jfif_density(99.5), 100);
        assert_eq!(jfif_density(1e9), u16::MAX);
        assert_eq!(jfif_density(f64::NAN), 0);
    }

    #[test]
    fn mismatched_data_is_rejected() {
        let settings = BitmapSettings::default();
        assert_eq!(
            BitmapFormat::Raw.encode(2, 2, &[0; 4], &settings),
            Err(BitmapError::DataLengthMismatch)
        );
    }
}
