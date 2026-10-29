//! Checksums used by the zlib and PNG formats.
//!
//! * [`Crc32`] / [`crc32`] — the IEEE CRC-32 that guards every PNG chunk.
//! * [`Adler32`] / [`adler32`] — the checksum in the zlib trailer.
//!
//! Both hashers are incremental: feeding data in pieces gives the same result
//! as hashing it in one go.
//!
//! ```
//! use pltrs_codec::checksum::{adler32, crc32, Adler32};
//!
//! assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
//!
//! let mut hasher = Adler32::new();
//! hasher.update(b"Wiki");
//! hasher.update(b"pedia");
//! assert_eq!(hasher.finish(), adler32(b"Wikipedia"));
//! ```

mod adler32;
mod crc32;

pub use adler32::{Adler32, adler32};
pub use crc32::{Crc32, crc32};
