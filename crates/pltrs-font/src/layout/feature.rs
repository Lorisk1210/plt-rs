//! Feature settings (HarfBuzz's `hb_feature_t`, parsed like
//! `hb_feature_from_string`) and HarfBuzz's script / language system
//! selection.

use std::fmt;
use std::str::FromStr;

use crate::layout::common::{LangSys, LayoutTable};
use crate::types::Tag;

/// An OpenType feature setting: a value for one feature over a range of
/// characters (the whole text unless restricted).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Feature {
    /// Feature tag, e.g. `kern`.
    pub tag: Tag,
    /// Feature value: 0 disables, 1 enables, larger values select
    /// alternates.
    pub value: u32,
    /// Index of the first character the setting applies to.
    pub start: u32,
    /// Index one past the last character the setting applies to
    /// ([`Feature::GLOBAL_END`]: the end of the text).
    pub end: u32,
}

pltrs_core::archive_struct!(Feature {
    tag,
    value,
    start,
    end
});

impl Feature {
    /// The start of a setting for the whole text.
    pub const GLOBAL_START: u32 = 0;
    /// The end of a setting reaching the end of the text.
    pub const GLOBAL_END: u32 = u32::MAX;

    /// A setting of `tag` to `value` for the whole text.
    pub const fn new(tag: Tag, value: u32) -> Self {
        Self {
            tag,
            value,
            start: Self::GLOBAL_START,
            end: Self::GLOBAL_END,
        }
    }

    /// The same setting for characters `start..end` only.
    #[must_use]
    pub const fn with_range(self, start: u32, end: u32) -> Self {
        Self { start, end, ..self }
    }

    /// Whether the setting covers the whole text.
    pub const fn is_global(&self) -> bool {
        self.start == Self::GLOBAL_START && self.end == Self::GLOBAL_END
    }
}

/// Error returned when a feature string cannot be parsed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParseFeatureError;

impl fmt::Display for ParseFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid OpenType feature string")
    }
}

impl std::error::Error for ParseFeatureError {}

impl FromStr for Feature {
    type Err = ParseFeatureError;

    /// Parses HarfBuzz's feature syntax (`hb_feature_from_string`): an
    /// optional `+` or `-`, a tag of up to four characters (or exactly
    /// four in quotes), an optional character range (`[3:5]`, `[3]`,
    /// `[:5]`, `[3:]`) and an optional value (`=2`, `=on`, ` off`), e.g.
    /// `kern`, `-liga`, `ss01`, `aalt=2`, `kern[3:5]=0`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Parser {
            text: text.as_bytes(),
            pos: 0,
        }
        .feature()
        .ok_or(ParseFeatureError)
    }
}

/// Characters HarfBuzz skips between the parts of a feature (`ISSPACE`).
const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Bytes of a number HarfBuzz looks at (`hb_parse_int` copies them into a
/// 32-byte buffer).
const NUMBER_WINDOW: usize = 31;

/// A cursor over a feature string, mirroring HarfBuzz's parsing helpers.
struct Parser<'a> {
    text: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.pos).copied()
    }

    /// `parse_space`.
    fn skip_space(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.pos += 1;
        }
    }

    /// `parse_char`: skips spaces, then consumes `c` if it comes next.
    fn eat(&mut self, c: u8) -> bool {
        self.skip_space();
        let found = self.peek() == Some(c);
        self.pos += usize::from(found);
        found
    }

    /// `hb_parse_int` (C's `strtol` in base 10 over at most 31 bytes, the
    /// result truncated to an `int`), reinterpreted as unsigned like
    /// `parse_uint`; consumes nothing when there is no number.
    fn int(&mut self) -> Option<u32> {
        let end = self.text.len().min(self.pos + NUMBER_WINDOW);
        let window = &self.text[self.pos..end];
        let mut i = window.iter().take_while(|&&b| is_space(b)).count();
        let negative = window.get(i) == Some(&b'-');
        i += usize::from(matches!(window.get(i), Some(b'-' | b'+')));
        let digits = window[i..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if digits == 0 {
            return None;
        }
        // At most 31 digits, which an `i128` holds.
        let magnitude = window[i..i + digits]
            .iter()
            .fold(0i128, |acc, &d| acc * 10 + i128::from(d - b'0'));
        // `strtol` fails on numbers beyond a `long`.
        let value = i64::try_from(if negative { -magnitude } else { magnitude }).ok()?;
        self.pos += i + digits;
        let narrowed = value as i32 as u32;
        Some(narrowed)
    }

    /// `parse_bool`: consumes letters, which mean 1 or 0 when they spell
    /// `on` or `off` in any case.
    fn bool(&mut self) -> Option<u32> {
        self.skip_space();
        let start = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        let word = &self.text[start..self.pos];
        if word.eq_ignore_ascii_case(b"on") {
            Some(1)
        } else if word.eq_ignore_ascii_case(b"off") {
            Some(0)
        } else {
            None
        }
    }

    /// `parse_tag`: up to four bytes ending at a space, `=`, `[` or the
    /// closing quote; quoted tags have exactly four.
    fn tag(&mut self) -> Option<Tag> {
        self.skip_space();
        let quote = self.peek().filter(|&b| b == b'\'' || b == b'"');
        self.pos += usize::from(quote.is_some());
        let stop = quote.unwrap_or(0);
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|b| !matches!(b, b' ' | b'=' | b'[') && b != stop)
        {
            self.pos += 1;
        }
        let name = &self.text[start..self.pos];
        if name.is_empty() || name.len() > 4 {
            return None;
        }
        if let Some(quote) = quote {
            if name.len() != 4 || self.peek() != Some(quote) {
                return None;
            }
            self.pos += 1;
        }
        // `hb_tag_from_string` stops at a NUL and pads with spaces.
        let mut bytes = [b' '; 4];
        for (byte, &c) in bytes.iter_mut().zip(name.iter().take_while(|&&c| c != 0)) {
            *byte = c;
        }
        Some(Tag::new(&bytes))
    }

    /// `parse_feature_indices`: the optional `[start:end]` range.
    fn range(&mut self) -> Option<(u32, u32)> {
        self.skip_space();
        if !self.eat(b'[') {
            return Some((Feature::GLOBAL_START, Feature::GLOBAL_END));
        }
        let start = self.int();
        let end = if self.eat(b':') || self.eat(b';') {
            self.int().unwrap_or(Feature::GLOBAL_END)
        } else {
            start.map_or(Feature::GLOBAL_END, |start| start.wrapping_add(1))
        };
        self.eat(b']')
            .then_some((start.unwrap_or(Feature::GLOBAL_START), end))
    }

    /// `parse_feature_value_postfix`: the value after the tag, `prefix`
    /// when there is none; `None` when `=` has no value after it.
    fn value(&mut self, prefix: u32) -> Option<u32> {
        let equal = self.eat(b'=');
        match self.int().or_else(|| self.bool()) {
            Some(value) => Some(value),
            None if equal => None,
            None => Some(prefix),
        }
    }

    /// `parse_one_feature`.
    fn feature(&mut self) -> Option<Feature> {
        let prefix = if self.eat(b'-') {
            0
        } else {
            self.eat(b'+');
            1
        };
        let tag = self.tag()?;
        let (start, end) = self.range()?;
        let value = self.value(prefix)?;
        self.skip_space();
        (self.pos == self.text.len()).then_some(Feature {
            tag,
            value,
            start,
            end,
        })
    }
}

/// HarfBuzz's script selection (`hb_ot_layout_table_select_script`): the
/// first of `scripts` the table has, then `DFLT`, `dflt` and `latn`.
/// Returns the chosen script tag and its language system for the first
/// of `languages` it has (see [`LayoutTable::lang_sys`]).
pub(crate) fn select_lang_sys<'a>(
    table: &LayoutTable<'a>,
    scripts: &[Tag],
    languages: &[Tag],
) -> Option<(Tag, LangSys<'a>)> {
    let tags: Vec<Tag> = table.script_tags().collect();
    let chosen = scripts
        .iter()
        .copied()
        .chain([b"DFLT", b"dflt", b"latn"].map(Tag::new))
        .find(|tag| tags.contains(tag))?;
    Some((chosen, table.lang_sys(chosen, languages)?))
}

/// Index of the first feature of `lang_sys` with `tag`.
pub(crate) fn find_feature(
    table: &LayoutTable<'_>,
    lang_sys: &LangSys<'_>,
    tag: Tag,
) -> Option<u16> {
    lang_sys
        .feature_indices
        .iter()
        .find(|&index| table.feature(index).is_some_and(|(t, _)| t == tag))
}

/// Lookups of feature `tag` in the default language system of `script`
/// (with HarfBuzz's script fallbacks), or `None` when the feature is
/// absent.
pub(crate) fn feature_lookups(
    table: &LayoutTable<'_>,
    script: Option<Tag>,
    tag: Tag,
) -> Option<Vec<u16>> {
    let (_, lang_sys) = select_lang_sys(table, script.as_slice(), &[])?;
    let (_, lookups) = table.feature(find_feature(table, &lang_sys, tag)?)?;
    Some(lookups.iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::{layout_table, layout_table_with_languages};

    const KERN: Tag = Tag::new(b"kern");

    /// Tag, value and range of a valid feature string.
    fn parse(text: &str) -> (String, u32, u32, u32) {
        let feature: Feature = text.parse().expect("valid feature string");
        (
            feature.tag.to_string(),
            feature.value,
            feature.start,
            feature.end,
        )
    }

    fn ranged(tag: &str, value: u32, start: u32, end: u32) -> (String, u32, u32, u32) {
        (tag.to_owned(), value, start, end)
    }

    fn global(tag: &str, value: u32) -> (String, u32, u32, u32) {
        ranged(tag, value, 0, u32::MAX)
    }

    #[test]
    fn parses_values_and_prefixes() {
        assert_eq!("kern".parse(), Ok(Feature::new(KERN, 1)));
        assert_eq!("-kern".parse(), Ok(Feature::new(KERN, 0)));
        assert_eq!("+kern".parse(), Ok(Feature::new(KERN, 1)));
        assert_eq!(parse("kern=0"), global("kern", 0));
        assert_eq!(parse("aalt=2"), global("aalt", 2));
        assert_eq!(parse("liga=off"), global("liga", 0));
        assert_eq!(parse("liga=ON"), global("liga", 1));
        assert_eq!(parse("ss1"), global("ss1 ", 1));
        assert_eq!(parse(" - kern = 3 "), global("kern", 3));
        assert_eq!(parse("kern=+5"), global("kern", 5));
        assert_eq!(parse("kern=-1"), global("kern", u32::MAX));
        assert_eq!(
            parse("kern=4294967297"),
            global("kern", 1),
            "narrowed to int"
        );
        // HarfBuzz quirks: a value without `=` overrides the prefix, and
        // letters that are neither on nor off are dropped.
        assert_eq!(parse("-kern on"), global("kern", 1));
        assert_eq!(parse("kern 7"), global("kern", 7));
        assert_eq!(parse("kern xyz"), global("kern", 1));
        assert_eq!(parse("ke rn"), global("ke  ", 1));
        assert_eq!(parse("'kern'"), global("kern", 1));
        assert_eq!(parse("\"liga\"=0"), global("liga", 0));
    }

    #[test]
    fn parses_ranges() {
        assert_eq!(parse("kern[3:5]"), ranged("kern", 1, 3, 5));
        assert_eq!(parse("kern[3]"), ranged("kern", 1, 3, 4));
        assert_eq!(parse("kern[3:]"), ranged("kern", 1, 3, u32::MAX));
        assert_eq!(parse("kern[:5]=0"), ranged("kern", 0, 0, 5));
        assert_eq!(parse("kern[2;4]"), ranged("kern", 1, 2, 4));
        assert_eq!(parse("kern[]"), global("kern", 1));
        assert_eq!(parse("kern[:]"), global("kern", 1));
        assert_eq!(parse("aalt[3:5]=2"), ranged("aalt", 2, 3, 5));
        assert_eq!(parse("kern[-1]"), ranged("kern", 1, u32::MAX, 0));
        let feature: Feature = "kern[1:2]".parse().expect("valid");
        assert!(!feature.is_global());
        assert_eq!(feature, Feature::new(KERN, 1).with_range(1, 2));
    }

    #[test]
    fn rejects_malformed_features() {
        for text in [
            "", "  ", "-", "toolong", "kern=", "kern=x", "kern=1.5", "=1", "kern[", "kern[3",
            "kern[3:", "kern]", "'ker'", "'kern", "kern\t", "kern=1 2", "kern==1",
        ] {
            assert_eq!(text.parse::<Feature>(), Err(ParseFeatureError), "{text:?}");
        }
        let spaced = format!("kern={}1", " ".repeat(31));
        assert_eq!(
            spaced.parse::<Feature>(),
            Err(ParseFeatureError),
            "past the window"
        );
        assert!(ParseFeatureError.to_string().contains("feature"));
    }

    #[test]
    fn language_systems_fall_back_to_dflt_then_the_default() {
        let features: &[(&[u8; 4], &[u16])] = &[(b"liga", &[]), (b"locl", &[]), (b"case", &[])];
        let (trk, aze) = (Tag::new(b"TRK "), Tag::new(b"AZE "));
        let chosen = |languages: &[(&[u8; 4], &[u16])], asked: &[Tag]| {
            let data = layout_table_with_languages(*b"latn", languages, features, &[]);
            let table = LayoutTable::parse(&data, Tag::new(b"GSUB"), 7).unwrap();
            let (_, lang_sys) = select_lang_sys(&table, &[Tag::new(b"latn")], asked).unwrap();
            lang_sys.feature_indices.iter().collect::<Vec<u16>>()
        };
        let with_dflt: &[(&[u8; 4], &[u16])] = &[(b"TRK ", &[1]), (b"dflt", &[2])];
        assert_eq!(chosen(with_dflt, &[aze, trk]), [1], "first candidate found");
        assert_eq!(chosen(with_dflt, &[aze]), [2], "the dflt language system");
        assert_eq!(chosen(with_dflt, &[]), [2]);
        assert_eq!(chosen(&[(b"TRK ", &[1])], &[aze]), [0, 1, 2], "the default");
    }

    #[test]
    fn script_fallbacks() {
        let data = layout_table(*b"latn", &[(b"liga", &[0])], &[]);
        let table = LayoutTable::parse(&data, Tag::new(b"GSUB"), 7).unwrap();
        let (chosen, _) = select_lang_sys(&table, &[Tag::new(b"arab")], &[]).unwrap();
        assert_eq!(chosen, Tag::new(b"latn"));
        let liga = Tag::new(b"liga");
        assert_eq!(feature_lookups(&table, None, liga), Some(vec![0]));
        assert_eq!(feature_lookups(&table, None, KERN), None);
    }
}
