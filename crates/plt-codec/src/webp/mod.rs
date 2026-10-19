//! WebP files.
//!
//! [`decode`] reads a file as Pillow opens it (through libwebp's
//! `WebPAnimDecoder`): the first frame on its canvas, which is RGB, or
//! RGBA when the file declares alpha. Lossless (VP8L) bitstreams decode
//! exactly; lossy (VP8) ones, with their `ALPH` alpha planes, decode to
//! the pixels libwebp 1.6 produces, its arithmetic and "fancy" chroma
//! upsampling included.

mod alpha;
mod container;
mod error;
mod lossless;
mod lossy;

pub use error::DecodeError;

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
