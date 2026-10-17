//! Tolerance-aware structural comparison of JSON values.

use std::fmt::{self, Write as _};

use crate::json::Json;

/// Numeric tolerance: `|a - e| <= abs + rel * max(|a|, |e|)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// Relative tolerance.
    pub rel: f64,
    /// Absolute tolerance.
    pub abs: f64,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            rel: 1e-9,
            abs: 1e-12,
        }
    }
}

impl Tolerance {
    /// A tolerance with the given relative bound and a matching absolute
    /// floor, as used by probes that declare `"tol"`.
    pub fn relative(rel: f64) -> Self {
        Self {
            rel,
            abs: rel * 1e-3,
        }
    }

    /// Whether `actual` is within the tolerance of `expected` (NaN only
    /// matches NaN, infinities only match infinities of the same sign).
    pub fn accepts(self, expected: f64, actual: f64) -> bool {
        if expected.is_nan() || actual.is_nan() {
            return expected.is_nan() && actual.is_nan();
        }
        if expected.is_infinite() || actual.is_infinite() {
            return expected.is_infinite()
                && actual.is_infinite()
                && expected.is_sign_positive() == actual.is_sign_positive();
        }
        (expected - actual).abs() <= self.abs + self.rel * expected.abs().max(actual.abs())
    }
}

/// The first difference found between an expected and an actual value.
#[derive(Clone, Debug, PartialEq)]
pub struct Mismatch {
    /// Location inside the value, e.g. `[2].label`.
    pub path: String,
    /// The expected sub-value.
    pub expected: String,
    /// The actual sub-value.
    pub actual: String,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = if self.path.is_empty() {
            "<root>"
        } else {
            &self.path
        };
        write!(
            f,
            "at {at}: expected {}, got {}",
            self.expected, self.actual
        )
    }
}

/// Compares `actual` against `expected`; numbers use `tol`, everything else
/// must match exactly (object key order is ignored).
pub fn compare(expected: &Json, actual: &Json, tol: Tolerance) -> Result<(), Mismatch> {
    compare_at(expected, actual, tol, &mut String::new())
}

fn compare_at(
    expected: &Json,
    actual: &Json,
    tol: Tolerance,
    path: &mut String,
) -> Result<(), Mismatch> {
    let mismatch = |path: &str| Mismatch {
        path: path.to_owned(),
        expected: truncate(&expected.to_string()),
        actual: truncate(&actual.to_string()),
    };
    match (expected, actual) {
        (Json::Number(_) | Json::Integer(_), Json::Number(_) | Json::Integer(_))
            if tol.accepts(
                expected.as_f64().unwrap_or(f64::NAN),
                actual.as_f64().unwrap_or(f64::NAN),
            ) =>
        {
            Ok(())
        }
        (Json::Array(e), Json::Array(a)) if e.len() == a.len() => {
            for (i, (e, a)) in e.iter().zip(a).enumerate() {
                let len = path.len();
                write!(path, "[{i}]").expect("writing to a String cannot fail");
                compare_at(e, a, tol, path)?;
                path.truncate(len);
            }
            Ok(())
        }
        (Json::Object(e), Json::Object(a)) if e.len() == a.len() => {
            for (key, e_value) in e {
                let a_value = actual.get(key).ok_or_else(|| mismatch(path))?;
                let len = path.len();
                path.push('.');
                path.push_str(key);
                compare_at(e_value, a_value, tol, path)?;
                path.truncate(len);
            }
            Ok(())
        }
        (Json::Number(_) | Json::Integer(_) | Json::Array(_) | Json::Object(_), _) => {
            Err(mismatch(path))
        }
        _ if expected == actual => Ok(()),
        _ => Err(mismatch(path)),
    }
}

fn truncate(text: &str) -> String {
    const LIMIT: usize = 160;
    match text.char_indices().nth(LIMIT) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> Json {
        Json::parse(text).unwrap()
    }

    #[test]
    fn numbers_within_tolerance() {
        let tol = Tolerance::default();
        assert!(compare(&json("[1.0, NaN]"), &json("[1.0000000000001, NaN]"), tol).is_ok());
        let err = compare(&json("[1.0, 2.0]"), &json("[1.0, 2.1]"), tol).unwrap_err();
        assert_eq!(err.path, "[1]");
    }

    #[test]
    fn objects_ignore_key_order_but_not_keys() {
        let tol = Tolerance::default();
        assert!(
            compare(
                &json(r#"{"a": 1, "b": "x"}"#),
                &json(r#"{"b": "x", "a": 1}"#),
                tol
            )
            .is_ok()
        );
        let err = compare(
            &json(r#"{"a": {"b": "x"}}"#),
            &json(r#"{"a": {"b": "y"}}"#),
            tol,
        )
        .unwrap_err();
        assert_eq!(err.path, ".a.b");
        assert!(compare(&json(r#"{"a": 1}"#), &json(r#"{"c": 1}"#), tol).is_err());
    }

    #[test]
    fn type_and_length_mismatches() {
        let tol = Tolerance::default();
        assert!(compare(&json("[1]"), &json("[1, 2]"), tol).is_err());
        assert!(compare(&json("1"), &json("\"1\""), tol).is_err());
        assert!(compare(&json("Infinity"), &json("1e308"), tol).is_err());
    }
}
