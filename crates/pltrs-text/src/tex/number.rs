//! The numbers a TeX document and its cache are named with, written as
//! the program gave them.
//!
//! matplotlib writes the `str()` of the font size it is given into the
//! document (`\fontsize{12}{15.0}`) and of the resolution into the cache
//! key and `dvipng`'s command line, so `12` and `12.0` make different
//! documents, files and cache entries. [`TexNumber`] keeps which of the two
//! a size was; [`TexResolution`] also passes any text through, as
//! matplotlib passes `savefig.dpi`'s `'figure'` to `dvipng`.
//!
//! ```
//! use pltrs_text::tex::{TexNumber, TexResolution};
//! assert_eq!(TexNumber::from(12).to_string(), "12");
//! assert_eq!(TexNumber::from(12.0).to_string(), "12.0");
//! assert_eq!(TexResolution::from("figure").to_string(), "figure");
//! ```

use std::fmt;

use pltrs_core::pyfmt;

/// A number as the program gave it: a whole number or a float, written
/// as Python writes it (`12`, `12.0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TexNumber {
    /// A whole number.
    Integer(i64),
    /// A float.
    Float(f64),
}

impl TexNumber {
    /// The number's value (a whole number rounded as Python's `float()`
    /// rounds it).
    pub fn value(self) -> f64 {
        match self {
            Self::Integer(value) => value as f64,
            Self::Float(value) => value,
        }
    }
}

impl From<f64> for TexNumber {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for TexNumber {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl fmt::Display for TexNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Integer(value) => write!(f, "{value}"),
            Self::Float(value) => f.write_str(&pyfmt::repr(value)),
        }
    }
}

/// A resolution as the program gave it: a number, or any text, which
/// `dvipng` reads as it reads its `-D` argument.
#[derive(Clone, Debug, PartialEq)]
pub enum TexResolution {
    /// A number of dots per inch.
    Number(TexNumber),
    /// Text passed through as it is.
    Text(String),
}

impl From<f64> for TexResolution {
    fn from(value: f64) -> Self {
        Self::Number(value.into())
    }
}

impl From<TexNumber> for TexResolution {
    fn from(value: TexNumber) -> Self {
        Self::Number(value)
    }
}

impl From<&str> for TexResolution {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<String> for TexResolution {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl fmt::Display for TexResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(f),
            Self::Text(text) => f.write_str(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_python_writes_them() {
        assert_eq!(TexNumber::from(-3).to_string(), "-3");
        assert_eq!(TexNumber::from(1e22).to_string(), "1e+22");
        assert_eq!(TexNumber::from(f64::NAN).to_string(), "nan");
        assert_eq!(TexNumber::from(7).value(), 7.0);
        assert_eq!(TexResolution::from(72.0).to_string(), "72.0");
        assert_eq!(TexResolution::from(TexNumber::from(72)).to_string(), "72");
        assert_eq!(TexResolution::from(String::new()).to_string(), "");
    }
}
