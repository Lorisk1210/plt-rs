//! Parsing and formatting of rcParam values in rc-file syntax.

use plt_render::{CapStyle, JoinStyle};

use super::RcError;
use crate::markers::{Marker, MarkerSymbol};

/// A type that can be read from and written to rc-file text.
pub(crate) trait RcValue: Sized {
    /// Parses rc-file text, explaining the problem on failure.
    fn parse_rc(text: &str) -> Result<Self, String>;
    /// The rc-file text for the value.
    fn format_rc(&self) -> String;
}

/// Parses `text` for `key` and runs the parameter's extra validation.
pub(crate) fn parse_checked<T: RcValue>(
    key: &str,
    text: &str,
    check: impl Fn(&T) -> Result<(), String>,
) -> Result<T, RcError> {
    let invalid = |reason: String| RcError::InvalidValue {
        key: key.to_owned(),
        value: text.to_owned(),
        reason,
    };
    let value = T::parse_rc(text.trim()).map_err(invalid)?;
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
    fn parse_rc(text: &str) -> Result<Self, String> {
        text.parse()
            .map_err(|_| format!("could not convert {text:?} to float"))
    }
    fn format_rc(&self) -> String {
        format!("{self:?}")
    }
}

impl RcValue for i64 {
    fn parse_rc(text: &str) -> Result<Self, String> {
        text.parse()
            .map_err(|_| format!("could not convert {text:?} to int"))
    }
    fn format_rc(&self) -> String {
        self.to_string()
    }
}

impl RcValue for String {
    fn parse_rc(text: &str) -> Result<Self, String> {
        Ok(text.to_owned())
    }
    fn format_rc(&self) -> String {
        self.clone()
    }
}

impl<T: RcValue> RcValue for Option<T> {
    fn parse_rc(text: &str) -> Result<Self, String> {
        if text == "None" {
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
/// `_listify_validator`); surrounding brackets are tolerated.
pub(crate) fn split_list(text: &str) -> impl Iterator<Item = &str> {
    let inner = text
        .strip_prefix(['[', '('])
        .and_then(|t| t.strip_suffix([']', ')']))
        .unwrap_or(text);
    inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

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

impl RcValue for Vec<f64> {
    fn parse_rc(text: &str) -> Result<Self, String> {
        split_list(text).map(f64::parse_rc).collect()
    }
    fn format_rc(&self) -> String {
        self.iter()
            .map(RcValue::format_rc)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl RcValue for Vec<i64> {
    fn parse_rc(text: &str) -> Result<Self, String> {
        split_list(text).map(i64::parse_rc).collect()
    }
    fn format_rc(&self) -> String {
        self.iter()
            .map(RcValue::format_rc)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl<const N: usize> RcValue for [f64; N] {
    fn parse_rc(text: &str) -> Result<Self, String> {
        let values = Vec::<f64>::parse_rc(text)?;
        <[f64; N]>::try_from(values).map_err(|v| format!("expected {N} values, got {}", v.len()))
    }
    fn format_rc(&self) -> String {
        self.iter()
            .map(RcValue::format_rc)
            .collect::<Vec<_>>()
            .join(", ")
    }
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
    fn parse_rc(text: &str) -> Result<Self, String> {
        text.parse().or_else(|err| {
            text.parse::<i64>()
                .map_err(|_| format!("{err}"))
                .and_then(|code| Self::from_code(code).map_err(|e| e.to_string()))
        })
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
        assert_eq!(Vec::<f64>::parse_rc("[1, 2]"), Ok(vec![1.0, 2.0]));
        assert_eq!(
            Vec::<String>::parse_rc("a, , b"),
            Ok(vec!["a".to_owned(), "b".to_owned()])
        );
        assert_eq!(Option::<f64>::parse_rc("None"), Ok(None));
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
            Marker::parse_rc("None").map(|m| m.format_rc()),
            Ok("None".to_owned())
        );
        assert_eq!(CapStyle::parse_rc("projecting"), Ok(CapStyle::Projecting));
        assert!(JoinStyle::parse_rc("mitre").is_err());
    }
}
