//! Number and string formatting exactly as matplotlib's vector backends
//! write them with Python's formatting mini-language; the float layouts
//! come from [`plt_core::pyfmt`].

use std::fmt::Write as _;

use plt_core::pyfmt;
pub(crate) use plt_core::pyfmt::{fixed, repr};

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

/// Python's `%g` (`f"{value:g}"`): six significant digits, trailing zeros
/// removed, scientific notation for exponents outside `-4..6`.
pub(crate) fn general(value: f64) -> String {
    pyfmt::general(value, 6)
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
    fn strings() {
        assert_eq!(repr_str("M 0 0 \nL 1 1"), "'M 0 0 \\nL 1 1'");
        assert_eq!(repr_str("it's"), "\"it's\"");
    }
}
