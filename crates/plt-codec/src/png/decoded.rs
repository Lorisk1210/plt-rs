//! The result of decoding a PNG file.

use super::convert::Pixels;
use super::header::Header;
use super::metadata::Metadata;
use super::types::{BitDepth, ColorType};

/// The single colour that a `tRNS` chunk marks as fully transparent in a
/// greyscale or RGB image, as raw sample values at the image's bit depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorKey {
    /// A grey level.
    Gray(u16),
    /// Red, green and blue samples.
    Rgb([u16; 3]),
}

/// A decoded PNG image.
///
/// [`data`](Self::data) holds the pixels in PNG's own layout, de-interlaced:
/// rows top to bottom, each [`row_bytes`](Self::row_bytes) long; samples of
/// fewer than 8 bits packed most significant bits first; 16-bit samples as
/// big-endian byte pairs; indexed images as palette indices. Use
/// [`to_rgba8`](Self::to_rgba8) or [`to_rgba16`](Self::to_rgba16) for
/// uniform RGBA pixels with palette and transparency applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    header: Header,
    data: Vec<u8>,
    palette: Option<Vec<[u8; 4]>>,
    color_key: Option<ColorKey>,
    metadata: Metadata,
}

impl DecodedImage {
    pub(crate) fn new(
        header: Header,
        data: Vec<u8>,
        palette: Option<Vec<[u8; 4]>>,
        color_key: Option<ColorKey>,
        metadata: Metadata,
    ) -> Self {
        Self {
            header,
            data,
            palette,
            color_key,
            metadata,
        }
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.header.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.header.height
    }

    /// Colour type as stored in the file.
    pub const fn color_type(&self) -> ColorType {
        self.header.format.color
    }

    /// Bit depth as stored in the file.
    pub const fn bit_depth(&self) -> BitDepth {
        self.header.format.depth
    }

    /// Whether the file was Adam7-interlaced.
    pub const fn is_interlaced(&self) -> bool {
        self.header.interlaced
    }

    /// Bytes per row of [`data`](Self::data).
    pub fn row_bytes(&self) -> usize {
        self.data.len() / self.header.height as usize
    }

    /// The pixel data in PNG layout (see the type documentation).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Takes ownership of the pixel data.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// The palette of an indexed image as RGBA entries, with alpha from
    /// `tRNS` (255 where it gives none).
    pub fn palette(&self) -> Option<&[[u8; 4]]> {
        self.palette.as_deref()
    }

    /// The transparent colour of a greyscale or RGB image, if any.
    pub const fn color_key(&self) -> Option<ColorKey> {
        self.color_key
    }

    /// Resolution and text metadata.
    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// The pixels as 8-bit RGBA, 4 bytes per pixel.
    ///
    /// Grey is replicated into R, G and B; samples below 8 bits are scaled up
    /// exactly (e.g. 4-bit 15 becomes 255); 16-bit samples are rounded to the
    /// nearest 8-bit value. Alpha comes from the alpha channel, from the
    /// colour key (0 for matching pixels) or from the palette, and is 255
    /// otherwise. Palette indices beyond the palette yield opaque black.
    pub fn to_rgba8(&self) -> Vec<u8> {
        self.pixels().to_rgba8()
    }

    /// The pixels as 16-bit RGBA, 4 values per pixel; lossless for every
    /// PNG format. Lower depths are scaled up exactly (8-bit 255 becomes
    /// 65535), alpha is derived as for [`to_rgba8`](Self::to_rgba8).
    pub fn to_rgba16(&self) -> Vec<u16> {
        self.pixels().to_rgba16()
    }

    fn pixels(&self) -> Pixels<'_> {
        Pixels {
            width: self.header.width as usize,
            format: self.header.format,
            row_bytes: self.row_bytes(),
            data: &self.data,
            palette: self.palette.as_deref().unwrap_or_default(),
            color_key: self.color_key,
        }
    }
}
