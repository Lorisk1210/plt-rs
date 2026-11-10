//! Python's `repr` of a string, which matplotlib's messages write with
//! `{name!r}`.

use std::fmt::Write as _;

/// Python's `repr(text)` for a `str`: in single quotes, or in double
/// quotes when the text holds a single quote and no double quote; the
/// backslash, the quote, `\t`, `\n` and `\r` escaped, and the other
/// control characters as `\xNN` (or `\uNNNN` beyond Latin-1).
///
/// ```
/// use pltrs_core::pyfmt::str_repr;
/// assert_eq!(str_repr("nope.tfm"), "'nope.tfm'");
/// assert_eq!(str_repr("it's"), "\"it's\"");
/// assert_eq!(str_repr("a\\b\n"), "'a\\\\b\\n'");
/// ```
#[must_use]
pub fn str_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut repr = String::with_capacity(text.len() + 2);
    repr.push(quote);
    for c in text.chars() {
        match c {
            '\\' => repr.push_str("\\\\"),
            '\t' => repr.push_str("\\t"),
            '\n' => repr.push_str("\\n"),
            '\r' => repr.push_str("\\r"),
            c if c == quote => {
                repr.push('\\');
                repr.push(c);
            }
            // Writing to a `String` cannot fail.
            c if c.is_control() && u32::from(c) < 0x100 => {
                let _ = write!(repr, "\\x{:02x}", u32::from(c));
            }
            c if c.is_control() => {
                let _ = write!(repr, "\\u{:04x}", u32::from(c));
            }
            c => repr.push(c),
        }
    }
    repr.push(quote);
    repr
}

/// Python's `ascii(text)` for a `str`, which messages write with `%a`:
/// [`str_repr`] with every character beyond ASCII escaped as `\xNN`,
/// `\uNNNN` or `\UNNNNNNNN`.
///
/// ```
/// use pltrs_core::pyfmt::str_ascii;
/// assert_eq!(str_ascii("\u{1f60e}"), "'\\U0001f60e'");
/// assert_eq!(str_ascii("\\sigma é"), "'\\\\sigma \\xe9'");
/// ```
#[must_use]
pub fn str_ascii(text: &str) -> String {
    let mut ascii = String::new();
    for c in str_repr(text).chars() {
        // Writing to a `String` cannot fail.
        let _ = match u32::from(c) {
            code if code < 0x80 => write!(ascii, "{c}"),
            code if code < 0x100 => write!(ascii, "\\x{code:02x}"),
            code if code < 0x1_0000 => write!(ascii, "\\u{code:04x}"),
            code => write!(ascii, "\\U{code:08x}"),
        };
    }
    ascii
}

#[cfg(test)]
mod tests {
    use super::{str_ascii, str_repr};

    #[test]
    fn ascii_escapes_beyond_ascii_only() {
        assert_eq!(str_ascii("plain"), "'plain'");
        assert_eq!(str_ascii("\u{2603}\n"), "'\\u2603\\n'");
        assert_eq!(str_ascii("\u{85}"), "'\\x85'");
        assert_eq!(str_ascii("\u{10ffff}"), "'\\U0010ffff'");
    }

    #[test]
    fn quotes_as_python_does() {
        assert_eq!(str_repr(""), "''");
        assert_eq!(str_repr("a'b\"c"), "'a\\'b\"c'");
        assert_eq!(str_repr("say \"hi\""), "'say \"hi\"'");
        assert_eq!(str_repr("it's"), "\"it's\"");
    }

    #[test]
    fn escapes_control_characters() {
        assert_eq!(
            str_repr("\u{0}\u{1b}\u{7f}\u{85}"),
            "'\\x00\\x1b\\x7f\\x85'"
        );
        assert_eq!(str_repr("tab\there"), "'tab\\there'");
        assert_eq!(str_repr("é—ü"), "'é—ü'");
    }
}
