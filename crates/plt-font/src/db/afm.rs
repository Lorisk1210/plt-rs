//! matplotlib's AFM font list (`fontManager.afmlist`): the Type 1 fonts
//! whose metrics matplotlib ships in `mpl-data/fonts/{afm,pdfcorefonts}`
//! (the PostScript and PDF core fonts), and `findfont(..., fontext="afm")`
//! over all of them or over one directory.

use super::afm_catalogue::AFM_FONTS;
use crate::afm::{AfmError, AfmMetrics, bundled_metrics};
use crate::db::query::{FontQuery, FontStyle, FontVariant, GenericFamilies, Stretch, Weight};
use crate::db::score;
use crate::error::{FontError, Result};

/// The directory of matplotlib's data an AFM file lives in, which its PDF
/// and PostScript backends restrict their font lookups to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AfmDirectory {
    /// `fonts/pdfcorefonts`: the 14 PDF core fonts (Courier, Helvetica,
    /// Times, Symbol and `ZapfDingbats`), whose metrics plt-rs bundles.
    PdfCore,
    /// `fonts/afm`: the 35 standard PostScript fonts, Helvetica Light,
    /// Utopia and Computer Modern. Their files are catalogued but not
    /// bundled: their notices do not permit redistribution.
    PostScript,
}

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
    /// Where matplotlib keeps the file.
    pub directory: AfmDirectory,
}

/// The family `findfont` falls back to for AFM fonts.
const DEFAULT_FAMILY: &str = "Helvetica";

impl AfmFont {
    /// The catalogued AFM fonts, in matplotlib's catalogue order.
    pub fn all() -> &'static [Self] {
        &AFM_FONTS
    }

    /// The numeric weight (matplotlib's `weight_dict`; unknown keywords
    /// count as `normal`).
    pub fn weight_value(&self) -> Weight {
        self.weight.parse().unwrap_or(Weight::NORMAL)
    }

    /// The font's metrics.
    ///
    /// # Errors
    ///
    /// When the metrics file is not bundled (the fonts of
    /// [`AfmDirectory::PostScript`]).
    pub fn metrics(&self) -> Result<&'static AfmMetrics, AfmError> {
        bundled_metrics(self.file)
    }

    /// matplotlib's `findfont(prop, fontext="afm")`: the first font with the
    /// lowest score; when no family matches, the best `Helvetica` face if
    /// `fallback_to_default` is set.
    pub fn find(
        query: &FontQuery,
        generics: &GenericFamilies,
        fallback_to_default: bool,
    ) -> Result<&'static Self> {
        find_font(query, generics, fallback_to_default, None)
    }

    /// `findfont(prop, fontext="afm", directory=...)`, as the PDF and
    /// PostScript backends pick the font of a text: the best font of
    /// `directory`, falling back to its best `Helvetica` face.
    pub fn find_in(
        query: &FontQuery,
        generics: &GenericFamilies,
        directory: AfmDirectory,
    ) -> Result<&'static Self> {
        find_font(query, generics, true, Some(directory))
    }

    /// `_find_fonts_by_props(prop, fontext="afm", directory=...)`: the best
    /// font of `directory` for each family of the query that matches one,
    /// in family order, or its best `Helvetica` face when none does.
    pub fn find_all_in(
        query: &FontQuery,
        generics: &GenericFamilies,
        directory: AfmDirectory,
    ) -> Result<Vec<&'static Self>> {
        let found: Vec<&'static Self> = query
            .families
            .iter()
            .filter_map(|family| {
                let single = FontQuery {
                    families: vec![family.clone()],
                    ..query.clone()
                };
                best_match(&single, generics, Some(directory))
            })
            .collect();
        if found.is_empty() {
            Self::find_in(&with_default_family(query), generics, directory).map(|font| vec![font])
        } else {
            Ok(found)
        }
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

/// `query` asking for the default family instead of its own.
fn with_default_family(query: &FontQuery) -> FontQuery {
    FontQuery {
        families: vec![DEFAULT_FAMILY.to_owned()],
        ..query.clone()
    }
}

/// `findfont` over the fonts of `directory` (all fonts for `None`).
fn find_font(
    query: &FontQuery,
    generics: &GenericFamilies,
    fallback_to_default: bool,
    directory: Option<AfmDirectory>,
) -> Result<&'static AfmFont> {
    best_match(query, generics, directory)
        .or_else(|| {
            fallback_to_default
                .then(|| best_match(&with_default_family(query), generics, directory))?
        })
        .ok_or_else(|| FontError::NoMatchingFont(query.to_string()))
}

/// The best-scoring font of `directory` (all fonts for `None`), if its
/// score is below 10 (a family matched).
fn best_match(
    query: &FontQuery,
    generics: &GenericFamilies,
    directory: Option<AfmDirectory>,
) -> Option<&'static AfmFont> {
    let mut best: Option<(f64, &'static AfmFont)> = None;
    let candidates = AFM_FONTS
        .iter()
        .filter(|font| directory.is_none_or(|d| font.directory == d));
    for font in candidates {
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

    fn find_in(query: &FontQuery, directory: AfmDirectory) -> &'static str {
        AfmFont::find_in(query, &GenericFamilies::default(), directory)
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

    #[test]
    fn directories_restrict_the_search() {
        use AfmDirectory::{PdfCore, PostScript};
        // matplotlib: findfont(prop, fontext='afm', directory=...).
        let serif = FontQuery::families(["serif"]);
        assert_eq!(find_in(&serif, PdfCore), "Times-Roman.afm");
        assert_eq!(find_in(&serif, PostScript), "pncr8a.afm");
        let sans = FontQuery::families(["DejaVu Sans"]);
        assert_eq!(find_in(&sans, PdfCore), "Helvetica.afm");
        assert_eq!(find_in(&sans, PostScript), "phvr8a.afm");
        let palatino = FontQuery::families(["Palatino"]).with_weight(Weight::BOLD);
        assert_eq!(find_in(&palatino, PdfCore), "Helvetica-Bold.afm");
        let fonts = |families: &[&str]| -> Vec<&str> {
            AfmFont::find_all_in(
                &FontQuery::families(families.iter().copied()),
                &GenericFamilies::default(),
                PdfCore,
            )
            .expect("Helvetica always matches")
            .iter()
            .map(|font| font.file)
            .collect()
        };
        assert_eq!(
            fonts(&["Foo", "Times", "Courier"]),
            ["Times-Roman.afm", "Courier.afm"]
        );
        assert_eq!(fonts(&["Foo"]), ["Helvetica.afm"]);
    }

    #[test]
    fn only_core_fonts_have_metrics() {
        let core = AfmFont::find_in(
            &FontQuery::families(["Courier"]),
            &GenericFamilies::default(),
            AfmDirectory::PdfCore,
        )
        .expect("a core font");
        assert_eq!(
            core.metrics().and_then(AfmMetrics::font_name),
            Ok("Courier")
        );
        let base35 = find_in(&FontQuery::families(["Courier"]), AfmDirectory::PostScript);
        assert_eq!(base35, "pcrr8a.afm");
        assert!(
            AfmFont::all()
                .iter()
                .all(|font| font.metrics().is_ok() == (font.directory == AfmDirectory::PdfCore))
        );
    }
}
