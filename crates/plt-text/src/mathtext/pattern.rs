//! fontconfig patterns turned into font queries the way
//! `FontProperties(pattern)` does for the custom fontset's
//! `mathtext.rm`, `mathtext.it`, ... values.

use plt_font::{FontQuery, FontStyle, FontVariant, Stretch, Weight};

use crate::error::TextError;
use crate::fontconfig::FontconfigPattern;

/// Parses a fontconfig pattern into a font query (only the first family
/// and the first value of each property count, as in matplotlib).
pub(crate) fn parse_pattern(pattern: &str) -> Result<FontQuery, TextError> {
    let parsed = FontconfigPattern::parse(pattern)?;
    let mut query = FontQuery::default();
    for (key, values) in parsed.properties() {
        let value = values.first().map_or("", String::as_str);
        let invalid = || TextError::Font(format!("invalid font {key} {value:?} in {pattern:?}"));
        match key.as_str() {
            "family" => query.families = vec![value.to_owned()],
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
