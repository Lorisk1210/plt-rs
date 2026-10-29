//! WebP files.
//!
//! * [`decode`] reads a file as Pillow opens it (through libwebp's
//!   `WebPAnimDecoder`): the first frame on its canvas, which is RGB, or
//!   RGBA when the file declares alpha. Lossless (VP8L) bitstreams decode
//!   exactly; lossy (VP8) ones, with their `ALPH` alpha planes, decode to
//!   the pixels libwebp 1.6 produces, its arithmetic and "fancy" chroma
//!   upsampling included.
//! * [`encode_rgba8`] writes a lossless (VP8L) file: a palette for images
//!   of at most 256 colours, otherwise prediction, then LZ77 with a colour
//!   cache and Huffman codes. Decoding it gives back the exact pixels.
//!   [`encode_animation_rgba8`] writes an animation of such frames.
//!
//! ```
//! use pltrs_codec::webp;
//! let pixels = [255, 0, 0, 255, 0, 0, 255, 128];
//! let file = webp::encode_rgba8(2, 1, &pixels)?;
//! assert_eq!(webp::decode(&file)?.to_rgba8().concat(), pixels);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod alpha;
mod container;
mod error;
mod lossless;
mod lossy;

pub use error::{DecodeError, EncodeError};

use self::container::{Bitstream, Frame};
use crate::raster::{MAX_PIXELS, Pixels, Raster};

/// Decodes the first frame of a WebP file as Pillow does.
///
/// # Errors
///
/// When the data is not a WebP file, is truncated or malformed, or holds
/// an image larger than Pillow accepts.
pub fn decode(data: &[u8]) -> Result<Raster, DecodeError> {
    let file = container::parse(data)?;
    let (width, height) = (file.width, file.height);
    check_size(width, height)?;
    let frame = decode_frame(&file.frame)?;
    let canvas = if (file.frame.width, file.frame.height) == (width, height) {
        frame
    } else {
        let mut canvas = vec![[0u8; 4]; width as usize * height as usize];
        let frame_width = file.frame.width as usize;
        for (row, pixels) in frame.chunks_exact(frame_width).enumerate() {
            let start = (file.frame.y as usize + row) * width as usize + file.frame.x as usize;
            canvas[start..start + frame_width].copy_from_slice(pixels);
        }
        canvas
    };
    let pixels = if file.has_alpha {
        Pixels::Rgba(canvas)
    } else {
        Pixels::Rgb(canvas.into_iter().map(|[r, g, b, _]| [r, g, b]).collect())
    };
    Ok(Raster::new(width, height, pixels))
}

/// The largest side of a WebP image.
const MAX_SIDE: u32 = 1 << 14;

/// Encodes `width × height` straight RGBA pixels (row by row) as a
/// lossless WebP file.
///
/// # Errors
///
/// When a side is zero or larger than 16 384 pixels, or `pixels` does not
/// hold `width × height × 4` bytes.
pub fn encode_rgba8(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, EncodeError> {
    check_dimensions(width, height)?;
    let (bitstream, _) = lossless_bitstream(width, height, pixels)?;
    Ok(container::write(*b"VP8L", &bitstream))
}

/// One frame of an animation.
#[derive(Clone, Copy, Debug)]
pub struct AnimationFrame<'a> {
    /// `width × height × 4` bytes of straight RGBA, row by row.
    pub pixels: &'a [u8],
    /// How long the frame shows, in milliseconds (at most 16 777 215).
    pub duration_ms: u32,
}

/// Encodes `frames` of `width × height` pixels as a lossless animated WebP
/// that plays `loops` times (0: forever). Each frame replaces the whole
/// canvas.
///
/// # Errors
///
/// When there are no frames, a side is zero or larger than 16 384
/// pixels, or a frame does not hold `width × height × 4` bytes.
pub fn encode_animation_rgba8(
    width: u32,
    height: u32,
    frames: &[AnimationFrame<'_>],
    loops: u16,
) -> Result<Vec<u8>, EncodeError> {
    check_dimensions(width, height)?;
    if frames.is_empty() {
        return Err(EncodeError::NoFrames);
    }
    let coded = frames
        .iter()
        .map(|frame| {
            let (bitstream, alpha) = lossless_bitstream(width, height, frame.pixels)?;
            Ok((bitstream, alpha, frame.duration_ms))
        })
        .collect::<Result<Vec<_>, EncodeError>>()?;
    let uses_alpha = coded.iter().any(|&(_, alpha, _)| alpha);
    let frames: Vec<(Vec<u8>, u32)> = coded
        .into_iter()
        .map(|(bitstream, _, duration)| (bitstream, duration))
        .collect();
    Ok(container::write_animation(
        width, height, uses_alpha, loops, &frames,
    ))
}

/// Rejects sizes WebP cannot hold.
fn check_dimensions(width: u32, height: u32) -> Result<(), EncodeError> {
    if (1..=MAX_SIDE).contains(&width) && (1..=MAX_SIDE).contains(&height) {
        Ok(())
    } else {
        Err(EncodeError::InvalidDimensions { width, height })
    }
}

/// The VP8L bitstream of RGBA `pixels`, and whether they use alpha.
fn lossless_bitstream(
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<(Vec<u8>, bool), EncodeError> {
    let expected = width as usize * height as usize * 4;
    if pixels.len() != expected {
        return Err(EncodeError::DataLengthMismatch {
            expected,
            actual: pixels.len(),
        });
    }
    let argb: Vec<u32> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| u32::from_be_bytes([a, r, g, b]))
        .collect();
    let uses_alpha = argb.iter().any(|&pixel| pixel >> 24 != 0xFF);
    Ok((
        lossless::encode(&argb, width as usize, uses_alpha),
        uses_alpha,
    ))
}

/// Rejects images larger than Pillow decodes.
fn check_size(width: u32, height: u32) -> Result<(), DecodeError> {
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        Err(DecodeError::TooLarge { width, height })
    } else {
        Ok(())
    }
}

/// The RGBA pixels of a frame.
fn decode_frame(frame: &Frame<'_>) -> Result<Vec<[u8; 4]>, DecodeError> {
    match frame.bitstream {
        Bitstream::Lossless(data) => Ok(lossless::decode(data)?
            .into_iter()
            .map(|argb| {
                let [a, r, g, b] = argb.to_be_bytes();
                [r, g, b, a]
            })
            .collect()),
        Bitstream::Lossy { vp8, alpha } => {
            check_size(frame.width, frame.height)?;
            let rgb = lossy::decode(vp8)?;
            let alpha = alpha
                .map(|data| alpha::decode(data, frame.width, frame.height))
                .transpose()?;
            Ok(match alpha {
                Some(levels) => rgb
                    .into_iter()
                    .zip(levels)
                    .map(|([r, g, b], a)| [r, g, b, a])
                    .collect(),
                None => rgb.into_iter().map(|[r, g, b]| [r, g, b, 255]).collect(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_files_decode_to_their_pixels() {
        let pixels: Vec<u8> = (0..30u8).flat_map(|i| [i * 8, 255 - i, i, 255]).collect();
        let file = encode_rgba8(6, 5, &pixels).unwrap();
        let raster = decode(&file).unwrap();
        assert!(
            matches!(raster.pixels(), Pixels::Rgb(_)),
            "opaque files open as RGB"
        );
        assert_eq!(raster.to_rgba8().concat(), pixels);
        assert_eq!(
            encode_rgba8(0, 5, &[]),
            Err(EncodeError::InvalidDimensions {
                width: 0,
                height: 5
            })
        );
        assert_eq!(
            encode_rgba8(2, 2, &[0; 15]),
            Err(EncodeError::DataLengthMismatch {
                expected: 16,
                actual: 15
            })
        );
    }

    #[test]
    fn animations_open_on_their_first_frame() {
        let first = [10u8, 20, 30, 128].repeat(12);
        let second = [200u8, 0, 0, 255].repeat(12);
        let frames = [
            AnimationFrame {
                pixels: &first,
                duration_ms: 40,
            },
            AnimationFrame {
                pixels: &second,
                duration_ms: 40,
            },
        ];
        let file = encode_animation_rgba8(4, 3, &frames, 0).unwrap();
        let raster = decode(&file).unwrap();
        assert_eq!(raster.pixels(), &Pixels::Rgba(vec![[10, 20, 30, 128]; 12]));
        assert_eq!(
            encode_animation_rgba8(4, 3, &[], 0),
            Err(EncodeError::NoFrames)
        );
    }
}
