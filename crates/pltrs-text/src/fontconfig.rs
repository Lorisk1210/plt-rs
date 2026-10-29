//! fontconfig patterns (`family-size:key=value:constant`), parsed and
//! generated exactly as matplotlib's `_fontconfig_pattern` module does.
//!
//! ```
//! use pltrs_text::fontconfig::FontconfigPattern;
//!
//! let pattern = FontconfigPattern::parse("DejaVu Sans-12:bold:slant=italic")?;
//! assert_eq!(pattern.get("family"), Some(&["DejaVu Sans".to_owned()][..]));
//! assert_eq!(pattern.get("size"), Some(&["12".to_owned()][..]));
//! assert_eq!(pattern.get("weight"), Some(&["bold".to_owned()][..]));
//! # Ok::<(), pltrs_text::TextError>(())
//! ```

use std::collections::HashMap;

use crate::error::TextError;

/// Characters a family name must escape.
const FAMILY_PUNCTUATION: &str = "\\-:,";
/// Characters a property value must escape.
const VALUE_PUNCTUATION: &str = "\\=_:,";

/// Shorthand properties (`:bold`, `:italic`, ...) and what they stand for.
const CONSTANTS: [(&str, &str, &str); 24] = [
    ("thin", "weight", "light"),
    ("extralight", "weight", "light"),
    ("ultralight", "weight", "light"),
    ("light", "weight", "light"),
    ("book", "weight", "book"),
    ("regular", "weight", "regular"),
    ("normal", "weight", "normal"),
    ("medium", "weight", "medium"),
    ("demibold", "weight", "demibold"),
    ("semibold", "weight", "semibold"),
    ("bold", "weight", "bold"),
    ("extrabold", "weight", "extra bold"),
    ("black", "weight", "black"),
    ("heavy", "weight", "heavy"),
    ("roman", "slant", "normal"),
    ("italic", "slant", "italic"),
    ("oblique", "slant", "oblique"),
    ("ultracondensed", "width", "ultra-condensed"),
    ("extracondensed", "width", "extra-condensed"),
    ("condensed", "width", "condensed"),
    ("semicondensed", "width", "semi-condensed"),
    ("expanded", "width", "expanded"),
    ("extraexpanded", "width", "extra-expanded"),
    ("ultraexpanded", "width", "ultra-expanded"),
];

/// A parsed fontconfig pattern: property names and their values in order
/// of first appearance (`family` and `size` first), like the dict
/// matplotlib's `parse_fontconfig_pattern` returns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontconfigPattern {
    properties: Vec<(String, Vec<String>)>,
    /// Position of each key in `properties`.
    index: HashMap<String, usize>,
}

/// A malformed fontconfig pattern and where parsing stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontconfigError {
    /// The pattern.
    pub pattern: String,
    /// The index of the first character not understood.
    pub position: usize,
}

impl FontconfigError {
    /// pyparsing's explanation of the failure, which matplotlib's
    /// `parse_fontconfig_pattern` raises: the pattern, a caret under the
    /// character not understood, and the `ParseException` message.
    ///
    /// ```
    /// use pltrs_text::fontconfig::FontconfigPattern;
    /// let err = FontconfigPattern::try_parse("a:b:c").unwrap_err();
    /// assert_eq!(
    ///     err.explanation(),
    ///     "a:b:c\n ^\nParseException: Expected end of text, found ':'  (at char 1), \
    ///      (line:1, col:2)"
    /// );
    /// ```
    pub fn explanation(&self) -> String {
        let found = self.pattern.chars().nth(self.position).unwrap_or_default();
        format!(
            "{}\n{}^\nParseException: Expected end of text, found '{found}'  (at char {}), \
             (line:1, col:{})",
            self.pattern,
            " ".repeat(self.position),
            self.position,
            self.position + 1
        )
    }
}

impl std::fmt::Display for FontconfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid fontconfig pattern {:?} at character {}",
            self.pattern, self.position
        )
    }
}

impl std::error::Error for FontconfigError {}

impl From<FontconfigError> for TextError {
    fn from(err: FontconfigError) -> Self {
        Self::Font(err.to_string())
    }
}

impl FontconfigPattern {
    /// Parses `pattern`; malformed patterns yield [`TextError::Font`].
    pub fn parse(pattern: &str) -> Result<Self, TextError> {
        Ok(Self::try_parse(pattern)?)
    }

    /// Parses `pattern`, telling where a malformed one stops making sense.
    pub fn try_parse(pattern: &str) -> Result<Self, FontconfigError> {
        let mut scan = Scanner {
            chars: pattern.chars().collect(),
            pos: 0,
        };
        let mut parsed = Self::default();
        let families = scan.comma_separated(|s| Some(s.escaped_text(FAMILY_PUNCTUATION)));
        parsed.extend("family", families.unwrap_or_default());
        let save = scan.pos;
        match scan.eat('-').then(|| scan.comma_separated(Scanner::size)) {
            Some(Some(sizes)) => parsed.extend("size", sizes),
            _ => scan.pos = save,
        }
        loop {
            let save = scan.pos;
            if !scan.eat(':') {
                break;
            }
            let Some((key, values)) = scan.property() else {
                scan.pos = save;
                break;
            };
            parsed.extend(&key, values);
        }
        scan.skip_ws();
        if scan.pos == scan.chars.len() {
            Ok(parsed)
        } else {
            Err(FontconfigError {
                pattern: pattern.to_owned(),
                position: scan.pos,
            })
        }
    }

    /// Every property with its values.
    pub fn properties(&self) -> &[(String, Vec<String>)] {
        &self.properties
    }

    /// The values of one property, if it was given.
    pub fn get(&self, key: &str) -> Option<&[String]> {
        let &at = self.index.get(key)?;
        Some(&self.properties[at].1)
    }

    fn extend(&mut self, key: &str, values: Vec<String>) {
        if let Some(&at) = self.index.get(key) {
            self.properties[at].1.extend(values);
        } else {
            self.index.insert(key.to_owned(), self.properties.len());
            self.properties.push((key.to_owned(), values));
        }
    }
}

/// Escapes a family name for a pattern (a backslash before `\ - : ,`).
pub fn escape_family(family: &str) -> String {
    escape(family, FAMILY_PUNCTUATION)
}

/// Escapes a property value for a pattern (a backslash before `\ = _ : ,`).
pub fn escape_value(value: &str) -> String {
    escape(value, VALUE_PUNCTUATION)
}

fn escape(text: &str, punctuation: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if punctuation.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A cursor over the pattern, skipping whitespace before each element
/// like pyparsing does.
struct Scanner {
    chars: Vec<char>,
    pos: usize,
}

impl Scanner {
    fn skip_ws(&mut self) {
        while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip_ws();
        let found = self.chars.get(self.pos) == Some(&c);
        if found {
            self.pos += 1;
        }
        found
    }

    /// `([^P]|\\[P])*` with escapes removed, for punctuation `P`.
    fn escaped_text(&mut self, punctuation: &str) -> String {
        self.skip_ws();
        let mut out = String::new();
        while let Some(&c) = self.chars.get(self.pos) {
            if c == '\\' {
                match self.chars.get(self.pos + 1) {
                    Some(&next) if punctuation.contains(next) => {
                        out.push(next);
                        self.pos += 2;
                    }
                    _ => break,
                }
            } else if punctuation.contains(c) {
                break;
            } else {
                out.push(c);
                self.pos += 1;
            }
        }
        out
    }

    fn comma_separated(
        &mut self,
        item: impl Fn(&mut Self) -> Option<String>,
    ) -> Option<Vec<String>> {
        let mut items = vec![item(self)?];
        loop {
            let save = self.pos;
            if !self.eat(',') {
                break;
            }
            let Some(value) = item(self) else {
                self.pos = save;
                break;
            };
            items.push(value);
        }
        Some(items)
    }

    /// `[0-9]+\.?[0-9]*|\.[0-9]+`.
    fn size(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.pos;
        let digits = |s: &mut Self| {
            let begin = s.pos;
            while s.chars.get(s.pos).is_some_and(char::is_ascii_digit) {
                s.pos += 1;
            }
            s.pos > begin
        };
        let whole = digits(self);
        if self.chars.get(self.pos) == Some(&'.') {
            self.pos += 1;
            let fraction = digits(self);
            if !whole && !fraction {
                self.pos = start;
                return None;
            }
        } else if !whole {
            return None;
        }
        Some(self.chars[start..self.pos].iter().collect())
    }

    /// `[a-z]+`.
    fn name(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.pos;
        while self
            .chars
            .get(self.pos)
            .is_some_and(char::is_ascii_lowercase)
        {
            self.pos += 1;
        }
        (self.pos > start).then(|| self.chars[start..self.pos].iter().collect())
    }

    /// `name=value,...` or a shorthand constant.
    fn property(&mut self) -> Option<(String, Vec<String>)> {
        let start = self.pos;
        if let Some(name) = self.name()
            && self.eat('=')
        {
            let values = self.comma_separated(|s| Some(s.escaped_text(VALUE_PUNCTUATION)))?;
            return Some((name, values));
        }
        self.pos = start;
        self.skip_ws();
        let rest = &self.chars[self.pos..];
        let (constant, key, value) = CONSTANTS
            .iter()
            .filter(|(constant, _, _)| {
                let mut chars = rest.iter();
                constant.chars().all(|c| chars.next() == Some(&c))
            })
            .max_by_key(|(constant, _, _)| constant.len())?;
        self.pos += constant.len();
        Some(((*key).to_owned(), vec![(*value).to_owned()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(pattern: &str) -> Vec<(String, Vec<String>)> {
        FontconfigPattern::parse(pattern)
            .expect("valid pattern")
            .properties()
            .to_vec()
    }

    fn entry(key: &str, values: &[&str]) -> (String, Vec<String>) {
        (
            key.to_owned(),
            values.iter().map(|v| (*v).to_owned()).collect(),
        )
    }

    #[test]
    fn families_sizes_and_properties() {
        assert_eq!(
            props("Foo,Bar-12.5:bold:style=italic"),
            vec![
                entry("family", &["Foo", "Bar"]),
                entry("size", &["12.5"]),
                entry("weight", &["bold"]),
                entry("style", &["italic"]),
            ]
        );
        assert_eq!(
            props(":extrabold:roman"),
            vec![
                entry("family", &[""]),
                entry("weight", &["extra bold"]),
                entry("slant", &["normal"]),
            ]
        );
        assert_eq!(
            props(":foo=bar"),
            vec![entry("family", &[""]), entry("foo", &["bar"])]
        );
    }

    #[test]
    fn escapes() {
        assert_eq!(props(r"Foo\:Bar"), vec![entry("family", &["Foo:Bar"])]);
        assert_eq!(
            props(r"sans\-serif"),
            vec![entry("family", &["sans-serif"])]
        );
        assert_eq!(escape_family("sans-serif"), r"sans\-serif");
        assert_eq!(escape_value("a_b=c"), r"a\_b\=c");
    }

    #[test]
    fn malformed_patterns() {
        for pattern in ["a:b", "sans-serif", ":boldx", r"Foo\x"] {
            assert!(FontconfigPattern::parse(pattern).is_err(), "{pattern}");
        }
    }

    #[test]
    fn long_patterns_parse_in_linear_time() {
        // Every constant used to copy the rest of the pattern, and every key
        // to scan all earlier keys: both quadratic in the pattern length.
        let constants = ":bold".repeat(100_000);
        assert_eq!(props(&constants)[1].1.len(), 100_000);
        let letters = |mut i: usize| {
            (0..4)
                .map(|_| {
                    let c = char::from(b'a' + (i % 26) as u8);
                    i /= 26;
                    c
                })
                .collect::<String>()
        };
        let keys: String = (0..100_000)
            .flat_map(|i| [":".to_owned(), letters(i), "=v".to_owned()])
            .collect();
        let parsed = FontconfigPattern::parse(&keys).expect("valid pattern");
        assert_eq!(parsed.properties().len(), 100_001);
        assert_eq!(parsed.get(&letters(99_999)), Some(&["v".to_owned()][..]));
    }
}
