//! Conversion between RGBA8 frames and the server's `ZPixmap` image
//! format, and splitting of large images into requests the server accepts.

use super::setup::{Format, ImageByteOrder, Visual};
use crate::error::Error;
use crate::frame::{Frame, OwnedFrame};
use crate::geometry::{PhysicalRect, PhysicalSize};

/// One colour channel of a `TrueColor` visual.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Channel {
    shift: u32,
    /// Largest channel value (`2^bits - 1`).
    max: u32,
    /// Pixel bits for each 8-bit intensity.
    encode: Box<[u32; 256]>,
}

impl Channel {
    /// Describes the channel occupying `mask`, which must be one
    /// contiguous run of 1 to 16 bits.
    fn new(mask: u32) -> Option<Self> {
        if mask == 0 {
            return None;
        }
        let shift = mask.trailing_zeros();
        let bits = (mask >> shift).trailing_ones();
        if bits > 16 || mask >> shift != (1 << bits) - 1 {
            return None;
        }
        let max = (1 << bits) - 1;
        let mut encode = Box::new([0; 256]);
        for (value, slot) in (0_u32..).zip(encode.iter_mut()) {
            *slot = ((value * max + 127) / 255) << shift;
        }
        Some(Self { shift, max, encode })
    }

    fn decode(&self, pixel: u32) -> u8 {
        let value = (pixel >> self.shift) & self.max;
        ((value * 255 + self.max / 2) / self.max) as u8
    }
}

/// How the server stores pixels of the window visual in images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PixelFormat {
    depth: u8,
    bytes_per_pixel: usize,
    /// Rows are padded to a multiple of this many bytes.
    scanline_pad: usize,
    msb_first: bool,
    red: Channel,
    green: Channel,
    blue: Channel,
}

impl PixelFormat {
    /// The format of `visual` stored as `format` in `byte_order`.
    pub(crate) fn new(
        format: Format,
        visual: &Visual,
        byte_order: ImageByteOrder,
    ) -> Result<Self, Error> {
        let bytes_per_pixel = match format.bits_per_pixel {
            16 => 2,
            24 => 3,
            32 => 4,
            _ => return Err(Error::Unsupported("pixels of other than 16, 24 or 32 bits")),
        };
        let scanline_pad = match format.scanline_pad {
            8 | 16 | 32 => usize::from(format.scanline_pad / 8),
            _ => return Err(Error::Malformed("invalid scanline pad")),
        };
        let channel = |mask| Channel::new(mask).ok_or(Error::Unsupported("visual colour masks"));
        Ok(Self {
            depth: format.depth,
            bytes_per_pixel,
            scanline_pad,
            msb_first: byte_order == ImageByteOrder::MsbFirst,
            red: channel(visual.red_mask)?,
            green: channel(visual.green_mask)?,
            blue: channel(visual.blue_mask)?,
        })
    }

    /// The image depth.
    pub(crate) const fn depth(&self) -> u8 {
        self.depth
    }

    /// Bytes per row of an image `width` pixels wide.
    pub(crate) fn stride(&self, width: u32) -> usize {
        (width as usize * self.bytes_per_pixel).next_multiple_of(self.scanline_pad)
    }

    /// Appends `region` of `frame` (which must lie inside it) in this
    /// format. Alpha is dropped: windows are opaque.
    pub(crate) fn encode(&self, frame: &Frame<'_>, region: PhysicalRect, out: &mut Vec<u8>) {
        let stride = self.stride(region.width);
        let start = out.len();
        out.resize(start + stride * region.height as usize, 0);
        let (x0, x1) = (
            region.x as usize * 4,
            (region.x + region.width) as usize * 4,
        );
        for (row, y) in out[start..].chunks_exact_mut(stride).zip(region.y..) {
            let source = &frame.row(y)[x0..x1];
            let pixels = row.chunks_exact_mut(self.bytes_per_pixel);
            for (pixel, rgba) in pixels.zip(source.as_chunks::<4>().0) {
                let value = self.red.encode[usize::from(rgba[0])]
                    | self.green.encode[usize::from(rgba[1])]
                    | self.blue.encode[usize::from(rgba[2])];
                if self.msb_first {
                    pixel.copy_from_slice(&value.to_be_bytes()[4 - self.bytes_per_pixel..]);
                } else {
                    pixel.copy_from_slice(&value.to_le_bytes()[..self.bytes_per_pixel]);
                }
            }
        }
    }

    /// Converts image data of `size` in this format to opaque RGBA8.
    pub(crate) fn decode(&self, data: &[u8], size: PhysicalSize) -> Result<OwnedFrame, Error> {
        let stride = self.stride(size.width);
        let needed = stride * size.height as usize;
        let data = data
            .get(..needed)
            .ok_or(Error::Malformed("image data too short"))?;
        let mut rgba = Vec::with_capacity(size.area() as usize * 4);
        for row in data.chunks_exact(stride.max(1)) {
            for pixel in row
                .chunks_exact(self.bytes_per_pixel)
                .take(size.width as usize)
            {
                let mut bytes = [0; 4];
                let value = if self.msb_first {
                    bytes[4 - self.bytes_per_pixel..].copy_from_slice(pixel);
                    u32::from_be_bytes(bytes)
                } else {
                    bytes[..self.bytes_per_pixel].copy_from_slice(pixel);
                    u32::from_le_bytes(bytes)
                };
                rgba.extend([
                    self.red.decode(value),
                    self.green.decode(value),
                    self.blue.decode(value),
                    u8::MAX,
                ]);
            }
        }
        OwnedFrame::new(size.width, size.height, rgba)
    }

    /// Splits `region` into tiles whose image data fits in `max_bytes`,
    /// row bands first, columns only for absurdly wide regions.
    pub(crate) fn tiles(&self, region: PhysicalRect, max_bytes: usize) -> Vec<PhysicalRect> {
        let widest = (max_bytes.saturating_sub(self.scanline_pad) / self.bytes_per_pixel).max(1);
        let tile_width = region.width.min(u32::try_from(widest).unwrap_or(u32::MAX));
        let rows = (max_bytes / self.stride(tile_width).max(1)).max(1);
        let tile_height = region.height.min(u32::try_from(rows).unwrap_or(u32::MAX));
        let mut tiles = Vec::new();
        if region.is_empty() {
            return tiles;
        }
        for y in (region.y..region.y + region.height).step_by(tile_height as usize) {
            for x in (region.x..region.x + region.width).step_by(tile_width as usize) {
                let width = tile_width.min(region.x + region.width - x);
                let height = tile_height.min(region.y + region.height - y);
                tiles.push(PhysicalRect::new(x, y, width, height));
            }
        }
        tiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(
        bits_per_pixel: u8,
        depth: u8,
        masks: [u32; 3],
        order: ImageByteOrder,
    ) -> PixelFormat {
        let format = Format {
            depth,
            bits_per_pixel,
            scanline_pad: 32,
        };
        let visual = Visual {
            id: 0x21,
            class: super::super::setup::VISUAL_CLASS_TRUE_COLOR,
            red_mask: masks[0],
            green_mask: masks[1],
            blue_mask: masks[2],
        };
        PixelFormat::new(format, &visual, order).unwrap()
    }

    fn bgrx() -> PixelFormat {
        format(32, 24, [0xff_0000, 0xff00, 0xff], ImageByteOrder::LsbFirst)
    }

    fn sample() -> OwnedFrame {
        let pixels = [
            [255, 0, 0, 255],
            [0, 128, 0, 255],
            [0, 0, 255, 0],
            [10, 20, 30, 255],
        ];
        OwnedFrame::new(2, 2, pixels.concat()).unwrap()
    }

    #[test]
    fn encodes_the_common_24_bit_format() {
        let mut out = Vec::new();
        bgrx().encode(
            &sample().as_frame(),
            PhysicalRect::new(0, 0, 2, 2),
            &mut out,
        );
        assert_eq!(
            out,
            [0, 0, 255, 0, 0, 128, 0, 0, 255, 0, 0, 0, 30, 20, 10, 0]
        );
        let mut out = Vec::new();
        bgrx().encode(
            &sample().as_frame(),
            PhysicalRect::new(1, 1, 1, 1),
            &mut out,
        );
        assert_eq!(out, [30, 20, 10, 0]);
    }

    #[test]
    fn encodes_16_bit_and_msb_first_with_row_padding() {
        let rgb565 = format(16, 16, [0xf800, 0x07e0, 0x001f], ImageByteOrder::MsbFirst);
        assert_eq!(rgb565.stride(1), 4, "rows pad to 32 bits");
        let mut out = Vec::new();
        rgb565.encode(
            &sample().as_frame(),
            PhysicalRect::new(0, 0, 1, 2),
            &mut out,
        );
        assert_eq!(out, [0xf8, 0x00, 0, 0, 0x00, 0x1f, 0, 0]);
        let packed = format(24, 24, [0xff, 0xff00, 0xff_0000], ImageByteOrder::LsbFirst);
        let mut out = Vec::new();
        packed.encode(
            &sample().as_frame(),
            PhysicalRect::new(0, 1, 2, 1),
            &mut out,
        );
        assert_eq!(
            out,
            [0, 0, 255, 10, 20, 30, 0, 0],
            "RGB order, padded to 8 bytes"
        );
    }

    #[test]
    fn decode_inverts_encode() {
        for format in [
            bgrx(),
            format(16, 16, [0xf800, 0x07e0, 0x001f], ImageByteOrder::LsbFirst),
            format(
                32,
                30,
                [0x3ff0_0000, 0xffc00, 0x3ff],
                ImageByteOrder::MsbFirst,
            ),
        ] {
            let frame = sample();
            let mut data = Vec::new();
            format.encode(
                &frame.as_frame(),
                PhysicalRect::from_size(frame.size()),
                &mut data,
            );
            let decoded = format.decode(&data, frame.size()).unwrap();
            for (y, x) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let original = frame.pixel(x, y).unwrap();
                let round_trip = decoded.pixel(x, y).unwrap();
                for channel in 0..3 {
                    let error = original[channel].abs_diff(round_trip[channel]);
                    assert!(error <= 4, "{format:?} at ({x}, {y})");
                }
                assert_eq!(round_trip[3], 255);
            }
        }
        assert!(bgrx().decode(&[0; 15], PhysicalSize::new(2, 2)).is_err());
    }

    #[test]
    fn channels_of_more_than_16_bits_are_refused() {
        assert_eq!(Channel::new(0xffff).map(|c| c.max), Some(0xffff));
        for mask in [u32::MAX, 0x1_ffff, 0xffff_ff00, 0x8000_0001] {
            assert_eq!(Channel::new(mask), None, "{mask:#x}");
        }
    }

    #[test]
    fn rejects_unsupported_formats() {
        let visual = Visual {
            id: 1,
            class: 4,
            red_mask: 0xf0f,
            green_mask: 0xf0,
            blue_mask: 0xf000,
        };
        let format = Format {
            depth: 16,
            bits_per_pixel: 16,
            scanline_pad: 32,
        };
        assert!(PixelFormat::new(format, &visual, ImageByteOrder::LsbFirst).is_err());
        let format = Format {
            depth: 8,
            bits_per_pixel: 8,
            scanline_pad: 32,
        };
        assert!(PixelFormat::new(format, &visual, ImageByteOrder::LsbFirst).is_err());
    }

    #[test]
    fn tiles_cover_the_region_within_the_limit() {
        let format = bgrx();
        let region = PhysicalRect::new(5, 7, 1000, 300);
        let tiles = format.tiles(region, 262_140 - 28);
        assert_eq!(tiles.len(), 5);
        let mut covered = 0;
        for tile in &tiles {
            assert!(format.stride(tile.width) * tile.height as usize <= 262_140 - 28);
            covered += u64::from(tile.width) * u64::from(tile.height);
        }
        assert_eq!(covered, u64::from(region.width) * u64::from(region.height));
        assert_eq!(tiles[0], PhysicalRect::new(5, 7, 1000, 65));
        assert_eq!(tiles[4].y + tiles[4].height, 307);
        let narrow = format.tiles(PhysicalRect::new(0, 0, 10, 2), 16);
        assert_eq!(
            narrow.len(),
            8,
            "rows wider than the limit are split into columns"
        );
        assert_eq!(format.tiles(PhysicalRect::new(0, 0, 0, 5), 1000), []);
    }
}
