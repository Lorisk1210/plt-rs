//! fontconfig patterns turned into font queries the way
//! `FontProperties(pattern)` does for the custom fontset's
//! `mathtext.rm`, `mathtext.it`, ... values.

use plt_font::{FontQuery, FontVariant};

use crate::error::TextError;
use crate::fontconfig::FontconfigPattern;
use crate::mathtext::style::PatternDefaults;

/// Parses a fontconfig pattern into a font query (only the first family
/// and the first value of each property count, as in matplotlib); what the
/// pattern leaves out comes from `defaults`.
pub(crate) fn parse_pattern(
    pattern: &str,
    defaults: PatternDefaults,
) -> Result<FontQuery, TextError> {
    let parsed = FontconfigPattern::parse(pattern)?;
    let mut query = FontQuery {
        style: defaults.style,
        variant: defaults.variant,
        weight: defaults.weight,
        stretch: defaults.stretch,
        ..FontQuery::default()
    };
    for (key, values) in parsed.properties() {
        let value = values.first().map_or("", String::as_str);
        let invalid = || TextError::Font(format!("invalid font {key} {value:?} in {pattern:?}"));
        match key.as_str() {
            "family" => query.families = vec![value.to_owned()],
            "size" => {
                value.parse::<f64>().map_err(|_| invalid())?;
            }
            "weight" => query.weight = value.parse().map_err(|_| invalid())?,
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
    Ok(query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_font::{FontStyle, Stretch, Weight};

    #[test]
    fn families_and_constants() {
        let query = parse_pattern("sans:italic:bold", PatternDefaults::default()).expect("valid");
        assert_eq!(query.families, vec!["sans"]);
        assert_eq!(query.style, FontStyle::Italic);
        assert_eq!(query.weight, Weight::BOLD);
        let query =
            parse_pattern("DejaVu Serif:weight=bold", PatternDefaults::default()).expect("valid");
        assert_eq!(query.families, vec!["DejaVu Serif"]);
        assert_eq!(query.weight, Weight::BOLD);
    }

    #[test]
    fn empty_family_and_first_values_win() {
        let query = parse_pattern(":bold", PatternDefaults::default()).expect("valid");
        assert_eq!(query.families, vec![""]);
        let query = parse_pattern(
            "a,b-12:light:bold:width=condensed",
            PatternDefaults::default(),
        )
        .expect("valid");
        assert_eq!(query.families, vec!["a"]);
        assert_eq!(query.weight, Weight(200));
        assert_eq!(query.stretch, Stretch::CONDENSED);
        let query =
            parse_pattern(r"sans\-serif:slant=oblique", PatternDefaults::default()).expect("valid");
        assert_eq!(query.families, vec!["sans-serif"]);
        assert_eq!(query.style, FontStyle::Oblique);
    }

    #[test]
    fn what_a_pattern_leaves_out_comes_from_the_defaults() {
        let defaults = PatternDefaults {
            style: FontStyle::Italic,
            weight: Weight::BOLD,
            ..PatternDefaults::default()
        };
        let query = parse_pattern("DejaVu Sans", defaults).expect("valid");
        assert_eq!(query.families, vec!["DejaVu Sans"]);
        assert_eq!(
            (query.style, query.weight),
            (FontStyle::Italic, Weight::BOLD)
        );
        let query = parse_pattern("DejaVu Sans:roman", defaults).expect("valid");
        assert_eq!(
            (query.style, query.weight),
            (FontStyle::Normal, Weight::BOLD)
        );
    }

    #[test]
    fn invalid_patterns() {
        assert!(parse_pattern("sans-serif", PatternDefaults::default()).is_err());
        assert!(parse_pattern("sans:weight=fat", PatternDefaults::default()).is_err());
        assert!(parse_pattern("sans:colour=red", PatternDefaults::default()).is_err());
        assert!(parse_pattern("sans:variant=tiny", PatternDefaults::default()).is_err());
    }
}
