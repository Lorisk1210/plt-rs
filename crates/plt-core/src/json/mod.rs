//! JSON (RFC 8259) values, read in two dialects and written in two styles.
//!
//! * [`Json::parse`] reads documents as Python's `json` module writes them
//!   with `allow_nan=True` (the spec's golden files): the `NaN`, `Infinity`
//!   and `-Infinity` tokens are numbers, and integer literals stay
//!   [`Json::Integer`]s, apart from floats.
//! * [`Json::parse_strict`] reads untrusted input (the web backend's
//!   browser messages) by RFC 8259 alone: finite numbers only, and a
//!   shallow nesting limit.
//!
//! Both readers keep object members in document order, reject control
//! characters inside strings, turn lone surrogate escapes into U+FFFD and
//! report errors with their byte offset; both limit nesting, so hostile
//! input fails instead of exhausting the stack.
//!
//! [`Json`]'s `Display` writes a value on one line in Python's style (the
//! spec's goldens); [`string`] and [`number`] write strict JSON literals
//! for text assembled piece by piece.

mod parse;
mod value;
mod write;

pub use parse::ParseError;
pub use value::Json;
pub use write::{number, string};

#[cfg(test)]
mod tests;
