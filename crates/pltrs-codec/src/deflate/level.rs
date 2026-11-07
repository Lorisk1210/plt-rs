//! Compression levels and the match-finder parameters each one selects.

use std::error::Error;
use std::fmt;

/// A zlib-compatible compression level from 0 (store only) to 9 (smallest).
///
/// Levels trade speed for size as zlib-ng's do: level 0 emits stored
/// blocks, level 1 one fixed-code block of the first matches found, level
/// 2 greedy parsing, levels 3 to 6 greedy parsing with a look-ahead match
/// and levels 7 to 9 lazy matching, with progressively longer hash-chain
/// searches.
///
/// ```
/// use pltrs_codec::CompressionLevel;
///
/// let level = CompressionLevel::new(6)?;
/// assert_eq!(level, CompressionLevel::DEFAULT);
/// assert!(CompressionLevel::new(10).is_err());
/// # Ok::<(), pltrs_codec::deflate::InvalidCompressionLevel>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    /// Level 0: no compression, the data is stored verbatim.
    pub const NONE: Self = Self(0);
    /// Level 1: the fastest compressing level.
    pub const FASTEST: Self = Self(1);
    /// Level 6: zlib's default balance between speed and size.
    pub const DEFAULT: Self = Self(6);
    /// Level 9: the smallest output.
    pub const BEST: Self = Self(9);

    /// Returns the level `level`, which must be at most 9.
    pub const fn new(level: u8) -> Result<Self, InvalidCompressionLevel> {
        if level <= 9 {
            Ok(Self(level))
        } else {
            Err(InvalidCompressionLevel(level))
        }
    }

    /// Returns the numeric level, 0–9.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u8> for CompressionLevel {
    type Error = InvalidCompressionLevel;

    fn try_from(level: u8) -> Result<Self, Self::Error> {
        Self::new(level)
    }
}

impl From<CompressionLevel> for u8 {
    fn from(level: CompressionLevel) -> Self {
        level.0
    }
}

/// Error returned for a compression level above 9.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidCompressionLevel(pub u8);

impl fmt::Display for InvalidCompressionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid compression level {}; expected 0 to 9", self.0)
    }
}

impl Error for InvalidCompressionLevel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_validate_range() {
        for level in 0..=9 {
            assert_eq!(
                CompressionLevel::new(level).map(CompressionLevel::get),
                Ok(level)
            );
        }
        assert_eq!(
            CompressionLevel::try_from(10),
            Err(InvalidCompressionLevel(10))
        );
        assert_eq!(
            InvalidCompressionLevel(42).to_string(),
            "invalid compression level 42; expected 0 to 9"
        );
        assert_eq!(CompressionLevel::default(), CompressionLevel::DEFAULT);
        assert_eq!(u8::from(CompressionLevel::BEST), 9);
    }
}
