//! matplotlib's AFM font list (`fontManager.afmlist`): the Type 1 fonts
//! whose metrics matplotlib ships in `mpl-data/fonts/{afm,pdfcorefonts}`
//! (the PostScript and PDF core fonts), and `findfont(..., fontext="afm")`.

use crate::db::query::{FontQuery, FontStyle, FontVariant, GenericFamilies, Stretch, Weight};
use crate::db::score;
use crate::error::{FontError, Result};

/// One AFM font, described as matplotlib's `afmFontProperty` reads it from
/// the metrics file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AfmFont {
    /// File name of the metrics file.
    pub file: &'static str,
    /// Family name (`FamilyName`).
    pub name: &'static str,
    /// Italic when the `ItalicAngle` is non-zero.
    pub style: FontStyle,
    /// The `Weight` keyword in lower case (`medium`, `bold`, `roman`, ...).
    pub weight: &'static str,
    /// Condensed for narrow faces.
    pub stretch: Stretch,
}

/// The family `findfont` falls back to for AFM fonts.
const DEFAULT_FAMILY: &str = "Helvetica";

const fn afm(
    file: &'static str,
    name: &'static str,
    style: FontStyle,
    weight: &'static str,
    stretch: Stretch,
) -> AfmFont {
    AfmFont {
        file,
        name,
        style,
        weight,
        stretch,
    }
}

/// Every bundled AFM font in matplotlib's order (file names in Python
/// string order).
const AFM_FONTS: [AfmFont; 61] = [
    afm(
        "Courier-Bold.afm",
        "Courier",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "Courier-BoldOblique.afm",
        "Courier",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "Courier-Oblique.afm",
        "Courier",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "Courier.afm",
        "Courier",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "Helvetica-Bold.afm",
        "Helvetica",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "Helvetica-BoldOblique.afm",
        "Helvetica",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "Helvetica-Oblique.afm",
        "Helvetica",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "Helvetica.afm",
        "Helvetica",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "Symbol.afm",
        "Symbol",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "Times-Bold.afm",
        "Times",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "Times-BoldItalic.afm",
        "Times",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "Times-Italic.afm",
        "Times",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "Times-Roman.afm",
        "Times",
        FontStyle::Normal,
        "roman",
        Stretch::NORMAL,
    ),
    afm(
        "ZapfDingbats.afm",
        "ZapfDingbats",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "cmex10.afm",
        "Computer Modern",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "cmmi10.afm",
        "Computer Modern",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "cmr10.afm",
        "Computer Modern",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "cmsy10.afm",
        "Computer Modern",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "cmti10.afm",
        "cmti10",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "cmtt10.afm",
        "Computer Modern",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "pagd8a.afm",
        "ITC Avant Garde Gothic",
        FontStyle::Normal,
        "demi",
        Stretch::NORMAL,
    ),
    afm(
        "pagdo8a.afm",
        "ITC Avant Garde Gothic",
        FontStyle::Italic,
        "demi",
        Stretch::NORMAL,
    ),
    afm(
        "pagk8a.afm",
        "ITC Avant Garde Gothic",
        FontStyle::Normal,
        "book",
        Stretch::NORMAL,
    ),
    afm(
        "pagko8a.afm",
        "ITC Avant Garde Gothic",
        FontStyle::Italic,
        "book",
        Stretch::NORMAL,
    ),
    afm(
        "pbkd8a.afm",
        "ITC Bookman",
        FontStyle::Normal,
        "demi",
        Stretch::NORMAL,
    ),
    afm(
        "pbkdi8a.afm",
        "ITC Bookman",
        FontStyle::Italic,
        "demi",
        Stretch::NORMAL,
    ),
    afm(
        "pbkl8a.afm",
        "ITC Bookman",
        FontStyle::Normal,
        "light",
        Stretch::NORMAL,
    ),
    afm(
        "pbkli8a.afm",
        "ITC Bookman",
        FontStyle::Italic,
        "light",
        Stretch::NORMAL,
    ),
    afm(
        "pcrb8a.afm",
        "Courier",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "pcrbo8a.afm",
        "Courier",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "pcrr8a.afm",
        "Courier",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "pcrro8a.afm",
        "Courier",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "phvb8a.afm",
        "Helvetica",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "phvb8an.afm",
        "Helvetica",
        FontStyle::Normal,
        "bold",
        Stretch::CONDENSED,
    ),
    afm(
        "phvbo8a.afm",
        "Helvetica",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "phvbo8an.afm",
        "Helvetica",
        FontStyle::Italic,
        "bold",
        Stretch::CONDENSED,
    ),
    afm(
        "phvl8a.afm",
        "Helvetica",
        FontStyle::Normal,
        "light",
        Stretch::NORMAL,
    ),
    afm(
        "phvlo8a.afm",
        "Helvetica",
        FontStyle::Italic,
        "light",
        Stretch::NORMAL,
    ),
    afm(
        "phvr8a.afm",
        "Helvetica",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "phvr8an.afm",
        "Helvetica",
        FontStyle::Normal,
        "medium",
        Stretch::CONDENSED,
    ),
    afm(
        "phvro8a.afm",
        "Helvetica",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "phvro8an.afm",
        "Helvetica",
        FontStyle::Italic,
        "medium",
        Stretch::CONDENSED,
    ),
    afm(
        "pncb8a.afm",
        "New Century Schoolbook",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "pncbi8a.afm",
        "New Century Schoolbook",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "pncr8a.afm",
        "New Century Schoolbook",
        FontStyle::Normal,
        "roman",
        Stretch::NORMAL,
    ),
    afm(
        "pncri8a.afm",
        "New Century Schoolbook",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "pplb8a.afm",
        "Palatino",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "pplbi8a.afm",
        "Palatino",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "pplr8a.afm",
        "Palatino",
        FontStyle::Normal,
        "roman",
        Stretch::NORMAL,
    ),
    afm(
        "pplri8a.afm",
        "Palatino",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "psyr.afm",
        "Symbol",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "ptmb8a.afm",
        "Times",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "ptmbi8a.afm",
        "Times",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "ptmr8a.afm",
        "Times",
        FontStyle::Normal,
        "roman",
        Stretch::NORMAL,
    ),
    afm(
        "ptmri8a.afm",
        "Times",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "putb8a.afm",
        "Utopia",
        FontStyle::Normal,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "putbi8a.afm",
        "Utopia",
        FontStyle::Italic,
        "bold",
        Stretch::NORMAL,
    ),
    afm(
        "putr8a.afm",
        "Utopia",
        FontStyle::Normal,
        "regular",
        Stretch::NORMAL,
    ),
    afm(
        "putri8a.afm",
        "Utopia",
        FontStyle::Italic,
        "regular",
        Stretch::NORMAL,
    ),
    afm(
        "pzcmi8a.afm",
        "ITC Zapf Chancery",
        FontStyle::Italic,
        "medium",
        Stretch::NORMAL,
    ),
    afm(
        "pzdr.afm",
        "ITC Zapf Dingbats",
        FontStyle::Normal,
        "medium",
        Stretch::NORMAL,
    ),
];

impl AfmFont {
    /// The bundled AFM fonts, in matplotlib's catalogue order.
    pub fn all() -> &'static [Self] {
        &AFM_FONTS
    }

    /// The numeric weight (matplotlib's `weight_dict`; unknown keywords
    /// count as `normal`).
    pub fn weight_value(&self) -> Weight {
        Weight::parse(self.weight).unwrap_or(Weight::NORMAL)
    }

    /// matplotlib's `findfont(prop, fontext="afm")`: the first font with the
    /// lowest score; when no family matches, the best `Helvetica` face if
    /// `fallback_to_default` is set.
    pub fn find(
        query: &FontQuery,
        generics: &GenericFamilies,
        fallback_to_default: bool,
    ) -> Result<&'static Self> {
        best_match(query, generics)
            .or_else(|| {
                fallback_to_default.then(|| {
                    let fallback = FontQuery {
                        families: vec![DEFAULT_FAMILY.to_owned()],
                        ..query.clone()
                    };
                    best_match(&fallback, generics)
                })?
            })
            .ok_or_else(|| FontError::NoMatchingFont(query.to_string()))
    }

    /// The `findfont` score of this font for `query` (0 is a perfect match).
    fn score(&self, query: &FontQuery, generics: &GenericFamilies) -> f64 {
        score::family(&query.families, self.name, generics) * 10.0
            + score::style(query.style, self.style)
            + score::variant(query.variant, FontVariant::Normal)
            + score::weight(query.weight.0, self.weight_value().0)
            + score::stretch(query.stretch, self.stretch)
    }
}

/// The best-scoring font, if its score is below 10 (a family matched).
fn best_match(query: &FontQuery, generics: &GenericFamilies) -> Option<&'static AfmFont> {
    let mut best: Option<(f64, &'static AfmFont)> = None;
    for font in &AFM_FONTS {
        let score = font.score(query, generics);
        if best.is_none_or(|(best_score, _)| score < best_score) {
            best = Some((score, font));
        }
        if score == 0.0 {
            break;
        }
    }
    best.filter(|&(score, _)| score < 10.0)
        .map(|(_, font)| font)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(query: &FontQuery) -> &'static str {
        AfmFont::find(query, &GenericFamilies::default(), true)
            .expect("Helvetica always matches")
            .file
    }

    #[test]
    fn families_styles_and_weights() {
        assert_eq!(find(&FontQuery::families(["Times"])), "Times-Roman.afm");
        assert_eq!(
            find(&FontQuery::families(["Helvetica"]).with_weight(Weight::BOLD)),
            "Helvetica-Bold.afm"
        );
        assert_eq!(
            find(&FontQuery::families(["Palatino"]).with_style(FontStyle::Italic)),
            "pplri8a.afm"
        );
        assert_eq!(
            find(&FontQuery::families(["Helvetica"]).with_stretch(Stretch::CONDENSED)),
            "phvr8an.afm"
        );
    }

    #[test]
    fn generic_and_unknown_families() {
        assert_eq!(find(&FontQuery::families(["serif"])), "pncr8a.afm");
        assert_eq!(find(&FontQuery::families(["monospace"])), "Courier.afm");
        assert_eq!(find(&FontQuery::families(["Foo"])), "Helvetica.afm");
        assert!(
            AfmFont::find(
                &FontQuery::families(["Foo"]),
                &GenericFamilies::default(),
                false
            )
            .is_err()
        );
    }
}
