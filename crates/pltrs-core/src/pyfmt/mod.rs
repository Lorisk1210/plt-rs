//! Python-compatible number formatting.
//!
//! matplotlib builds tick labels with Python's float formatting — `repr`,
//! printf-style `%` formatting and the `str.format` mini-language — and
//! Rust's `{}` output differs from it (`1e-05` vs `1e-5`, `1e+16` vs
//! `10000000000000000`). This module reproduces `CPython`'s algorithms: the
//! digits come from Rust's exact formatting, which rounds half to even on
//! the exact binary value like `CPython`'s `dtoa`, and are laid out by a port
//! of `CPython`'s `format_float_short` and its format-spec machinery.
//!
//! * [`repr`], [`fixed`], [`exponential`], [`general`] — `repr(x)`,
//!   `'%.nf' % x`, `'%.ne' % x` and `'%.ng' % x`.
//! * [`PrintfFormat`] — `template % x` for any one-conversion template.
//! * [`FormatSpec`] — `format(x, spec)`.
//! * [`StrTemplate`] — `template.format(x=x, pos=pos)` or
//!   `template.format(x)`.
//! * [`auto_format`] — matplotlib's `%`-or-`format` choice for one value.
//!
//! ```
//! use pltrs_core::pyfmt;
//! assert_eq!(pyfmt::repr(1e-5), "1e-05");
//! assert_eq!(pyfmt::general(1e7, 6), "1e+07");
//! assert_eq!(pyfmt::fixed(0.125, 2), "0.12");
//! ```

mod dtoa;
mod printf;
mod spec;
mod template;

use std::fmt;

use dtoa::{EXACT_DECIMAL_PLACES, FloatStyle, Presentation, format_float};
pub use printf::PrintfFormat;
pub use spec::FormatSpec;
pub use template::{StrTemplate, TemplateValue};

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
    /// Valid Python that pltrs does not implement.
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

/// The widest field and the longest precision pltrs formats. Python
/// accepts any width up to `sys.maxsize` and allocates whatever is asked
/// (or raises `MemoryError`); pltrs refuses more than this with
/// [`FormatError::Unsupported`], so that no template exhausts memory.
pub const MAX_FIELD_SIZE: usize = 1 << 20;

/// `size` (a parsed width or precision) unless it exceeds
/// [`MAX_FIELD_SIZE`].
fn bounded_field(size: Option<usize>, what: &str) -> Result<Option<usize>, FormatError> {
    match size {
        Some(size) if size > MAX_FIELD_SIZE => Err(FormatError::Unsupported(format!(
            "a {what} of {size} (more than {MAX_FIELD_SIZE})"
        ))),
        _ => Ok(size),
    }
}

/// Python's `repr(x)` (and `str(x)`) for a float.
pub fn repr(x: f64) -> String {
    format_float(x, FloatStyle::REPR)
}

/// Python's `'%d' % x`: the value truncated towards zero. (Python raises
/// for NaN and infinity; those are written like [`repr`] here.)
pub fn integer(x: f64) -> String {
    if !x.is_finite() {
        return repr(x);
    }
    let digits = format!("{:.0}", x.trunc().abs());
    if x <= -1.0 {
        format!("-{digits}")
    } else {
        digits
    }
}

/// Python's `'%.{precision}f' % x`.
pub fn fixed(x: f64, precision: usize) -> String {
    if x.is_finite() && precision <= EXACT_DECIMAL_PLACES {
        // Rust's `{:.N}` rounds the exact binary value half to even, as
        // Python's `%f` does, and keeps the sign of negative zero; it is
        // the vector backends' hot path, so skip the general layout.
        format!("{x:.precision$}")
    } else {
        format_float(x, FloatStyle::new(Presentation::Fixed, precision))
    }
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

/// matplotlib's `cbook._auto_format_str`: `template % value` when Python
/// accepts that, else `template.format(value)` (so `"%.2f"`, `"{:.0%}"`
/// and plain text all work).
///
/// ```
/// use pltrs_core::pyfmt::auto_format;
/// assert_eq!(auto_format("%.2f m", 0.2).unwrap(), "0.20 m");
/// assert_eq!(auto_format("{} m", 0.2).unwrap(), "0.2 m");
/// assert_eq!(auto_format("%d or {}", 0.2).unwrap(), "0 or {}");
/// ```
///
/// # Errors
///
/// When neither form accepts `template`.
pub fn auto_format(template: &str, value: f64) -> Result<String, FormatError> {
    auto_format_value(template, value, false)
}

/// [`auto_format`] for a value that Python holds as an `int` when
/// `integer` (a whole number, as in numpy's integer arrays), so that
/// `"{:d}"` accepts it and `"{}"` prints no decimal point.
///
/// ```
/// use pltrs_core::pyfmt::auto_format_value;
/// assert_eq!(auto_format_value("{:d}", 5.0, true).unwrap(), "5");
/// assert_eq!(auto_format_value("{}", 5.0, false).unwrap(), "5.0");
/// assert!(auto_format_value("{:d}", 5.0, false).is_err());
/// ```
///
/// # Errors
///
/// When neither form accepts `template`.
pub fn auto_format_value(template: &str, value: f64, integer: bool) -> Result<String, FormatError> {
    match PrintfFormat::parse(template).and_then(|format| format.format(value)) {
        Err(FormatError::Type(_) | FormatError::Value(_)) if integer => {
            let whole = TemplateValue::Int(value as i64);
            StrTemplate::parse_positional_int(template)?.render_values(&[whole], |text| text)
        }
        Err(FormatError::Type(_) | FormatError::Value(_)) => {
            StrTemplate::parse_positional(template)?.render(value, None, |text| text)
        }
        printed => printed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_format_falls_back_to_str_format() {
        assert_eq!(auto_format("%g", 3.0), Ok("3".to_owned()));
        assert_eq!(auto_format("{:.0%}", 0.43), Ok("43%".to_owned()));
        assert_eq!(auto_format("const", 1.0), Ok("const".to_owned()));
        assert_eq!(auto_format("%d", f64::NAN), Ok("%d".to_owned()));
        assert!(auto_format("{y}", 1.0).is_err());
    }

    #[test]
    fn convenience_functions() {
        assert_eq!(repr(10_000_000.0), "10000000.0");
        assert_eq!(integer(-1500.25), "-1500");
        assert_eq!(integer(-0.5), "0");
        assert_eq!(integer(1e20), "100000000000000000000");
        assert_eq!(fixed(-0.0, 1), "-0.0");
        assert_eq!(fixed(0.25, 70_000).len(), 70_002);
        assert_eq!(exponential(1500.25, 1), "1.5e+03");
        assert_eq!(general(1234.5678, 6), "1234.57");
        assert_eq!(general_alternate(1.234_567_89, 8), "1.2345679");
        assert_eq!(general_alternate(1e-7, 4), "1.000e-07");
    }

    #[test]
    fn fixed_fast_path_matches_the_general_layout() {
        let mut rng = crate::rng::SplitMix64::new(7);
        let mut values = vec![0.0, -0.0, 0.5, 1.5, 2.5, -2.5, 0.125, -0.001, 1e300, 5e-324];
        for _ in 0..5000 {
            let magnitude = 10f64.powi((rng.next_u64() % 40) as i32 - 20);
            let sign = if rng.next_u64().is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            values.push(sign * rng.next_f64() * magnitude);
        }
        for value in values {
            for precision in 0..=12 {
                assert_eq!(
                    fixed(value, precision),
                    format_float(value, FloatStyle::new(Presentation::Fixed, precision)),
                    "{value:e} with {precision} digits"
                );
            }
        }
    }
}
