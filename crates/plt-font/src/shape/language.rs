//! The languages of shaped text: BCP 47 language tags compared the way
//! HarfBuzz interns them (`hb_language_from_string`), and the character
//! ranges libraqm gives them (`raqm_set_language`).

use std::fmt;
use std::hash::{Hash, Hasher};

/// A BCP 47 language tag (`"sr"`, `"tr-TR"`, `"zh-Hant-HK"`).
///
/// Tags compare as HarfBuzz interns them: ASCII letters in either case,
/// `_` read as `-`, and the tag cut at the first character that is not an
/// ASCII letter, digit, `-` or `_` (so `"sr@latin"` is `"sr"`). A tag
/// that is empty after that (HarfBuzz's invalid language for `""`) names
/// no language system, like an unknown language.
#[derive(Clone)]
pub struct LanguageTag {
    text: Box<str>,
    canonical: Box<str>,
}

impl LanguageTag {
    /// The tag `text`, as written.
    pub fn new(text: &str) -> Self {
        let canonical = text
            .bytes()
            .map_while(|byte| match byte {
                b'a'..=b'z' | b'0'..=b'9' | b'-' => Some(char::from(byte)),
                b'A'..=b'Z' => Some(char::from(byte.to_ascii_lowercase())),
                b'_' => Some('-'),
                _ => None,
            })
            .collect();
        Self {
            text: text.into(),
            canonical,
        }
    }

    /// The tag as written.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The tag as HarfBuzz interns it (lower case, `-` separated).
    pub fn canonical(&self) -> &str {
        &self.canonical
    }
}

impl PartialEq for LanguageTag {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for LanguageTag {}

impl Hash for LanguageTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical.hash(state);
    }
}

impl fmt::Debug for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LanguageTag({:?})", self.text)
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<&str> for LanguageTag {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for LanguageTag {
    fn from(text: String) -> Self {
        Self::new(&text)
    }
}

/// A language for the characters `start..end` of a text (indices count
/// characters, i.e. Unicode scalar values), as libraqm's
/// `raqm_set_language` sets it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LanguageRange {
    /// The language.
    pub tag: LanguageTag,
    /// Index of the first character.
    pub start: usize,
    /// Index one past the last character (a range ending at or before its
    /// start is empty).
    pub end: usize,
}

impl LanguageRange {
    /// `tag` for characters `start..end`.
    pub fn new(tag: impl Into<LanguageTag>, start: usize, end: usize) -> Self {
        Self {
            tag: tag.into(),
            start,
            end,
        }
    }
}

/// The language of each of `len` characters under `ranges`: later ranges
/// override earlier ones, characters past the end of the text are
/// ignored, and characters no range covers keep the default language
/// (`None`).
pub(crate) fn char_languages(ranges: &[LanguageRange], len: usize) -> Vec<Option<&LanguageTag>> {
    let mut languages = vec![None; len];
    for range in ranges {
        for language in languages.iter_mut().take(range.end).skip(range.start) {
            *language = Some(&range.tag);
        }
    }
    languages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_compare_as_harfbuzz_interns_them() {
        assert_eq!(LanguageTag::new("sr_Latn").canonical(), "sr-latn");
        assert_eq!(LanguageTag::new("TR"), LanguageTag::new("tr"));
        assert_eq!(LanguageTag::new("sr@latin").canonical(), "sr");
        assert_eq!(LanguageTag::new("en-IE.UTF-8").canonical(), "en-ie");
        assert_eq!(LanguageTag::new(" sr").canonical(), "");
        assert_eq!(LanguageTag::new("srb\u{e9}").canonical(), "srb");
        assert_eq!(LanguageTag::new("").canonical(), "");
        assert_ne!(LanguageTag::new("sr"), LanguageTag::new("sr-x"));
        let tag = LanguageTag::from("Tr_tR".to_owned());
        assert_eq!(
            (tag.as_str(), tag.to_string()),
            ("Tr_tR", "Tr_tR".to_owned())
        );
        assert_eq!(format!("{tag:?}"), "LanguageTag(\"Tr_tR\")");
        let hash = |tag: &LanguageTag| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            tag.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&tag), hash(&LanguageTag::new("tr-tr")));
    }

    #[test]
    fn later_ranges_win_and_ranges_are_clipped() {
        let ranges = [
            LanguageRange::new("sr", 0, 4),
            LanguageRange::new("ru", 2, 9),
            LanguageRange::new("tr", 3, 1),
        ];
        let languages: Vec<Option<&str>> = char_languages(&ranges, 6)
            .into_iter()
            .map(|tag| tag.map(LanguageTag::as_str))
            .collect();
        assert_eq!(
            languages,
            [
                Some("sr"),
                Some("sr"),
                Some("ru"),
                Some("ru"),
                Some("ru"),
                Some("ru")
            ]
        );
        assert_eq!(
            char_languages(&ranges, 0),
            Vec::<Option<&LanguageTag>>::new()
        );
        assert_eq!(char_languages(&[], 2), [None, None]);
    }
}
