//! fontconfig patterns turned into font queries the way
//! `FontProperties(pattern)` does for the custom fontset's
//! `mathtext.rm`, `mathtext.it`, ... values.

use pltrs_font::{FontQuery, FontVariant, Weight};

use crate::error::TextError;
use crate::fontconfig::FontconfigPattern;
use crate::mathtext::style::PatternDefaults;

/// A font pattern as `FontProperties(pattern)` takes it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedPattern {
    /// The fonts it asks for.
    pub(crate) query: FontQuery,
    /// The name its weight was given as, if it was one (the font
    /// manager's messages name it).
    pub(crate) weight_keyword: Option<&'static str>,
}

/// Parses a fontconfig pattern into a font query (only the first family
/// and the first value of each property count, as in matplotlib); what the
/// pattern leaves out comes from `defaults`.
pub(crate) fn parse_pattern(
    pattern: &str,
    defaults: PatternDefaults,
) -> Result<ParsedPattern, TextError> {
    let parsed = FontconfigPattern::parse(pattern)?;
    let mut query = FontQuery {
        style: defaults.style,
        variant: defaults.variant,
        weight: defaults.weight,
        stretch: defaults.stretch,
        ..FontQuery::default()
    };
    let mut weight_keyword = defaults.weight_keyword;
    for (key, values) in parsed.properties() {
        let value = values.first().map_or("", String::as_str);
        let invalid = || TextError::Font(format!("invalid font {key} {value:?} in {pattern:?}"));
        match key.as_str() {
            "family" => query.families = vec![value.to_owned()],
            "size" => {
                value.parse::<f64>().map_err(|_| invalid())?;
            }
            "weight" => {
                query.weight = value.parse().map_err(|_| invalid())?;
                weight_keyword = Weight::keyword(value);
            }
            "slant" | "style" => query.style = value.parse().map_err(|_| invalid())?,
            "width" | "stretch" => query.stretch = value.parse().map_err(|_| invalid())?,
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
    Ok(ParsedPattern {
        query,
        weight_keyword,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_font::{FontStyle, Stretch, Weight};

    #[test]
    fn families_and_constants() {
        let parsed = parse_pattern("sans:italic:bold", PatternDefaults::default()).expect("valid");
        assert_eq!(parsed.weight_keyword, Some("bold"));
        let query = parsed.query;
        assert_eq!(query.families, vec!["sans"]);
        assert_eq!(query.style, FontStyle::Italic);
        assert_eq!(query.weight, Weight::BOLD);
        let query = parse_pattern("DejaVu Serif:weight=bold", PatternDefaults::default())
            .expect("valid")
            .query;
        assert_eq!(query.families, vec!["DejaVu Serif"]);
        assert_eq!(query.weight, Weight::BOLD);
    }

    #[test]
    fn empty_family_and_first_values_win() {
        let query = parse_pattern(":bold", PatternDefaults::default())
            .expect("valid")
            .query;
        assert_eq!(query.families, vec![""]);
        let query = parse_pattern(
            "a,b-12:light:bold:width=condensed",
            PatternDefaults::default(),
        )
        .expect("valid")
        .query;
        assert_eq!(query.families, vec!["a"]);
        assert_eq!(query.weight, Weight(200));
        assert_eq!(query.stretch, Stretch::CONDENSED);
        let query = parse_pattern(r"sans\-serif:slant=oblique", PatternDefaults::default())
            .expect("valid")
            .query;
        assert_eq!(query.families, vec!["sans-serif"]);
        assert_eq!(query.style, FontStyle::Oblique);
    }

    #[test]
    fn what_a_pattern_leaves_out_comes_from_the_defaults() {
        let defaults = PatternDefaults {
            style: FontStyle::Italic,
            weight: Weight::BOLD,
            weight_keyword: Some("bold"),
            ..PatternDefaults::default()
        };
        let parsed = parse_pattern("DejaVu Sans", defaults).expect("valid");
        assert_eq!(parsed.query.families, vec!["DejaVu Sans"]);
        assert_eq!(
            (
                parsed.query.style,
                parsed.query.weight,
                parsed.weight_keyword
            ),
            (FontStyle::Italic, Weight::BOLD, Some("bold"))
        );
        let query = parse_pattern("DejaVu Sans:roman", defaults)
            .expect("valid")
            .query;
        assert_eq!(
            (query.style, query.weight),
            (FontStyle::Normal, Weight::BOLD)
        );
    }

    #[test]
    fn a_weight_keeps_the_name_it_was_given_as() {
        let weight = |pattern| {
            let parsed = parse_pattern(pattern, PatternDefaults::default()).expect("valid");
            (parsed.query.weight, parsed.weight_keyword)
        };
        assert_eq!(
            weight("DejaVu Sans:semibold"),
            (Weight(600), Some("semibold"))
        );
        assert_eq!(
            weight("DejaVu Sans:weight=heavy"),
            (Weight(800), Some("heavy"))
        );
        assert_eq!(weight("DejaVu Sans:weight=600"), (Weight(600), None));
    }

    #[test]
    fn invalid_patterns() {
        assert!(parse_pattern("sans-serif", PatternDefaults::default()).is_err());
        assert!(parse_pattern("sans:weight=fat", PatternDefaults::default()).is_err());
        assert!(parse_pattern("sans:colour=red", PatternDefaults::default()).is_err());
        assert!(parse_pattern("sans:variant=tiny", PatternDefaults::default()).is_err());
    }
}
