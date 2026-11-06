//! Font management: the bundled fonts, optional system fonts,
//! matplotlib's `findfont` matching, and Type 1 fonts opened by path.

mod afm;
mod afm_catalogue;
mod arena;
mod bundled;
mod catalogue;
mod entry;
mod held;
mod query;
mod score;
mod sparse;
mod system;
mod type1;

pub use afm::{AfmDirectory, AfmFont};
pub use catalogue::{CatalogueReport, CatalogueState, catalogue_file_name};
pub use entry::{FaceStyle, FontEntry, FontId, alternative_family_names, face_style};
pub use query::{
    FontQuery, FontStyle, FontVariant, GenericFamilies, GenericFamily, Stretch, Weight,
};
pub use score::{
    family as score_family, size as score_size, stretch as score_stretch, style as score_style,
    variant as score_variant, weight as score_weight,
};
pub use system::{RegistryReader, read_registry_with};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, PoisonError, RwLock, RwLockReadGuard};

use crate::autofit::Autohinter;
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::shape::FontChain;
use crate::tables::directory::face_count;
use arena::Arena;
use bundled::{BUNDLED_FONTS, LAST_RESORT};
use held::HeldFile;

/// The contents of the bundled font file called `file_name` (such as
/// `"Inter-Regular.ttf"`), for embedding it elsewhere (web pages).
pub fn bundled_font_data(file_name: &str) -> Option<&'static [u8]> {
    BUNDLED_FONTS
        .iter()
        .find(|font| font.file == file_name)
        .map(|font| font.data)
}

/// `(file name, contents)` of every bundled font file, for writing them
/// where other programs look for fonts (matplotlib's `mpl-data/fonts/ttf`).
///
/// ```
/// let mut fonts = pltrs_font::bundled_font_files();
/// assert!(fonts.any(|(name, data)| name == "DejaVuSans.ttf" && !data.is_empty()));
/// ```
pub fn bundled_font_files() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    BUNDLED_FONTS.iter().map(|font| (font.file, font.data))
}

/// The bundled font `path` stands for: a bare bundled file name, or a
/// file holding exactly a bundled font's bytes under its name (one
/// written out of the binary).
fn bundled_name_of(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    let font = BUNDLED_FONTS.iter().find(|font| font.file == name)?;
    let bare = path.parent().is_none_or(|dir| dir.as_os_str().is_empty());
    let same = || {
        fs::metadata(path).is_ok_and(|meta| meta.len() == font.data.len() as u64)
            && fs::read(path).is_ok_and(|bytes| bytes == font.data)
    };
    (bare || same()).then_some(font.file)
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

/// Font bytes: embedded, or a file held for drawing from the first use
/// of one of its faces (which share it).
#[derive(Clone, Debug)]
enum FontData {
    Static(&'static [u8]),
    File(Arc<FontFile>),
}

/// A font file on disk, held for drawing once one of its faces is used.
#[derive(Debug)]
struct FontFile {
    path: PathBuf,
    held: OnceLock<std::result::Result<Box<HeldFile>, std::io::ErrorKind>>,
}

impl FontData {
    /// The file at `path`, held for drawing on first use.
    fn read_lazily(path: &Path) -> Self {
        Self::File(Arc::new(FontFile {
            path: path.to_path_buf(),
            held: OnceLock::new(),
        }))
    }

    /// Parses face `index`.
    fn face(&self, index: u32) -> Result<Face<'_>> {
        match self {
            Self::Static(bytes) => Face::parse(bytes, index),
            Self::File(file) => {
                let held = file
                    .held
                    .get_or_init(|| {
                        HeldFile::open(&file.path)
                            .map(Box::new)
                            .map_err(|error| error.kind())
                    })
                    .as_ref()
                    .map_err(|&kind| FontError::Io {
                        path: file.path.clone(),
                        kind,
                    })?;
                Face::parse_held(&**held, index)
            }
        }
    }
}

#[derive(Debug)]
struct StoredFace {
    source: FontSource,
    index: u32,
    data: FontData,
    /// The autohinter's analysis of the face, made on first use.
    autohinter: OnceLock<Box<Autohinter>>,
}

/// A catalogue of font faces with matplotlib's matching rules. Faces can
/// be added while the catalogue is shared.
#[derive(Debug)]
pub struct FontDatabase {
    faces: Arena<StoredFace>,
    entries: RwLock<Vec<FontEntry>>,
    generics: GenericFamilies,
    default_family: String,
    type1: type1::Type1Faces,
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
            faces: Arena::new(),
            entries: RwLock::new(Vec::new()),
            generics: GenericFamilies::default(),
            default_family: "DejaVu Sans".to_owned(),
            type1: type1::Type1Faces::default(),
        }
    }

    /// A database with every bundled font (matplotlib's `mpl-data` fonts
    /// and Inter).
    ///
    /// Their catalogue entries are known in advance (the tests check them
    /// against the fonts), so that no page of the embedded fonts is read
    /// until a face is used: parsing all 45 would map about 6 MiB of them.
    pub fn with_bundled_fonts() -> Self {
        let db = Self::new();
        let mut entries = Vec::new();
        for font in &BUNDLED_FONTS {
            let id = FontId(db.faces.push(StoredFace {
                source: FontSource::Bundled(font.file),
                index: 0,
                data: FontData::Static(font.data),
                autohinter: OnceLock::new(),
            }));
            entries.extend(font.entries.iter().map(|entry| FontEntry {
                font: id,
                name: entry.name.to_owned(),
                style: entry.style,
                variant: FontVariant::Normal,
                weight: entry.weight,
                stretch: Stretch::NORMAL,
            }));
        }
        *db.entries.write().unwrap_or_else(PoisonError::into_inner) = entries;
        db
    }

    /// Adds every face of `bytes`, catalogued for matching unless only
    /// `catalogue` is unset.
    fn add_faces(
        &self,
        source: &FontSource,
        bytes: &[u8],
        catalogue: bool,
        data: &FontData,
    ) -> Result<Vec<FontId>> {
        Ok(self.add_described(source, describe_faces(bytes)?, catalogue, data))
    }

    /// Adds a face of `source` for each of `faces`, in order, catalogued
    /// with its descriptions unless only `catalogue` is unset; the faces
    /// share `data`.
    fn add_described(
        &self,
        source: &FontSource,
        faces: Vec<Vec<FaceStyle>>,
        catalogue: bool,
        data: &FontData,
    ) -> Vec<FontId> {
        faces
            .into_iter()
            .zip(0..)
            .map(|(descriptions, index)| {
                let id = FontId(self.faces.push(StoredFace {
                    source: source.clone(),
                    index,
                    data: data.clone(),
                    autohinter: OnceLock::new(),
                }));
                if catalogue {
                    self.entries
                        .write()
                        .unwrap_or_else(PoisonError::into_inner)
                        .extend(
                            descriptions
                                .into_iter()
                                .map(|description| FontEntry::described(id, description)),
                        );
                }
                id
            })
            .collect()
    }

    /// Adds every face of the font file at `path` to the catalogue
    /// (matplotlib's `fontManager.addfont`). The file is parsed now and
    /// read again lazily when a face is requested.
    pub fn add_file(&self, path: impl AsRef<Path>) -> Result<Vec<FontId>> {
        self.load_file(path.as_ref(), true)
    }

    /// The faces of the font file `path` for text set in it regardless of
    /// matching (matplotlib's `FontProperties(fname=...)`): read on first
    /// use without being catalogued. A bare file name may also name a
    /// bundled font, which has no path on disk.
    pub fn file_faces(&self, path: impl AsRef<Path>) -> Result<Vec<FontId>> {
        let path = path.as_ref();
        if let Some(name) = bundled_name_of(path) {
            let bundled = self
                .faces_where(|source| matches!(source, FontSource::Bundled(file) if *file == name));
            if !bundled.is_empty() {
                return Ok(bundled);
            }
        }
        let known = self.faces_from(&FontSource::File(path.to_path_buf()));
        if known.is_empty() {
            self.load_file(path, false)
        } else {
            Ok(known)
        }
    }

    /// Reads the font file at `path` (but the tables only drawing reads)
    /// and adds its faces.
    fn load_file(&self, path: &Path, catalogue: bool) -> Result<Vec<FontId>> {
        let bytes = sparse::read(path).map_err(|error| FontError::Io {
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;
        self.add_faces(
            &FontSource::File(path.to_path_buf()),
            &bytes,
            catalogue,
            &FontData::read_lazily(path),
        )
    }

    /// Adds all `.ttf`/`.otf`/`.ttc` files below `dir`; files that cannot
    /// be read or parsed are skipped, as matplotlib does. Returns the number
    /// of faces added.
    pub fn add_directory(&self, dir: impl AsRef<Path>) -> usize {
        system::font_files(dir.as_ref())
            .iter()
            .map(|path| self.add_file(path).map_or(0, |ids| ids.len()))
            .sum()
    }

    /// Adds the fonts installed on the system, where matplotlib's
    /// `findSystemFonts` finds them (see
    /// [`FontDatabase::system_font_directories`]; on Windows also the
    /// font files the registry lists elsewhere). Returns the number of
    /// faces added.
    pub fn add_system_fonts(&self) -> usize {
        system::SystemFonts::of_this_system()
            .font_files()
            .iter()
            .map(|path| self.add_file(path).map_or(0, |ids| ids.len()))
            .sum()
    }

    /// Adds the fonts installed on the system as
    /// [`add_system_fonts`](Self::add_system_fonts) does, keeping their
    /// catalogue in the file `catalogue` between processes, as matplotlib
    /// keeps its font list in its cache directory: only the font files new
    /// or changed since it was written are read, and none is kept open
    /// (see [`add_files_catalogued`](Self::add_files_catalogued)).
    pub fn add_system_fonts_catalogued(&self, catalogue: &Path) -> CatalogueReport {
        self.add_files_catalogued(
            &system::SystemFonts::of_this_system().font_files(),
            catalogue,
        )
    }

    /// Adds the faces of the font files `paths`, in order, as
    /// [`add_file`](Self::add_file) adds each (skipping files that cannot
    /// be read or parsed), with their catalogue kept in the file
    /// `catalogue`: the files unchanged since it was written are not
    /// opened, and the catalogue is brought up to date (it is rebuilt when
    /// it is missing, broken or of another version, and the report says
    /// why it could not be written, when it could not). Faces are read
    /// when they are used.
    pub fn add_files_catalogued(&self, paths: &[PathBuf], catalogue: &Path) -> CatalogueReport {
        let (files, report) = catalogue::refresh(catalogue, paths, describe_file);
        for file in files {
            if let Some(faces) = file.faces {
                let source = FontSource::File(file.path.clone());
                self.add_described(&source, faces, true, &FontData::read_lazily(&file.path));
            }
        }
        report
    }

    /// The directories this system keeps fonts in, as matplotlib searches
    /// them: on Linux `/usr/share/fonts`, `/usr/local/share/fonts`,
    /// `$XDG_DATA_HOME/fonts` (`~/.local/share/fonts`), `~/.fonts` and the
    /// old X11 directories; on macOS those and `/Library/Fonts`,
    /// `/System/Library/Fonts`, `~/Library/Fonts`; on Windows
    /// `%WINDIR%\Fonts` (the `Fonts` shell folder) and the user's
    /// `%LOCALAPPDATA%\Microsoft\Windows\Fonts`.
    pub fn system_font_directories() -> Vec<PathBuf> {
        system::SystemFonts::of_this_system().directories
    }

    /// All catalogue entries, in insertion order (read-locked while held).
    pub fn entries(&self) -> RwLockReadGuard<'_, Vec<FontEntry>> {
        self.entries.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Appends `entry` to the catalogue: a name, style and weight its face
    /// is matched by from now on (matplotlib's `ttflist.append`).
    pub fn add_entry(&self, entry: FontEntry) {
        self.entries
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push(entry);
    }

    /// Removes the first catalogue entry equal to `entry` (matplotlib's
    /// `ttflist.remove`); whether there was one.
    pub fn remove_entry(&self, entry: &FontEntry) -> bool {
        let mut entries = self.entries.write().unwrap_or_else(PoisonError::into_inner);
        let found = entries.iter().position(|candidate| candidate == entry);
        found.map(|index| entries.remove(index)).is_some()
    }

    /// The faces added from `source`, in order.
    pub fn faces_from(&self, source: &FontSource) -> Vec<FontId> {
        self.faces_where(|candidate| candidate == source)
    }

    /// The faces whose source passes `wanted`, in order.
    fn faces_where(&self, wanted: impl Fn(&FontSource) -> bool) -> Vec<FontId> {
        self.faces
            .iter()
            .enumerate()
            .filter(|(_, face)| wanted(&face.source))
            .map(|(index, _)| FontId(index))
            .collect()
    }

    /// The number of faces, catalogued or not.
    pub fn face_count(&self) -> usize {
        self.faces.len()
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
        stored.data.face(stored.index)
    }

    /// The autohinter's analysis of face `id`, made on first use and kept
    /// (use it with [`Self::face`]).
    pub fn autohinter(&self, id: FontId) -> Result<&Autohinter> {
        let stored = self.faces.get(id.0).ok_or(FontError::UnknownFont(id.0))?;
        if let Some(analysis) = stored.autohinter.get() {
            return Ok(analysis);
        }
        let face = self.face(id)?;
        Ok(stored
            .autohinter
            .get_or_init(|| Box::new(Autohinter::new(&face))))
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
        self.best_entry(query, generics, |_| true)
            .filter(|(_, score)| *score < 10.0)
            .map(|(entry, _)| entry.font)
    }

    /// The best-scoring entry among those of the faces `accept` takes, and
    /// its score, as matplotlib's `_findfont_cached` finds it (`accept`
    /// restricts the search to a directory); `None` when it takes none. A
    /// score of 10 or more means that no family matched.
    pub fn best_entry(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
        accept: impl Fn(FontId) -> bool,
    ) -> Option<(FontEntry, f64)> {
        let mut best: Option<(&FontEntry, f64)> = None;
        let entries = self.entries();
        for entry in entries.iter().filter(|entry| accept(entry.font)) {
            let score = score::total(query, entry, generics);
            if best.is_none_or(|(_, best_score)| score < best_score) {
                best = Some((entry, score));
            }
            if score == 0.0 {
                break;
            }
        }
        best.map(|(entry, score)| (entry.clone(), score))
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
        let found: Vec<FontId> = self
            .family_matches(query, generics)
            .into_iter()
            .flatten()
            .map(|entry| entry.font)
            .collect();
        if found.is_empty() {
            Err(FontError::NoMatchingFont(query.to_string()))
        } else {
            Ok(found)
        }
    }

    /// The entry of the best face of each family of the query, in family
    /// order, `None` for a family no face matches: what
    /// `_find_fonts_by_props` finds family by family (and logs about, for
    /// the families it does not find).
    pub fn family_matches(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
    ) -> Vec<Option<FontEntry>> {
        query
            .families
            .iter()
            .map(|family| {
                let single = FontQuery {
                    families: vec![family.clone()],
                    ..query.clone()
                };
                self.best_entry(&single, generics, |_| true)
                    .filter(|(_, score)| *score < 10.0)
                    .map(|(entry, _)| entry)
            })
            .collect()
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

/// The descriptions of the catalogue entries of each face of the font
/// `bytes` hold, failing when one of the faces cannot be parsed.
fn describe_faces(bytes: &[u8]) -> Result<Vec<Vec<FaceStyle>>> {
    (0..face_count(bytes)?)
        .map(|index| Face::parse(bytes, index).map(|face| entry::descriptions(&face)))
        .collect()
}

/// What [`FontDatabase::add_file`] catalogues of the font file at `path`:
/// the descriptions of each face, none when a face cannot be parsed.
fn describe_file(path: &Path) -> std::io::Result<Vec<Vec<FaceStyle>>> {
    let bytes = sparse::read(path)?;
    Ok(describe_faces(&bytes).unwrap_or_default())
}

#[cfg(test)]
mod tests;
