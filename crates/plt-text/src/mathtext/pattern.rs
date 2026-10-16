//! fontconfig patterns (`family-size:key=value:constant`), as matplotlib's
//! `_fontconfig_pattern.parse_fontconfig_pattern` reads them, turned into
//! font queries the way `FontProperties(pattern)` does.

use plt_font::{FontQuery, FontStyle, FontVariant, Stretch, Weight};

use crate::error::TextError;

/// Characters a family name must escape.
const FAMILY_PUNCTUATION: &str = "\\-:,";
/// Characters a property value must escape.
const VALUE_PUNCTUATION: &str = "\\=_:,";

/// Shorthand properties (`:bold`, `:italic`, ...).
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

/// A cursor over the pattern, skipping whitespace before each element
/// like pyparsing does.
struct Scanner<'a> {
    chars: Vec<char>,
    pos: usize,
    pattern: &'a str,
}

impl Scanner<'_> {
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
        let rest: String = self.chars[self.pos..].iter().collect();
        let (constant, key, value) = CONSTANTS
            .iter()
            .filter(|(constant, _, _)| rest.starts_with(constant))
            .max_by_key(|(constant, _, _)| constant.len())?;
        self.pos += constant.len();
        Some(((*key).to_owned(), vec![(*value).to_owned()]))
    }

    fn error(&self) -> TextError {
        TextError::Font(format!("invalid fontconfig pattern {:?}", self.pattern))
    }
}

/// Parses a fontconfig pattern into a font query (only the first family
/// and the first value of each property count, as in matplotlib).
pub(crate) fn parse_pattern(pattern: &str) -> Result<FontQuery, TextError> {
    let mut scan = Scanner {
        chars: pattern.chars().collect(),
        pos: 0,
        pattern,
    };
    let families = scan
        .comma_separated(|s| Some(s.escaped_text(FAMILY_PUNCTUATION)))
        .ok_or_else(|| scan.error())?;
    let save = scan.pos;
    if !(scan.eat('-') && scan.comma_separated(Scanner::size).is_some()) {
        scan.pos = save;
    }
    let mut properties: Vec<(String, Vec<String>)> = Vec::new();
    loop {
        let save = scan.pos;
        if !scan.eat(':') {
            break;
        }
        let Some((key, values)) = scan.property() else {
            scan.pos = save;
            break;
        };
        match properties.iter_mut().find(|(k, _)| *k == key) {
            Some((_, existing)) => existing.extend(values),
            None => properties.push((key, values)),
        }
    }
    scan.skip_ws();
    if scan.pos != scan.chars.len() {
        return Err(scan.error());
    }
    let mut query = FontQuery::families([families[0].clone()]);
    for (key, values) in &properties {
        let value = values[0].as_str();
        let invalid = || TextError::Font(format!("invalid font {key} {value:?} in {pattern:?}"));
        match key.as_str() {
            "family" => {}
            "size" => {
                value.parse::<f64>().map_err(|_| invalid())?;
            }
            "weight" => query.weight = Weight::parse(value).ok_or_else(invalid)?,
            "slant" | "style" => query.style = FontStyle::parse(value).ok_or_else(invalid)?,
            "width" | "stretch" => query.stretch = Stretch::parse(value).ok_or_else(invalid)?,
            "variant" => {
                query.variant = match value {
                    "normal" => FontVariant::Normal,
                    "small-caps" => FontVariant::SmallCaps,
                    _ => return Err(invalid()),
                };
            }
            _ => return Err(invalid()),
        }
    }
    Ok(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_and_constants() {
        let query = parse_pattern("sans:italic:bold").expect("valid");
        assert_eq!(query.families, vec!["sans"]);
        assert_eq!(query.style, FontStyle::Italic);
        assert_eq!(query.weight, Weight::BOLD);
        let query = parse_pattern("DejaVu Serif:weight=bold").expect("valid");
        assert_eq!(query.families, vec!["DejaVu Serif"]);
        assert_eq!(query.weight, Weight::BOLD);
    }

    #[test]
    fn empty_family_and_first_values_win() {
        let query = parse_pattern(":bold").expect("valid");
        assert_eq!(query.families, vec![""]);
        let query = parse_pattern("a,b-12:light:bold:width=condensed").expect("valid");
        assert_eq!(query.families, vec!["a"]);
        assert_eq!(query.weight, Weight(200));
        assert_eq!(query.stretch, Stretch::CONDENSED);
        let query = parse_pattern(r"sans\-serif:slant=oblique").expect("valid");
        assert_eq!(query.families, vec!["sans-serif"]);
        assert_eq!(query.style, FontStyle::Oblique);
    }

    #[test]
    fn invalid_patterns() {
        assert!(parse_pattern("sans-serif").is_err());
        assert!(parse_pattern("sans:weight=fat").is_err());
        assert!(parse_pattern("sans:colour=red").is_err());
        assert!(parse_pattern("sans:variant=tiny").is_err());
    }
}
