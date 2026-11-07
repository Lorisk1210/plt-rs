//! The PNG encoder.

use super::chunk::{ChunkType, SIGNATURE, write_chunk};
use super::error::EncodeError;
use super::filter::{FilterStrategy, FilteredRows};
use super::header::Header;
use super::image::ImageRef;
use super::metadata::{PhysicalDimensions, TextEntry};
use super::text;
use crate::CompressionLevel;
use crate::deflate::encode::{self, Rows};
use crate::deflate::{MemoryLevel, Options, Strategy};
use pltrs_core::alloc::{self, OutOfMemory};

/// The smallest output buffer Pillow compresses into (`MAXBLOCK`); each
/// buffer becomes one `IDAT` chunk.
const MIN_BUFFER: usize = 1 << 16;

/// The filtered, zlib-compressed scanlines of `image` (the contents of
/// its `IDAT` chunks), filtered as they are compressed, and where Pillow
/// would end each chunk: rows compressed one at a time with zlib's
/// memory level 9 into buffers of 64 KiB or four bytes per pixel of a
/// row, whichever is more.
///
/// # Errors
///
/// When the memory for the compressor or the compressed scanlines cannot
/// be allocated.
pub(super) fn compress_image(
    image: &ImageRef<'_>,
    filter: FilterStrategy,
    level: CompressionLevel,
    strategy: Strategy,
) -> Result<Rows, OutOfMemory> {
    let mut rows = FilteredRows::new(
        image.data(),
        image.row_bytes(),
        image.format().filter_stride(),
        filter,
    )?;
    let options = Options::new(level)
        .strategy(strategy)
        .memory_level(MemoryLevel::MAX);
    let buffer = MIN_BUFFER.max(4 * image.width() as usize);
    encode::rows(&mut rows, options, buffer)
}

/// The pieces of a compressed image as Pillow cuts them into chunks.
pub(super) fn chunks(rows: &Rows) -> impl Iterator<Item = &[u8]> {
    let starts = std::iter::once(0).chain(rows.ends.iter().copied());
    starts
        .zip(&rows.ends)
        .map(|(start, &end)| &rows.stream[start..end])
}

/// How the `pHYs` chunk was requested.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Resolution {
    Dimensions(PhysicalDimensions),
    Dpi(f64),
}

/// Configures and runs PNG encoding.
///
/// The defaults match what matplotlib gets from Pillow: zlib level 6 and
/// [`Strategy::Filtered`] with Pillow's adaptive filtering, and no
/// ancillary chunks.
///
/// ```
/// use pltrs_codec::CompressionLevel;
/// use pltrs_codec::png::{Encoder, FilterStrategy, FilterType, ImageRef};
///
/// let pixels = vec![255u8; 64 * 48 * 4];
/// let png = Encoder::new()
///     .compression(CompressionLevel::BEST)
///     .filter(FilterStrategy::Fixed(FilterType::Up))
///     .dpi(72.0)
///     .text("Software", "pltrs")
///     .encode(ImageRef::rgba8(64, 48, &pixels)?)?;
/// assert!(png.starts_with(&pltrs_codec::png::SIGNATURE));
/// # Ok::<(), pltrs_codec::png::EncodeError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Encoder {
    level: CompressionLevel,
    strategy: Strategy,
    filter: FilterStrategy,
    resolution: Option<Resolution>,
    text: Vec<TextEntry>,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            level: CompressionLevel::DEFAULT,
            strategy: Strategy::Filtered,
            filter: FilterStrategy::Adaptive,
            resolution: None,
            text: Vec::new(),
        }
    }
}

impl Encoder {
    /// An encoder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the zlib compression level.
    #[must_use]
    pub fn compression(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets the zlib strategy (Pillow's `compress_type`;
    /// [`Strategy::Filtered`] by default).
    #[must_use]
    pub fn strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Sets how scanline filters are chosen.
    #[must_use]
    pub fn filter(mut self, strategy: FilterStrategy) -> Self {
        self.filter = strategy;
        self
    }

    /// Writes a `pHYs` chunk with these dimensions.
    #[must_use]
    pub fn physical_dimensions(mut self, dimensions: PhysicalDimensions) -> Self {
        self.resolution = Some(Resolution::Dimensions(dimensions));
        self
    }

    /// Writes a `pHYs` chunk for `dpi` dots per inch, as matplotlib does with
    /// the figure's dpi. An invalid value (see
    /// [`PhysicalDimensions::from_dpi`]) makes [`encode`](Self::encode) fail
    /// with [`EncodeError::InvalidDpi`].
    #[must_use]
    pub fn dpi(mut self, dpi: f64) -> Self {
        self.resolution = Some(Resolution::Dpi(dpi));
        self
    }

    /// Adds a text entry, written as `tEXt` when the text is Latin-1 and as
    /// `iTXt` (UTF-8) otherwise. Keywords are validated by
    /// [`encode`](Self::encode).
    #[must_use]
    pub fn text(mut self, keyword: impl Into<String>, text: impl Into<String>) -> Self {
        self.text.push(TextEntry::new(keyword, text));
        self
    }

    /// Encodes `image` into a complete PNG file.
    pub fn encode(&self, image: ImageRef<'_>) -> Result<Vec<u8>, EncodeError> {
        let physical = match self.resolution {
            None => None,
            Some(Resolution::Dimensions(dimensions)) => Some(dimensions),
            Some(Resolution::Dpi(dpi)) => {
                Some(PhysicalDimensions::from_dpi(dpi).ok_or(EncodeError::InvalidDpi(dpi))?)
            }
        };
        let text_chunks = self
            .text
            .iter()
            .map(text::encode)
            .collect::<Result<Vec<_>, _>>()?;

        let format = image.format();
        let compressed = compress_image(&image, self.filter, self.level, self.strategy)?;

        let text_len: usize = text_chunks.iter().map(|(_, data)| data.len() + 12).sum();
        let idat_len = compressed.stream.len() + 12 * compressed.ends.len();
        let mut png = alloc::try_with_capacity(idat_len + text_len + 128)?;
        png.extend_from_slice(&SIGNATURE);
        let header = Header {
            width: image.width(),
            height: image.height(),
            format,
            interlaced: false,
        };
        write_chunk(&mut png, ChunkType::IHDR, &header.to_bytes());
        // Text before the resolution, in the order Pillow writes them.
        for (kind, data) in &text_chunks {
            write_chunk(&mut png, *kind, data);
        }
        if let Some(physical) = physical {
            write_chunk(&mut png, ChunkType::PHYS, &physical.to_bytes());
        }
        for part in chunks(&compressed) {
            write_chunk(&mut png, ChunkType::IDAT, part);
        }
        write_chunk(&mut png, ChunkType::IEND, &[]);
        Ok(png)
    }
}
