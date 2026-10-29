//! TIFF decoding as Pillow does it: the first image file directory
//! (`ifd`) says how the pixels are laid out (`layout`); strips or tiles
//! are read — uncompressed ones as Pillow reads them itself, compressed
//! ones as libtiff decodes them for it (`compression`, `lzw`) — and
//! unpacked into the Pillow mode (`unpack`), and the image is turned
//! upright (`orientation`).

mod compression;
mod error;
mod ifd;
mod layout;
mod lzw;
mod orientation;
mod unpack;

#[cfg(test)]
mod tests;

pub use error::DecodeError;

use self::compression::Compression;
use self::ifd::{ByteOrder, Directory};
use self::layout::{Description, Layout};
use self::orientation::Canvas;
use crate::raster::{self, Palette, Pixels, Raster};

const IMAGE_WIDTH: u16 = 256;
const IMAGE_LENGTH: u16 = 257;
const BITS_PER_SAMPLE: u16 = 258;
const COMPRESSION: u16 = 259;
const PHOTOMETRIC: u16 = 262;
const FILL_ORDER: u16 = 266;
const STRIP_OFFSETS: u16 = 273;
const ORIENTATION: u16 = 274;
const SAMPLES_PER_PIXEL: u16 = 277;
const ROWS_PER_STRIP: u16 = 278;
const STRIP_BYTE_COUNTS: u16 = 279;
const PLANAR_CONFIGURATION: u16 = 284;
const PREDICTOR: u16 = 317;
const COLOR_MAP: u16 = 320;
const TILE_WIDTH: u16 = 322;
const TILE_LENGTH: u16 = 323;
const TILE_OFFSETS: u16 = 324;
const TILE_BYTE_COUNTS: u16 = 325;
const EXTRA_SAMPLES: u16 = 338;
const SAMPLE_FORMAT: u16 = 339;
/// A Windows Media Photo tag, which makes Pillow refuse the file.
const WINDOWS_MEDIA_PHOTO: u16 = 0xBC01;
/// The most samples per pixel Pillow reads.
const MAX_SAMPLES: u64 = 6;

/// Decodes the first image of a TIFF file into the image Pillow opens:
/// bilevel, grey (8- or 16-bit), grey with alpha, RGB, RGBA, palette or
/// CMYK pixels, turned as the `Orientation` tag says.
///
/// # Errors
///
/// When the data is not a TIFF file, describes a layout or compression
/// that is not supported, is truncated or corrupt, or claims more pixels
/// than its size allows.
pub fn decode(data: &[u8]) -> Result<Raster, DecodeError> {
    let ifd = Directory::read(data)?;
    if ifd.has(WINDOWS_MEDIA_PHOTO) {
        return Err(DecodeError::UnsupportedLayout);
    }
    let image = Image::read(&ifd)?;
    let palette = image.palette(&ifd)?;
    let mut canvas = image.canvas(data.len())?;
    if ifd.has(TILE_OFFSETS) && !ifd.has(STRIP_OFFSETS) {
        image.read_tiles(&ifd, data, &mut canvas)?;
    } else {
        image.read_strips(&ifd, data, &mut canvas)?;
    }
    let orientation = ifd.number(ORIENTATION).ok().flatten().unwrap_or(1);
    let canvas = canvas.oriented(orientation);
    let side = |n: usize| u32::try_from(n).expect("checked against MAX_PIXELS");
    let (width, height) = (side(canvas.width), side(canvas.height));
    Ok(Raster::new(
        width,
        height,
        pixels(image.layout, canvas.bytes, palette),
    ))
}

/// What the directory says about the image.
#[derive(Debug)]
struct Image {
    width: usize,
    height: usize,
    layout: Layout,
    order: ByteOrder,
    compression: Compression,
    fill_order: u64,
    /// The size of a sample in bytes when rows are stored as horizontal
    /// differences (predictor 2).
    differencing: Option<usize>,
}

impl Image {
    /// Reads the tags that describe the pixels, normalised as Pillow's
    /// `TiffImageFile._setup` does.
    fn read(ifd: &Directory<'_>) -> Result<Self, DecodeError> {
        let compression = Compression::from_tag(ifd.number(COMPRESSION)?.unwrap_or(1))?;
        if ifd.number(PLANAR_CONFIGURATION)? == Some(2) {
            return Err(DecodeError::UnsupportedPlanarConfiguration);
        }
        let side = |tag| -> Result<usize, DecodeError> {
            let value = ifd.number(tag)?.ok_or(DecodeError::MissingTag(tag))?;
            usize::try_from(value).map_err(|_| DecodeError::InvalidTag(tag))
        };
        let (width, height) = (side(IMAGE_WIDTH)?, side(IMAGE_LENGTH)?);
        if width == 0 || height == 0 {
            return Err(DecodeError::EmptyImage);
        }
        let mut sample_format = ifd.array(SAMPLE_FORMAT)?.unwrap_or_else(|| vec![1]);
        if sample_format.len() > 1 && sample_format.iter().all(|&f| f == sample_format[0]) {
            sample_format.truncate(1);
        }
        let mut bits = ifd.array(BITS_PER_SAMPLE)?.unwrap_or_else(|| vec![1]);
        let extra = ifd.array(EXTRA_SAMPLES)?.unwrap_or_default();
        let samples = ifd.number(SAMPLES_PER_PIXEL)?.unwrap_or(1);
        if samples > MAX_SAMPLES {
            return Err(DecodeError::UnsupportedLayout);
        }
        let samples = usize::try_from(samples).expect("checked against MAX_SAMPLES");
        if samples < bits.len() {
            bits.truncate(samples);
        } else if samples > bits.len() && bits.len() == 1 {
            bits = vec![bits[0]; samples];
        }
        if bits.len() != samples {
            return Err(DecodeError::UnsupportedLayout);
        }
        let order = ifd.order();
        let fill_order = ifd.number(FILL_ORDER)?.unwrap_or(1);
        let photometric = ifd.number(PHOTOMETRIC)?.unwrap_or(0);
        let layout = Layout::find(&Description {
            order,
            photometric,
            sample_format: &sample_format,
            fill_order,
            bits: &bits,
            extra: &extra,
        })
        .ok_or(DecodeError::UnsupportedLayout)?;
        if !Self::readable(layout, photometric, compression, fill_order) {
            return Err(DecodeError::UnsupportedLayout);
        }
        let predictor = if compression.uses_predictor() {
            ifd.number(PREDICTOR)?.unwrap_or(1)
        } else {
            1
        };
        let differencing = match (predictor, layout.sample_bytes()) {
            (1, _) => None,
            (2, Some(sample_bytes)) => Some(sample_bytes),
            _ => {
                return Err(DecodeError::UnsupportedPredictor(
                    u16::try_from(predictor).unwrap_or(u16::MAX),
                ));
            }
        };
        Ok(Self {
            width,
            height,
            layout,
            order,
            compression,
            fill_order,
            differencing,
        })
    }

    /// Whether Pillow can read a layout it knows with this compression
    /// and fill order: libtiff, which reads compressed data, refuses
    /// one-sample YCbCr, and Pillow has no unpackers for bit-reversed
    /// inverted 8-bit grey or packed palette indices, which it reads
    /// itself when uncompressed.
    fn readable(
        layout: Layout,
        photometric: u64,
        compression: Compression,
        fill_order: u64,
    ) -> bool {
        const YCBCR: u64 = 6;
        if compression != Compression::None {
            return photometric != YCBCR;
        }
        let missing_unpacker = matches!(
            layout,
            Layout::Gray {
                bits: 8,
                white_is_zero: true
            } | Layout::Palette { bits: 1 | 2 | 4 }
        );
        !(fill_order == 2 && missing_unpacker)
    }

    /// The blank canvas of the image, once its size is known to be
    /// within Pillow's limit and within what `input_len` bytes of data
    /// can hold.
    fn canvas(&self, input_len: usize) -> Result<Canvas, DecodeError> {
        let stored = self
            .row_bytes(self.width)
            .checked_mul(self.height)
            .ok_or(self.too_large())?;
        let input = u64::try_from(input_len).unwrap_or(u64::MAX);
        let max_stored = input.saturating_mul(self.compression.max_expansion());
        if !raster::fits(self.width32()?, self.height32()?, input_len, u64::MAX)
            || u64::try_from(stored).unwrap_or(u64::MAX) > max_stored
        {
            return Err(self.too_large());
        }
        let channels = self.layout.channels();
        Ok(Canvas {
            width: self.width,
            height: self.height,
            channels,
            bytes: vec![0; self.width * self.height * channels],
        })
    }

    /// The palette of a palette image, from the `ColorMap` tag: all red
    /// levels, then all green, then all blue, of which Pillow keeps the
    /// high bytes. libtiff, which reads compressed images, ignores a map
    /// without exactly three levels per index value, and then refuses an
    /// image of fewer than 8 bits.
    fn palette(&self, ifd: &Directory<'_>) -> Result<Option<Palette>, DecodeError> {
        let bits = match self.layout {
            Layout::Palette { bits } => bits,
            Layout::PaletteExtra | Layout::PaletteAlpha => 8,
            _ => return Ok(None),
        };
        let map = ifd
            .array(COLOR_MAP)?
            .ok_or(DecodeError::MissingTag(COLOR_MAP))?;
        let entries = map.len() / 3;
        let libtiff_refuses =
            self.compression != Compression::None && bits < 8 && map.len() != 3 << bits;
        if entries > 256 || libtiff_refuses {
            return Err(DecodeError::InvalidTag(COLOR_MAP));
        }
        let high = |value: u64| u8::try_from((value >> 8).min(255)).expect("clamped to a byte");
        let colors = (0..entries)
            .map(|i| {
                [
                    high(map[i]),
                    high(map[i + entries]),
                    high(map[i + 2 * entries]),
                ]
            })
            .collect();
        Ok(Some(Palette::new(colors, None)))
    }

    fn too_large(&self) -> DecodeError {
        let clamp = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        DecodeError::TooLarge {
            width: clamp(self.width),
            height: clamp(self.height),
        }
    }

    fn width32(&self) -> Result<u32, DecodeError> {
        u32::try_from(self.width).map_err(|_| self.too_large())
    }

    fn height32(&self) -> Result<u32, DecodeError> {
        u32::try_from(self.height).map_err(|_| self.too_large())
    }

    /// Bytes of a stored row of `width` pixels.
    fn row_bytes(&self, width: usize) -> usize {
        width
            .saturating_mul(self.layout.bits_per_pixel())
            .div_ceil(8)
    }

    /// Reads the strips into the canvas. Uncompressed strips are read as
    /// Pillow reads them, from their offsets alone, and a strip holding
    /// the whole image comes from the last offset; compressed ones as
    /// libtiff reads them, within their byte counts.
    fn read_strips(
        &self,
        ifd: &Directory<'_>,
        data: &[u8],
        canvas: &mut Canvas,
    ) -> Result<(), DecodeError> {
        let rows_per_strip = match ifd.number(ROWS_PER_STRIP)? {
            None => self.height,
            Some(0) => return Err(DecodeError::InvalidTag(ROWS_PER_STRIP)),
            Some(rows) => usize::try_from(rows).unwrap_or(usize::MAX).min(self.height),
        };
        let offsets = ifd
            .array(STRIP_OFFSETS)?
            .ok_or(DecodeError::MissingTag(STRIP_OFFSETS))?;
        let row_bytes = self.row_bytes(self.width);
        let strips = self.height.div_ceil(rows_per_strip);
        if self.compression == Compression::None {
            let offsets = match offsets.split_last() {
                Some((last, _)) if rows_per_strip == self.height => std::slice::from_ref(last),
                _ => &offsets[..offsets.len().min(strips)],
            };
            for (strip, &offset) in offsets.iter().enumerate() {
                let top = strip * rows_per_strip;
                let rows = rows_per_strip.min(self.height - top);
                let stored = raw_data(data, offset, rows * row_bytes)?;
                self.place(stored, row_bytes, (0, top), (self.width, rows), canvas);
            }
            return Ok(());
        }
        let counts = ifd
            .array(STRIP_BYTE_COUNTS)?
            .ok_or(DecodeError::MissingTag(STRIP_BYTE_COUNTS))?;
        for strip in 0..strips {
            let top = strip * rows_per_strip;
            let rows = rows_per_strip.min(self.height - top);
            let stored = self.decompress(
                data,
                (&offsets, &counts),
                strip,
                rows * row_bytes,
                row_bytes,
            )?;
            self.place(&stored, row_bytes, (0, top), (self.width, rows), canvas);
        }
        Ok(())
    }

    /// Reads the tiles into the canvas, cutting those on the right and
    /// bottom edges to the image.
    fn read_tiles(
        &self,
        ifd: &Directory<'_>,
        data: &[u8],
        canvas: &mut Canvas,
    ) -> Result<(), DecodeError> {
        let side = |tag| -> Result<usize, DecodeError> {
            match ifd.number(tag)? {
                Some(0) | None => Err(DecodeError::InvalidTag(tag)),
                Some(value) => usize::try_from(value).map_err(|_| DecodeError::InvalidTag(tag)),
            }
        };
        let (tile_width, tile_length) = (side(TILE_WIDTH)?, side(TILE_LENGTH)?);
        let offsets = ifd.array(TILE_OFFSETS)?.unwrap_or_default();
        let row_bytes = self.row_bytes(tile_width);
        let across = self.width.div_ceil(tile_width);
        let tiles = across.saturating_mul(self.height.div_ceil(tile_length));
        let counts = if self.compression == Compression::None {
            Vec::new()
        } else {
            ifd.array(TILE_BYTE_COUNTS)?
                .ok_or(DecodeError::MissingTag(TILE_BYTE_COUNTS))?
        };
        // Pillow reads a single tile covering the image from the last
        // offset.
        let whole = tile_width == self.width && tile_length == self.height;
        for (tile, &offset) in offsets.iter().enumerate().take(tiles) {
            let (left, top) = ((tile % across) * tile_width, (tile / across) * tile_length);
            let size = (
                tile_width.min(self.width - left),
                tile_length.min(self.height - top),
            );
            if self.compression == Compression::None {
                let offset = if whole {
                    *offsets.last().unwrap_or(&offset)
                } else {
                    offset
                };
                let need = (size.1 - 1)
                    .saturating_mul(row_bytes)
                    .saturating_add(self.row_bytes(size.0));
                let stored = raw_data(data, offset, need)?;
                self.place(stored, row_bytes, (left, top), size, canvas);
            } else {
                let expected = tile_length.saturating_mul(row_bytes);
                let stored =
                    self.decompress(data, (&offsets, &counts), tile, expected, row_bytes)?;
                self.place(&stored, row_bytes, (left, top), size, canvas);
            }
        }
        Ok(())
    }

    /// Decompresses strip or tile `index`, given the offsets and byte
    /// counts of all, into its `expected` bytes and undoes horizontal
    /// differencing over its rows of `row_bytes`. Data past the end of
    /// the file is cut off, as libtiff does.
    fn decompress(
        &self,
        data: &[u8],
        (offsets, counts): (&[u64], &[u64]),
        index: usize,
        expected: usize,
        row_bytes: usize,
    ) -> Result<Vec<u8>, DecodeError> {
        let start = offsets
            .get(index)
            .and_then(|&o| usize::try_from(o).ok())
            .filter(|&o| o < data.len())
            .ok_or(DecodeError::Truncated)?;
        let count = counts
            .get(index)
            .and_then(|&c| usize::try_from(c).ok())
            .ok_or(DecodeError::CorruptData)?;
        let mut stored = data[start..start.saturating_add(count).min(data.len())].to_vec();
        let max = u64::try_from(stored.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(self.compression.max_expansion());
        if u64::try_from(expected).unwrap_or(u64::MAX) > max {
            return Err(DecodeError::CorruptData);
        }
        if self.fill_order == 2 {
            reverse_bits(&mut stored);
        }
        let mut decoded = self.compression.decompress(&stored, expected)?;
        if let Some(sample_bytes) = self.differencing {
            let samples = self.layout.samples();
            compression::undo_differencing(
                &mut decoded,
                row_bytes,
                samples,
                sample_bytes,
                self.order,
            );
        }
        Ok(decoded)
    }

    /// Unpacks `rows` rows of `width` pixels from stored rows of
    /// `row_bytes` into the canvas at `(left, top)`.
    fn place(
        &self,
        stored: &[u8],
        row_bytes: usize,
        (left, top): (usize, usize),
        (width, rows): (usize, usize),
        canvas: &mut Canvas,
    ) {
        let channels = canvas.channels;
        let mut reversed = Vec::new();
        for row in 0..rows {
            let start = row * row_bytes;
            let mut source = &stored[start..(start + row_bytes).min(stored.len())];
            if self.fill_order == 2 && self.compression == Compression::None {
                reversed.clear();
                reversed.extend_from_slice(source);
                reverse_bits(&mut reversed);
                source = &reversed;
            }
            let at = ((top + row) * canvas.width + left) * channels;
            unpack::unpack_row(
                self.layout,
                self.order,
                source,
                &mut canvas.bytes[at..at + width * channels],
            );
        }
    }
}

/// The `need` bytes of uncompressed data at `offset`.
fn raw_data(data: &[u8], offset: u64, need: usize) -> Result<&[u8], DecodeError> {
    usize::try_from(offset)
        .ok()
        .and_then(|start| data.get(start..start.checked_add(need)?))
        .ok_or(DecodeError::Truncated)
}

/// Reverses the bits of every byte (fill order 2).
fn reverse_bits(bytes: &mut [u8]) {
    for byte in bytes {
        *byte = byte.reverse_bits();
    }
}

/// The pixels of a decoded canvas in the layout's Pillow mode.
fn pixels(layout: Layout, bytes: Vec<u8>, palette: Option<Palette>) -> Pixels {
    let palette = || palette.unwrap_or_else(|| Palette::new(Vec::new(), None));
    match layout {
        Layout::Bilevel { .. } => Pixels::Bilevel(bytes),
        Layout::Gray { .. } => Pixels::Gray(bytes),
        Layout::Gray16 => Pixels::Gray16(
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| u16::from_le_bytes(pair))
                .collect(),
        ),
        Layout::GrayAlpha => Pixels::GrayAlpha(bytes.as_chunks::<2>().0.to_vec()),
        Layout::Rgb { .. } => Pixels::Rgb(bytes.as_chunks::<3>().0.to_vec()),
        Layout::Rgba { .. } => Pixels::Rgba(bytes.as_chunks::<4>().0.to_vec()),
        Layout::Cmyk { .. } => Pixels::Cmyk(bytes.as_chunks::<4>().0.to_vec()),
        Layout::Palette { .. } | Layout::PaletteExtra => Pixels::Indexed {
            palette: palette(),
            indices: bytes,
        },
        Layout::PaletteAlpha => Pixels::IndexedAlpha {
            palette: palette(),
            pixels: bytes.as_chunks::<2>().0.to_vec(),
        },
    }
}
