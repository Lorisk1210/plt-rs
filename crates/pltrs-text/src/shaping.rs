//! What matplotlib's backends learn from a text artist about shaping its
//! line: the OpenType features and the language (`Text.get_fontfeatures`
//! and `Text.get_language`), which they hand to libraqm when they lay the
//! line out for drawing.

use pltrs_font::{Feature, LanguageRange, LanguageTag, ShapeOptions};

use crate::error::TextError;

/// The language of a text (matplotlib's `Text.set_language` argument).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Language {
    /// One BCP 47 language tag for the whole text (`"sr"`, `"tr-TR"`).
    Tag(LanguageTag),
    /// Languages of character ranges (matplotlib's list of
    /// `(language, start, end)`), later ranges winning where they overlap.
    Ranges(Vec<LanguageRange>),
}

pltrs_core::archive_enum!(Language {
    Tag(v0),
    Ranges(v0),
});

impl Language {
    /// The ranges libraqm is given for `text`: matplotlib's `ft2font`
    /// wrapper gives a tag the whole text, and ranges go as they are.
    ///
    /// # Errors
    ///
    /// [`TextError::LanguageRange`] for the first range of a non-empty
    /// text that starts at or after its end or ends after it, which
    /// libraqm refuses (`raqm_set_language`); ranges ending before they
    /// start are accepted and cover nothing.
    pub fn ranges(&self, text: &str) -> Result<Vec<LanguageRange>, TextError> {
        let len = text.chars().count();
        match self {
            Self::Tag(tag) => Ok(vec![LanguageRange::new(tag.clone(), 0, len)]),
            Self::Ranges(ranges) => match ranges
                .iter()
                .find(|range| len > 0 && (range.start >= len || range.end > len))
            {
                Some(range) => Err(TextError::LanguageRange {
                    language: range.tag.to_string(),
                    start: range.start,
                    end: range.end,
                }),
                None => Ok(ranges.clone()),
            },
        }
    }
}

impl From<LanguageTag> for Language {
    fn from(tag: LanguageTag) -> Self {
        Self::Tag(tag)
    }
}

impl From<&str> for Language {
    fn from(tag: &str) -> Self {
        Self::Tag(tag.into())
    }
}

impl From<String> for Language {
    fn from(tag: String) -> Self {
        Self::Tag(tag.into())
    }
}

impl From<Vec<LanguageRange>> for Language {
    fn from(ranges: Vec<LanguageRange>) -> Self {
        Self::Ranges(ranges)
    }
}

impl FromIterator<LanguageRange> for Language {
    fn from_iter<I: IntoIterator<Item = LanguageRange>>(ranges: I) -> Self {
        Self::Ranges(ranges.into_iter().collect())
    }
}

/// How a line is shaped beyond its font: the settings matplotlib's
/// backends receive with a single-line text artist (multi-line texts,
/// mathtext and text measured for layout get the defaults).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shaping {
    /// OpenType feature settings on top of `HarfBuzz`'s defaults, in order.
    pub features: Vec<Feature>,
    /// The language; `None` leaves every character in `HarfBuzz`'s default
    /// language, for which pltrs picks no language system.
    pub language: Option<Language>,
}

impl Shaping {
    /// Whether these are the defaults, which shape like no settings at all.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The shaping options for laying out `text` (the whole text the
    /// artist was given, of which only the first paragraph is laid out).
    ///
    /// # Errors
    ///
    /// As [`Language::ranges`].
    pub(crate) fn shape_options(&self, text: &str) -> Result<ShapeOptions, TextError> {
        let languages = match &self.language {
            Some(language) => language.ranges(text)?,
            None => Vec::new(),
        };
        Ok(ShapeOptions {
            features: self.features.clone(),
            languages,
            ..ShapeOptions::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(language: &Language, text: &str) -> Result<Vec<(String, usize, usize)>, TextError> {
        Ok(language
            .ranges(text)?
            .into_iter()
            .map(|range| (range.tag.to_string(), range.start, range.end))
            .collect())
    }

    #[test]
    fn tags_cover_the_whole_text() {
        let serbian = Language::from("sr");
        assert_eq!(spans(&serbian, "бгд"), Ok(vec![("sr".to_owned(), 0, 3)]));
        assert_eq!(spans(&serbian, ""), Ok(vec![("sr".to_owned(), 0, 0)]));
        assert_eq!(Language::from("sr".to_owned()), serbian);
        assert_eq!(Language::from(LanguageTag::new("SR")), serbian);
    }

    #[test]
    fn ranges_inside_the_text_are_kept_in_order() {
        let overlapping: Language = [
            LanguageRange::new("sr", 0, 3),
            LanguageRange::new("ru", 1, 2),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            spans(&overlapping, "абв"),
            Ok(vec![("sr".to_owned(), 0, 3), ("ru".to_owned(), 1, 2)])
        );
        let reversed = Language::from(vec![LanguageRange::new("sr", 2, 1)]);
        assert_eq!(spans(&reversed, "абв"), Ok(vec![("sr".to_owned(), 2, 1)]));
        assert!(spans(&Language::Ranges(Vec::new()), "абв").is_ok_and(|r| r.is_empty()));
    }

    #[test]
    fn ranges_outside_a_text_fail_like_libraqm() {
        let error = |start, end| TextError::LanguageRange {
            language: "sr".to_owned(),
            start,
            end,
        };
        let range = |start, end| Language::from(vec![LanguageRange::new("sr", start, end)]);
        assert_eq!(spans(&range(0, 4), "абв"), Err(error(0, 4)));
        assert_eq!(spans(&range(3, 3), "абв"), Err(error(3, 3)));
        assert_eq!(spans(&range(5, 2), "абв"), Err(error(5, 2)));
        assert!(
            spans(&range(0, 9), "").is_ok(),
            "empty texts accept anything"
        );
        let first_bad = Language::from(vec![
            LanguageRange::new("ok", 0, 1),
            LanguageRange::new("sr", 0, 4),
            LanguageRange::new("tr", 7, 8),
        ]);
        assert_eq!(spans(&first_bad, "абв"), Err(error(0, 4)));
        assert!(
            error(0, 4)
                .to_string()
                .contains("between 0 and 4 characters")
        );
    }

    #[test]
    fn shape_options_carry_features_and_ranges() {
        let shaping = Shaping {
            features: vec!["-kern".parse().expect("valid")],
            language: Some("tr".into()),
        };
        assert!(!shaping.is_default() && Shaping::default().is_default());
        let options = shaping.shape_options("fi").expect("valid");
        assert_eq!(options.features, shaping.features);
        assert_eq!(options.languages, [LanguageRange::new("tr", 0, 2)]);
        assert!(
            Shaping::default()
                .shape_options("fi")
                .is_ok_and(|o| o.languages.is_empty())
        );
    }
}
