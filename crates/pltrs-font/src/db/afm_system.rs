//! The Adobe Font Metrics files installed on the system, which
//! matplotlib's AFM font list (`fontManager.afmlist`) holds after its own
//! (`findSystemFonts(fontext="afm")`): the URW base 35 fonts of
//! Ghostscript on Linux, for instance. Each is described as
//! `afmFontProperty` describes it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use super::afm::{AFM_DEFAULT_FAMILY, AfmFont, afm_score};
use super::entry::FaceStyle;
use super::query::{FontQuery, FontStyle, FontVariant, GenericFamilies, Stretch, Weight};
use super::system::is_afm_file;
use super::{FontDatabase, describe_font_file};
use crate::afm::{AfmError, AfmMetrics};
use crate::error::{FontError, Result};

/// What matplotlib's `afmFontProperty` reads from a metrics file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfmStyle {
    /// The family name (`get_familyname`).
    pub name: String,
    /// Italic for a non-zero `ItalicAngle` or an italic family name,
    /// oblique for an oblique one.
    pub style: FontStyle,
    /// Small capitals for the families `capitals` and `small-caps`.
    pub variant: FontVariant,
    /// The `Weight` keyword in lower case, or `normal` for one matplotlib
    /// does not know.
    pub weight: String,
    /// From the font name: semi-condensed, condensed, expanded or normal.
    pub stretch: Stretch,
}

pltrs_core::archive_struct!(AfmStyle {
    name,
    style,
    variant,
    weight,
    stretch
});

impl AfmStyle {
    /// `afmFontProperty` of `metrics`.
    ///
    /// # Errors
    ///
    /// When the header lacks the names, the italic angle or the weight
    /// (matplotlib skips such files).
    pub fn of(metrics: &AfmMetrics) -> Result<Self, AfmError> {
        let name = metrics.family_name()?;
        let font_name = metrics.font_name()?.to_lowercase();
        let lower = name.to_lowercase();
        let style = if metrics.angle()? != 0.0 || lower.contains("italic") {
            FontStyle::Italic
        } else if lower.contains("oblique") {
            FontStyle::Oblique
        } else {
            FontStyle::Normal
        };
        let variant = if lower == "capitals" || lower == "small-caps" {
            FontVariant::SmallCaps
        } else {
            FontVariant::Normal
        };
        let mut weight = metrics.weight()?.to_lowercase();
        if weight.parse::<Weight>().is_err() {
            "normal".clone_into(&mut weight);
        }
        let stretch = if font_name.contains("demi cond") {
            Stretch::SEMI_CONDENSED
        } else if ["narrow", "cond"].iter().any(|w| font_name.contains(w)) {
            Stretch::CONDENSED
        } else if ["wide", "expanded", "extended"]
            .iter()
            .any(|w| font_name.contains(w))
        {
            Stretch::EXPANDED
        } else {
            Stretch::NORMAL
        };
        Ok(Self {
            name: name.to_owned(),
            style,
            variant,
            weight,
            stretch,
        })
    }

    /// The numeric weight (unknown keywords were made `normal`).
    pub fn weight_value(&self) -> Weight {
        self.weight.parse().unwrap_or(Weight::NORMAL)
    }

    /// The `findfont` score of the font for `query` (0 is a perfect
    /// match).
    pub fn score(&self, query: &FontQuery, generics: &GenericFamilies) -> f64 {
        afm_score(
            query,
            generics,
            &self.name,
            self.style,
            self.variant,
            self.weight_value(),
            self.stretch,
        )
    }
}

/// An AFM font file of the system and its description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfmEntry {
    /// The metrics file.
    pub path: PathBuf,
    /// What `afmFontProperty` reads from it.
    pub style: AfmStyle,
}

impl AfmEntry {
    /// Reads the metrics file at `path` and describes it.
    ///
    /// # Errors
    ///
    /// When the file cannot be read, is no AFM file, or lacks what
    /// `afmFontProperty` reads.
    pub fn read(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).map_err(|error| FontError::Io {
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;
        let metrics = AfmMetrics::parse(&bytes).map_err(FontError::Afm)?;
        Ok(Self {
            path: path.to_path_buf(),
            style: AfmStyle::of(&metrics).map_err(FontError::Afm)?,
        })
    }
}

/// What the catalogue keeps of a file: the descriptions of each of its
/// faces (none when it holds no font pltrs reads), or that of an Adobe
/// Font Metrics file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Described {
    Faces(Vec<Vec<FaceStyle>>),
    Afm(AfmStyle),
}

pltrs_core::archive_enum!(Described {
    Faces(faces),
    Afm(style)
});

/// What the catalogue keeps of the file at `path`: an AFM file that is
/// read but cannot be described holds no font, as matplotlib skips it.
pub(super) fn describe_file(path: &Path) -> io::Result<Described> {
    if !is_afm_file(path) {
        return describe_font_file(path).map(Described::Faces);
    }
    match AfmEntry::read(path) {
        Ok(entry) => Ok(Described::Afm(entry.style)),
        Err(FontError::Io { kind, .. }) => Err(kind.into()),
        Err(_) => Ok(Described::Faces(Vec::new())),
    }
}

/// A font of matplotlib's AFM font list: one it ships, or one of the
/// system's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfmChoice {
    /// A font of matplotlib's data.
    Bundled(&'static AfmFont),
    /// A font of the system (or added with `addfont`).
    System(AfmEntry),
}

impl AfmChoice {
    /// The font's file: the bare name of a bundled one, the path of the
    /// system's.
    pub fn file(&self) -> String {
        match self {
            Self::Bundled(font) => font.file.to_owned(),
            Self::System(entry) => entry.path.display().to_string(),
        }
    }
}

impl FontDatabase {
    /// The first font of the whole AFM font list (matplotlib's, then the
    /// system's) with the lowest `findfont` score for `query`, and that
    /// score (10 or more when no family matched).
    pub fn best_afm(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
    ) -> Option<(AfmChoice, f64)> {
        let bundled = AfmFont::best_scored(query, generics, None)
            .map(|(font, score)| (AfmChoice::Bundled(font), score));
        let system = self
            .best_afm_entry(query, generics)
            .map(|(entry, score)| (AfmChoice::System(entry), score));
        match (bundled, system) {
            (Some(bundled), Some(system)) if system.1 < bundled.1 => Some(system),
            (bundled, system) => bundled.or(system),
        }
    }

    /// matplotlib's `findfont(prop, fontext="afm")` over the whole AFM
    /// font list: the best font a family of `query` matches, else, with
    /// `fallback_to_default`, the best `Helvetica` face.
    ///
    /// # Errors
    ///
    /// When no family matches (and none of `Helvetica` either).
    pub fn find_afm(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
        fallback_to_default: bool,
    ) -> Result<AfmChoice> {
        let matched = |query: &FontQuery| {
            self.best_afm(query, generics)
                .filter(|(_, score)| *score < 10.0)
                .map(|(font, _)| font)
        };
        matched(query)
            .or_else(|| {
                let default = FontQuery {
                    families: vec![AFM_DEFAULT_FAMILY.to_owned()],
                    ..query.clone()
                };
                fallback_to_default.then(|| matched(&default))?
            })
            .ok_or_else(|| FontError::NoMatchingFont(query.to_string()))
    }

    /// Adds the AFM font of the metrics file at `path` to the AFM font
    /// list (matplotlib's `fontManager.addfont` of an `.afm` file).
    ///
    /// # Errors
    ///
    /// When the file cannot be read or described.
    pub fn add_afm_file(&self, path: impl AsRef<Path>) -> Result<()> {
        self.add_afm(AfmEntry::read(path.as_ref())?);
        Ok(())
    }

    /// Appends `entry` to the AFM font list.
    pub(super) fn add_afm(&self, entry: AfmEntry) {
        self.afm_entries_mut().push(entry);
    }

    /// The AFM fonts added (the system's), in order: matplotlib's
    /// `afmlist` after its own fonts ([`AfmFont`](crate::AfmFont)).
    pub fn afm_entries(&self) -> Vec<AfmEntry> {
        self.afm
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The first of the AFM fonts added with the lowest `findfont` score
    /// for `query`, and that score (10 or more when no family matched);
    /// `None` when none was added.
    pub fn best_afm_entry(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
    ) -> Option<(AfmEntry, f64)> {
        let entries = self.afm.read().unwrap_or_else(PoisonError::into_inner);
        let mut best: Option<(&AfmEntry, f64)> = None;
        for entry in entries.iter() {
            let score = entry.style.score(query, generics);
            if best.is_none_or(|(_, best_score)| score < best_score) {
                best = Some((entry, score));
            }
            if score == 0.0 {
                break;
            }
        }
        best.map(|(entry, score)| (entry.clone(), score))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_metrics_as_matplotlib() {
        let data = b"StartFontMetrics 2\nFontName NimbusSansNarrow-BoldOblique\n\
                     FullName Nimbus Sans Narrow Bold Oblique\nFamilyName Nimbus Sans Narrow\n\
                     Weight Bold\nItalicAngle -12\nStartCharMetrics 0\nEndCharMetrics\n";
        let metrics = AfmMetrics::parse(data).unwrap();
        let style = AfmStyle::of(&metrics).unwrap();
        assert_eq!(style.name, "Nimbus Sans Narrow");
        assert_eq!(
            (style.style, style.variant, style.weight.as_str()),
            (FontStyle::Italic, FontVariant::Normal, "bold")
        );
        assert_eq!(style.stretch, Stretch::CONDENSED);
        assert_eq!(style.weight_value(), Weight::BOLD);
        let query = FontQuery::families(["Nimbus Sans Narrow"])
            .with_weight(Weight::BOLD)
            .with_style(FontStyle::Italic)
            .with_stretch(Stretch::CONDENSED);
        let generics = GenericFamilies::default();
        // matplotlib scores a numeric weight against a keyword 0.05 off.
        assert_eq!(style.score(&query, &generics), 0.05);
        let odd = b"StartFontMetrics 2\nFontName X-Wide\nFamilyName small-caps\n\
                    Weight Hairline\nItalicAngle 0\nStartCharMetrics 0\nEndCharMetrics\n";
        let style = AfmStyle::of(&AfmMetrics::parse(odd).unwrap()).unwrap();
        assert_eq!(
            (style.variant, style.weight.as_str(), style.stretch),
            (FontVariant::SmallCaps, "normal", Stretch::EXPANDED)
        );
        let unweighted = b"StartFontMetrics 2\nFontName X\nStartCharMetrics 0\nEndCharMetrics\n";
        assert!(AfmStyle::of(&AfmMetrics::parse(unweighted).unwrap()).is_err());
    }

    #[test]
    fn databases_list_and_find_added_afm_fonts() {
        let dir = std::env::temp_dir().join(format!("pltrs-font-afm-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, family: &str, weight: &str| {
            let path = dir.join(name);
            let text = format!(
                "StartFontMetrics 2\nFontName {family}-{weight}\nFamilyName {family}\n\
                 Weight {weight}\nItalicAngle 0\nStartCharMetrics 0\nEndCharMetrics\n"
            );
            fs::write(&path, text).unwrap();
            path
        };
        let (regular, bold) = (
            file("a.afm", "Nimbus", "Regular"),
            file("b.AFM", "Nimbus", "Bold"),
        );
        let broken = dir.join("c.afm");
        fs::write(&broken, b"not metrics").unwrap();
        let db = FontDatabase::new();
        db.add_afm_file(&regular).unwrap();
        db.add_afm_file(&bold).unwrap();
        assert!(db.add_afm_file(&broken).is_err());
        assert!(db.add_afm_file(dir.join("missing.afm")).is_err());
        let names: Vec<_> = db.afm_entries().into_iter().map(|e| e.path).collect();
        assert_eq!(names, [regular.clone(), bold.clone()]);
        let generics = GenericFamilies::default();
        let query = FontQuery::families(["Nimbus"]).with_weight(Weight::BOLD);
        let (found, score) = db.best_afm_entry(&query, &generics).unwrap();
        assert_eq!((found.path, score), (bold.clone(), 0.05));
        assert!(matches!(describe_file(&regular), Ok(Described::Afm(_))));
        assert_eq!(
            describe_file(&broken).ok(),
            Some(Described::Faces(Vec::new()))
        );
        // The system's fonts come after matplotlib's, and win only when
        // better.
        let found = db.find_afm(&query, &generics, false).unwrap();
        assert_eq!(found.file(), bold.display().to_string());
        let helvetica = FontQuery::families(["Helvetica"]);
        let found = db.find_afm(&helvetica, &generics, false).unwrap();
        assert_eq!(found.file(), "Helvetica.afm");
        let unknown = FontQuery::families(["Unknown"]);
        assert!(db.find_afm(&unknown, &generics, false).is_err());
        let fallback = db.find_afm(&unknown, &generics, true).unwrap();
        assert!(matches!(fallback, AfmChoice::Bundled(font) if font.name == "Helvetica"));
        assert!(describe_file(&dir.join("missing.afm")).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
