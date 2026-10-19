//! Font management: the bundled fonts, optional system fonts and
//! matplotlib's `findfont` matching.

mod afm;
mod afm_catalogue;
mod bundled;
mod entry;
mod query;
mod score;
mod system;

pub use afm::{AfmDirectory, AfmFont};
pub use entry::{FontEntry, FontId};
pub use query::{
    FontQuery, FontStyle, FontVariant, GenericFamilies, GenericFamily, Stretch, Weight,
};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::autofit::Autohinter;
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::shape::FontChain;
use crate::tables::directory::face_count;
use bundled::{BUNDLED_FONTS, LAST_RESORT};

/// The contents of the bundled font file called `file_name` (such as
/// `"Inter-Regular.ttf"`), for embedding it elsewhere (web pages).
pub fn bundled_font_data(file_name: &str) -> Option<&'static [u8]> {
    BUNDLED_FONTS
        .iter()
        .find(|(name, _)| *name == file_name)
        .map(|&(_, data)| data)
}

/// Where a face's bytes come from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FontSource {
    /// A font embedded in the crate (file name as in matplotlib's
    /// `mpl-data/fonts/ttf`, or as Inter's release names it).
    Bundled(&'static str),
    /// A font file on disk.
    File(PathBuf),
}

impl FontSource {
    /// The file name, without directories.
    pub fn file_name(&self) -> Option<&str> {
        match self {
            Self::Bundled(name) => Some(name),
            Self::File(path) => path.file_name().and_then(|name| name.to_str()),
        }
    }
}

/// Font bytes: embedded, or read from disk on first use.
#[derive(Debug)]
enum FontData {
    Static(&'static [u8]),
    File {
        path: PathBuf,
        bytes: OnceLock<std::result::Result<Vec<u8>, std::io::ErrorKind>>,
    },
}

impl FontData {
    fn bytes(&self) -> Result<&[u8]> {
        match self {
            Self::Static(bytes) => Ok(bytes),
            Self::File { path, bytes } => bytes
                .get_or_init(|| fs::read(path).map_err(|error| error.kind()))
                .as_deref()
                .map_err(|&kind| FontError::Io {
                    path: path.clone(),
                    kind,
                }),
        }
    }
}

#[derive(Debug)]
struct StoredFace {
    source: FontSource,
    index: u32,
    data: FontData,
    /// The autohinter's analysis of the face, made on first use.
    autohinter: OnceLock<Autohinter>,
}

/// A catalogue of font faces with matplotlib's matching rules.
#[derive(Debug)]
pub struct FontDatabase {
    faces: Vec<StoredFace>,
    entries: Vec<FontEntry>,
    generics: GenericFamilies,
    default_family: String,
}

impl Default for FontDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl FontDatabase {
    /// An empty database with matplotlib's default generic families and
    /// `DejaVu Sans` as the default family.
    pub fn new() -> Self {
        Self {
            faces: Vec::new(),
            entries: Vec::new(),
            generics: GenericFamilies::default(),
            default_family: "DejaVu Sans".to_owned(),
        }
    }

    /// A database with every bundled font (matplotlib's `mpl-data` fonts
    /// and Inter).
    pub fn with_bundled_fonts() -> Self {
        let mut db = Self::new();
        for (name, data) in BUNDLED_FONTS {
            // Bundled fonts are known to parse (see the tests); a failure
            // would only drop that font from the catalogue.
            let _ = db.add_faces(&FontSource::Bundled(name), data, || FontData::Static(data));
        }
        db
    }

    fn add_faces(
        &mut self,
        source: &FontSource,
        bytes: &[u8],
        data: impl Fn() -> FontData,
    ) -> Result<Vec<FontId>> {
        // Parse every face first so that a broken collection adds nothing.
        let faces = (0..face_count(bytes)?)
            .map(|index| Face::parse(bytes, index).map(|face| (index, face)))
            .collect::<Result<Vec<_>>>()?;
        let mut ids = Vec::new();
        for (index, face) in faces {
            let id = FontId(self.faces.len());
            self.entries.extend(entry::entries_for_face(&face, id));
            self.faces.push(StoredFace {
                source: source.clone(),
                index,
                data: data(),
                autohinter: OnceLock::new(),
            });
            ids.push(id);
        }
        Ok(ids)
    }

    /// Adds every face of the font file at `path`. The file is parsed now
    /// and read again lazily when a face is requested.
    pub fn add_file(&mut self, path: impl AsRef<Path>) -> Result<Vec<FontId>> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| FontError::Io {
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;
        self.add_faces(&FontSource::File(path.to_path_buf()), &bytes, || {
            FontData::File {
                path: path.to_path_buf(),
                bytes: OnceLock::new(),
            }
        })
    }

    /// Adds all `.ttf`/`.otf`/`.ttc` files below `dir`; files that cannot
    /// be read or parsed are skipped, as matplotlib does. Returns the number
    /// of faces added.
    pub fn add_directory(&mut self, dir: impl AsRef<Path>) -> usize {
        system::font_files(dir.as_ref())
            .iter()
            .map(|path| self.add_file(path).map_or(0, |ids| ids.len()))
            .sum()
    }

    /// Adds the fonts of the standard Unix font directories (see
    /// [`FontDatabase::system_font_directories`]).
    pub fn add_system_fonts(&mut self) -> usize {
        Self::system_font_directories()
            .iter()
            .map(|dir| self.add_directory(dir))
            .sum()
    }

    /// `/usr/share/fonts`, `/usr/local/share/fonts`, `~/.local/share/fonts`
    /// (or `$XDG_DATA_HOME/fonts`) and `~/.fonts`.
    pub fn system_font_directories() -> Vec<PathBuf> {
        system::font_directories()
    }

    /// All catalogue entries, in insertion order.
    pub fn entries(&self) -> &[FontEntry] {
        &self.entries
    }

    /// Where face `id` comes from.
    pub fn source(&self, id: FontId) -> Option<&FontSource> {
        self.faces.get(id.0).map(|face| &face.source)
    }

    /// Index of face `id` inside its file.
    pub fn face_index(&self, id: FontId) -> Option<u32> {
        self.faces.get(id.0).map(|face| face.index)
    }

    /// Parses face `id`.
    pub fn face(&self, id: FontId) -> Result<Face<'_>> {
        let stored = self.faces.get(id.0).ok_or(FontError::UnknownFont(id.0))?;
        Face::parse(stored.data.bytes()?, stored.index)
    }

    /// The autohinter's analysis of face `id`, made on first use and kept
    /// (use it with [`Self::face`]).
    pub fn autohinter(&self, id: FontId) -> Result<&Autohinter> {
        let stored = self.faces.get(id.0).ok_or(FontError::UnknownFont(id.0))?;
        if let Some(analysis) = stored.autohinter.get() {
            return Ok(analysis);
        }
        let face = self.face(id)?;
        Ok(stored.autohinter.get_or_init(|| Autohinter::new(&face)))
    }

    /// The generic family preference lists.
    pub fn generic_families(&self) -> &GenericFamilies {
        &self.generics
    }

    /// Mutable access to the generic family lists (matplotlib's
    /// `font.serif`, `font.sans-serif`, ... rcParams).
    pub fn generic_families_mut(&mut self) -> &mut GenericFamilies {
        &mut self.generics
    }

    /// Sets the family used when nothing matches (default `DejaVu Sans`).
    pub fn set_default_family(&mut self, family: impl Into<String>) {
        self.default_family = family.into();
    }

    /// The bundled Last Resort font, if the database contains it.
    pub fn last_resort(&self) -> Option<FontId> {
        self.faces
            .iter()
            .position(|face| face.source == FontSource::Bundled(LAST_RESORT))
            .map(FontId)
    }

    /// matplotlib's `findfont` without fallback: the best-scoring face, or
    /// `None` when no family matches.
    fn best_match(&self, query: &FontQuery, generics: &GenericFamilies) -> Option<FontId> {
        let mut best: Option<(f64, FontId)> = None;
        for entry in &self.entries {
            let score = score::total(query, entry, generics);
            if best.is_none_or(|(best_score, _)| score < best_score) {
                best = Some((score, entry.font));
            }
            if score == 0.0 {
                break;
            }
        }
        best.filter(|&(score, _)| score < 10.0)
            .map(|(_, font)| font)
    }

    /// matplotlib's `findfont(prop, fallback_to_default=True)`: when no
    /// family matches, the default family is searched with the same style,
    /// weight and stretch.
    pub fn find_font(&self, query: &FontQuery) -> Result<FontId> {
        self.find_font_with(query, &self.generics)
    }

    /// [`find_font`](Self::find_font) with explicit generic-family lists
    /// (matplotlib reads them from the current `font.*` rcParams).
    pub fn find_font_with(&self, query: &FontQuery, generics: &GenericFamilies) -> Result<FontId> {
        self.best_match(query, generics)
            .or_else(|| {
                let fallback = FontQuery {
                    families: vec![self.default_family.clone()],
                    ..query.clone()
                };
                self.best_match(&fallback, generics)
            })
            .ok_or_else(|| FontError::NoMatchingFont(query.to_string()))
    }

    /// `findfont(prop, fallback_to_default=False)`.
    pub fn find_font_exact(&self, query: &FontQuery) -> Result<FontId> {
        self.find_font_exact_with(query, &self.generics)
    }

    /// [`find_font_exact`](Self::find_font_exact) with explicit
    /// generic-family lists.
    pub fn find_font_exact_with(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
    ) -> Result<FontId> {
        self.best_match(query, generics)
            .ok_or_else(|| FontError::NoMatchingFont(query.to_string()))
    }

    /// matplotlib's `_find_fonts_by_props`: the best face for each family of
    /// the query (families without a match are skipped), or the default
    /// family's best face when none matches.
    pub fn find_fonts(&self, query: &FontQuery) -> Result<Vec<FontId>> {
        self.find_fonts_with(query, &self.generics)
    }

    /// [`find_fonts`](Self::find_fonts) with explicit generic-family lists.
    pub fn find_fonts_with(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
    ) -> Result<Vec<FontId>> {
        self.find_fonts_exact_with(query, generics)
            .or_else(|_| self.find_font_with(query, generics).map(|id| vec![id]))
    }

    /// `_find_fonts_by_props(prop, fallback_to_default=False)`: the best
    /// face for each family of the query, failing when no family matches.
    pub fn find_fonts_exact_with(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
    ) -> Result<Vec<FontId>> {
        let found: Vec<FontId> = query
            .families
            .iter()
            .filter_map(|family| {
                self.best_match(
                    &FontQuery {
                        families: vec![family.clone()],
                        ..query.clone()
                    },
                    generics,
                )
            })
            .collect();
        if found.is_empty() {
            Err(FontError::NoMatchingFont(query.to_string()))
        } else {
            Ok(found)
        }
    }

    /// A fallback chain over `fonts`, ending with the Last Resort font
    /// (using its first character map, like matplotlib) when `last_resort`
    /// is set and the font is not already in the list.
    pub fn font_chain(&self, fonts: &[FontId], last_resort: bool) -> Result<FontChain<'_>> {
        let resort = self.last_resort();
        let mut ids = fonts.to_vec();
        if let Some(id) = resort.filter(|id| last_resort && !ids.contains(id)) {
            ids.push(id);
        }
        let faces = ids
            .iter()
            .map(|&id| {
                let mut face = self.face(id)?;
                if Some(id) == resort {
                    face.select_charmap(0)?;
                }
                Ok(face)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(FontChain::from_database(faces, self, ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_are_catalogued() {
        let db = FontDatabase::with_bundled_fonts();
        assert_eq!(db.faces.len(), BUNDLED_FONTS.len());
        assert!(db.entries().iter().any(|entry| entry.name == "DejaVu Sans"));
        let resort = db.last_resort().unwrap();
        assert_eq!(
            db.source(resort).and_then(FontSource::file_name),
            Some(LAST_RESORT)
        );
        assert_eq!(db.face_index(resort), Some(0));
        assert!(db.face(FontId(usize::MAX)).is_err());
        for index in 0..db.faces.len() {
            assert!(db.face(FontId(index)).is_ok());
        }
    }

    #[test]
    fn find_font_basics() {
        let db = FontDatabase::with_bundled_fonts();
        let name = |id: FontId| {
            db.source(id)
                .and_then(FontSource::file_name)
                .unwrap_or_default()
                .to_owned()
        };
        assert_eq!(
            name(db.find_font(&FontQuery::default()).unwrap()),
            "DejaVuSans.ttf"
        );
        let bold = FontQuery::families(["serif"]).with_weight(Weight::BOLD);
        assert_eq!(name(db.find_font(&bold).unwrap()), "DejaVuSerif-Bold.ttf");
        let missing = FontQuery::families(["No Such Font"]);
        assert_eq!(name(db.find_font(&missing).unwrap()), "DejaVuSans.ttf");
        assert!(db.find_font_exact(&missing).is_err());
        let fonts = db
            .find_fonts(&FontQuery::families([
                "cmr10",
                "No Such Font",
                "DejaVu Sans",
            ]))
            .unwrap();
        assert_eq!(fonts.len(), 2);
        let chain = db.font_chain(&fonts, true).unwrap();
        assert_eq!(chain.faces().len(), 3);
        assert_eq!(
            FontDatabase::new().find_font(&FontQuery::default()),
            Err(FontError::NoMatchingFont(FontQuery::default().to_string()))
        );
    }

    #[test]
    fn bundled_files_are_available_by_name() {
        let inter = bundled_font_data("Inter-Regular.ttf").unwrap();
        assert_eq!(face_count(inter), Ok(1));
        assert!(bundled_font_data("Inter-Thin.ttf").is_none());
    }

    #[test]
    fn inter_is_found_by_name_only() {
        let db = FontDatabase::with_bundled_fonts();
        let name = |query: &FontQuery| {
            let id = db.find_font(query).unwrap();
            db.source(id)
                .and_then(FontSource::file_name)
                .unwrap()
                .to_owned()
        };
        let semibold = FontQuery::families(["Inter", "DejaVu Sans"]).with_weight(Weight(600));
        assert_eq!(name(&semibold), "Inter-SemiBold.ttf");
        let italic = FontQuery::families(["Inter"]).with_style(FontStyle::Italic);
        assert_eq!(name(&italic), "Inter-Italic.ttf");
        // matplotlib's lists do not name Inter: their queries are unchanged.
        let sans = FontQuery::families(["sans-serif"]).with_weight(Weight(600));
        assert_eq!(name(&sans), "DejaVuSans-Bold.ttf");
        let unknown = FontQuery::families(["Helvetica"]).with_weight(Weight(600));
        assert_eq!(name(&unknown), "DejaVuSans-Bold.ttf");
    }

    #[test]
    fn unreadable_files_are_reported() {
        let mut db = FontDatabase::new();
        assert!(matches!(
            db.add_file("/nonexistent/font.ttf"),
            Err(FontError::Io { .. })
        ));
        assert_eq!(db.add_directory("/nonexistent"), 0);
    }
}
