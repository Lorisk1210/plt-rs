//! Why a JPEG file could not be decoded.

use std::fmt;

/// Which kind of table a scan referred to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableKind {
    /// A quantization table.
    Quantization,
    /// A Huffman table for DC coefficients.
    DcHuffman,
    /// A Huffman table for AC coefficients.
    AcHuffman,
}

/// Why a JPEG file could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The data does not start with a JPEG start-of-image marker.
    NotJpeg,
    /// The data ends before the image does.
    Truncated,
    /// A marker segment is too short, too long or holds invalid values.
    MalformedSegment {
        /// The marker (the byte after `0xFF`).
        marker: u8,
    },
    /// A marker that has no meaning in a JPEG file.
    UnknownMarker(u8),
    /// The frame uses a coding process the decoder does not implement:
    /// arithmetic coding, lossless or hierarchical JPEG.
    UnsupportedProcess {
        /// The frame's start-of-frame marker.
        marker: u8,
    },
    /// Samples of another precision than 8 bits.
    UnsupportedPrecision(u8),
    /// Another number of components than 1, 3 or 4.
    UnsupportedComponents(u8),
    /// A side of the frame is zero.
    EmptyImage,
    /// The image has more pixels than the input can hold, or than Pillow
    /// accepts.
    TooLarge {
        /// The declared width.
        width: u16,
        /// The declared height.
        height: u16,
    },
    /// Sampling factors outside 1–4, ratios that are not whole numbers,
    /// or an MCU of more than ten blocks.
    InvalidSampling,
    /// A second start-of-frame or start-of-image marker.
    DuplicateMarker(u8),
    /// A scan before the frame header, or an end of image before any scan.
    MissingFrame,
    /// A scan refers to a table that was never defined.
    MissingTable {
        /// The kind of table.
        kind: TableKind,
        /// Its number.
        index: u8,
    },
    /// A Huffman table whose code lengths do not form a prefix code, or a
    /// DC table with symbols above 15.
    InvalidHuffmanTable,
    /// The entropy-coded data holds a bit sequence that is not a code.
    InvalidHuffmanCode,
    /// Progressive DC differences add up past the range of an `int`,
    /// which libjpeg rejects.
    CoefficientOverflow,
    /// A scan header names an unknown or repeated component, or has
    /// spectral selection or successive approximation parameters that are
    /// not valid for a progressive scan.
    InvalidScan,
    /// The restart marker expected after a restart interval is missing.
    MissingRestart {
        /// The restart number (0–7) expected.
        expected: u8,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotJpeg => f.write_str("not a JPEG file"),
            Self::Truncated => f.write_str("the JPEG data ends early"),
            Self::MalformedSegment { marker } => {
                write!(f, "malformed JPEG marker segment 0xFF{marker:02X}")
            }
            Self::UnknownMarker(marker) => write!(f, "unknown JPEG marker 0xFF{marker:02X}"),
            Self::UnsupportedProcess { marker } => write!(
                f,
                "unsupported JPEG process (marker 0xFF{marker:02X}); only Huffman-coded \
                 baseline, extended and progressive JPEG can be read"
            ),
            Self::UnsupportedPrecision(bits) => write!(f, "cannot handle {bits}-bit samples"),
            Self::UnsupportedComponents(count) => {
                write!(f, "cannot handle JPEG images of {count} components")
            }
            Self::EmptyImage => f.write_str("the JPEG image has no pixels"),
            Self::TooLarge { width, height } => {
                write!(f, "a {width} × {height} JPEG image is too large to decode")
            }
            Self::InvalidSampling => f.write_str("unsupported JPEG sampling factors"),
            Self::DuplicateMarker(marker) => {
                write!(f, "repeated JPEG marker 0xFF{marker:02X}")
            }
            Self::MissingFrame => f.write_str("the JPEG data has no frame or no scan"),
            Self::MissingTable { kind, index } => {
                let kind = match kind {
                    TableKind::Quantization => "quantization",
                    TableKind::DcHuffman => "DC Huffman",
                    TableKind::AcHuffman => "AC Huffman",
                };
                write!(f, "the JPEG data uses undefined {kind} table {index}")
            }
            Self::InvalidHuffmanTable => f.write_str("invalid JPEG Huffman table"),
            Self::InvalidHuffmanCode => f.write_str("corrupt JPEG data: invalid Huffman code"),
            Self::CoefficientOverflow => {
                f.write_str("corrupt JPEG data: a DC coefficient is out of range")
            }
            Self::InvalidScan => f.write_str("invalid JPEG scan header"),
            Self::MissingRestart { expected } => {
                write!(f, "corrupt JPEG data: restart marker {expected} is missing")
            }
        }
    }
}

impl std::error::Error for DecodeError {}
