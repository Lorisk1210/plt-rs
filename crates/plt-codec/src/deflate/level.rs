//! Compression levels and the match-finder parameters each one selects.

use std::error::Error;
use std::fmt;

/// A zlib-compatible compression level from 0 (store only) to 9 (smallest).
///
/// Levels trade speed for size the way zlib's do: level 0 emits stored
/// blocks, levels 1–3 use greedy parsing and levels 4–9 lazy matching with
/// progressively longer hash-chain searches.
///
/// ```
/// use plt_codec::CompressionLevel;
///
/// let level = CompressionLevel::new(6)?;
/// assert_eq!(level, CompressionLevel::DEFAULT);
/// assert!(CompressionLevel::new(10).is_err());
/// # Ok::<(), plt_codec::deflate::InvalidCompressionLevel>(())
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

    /// Parameters of the compressor at this level (zlib's
    /// `configuration_table`).
    pub(crate) const fn params(self) -> LevelParams {
        let (parsing, good_length, lazy_length, nice_length, max_chain) = match self.0 {
            0 => (Parsing::Stored, 0, 0, 0, 0),
            1 => (Parsing::Greedy, 4, 4, 8, 4),
            2 => (Parsing::Greedy, 4, 5, 16, 8),
            3 => (Parsing::Greedy, 4, 6, 32, 32),
            4 => (Parsing::Lazy, 4, 4, 16, 16),
            5 => (Parsing::Lazy, 8, 16, 32, 32),
            6 => (Parsing::Lazy, 8, 16, 128, 128),
            7 => (Parsing::Lazy, 8, 32, 128, 256),
            8 => (Parsing::Lazy, 32, 128, 258, 1024),
            _ => (Parsing::Lazy, 32, 258, 258, 4096),
        };
        LevelParams {
            parsing,
            good_length,
            lazy_length,
            nice_length,
            max_chain,
        }
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

/// How the LZ77 stage chooses between literals and matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Parsing {
    /// No LZ77 at all: stored blocks only.
    Stored,
    /// Take the longest match at each position.
    Greedy,
    /// Defer each match by one byte to see whether a longer one follows.
    Lazy,
}

/// Match-finder tuning for one compression level.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LevelParams {
    /// Literal/match selection strategy.
    pub(crate) parsing: Parsing,
    /// Once the previous match is this long, chain searches are cut to a
    /// quarter.
    pub(crate) good_length: usize,
    /// Lazy parsing: do not look for a better match after one this long.
    /// Greedy parsing: insert every position of matches up to this long into
    /// the hash chains.
    pub(crate) lazy_length: usize,
    /// Stop searching as soon as a match this long is found.
    pub(crate) nice_length: usize,
    /// Maximum number of hash-chain candidates examined per search.
    pub(crate) max_chain: usize,
}

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

    #[test]
    fn parsing_strategy_follows_zlib() {
        assert_eq!(CompressionLevel::NONE.params().parsing, Parsing::Stored);
        assert_eq!(CompressionLevel::FASTEST.params().parsing, Parsing::Greedy);
        assert_eq!(CompressionLevel::DEFAULT.params().parsing, Parsing::Lazy);
        assert_eq!(CompressionLevel::BEST.params().max_chain, 4096);
    }
}
