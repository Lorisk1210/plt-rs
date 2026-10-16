//! Python-compatible number formatting.
//!
//! matplotlib builds tick labels with Python's float formatting — `repr`,
//! printf-style `%` formatting and the `str.format` mini-language — and
//! Rust's `{}` output differs from it (`1e-05` vs `1e-5`, `1e+16` vs
//! `10000000000000000`). This module reproduces CPython's algorithms: the
//! digits come from Rust's exact formatting, which rounds half to even on
//! the exact binary value like CPython's `dtoa`, and are laid out by a port
//! of CPython's `format_float_short` and its format-spec machinery.
//!
//! * [`repr`], [`fixed`], [`exponential`], [`general`] — `repr(x)`,
//!   `'%.nf' % x`, `'%.ne' % x` and `'%.ng' % x`.
//! * [`PrintfFormat`] — `template % x` for any one-conversion template.
//! * [`FormatSpec`] — `format(x, spec)`.
//! * [`StrTemplate`] — `template.format(x=x, pos=pos)`.
//!
//! ```
//! use plt::ticker::pyfmt;
//! assert_eq!(pyfmt::repr(1e-5), "1e-05");
//! assert_eq!(pyfmt::general(1e7, 6), "1e+07");
//! assert_eq!(pyfmt::fixed(0.125, 2), "0.12");
//! ```

mod dtoa;
mod printf;
mod spec;
mod template;

use std::fmt;

use dtoa::{FloatStyle, Presentation, format_float};
pub use printf::PrintfFormat;
pub use spec::FormatSpec;
pub use template::StrTemplate;

/// Why a Python-style format string was rejected; the variants mirror the
/// Python exception that would have been raised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// Python's `ValueError`: a malformed template or spec.
    Value(String),
    /// Python's `TypeError`: the template does not fit the argument.
    Type(String),
    /// Python's `KeyError`: a replacement field names an unknown argument.
    Key(String),
    /// Python's `IndexError`: a positional replacement field.
    Index(String),
    /// Valid Python that plt-rs does not implement.
    Unsupported(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(msg) | Self::Type(msg) | Self::Index(msg) => f.write_str(msg),
            Self::Key(name) => write!(f, "unknown format field {name:?}"),
            Self::Unsupported(what) => write!(f, "unsupported format feature: {what}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Python's `repr(x)` (and `str(x)`) for a float.
pub fn repr(x: f64) -> String {
    format_float(x, FloatStyle::REPR)
}

/// Python's `'%.{precision}f' % x`.
pub fn fixed(x: f64, precision: usize) -> String {
    format_float(x, FloatStyle::new(Presentation::Fixed, precision))
}

/// Python's `'%.{precision}e' % x`.
pub fn exponential(x: f64, precision: usize) -> String {
    format_float(x, FloatStyle::new(Presentation::Exponent, precision))
}

/// Python's `'%.{precision}g' % x`.
pub fn general(x: f64, precision: usize) -> String {
    format_float(x, FloatStyle::new(Presentation::General, precision))
}

/// Python's `'%#.{precision}g' % x`: like [`general`] but keeping trailing
/// zeros and the decimal point.
pub fn general_alternate(x: f64, precision: usize) -> String {
    format_float(
        x,
        FloatStyle {
            alternate: true,
            ..FloatStyle::new(Presentation::General, precision)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convenience_functions() {
        assert_eq!(repr(10_000_000.0), "10000000.0");
        assert_eq!(fixed(-0.0, 1), "-0.0");
        assert_eq!(exponential(1500.25, 1), "1.5e+03");
        assert_eq!(general(1234.5678, 6), "1234.57");
        assert_eq!(general_alternate(3.14159265, 8), "3.1415927");
        assert_eq!(general_alternate(1e-7, 4), "1.000e-07");
    }
}
