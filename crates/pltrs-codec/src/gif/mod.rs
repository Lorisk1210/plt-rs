//! GIF files.
//!
//! * [`GifEncoder`] writes animated GIFs (GIF89a): RGBA frames are reduced
//!   to one shared palette of at most 256 colours, frames identical to the
//!   one before are merged into it, and opaque animations store only the
//!   rectangle that changed from frame to frame.
//! * [`decode`] reads a file's first frame as Pillow opens it: palette
//!   indices (local or global table, interlaced or not) with the
//!   transparent index, on the logical screen.

mod decode;
mod lzw;
mod palette;

use std::fmt;

pub use self::decode::{DecodeError, decode};
use self::palette::Palette;

/// One frame of an animation: RGBA pixels, row by row, and how long the
/// frame is shown in hundredths of a second.
#[derive(Clone, Copy, Debug)]
pub struct GifFrame<'a> {
    /// `width × height × 4` bytes of straight (not premultiplied) RGBA.
    pub pixels: &'a [u8],
    /// The display time in centiseconds.
    pub delay: u16,
}

/// Why frames could not be encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GifError {
    /// A side is zero or larger than GIF's 65 535 pixels.
    InvalidDimensions {
        /// The requested width.
        width: u32,
        /// The requested height.
        height: u32,
    },
    /// A frame's pixel data does not have `width × height × 4` bytes.
    DataLengthMismatch {
        /// The frame's position.
        index: usize,
        /// The bytes needed.
        expected: usize,
        /// The bytes given.
        actual: usize,
    },
    /// There were no frames.
    NoFrames,
}

impl fmt::Display for GifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => write!(
                f,
                "a GIF cannot be {width} × {height} pixels (each side must be 1 to 65535)"
            ),
            Self::DataLengthMismatch {
                index,
                expected,
                actual,
            } => write!(
                f,
                "frame {index} has {actual} bytes of pixel data, expected {expected}"
            ),
            Self::NoFrames => f.write_str("an animation needs at least one frame"),
        }
    }
}

impl std::error::Error for GifError {}

/// Configures and runs GIF encoding.
///
/// ```
/// use pltrs_codec::gif::{GifEncoder, GifFrame};
/// let black = vec![0, 0, 0, 255].repeat(16);
/// let white = vec![255; 4 * 16];
/// let frames = [
///     GifFrame { pixels: &black, delay: 10 },
///     GifFrame { pixels: &white, delay: 10 },
/// ];
/// let gif = GifEncoder::new().encode(4, 4, &frames)?;
/// assert!(gif.starts_with(b"GIF89a"));
/// # Ok::<(), pltrs_codec::gif::GifError>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GifEncoder {
    plays: u16,
}

impl GifEncoder {
    /// An encoder looping forever.
    pub fn new() -> Self {
        Self::default()
    }

    /// How often the animation plays (0, the default, loops forever).
    #[must_use]
    pub fn plays(mut self, plays: u16) -> Self {
        self.plays = plays;
        self
    }

    /// Encodes `width × height` frames into a complete GIF file.
    ///
    /// # Errors
    ///
    /// When there are no frames, a side is out of GIF's range or a frame
    /// has the wrong amount of pixel data.
    pub fn encode(
        &self,
        width: u32,
        height: u32,
        frames: &[GifFrame<'_>],
    ) -> Result<Vec<u8>, GifError> {
        let invalid = GifError::InvalidDimensions { width, height };
        let side = |n: u32| u16::try_from(n).ok().filter(|&n| n > 0);
        let (Some(columns), Some(rows)) = (side(width), side(height)) else {
            return Err(invalid);
        };
        let screen = Screen {
            width: columns,
            height: rows,
        };
        let expected = screen.pixels() * 4;
        if let Some((index, frame)) = frames
            .iter()
            .enumerate()
            .find(|(_, frame)| frame.pixels.len() != expected)
        {
            return Err(GifError::DataLengthMismatch {
                index,
                expected,
                actual: frame.pixels.len(),
            });
        }
        if frames.is_empty() {
            return Err(GifError::NoFrames);
        }
        let pixels: Vec<&[u8]> = frames.iter().map(|frame| frame.pixels).collect();
        let palette = Palette::for_frames(&pixels);
        let shown = merge_repeats(frames.iter().map(|frame| Shown {
            indices: palette.indices(frame.pixels),
            delay: frame.delay,
        }));
        let mut gif = Vec::new();
        let table_bits = write_header(&mut gif, screen, &palette);
        if self.plays != 1 {
            write_looping(&mut gif, self.plays.saturating_sub(1));
        }
        let mut previous: Option<&[u8]> = None;
        for frame in &shown {
            // Frames with transparent pixels cannot be drawn over the one
            // before, so each replaces it whole.
            let changed = match (previous, palette.transparent()) {
                (Some(previous), None) => changed_area(screen, previous, &frame.indices),
                _ => Area::whole(screen),
            };
            write_frame(&mut gif, screen, frame, changed, &palette, table_bits);
            previous = Some(&frame.indices);
        }
        gif.push(TRAILER);
        Ok(gif)
    }
}

/// Starts an extension block.
const EXTENSION: u8 = 0x21;
/// Starts an image.
const IMAGE: u8 = 0x2C;
/// Ends the file.
const TRAILER: u8 = 0x3B;

/// The size of the logical screen.
#[derive(Clone, Copy, Debug)]
struct Screen {
    width: u16,
    height: u16,
}

impl Screen {
    fn pixels(self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }
}

/// A rectangle of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Area {
    left: u16,
    top: u16,
    width: u16,
    height: u16,
}

impl Area {
    fn whole(screen: Screen) -> Self {
        Self {
            left: 0,
            top: 0,
            width: screen.width,
            height: screen.height,
        }
    }
}

/// A frame as shown: its palette indices and its (merged) delay.
struct Shown {
    indices: Vec<u8>,
    delay: u16,
}

/// The frames with each run of identical frames merged into one showing
/// for their summed delay.
fn merge_repeats(frames: impl Iterator<Item = Shown>) -> Vec<Shown> {
    let mut shown: Vec<Shown> = Vec::new();
    for frame in frames {
        match shown.last_mut() {
            Some(last) if last.indices == frame.indices => {
                last.delay = last.delay.saturating_add(frame.delay);
            }
            _ => shown.push(frame),
        }
    }
    shown
}

/// The smallest rectangle holding every pixel that differs between two
/// frames of different content.
fn changed_area(screen: Screen, before: &[u8], after: &[u8]) -> Area {
    let width = usize::from(screen.width);
    let changed_rows = || {
        before
            .chunks_exact(width)
            .zip(after.chunks_exact(width))
            .enumerate()
            .filter(|(_, (a, b))| a != b)
    };
    let top = changed_rows().next().map_or(0, |(row, _)| row);
    let bottom = changed_rows().next_back().map_or(0, |(row, _)| row);
    let (left, right) = changed_rows().fold((width, 0), |(left, right), (_, (a, b))| {
        let first = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(left);
        let last = a.iter().zip(b).rposition(|(x, y)| x != y).unwrap_or(right);
        (left.min(first), right.max(last))
    });
    let coordinate = |n: usize| u16::try_from(n).expect("inside the screen");
    Area {
        left: coordinate(left),
        top: coordinate(top),
        width: coordinate(right - left + 1),
        height: coordinate(bottom - top + 1),
    }
}

/// Writes the signature, the screen descriptor and the global colour
/// table; returns the table's size as a power of two.
fn write_header(gif: &mut Vec<u8>, screen: Screen, palette: &Palette) -> u8 {
    let colors = palette.colors();
    // A table holds 2 to 256 colours, a power of two.
    let bits = (1..=8).find(|&bits| colors.len() <= 1 << bits).unwrap_or(8);
    gif.extend_from_slice(b"GIF89a");
    gif.extend_from_slice(&screen.width.to_le_bytes());
    gif.extend_from_slice(&screen.height.to_le_bytes());
    // A global table of 8-bit colours.
    gif.extend_from_slice(&[0x80 | 0x70 | (bits - 1), 0, 0]);
    for slot in 0..1_usize << bits {
        gif.extend_from_slice(&colors.get(slot).copied().unwrap_or_default());
    }
    bits
}

/// Writes the NETSCAPE2.0 extension repeating the animation `repeats`
/// times after the first play (0 repeats forever).
fn write_looping(gif: &mut Vec<u8>, repeats: u16) {
    gif.extend_from_slice(&[EXTENSION, 0xFF, 11]);
    gif.extend_from_slice(b"NETSCAPE2.0");
    gif.extend_from_slice(&[3, 1]);
    gif.extend_from_slice(&repeats.to_le_bytes());
    gif.push(0);
}

/// Writes one frame: its graphic control extension, its image descriptor
/// and the compressed indices of `area`.
fn write_frame(
    gif: &mut Vec<u8>,
    screen: Screen,
    frame: &Shown,
    area: Area,
    palette: &Palette,
    table_bits: u8,
) {
    // Transparent frames are cleared to the background after showing (2);
    // opaque ones stay for the next to draw over (1).
    let (disposal, flags, transparent) = match palette.transparent() {
        Some(index) => (2, 1, index),
        None => (1, 0, 0),
    };
    gif.extend_from_slice(&[EXTENSION, 0xF9, 4, disposal << 2 | flags]);
    gif.extend_from_slice(&frame.delay.to_le_bytes());
    gif.extend_from_slice(&[transparent, 0]);
    gif.push(IMAGE);
    for value in [area.left, area.top, area.width, area.height] {
        gif.extend_from_slice(&value.to_le_bytes());
    }
    gif.push(0);
    let width = usize::from(screen.width);
    let (left, columns) = (usize::from(area.left), usize::from(area.width));
    let indices: Vec<u8> = frame
        .indices
        .chunks_exact(width)
        .skip(usize::from(area.top))
        .take(usize::from(area.height))
        .flat_map(|row| &row[left..left + columns])
        .copied()
        .collect();
    let min_code_size = table_bits.max(2);
    gif.push(min_code_size);
    for block in lzw::compress(&indices, min_code_size).chunks(255) {
        gif.push(u8::try_from(block.len()).expect("blocks hold at most 255 bytes"));
        gif.extend_from_slice(block);
    }
    gif.push(0);
}

#[cfg(test)]
mod tests;
