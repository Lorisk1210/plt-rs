//! Reading the chunk sequence of a PNG file into its parts, enforcing the
//! chunk ordering rules.
//!
//! Critical chunks are validated strictly. Damaged or misplaced ancillary
//! chunks (text, `pHYs`, a `tRNS` after the image data) are skipped, as
//! libpng does; unknown ancillary chunks are ignored.

use super::chunk::{Chunk, ChunkReader, ChunkType};
use super::decoded::ColorKey;
use super::error::DecodeError;
use super::header::Header;
use super::metadata::{Metadata, PhysicalDimensions};
use super::text;
use super::types::ColorType;

/// Everything needed to reconstruct the image.
#[derive(Debug)]
pub(crate) struct Parts {
    pub(crate) header: Header,
    /// Palette entries as RGBA, alpha taken from `tRNS` (opaque by default).
    pub(crate) palette: Option<Vec<[u8; 4]>>,
    pub(crate) color_key: Option<ColorKey>,
    /// The concatenated `IDAT` payloads.
    pub(crate) image_data: Vec<u8>,
    pub(crate) metadata: Metadata,
}

/// Where the reader is relative to the `IDAT` run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImageDataState {
    Before,
    Inside,
    After,
}

/// Splits `png` into its parts.
pub(crate) fn read_parts(png: &[u8]) -> Result<Parts, DecodeError> {
    let mut reader = ChunkReader::new(png)?;
    let first = reader.next_chunk()?;
    if first.kind != ChunkType::IHDR {
        return Err(DecodeError::MissingHeader);
    }
    let mut parser = Parser {
        parts: Parts {
            header: Header::parse(first.data)?,
            palette: None,
            color_key: None,
            image_data: Vec::new(),
            metadata: Metadata::default(),
        },
        state: ImageDataState::Before,
        seen_transparency: false,
    };
    loop {
        let chunk = reader.next_chunk()?;
        if chunk.kind == ChunkType::IEND {
            return parser.finish();
        }
        parser.accept(chunk)?;
    }
}

struct Parser {
    parts: Parts,
    state: ImageDataState,
    seen_transparency: bool,
}

impl Parser {
    fn accept(&mut self, chunk: Chunk<'_>) -> Result<(), DecodeError> {
        if chunk.kind == ChunkType::IDAT {
            if self.state == ImageDataState::After {
                return Err(DecodeError::ChunkOutOfOrder(ChunkType::IDAT));
            }
            self.state = ImageDataState::Inside;
            self.parts.image_data.extend_from_slice(chunk.data);
            return Ok(());
        }
        if self.state == ImageDataState::Inside {
            self.state = ImageDataState::After;
        }
        let before_image_data = self.state == ImageDataState::Before;
        match chunk.kind {
            ChunkType::IHDR => return Err(DecodeError::DuplicateChunk(ChunkType::IHDR)),
            ChunkType::PLTE => self.read_palette(chunk.data)?,
            ChunkType::TRNS if before_image_data => self.read_transparency(chunk.data)?,
            ChunkType::PHYS if self.parts.metadata.physical.is_none() => {
                self.parts.metadata.physical = PhysicalDimensions::from_bytes(chunk.data);
            }
            ChunkType::TEXT | ChunkType::ZTXT | ChunkType::ITXT => {
                self.parts
                    .metadata
                    .text
                    .extend(text::decode(chunk.kind, chunk.data));
            }
            kind if kind.is_critical() => return Err(DecodeError::UnknownCriticalChunk(kind)),
            _ => {}
        }
        Ok(())
    }

    fn read_palette(&mut self, data: &[u8]) -> Result<(), DecodeError> {
        if self.state != ImageDataState::Before || self.seen_transparency {
            return Err(DecodeError::ChunkOutOfOrder(ChunkType::PLTE));
        }
        if self.parts.palette.is_some() {
            return Err(DecodeError::DuplicateChunk(ChunkType::PLTE));
        }
        if matches!(
            self.parts.header.format.color,
            ColorType::Gray | ColorType::GrayAlpha
        ) {
            return Err(DecodeError::UnexpectedPalette);
        }
        let (entries, rest) = data.as_chunks::<3>();
        if entries.is_empty() || entries.len() > 256 || !rest.is_empty() {
            return Err(DecodeError::InvalidPalette { length: data.len() });
        }
        self.parts.palette = Some(entries.iter().map(|&[r, g, b]| [r, g, b, 255]).collect());
        Ok(())
    }

    fn read_transparency(&mut self, data: &[u8]) -> Result<(), DecodeError> {
        if self.seen_transparency {
            return Err(DecodeError::DuplicateChunk(ChunkType::TRNS));
        }
        self.seen_transparency = true;
        let samples = |data: &[u8]| -> Vec<u16> {
            let (pairs, _) = data.as_chunks::<2>();
            pairs.iter().map(|&pair| u16::from_be_bytes(pair)).collect()
        };
        match (self.parts.header.format.color, &mut self.parts.palette) {
            (ColorType::Gray, _) if data.len() == 2 => {
                self.parts.color_key = Some(ColorKey::Gray(samples(data)[0]));
            }
            (ColorType::Rgb, _) if data.len() == 6 => {
                let rgb = samples(data);
                self.parts.color_key = Some(ColorKey::Rgb([rgb[0], rgb[1], rgb[2]]));
            }
            (ColorType::Indexed, Some(palette)) if data.len() <= palette.len() => {
                for (entry, &alpha) in palette.iter_mut().zip(data) {
                    entry[3] = alpha;
                }
            }
            _ => return Err(DecodeError::InvalidTransparency),
        }
        Ok(())
    }

    fn finish(self) -> Result<Parts, DecodeError> {
        if self.state == ImageDataState::Before {
            return Err(DecodeError::MissingImageData);
        }
        if self.parts.header.format.color == ColorType::Indexed && self.parts.palette.is_none() {
            return Err(DecodeError::MissingPalette);
        }
        Ok(self.parts)
    }
}
