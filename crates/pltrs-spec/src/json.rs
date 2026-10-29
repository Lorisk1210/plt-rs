//! The JSON of the golden files: pltrs-core's reader and writer.
//!
//! Goldens are produced by Python's `json` module with `allow_nan=True`;
//! [`Json::parse`] reads them (with the `NaN`, `Infinity` and `-Infinity`
//! tokens, integer literals apart from floats and object keys in document
//! order) and [`Json`]'s `Display` writes values back in the same style.

pub use pltrs_core::json::{Json, ParseError};
