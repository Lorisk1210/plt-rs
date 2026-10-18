//! Baseline JPEG encoding of RGB images with libjpeg-turbo's defaults —
//! the settings Pillow saves with: JFIF, YCbCr with 4:2:0 chroma
//! subsampling, the accurate integer DCT, Annex K quantization tables
//! scaled by quality and the standard Huffman tables. Encoding mirrors
//! libjpeg's arithmetic, so files match Pillow's byte for byte.

mod fdct;
mod huffman;
mod planes;
mod tables;

use std::fmt;

use self::fdct::{Quantizer, forward_dct};
use self::huffman::{BitWriter, BlockCoder, CodeTable};
use self::planes::{Plane, planes};
use self::tables::{
    AC_CHROMINANCE, AC_LUMINANCE, DC_CHROMINANCE, DC_LUMINANCE, HuffmanSpec, TableSet, ZIGZAG,
    quant_table,
};

/// Settings of an encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JpegOptions {
    /// Quality from 1 (smallest) to 100 (best); 75 by default, as in
    /// Pillow.
    pub quality: u8,
    /// The resolution recorded in the JFIF header, in dots per inch.
    pub dpi: u16,
}

impl Default for JpegOptions {
    fn default() -> Self {
        Self {
            quality: 75,
            dpi: 72,
        }
    }
}

/// Why an image could not be encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JpegError {
    /// A side is zero or larger than JPEG's 65 535 pixels.
    InvalidDimensions {
        /// The requested width.
        width: u32,
        /// The requested height.
        height: u32,
    },
    /// The pixel data does not have `width × height × 3` bytes.
    DataLengthMismatch {
        /// The bytes needed.
        expected: usize,
        /// The bytes given.
        actual: usize,
    },
}

impl fmt::Display for JpegError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => write!(
                f,
                "a JPEG image cannot be {width} × {height} pixels (each side must be 1 to 65535)"
            ),
            Self::DataLengthMismatch { expected, actual } => write!(
                f,
                "the image has {actual} bytes of pixel data, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for JpegError {}

/// Start of image.
const SOI: u8 = 0xD8;
/// Application segment 0 (JFIF).
const APP0: u8 = 0xE0;
/// Quantization tables.
const DQT: u8 = 0xDB;
/// Baseline frame header.
const SOF0: u8 = 0xC0;
/// Huffman tables.
const DHT: u8 = 0xC4;
/// Start of scan.
const SOS: u8 = 0xDA;
/// End of image.
const EOI: u8 = 0xD9;

/// Encodes `width × height` RGB pixels, row by row, as a baseline JPEG
/// file.
///
/// ```
/// use plt_codec::jpeg::{JpegOptions, encode_rgb8};
/// let jpeg = encode_rgb8(2, 2, &[200; 12], &JpegOptions::default())?;
/// assert!(jpeg.starts_with(&[0xFF, 0xD8]) && jpeg.ends_with(&[0xFF, 0xD9]));
/// # Ok::<(), plt_codec::jpeg::JpegError>(())
/// ```
///
/// # Errors
///
/// When a side is out of range or the data has the wrong length.
pub fn encode_rgb8(
    width: u32,
    height: u32,
    rgb: &[u8],
    options: &JpegOptions,
) -> Result<Vec<u8>, JpegError> {
    let side = |n: u32| u16::try_from(n).ok().filter(|&n| n > 0);
    let (Some(columns), Some(rows)) = (side(width), side(height)) else {
        return Err(JpegError::InvalidDimensions { width, height });
    };
    let expected = usize::from(columns) * usize::from(rows) * 3;
    if rgb.len() != expected {
        return Err(JpegError::DataLengthMismatch {
            expected,
            actual: rgb.len(),
        });
    }
    let luma_table = quant_table(TableSet::Luminance, options.quality);
    let chroma_table = quant_table(TableSet::Chrominance, options.quality);
    let mut jpeg = Vec::new();
    jpeg.extend_from_slice(&[0xFF, SOI]);
    write_jfif(&mut jpeg, options.dpi);
    write_quant_table(&mut jpeg, TableSet::Luminance, &luma_table);
    write_quant_table(&mut jpeg, TableSet::Chrominance, &chroma_table);
    write_frame(&mut jpeg, columns, rows);
    for (class, set, spec) in [
        (0, TableSet::Luminance, &DC_LUMINANCE),
        (1, TableSet::Luminance, &AC_LUMINANCE),
        (0, TableSet::Chrominance, &DC_CHROMINANCE),
        (1, TableSet::Chrominance, &AC_CHROMINANCE),
    ] {
        write_huffman_table(&mut jpeg, class, set, spec);
    }
    write_scan_header(&mut jpeg);
    let planes = planes(usize::from(columns), usize::from(rows), rgb);
    jpeg.extend(scan_data(
        &planes,
        usize::from(columns),
        usize::from(rows),
        [Quantizer::new(&luma_table), Quantizer::new(&chroma_table)],
    ));
    jpeg.extend_from_slice(&[0xFF, EOI]);
    Ok(jpeg)
}

/// The entropy-coded MCUs of an image: per 16×16 pixels, four Y blocks,
/// one Cb and one Cr. Luminance blocks past the image's last block column
/// or row are "dummy" blocks holding only the DC term of the block before,
/// as libjpeg's `compress_data` makes them.
fn scan_data(
    planes: &[Plane; 3],
    width: usize,
    height: usize,
    [luma, chroma]: [Quantizer; 2],
) -> Vec<u8> {
    let tables = [
        CodeTable::new(&DC_LUMINANCE),
        CodeTable::new(&AC_LUMINANCE),
        CodeTable::new(&DC_CHROMINANCE),
        CodeTable::new(&AC_CHROMINANCE),
    ];
    let mut coders = [
        BlockCoder::new(&tables[0], &tables[1]),
        BlockCoder::new(&tables[2], &tables[3]),
        BlockCoder::new(&tables[2], &tables[3]),
    ];
    let (luma_columns, luma_rows) = (width.div_ceil(8), height.div_ceil(8));
    let mut out = BitWriter::default();
    for mcu_row in 0..height.div_ceil(16) {
        for mcu_column in 0..width.div_ceil(16) {
            let mut previous = [0; 64];
            for y in 0..2 {
                for x in 0..2 {
                    let (column, row) = (mcu_column * 2 + x, mcu_row * 2 + y);
                    let block = if column < luma_columns && row < luma_rows {
                        luma.quantize(&forward_dct(&planes[0].block(column, row)))
                    } else {
                        dummy(previous[0])
                    };
                    coders[0].encode(&mut out, &block);
                    previous = block;
                }
            }
            for (coder, plane) in coders[1..].iter_mut().zip(&planes[1..]) {
                let block = chroma.quantize(&forward_dct(&plane.block(mcu_column, mcu_row)));
                coder.encode(&mut out, &block);
            }
        }
    }
    out.finish()
}

/// A block with only a DC term.
fn dummy(dc: i32) -> [i32; 64] {
    let mut block = [0; 64];
    block[0] = dc;
    block
}

/// Writes a marker segment: the marker and its length-prefixed payload.
fn write_segment(jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    let length = u16::try_from(payload.len() + 2).expect("segments are short");
    jpeg.extend_from_slice(&[0xFF, marker]);
    jpeg.extend_from_slice(&length.to_be_bytes());
    jpeg.extend_from_slice(payload);
}

/// The JFIF 1.01 header with the resolution in dots per inch.
fn write_jfif(jpeg: &mut Vec<u8>, dpi: u16) {
    let mut payload = b"JFIF\0\x01\x01\x01".to_vec();
    payload.extend_from_slice(&dpi.to_be_bytes());
    payload.extend_from_slice(&dpi.to_be_bytes());
    payload.extend_from_slice(&[0, 0]);
    write_segment(jpeg, APP0, &payload);
}

/// A quantization table of 8-bit steps, in zigzag order.
fn write_quant_table(jpeg: &mut Vec<u8>, set: TableSet, table: &[u16; 64]) {
    let mut payload = vec![set.index()];
    payload.extend(
        ZIGZAG
            .iter()
            .map(|&i| u8::try_from(table[i]).expect("baseline steps are bytes")),
    );
    write_segment(jpeg, DQT, &payload);
}

/// The baseline frame header: 8-bit samples, Y sampled 2×2, Cb and Cr
/// 1×1.
fn write_frame(jpeg: &mut Vec<u8>, width: u16, height: u16) {
    let mut payload = vec![8];
    payload.extend_from_slice(&height.to_be_bytes());
    payload.extend_from_slice(&width.to_be_bytes());
    payload.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    write_segment(jpeg, SOF0, &payload);
}

/// A Huffman table of `class` 0 (DC) or 1 (AC).
fn write_huffman_table(jpeg: &mut Vec<u8>, class: u8, set: TableSet, spec: &HuffmanSpec) {
    let mut payload = vec![class << 4 | set.index()];
    payload.extend_from_slice(&spec.counts);
    payload.extend_from_slice(spec.symbols);
    write_segment(jpeg, DHT, &payload);
}

/// The header of the single interleaved scan.
fn write_scan_header(jpeg: &mut Vec<u8>) {
    write_segment(jpeg, SOS, &[3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The marker segments of a file, up to the scan data.
    fn markers(jpeg: &[u8]) -> Vec<u8> {
        let mut found = Vec::new();
        let mut at = 2;
        loop {
            let marker = jpeg[at + 1];
            found.push(marker);
            if marker == SOS {
                return found;
            }
            at += 2 + usize::from(u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]));
        }
    }

    #[test]
    fn segments_come_in_libjpeg_order() {
        let jpeg = encode_rgb8(17, 9, &[90; 17 * 9 * 3], &JpegOptions::default()).unwrap();
        assert_eq!(
            markers(&jpeg),
            [APP0, DQT, DQT, SOF0, DHT, DHT, DHT, DHT, SOS]
        );
        assert_eq!(&jpeg[6..11], b"JFIF\0");
        assert_eq!(&jpeg[jpeg.len() - 2..], [0xFF, EOI]);
    }

    #[test]
    fn edge_sizes_encode() {
        for (width, height) in [(1, 1), (8, 8), (15, 17), (16, 16), (33, 2)] {
            let rgb: Vec<u8> = (0..width * height * 3)
                .map(|i| u8::try_from(i * 7 % 256).unwrap())
                .collect();
            let jpeg = encode_rgb8(width, height, &rgb, &JpegOptions::default()).unwrap();
            assert!(jpeg.len() > 300, "{width} × {height}");
        }
    }

    #[test]
    fn bad_input_is_rejected() {
        let options = JpegOptions::default();
        assert_eq!(
            encode_rgb8(0, 4, &[], &options),
            Err(JpegError::InvalidDimensions {
                width: 0,
                height: 4
            })
        );
        assert_eq!(
            encode_rgb8(1, 1, &[0; 4], &options),
            Err(JpegError::DataLengthMismatch {
                expected: 3,
                actual: 4
            })
        );
    }
}
