//! matplotlib's `FontManager`: the font lists and `findfont`.

use std::sync::Arc;

use pltrs_text::{
    AfmDirectory, AfmFont, FontEntry, FontId, FontSource, FontStyle, FontVariant, Stretch,
    TextEngine, Type1Face,
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

    /// The file of a catalogued face, as matplotlib's font lists name it:
    /// its path, or its file name for a bundled font (which has no path).
    fn file_name(self, id: FontId) -> String {
        match self.engine.database().source(id) {
            Some(FontSource::File(path)) => path.display().to_string(),
            Some(FontSource::Bundled(name)) => (*name).to_owned(),
            None => String::new(),
        }
    }

    /// A catalogued face's entry as matplotlib's `ttflist` holds it.
    fn list_entry(self, entry: &FontEntry) -> FontListEntry {
        FontListEntry {
            file: self.file_name(entry.font),
            index: self
                .engine
                .database()
                .face_index(entry.font)
                .unwrap_or_default(),
            name: entry.name.clone(),
            style: entry.style,
            variant: entry.variant,
            weight: FontWeight::numeric(entry.weight),
            stretch: stretch_named(entry.stretch.0),
        }
    }

    /// The font list for `ext`, in matching order (`ttflist`/`afmlist`).
    pub fn font_list(&self, ext: FontExt) -> Vec<FontListEntry> {
        match ext {
            FontExt::Ttf => self
                .engine
                .database()
                .entries()
                .iter()
                .map(|entry| self.list_entry(entry))
                .collect(),
            FontExt::Afm => AfmFont::all().iter().map(afm_entry).collect(),
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

    /// Appends a TrueType font-list entry (matplotlib's `ttflist.append`):
    /// the face `entry.index` of the file `entry.file` (a path, or the
    /// file name of a bundled font) is matched by the entry's name, style,
    /// weight and stretch from now on.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or has no such face.
    pub fn add_entry(self, entry: &FontListEntry) -> Result<(), TextError> {
        self.engine.add_font_entry(self.catalogue_entry(entry)?);
        Ok(())
    }

    /// Removes the first TrueType font-list entry equal to `entry`
    /// (matplotlib's `ttflist.remove`); whether there was one.
    ///
    /// # Errors
    ///
    /// When the entry's file cannot be read or has no such face.
    pub fn remove_entry(self, entry: &FontListEntry) -> Result<bool, TextError> {
        Ok(self.engine.remove_font_entry(&self.catalogue_entry(entry)?))
    }

    /// The catalogue entry of a font-list entry: its face found by file
    /// and index.
    fn catalogue_entry(self, entry: &FontListEntry) -> Result<FontEntry, TextError> {
        let faces = self.engine.file_faces(&entry.file)?;
        let font = usize::try_from(entry.index)
            .ok()
            .and_then(|index| faces.get(index).copied())
            .ok_or_else(|| {
                TextError::Font(format!("{} has no face {}", entry.file, entry.index))
            })?;
        Ok(FontEntry {
            font,
            name: entry.name.clone(),
            style: entry.style,
            variant: entry.variant,
            weight: entry.weight.value(),
            stretch: Stretch(entry.stretch.value()),
        })
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

    /// The best TrueType entry for `props` among the faces whose source
    /// `accept` takes, with its score: the loop of matplotlib's
    /// `_findfont_cached`, which `findfont(directory=...)` restricts and
    /// whose result decides what `findfont` logs. A score of 10 or more
    /// means that no family matched; `None` when `accept` takes no face.
    pub fn best_ttf_entry(
        &self,
        props: &FontProperties,
        rc: &RcParams,
        accept: impl Fn(&FontSource) -> bool,
    ) -> Option<(FontListEntry, f64)> {
        let request = props.request(rc);
        let db = self.engine.database();
        db.best_entry(&request.query, &request.generics, |id| {
            db.source(id).is_some_and(&accept)
        })
        .map(|(entry, score)| (self.list_entry(&entry), score))
    }

    /// The best AFM entry for `props` among the fonts of `directory` (all
    /// of them for `None`), with its score: [`best_ttf_entry`]'s
    /// counterpart for `afmlist` (`findfont(prop, fontext="afm",
    /// directory=...)`).
    ///
    /// [`best_ttf_entry`]: Self::best_ttf_entry
    pub fn best_afm_entry(
        &self,
        props: &FontProperties,
        rc: &RcParams,
        directory: Option<AfmDirectory>,
    ) -> Option<(FontListEntry, f64)> {
        let request = props.request(rc);
        AfmFont::best_scored(&request.query, &request.generics, directory)
            .map(|(font, score)| (afm_entry(font), score))
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

/// An AFM font as an entry of `afmlist` (matplotlib's `afmFontProperty`).
fn afm_entry(font: &AfmFont) -> FontListEntry {
    FontListEntry {
        file: font.file.to_owned(),
        index: 0,
        name: font.name.to_owned(),
        style: font.style,
        variant: FontVariant::Normal,
        weight: FontWeight::from_name(font.weight)
            .unwrap_or(FontWeight::numeric(font.weight_value().0)),
        stretch: stretch_named(font.stretch.0),
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
    fn best_afm_entries_within_a_directory() {
        let manager = FontManager::bundled();
        let rc = RcParams::matplotlib();
        let serif = props(&["serif"]);
        let best = |directory| {
            manager
                .best_afm_entry(&serif, &rc, directory)
                .map(|(entry, score)| (entry.file, score < 10.0))
        };
        assert_eq!(
            best(Some(AfmDirectory::PdfCore)),
            Some(("Times-Roman.afm".to_owned(), true))
        );
        assert_eq!(
            best(Some(AfmDirectory::PostScript)),
            Some(("pncr8a.afm".to_owned(), true))
        );
        let (_, unknown) = manager
            .best_afm_entry(&props(&["No Such Font"]), &rc, None)
            .expect("fonts to score");
        assert!(unknown >= 10.0);
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

    #[test]
    fn best_entries_within_some_sources() {
        let manager = FontManager::bundled();
        let rc = RcParams::matplotlib();
        let heavy = FontProperties {
            weight: FontWeight::numeric(750),
            ..props(&["DejaVu Sans"])
        };
        let (entry, score) = manager.best_ttf_entry(&heavy, &rc, |_| true).unwrap();
        assert_eq!(entry.file, "DejaVuSans-Bold.ttf");
        assert_eq!(entry.weight, FontWeight::numeric(700));
        assert!(score < 10.0);
        let serif_only = |source: &FontSource| {
            source
                .file_name()
                .is_some_and(|name| name.starts_with("DejaVuSerif"))
        };
        let (entry, score) = manager.best_ttf_entry(&heavy, &rc, serif_only).unwrap();
        assert!(entry.file.starts_with("DejaVuSerif") && score >= 10.0);
        assert!(manager.best_ttf_entry(&heavy, &rc, |_| false).is_none());
    }

    #[test]
    fn list_entries_are_appended_and_removed() {
        // The global engine: other tests count the bundled engine's list.
        let manager = FontManager::global();
        let rc = RcParams::matplotlib();
        let entry = FontListEntry {
            name: "pltrs test alias".to_owned(),
            ..manager.font_list(FontExt::Ttf)[0].clone()
        };
        let alias = props(&["pltrs test alias"]);
        assert!(manager.find_font(&alias, &rc, FontExt::Ttf, false).is_err());
        manager.add_entry(&entry).unwrap();
        assert_eq!(
            manager.find_font(&alias, &rc, FontExt::Ttf, false),
            Ok(entry.file.clone())
        );
        assert_eq!(manager.remove_entry(&entry), Ok(true));
        assert_eq!(manager.remove_entry(&entry), Ok(false));
        assert!(manager.find_font(&alias, &rc, FontExt::Ttf, false).is_err());
        let missing = FontListEntry { index: 7, ..entry };
        assert!(manager.add_entry(&missing).is_err());
    }
}
