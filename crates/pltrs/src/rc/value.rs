//! Parsing and formatting of rcParam values in rc-file syntax.

use pltrs_render::{CapStyle, JoinStyle};

use super::RcError;
use crate::markers::{Marker, MarkerSymbol};

/// A type that can be read from and written to rc-file text.
pub(crate) trait RcValue: Sized {
    /// Parses rc-file text, explaining the problem on failure.
    fn parse_rc(text: &str) -> Result<Self, String>;
    /// The rc-file text for the value.
    fn format_rc(&self) -> String;
    /// Whether `text` means `None` where this type is optional: exactly
    /// `None` for most validators, `none` in any case for matplotlib's
    /// plain type validators (`validate_float_or_None` and the like).
    fn is_none_text(text: &str) -> bool {
        text == "None"
    }
}

/// `text` without the underscores Python allows between digits of a
/// number (`1_000`), or `None` when an underscore is misplaced.
fn without_digit_separators(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let placed = bytes.iter().enumerate().all(|(i, &byte)| {
        byte != b'_'
            || (i > 0
                && bytes[i - 1].is_ascii_digit()
                && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
    });
    placed.then(|| text.replace('_', ""))
}

/// Parses `text` for `key` with `parse` and runs the parameter's extra
/// validation.
pub(crate) fn parse_checked<T>(
    key: &str,
    text: &str,
    parse: impl Fn(&str) -> Result<T, String>,
    check: impl Fn(&T) -> Result<(), String>,
) -> Result<T, RcError> {
    let invalid = |reason: String| RcError::InvalidValue {
        key: key.to_owned(),
        value: text.to_owned(),
        reason,
    };
    let value = parse(text.trim()).map_err(invalid)?;
    check(&value).map_err(invalid)?;
    Ok(value)
}

impl RcValue for bool {
    fn parse_rc(text: &str) -> Result<Self, String> {
        match text.to_ascii_lowercase().as_str() {
            "t" | "y" | "yes" | "on" | "true" | "1" => Ok(true),
            "f" | "n" | "no" | "off" | "false" | "0" => Ok(false),
            _ => Err(format!("cannot convert {text:?} to bool")),
        }
    }
    fn format_rc(&self) -> String {
        if *self { "True" } else { "False" }.to_owned()
    }
}

impl RcValue for f64 {
    /// Python's `float`: digit separators allowed, `inf` and `nan` in any
    /// case.
    fn parse_rc(text: &str) -> Result<Self, String> {
        without_digit_separators(text)
            .and_then(|digits| digits.parse().ok())
            .ok_or_else(|| format!("could not convert {text:?} to float"))
    }
    fn format_rc(&self) -> String {
        format!("{self:?}")
    }
    fn is_none_text(text: &str) -> bool {
        text.eq_ignore_ascii_case("none")
    }
}

impl RcValue for i64 {
    /// Python's `int`: digit separators allowed.
    fn parse_rc(text: &str) -> Result<Self, String> {
        without_digit_separators(text)
            .and_then(|digits| digits.parse().ok())
            .ok_or_else(|| format!("could not convert {text:?} to int"))
    }
    fn format_rc(&self) -> String {
        self.to_string()
    }
    fn is_none_text(text: &str) -> bool {
        text.eq_ignore_ascii_case("none")
    }
}

impl RcValue for String {
    fn parse_rc(text: &str) -> Result<Self, String> {
        Ok(text.to_owned())
    }
    fn format_rc(&self) -> String {
        self.clone()
    }
    fn is_none_text(text: &str) -> bool {
        text.eq_ignore_ascii_case("none")
    }
}

impl<T: RcValue> RcValue for Option<T> {
    fn parse_rc(text: &str) -> Result<Self, String> {
        if T::is_none_text(text) {
            Ok(None)
        } else {
            T::parse_rc(text).map(Some)
        }
    }
    fn format_rc(&self) -> String {
        self.as_ref()
            .map_or_else(|| "None".to_owned(), RcValue::format_rc)
    }
}

/// Splits a comma-separated list, dropping empty items (matplotlib's
/// `_listify_validator`, which takes no brackets).
pub(crate) fn split_list(text: &str) -> impl Iterator<Item = &str> {
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// The numbers rc lists hold.
pub(crate) trait ListItem: RcValue {}

impl ListItem for f64 {}

impl ListItem for i64 {}

impl RcValue for Vec<String> {
    fn parse_rc(text: &str) -> Result<Self, String> {
        Ok(text
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect())
    }
    fn format_rc(&self) -> String {
        self.join(", ")
    }
}

impl<T: ListItem> RcValue for Vec<T> {
    fn parse_rc(text: &str) -> Result<Self, String> {
        split_list(text).map(T::parse_rc).collect()
    }
    fn format_rc(&self) -> String {
        format_list(self)
    }
}

impl<T: ListItem, const N: usize> RcValue for [T; N] {
    fn parse_rc(text: &str) -> Result<Self, String> {
        let values = Vec::<T>::parse_rc(text)?;
        <[T; N]>::try_from(values).map_err(|v| format!("expected {N} values, got {}", v.len()))
    }
    fn format_rc(&self) -> String {
        format_list(self)
    }
}

/// The rc-file text of a list of numbers.
fn format_list<T: RcValue>(items: &[T]) -> String {
    items
        .iter()
        .map(RcValue::format_rc)
        .collect::<Vec<_>>()
        .join(", ")
}

impl RcValue for CapStyle {
    fn parse_rc(text: &str) -> Result<Self, String> {
        Self::from_name(text).ok_or_else(|| format!("{text:?} is not a valid CapStyle"))
    }
    fn format_rc(&self) -> String {
        self.name().to_owned()
    }
}

impl RcValue for JoinStyle {
    fn parse_rc(text: &str) -> Result<Self, String> {
        Self::from_name(text).ok_or_else(|| format!("{text:?} is not a valid JoinStyle"))
    }
    fn format_rc(&self) -> String {
        self.name().to_owned()
    }
}

impl RcValue for Marker {
    /// matplotlib's `_validate_marker`: an integer code first (`"4"` is
    /// `CARETLEFT`, not the `"4"` tri), then a marker text.
    fn parse_rc(text: &str) -> Result<Self, String> {
        match i64::parse_rc(text) {
            Ok(code) => Self::from_code(code).map_err(|err| err.to_string()),
            Err(_) => text.parse().map_err(|err| format!("{err}")),
        }
    }
    fn format_rc(&self) -> String {
        let Self::Symbol(symbol) = self else {
            return "custom".to_owned();
        };
        MarkerSymbol::all().find(|(_, s)| s == symbol).map_or_else(
            String::new,
            |(key, _)| match key {
                crate::markers::MarkerKey::Char(c) => c.to_owned(),
                crate::markers::MarkerKey::Code(code) => code.to_string(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booleans_accept_matplotlib_spellings() {
        for yes in ["True", "yes", "ON", "1", "t"] {
            assert_eq!(bool::parse_rc(yes), Ok(true), "{yes}");
        }
        for no in ["False", "no", "off", "0", "F"] {
            assert_eq!(bool::parse_rc(no), Ok(false), "{no}");
        }
        assert!(bool::parse_rc("maybe").is_err());
    }

    #[test]
    fn lists_and_options() {
        assert_eq!(Vec::<f64>::parse_rc("3.7, 1.6"), Ok(vec![3.7, 1.6]));
        assert!(Vec::<f64>::parse_rc("[1, 2]").is_err(), "no brackets");
        assert_eq!(<[i64; 2]>::parse_rc("-5, 6"), Ok([-5, 6]));
        assert!(<[i64; 2]>::parse_rc("-5").is_err());
        assert_eq!(
            Vec::<String>::parse_rc("a, , b"),
            Ok(vec!["a".to_owned(), "b".to_owned()])
        );
        assert_eq!(Option::<f64>::parse_rc("None"), Ok(None));
        assert_eq!(Option::<f64>::parse_rc("none"), Ok(None));
        assert_eq!(f64::parse_rc("1_000.5"), Ok(1000.5));
        assert_eq!(i64::parse_rc("1_000"), Ok(1000));
        assert!(i64::parse_rc("1__000").is_err() && f64::parse_rc("_1").is_err());
        assert_eq!(Option::<f64>::parse_rc("2.5"), Ok(Some(2.5)));
        assert!(<[f64; 3]>::parse_rc("1, 2").is_err());
    }

    #[test]
    fn markers_and_styles() {
        assert_eq!(
            Marker::parse_rc("o"),
            Ok(Marker::Symbol(MarkerSymbol::Circle))
        );
        assert_eq!(
            Marker::parse_rc("7"),
            Ok(Marker::Symbol(MarkerSymbol::CaretDown))
        );
        assert_eq!(
            Marker::parse_rc("4"),
            Ok(Marker::Symbol(MarkerSymbol::CaretLeft)),
            "integer codes come first"
        );
        assert_eq!(
            Marker::parse_rc("None").map(|m| m.format_rc()),
            Ok("None".to_owned())
        );
        assert_eq!(CapStyle::parse_rc("projecting"), Ok(CapStyle::Projecting));
        assert!(JoinStyle::parse_rc("mitre").is_err());
    }
}
