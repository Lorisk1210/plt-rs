//! Feature settings and HarfBuzz's script / language system selection.

use std::fmt;
use std::str::FromStr;

use crate::layout::common::{LangSys, LayoutTable};
use crate::types::Tag;

/// An OpenType feature setting applied to the whole text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Feature {
    /// Feature tag, e.g. `kern`.
    pub tag: Tag,
    /// Feature value: 0 disables, 1 enables, larger values select
    /// alternates.
    pub value: u32,
}

impl Feature {
    /// A feature with an explicit value.
    pub const fn new(tag: Tag, value: u32) -> Self {
        Self { tag, value }
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

    /// Parses HarfBuzz-style global feature strings: `kern`, `+kern`,
    /// `-kern`, `kern=0`, `aalt=2` (ranges are not supported).
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (text, default) = match text.as_bytes().first() {
            Some(b'-') => (&text[1..], 0),
            Some(b'+') => (&text[1..], 1),
            _ => (text, 1),
        };
        let (name, value) = match text.split_once('=') {
            Some((name, value)) => match value.trim() {
                "on" | "true" => (name, 1),
                "off" | "false" => (name, 0),
                number => (name, number.parse().map_err(|_| ParseFeatureError)?),
            },
            None => (text, default),
        };
        let tag = Tag::from_str_padded(name.trim().trim_matches(|c| c == '"' || c == '\''))
            .ok_or(ParseFeatureError)?;
        Ok(Self { tag, value })
    }
}

/// HarfBuzz's script selection: the script's own tag, then `DFLT`, `dflt`
/// and `latn`. Returns the chosen script tag and its language system for
/// `language` (or the default one).
pub(crate) fn select_lang_sys<'a>(
    table: &LayoutTable<'a>,
    script: Option<Tag>,
    language: Option<Tag>,
) -> Option<(Tag, LangSys<'a>)> {
    let candidates = script
        .into_iter()
        .chain([b"DFLT", b"dflt", b"latn"].map(Tag::new));
    let tags: Vec<Tag> = table.script_tags().collect();
    let chosen = candidates.into_iter().find(|tag| tags.contains(tag))?;
    Some((chosen, table.lang_sys(chosen, language)?))
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

/// Lookups of feature `tag` for `script` (with HarfBuzz's script
/// fallbacks), or `None` when the feature is absent.
pub(crate) fn feature_lookups(
    table: &LayoutTable<'_>,
    script: Option<Tag>,
    tag: Tag,
) -> Option<Vec<u16>> {
    let (_, lang_sys) = select_lang_sys(table, script, None)?;
    let (_, lookups) = table.feature(find_feature(table, &lang_sys, tag)?)?;
    Some(lookups.iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::layout_table;

    #[test]
    fn parses_feature_strings() {
        let kern = Tag::new(b"kern");
        assert_eq!("kern".parse(), Ok(Feature::new(kern, 1)));
        assert_eq!("-kern".parse(), Ok(Feature::new(kern, 0)));
        assert_eq!("+kern".parse(), Ok(Feature::new(kern, 1)));
        assert_eq!("kern=0".parse(), Ok(Feature::new(kern, 0)));
        assert_eq!("aalt=2".parse(), Ok(Feature::new(Tag::new(b"aalt"), 2)));
        assert_eq!("liga=off".parse(), Ok(Feature::new(Tag::new(b"liga"), 0)));
        assert_eq!("ss1".parse(), Ok(Feature::new(Tag::new(b"ss1 "), 1)));
        assert_eq!("".parse::<Feature>(), Err(ParseFeatureError));
        assert_eq!("kern=x".parse::<Feature>(), Err(ParseFeatureError));
        assert_eq!("toolong".parse::<Feature>(), Err(ParseFeatureError));
        assert!(ParseFeatureError.to_string().contains("feature"));
    }

    #[test]
    fn script_fallbacks() {
        let data = layout_table(*b"latn", &[(b"liga", &[0])], &[]);
        let table = LayoutTable::parse(&data, Tag::new(b"GSUB"), 7).unwrap();
        let (chosen, _) = select_lang_sys(&table, Some(Tag::new(b"arab")), None).unwrap();
        assert_eq!(chosen, Tag::new(b"latn"));
        let liga = Tag::new(b"liga");
        assert_eq!(feature_lookups(&table, None, liga), Some(vec![0]));
        assert_eq!(feature_lookups(&table, None, Tag::new(b"kern")), None);
    }
}
