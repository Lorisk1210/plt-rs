//! JSON values and strings compared: structures exactly, numbers within
//! a relative 1e-6 (as `cargo xtask diff` compares them), strings as
//! [`text`](super::text) normalizes them, with the images they embed
//! compared by their pixels and the JSON they hold (the messages of
//! matplotlib's web protocol) as JSON.

use std::fmt;

use pltrs_core::json::Json;

use super::media::{compare_images, embedded_images};
use super::text::{Names, normalized};

/// The relative and absolute tolerances of numbers.
const RTOL: f64 = 1e-6;
const ATOL: f64 = 1e-9;

/// The relative tolerance of the numbers in texts (reprs, messages).
const TEXT_RTOL: f64 = 1e-12;

/// The characters of each side a mismatch of strings shows.
const EXCERPT: usize = 60;

/// Where two values differ, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Mismatch {
    /// The keys and indices leading to the difference, outermost first.
    path: Vec<String>,
    /// What differs there.
    detail: String,
}

impl Mismatch {
    /// A difference described by `detail`.
    pub(super) fn new(detail: impl Into<String>) -> Self {
        Self {
            path: Vec::new(),
            detail: detail.into(),
        }
    }

    /// The difference, found under the key or index `segment`.
    pub(super) fn within(mut self, segment: impl Into<String>) -> Self {
        self.path.insert(0, segment.into());
        self
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            f.write_str(&self.detail)
        } else {
            write!(f, "{}: {}", self.path.join("."), self.detail)
        }
    }
}

impl From<String> for Mismatch {
    fn from(detail: String) -> Self {
        Self::new(detail)
    }
}

/// The ids each side's record names so far (see [`Names`]).
#[derive(Clone, Debug, Default)]
pub(super) struct Context {
    /// matplotlib's.
    pub(super) reference: Names,
    /// pltrs'.
    pub(super) pltrs: Names,
}

/// Whether the values `expected` (matplotlib's) and `actual` (pltrs')
/// are the same.
pub(super) fn compare_json(
    expected: &Json,
    actual: &Json,
    cx: &mut Context,
) -> Result<(), Mismatch> {
    match (expected, actual) {
        (Json::Object(a), Json::Object(b)) => {
            if sorted_keys(a) != sorted_keys(b) {
                return Err(Mismatch::new(format!(
                    "keys {:?}, not {:?}",
                    sorted_keys(b),
                    sorted_keys(a)
                )));
            }
            for (key, value) in a {
                let other = actual.get(key).unwrap_or(&Json::Null);
                compare_json(value, other, cx).map_err(|m| m.within(key))?;
            }
            Ok(())
        }
        (Json::Array(a), Json::Array(b)) => {
            if a.len() != b.len() {
                return Err(Mismatch::new(format!("{} items, not {}", b.len(), a.len())));
            }
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                compare_json(x, y, cx).map_err(|m| m.within(format!("[{index}]")))?;
            }
            Ok(())
        }
        (Json::String(a), Json::String(b)) => compare_strings(a, b, cx),
        (Json::Number(a), Json::Number(b)) if close(*a, *b) => Ok(()),
        (Json::Integer(a), Json::Integer(b)) if a == b => Ok(()),
        _ if expected == actual => Ok(()),
        _ => Err(Mismatch::new(format!("{actual}, not {expected}"))),
    }
}

/// The keys of the object `members`, sorted.
fn sorted_keys(members: &[(String, Json)]) -> Vec<&str> {
    let mut keys: Vec<&str> = members.iter().map(|(key, _)| key.as_str()).collect();
    keys.sort_unstable();
    keys
}

/// Whether the numbers `a` and `b` are within the tolerances.
fn close(a: f64, b: f64) -> bool {
    a.total_cmp(&b).is_eq() || (a - b).abs() <= ATOL.max(RTOL * a.abs().max(b.abs()))
}

/// Whether the strings `expected` and `actual` are the same: as JSON
/// when both hold JSON objects or arrays, else with the images they embed
/// compared by their pixels and the rest normalized.
pub(super) fn compare_strings(
    expected: &str,
    actual: &str,
    cx: &mut Context,
) -> Result<(), Mismatch> {
    if let (Some(a), Some(b)) = (json_in(expected), json_in(actual)) {
        return compare_json(&a, &b, cx);
    }
    let (expected_text, expected_images) = embedded_images(expected)?;
    let (actual_text, actual_images) = embedded_images(actual)?;
    if expected_images.len() != actual_images.len() {
        return Err(Mismatch::new(format!(
            "{} images, not {}",
            actual_images.len(),
            expected_images.len()
        )));
    }
    for (index, ((format, a), (other, b))) in expected_images.iter().zip(&actual_images).enumerate()
    {
        if format != other {
            return Err(Mismatch::new(format!(
                "image {index}: {other:?}, not {format:?}"
            )));
        }
        compare_images(*format, a, b)
            .map_err(|err| Mismatch::new(err).within(format!("image {index}")))?;
    }
    let a = normalized(&expected_text, &mut cx.reference);
    let b = normalized(&actual_text, &mut cx.pltrs);
    if a == b || same_but_rounding(&a, &b) {
        Ok(())
    } else {
        Err(Mismatch::new(text_difference(&a, &b)))
    }
}

/// Whether the texts `a` and `b` differ only in numbers that are within
/// [`TEXT_RTOL`] of each other: the last digits of floats that pltrs
/// (Rust) and numpy's vectorized functions round differently (numpy's
/// `exp` on AVX-512, say).
fn same_but_rounding(a: &str, b: &str) -> bool {
    let (a_parts, a_numbers) = split_numbers(a);
    let (b_parts, b_numbers) = split_numbers(b);
    a_parts == b_parts
        && a_numbers.iter().zip(&b_numbers).all(|(x, y)| {
            x.total_cmp(y).is_eq() || (x - y).abs() <= TEXT_RTOL * x.abs().max(y.abs())
        })
}

/// The text between the decimal numbers of `text`, and the numbers.
fn split_numbers(text: &str) -> (Vec<&str>, Vec<f64>) {
    let (mut parts, mut numbers) = (Vec::new(), Vec::new());
    let mut rest = text;
    while let Some(start) = rest.find(|c: char| c.is_ascii_digit()) {
        let digits = |s: &str| s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let mut end = start + digits(&rest[start..]);
        if rest[end..].starts_with('.') {
            end += 1 + digits(&rest[end + 1..]);
        }
        let exponent = rest[end..]
            .strip_prefix(['e', 'E'])
            .map(|e| e.strip_prefix(['+', '-']).unwrap_or(e));
        if let Some(after) = exponent.filter(|e| e.starts_with(|c: char| c.is_ascii_digit())) {
            end = rest.len() - after.len() + digits(after);
        }
        parts.push(&rest[..start]);
        numbers.push(rest[start..end].parse().unwrap_or(f64::NAN));
        rest = &rest[end..];
    }
    parts.push(rest);
    (parts, numbers)
}

/// The JSON object or array `text` holds, if it holds one.
fn json_in(text: &str) -> Option<Json> {
    let trimmed = text.trim_start();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    Json::parse(text).ok()
}

/// Where the texts `expected` and `actual` part, with what each has
/// there.
fn text_difference(expected: &str, actual: &str) -> String {
    let start = expected
        .char_indices()
        .zip(actual.chars())
        .find(|((_, a), b)| a != b)
        .map_or_else(|| expected.len().min(actual.len()), |((at, _), _)| at);
    let start = floor_char_boundary(expected, start.saturating_sub(EXCERPT / 3));
    let excerpt = |text: &str| -> String {
        let from = floor_char_boundary(text, start.min(text.len()));
        text[from..].chars().take(EXCERPT).collect()
    };
    format!("{:?}, not {:?}", excerpt(actual), excerpt(expected))
}

/// The largest char boundary of `text` at or before `at`.
fn floor_char_boundary(text: &str, at: usize) -> usize {
    (0..=at.min(text.len()))
        .rev()
        .find(|&index| text.is_char_boundary(index))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(a: &str, b: &str) -> Result<(), String> {
        let (a, b) = (Json::parse(a).unwrap(), Json::parse(b).unwrap());
        compare_json(&a, &b, &mut Context::default()).map_err(|m| m.to_string())
    }

    #[test]
    fn structures_compare_with_paths() {
        assert_eq!(compare(r#"{"a": [1, 2]}"#, r#"{"a": [1, 2]}"#), Ok(()));
        assert_eq!(
            compare(r#"{"a": [1, 2]}"#, r#"{"a": [1, 3]}"#),
            Err("a.[1]: 3, not 2".to_owned())
        );
        assert_eq!(
            compare(r#"{"a": 1}"#, r#"{"b": 1}"#),
            Err(r#"keys ["b"], not ["a"]"#.to_owned())
        );
        assert_eq!(compare("[1]", "[1, 2]"), Err("2 items, not 1".to_owned()));
        assert_eq!(compare("1.0", "1"), Err("1, not 1.0".to_owned()));
        assert_eq!(compare("0.1", "0.10000000001"), Ok(()));
        assert_eq!(
            compare(r#"{"b": 1, "a": 2}"#, r#"{"a": 2, "b": 1}"#),
            Ok(())
        );
    }

    #[test]
    fn strings_holding_json_compare_as_json() {
        assert_eq!(
            compare(r#""{\"type\": \"draw\"}""#, r#""{\"type\":\"draw\"}""#),
            Ok(())
        );
        assert_eq!(
            compare(r#""{\"type\": \"draw\"}""#, r#""{\"type\": \"ack\"}""#),
            Err(r#"type: "ack", not "draw""#.to_owned())
        );
    }

    #[test]
    fn texts_differing_in_rounding_alone_are_the_same() {
        assert_eq!(
            compare(
                r#""(np.float64(0.8912509381337455), 1)""#,
                r#""(np.float64(0.8912509381337456), 1)""#
            ),
            Ok(())
        );
        assert!(compare(r#""(0.89125, 1)""#, r#""(0.89126, 1)""#).is_err());
        assert!(compare(r#""size 640x480""#, r#""size 640x481""#).is_err());
        assert_eq!(
            compare(
                r#""1.5e-07 and 2E+3""#,
                r#""1.5000000000000001e-07 and 2E+3""#
            ),
            Ok(())
        );
        assert_eq!(
            split_numbers("x=1.25e3, y=-2. z7"),
            (vec!["x=", ", y=-", " z", ""], vec![1250.0, 2.0, 7.0])
        );
    }

    #[test]
    fn strings_compare_normalized() {
        let id = |n: u8| format!("{:032x}", u128::from(n) * 0x1234_5678_9abc);
        let a = format!("\"x {} {} at 0x7f12\"", id(1), id(2));
        let b = format!("\"x {} {} at 0x9a\"", id(3), id(4));
        assert_eq!(compare(&a, &b), Ok(()));
        let swapped = format!("\"x {} {} at 0x9a\"", id(3), id(3));
        assert!(compare(&a, &swapped).is_err());
        assert_eq!(
            compare(
                r#""<matplotlib.lines.Line2D at 0x1>""#,
                r#""<matplotlib.lines.Line3D at 0x1>""#
            ),
            Err(
                r#""atplotlib.lines.Line3D at 0x…>", not "atplotlib.lines.Line2D at 0x…>""#
                    .to_owned()
            )
        );
    }
}
