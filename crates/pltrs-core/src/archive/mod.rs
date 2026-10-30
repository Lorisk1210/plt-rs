//! Archives: values written to bytes and read back exactly, in this
//! process or another (what pickling a figure rests on).
//!
//! A type is archivable when it implements [`Archive`]: it writes itself
//! to an [`Encoder`] and reads itself back from a [`Decoder`]. The format
//! is positional and compact: integers as LEB128 varints, floats as their
//! bits, strings and sequences with their length first, enum variants as
//! their index. Archives are read by the pltrs build that wrote them
//! ([`to_bytes`] heads them with the format and the crate version, which
//! [`from_bytes`] checks).
//!
//! * Plain types implement [`Archive`] with
//!   [`archive_struct!`](crate::archive_struct) and
//!   [`archive_enum!`](crate::archive_enum), which list their fields so
//!   that a field added later fails to compile until it is archived too.
//! * Shared values keep their sharing: an [`Rc`](std::rc::Rc) reached
//!   twice is written once and read back as one value, so state shared
//!   through `Rc<RefCell<_>>` stays shared.
//! * Trait objects (`Box<dyn Trait>`, `Rc<dyn Trait>`) archive the
//!   concrete type behind them, named by a tag
//!   ([`archive_kinds!`](crate::archive_kinds)); a type of the program's
//!   that pltrs does not know is archived by the program, through the
//!   [`ProgramKinds`] it hands the encoder and the decoder.
//! * A value of the program's that the archive cannot hold (a Python
//!   object, for pltrs's Python package) is an [`External`]: the archive
//!   holds its index in the list of externals, which the program archives
//!   in its own way beside the bytes.
//!
//! Reading never panics on malformed bytes: it fails with an
//! [`ArchiveError`], and nesting deeper than [`MAX_DEPTH`] is refused.
//!
//! ```
//! use pltrs_core::archive::{self, NoProgram};
//! use pltrs_core::{Bbox, Point};
//!
//! #[derive(Debug, PartialEq)]
//! struct Marker {
//!     at: Point,
//!     label: Option<String>,
//! }
//! pltrs_core::archive_struct!(Marker { at, label });
//!
//! let marker = Marker { at: Point::new(1.0, 2.0), label: Some("a".into()) };
//! let (bytes, externals) = archive::to_bytes(&marker, &NoProgram)?;
//! let back: Marker = archive::from_bytes(&bytes, &externals, &NoProgram)?;
//! assert_eq!(back, marker);
//! # Ok::<(), archive::ArchiveError>(())
//! ```

mod decoder;
mod encoder;
mod error;
mod macros;
mod program;
mod std_impls;
#[cfg(test)]
mod tests;

pub use decoder::{Decoder, MAX_DEPTH};
pub use encoder::Encoder;
pub use error::ArchiveError;
pub use program::{External, KindOf, NoProgram, ProgramFn, ProgramKinds};

/// The bytes every archive starts with.
const MAGIC: &[u8; 8] = b"PLTRSARC";

/// The version of the archive format itself, raised whenever the
/// encoding of a primitive or of the header changes.
const FORMAT: u64 = 1;

/// A value that writes itself to an archive and reads itself back.
///
/// `decode` reads exactly what `encode` wrote, in the same order. Types
/// with invariants check them when they are read, as their constructors
/// do, since the bytes may come from anywhere.
pub trait Archive: Sized {
    /// Writes the value.
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError>;

    /// Reads a value written by [`encode`](Self::encode).
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError>;
}

/// A value an [`Rc`](std::rc::Rc) shares in an archive (see
/// [`Encoder::shared`]): any [`Archive`] value, a string or slice, or a
/// trait object whose kinds [`archive_kinds!`](crate::archive_kinds)
/// lists.
pub trait ArchiveRc: 'static {
    /// Writes the shared value.
    fn encode_rc(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError>;

    /// Reads back a value written by [`encode_rc`](Self::encode_rc), in
    /// a new `Rc`.
    fn decode_rc(decoder: &mut Decoder<'_>) -> Result<std::rc::Rc<Self>, ArchiveError>;
}

/// Archives `value`: the bytes, headed by the format and pltrs's version,
/// and the externals they refer to, in the order of their indices.
/// `program` archives the program's types pltrs does not know.
pub fn to_bytes<T: Archive>(
    value: &T,
    program: &dyn ProgramKinds,
) -> Result<(Vec<u8>, Vec<External>), ArchiveError> {
    let mut encoder = Encoder::new(program);
    encoder.write_raw(MAGIC);
    encoder.write_varint(FORMAT);
    encoder.write_str(env!("CARGO_PKG_VERSION"));
    value.encode(&mut encoder)?;
    Ok(encoder.finish())
}

/// Reads back a value archived by [`to_bytes`] with the externals it
/// returned (as the program restored them) and the same `program`.
pub fn from_bytes<T: Archive>(
    bytes: &[u8],
    externals: &[External],
    program: &dyn ProgramKinds,
) -> Result<T, ArchiveError> {
    let mut decoder = Decoder::new(bytes, externals, program);
    if decoder.read_raw(MAGIC.len()).ok() != Some(MAGIC.as_slice()) {
        return Err(ArchiveError::NotAnArchive);
    }
    let format = decoder.read_varint()?;
    let version = decoder.read_str()?;
    if format != FORMAT || version != env!("CARGO_PKG_VERSION") {
        return Err(ArchiveError::Version {
            found: format!("{version} (format {format})"),
            expected: format!("{} (format {FORMAT})", env!("CARGO_PKG_VERSION")),
        });
    }
    let value = T::decode(&mut decoder)?;
    decoder.finish()?;
    Ok(value)
}
