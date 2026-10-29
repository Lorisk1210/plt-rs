//! Errors reported while decoding DEFLATE streams.

use std::error::Error;
use std::fmt;

/// The Huffman alphabet an [`InflateError`] refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HuffmanTable {
    /// The literal/length alphabet (literal bytes, end of block, lengths).
    LiteralLength,
    /// The distance alphabet.
    Distance,
    /// The code-length alphabet used to transmit dynamic Huffman codes.
    CodeLength,
}

impl fmt::Display for HuffmanTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LiteralLength => "literal/length",
            Self::Distance => "distance",
            Self::CodeLength => "code-length",
        })
    }
}

/// An error found while decompressing a DEFLATE stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InflateError {
    /// The input ended before the final block was complete.
    UnexpectedEof,
    /// A block header used the reserved block type 3.
    InvalidBlockType,
    /// A stored block's length disagrees with its one's complement copy.
    StoredLengthMismatch {
        /// The `LEN` field.
        length: u16,
        /// The `NLEN` field, which should equal `!length`.
        complement: u16,
    },
    /// A dynamic block declared more codes than the alphabet has symbols.
    TooManyCodes {
        /// The alphabet concerned.
        table: HuffmanTable,
        /// The declared number of codes.
        count: usize,
    },
    /// The code lengths of an alphabet over-subscribe the code space.
    OversubscribedCode(HuffmanTable),
    /// The code lengths of an alphabet leave part of the code space unused,
    /// which DEFLATE only permits for a single one-bit code.
    IncompleteCode(HuffmanTable),
    /// A dynamic block gave the end-of-block symbol no code.
    MissingEndOfBlock,
    /// A "repeat previous length" instruction came before any length.
    RepeatWithoutPrevious,
    /// A run of code lengths extends past the declared number of codes.
    CodeLengthOverflow,
    /// The input holds a bit pattern that is no valid code, or one of the
    /// reserved symbols (literal/length 286–287, distance 30–31).
    InvalidCode(HuffmanTable),
    /// A back-reference points before the start of the output.
    DistanceTooFarBack {
        /// The distance of the back-reference.
        distance: usize,
        /// The number of bytes decoded so far.
        available: usize,
    },
    /// The decompressed data would exceed the caller's size limit.
    OutputLimitExceeded {
        /// The limit, in bytes.
        limit: usize,
    },
}

impl fmt::Display for InflateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnexpectedEof => f.write_str("DEFLATE stream ended unexpectedly"),
            Self::InvalidBlockType => f.write_str("invalid DEFLATE block type 3"),
            Self::StoredLengthMismatch { length, complement } => write!(
                f,
                "stored block length {length:#06x} does not match its complement {complement:#06x}"
            ),
            Self::TooManyCodes { table, count } => {
                write!(f, "dynamic block declares {count} {table} codes")
            }
            Self::OversubscribedCode(table) => {
                write!(f, "over-subscribed {table} Huffman code")
            }
            Self::IncompleteCode(table) => write!(f, "incomplete {table} Huffman code"),
            Self::MissingEndOfBlock => f.write_str("dynamic block has no end-of-block code"),
            Self::RepeatWithoutPrevious => {
                f.write_str("code-length repeat with no previous length")
            }
            Self::CodeLengthOverflow => {
                f.write_str("code-length run exceeds the number of declared codes")
            }
            Self::InvalidCode(table) => write!(f, "invalid {table} code"),
            Self::DistanceTooFarBack {
                distance,
                available,
            } => write!(
                f,
                "back-reference distance {distance} exceeds the {available} bytes decoded so far"
            ),
            Self::OutputLimitExceeded { limit } => {
                write!(f, "decompressed data exceeds the limit of {limit} bytes")
            }
        }
    }
}

impl Error for InflateError {}
