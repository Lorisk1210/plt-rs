//! AVIF files: [AV1](crate::av1) still pictures in the HEIF container,
//! written as Pillow writes them through libavif (matplotlib's
//! `print_avif`).
//!
//! [`encode_rgba8`] converts RGBA pixels to BT.601 YUV in full range
//! (4:2:0 by default, as Pillow subsamples), codes them at a quality from
//! 0 to 100 mapped to quantizers as libavif maps it, and stores alpha, when
//! some pixel is not opaque, as an auxiliary monochrome image. Quality 100
//! is lossless, as in libavif's lossless mode: the colours are stored as
//! they are (G, B, R in 4:4:4) and decode to the exact pixels.
//!
//! ```
//! use pltrs_codec::avif::{AvifOptions, encode_rgba8};
//! let pixels = [255, 0, 0, 255, 0, 0, 255, 128];
//! let file = encode_rgba8(2, 1, &pixels, &AvifOptions::default())?;
//! assert_eq!(&file[4..12], b"ftypavif");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod color;
mod container;

use std::fmt;

use crate::av1::{self, Av1Error, Bitstream, ChromaSampling, ColorDescription, Picture};
use container::Item;

/// The largest side libavif decodes (`AVIF_DEFAULT_IMAGE_DIMENSION_LIMIT`).
pub const MAX_SIDE: u32 = 32768;
/// The most pixels libavif decodes (`AVIF_DEFAULT_IMAGE_SIZE_LIMIT`).
pub const MAX_PIXELS: u64 = 16384 * 16384;

/// libavif's quantizer (0 to 63) for a colour quality from 0 to 99, the
/// curve of `tune=iq` it uses with libaom 3.13 and later.
#[rustfmt::skip]
const COLOR_QUANTIZERS: [u8; 100] = [
    63, 63, 63, 62, 62, 62, 61, 61, 60, 60, 59, 59, 58, 58, 57, 57, 56, 56, 55, 55,
    54, 54, 53, 53, 52, 52, 51, 51, 50, 50, 49, 49, 48, 48, 47, 46, 46, 45, 45, 44,
    43, 43, 42, 42, 41, 40, 40, 39, 39, 38, 37, 37, 36, 36, 35, 34, 33, 33, 32, 31,
    30, 30, 29, 28, 27, 27, 26, 25, 24, 24, 23, 22, 21, 21, 20, 19, 18, 18, 17, 16,
    15, 15, 14, 13, 12, 12, 11, 10, 9, 9, 8, 7, 6, 6, 5, 4, 3, 3, 2, 1,
];

/// How [`encode_rgba8`] codes a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvifOptions {
    /// Pillow's `quality`, from 0 (smallest) to 100 (lossless); 75 by
    /// default.
    pub quality: u8,
    /// Pillow's `subsampling` of lossy pictures: 4:2:0 by default, 4:4:4,
    /// or 4:0:0 (luma only, a grey picture).
    pub subsampling: ChromaSampling,
}

impl Default for AvifOptions {
    /// Pillow's defaults: quality 75 in 4:2:0.
    fn default() -> Self {
        Self {
            quality: 75,
            subsampling: ChromaSampling::Yuv420,
        }
    }
}

/// Why pixels could not be encoded as AVIF.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvifError {
    /// A side is zero, or the picture is larger than libavif decodes
    /// ([`MAX_SIDE`], [`MAX_PIXELS`]).
    Size {
        /// The width given.
        width: u32,
        /// The height given.
        height: u32,
    },
    /// The pixel data does not hold `width × height × 4` bytes.
    DataLength,
    /// The quality is above 100.
    Quality(u8),
    /// The AV1 encoder refused the picture.
    Av1(Av1Error),
}

impl fmt::Display for AvifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size { width, height } => write!(f, "cannot write a {width}×{height} AVIF"),
            Self::DataLength => f.write_str("the pixel data does not match the image size"),
            Self::Quality(quality) => write!(f, "invalid AVIF quality {quality} (0 to 100)"),
            Self::Av1(err) => write!(f, "cannot encode the AV1 picture: {err}"),
        }
    }
}

impl std::error::Error for AvifError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Av1(err) => Some(err),
            _ => None,
        }
    }
}

impl From<Av1Error> for AvifError {
    fn from(err: Av1Error) -> Self {
        Self::Av1(err)
    }
}

/// The AV1 quantizer index of libavif's quantizer (`av1_quantizer_to_qindex`).
fn qindex(quantizer: u8) -> u8 {
    match quantizer {
        0..=61 => quantizer * 4,
        62 => 249,
        _ => 255,
    }
}

/// The quantizer index of the colour planes at `quality` (below 100).
fn color_qindex(quality: u8) -> u8 {
    qindex(COLOR_QUANTIZERS[usize::from(quality)])
}

/// The quantizer index of the alpha plane at `quality`: libavif codes
/// alpha with `tune=psnr`, whose quantizer is linear in the quality.
fn alpha_qindex(quality: u8) -> u8 {
    let quantizer = ((100 - u32::from(quality)) * 63 + 50) / 100;
    qindex(quantizer as u8)
}

/// Encodes `width × height` straight RGBA pixels (row by row) as an AVIF
/// file.
///
/// # Errors
///
/// When a side is zero or the picture is larger than libavif decodes,
/// `pixels` does not hold `width × height × 4` bytes, or the quality is
/// above 100.
pub fn encode_rgba8(
    width: u32,
    height: u32,
    pixels: &[u8],
    options: &AvifOptions,
) -> Result<Vec<u8>, AvifError> {
    let area = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE || area > MAX_PIXELS {
        return Err(AvifError::Size { width, height });
    }
    if usize::try_from(area * 4) != Ok(pixels.len()) {
        return Err(AvifError::DataLength);
    }
    if options.quality > 100 {
        return Err(AvifError::Quality(options.quality));
    }
    let (w, h) = (width as usize, height as usize);
    let lossless = options.quality == 100;
    let (planes, sampling, description, qindex) = if lossless {
        let planes = color::identity(pixels);
        (
            planes,
            ChromaSampling::Yuv444,
            ColorDescription::SRGB_IDENTITY,
            0,
        )
    } else {
        let planes = color::bt601(w, h, pixels, options.subsampling);
        let qindex = color_qindex(options.quality);
        (
            planes,
            options.subsampling,
            ColorDescription::SRGB_BT601,
            qindex,
        )
    };
    let plane_slices: Vec<&[u8]> = [&planes.y, &planes.u, &planes.v]
        .into_iter()
        .take(sampling.planes())
        .map(Vec::as_slice)
        .collect();
    let picture =
        Picture::new(width, height, sampling, &plane_slices)?.with_color(description, true)?;
    let color_stream = av1::encode(&picture, qindex);
    let alpha = color::alpha(pixels);
    let alpha_stream: Option<Bitstream> = if alpha.iter().all(|&a| a == 255) {
        None
    } else {
        let picture = Picture::new(width, height, ChromaSampling::Monochrome, &[&alpha])?;
        let qindex = if lossless {
            0
        } else {
            alpha_qindex(options.quality)
        };
        Some(av1::encode(&picture, qindex))
    };
    let mut items = vec![Item {
        bitstream: &color_stream,
        color: Some((description, true)),
    }];
    if let Some(stream) = &alpha_stream {
        items.push(Item {
            bitstream: stream,
            color: None,
        });
    }
    Ok(container::write((width, height), &items))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualities_map_as_libavif_maps_them() {
        assert_eq!(color_qindex(75), 76);
        assert_eq!(color_qindex(0), 255);
        assert_eq!(color_qindex(60), 120);
        assert_eq!(alpha_qindex(75), 64);
        assert_eq!(alpha_qindex(0), 255);
        assert_eq!(alpha_qindex(99), 4);
    }

    #[test]
    fn inputs_are_checked() {
        let options = AvifOptions::default();
        assert_eq!(
            encode_rgba8(0, 1, &[], &options),
            Err(AvifError::Size {
                width: 0,
                height: 1
            })
        );
        let huge = encode_rgba8(MAX_SIDE + 1, 1, &[], &options);
        assert!(matches!(huge, Err(AvifError::Size { .. })));
        assert_eq!(
            encode_rgba8(1, 1, &[0; 3], &options),
            Err(AvifError::DataLength)
        );
        let bad = AvifOptions {
            quality: 101,
            ..options
        };
        assert_eq!(
            encode_rgba8(1, 1, &[0; 4], &bad),
            Err(AvifError::Quality(101))
        );
    }

    #[test]
    fn alpha_is_stored_only_when_some_pixel_is_translucent() {
        let options = AvifOptions::default();
        let opaque = encode_rgba8(2, 2, &[90, 20, 30, 255].repeat(4), &options).unwrap();
        let translucent = encode_rgba8(2, 2, &[90, 20, 30, 254].repeat(4), &options).unwrap();
        let has_alpha = |file: &[u8]| file.windows(4).any(|w| w == b"auxC");
        assert!(!has_alpha(&opaque));
        assert!(has_alpha(&translucent));
    }
}
