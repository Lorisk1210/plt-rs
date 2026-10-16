//! matplotlib's `FontManager`: the font lists and `findfont`.

use plt_text::{AfmFont, FontEntry, FontId, FontSource, FontStyle, FontVariant, TextEngine};

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

    /// matplotlib's `findfont`: the file name of the best match for
    /// `props` (generic families resolved through `rc`). Without a family
    /// match, the default family (`DejaVu Sans`, or `Helvetica` for AFM) is
    /// used if `fallback_to_default` is set, else it is an error.
    pub fn find_font(
        &self,
        props: &FontProperties,
        rc: &RcParams,
        ext: FontExt,
        fallback_to_default: bool,
    ) -> Result<String, TextError> {
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
        assert_eq!(ttf.len(), 40);
        assert_eq!(ttf[0].file, "DejaVuSans-Bold.ttf");
        assert_eq!(manager.font_list(FontExt::Afm).len(), 61);
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
