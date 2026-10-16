//! Scanline reconstruction: unfiltering the decompressed stream and, for
//! Adam7-interlaced images, scattering the seven passes into place.

use super::error::DecodeError;
use super::filter::{FilterType, unfilter};
use super::header::Header;
use super::types::PixelFormat;

/// One Adam7 pass: the pixels at `(x0 + i·dx, y0 + j·dy)`.
#[derive(Clone, Copy, Debug)]
struct Pass {
    x0: usize,
    y0: usize,
    dx: usize,
    dy: usize,
}

const ADAM7: [Pass; 7] = [
    Pass {
        x0: 0,
        y0: 0,
        dx: 8,
        dy: 8,
    },
    Pass {
        x0: 4,
        y0: 0,
        dx: 8,
        dy: 8,
    },
    Pass {
        x0: 0,
        y0: 4,
        dx: 4,
        dy: 8,
    },
    Pass {
        x0: 2,
        y0: 0,
        dx: 4,
        dy: 4,
    },
    Pass {
        x0: 0,
        y0: 2,
        dx: 2,
        dy: 4,
    },
    Pass {
        x0: 1,
        y0: 0,
        dx: 2,
        dy: 2,
    },
    Pass {
        x0: 0,
        y0: 1,
        dx: 1,
        dy: 2,
    },
];

/// One run of rows in the filtered stream: the whole image, or one
/// non-empty Adam7 pass.
#[derive(Clone, Copy, Debug)]
struct SubImage {
    pass: Pass,
    width: usize,
    height: usize,
    row_bytes: usize,
    /// Bytes of filtered stream: each row plus its filter type byte.
    stream_len: usize,
}

/// How the decompressed, filtered stream of an image is laid out.
#[derive(Clone, Debug)]
pub(crate) struct StreamLayout {
    interlaced: bool,
    sub_images: Vec<SubImage>,
    len: usize,
}

impl StreamLayout {
    /// The layout for `header`, or `None` if its size overflows `usize`.
    pub(crate) fn new(header: &Header) -> Option<Self> {
        let (width, height) = (header.width as usize, header.height as usize);
        let passes: &[Pass] = if header.interlaced {
            &ADAM7
        } else {
            &[Pass {
                x0: 0,
                y0: 0,
                dx: 1,
                dy: 1,
            }]
        };
        let mut sub_images = Vec::with_capacity(passes.len());
        let mut len = 0usize;
        for &pass in passes {
            let width = width.saturating_sub(pass.x0).div_ceil(pass.dx);
            let height = height.saturating_sub(pass.y0).div_ceil(pass.dy);
            if width == 0 || height == 0 {
                continue;
            }
            let row_bytes = header.format.row_bytes(u32::try_from(width).ok()?)?;
            let stream_len = (row_bytes + 1).checked_mul(height)?;
            len = len.checked_add(stream_len)?;
            sub_images.push(SubImage {
                pass,
                width,
                height,
                row_bytes,
                stream_len,
            });
        }
        Some(Self {
            interlaced: header.interlaced,
            sub_images,
            len,
        })
    }

    /// Length of the filtered stream in bytes.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Rebuilds the pixels of an image in `format` whose rows are
    /// `image_row_bytes` long from the filtered `stream` of exactly
    /// [`len`](Self::len) bytes.
    pub(crate) fn reconstruct(
        &self,
        stream: &[u8],
        format: PixelFormat,
        image_row_bytes: usize,
        image_bytes: usize,
    ) -> Result<Vec<u8>, DecodeError> {
        debug_assert_eq!(stream.len(), self.len);
        let stride = format.filter_stride();
        let mut image = vec![0; image_bytes];
        if !self.interlaced {
            unfilter_rows(stream, &mut image, image_row_bytes, stride)?;
            return Ok(image);
        }
        let mut pass_pixels = Vec::new();
        let mut rest = stream;
        for &sub in &self.sub_images {
            let (pass_stream, tail) = rest.split_at(sub.stream_len);
            rest = tail;
            pass_pixels.clear();
            pass_pixels.resize(sub.row_bytes * sub.height, 0);
            unfilter_rows(pass_stream, &mut pass_pixels, sub.row_bytes, stride)?;
            scatter(&pass_pixels, sub, format, &mut image, image_row_bytes);
        }
        Ok(image)
    }
}

/// Unfilters consecutive rows of `row_bytes` bytes from `stream` (each
/// preceded by its filter type) into `out`.
fn unfilter_rows(
    stream: &[u8],
    out: &mut [u8],
    row_bytes: usize,
    stride: usize,
) -> Result<(), DecodeError> {
    let zero_row = vec![0; row_bytes];
    for (y, line) in stream.chunks_exact(row_bytes + 1).enumerate() {
        let filter =
            FilterType::from_byte(line[0]).ok_or(DecodeError::InvalidFilterType(line[0]))?;
        let (done, rest) = out.split_at_mut(y * row_bytes);
        let previous = if y == 0 {
            zero_row.as_slice()
        } else {
            &done[(y - 1) * row_bytes..]
        };
        let row = &mut rest[..row_bytes];
        row.copy_from_slice(&line[1..]);
        unfilter(filter, stride, previous, row);
    }
    Ok(())
}

/// Copies the pixels of one pass to their places in the full image.
fn scatter(
    pixels: &[u8],
    sub: SubImage,
    format: PixelFormat,
    image: &mut [u8],
    image_row_bytes: usize,
) {
    let bits = format.bits_per_pixel();
    let rows = image
        .chunks_exact_mut(image_row_bytes)
        .skip(sub.pass.y0)
        .step_by(sub.pass.dy);
    for (pass_row, image_row) in pixels.chunks_exact(sub.row_bytes).zip(rows) {
        let columns = (sub.pass.x0..).step_by(sub.pass.dx);
        if bits >= 8 {
            let bytes = bits / 8;
            for (pixel, x) in pass_row.chunks_exact(bytes).zip(columns) {
                image_row[x * bytes..(x + 1) * bytes].copy_from_slice(pixel);
            }
        } else {
            for (index, x) in (0..sub.width).zip(columns) {
                let value = read_bits(pass_row, index * bits, bits);
                write_bits(image_row, x * bits, bits, value);
            }
        }
    }
}

/// The `bits`-bit value starting `offset` bits into `row`, MSB first.
fn read_bits(row: &[u8], offset: usize, bits: usize) -> u8 {
    let shift = 8 - bits - offset % 8;
    (row[offset / 8] >> shift) & ((1 << bits) - 1)
}

/// ORs a `bits`-bit value into `row` at `offset` bits, MSB first.
fn write_bits(row: &mut [u8], offset: usize, bits: usize, value: u8) {
    let shift = 8 - bits - offset % 8;
    row[offset / 8] |= value << shift;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::png::types::{BitDepth, ColorType};

    fn header(width: u32, height: u32, interlaced: bool) -> Header {
        Header {
            width,
            height,
            format: PixelFormat::new(ColorType::Gray, BitDepth::Eight).unwrap(),
            interlaced,
        }
    }

    #[test]
    fn passes_cover_every_pixel_exactly_once() {
        for (width, height) in [(1, 1), (2, 3), (8, 8), (9, 17), (13, 11)] {
            let mut seen = vec![0; width * height];
            let layout = StreamLayout::new(&header(width as u32, height as u32, true)).unwrap();
            for sub in layout.sub_images {
                for j in 0..sub.height {
                    for i in 0..sub.width {
                        seen[(sub.pass.y0 + j * sub.pass.dy) * width
                            + sub.pass.x0
                            + i * sub.pass.dx] += 1;
                    }
                }
            }
            assert!(seen.iter().all(|&n| n == 1), "{width}×{height}");
        }
    }

    #[test]
    fn stream_lengths() {
        let len = |width, height, interlaced| {
            StreamLayout::new(&header(width, height, interlaced)).map(|layout| layout.len())
        };
        assert_eq!(len(5, 3, false), Some(18));
        // 1×1 interlaced: only pass 1 is non-empty.
        assert_eq!(len(1, 1, true), Some(2));
        // 8×8 interlaced: passes hold 1, 1, 2, 4, 8, 16, 32 pixels in
        // 1, 1, 1, 2, 2, 4, 4 rows.
        assert_eq!(len(8, 8, true), Some(64 + 15));
    }

    #[test]
    fn sub_byte_pixels_are_packed_msb_first() {
        let mut row = [0u8; 2];
        write_bits(&mut row, 0, 2, 0b11);
        write_bits(&mut row, 6, 2, 0b01);
        write_bits(&mut row, 8, 4, 0b1010);
        assert_eq!(row, [0b1100_0001, 0b1010_0000]);
        assert_eq!(read_bits(&row, 6, 2), 0b01);
        assert_eq!(read_bits(&row, 8, 4), 0b1010);
        assert_eq!(read_bits(&row, 7, 1), 1);
    }

    #[test]
    fn invalid_filter_types_are_reported() {
        let mut out = [0; 2];
        assert_eq!(
            unfilter_rows(&[5, 1, 2], &mut out, 2, 1),
            Err(DecodeError::InvalidFilterType(5))
        );
    }
}
