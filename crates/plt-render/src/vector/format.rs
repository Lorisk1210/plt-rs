//! Number and string formatting exactly as matplotlib's vector backends
//! write them with Python's formatting mini-language.

use std::fmt::Write as _;

use plt_core::Rgba;

/// `f"{value:.{digits}f}"` with trailing zeros and a trailing point
/// removed (`_short_float_fmt` for 6 digits, `pdfRepr` for 10, the
/// PostScript backend's `_nums_to_str` for 3).
pub(crate) fn trimmed(value: f64, digits: usize) -> String {
    let mut out = fixed(value, digits);
    if out.contains('.') {
        let len = out.trim_end_matches('0').trim_end_matches('.').len();
        out.truncate(len);
    }
    out
}

/// matplotlib's `_short_float_fmt`: `%f` without trailing zeros.
pub(crate) fn short(value: f64) -> String {
    trimmed(value, 6)
}

/// `f"{value:.{digits}f}"`: correctly rounded, ties to even, Python's
/// spelling of non-finite values.
pub(crate) fn fixed(value: f64, digits: usize) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    format!("{value:.digits$}")
}

/// The decimal digits and exponent of the shortest representation that
/// round-trips (`d.ddd × 10^exp`), as Rust's `{:e}` writes them.
fn shortest_digits(value: f64) -> (String, i32) {
    let text = format!("{:e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').expect("`{:e}` writes an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    (
        digits,
        exponent.parse().expect("`{:e}` writes an integer exponent"),
    )
}

/// Python's `repr(float)` (also `str(float)`): the shortest round-trip
/// digits, positional for exponents in `-4..16`, scientific otherwise.
pub(crate) fn repr(value: f64) -> String {
    if !value.is_finite() {
        return fixed(value, 0);
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    if value == 0.0 {
        return format!("{sign}0.0");
    }
    let (digits, exponent) = shortest_digits(value);
    if (-4..16).contains(&exponent) {
        format!("{sign}{}", positional(&digits, exponent, true))
    } else {
        format!("{sign}{}", scientific(&digits, exponent))
    }
}

/// Python's `%g` (`f"{value:g}"`): six significant digits, trailing zeros
/// removed, scientific notation for exponents outside `-4..6`.
pub(crate) fn general(value: f64) -> String {
    if !value.is_finite() {
        return fixed(value, 0);
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_owned();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    // Round to six significant digits first; the exponent of the rounded
    // value decides the notation.
    let rounded = format!("{:.5e}", value.abs());
    let (mantissa, exponent) = rounded.split_once('e').expect("`{:e}` writes an exponent");
    let exponent: i32 = exponent.parse().expect("`{:e}` writes an integer exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if (-4..6).contains(&exponent) {
        format!("{sign}{}", positional(digits, exponent, false))
    } else {
        format!("{sign}{}", scientific(digits, exponent))
    }
}

/// `digits × 10^(exponent - len + 1)` written positionally; with
/// `point_zero`, integral values keep a `.0` (Python's `repr`).
fn positional(digits: &str, exponent: i32, point_zero: bool) -> String {
    let len = digits.len() as i32;
    let mut out = String::new();
    if exponent < 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-exponent - 1) as usize));
        out.push_str(digits);
    } else if exponent + 1 >= len {
        out.push_str(digits);
        out.extend(std::iter::repeat_n('0', (exponent + 1 - len) as usize));
        if point_zero {
            out.push_str(".0");
        }
    } else {
        let (int, frac) = digits.split_at((exponent + 1) as usize);
        out.push_str(int);
        out.push('.');
        out.push_str(frac);
    }
    out
}

/// `d.ddde±XX` as Python writes exponents (sign and at least two digits).
fn scientific(digits: &str, exponent: i32) -> String {
    let (first, rest) = digits.split_at(1);
    let mut out = first.to_owned();
    if !rest.is_empty() {
        out.push('.');
        out.push_str(rest);
    }
    let sign = if exponent < 0 { '-' } else { '+' };
    let _ = write!(out, "e{sign}{:02}", exponent.abs());
    out
}

/// Python's `round(value * 255)` for one colour channel (ties to even).
fn channel(value: f64) -> u8 {
    (value * 255.0).round_ties_even().clamp(0.0, 255.0) as u8
}

/// matplotlib's `rgb2hex`: `#rrggbb`, alpha ignored.
pub(crate) fn hex_color(color: Rgba) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}

/// Python's `repr(str)` for the printable ASCII strings used as SVG id
/// keys: single quotes unless the text contains one and no double quote.
pub(crate) fn repr_str(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Lowercase hexadecimal digits of `bytes` (Python's `bytes.hex()`).
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Python's `repr(bytes)`: `b'…'` with printable ASCII kept and other
/// bytes escaped.
pub(crate) fn repr_bytes(data: &[u8]) -> String {
    let quote = if data.contains(&b'\'') && !data.contains(&b'"') {
        b'"'
    } else {
        b'\''
    };
    let mut out = String::with_capacity(data.len() * 2 + 3);
    out.push('b');
    out.push(char::from(quote));
    for &byte in data {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b if b == quote => {
                out.push('\\');
                out.push(char::from(b));
            }
            0x20..=0x7e => out.push(char::from(byte)),
            _ => {
                let _ = write!(out, "\\x{byte:02x}");
            }
        }
    }
    out.push(char::from(quote));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_repr() {
        assert_eq!(repr_bytes(b"\x89PNG\r\n"), "b'\\x89PNG\\r\\n'");
        assert_eq!(repr_bytes(b"a'b"), "b\"a'b\"");
    }

    #[test]
    fn trimmed_fixed_point() {
        assert_eq!(short(460.8), "460.8");
        assert_eq!(short(100.0), "100");
        assert_eq!(short(-0.0), "-0");
        assert_eq!(short(1e-7), "0");
        assert_eq!(short(0.123_456_789), "0.123457");
        assert_eq!(trimmed(31.0 / 255.0, 10), "0.1215686275");
        assert_eq!(trimmed(1.0, 3), "1");
        assert_eq!(fixed(10.0, 3), "10.000");
        assert_eq!(short(f64::NAN), "nan");
    }

    #[test]
    fn python_repr() {
        assert_eq!(repr(1.0), "1.0");
        assert_eq!(repr(57.6), "57.6");
        assert_eq!(repr(-0.0), "-0.0");
        assert_eq!(repr(1e-5), "1e-05");
        assert_eq!(repr(0.0001), "0.0001");
        assert_eq!(repr(1e16), "1e+16");
        assert_eq!(repr(123_456_789_012_345.6), "123456789012345.6");
        assert_eq!(repr(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(repr(1.5e300), "1.5e+300");
    }

    #[test]
    fn python_general() {
        assert_eq!(general(73.832_727), "73.8327");
        assert_eq!(general(307.584), "307.584");
        assert_eq!(general(1.0), "1");
        assert_eq!(general(0.0), "0");
        assert_eq!(general(1_000_000.0), "1e+06");
        assert_eq!(general(999_999.5), "1e+06");
        assert_eq!(general(0.000_012_345), "1.2345e-05");
        assert_eq!(general(0.0001), "0.0001");
        assert_eq!(general(-2.5), "-2.5");
        assert_eq!(general(792.0), "792");
    }

    #[test]
    fn colors_and_strings() {
        assert_eq!(hex_color(Rgba::opaque(0.5, 1.0, 0.0)), "#80ff00");
        assert_eq!(
            hex_color(Rgba::opaque(0.121_568_63, 0.466_666_67, 0.705_882_35)),
            "#1f77b4"
        );
        assert_eq!(repr_str("M 0 0 \nL 1 1"), "'M 0 0 \\nL 1 1'");
        assert_eq!(repr_str("it's"), "\"it's\"");
    }
}
