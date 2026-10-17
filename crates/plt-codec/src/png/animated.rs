//! Animated PNG (APNG) encoding: a sequence of full frames with display
//! durations, readable as a still PNG of the first frame by decoders
//! without APNG support.

use super::chunk::{ChunkType, SIGNATURE, write_chunk};
use super::encoder::compress_image;
use super::error::EncodeError;
use super::filter::FilterStrategy;
use super::header::Header;
use super::image::ImageRef;
use crate::CompressionLevel;

const ACTL: ChunkType = ChunkType(*b"acTL");
const FCTL: ChunkType = ChunkType(*b"fcTL");
const FDAT: ChunkType = ChunkType(*b"fdAT");

/// One frame of an animation: the image and how long it is shown, as a
/// fraction of a second (`delay.0 / delay.1`).
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// The frame's pixels; every frame has the size and format of the
    /// first.
    pub image: ImageRef<'a>,
    /// Numerator and denominator of the display time in seconds.
    pub delay: (u16, u16),
}

/// Configures and runs APNG encoding.
///
/// ```
/// use plt_codec::png::{AnimatedEncoder, Frame, ImageRef};
/// let black = vec![0u8; 4 * 4 * 4];
/// let white = vec![255u8; 4 * 4 * 4];
/// let frames = [
///     Frame { image: ImageRef::rgba8(4, 4, &black)?, delay: (1, 10) },
///     Frame { image: ImageRef::rgba8(4, 4, &white)?, delay: (1, 10) },
/// ];
/// let apng = AnimatedEncoder::new().encode(&frames)?;
/// assert!(apng.windows(4).any(|w| w == b"acTL"));
/// # Ok::<(), plt_codec::png::EncodeError>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimatedEncoder {
    level: CompressionLevel,
    filter: FilterStrategy,
    plays: u32,
}

impl AnimatedEncoder {
    /// An encoder looping forever with default compression.
    pub fn new() -> Self {
        Self::default()
    }

    /// How often the animation plays (0, the default, loops forever).
    #[must_use]
    pub fn plays(mut self, plays: u32) -> Self {
        self.plays = plays;
        self
    }

    /// Sets the zlib compression level.
    #[must_use]
    pub fn compression(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }

    /// Encodes the frames into a complete APNG file.
    ///
    /// # Errors
    ///
    /// When there are no frames, a frame differs in size or format from
    /// the first, or there are more than `u32::MAX` frames.
    pub fn encode(&self, frames: &[Frame<'_>]) -> Result<Vec<u8>, EncodeError> {
        let first = frames.first().ok_or(EncodeError::NoFrames)?.image;
        if let Some(index) = frames.iter().position(|frame| {
            frame.image.width() != first.width()
                || frame.image.height() != first.height()
                || frame.image.format() != first.format()
        }) {
            return Err(EncodeError::FrameMismatch { index });
        }
        let count = u32::try_from(frames.len()).map_err(|_| EncodeError::ImageTooLarge)?;
        let mut png = Vec::new();
        png.extend_from_slice(&SIGNATURE);
        let header = Header {
            width: first.width(),
            height: first.height(),
            format: first.format(),
            interlaced: false,
        };
        write_chunk(&mut png, ChunkType::IHDR, &header.to_bytes());
        let mut control = Vec::with_capacity(8);
        control.extend_from_slice(&count.to_be_bytes());
        control.extend_from_slice(&self.plays.to_be_bytes());
        write_chunk(&mut png, ACTL, &control);
        let mut sequence = 0_u32;
        let mut next = || {
            let number = sequence;
            sequence += 1;
            number
        };
        for (index, frame) in frames.iter().enumerate() {
            write_chunk(&mut png, FCTL, &frame_control(next(), frame));
            let data = compress_image(&frame.image, self.filter, self.level);
            if index == 0 {
                write_chunk(&mut png, ChunkType::IDAT, &data);
            } else {
                let mut chunk = next().to_be_bytes().to_vec();
                chunk.extend_from_slice(&data);
                write_chunk(&mut png, FDAT, &chunk);
            }
        }
        write_chunk(&mut png, ChunkType::IEND, &[]);
        Ok(png)
    }
}

/// The `fcTL` chunk of a full frame: no offset, no disposal, replacing
/// the previous frame.
fn frame_control(sequence: u32, frame: &Frame<'_>) -> Vec<u8> {
    let mut data = Vec::with_capacity(26);
    data.extend_from_slice(&sequence.to_be_bytes());
    data.extend_from_slice(&frame.image.width().to_be_bytes());
    data.extend_from_slice(&frame.image.height().to_be_bytes());
    data.extend_from_slice(&0_u32.to_be_bytes());
    data.extend_from_slice(&0_u32.to_be_bytes());
    data.extend_from_slice(&frame.delay.0.to_be_bytes());
    data.extend_from_slice(&frame.delay.1.to_be_bytes());
    data.extend_from_slice(&[0, 0]);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::png::decode;

    #[test]
    fn first_frame_decodes_as_a_still_image() {
        let red = [255u8, 0, 0, 255].repeat(6);
        let blue = [0u8, 0, 255, 255].repeat(6);
        let frames = [
            Frame {
                image: ImageRef::rgba8(3, 2, &red).unwrap(),
                delay: (1, 2),
            },
            Frame {
                image: ImageRef::rgba8(3, 2, &blue).unwrap(),
                delay: (1, 2),
            },
        ];
        let apng = AnimatedEncoder::new().plays(1).encode(&frames).unwrap();
        let still = decode(&apng).unwrap();
        assert_eq!(still.to_rgba8(), red);
        let fdat = apng.windows(4).filter(|w| *w == b"fdAT").count();
        assert_eq!(fdat, 1);
    }

    #[test]
    fn frames_must_match() {
        assert_eq!(
            AnimatedEncoder::new().encode(&[]),
            Err(EncodeError::NoFrames)
        );
        let small = vec![0u8; 4];
        let large = vec![0u8; 16];
        let frames = [
            Frame {
                image: ImageRef::rgba8(1, 1, &small).unwrap(),
                delay: (1, 1),
            },
            Frame {
                image: ImageRef::rgba8(2, 2, &large).unwrap(),
                delay: (1, 1),
            },
        ];
        assert_eq!(
            AnimatedEncoder::new().encode(&frames),
            Err(EncodeError::FrameMismatch { index: 1 })
        );
    }
}
