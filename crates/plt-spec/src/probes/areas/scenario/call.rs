//! The arguments of one scenario step, read the way Python binds them.

use std::cell::Cell;

use plt_core::Point;

use crate::json::Json;
use crate::probes::ProbeError;

/// A method call of a step: its name, positional arguments and keyword
/// arguments. Every argument a handler reads is marked used, so
/// [`Call::finish`] can reject arguments plt-rs would silently ignore.
#[derive(Debug)]
pub(crate) struct Call<'a> {
    /// The method name (the last component of a dotted path).
    pub(crate) method: &'a str,
    positional: &'a [Json],
    keywords: &'a [(String, Json)],
    used: Cell<u64>,
}

/// Keyword arguments are tracked after this many positional ones.
const KEYWORD_BIT: usize = 32;

/// The index to pass for keyword-only parameters.
pub(crate) const KEYWORD_ONLY: usize = usize::MAX;

impl<'a> Call<'a> {
    /// Splits the arguments of a step: a trailing object holds the keyword
    /// arguments unless it constructs an object (has a `new` key).
    pub(crate) fn new(method: &'a str, args: &'a [Json]) -> Result<Self, ProbeError> {
        let (positional, keywords) = match args.split_last() {
            Some((Json::Object(entries), rest)) if !entries.iter().any(|(k, _)| k == "new") => {
                (rest, entries.as_slice())
            }
            _ => (args, &[][..]),
        };
        if positional.len() > KEYWORD_BIT || keywords.len() > 64 - KEYWORD_BIT {
            return Err(ProbeError::BadArgs(format!(
                "too many arguments for {method}"
            )));
        }
        Ok(Self {
            method,
            positional,
            keywords,
            used: Cell::new(0),
        })
    }

    fn mark(&self, bit: usize) {
        self.used.set(self.used.get() | (1 << bit));
    }

    /// The number of positional arguments.
    pub(crate) fn arity(&self) -> usize {
        self.positional.len()
    }

    /// The argument at `index` or named `keyword`, `None` when absent or
    /// `null` (Python's `None` default).
    pub(crate) fn arg(&self, index: usize, keyword: &str) -> Option<&'a Json> {
        if let Some(value) = self.positional.get(index) {
            self.mark(index);
            return Some(value).filter(|v| !v.is_null());
        }
        self.keyword(keyword)
    }

    /// The keyword argument `name`, `None` when absent or `null`.
    pub(crate) fn keyword(&self, name: &str) -> Option<&'a Json> {
        let position = self.keywords.iter().position(|(key, _)| key == name)?;
        self.mark(KEYWORD_BIT + position);
        Some(&self.keywords[position].1).filter(|v| !v.is_null())
    }

    /// Every keyword argument, in order (read each with [`Call::keyword`]
    /// to mark it used).
    pub(crate) fn keywords(&self) -> &'a [(String, Json)] {
        self.keywords
    }

    /// Whether the keyword `name` was given, even as `null`.
    pub(crate) fn has_keyword(&self, name: &str) -> bool {
        self.keywords.iter().any(|(key, _)| key == name)
    }

    /// A required argument.
    pub(crate) fn required(&self, index: usize, keyword: &str) -> Result<&'a Json, ProbeError> {
        self.arg(index, keyword)
            .ok_or_else(|| self.bad(&format!("missing argument {keyword:?}")))
    }

    /// A required number.
    pub(crate) fn f64(&self, index: usize, keyword: &str) -> Result<f64, ProbeError> {
        number(self.required(index, keyword)?).ok_or_else(|| self.wrong(keyword, "a number"))
    }

    /// An optional number.
    pub(crate) fn opt_f64(&self, index: usize, keyword: &str) -> Result<Option<f64>, ProbeError> {
        self.arg(index, keyword)
            .map(|v| number(v).ok_or_else(|| self.wrong(keyword, "a number")))
            .transpose()
    }

    /// An optional boolean.
    pub(crate) fn opt_bool(&self, index: usize, keyword: &str) -> Result<Option<bool>, ProbeError> {
        self.arg(index, keyword)
            .map(|v| v.as_bool().ok_or_else(|| self.wrong(keyword, "a boolean")))
            .transpose()
    }

    /// A required string.
    pub(crate) fn str(&self, index: usize, keyword: &str) -> Result<&'a str, ProbeError> {
        self.required(index, keyword)?
            .as_str()
            .ok_or_else(|| self.wrong(keyword, "a string"))
    }

    /// An optional string.
    pub(crate) fn opt_str(
        &self,
        index: usize,
        keyword: &str,
    ) -> Result<Option<&'a str>, ProbeError> {
        self.arg(index, keyword)
            .map(|v| v.as_str().ok_or_else(|| self.wrong(keyword, "a string")))
            .transpose()
    }

    /// A required array of numbers.
    pub(crate) fn f64_vec(&self, index: usize, keyword: &str) -> Result<Vec<f64>, ProbeError> {
        numbers(self.required(index, keyword)?).ok_or_else(|| self.wrong(keyword, "numbers"))
    }

    /// A required array of `[x, y]` pairs (`np.column_stack` data).
    pub(crate) fn points(&self, index: usize, keyword: &str) -> Result<Vec<Point>, ProbeError> {
        let value = self.required(index, keyword)?;
        value
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .map(|row| match numbers(row)?.as_slice() {
                        [x, y] => Some(Point::new(*x, *y)),
                        _ => None,
                    })
                    .collect()
            })
            .ok_or_else(|| self.wrong(keyword, "[[x, y], ...]"))
    }

    /// A required 2-D array of numbers, row by row.
    pub(crate) fn rows(&self, index: usize, keyword: &str) -> Result<Vec<Vec<f64>>, ProbeError> {
        self.required(index, keyword)?
            .as_array()
            .and_then(|rows| rows.iter().map(numbers).collect())
            .ok_or_else(|| self.wrong(keyword, "a 2-D array of numbers"))
    }

    /// A constructed argument `{"new": name, "args": [...]}`: its name and
    /// arguments, `None` for any other value.
    pub(crate) fn constructed(&self, index: usize, keyword: &str) -> Option<(&'a str, &'a [Json])> {
        let value = self.arg(index, keyword)?;
        let name = value.get("new")?.as_str()?;
        let args = match value.get("args") {
            None => &[][..],
            Some(args) => args.as_array()?,
        };
        Some((name, args))
    }

    /// Rejects the call unless every argument was read by its handler.
    pub(crate) fn finish(&self) -> Result<(), ProbeError> {
        let used = self.used.get();
        let unread = (0..self.positional.len())
            .filter(|&i| used & (1 << i) == 0)
            .map(|i| format!("argument {i}"))
            .chain(
                self.keywords
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| used & (1 << (KEYWORD_BIT + i)) == 0)
                    .map(|(_, (key, _))| format!("keyword {key:?}")),
            )
            .collect::<Vec<_>>();
        if unread.is_empty() {
            Ok(())
        } else {
            Err(self.bad(&format!("unsupported {}", unread.join(", "))))
        }
    }

    /// A harness error about this call.
    pub(crate) fn bad(&self, why: &str) -> ProbeError {
        ProbeError::BadArgs(format!("{}: {why}", self.method))
    }

    fn wrong(&self, keyword: &str, expected: &str) -> ProbeError {
        self.bad(&format!("argument {keyword:?} is not {expected}"))
    }
}

/// A number, with Python's `True`/`False` counting as 1 and 0 the way
/// numpy reads them.
fn number(value: &Json) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_bool().map(|b| f64::from(u8::from(b))))
}

fn numbers(value: &Json) -> Option<Vec<f64>> {
    value.as_array()?.iter().map(number).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<Json> {
        match Json::parse(text).unwrap() {
            Json::Array(items) => items,
            _ => unreachable!(),
        }
    }

    #[test]
    fn trailing_objects_are_keywords_unless_they_construct() {
        let raw = args(r#"[1, null, {"auto": true}]"#);
        let call = Call::new("set_xlim", &raw).unwrap();
        assert_eq!(call.arity(), 2);
        assert_eq!(call.opt_f64(0, "left"), Ok(Some(1.0)));
        assert_eq!(call.opt_f64(1, "right"), Ok(None));
        assert_eq!(call.opt_bool(9, "auto"), Ok(Some(true)));
        assert_eq!(call.finish(), Ok(()));
        let raw = args(r#"[{"new": "MultipleLocator", "args": [1]}]"#);
        assert_eq!(Call::new("set_major_locator", &raw).unwrap().arity(), 1);
    }

    #[test]
    fn unread_arguments_are_reported() {
        let raw = args(r#"[1, {"emit": false}]"#);
        let call = Call::new("set_xlim", &raw).unwrap();
        call.f64(0, "left").unwrap();
        let err = call.finish().unwrap_err();
        assert!(err.to_string().contains("keyword \"emit\""), "{err}");
    }

    #[test]
    fn typed_getters_reject_wrong_shapes() {
        let raw = args(r#"[[[1, 2], [3, 4]], "a", [true, 2]]"#);
        let call = Call::new("m", &raw).unwrap();
        assert_eq!(
            call.points(0, "xy"),
            Ok(vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)])
        );
        assert!(call.f64(1, "s").is_err());
        assert_eq!(call.f64_vec(2, "v"), Ok(vec![1.0, 2.0]));
        assert!(call.str(5, "missing").is_err());
    }
}
