//! The bits of Python and NumPy semantics matplotlib's `_type1font`
//! relies on: slice and index normalisation, `int()` and `float()` of
//! token text, float `repr` and `%f` formatting, NumPy's `round` and the
//! ASCII and Latin-1 codecs.

use std::ops::Range;

use super::error::Type1Error;

/// The start of `seq[start:]` for a sequence of `len` items.
pub(crate) fn slice_from(len: usize, start: i64) -> usize {
    normalize(len, start)
}

/// The items of `seq[start:stop]` for a sequence of `len` items.
pub(crate) fn slice(len: usize, start: i64, stop: i64) -> Range<usize> {
    let start = normalize(len, start);
    start..normalize(len, stop).max(start)
}

/// A slice bound: negative bounds count from the end, and bounds are
/// clamped to the sequence.
fn normalize(len: usize, bound: i64) -> usize {
    let len_i = i64::try_from(len).unwrap_or(i64::MAX);
    let bound = if bound < 0 {
        (len_i + bound).max(0)
    } else {
        bound.min(len_i)
    };
    usize::try_from(bound).unwrap_or(len)
}

/// The position `seq[index]` reads, `None` where Python raises
/// `IndexError`.
pub(crate) fn index(len: usize, index: i64) -> Option<usize> {
    let len_i = i64::try_from(len).ok()?;
    let index = if index < 0 { len_i + index } else { index };
    usize::try_from(index).ok().filter(|&i| i < len)
}

/// Python's white space for `int()` and `float()` among the characters
/// a token can hold.
pub(crate) const fn is_python_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\x0b' | '\x0c' | '\r' | '\x1c'..='\x1f' | ' '
    )
}

/// Whether `text` is a run of decimal digits, with single underscores
/// between digits allowed.
fn is_digit_part(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('_')
        && !text.ends_with('_')
        && !text.contains("__")
        && text.chars().all(|c| c.is_ascii_digit() || c == '_')
}

/// Strips Python white space and a leading sign: `(negative, rest)`.
fn unsigned(text: &str) -> (bool, &str) {
    let text = text.trim_matches(is_python_space);
    match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    }
}

/// Python's `float(text)`, `None` where it raises `ValueError`.
pub(crate) fn parse_float(text: &str) -> Option<f64> {
    let (negative, body) = unsigned(text);
    let lower = body.to_ascii_lowercase();
    let magnitude = match lower.as_str() {
        "inf" | "infinity" => f64::INFINITY,
        "nan" => f64::NAN,
        _ => {
            let (mantissa, exponent) = match lower.split_once('e') {
                Some((mantissa, exponent)) => (mantissa, Some(exponent)),
                None => (lower.as_str(), None),
            };
            let valid_mantissa = match mantissa.split_once('.') {
                Some((int, frac)) => {
                    (int.is_empty() || is_digit_part(int))
                        && (frac.is_empty() || is_digit_part(frac))
                        && !(int.is_empty() && frac.is_empty())
                }
                None => is_digit_part(mantissa),
            };
            let valid_exponent = exponent.is_none_or(|e| {
                let e = e.strip_prefix(['+', '-']).unwrap_or(e);
                is_digit_part(e)
            });
            if !valid_mantissa || !valid_exponent {
                return None;
            }
            lower.replace('_', "").parse().ok()?
        }
    };
    Some(if negative { -magnitude } else { magnitude })
}

/// Python's `int(text)` in base 10, `None` where it raises `ValueError`
/// (and for values beyond 64 bits, which Python would keep).
pub(crate) fn parse_int(text: &str) -> Option<i64> {
    let (negative, body) = unsigned(text);
    if !is_digit_part(body) {
        return None;
    }
    let magnitude: i128 = body.replace('_', "").parse().ok()?;
    i64::try_from(if negative { -magnitude } else { magnitude }).ok()
}

/// Python's `repr` of a float: the shortest digits that round-trip,
/// positional between 1e-4 and 1e16 with at least one decimal, else
/// scientific with a signed exponent of at least two digits.
pub(crate) fn repr(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("`{:e}` always writes an exponent");
    let exponent: i32 = exponent.parse().expect("`{:e}` writes an integer exponent");
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let point = exponent + 1;
    if !(-3..=16).contains(&point) {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let exp_sign = if exponent < 0 { '-' } else { '+' };
        return format!("{sign}{first}{fraction}e{exp_sign}{:02}", exponent.abs());
    }
    let count = digits.len() as i32;
    let body = if point <= 0 {
        format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize))
    } else if point >= count {
        format!("{digits}{}.0", "0".repeat((point - count) as usize))
    } else {
        let (int, frac) = digits.split_at(point as usize);
        format!("{int}.{frac}")
    };
    format!("{sign}{body}")
}

/// Python's `f'{value:.{precision}f}'`.
pub(crate) fn fixed(value: f64, precision: usize) -> String {
    if value.is_nan() {
        "nan".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 { "inf" } else { "-inf" }.to_owned()
    } else {
        format!("{value:.precision$}")
    }
}

/// matplotlib's `cbook._format_approx`: at most `precision` decimals,
/// without trailing zeros or a trailing point.
pub(crate) fn format_approx(value: f64, precision: usize) -> String {
    let text = fixed(value, precision);
    let trimmed = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text.as_str()
    };
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// NumPy's `round(value, decimals)` for a `float64`: scaled, rounded half
/// to even and scaled back, which differs from Python's correctly rounded
/// `round` near ties.
pub(crate) fn numpy_round(value: f64, decimals: i32) -> f64 {
    let scale = 10f64.powi(decimals);
    (value * scale).round_ties_even() / scale
}

/// `bytes.decode('ascii', 'replace')`: every byte above 127 becomes
/// U+FFFD.
pub(crate) fn ascii_lossy(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii() {
                char::from(b)
            } else {
                '\u{fffd}'
            }
        })
        .collect()
}

/// `str.encode('latin-1')`.
pub(crate) fn latin1(text: &str) -> Result<Vec<u8>, Type1Error> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).map_err(|_| Type1Error::NotLatin1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_and_indices() {
        assert_eq!(slice(5, 1, 3), 1..3);
        assert_eq!(slice(5, -2, 10), 3..5);
        assert_eq!(slice(5, 4, 2), 4..4);
        assert_eq!(slice(5, -9, -7), 0..0);
        assert_eq!(slice_from(5, -1), 4);
        assert_eq!(index(3, -1), Some(2));
        assert_eq!(index(3, -4), None);
        assert_eq!(index(3, 3), None);
    }

    #[test]
    fn python_number_grammar() {
        assert_eq!(parse_float("1234"), Some(1234.0));
        assert_eq!(parse_float("-9.81"), Some(-9.81));
        assert_eq!(parse_float(".5"), Some(0.5));
        assert_eq!(parse_float("5."), Some(5.0));
        assert_eq!(parse_float("1_0e1_0"), Some(1e11));
        assert_eq!(parse_float("\x0b2"), Some(2.0));
        assert!(parse_float("-NaN").is_some_and(f64::is_nan));
        assert_eq!(parse_float("Infinity"), Some(f64::INFINITY));
        for bad in [
            "", ".", "1__0", "_1", "1e", "e5", "abc", "1.2.3", "--1", "0x10",
        ] {
            assert_eq!(parse_float(bad), None, "{bad}");
        }
        assert_eq!(parse_int("007"), Some(7));
        assert_eq!(parse_int("-1_000"), Some(-1000));
        assert_eq!(parse_int("1e5"), None);
        assert_eq!(parse_int("99999999999999999999"), None);
    }

    #[test]
    fn float_repr_and_formatting() {
        assert_eq!(repr(-45.0), "-45.0");
        assert_eq!(repr(-9.4809), "-9.4809");
        assert_eq!(repr(0.0001), "0.0001");
        assert_eq!(repr(0.00001), "1e-05");
        assert_eq!(repr(1e16), "1e+16");
        assert_eq!(repr(1_234_567_890_123_456.0), "1234567890123456.0");
        assert_eq!(repr(1.5e300), "1.5e+300");
        assert_eq!(repr(-0.0), "-0.0");
        assert_eq!(repr(f64::NAN), "nan");
        assert_eq!(format_approx(0.000_167, 6), "0.000167");
        assert_eq!(format_approx(0.001, 6), "0.001");
        assert_eq!(format_approx(1000.0, 6), "1000");
        assert_eq!(format_approx(0.0, 6), "0");
        assert_eq!(format_approx(-1e-9, 6), "-0");
        assert_eq!(format_approx(f64::INFINITY, 6), "inf");
    }

    #[test]
    fn numpy_rounding_and_codecs() {
        assert_eq!(numpy_round(0.123_455, 5), 0.123_46);
        assert_eq!(numpy_round(45.000_005, 5), 45.0);
        assert_eq!(numpy_round(-9.480_903_617_950_83, 5), -9.4809);
        assert_eq!(ascii_lossy(b"a\xffb"), "a\u{fffd}b");
        assert_eq!(latin1("caf\u{e9}"), Ok(b"caf\xe9".to_vec()));
        assert_eq!(latin1("\u{fffd}"), Err(Type1Error::NotLatin1));
    }
}
