//! Reading the first frame of a GIF file as Pillow opens it
//! (`GifImagePlugin` and `GifDecode.c`): mode `P` with the frame's
//! palette (its local table, else the global one) and the transparent
//! index of its graphic control extension, or mode `L` when the palette
//! is missing or is the identity grey ramp. The frame is drawn on a
//! canvas of the logical screen (enlarged to hold the frame) filled with
//! the transparent index, or index 0.

use std::fmt;

use super::lzw::{self, Finish, LzwError};
use super::{EXTENSION, IMAGE, TRAILER};
use crate::raster::{self, Palette, Pixels, Raster};

/// At most this many pixels per byte of input: an LZW code of 12 bits
/// stands for at most 4095 indices, so valid images stay well below.
const MAX_PIXELS_PER_BYTE: u64 = 4096;
/// The graphic control extension's label.
const GRAPHIC_CONTROL: u8 = 0xF9;
/// The comment extension's label.
const COMMENT: u8 = 0xFE;
/// The application extension's label.
const APPLICATION: u8 = 0xFF;

/// Why a GIF file could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The data does not start with a GIF signature.
    NotGif,
    /// The data ends inside a structure.
    Truncated,
    /// The file holds no image.
    NoImage,
    /// A graphic control extension too short for its fields.
    MalformedExtension,
    /// The frame, or the canvas, has no pixels.
    EmptyImage,
    /// The canvas has more pixels than the input can describe, or than
    /// Pillow accepts.
    TooLarge {
        /// The canvas width.
        width: u32,
        /// The canvas height.
        height: u32,
    },
    /// The LZW minimum code size is above 12.
    InvalidCodeSize(u8),
    /// The image data holds an invalid LZW code.
    InvalidCode,
    /// The image data ends before the frame is complete.
    Incomplete,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotGif => f.write_str("not a GIF file"),
            Self::Truncated => f.write_str("the GIF data ends early"),
            Self::NoImage => f.write_str("the GIF file holds no image"),
            Self::MalformedExtension => f.write_str("malformed GIF graphic control extension"),
            Self::EmptyImage => f.write_str("the GIF image has no pixels"),
            Self::TooLarge { width, height } => {
                write!(f, "a {width} × {height} GIF image is too large to decode")
            }
            Self::InvalidCodeSize(bits) => write!(f, "invalid GIF LZW code size {bits}"),
            Self::InvalidCode => f.write_str("corrupt GIF image data"),
            Self::Incomplete => f.write_str("the GIF image data ends before the image does"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// A position in the file.
#[derive(Debug)]
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let bytes = self
            .data
            .get(self.pos..self.pos + n)
            .ok_or(DecodeError::Truncated)?;
        self.pos += n;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// The next data sub-block as Pillow's `GifImageFile.data` reads it:
    /// `None` at a block terminator or the end of the data, and possibly
    /// short when the data ends inside the block.
    fn sub_block(&mut self) -> Option<&'a [u8]> {
        let len = usize::from(*self.data.get(self.pos)?);
        self.pos += 1;
        if len == 0 {
            return None;
        }
        let end = (self.pos + len).min(self.data.len());
        let block = &self.data[self.pos..end];
        self.pos = end;
        Some(block)
    }

    /// Skips sub-blocks up to the next terminator, or one that is empty
    /// because the data ended.
    fn skip_sub_blocks(&mut self) {
        while self.sub_block().is_some_and(|block| !block.is_empty()) {}
    }

    /// A colour table of `2^bits` entries, or `None` when it is the
    /// identity grey ramp, which Pillow does not treat as a palette.
    fn color_table(&mut self, bits: u8) -> Result<Option<Vec<[u8; 3]>>, DecodeError> {
        let table = self.bytes(3 << bits)?;
        let colors = table.as_chunks::<3>().0.to_vec();
        let ramp = (0..=255).zip(&colors).all(|(i, &c)| c == [i; 3]);
        Ok((!ramp).then_some(colors))
    }
}

/// The bytes of image data, read from consecutive sub-blocks as Pillow's
/// decoder reads them: a block's bytes are used only once it is whole,
/// and a zero-length block is just an empty block.
struct SubBlockBytes<'a> {
    data: &'a [u8],
    pos: usize,
    left: usize,
}

impl Iterator for SubBlockBytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        while self.left == 0 {
            let len = usize::from(*self.data.get(self.pos)?);
            if self.data.len() - self.pos - 1 < len {
                return None;
            }
            self.pos += 1;
            self.left = len;
        }
        self.left -= 1;
        self.pos += 1;
        Some(self.data[self.pos - 1])
    }
}

/// The frame's rectangle on the canvas and the order its rows arrive in.
struct Frame {
    canvas_width: usize,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    /// The interlace pass (1 to 3 still to come after the current one)
    /// or 0 when the rows are sequential or in the last pass.
    pass: u8,
    step: usize,
    x: usize,
    y: usize,
}

impl Frame {
    /// Writes the next index; returns `true` once every row is written
    /// (Pillow's `NEWLINE`).
    fn put(&mut self, canvas: &mut [u8], index: u8) -> bool {
        canvas[(self.top + self.y) * self.canvas_width + self.left + self.x] = index;
        self.x += 1;
        if self.x < self.width {
            return false;
        }
        self.x = 0;
        self.y += self.step;
        while self.y >= self.height {
            (self.y, self.step, self.pass) = match self.pass {
                1 => (4, 8, 2),
                2 => (2, 4, 3),
                3 => (1, 2, 0),
                _ => return true,
            };
        }
        false
    }
}

/// Decodes the first frame of a GIF file into the image Pillow opens:
/// palette indices with the frame's palette and transparent index, or
/// grey levels when there is no palette or it is the identity ramp.
///
/// # Errors
///
/// When the data is not a GIF file, has no image, is malformed or
/// truncated, or claims more pixels than its size allows.
pub fn decode(data: &[u8]) -> Result<Raster, DecodeError> {
    let mut reader = Reader { data, pos: 0 };
    let header = reader.bytes(6).map_err(|_| DecodeError::NotGif)?;
    if header != b"GIF87a" && header != b"GIF89a" {
        return Err(DecodeError::NotGif);
    }
    let (screen_width, screen_height) = (reader.u16()?, reader.u16()?);
    let flags = reader.byte()?;
    reader.bytes(2)?;
    let global = if flags & 0x80 != 0 {
        reader.color_table((flags & 7) + 1)?
    } else {
        None
    };
    let transparent = read_extensions(&mut reader)?;
    let (left, top) = (reader.u16()?, reader.u16()?);
    let (width, height) = (reader.u16()?, reader.u16()?);
    let flags = reader.byte()?;
    let palette = if flags & 0x80 != 0 {
        reader.color_table((flags & 7) + 1)?
    } else {
        global
    };
    let min_code_size = reader.byte()?;
    let canvas_width = u32::from(screen_width).max(u32::from(left) + u32::from(width));
    let canvas_height = u32::from(screen_height).max(u32::from(top) + u32::from(height));
    if width == 0 || height == 0 {
        return Err(DecodeError::EmptyImage);
    }
    if !raster::fits(canvas_width, canvas_height, data.len(), MAX_PIXELS_PER_BYTE) {
        return Err(DecodeError::TooLarge {
            width: canvas_width,
            height: canvas_height,
        });
    }
    let mut canvas = vec![transparent.unwrap_or(0); canvas_width as usize * canvas_height as usize];
    let interlaced = flags & 0x40 != 0;
    let mut frame = Frame {
        canvas_width: canvas_width as usize,
        left: usize::from(left),
        top: usize::from(top),
        width: usize::from(width),
        height: usize::from(height),
        pass: u8::from(interlaced),
        step: if interlaced { 8 } else { 1 },
        x: 0,
        y: 0,
    };
    let bytes = SubBlockBytes {
        data,
        pos: reader.pos,
        left: 0,
    };
    let finish = lzw::decompress(bytes, min_code_size, |index| frame.put(&mut canvas, index))
        .map_err(|err| match err {
            LzwError::InvalidCodeSize => DecodeError::InvalidCodeSize(min_code_size),
            LzwError::InvalidCode => DecodeError::InvalidCode,
            LzwError::Truncated => DecodeError::Truncated,
        })?;
    if finish == Finish::EndCode {
        return Err(DecodeError::Incomplete);
    }
    let pixels = match palette {
        Some(colors) => Pixels::Indexed {
            palette: Palette::new(colors, transparent),
            indices: canvas,
        },
        None => Pixels::Gray(canvas),
    };
    Ok(Raster::new(canvas_width, canvas_height, pixels))
}

/// Skips the blocks before the first image descriptor as Pillow's
/// `GifImageFile._seek` does, returning the transparent index of the last
/// graphic control extension that set one. Bytes that start no block are
/// skipped; after an extension's first sub-block (comments excepted, and
/// the loop count of a NETSCAPE2.0 extension read too), sub-blocks are
/// skipped up to a terminator.
fn read_extensions(reader: &mut Reader<'_>) -> Result<Option<u8>, DecodeError> {
    let mut transparent = None;
    loop {
        match reader.byte().map_err(|_| DecodeError::NoImage)? {
            IMAGE => return Ok(transparent),
            TRAILER => return Err(DecodeError::NoImage),
            EXTENSION => {
                let label = reader.byte()?;
                let block = reader.sub_block();
                match (label, block) {
                    (GRAPHIC_CONTROL, Some(block)) => {
                        let [flags, _, _, rest @ ..] = block else {
                            return Err(DecodeError::MalformedExtension);
                        };
                        if flags & 1 == 1 {
                            let index = rest.first().ok_or(DecodeError::MalformedExtension)?;
                            transparent = Some(*index);
                        }
                    }
                    (COMMENT, first) => {
                        if first.is_some_and(|block| !block.is_empty()) {
                            reader.skip_sub_blocks();
                        }
                        continue;
                    }
                    (APPLICATION, Some(block)) if block.starts_with(b"NETSCAPE2.0") => {
                        reader.sub_block();
                    }
                    _ => {}
                }
                reader.skip_sub_blocks();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gif::{GifEncoder, GifFrame};

    /// A GIF of `pixels` (RGBA) made by the encoder.
    fn encoded(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let frames = [GifFrame { pixels, delay: 1 }];
        GifEncoder::new().encode(width, height, &frames).unwrap()
    }

    #[test]
    fn encoder_output_decodes_to_its_colours() {
        let pixels: Vec<u8> = (0..6 * 5)
            .flat_map(|i| [(i * 40 % 256) as u8, 10, 200, 255])
            .collect();
        let image = decode(&encoded(6, 5, &pixels)).unwrap();
        assert_eq!((image.width(), image.height()), (6, 5));
        let rgba: Vec<u8> = image.to_rgba8().into_iter().flatten().collect();
        assert_eq!(rgba, pixels);
    }

    #[test]
    fn transparency_becomes_alpha_zero() {
        let pixels = [[0, 0, 0, 0], [255, 0, 0, 255]].concat();
        let image = decode(&encoded(2, 1, &pixels)).unwrap();
        let Pixels::Indexed { palette, indices } = image.pixels() else {
            panic!("a palette image");
        };
        assert_eq!(palette.rgba(indices[0])[3], 0);
        assert_eq!(palette.rgba(indices[1]), [255, 0, 0, 255]);
    }

    /// A hand-made GIF: a 4×4 screen, an interlaced 2×4 frame at (1, 0)
    /// whose rows hold indices 0, 1, 2, 3.
    fn interlaced() -> Vec<u8> {
        let mut gif = b"GIF89a\x04\x00\x04\x00\x80\x00\x00".to_vec();
        gif.extend_from_slice(&[0, 0, 0, 10, 10, 10]);
        gif.extend_from_slice(&[EXTENSION, GRAPHIC_CONTROL, 4, 1, 0, 0, 1, 0]);
        gif.extend_from_slice(&[IMAGE, 1, 0, 0, 0, 2, 0, 4, 0, 0x40, 2]);
        // Rows arrive as 0, 2, 1, 3: indices 0 0, 2 2, 1 1, 3 3.
        let indices = [0, 0, 2, 2, 1, 1, 3, 3];
        let packed = lzw::compress(&indices, 2);
        gif.push(u8::try_from(packed.len()).unwrap());
        gif.extend_from_slice(&packed);
        gif.extend_from_slice(&[0, TRAILER]);
        gif
    }

    #[test]
    fn interlaced_frames_land_on_a_transparent_canvas() {
        let image = decode(&interlaced()).unwrap();
        let Pixels::Indexed { palette, indices } = image.pixels() else {
            panic!("a palette image");
        };
        assert_eq!(palette.transparent(), Some(1));
        assert_eq!(
            indices,
            &[1, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 1, 1, 3, 3, 1],
            "rows 0 to 3 of the frame, the rest the transparent index"
        );
    }

    #[test]
    fn grey_ramps_are_not_palettes() {
        let mut gif = interlaced();
        gif[13..19].copy_from_slice(&[0, 0, 0, 1, 1, 1]);
        assert!(matches!(decode(&gif).unwrap().pixels(), Pixels::Gray(_)));
    }

    #[test]
    fn bad_files_are_rejected() {
        assert_eq!(decode(b"GIF8"), Err(DecodeError::NotGif));
        assert_eq!(decode(b"PNG89a\0\0\0\0\0\0\0"), Err(DecodeError::NotGif));
        assert_eq!(
            decode(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"),
            Err(DecodeError::NoImage)
        );
        let mut huge = b"GIF89a\xff\xff\xff\xff\x00\x00\x00".to_vec();
        huge.extend_from_slice(&[IMAGE, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x4C, 0x01, 0]);
        assert!(matches!(decode(&huge), Err(DecodeError::TooLarge { .. })));
        let mut empty = interlaced();
        empty[32] = 0;
        assert_eq!(decode(&empty), Err(DecodeError::EmptyImage));
        let mut code_size = interlaced();
        code_size[37] = 13;
        assert_eq!(decode(&code_size), Err(DecodeError::InvalidCodeSize(13)));
    }

    #[test]
    fn truncations_and_corruptions_never_panic() {
        let gif = interlaced();
        // The image is complete once its last data block is in.
        let complete = gif.len() - 2;
        for len in 0..complete {
            assert!(decode(&gif[..len]).is_err(), "truncated to {len}");
        }
        assert!(decode(&gif[..complete]).is_ok());
        let mut rng = crate::test_util::Rng::new(11);
        let big = encoded(
            23,
            17,
            &(0..23 * 17 * 4)
                .map(|i| (i * 7 % 251) as u8)
                .collect::<Vec<_>>(),
        );
        for _ in 0..3000 {
            let mut corrupt = big.clone();
            for _ in 0..=rng.below(4) {
                let at = rng.below(corrupt.len());
                corrupt[at] ^= 1 << rng.below(8);
            }
            let _ = decode(&corrupt);
        }
    }
}
