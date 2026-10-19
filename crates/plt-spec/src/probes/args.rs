//! Typed access to a probe's recorded arguments.

use super::ProbeError;
use crate::json::Json;

/// A probe's argument object with typed, error-reporting getters.
#[derive(Clone, Copy, Debug)]
pub struct Args<'a> {
    raw: &'a Json,
}

impl<'a> Args<'a> {
    /// Wraps a recorded argument object.
    pub const fn new(raw: &'a Json) -> Self {
        Self { raw }
    }

    /// Whether the argument was recorded at all, possibly as `null`.
    pub fn contains(&self, key: &str) -> bool {
        self.raw.get(key).is_some()
    }

    /// The raw argument, if present and not `null`.
    pub fn get(&self, key: &str) -> Option<&'a Json> {
        self.raw.get(key).filter(|v| !v.is_null())
    }

    /// The raw argument, required.
    pub fn json(&self, key: &str) -> Result<&'a Json, ProbeError> {
        self.get(key)
            .ok_or_else(|| ProbeError::BadArgs(format!("missing argument {key:?}")))
    }

    /// A required number.
    pub fn f64(&self, key: &str) -> Result<f64, ProbeError> {
        self.json(key)?
            .as_f64()
            .ok_or_else(|| wrong_type(key, "a number"))
    }

    /// An optional number.
    pub fn opt_f64(&self, key: &str) -> Result<Option<f64>, ProbeError> {
        self.get(key)
            .map(|v| v.as_f64().ok_or_else(|| wrong_type(key, "a number")))
            .transpose()
    }

    /// A required integer.
    pub fn i64(&self, key: &str) -> Result<i64, ProbeError> {
        self.json(key)?
            .as_i64()
            .ok_or_else(|| wrong_type(key, "an integer"))
    }

    /// An optional integer.
    pub fn opt_i64(&self, key: &str) -> Result<Option<i64>, ProbeError> {
        self.get(key)
            .map(|v| v.as_i64().ok_or_else(|| wrong_type(key, "an integer")))
            .transpose()
    }

    /// A required boolean.
    pub fn bool(&self, key: &str) -> Result<bool, ProbeError> {
        self.json(key)?
            .as_bool()
            .ok_or_else(|| wrong_type(key, "a boolean"))
    }

    /// An optional boolean.
    pub fn opt_bool(&self, key: &str) -> Result<Option<bool>, ProbeError> {
        self.get(key)
            .map(|v| v.as_bool().ok_or_else(|| wrong_type(key, "a boolean")))
            .transpose()
    }

    /// A required string.
    pub fn str(&self, key: &str) -> Result<&'a str, ProbeError> {
        self.json(key)?
            .as_str()
            .ok_or_else(|| wrong_type(key, "a string"))
    }

    /// An optional string.
    pub fn opt_str(&self, key: &str) -> Result<Option<&'a str>, ProbeError> {
        self.get(key)
            .map(|v| v.as_str().ok_or_else(|| wrong_type(key, "a string")))
            .transpose()
    }

    /// A required array of numbers.
    pub fn f64_vec(&self, key: &str) -> Result<Vec<f64>, ProbeError> {
        self.json(key)?
            .as_f64_vec()
            .ok_or_else(|| wrong_type(key, "an array of numbers"))
    }

    /// A required 2-D array of numbers, row by row.
    pub fn rows(&self, key: &str) -> Result<Vec<Vec<f64>>, ProbeError> {
        self.json(key)?
            .as_array()
            .and_then(|rows| rows.iter().map(Json::as_f64_vec).collect())
            .ok_or_else(|| wrong_type(key, "a 2-D array of numbers"))
    }

    /// A required 2-D array of booleans, row by row.
    pub fn bool_rows(&self, key: &str) -> Result<Vec<Vec<bool>>, ProbeError> {
        self.json(key)?
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .map(|row| row.as_array()?.iter().map(Json::as_bool).collect())
                    .collect()
            })
            .ok_or_else(|| wrong_type(key, "a 2-D array of booleans"))
    }

    /// A required array of number pairs (e.g. points).
    pub fn pairs(&self, key: &str) -> Result<Vec<(f64, f64)>, ProbeError> {
        self.json(key)?
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| match item.as_f64_vec()?.as_slice() {
                        [x, y] => Some((*x, *y)),
                        _ => None,
                    })
                    .collect()
            })
            .ok_or_else(|| wrong_type(key, "an array of [x, y] pairs"))
    }
}

fn wrong_type(key: &str, expected: &str) -> ProbeError {
    ProbeError::BadArgs(format!("argument {key:?} is not {expected}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_getters() {
        let raw = Json::parse(
            r#"{"x": 1.5, "n": 3, "s": "a", "p": [[1, 2]], "z": null, "m": [[true], [false]]}"#,
        )
        .unwrap();
        let args = Args::new(&raw);
        assert_eq!(args.f64("x"), Ok(1.5));
        assert_eq!(args.i64("n"), Ok(3));
        assert_eq!(args.str("s"), Ok("a"));
        assert_eq!(args.pairs("p"), Ok(vec![(1.0, 2.0)]));
        assert_eq!(args.rows("p"), Ok(vec![vec![1.0, 2.0]]));
        assert_eq!(args.bool_rows("m"), Ok(vec![vec![true], vec![false]]));
        assert!(matches!(args.bool_rows("p"), Err(ProbeError::BadArgs(_))));
        assert_eq!(args.opt_f64("z"), Ok(None));
        assert_eq!(args.opt_f64("missing"), Ok(None));
        assert!(matches!(args.i64("x"), Err(ProbeError::BadArgs(_))));
        assert!(matches!(args.str("missing"), Err(ProbeError::BadArgs(_))));
    }

    #[test]
    fn contains_distinguishes_null_from_absent() {
        let raw = Json::parse(r#"{"z": null, "x": 1}"#).unwrap();
        let args = Args::new(&raw);
        assert!(args.contains("z") && args.get("z").is_none());
        assert!(args.contains("x"));
        assert!(!args.contains("missing"));
    }
}
