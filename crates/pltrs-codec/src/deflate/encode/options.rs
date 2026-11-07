//! What a compressor is asked for: the level, the strategy and the memory
//! level of zlib's `deflateInit2`, and the parameters zlib-ng's
//! `configuration_table` derives from the level.

use std::error::Error;
use std::fmt;

use crate::deflate::CompressionLevel;

/// How the compressor looks for matches (zlib's `strategy`).
///
/// Every strategy writes a valid DEFLATE stream; they trade the search
/// for repeated strings against speed or against data whose redundancy
/// lies elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Strategy {
    /// The level's own parsing (`Z_DEFAULT_STRATEGY`).
    #[default]
    Default,
    /// For filtered image data (`Z_FILTERED`): the lazy levels 7–9 drop
    /// matches of five bytes or fewer. Pillow writes PNG images with it.
    Filtered,
    /// Huffman coding of literals only, no matches (`Z_HUFFMAN_ONLY`).
    HuffmanOnly,
    /// Matches of distance one only, runs of a byte (`Z_RLE`).
    Rle,
    /// The level's parsing, always written with the fixed Huffman codes
    /// (`Z_FIXED`).
    Fixed,
}

impl Strategy {
    /// Whether the zlib header records the fastest level for it (zlib-ng
    /// writes `FLEVEL` 0 for the strategies from `Z_HUFFMAN_ONLY` on).
    pub(crate) fn is_fastest_kind(self) -> bool {
        matches!(self, Self::HuffmanOnly | Self::Rle | Self::Fixed)
    }
}

/// How much memory the compressor buffers symbols in (zlib's
/// `memLevel`, 1 to 9): a block holds up to `2^(level + 6) - 1` symbols.
///
/// ```
/// use pltrs_codec::deflate::MemoryLevel;
///
/// assert_eq!(MemoryLevel::new(8)?, MemoryLevel::DEFAULT);
/// assert!(MemoryLevel::new(0).is_err());
/// # Ok::<(), pltrs_codec::deflate::InvalidMemoryLevel>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryLevel(u8);

impl MemoryLevel {
    /// zlib's default, 8: blocks of up to 16 383 symbols.
    pub const DEFAULT: Self = Self(8);
    /// The largest, 9: blocks of up to 32 767 symbols, as Pillow asks.
    pub const MAX: Self = Self(9);

    /// The memory level `level`, which must be between 1 and 9.
    pub const fn new(level: u8) -> Result<Self, InvalidMemoryLevel> {
        if matches!(level, 1..=9) {
            Ok(Self(level))
        } else {
            Err(InvalidMemoryLevel(level))
        }
    }

    /// The numeric level, 1–9.
    pub const fn get(self) -> u8 {
        self.0
    }

    /// Room for symbols in a block (zlib's `lit_bufsize`).
    pub(crate) const fn symbol_buffer(self) -> usize {
        1 << (self.0 + 6)
    }
}

impl Default for MemoryLevel {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Error returned for a memory level outside 1 to 9.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidMemoryLevel(pub u8);

impl fmt::Display for InvalidMemoryLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid memory level {}; expected 1 to 9", self.0)
    }
}

impl Error for InvalidMemoryLevel {}

/// Everything that decides the compressed bytes of an input, as zlib's
/// `deflateInit2` takes it (with a 32 KiB window).
///
/// ```
/// use pltrs_codec::CompressionLevel;
/// use pltrs_codec::deflate::{self, MemoryLevel, Options, Strategy};
///
/// let options = Options::new(CompressionLevel::BEST)
///     .strategy(Strategy::Filtered)
///     .memory_level(MemoryLevel::MAX);
/// let packed = deflate::compress_with(b"abcabcabcabcabc", options)?;
/// assert_eq!(deflate::decompress(&packed)?, b"abcabcabcabcabc");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Options {
    pub(crate) level: CompressionLevel,
    pub(crate) strategy: Strategy,
    pub(crate) memory_level: MemoryLevel,
}

impl Options {
    /// The options zlib's `deflateInit` gives `level`: the default
    /// strategy and memory level.
    pub const fn new(level: CompressionLevel) -> Self {
        Self {
            level,
            strategy: Strategy::Default,
            memory_level: MemoryLevel::DEFAULT,
        }
    }

    /// Sets the strategy.
    #[must_use]
    pub const fn strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Sets the memory level.
    #[must_use]
    pub const fn memory_level(mut self, memory_level: MemoryLevel) -> Self {
        self.memory_level = memory_level;
        self
    }
}

/// The parsing a level compresses with (zlib-ng's `deflate_*` functions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Method {
    /// Stored blocks only (`deflate_stored`, level 0).
    Stored,
    /// One fixed-Huffman block, the first match taken (`deflate_quick`,
    /// level 1).
    Quick,
    /// Greedy parsing (`deflate_fast`, level 2).
    Fast,
    /// Greedy parsing that looks one match ahead (`deflate_medium`, levels
    /// 3–6).
    Medium,
    /// Lazy parsing (`deflate_slow`, levels 7–9).
    Slow,
}

/// The match finder's tuning at a level (a row of `configuration_table`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Config {
    /// Once the previous match is this long, chains are searched a quarter
    /// as far.
    pub(super) good_length: u32,
    /// Lazy parsing: no better match is looked for after one this long;
    /// greedy: matches up to this long have all their strings hashed.
    pub(super) max_lazy: u32,
    /// The search stops at a match this long.
    pub(super) nice_length: u32,
    /// The most chain links followed per search.
    pub(super) max_chain: u32,
    /// The parsing.
    pub(super) method: Method,
}

impl Config {
    /// zlib-ng's `configuration_table` row for `level`.
    pub(super) const fn of(level: CompressionLevel) -> Self {
        let (good_length, max_lazy, nice_length, max_chain, method) = match level.get() {
            0 => (0, 0, 0, 0, Method::Stored),
            1 => (0, 0, 0, 0, Method::Quick),
            2 => (4, 4, 8, 4, Method::Fast),
            3 => (4, 6, 16, 6, Method::Medium),
            4 => (4, 12, 32, 24, Method::Medium),
            5 => (8, 16, 32, 32, Method::Medium),
            6 => (8, 16, 128, 128, Method::Medium),
            7 => (8, 32, 128, 256, Method::Slow),
            8 => (32, 128, 258, 1024, Method::Slow),
            _ => (32, 258, 258, 4096, Method::Slow),
        };
        Self {
            good_length,
            max_lazy,
            nice_length,
            max_chain,
            method,
        }
    }

    /// Whether the level hashes with zlib's rolling three-byte hash
    /// rather than the multiplicative four-byte one: chains longer than
    /// 1024 links, where the slow match finder looks other chains up.
    pub(super) const fn rolls(self) -> bool {
        self.max_chain > 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_levels_validate_range() {
        for level in 1..=9 {
            assert_eq!(MemoryLevel::new(level).map(MemoryLevel::get), Ok(level));
        }
        assert_eq!(MemoryLevel::new(10), Err(InvalidMemoryLevel(10)));
        assert_eq!(
            InvalidMemoryLevel(0).to_string(),
            "invalid memory level 0; expected 1 to 9"
        );
        assert_eq!(MemoryLevel::DEFAULT.symbol_buffer(), 16_384);
        assert_eq!(MemoryLevel::MAX.symbol_buffer(), 32_768);
        assert_eq!(MemoryLevel::default(), MemoryLevel::DEFAULT);
    }

    #[test]
    fn levels_select_zlib_ng_methods() {
        let method = |level| Config::of(CompressionLevel::new(level).unwrap()).method;
        assert_eq!(method(0), Method::Stored);
        assert_eq!(method(1), Method::Quick);
        assert_eq!(method(2), Method::Fast);
        assert!((3..=6).all(|level| method(level) == Method::Medium));
        assert!((7..=9).all(|level| method(level) == Method::Slow));
        assert!(Config::of(CompressionLevel::BEST).rolls());
        assert!(!Config::of(CompressionLevel::new(8).unwrap()).rolls());
    }

    #[test]
    fn options_keep_what_they_are_given() {
        let options = Options::new(CompressionLevel::FASTEST)
            .strategy(Strategy::Rle)
            .memory_level(MemoryLevel::MAX);
        assert_eq!(options.level, CompressionLevel::FASTEST);
        assert_eq!(options.strategy, Strategy::Rle);
        assert_eq!(options.memory_level, MemoryLevel::MAX);
        assert_eq!(Options::default(), Options::new(CompressionLevel::DEFAULT));
        assert!(Strategy::Fixed.is_fastest_kind() && !Strategy::Filtered.is_fastest_kind());
    }
}
