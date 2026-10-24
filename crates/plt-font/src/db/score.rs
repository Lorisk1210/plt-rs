//! matplotlib's `FontManager` scoring functions. Each returns 0.0 for a
//! perfect match and up to 1.0 for a bad one.

use crate::db::entry::FontEntry;
use crate::db::query::{
    FontQuery, FontStyle, FontVariant, GenericFamilies, GenericFamily, Stretch,
};

/// `score_family`: position of the entry's family in the query list, with
/// generic families expanded through their preference lists.
pub(crate) fn family(families: &[String], name: &str, generics: &GenericFamilies) -> f64 {
    if families.is_empty() {
        return 1.0;
    }
    let name = name.to_lowercase();
    let step = 1.0 / families.len() as f64;
    for (i, family) in families.iter().enumerate() {
        let family = family.to_lowercase();
        if let Ok(generic) = family.parse::<GenericFamily>() {
            let options = generics.get(generic);
            if let Some(index) = options
                .iter()
                .position(|option| option.to_lowercase() == name)
            {
                return (i as f64 + index as f64 / options.len() as f64) * step;
            }
        } else if family == name {
            return i as f64 * step;
        }
    }
    1.0
}

/// `score_style`: italic and oblique are close substitutes.
pub(crate) fn style(wanted: FontStyle, actual: FontStyle) -> f64 {
    if wanted == actual {
        0.0
    } else if wanted != FontStyle::Normal && actual != FontStyle::Normal {
        0.1
    } else {
        1.0
    }
}

/// `score_variant`.
pub(crate) fn variant(wanted: FontVariant, actual: FontVariant) -> f64 {
    if wanted == actual { 0.0 } else { 1.0 }
}

/// `score_stretch`.
pub(crate) fn stretch(wanted: Stretch, actual: Stretch) -> f64 {
    f64::from(wanted.0.abs_diff(actual.0)) / 1000.0
}

/// matplotlib's `score_weight` of two numeric weights: 0.05 for equal
/// weights, growing to 1.0 as they differ (the exact-name shortcut, a
/// score of 0, never applies to TrueType entries, whose weights are
/// numeric).
///
/// ```
/// assert_eq!(plt_font::score_weight(400, 400), 0.05);
/// assert!((plt_font::score_weight(400, 700) - 0.335).abs() < 1e-12);
/// ```
pub fn weight(wanted: u16, actual: u16) -> f64 {
    0.95 * (f64::from(wanted.abs_diff(actual)) / 1000.0) + 0.05
}

/// The total `findfont` score: family dominates (×10), then style,
/// variant, weight and stretch. Scalable fonts always match the size.
pub(crate) fn total(query: &FontQuery, entry: &FontEntry, generics: &GenericFamilies) -> f64 {
    family(&query.families, &entry.name, generics) * 10.0
        + style(query.style, entry.style)
        + variant(query.variant, entry.variant)
        + weight(query.weight.0, entry.weight)
        + stretch(query.stretch, entry.stretch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn families(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn family_scores() {
        let generics = GenericFamilies::default();
        assert_eq!(
            family(&families(&["DejaVu Sans"]), "dejavu sans", &generics),
            0.0
        );
        assert_eq!(
            family(&families(&["Foo", "DejaVu Sans"]), "DejaVu Sans", &generics),
            0.5
        );
        assert_eq!(
            family(&families(&["serif"]), "DejaVu Serif", &generics),
            0.0
        );
        let monospace_third = 2.0 / 10.0;
        assert!(
            (family(
                &families(&["monospace"]),
                "Computer Modern Typewriter",
                &generics
            ) - monospace_third)
                .abs()
                < 1e-12
        );
        assert_eq!(
            family(&families(&["sans"]), "Verdana", &generics),
            4.0 / 11.0
        );
        assert_eq!(family(&families(&["Foo"]), "DejaVu Sans", &generics), 1.0);
        assert_eq!(family(&[], "DejaVu Sans", &generics), 1.0);
    }

    #[test]
    fn attribute_scores() {
        assert_eq!(style(FontStyle::Italic, FontStyle::Oblique), 0.1);
        assert_eq!(style(FontStyle::Normal, FontStyle::Oblique), 1.0);
        assert_eq!(style(FontStyle::Oblique, FontStyle::Oblique), 0.0);
        assert_eq!(variant(FontVariant::SmallCaps, FontVariant::Normal), 1.0);
        assert_eq!(stretch(Stretch(300), Stretch(500)), 0.2);
        assert!((weight(400, 700) - 0.335).abs() < 1e-12);
        assert_eq!(weight(400, 400), 0.05);
    }
}
