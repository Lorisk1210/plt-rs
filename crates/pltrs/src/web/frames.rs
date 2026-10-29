//! The PNG frames sent to browsers: whole canvases, or only the pixels
//! that changed since the previous frame (matplotlib's
//! `FigureCanvasWebAggCore.get_diff_image`).

use std::fmt;

use pltrs_codec::CompressionLevel;
use pltrs_codec::png::{EncodeError, Encoder, ImageRef};

use super::protocol::ImageMode;

/// Encodes successive canvases as full or difference frames.
#[derive(Default)]
pub(crate) struct FrameEncoder {
    /// The pixels of the previous frame.
    last: Vec<u8>,
    /// The size of the previous frame.
    size: (u32, u32),
    /// Whether the next frame must be whole.
    force_full: bool,
}

impl fmt::Debug for FrameEncoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameEncoder")
            .field("size", &self.size)
            .field("force_full", &self.force_full)
            .finish_non_exhaustive()
    }
}

impl FrameEncoder {
    /// An encoder whose first frame is whole.
    pub(crate) fn new() -> Self {
        Self {
            force_full: true,
            ..Self::default()
        }
    }

    /// Makes the next frame whole (a client joined or rescaled).
    pub(crate) fn force_full(&mut self) {
        self.force_full = true;
    }

    /// The frame for RGBA `pixels` of `width × height`: whole when forced,
    /// after a size change or when a pixel is not opaque (a difference
    /// cannot overlay transparency); otherwise the changed pixels with the
    /// others transparent.
    pub(crate) fn encode(
        &mut self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<(ImageMode, Vec<u8>), EncodeError> {
        let whole = self.force_full
            || self.size != (width, height)
            || self.last.len() != pixels.len()
            || pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] != 255);
        let (mode, png) = if whole {
            (ImageMode::Full, encode_png(width, height, pixels)?)
        } else {
            let mut diff = vec![0; pixels.len()];
            let pairs = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .zip(self.last.as_chunks::<4>().0);
            for (out, (new, old)) in diff.as_chunks_mut::<4>().0.iter_mut().zip(pairs) {
                if new != old {
                    *out = *new;
                }
            }
            (ImageMode::Diff, encode_png(width, height, &diff)?)
        };
        self.last.clear();
        self.last.extend_from_slice(pixels);
        self.size = (width, height);
        self.force_full = false;
        Ok((mode, png))
    }
}

/// RGBA pixels as a PNG, compressed for speed.
fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, EncodeError> {
    Encoder::new()
        .compression(CompressionLevel::FASTEST)
        .encode(ImageRef::rgba8(width, height, pixels)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(png: &[u8]) -> Vec<u8> {
        pltrs_codec::png::decode(png)
            .expect("a valid PNG")
            .to_rgba8()
    }

    #[test]
    fn differences_follow_whole_frames() {
        let mut encoder = FrameEncoder::new();
        let mut pixels = vec![255; 2 * 2 * 4];
        let (mode, png) = encoder.encode(2, 2, &pixels).unwrap();
        assert_eq!(mode, ImageMode::Full);
        assert_eq!(decode(&png), pixels);
        pixels[4..8].copy_from_slice(&[1, 2, 3, 255]);
        let (mode, png) = encoder.encode(2, 2, &pixels).unwrap();
        assert_eq!(mode, ImageMode::Diff);
        let mut expected = vec![0; 16];
        expected[4..8].copy_from_slice(&[1, 2, 3, 255]);
        assert_eq!(decode(&png), expected);
        encoder.force_full();
        assert_eq!(encoder.encode(2, 2, &pixels).unwrap().0, ImageMode::Full);
    }

    #[test]
    fn size_changes_and_transparency_force_whole_frames() {
        let mut encoder = FrameEncoder::new();
        encoder.encode(1, 1, &[255; 4]).unwrap();
        assert_eq!(encoder.encode(2, 1, &[255; 8]).unwrap().0, ImageMode::Full);
        assert_eq!(encoder.encode(2, 1, &[255; 8]).unwrap().0, ImageMode::Diff);
        let translucent = [255, 255, 255, 128, 255, 255, 255, 255];
        assert_eq!(
            encoder.encode(2, 1, &translucent).unwrap().0,
            ImageMode::Full
        );
    }
}
