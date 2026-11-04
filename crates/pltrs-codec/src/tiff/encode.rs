//! Baseline TIFF encoding of RGBA images: little-endian, uncompressed, one
//! strip, unassociated alpha and the resolution in pixels per inch — the
//! layout Pillow writes for `Image.save(format="tiff", dpi=...)`.

use std::fmt;

use pltrs_core::alloc::{self, OutOfMemory};

/// Why an image could not be encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TiffError {
    /// A side is zero.
    InvalidDimensions {
        /// The requested width.
        width: u32,
        /// The requested height.
        height: u32,
    },
    /// The pixel data does not have `width × height × 4` bytes.
    DataLengthMismatch {
        /// The bytes needed.
        expected: usize,
        /// The bytes given.
        actual: usize,
    },
    /// The file would pass TIFF's 4 GiB limit.
    TooLarge,
    /// The memory for the file cannot be allocated.
    OutOfMemory(OutOfMemory),
}

impl fmt::Display for TiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(f, "a TIFF image cannot be {width} × {height} pixels")
            }
            Self::DataLengthMismatch { expected, actual } => write!(
                f,
                "the image has {actual} bytes of pixel data, expected {expected}"
            ),
            Self::TooLarge => f.write_str("the image is too large for a TIFF file"),
            Self::OutOfMemory(err) => write!(f, "cannot write the TIFF file: {err}"),
        }
    }
}

impl std::error::Error for TiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutOfMemory(err) => Some(err),
            _ => None,
        }
    }
}

/// The number of directory entries written.
const ENTRY_COUNT: u16 = 14;

/// The field types used.
#[derive(Clone, Copy)]
enum FieldType {
    Short = 3,
    Long = 4,
    Rational = 5,
}

/// A directory entry: tag, type, count and the value or the offset of the
/// values.
struct Entry {
    tag: u16,
    kind: FieldType,
    count: u32,
    value: u32,
}

/// Encodes `width × height` straight RGBA pixels, row by row, as a TIFF
/// file with a resolution of `dpi` pixels per inch.
///
/// ```
/// let tiff = pltrs_codec::tiff::encode_rgba8(2, 1, &[255; 8], 100.0)?;
/// assert!(tiff.starts_with(b"II*\0"));
/// # Ok::<(), pltrs_codec::tiff::TiffError>(())
/// ```
///
/// # Errors
///
/// When a side is zero, the data has the wrong length, the file would be
/// larger than 4 GiB or its memory cannot be allocated.
pub fn encode_rgba8(width: u32, height: u32, data: &[u8], dpi: f64) -> Result<Vec<u8>, TiffError> {
    if width == 0 || height == 0 {
        return Err(TiffError::InvalidDimensions { width, height });
    }
    let expected = usize::try_from(u64::from(width) * u64::from(height) * 4)
        .map_err(|_| TiffError::TooLarge)?;
    if data.len() != expected {
        return Err(TiffError::DataLengthMismatch {
            expected,
            actual: data.len(),
        });
    }
    // The header, the directory, then the values that do not fit in an
    // entry: the bits per sample and the two resolutions.
    let directory_end = 8 + 2 + u32::from(ENTRY_COUNT) * 12 + 4;
    let bits_at = directory_end;
    let x_resolution_at = bits_at + 8;
    let y_resolution_at = x_resolution_at + 8;
    let strip_at = y_resolution_at + 8;
    let strip_len = u32::try_from(data.len()).map_err(|_| TiffError::TooLarge)?;
    strip_at.checked_add(strip_len).ok_or(TiffError::TooLarge)?;
    let entry = |tag, kind, count, value| Entry {
        tag,
        kind,
        count,
        value,
    };
    let entries = [
        entry(256, FieldType::Long, 1, width),
        entry(257, FieldType::Long, 1, height),
        entry(258, FieldType::Short, 4, bits_at),
        // No compression.
        entry(259, FieldType::Short, 1, 1),
        // RGB.
        entry(262, FieldType::Short, 1, 2),
        entry(273, FieldType::Long, 1, strip_at),
        entry(277, FieldType::Short, 1, 4),
        entry(278, FieldType::Long, 1, height),
        entry(279, FieldType::Long, 1, strip_len),
        entry(282, FieldType::Rational, 1, x_resolution_at),
        entry(283, FieldType::Rational, 1, y_resolution_at),
        // Samples interleaved.
        entry(284, FieldType::Short, 1, 1),
        // Resolution in inches.
        entry(296, FieldType::Short, 1, 2),
        // The fourth sample is unassociated alpha.
        entry(338, FieldType::Short, 1, 2),
    ];
    let mut tiff = alloc::try_with_capacity(data.len() + 256).map_err(TiffError::OutOfMemory)?;
    tiff.extend_from_slice(b"II*\0");
    tiff.extend_from_slice(&8_u32.to_le_bytes());
    tiff.extend_from_slice(
        &u16::try_from(entries.len())
            .expect("14 entries")
            .to_le_bytes(),
    );
    for entry in &entries {
        tiff.extend_from_slice(&entry.tag.to_le_bytes());
        tiff.extend_from_slice(&(entry.kind as u16).to_le_bytes());
        tiff.extend_from_slice(&entry.count.to_le_bytes());
        match (entry.kind, entry.count) {
            // A single short sits in the first two bytes of the field.
            (FieldType::Short, 1) => {
                let short = u16::try_from(entry.value).expect("short values fit");
                tiff.extend_from_slice(&short.to_le_bytes());
                tiff.extend_from_slice(&[0, 0]);
            }
            _ => tiff.extend_from_slice(&entry.value.to_le_bytes()),
        }
    }
    tiff.extend_from_slice(&0_u32.to_le_bytes());
    for _ in 0..4 {
        tiff.extend_from_slice(&8_u16.to_le_bytes());
    }
    let (numerator, denominator) = rational(dpi);
    for _ in 0..2 {
        tiff.extend_from_slice(&numerator.to_le_bytes());
        tiff.extend_from_slice(&denominator.to_le_bytes());
    }
    tiff.extend_from_slice(data);
    Ok(tiff)
}

/// The closest fraction to a positive `value` with 32-bit terms, by
/// continued fractions; values that are not finite and positive give
/// `1/1`.
fn rational(value: f64) -> (u32, u32) {
    if !(value.is_finite() && value > 0.0) {
        return (1, 1);
    }
    let limit = f64::from(u32::MAX);
    if value >= limit {
        return (u32::MAX, 1);
    }
    // Convergents h/k of the continued fraction of `value`.
    let (mut h, mut h_prev) = (1.0_f64, 0.0_f64);
    let (mut k, mut k_prev) = (0.0_f64, 1.0_f64);
    let mut rest = value;
    let mut best = (value.round().max(1.0), 1.0);
    for _ in 0..64 {
        let term = rest.floor();
        let (h_next, k_next) = (term * h + h_prev, term * k + k_prev);
        if h_next > limit || k_next > limit {
            break;
        }
        (h_prev, h, k_prev, k) = (h, h_next, k, k_next);
        if h > 0.0 {
            best = (h, k);
        }
        let fraction = rest - term;
        if fraction < 1e-12 || (h / k - value).abs() <= f64::EPSILON * value {
            break;
        }
        rest = 1.0 / fraction;
    }
    let term = |x: f64| {
        // Convergents are whole numbers within u32 by construction.
        x as u32
    };
    (term(best.0), term(best.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_at(data: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([data[at], data[at + 1]])
    }

    fn u32_at(data: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn layout_matches_pillow() {
        let pixels: Vec<u8> = (0..24).collect();
        let tiff = encode_rgba8(3, 2, &pixels, 100.0).unwrap();
        assert_eq!(tiff.len(), 230, "Pillow's file for the same image");
        assert_eq!(u16_at(&tiff, 8), 14);
        let tags: Vec<_> = (0..14).map(|i| u16_at(&tiff, 10 + 12 * i)).collect();
        assert_eq!(
            tags,
            [
                256, 257, 258, 259, 262, 273, 277, 278, 279, 282, 283, 284, 296, 338
            ]
        );
        assert_eq!(u32_at(&tiff, 10 + 12 * 5 + 8), 206, "strip offset");
        assert_eq!(&tiff[206..], &pixels[..]);
        assert_eq!((u32_at(&tiff, 190), u32_at(&tiff, 194)), (100, 1));
    }

    #[test]
    fn resolutions_become_fractions() {
        assert_eq!(rational(72.0), (72, 1));
        assert_eq!(rational(72.5), (145, 2));
        assert_eq!(rational(0.1), (1, 10));
        assert_eq!(rational(f64::NAN), (1, 1));
        assert_eq!(rational(1e12), (u32::MAX, 1));
        let (n, d) = rational(std::f64::consts::PI);
        assert!((f64::from(n) / f64::from(d) - std::f64::consts::PI).abs() < 1e-15);
    }

    #[test]
    fn bad_input_is_rejected() {
        assert_eq!(
            encode_rgba8(0, 1, &[], 72.0),
            Err(TiffError::InvalidDimensions {
                width: 0,
                height: 1
            })
        );
        assert_eq!(
            encode_rgba8(1, 1, &[0; 3], 72.0),
            Err(TiffError::DataLengthMismatch {
                expected: 4,
                actual: 3
            })
        );
    }
}
