//! matplotlib's `FontManager`: the font lists and `findfont`.

use std::sync::Arc;

use pltrs_text::{
    AfmFont, FontEntry, FontId, FontSource, FontStyle, FontVariant, TextEngine, Type1Face,
};

use super::{FontProperties, FontStretch, FontWeight};
use crate::rc::RcParams;
use crate::text::TextError;

/// The kind of font file to search (matplotlib's `fontext`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontExt {
    /// TrueType/OpenType fonts (`ttflist`).
    #[default]
    Ttf,
    /// Adobe font metrics of the PostScript/PDF core fonts (`afmlist`).
    Afm,
}

/// One font of a font list (matplotlib's `FontEntry`).
#[derive(Clone, Debug, PartialEq)]
pub struct FontListEntry {
    /// File name, without directories.
    pub file: String,
    /// Index of the face within the file.
    pub index: u32,
    /// Family name.
    pub name: String,
    /// Slant.
    pub style: FontStyle,
    /// Variant.
    pub variant: FontVariant,
    /// Weight (numeric for TrueType fonts, a keyword for AFM fonts).
    pub weight: FontWeight,
    /// Stretch.
    pub stretch: FontStretch,
}

/// Finds font files for font properties (matplotlib's `FontManager`).
#[derive(Clone, Copy, Debug)]
pub struct FontManager {
    engine: &'static TextEngine,
}

/// The stretch keyword matplotlib records for a catalogued stretch value.
fn stretch_named(value: u16) -> FontStretch {
    let name = match value {
        300 => "condensed",
        400 => "semi-condensed",
        700 => "expanded",
        _ => "normal",
    };
    FontStretch::from_name(name).unwrap_or(FontStretch::NORMAL)
}

impl FontManager {
    /// The fonts text is drawn with: matplotlib's bundled fonts plus the
    /// fonts installed on the system.
    pub fn global() -> Self {
        Self {
            engine: TextEngine::global(),
        }
    }

    /// Only matplotlib's bundled fonts, independent of the host.
    pub fn bundled() -> Self {
        Self {
            engine: TextEngine::bundled(),
        }
    }

    /// The file name of a catalogued face.
    fn file_name(self, id: FontId) -> String {
        self.engine
            .database()
            .source(id)
            .and_then(FontSource::file_name)
            .unwrap_or_default()
            .to_owned()
    }

    /// The font list for `ext`, in matching order (`ttflist`/`afmlist`).
    pub fn font_list(&self, ext: FontExt) -> Vec<FontListEntry> {
        match ext {
            FontExt::Ttf => {
                let db = self.engine.database();
                db.entries()
                    .iter()
                    .map(|entry: &FontEntry| FontListEntry {
                        file: self.file_name(entry.font),
                        index: db.face_index(entry.font).unwrap_or_default(),
                        name: entry.name.clone(),
                        style: entry.style,
                        variant: entry.variant,
                        weight: FontWeight::numeric(entry.weight),
                        stretch: stretch_named(entry.stretch.0),
                    })
                    .collect()
            }
            FontExt::Afm => AfmFont::all()
                .iter()
                .map(|font| FontListEntry {
                    file: font.file.to_owned(),
                    index: 0,
                    name: font.name.to_owned(),
                    style: font.style,
                    variant: FontVariant::Normal,
                    weight: FontWeight::from_name(font.weight)
                        .unwrap_or(FontWeight::numeric(font.weight_value().0)),
                    stretch: stretch_named(font.stretch.0),
                })
                .collect(),
        }
    }

    /// The family names of the TrueType font list, sorted and without
    /// duplicates (matplotlib's `get_font_names`, which leaves them
    /// unordered).
    pub fn font_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .engine
            .database()
            .entries()
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// matplotlib's `FontManager.score_weight`: 0 when both weights were
    /// given as the same keyword, else `0.95 · |w₁ − w₂| / 1000 + 0.05` of
    /// their numeric values (so 0.05 for equal numbers).
    ///
    /// ```
    /// use pltrs::fonts::{FontManager, FontWeight};
    /// let regular = FontWeight::from_name("regular").unwrap();
    /// assert_eq!(FontManager::score_weight(regular, regular), 0.0);
    /// assert_eq!(FontManager::score_weight(FontWeight::NORMAL, regular), 0.05);
    /// ```
    pub fn score_weight(wanted: FontWeight, actual: FontWeight) -> f64 {
        match (wanted.name(), actual.name()) {
            (Some(a), Some(b)) if a == b => 0.0,
            _ => pltrs_text::score_weight(wanted.value(), actual.value()),
        }
    }

    /// Adds the fonts of the file at `path` (matplotlib's
    /// `fontManager.addfont`): they are matched by their family names
    /// from now on.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or holds no font.
    pub fn add_font(self, path: impl AsRef<std::path::Path>) -> Result<(), TextError> {
        self.engine.add_font_file(path).map(drop)
    }

    /// The Type 1 font (`.pfa` or `.pfb`) at `path`, as matplotlib's
    /// `get_font` opens the TeX fonts of `text.usetex` through `FreeType`:
    /// read once and shared.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or is not a Type 1 font.
    pub fn type1_face(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Arc<Type1Face>, TextError> {
        Ok(self.engine.database().type1_face(path)?)
    }

    /// matplotlib's `findfont`: the file `props` names, else the file name
    /// of the best match for `props` (generic families resolved through
    /// `rc`). Without a family match, the default family (`DejaVu Sans`,
    /// or `Helvetica` for AFM) is used if `fallback_to_default` is set,
    /// else it is an error.
    pub fn find_font(
        &self,
        props: &FontProperties,
        rc: &RcParams,
        ext: FontExt,
        fallback_to_default: bool,
    ) -> Result<String, TextError> {
        if let Some(file) = &props.file {
            return Ok(file.display().to_string());
        }
        let request = props.request(rc);
        let (query, generics) = (&request.query, &request.generics);
        match ext {
            FontExt::Ttf => {
                let db = self.engine.database();
                let id = if fallback_to_default {
                    db.find_font_with(query, generics)?
                } else {
                    db.find_font_exact_with(query, generics)?
                };
                Ok(self.file_name(id))
            }
            FontExt::Afm => Ok(AfmFont::find(query, generics, fallback_to_default)?
                .file
                .to_owned()),
        }
    }

    /// matplotlib's `_find_fonts_by_props`: the best TrueType file for
    /// every family of `props` that matches, in family order; if none does,
    /// the default family's best file (or an error without
    /// `fallback_to_default`).
    pub fn find_fonts(
        &self,
        props: &FontProperties,
        rc: &RcParams,
        fallback_to_default: bool,
    ) -> Result<Vec<String>, TextError> {
        let request = props.request(rc);
        let db = self.engine.database();
        let ids = if fallback_to_default {
            db.find_fonts_with(&request.query, &request.generics)?
        } else {
            db.find_fonts_exact_with(&request.query, &request.generics)?
        };
        Ok(ids.into_iter().map(|id| self.file_name(id)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(families: &[&str]) -> FontProperties {
        FontProperties {
            families: families.iter().map(|f| (*f).to_owned()).collect(),
            ..FontProperties::from_rc(&RcParams::matplotlib())
        }
    }

    #[test]
    fn bundled_lists() {
        let manager = FontManager::bundled();
        let ttf = manager.font_list(FontExt::Ttf);
        let (inter, matplotlib): (Vec<_>, Vec<_>) = ttf
            .iter()
            .partition(|entry| entry.file.starts_with("Inter-"));
        assert_eq!(matplotlib.len(), 40);
        assert_eq!(ttf[0].file, "DejaVuSans-Bold.ttf");
        let mut names: Vec<&str> = inter.iter().map(|entry| entry.name.as_str()).collect();
        names.dedup();
        // Five faces; SemiBold is also listed under its legacy family name.
        assert_eq!((inter.len(), names), (6, vec!["Inter", "Inter SemiBold"]));
        assert_eq!(manager.font_list(FontExt::Afm).len(), 61);
    }

    #[test]
    fn type1_fonts_by_path() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../pltrs-font/tests/fixtures/type1/cmr10.pfb");
        let face = FontManager::bundled().type1_face(&path).unwrap();
        assert_eq!(face.family_name(), Some("Computer Modern"));
        assert!(
            FontManager::bundled()
                .type1_face("/nonexistent.pfb")
                .is_err()
        );
    }

    #[test]
    fn find_font_with_and_without_fallback() {
        let manager = FontManager::bundled();
        let rc = RcParams::matplotlib();
        let find = |families: &[&str], ext, fallback| {
            manager.find_font(&props(families), &rc, ext, fallback)
        };
        assert_eq!(
            find(&["serif"], FontExt::Ttf, true).as_deref(),
            Ok("DejaVuSerif.ttf")
        );
        assert_eq!(
            find(&["Foo"], FontExt::Ttf, true).as_deref(),
            Ok("DejaVuSans.ttf")
        );
        assert!(find(&["Foo"], FontExt::Ttf, false).is_err());
        assert_eq!(
            find(&["Foo"], FontExt::Afm, true).as_deref(),
            Ok("Helvetica.afm")
        );
        assert_eq!(
            manager.find_fonts(&props(&["cmr10", "Foo", "DejaVu Sans"]), &rc, false),
            Ok(vec!["cmr10.ttf".to_owned(), "DejaVuSans.ttf".to_owned()])
        );
        assert!(manager.find_fonts(&props(&["Foo"]), &rc, false).is_err());
    }
}
