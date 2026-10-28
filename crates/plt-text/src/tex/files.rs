//! Where TeX's files come from and what is kept of them: the metrics and
//! virtual fonts DVI files refer to, the font map, encodings, parsed DVI
//! files, and how a DVI glyph maps to a glyph of its Type 1 font
//! (matplotlib's `lru_cache`d `_tfmfile`, `_vffile`, `PsfontsMap`, and
//! `DviFont.resolve_path`, `effects` and `_index_dvi_to_freetype`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use plt_font::{GlyphId, Type1Effects, Type1Face};

use super::dvi::{DviFont, RawPage, read_dvi};
use super::error::{TexError, TexFile};
use super::kpathsea::Kpathsea;
use super::psfonts::{PsFont, PsfontsMap, parse_enc};
use super::tfm::Tfm;
use super::vf::Vf;
use crate::engine::TextEngine;

/// How deeply virtual fonts may refer to other virtual fonts.
const MAX_VF_DEPTH: usize = 8;

/// How many entries a cache keeps before it starts afresh.
const CACHE_LIMIT: usize = 4096;

/// Finds TeX's files by name (matplotlib's `find_tex_file`).
pub trait FileFinder: Send + Sync + Debug {
    /// The path of the file called `name`.
    ///
    /// # Errors
    ///
    /// [`TexError::NotFound`] when there is no such file.
    fn find(&self, name: &str) -> Result<PathBuf, TexError>;

    /// The contents of the file called `name`.
    ///
    /// # Errors
    ///
    /// When it cannot be found or read.
    fn read(&self, name: &str) -> Result<Vec<u8>, TexError> {
        let path = self.find(name)?;
        std::fs::read(&path).map_err(|err| TexError::io(&path, &err))
    }
}

/// Where a DVI or VF file's fonts come from.
pub trait FontSource {
    /// The metrics of TeX font `name`.
    ///
    /// # Errors
    ///
    /// [`TexError::NotFound`] when the font has no metrics (a DVI file
    /// only fails when it sets a glyph in it), or when they are malformed.
    fn tfm(&self, name: &str) -> Result<Arc<Tfm>, TexError>;

    /// The virtual font TeX font `name` is, `None` when it is not one.
    ///
    /// # Errors
    ///
    /// When the virtual font is malformed.
    fn vf(&self, name: &str) -> Result<Option<Arc<Vf>>, TexError>;
}

thread_local! {
    /// The virtual fonts being read on this thread, outermost first.
    static LOADING_VF: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// A map behind a lock, shared by the caches.
#[derive(Debug)]
struct Cache<K, V>(Mutex<HashMap<K, V>>);

impl<K: Eq + Hash, V: Clone> Cache<K, V> {
    fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    fn get(&self, key: &K) -> Option<V> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
    }

    fn insert(&self, key: K, value: V) -> V {
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if map.len() >= CACHE_LIMIT {
            map.clear();
        }
        map.insert(key, value.clone());
        value
    }
}

/// TeX's files found through a [`FileFinder`], each read once.
#[derive(Debug)]
pub struct TexFiles {
    finder: Box<dyn FileFinder>,
    tfms: Cache<String, Arc<Tfm>>,
    vfs: Cache<String, Option<Arc<Vf>>>,
    maps: Cache<PathBuf, Arc<PsfontsMap>>,
    encodings: Cache<PathBuf, Arc<Vec<String>>>,
    glyph_maps: Cache<Vec<u8>, Arc<Vec<GlyphId>>>,
    pages: Cache<PathBuf, Arc<Vec<RawPage>>>,
}

impl TexFiles {
    /// Files found by `finder`.
    pub fn new(finder: impl FileFinder + 'static) -> Self {
        Self {
            finder: Box::new(finder),
            tfms: Cache::new(),
            vfs: Cache::new(),
            maps: Cache::new(),
            encodings: Cache::new(),
            glyph_maps: Cache::new(),
            pages: Cache::new(),
        }
    }

    /// The files of the texmf tree, found by kpathsea, shared by the whole
    /// process; fonts kpathsea makes go to `cache_dir/vartexfonts` (the
    /// cache directory of the first call).
    pub fn kpathsea(cache_dir: &Path) -> &'static Self {
        static FILES: OnceLock<TexFiles> = OnceLock::new();
        FILES.get_or_init(|| Self::new(Kpathsea::new(cache_dir)))
    }

    /// The path of the file called `name` (`find_tex_file`).
    ///
    /// # Errors
    ///
    /// When there is none.
    pub fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        self.finder.find(name)
    }

    /// The pages of the DVI file at `path`, read once.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or is malformed, or a font it sets
    /// glyphs in cannot be found.
    pub fn dvi_pages(&self, path: &Path) -> Result<Arc<Vec<RawPage>>, TexError> {
        if let Some(pages) = self.pages.get(&path.to_path_buf()) {
            return Ok(pages);
        }
        let data = std::fs::read(path).map_err(|err| TexError::io(path, &err))?;
        let pages = Arc::new(read_dvi(&data, self)?);
        Ok(self.pages.insert(path.to_path_buf(), pages))
    }

    /// The font map `pdftex.map`.
    fn font_map(&self) -> Result<Arc<PsfontsMap>, TexError> {
        let path = self.find("pdftex.map")?;
        if let Some(map) = self.maps.get(&path) {
            return Ok(map);
        }
        let data = std::fs::read(&path).map_err(|err| TexError::io(&path, &err))?;
        let map = Arc::new(PsfontsMap::parse(&data, &path.display().to_string()));
        Ok(self.maps.insert(path, map))
    }

    /// The PostScript font `pdftex.map` gives for TeX font `texname`.
    ///
    /// # Errors
    ///
    /// When the map cannot be found, has no usable entry for the font, or
    /// the files its entry names cannot be found.
    pub fn psfont(&self, texname: &[u8]) -> Result<PsFont, TexError> {
        self.font_map()?.get(texname, self.finder.as_ref())
    }

    /// The glyph names of the encoding file at `path` (`_parse_enc`).
    ///
    /// # Errors
    ///
    /// When it cannot be read or is malformed.
    pub fn encoding(&self, path: &Path) -> Result<Arc<Vec<String>>, TexError> {
        if let Some(names) = self.encodings.get(&path.to_path_buf()) {
            return Ok(names);
        }
        let data = std::fs::read(path).map_err(|err| TexError::io(path, &err))?;
        let names = Arc::new(parse_enc(&data)?);
        Ok(self.encodings.insert(path.to_path_buf(), names))
    }

    /// The font file of `font` (`DviFont.resolve_path`): its Type 1 file
    /// from the font map, or the PK bitmap kpathsea makes for a Metafont
    /// font the map does not know.
    ///
    /// # Errors
    ///
    /// When the map does not know the font and it is not a Metafont font,
    /// or the map names no Type 1 file for it.
    pub fn font_path(&self, font: &DviFont) -> Result<PathBuf, TexError> {
        match self.psfont(font.texname()) {
            Ok(psfont) => psfont.filename.ok_or_else(|| TexError::NoType1Font {
                psname: String::from_utf8_lossy(&psfont.psname).into_owned(),
                texname: String::from_utf8_lossy(&psfont.texname).into_owned(),
            }),
            Err(unmapped @ TexError::UnmappedFont { .. }) => {
                let name = font.name();
                match self.find(&format!("{name}.mf")) {
                    Ok(_) => self.find(&format!("{name}.600pk")),
                    Err(_) => Err(unmapped),
                }
            }
            Err(err) => Err(err),
        }
    }

    /// The `SlantFont` and `ExtendFont` effects of `font` (none for PK
    /// fonts).
    ///
    /// # Errors
    ///
    /// As [`font_path`](Self::font_path).
    pub fn effects(&self, font: &DviFont) -> Result<Type1Effects, TexError> {
        // matplotlib's `resolve_path().match("*.600pk")`.
        if self.font_path(font)?.to_string_lossy().ends_with(".600pk") {
            return Ok(Type1Effects::default());
        }
        Ok(self.psfont(font.texname())?.effects)
    }

    /// The Type 1 face of `font`, as FreeType opens it.
    ///
    /// # Errors
    ///
    /// As [`font_path`](Self::font_path), and when the file is not a Type
    /// 1 font (PK bitmaps cannot be drawn).
    pub fn face(&self, font: &DviFont) -> Result<Arc<Type1Face>, TexError> {
        let path = self.font_path(font)?;
        if is_pk(&path) {
            return Err(TexError::Unsupported(format!(
                "{} is a PK bitmap font, which cannot be drawn",
                path.display()
            )));
        }
        TextEngine::global()
            .database()
            .type1_face(&path)
            .map_err(|err| {
                TexError::malformed(TexFile::Type1, format!("{}: {err}", path.display()))
            })
    }

    /// The glyph of `font`'s Type 1 face that character `code` of a DVI
    /// file shows (`_index_dvi_to_freetype`): through the encoding the
    /// font map gives, else the font's own encoding vector. Negative codes
    /// count from the end, as Python's indexing does.
    ///
    /// # Errors
    ///
    /// As [`face`](Self::face), when the encoding cannot be read, or when
    /// `code` is beyond it.
    pub fn glyph_index(&self, font: &DviFont, code: i64) -> Result<GlyphId, TexError> {
        let glyphs = if let Some(glyphs) = self.glyph_maps.get(&font.texname().to_vec()) {
            glyphs
        } else {
            let face = self.face(font)?;
            let glyphs = match self.psfont(font.texname())?.encoding {
                Some(encoding) => self
                    .encoding(&encoding)?
                    .iter()
                    .map(|name| face.glyph_index(name).unwrap_or(GlyphId(0)))
                    .collect(),
                None => face.encoding_vector().to_vec(),
            };
            self.glyph_maps
                .insert(font.texname().to_vec(), Arc::new(glyphs))
        };
        let len = glyphs.len() as i64;
        let index = if code < 0 { code + len } else { code };
        usize::try_from(index)
            .ok()
            .and_then(|index| glyphs.get(index).copied())
            .ok_or_else(|| {
                TexError::malformed(
                    TexFile::Encoding,
                    format!("character {code} is beyond the encoding of {}", font.name()),
                )
            })
    }
}

/// Whether `path` is a PK bitmap font (`*pk`, as matplotlib's
/// `_as_unicode_or_name` tells them; kpathsea finds `name.600pk` as
/// `dpi600/name.pk`).
fn is_pk(path: &Path) -> bool {
    path.to_string_lossy().to_lowercase().ends_with("pk")
}

impl FontSource for TexFiles {
    fn tfm(&self, name: &str) -> Result<Arc<Tfm>, TexError> {
        let name = name.to_owned();
        if let Some(tfm) = self.tfms.get(&name) {
            return Ok(tfm);
        }
        let tfm = Arc::new(Tfm::parse(&self.finder.read(&format!("{name}.tfm"))?)?);
        Ok(self.tfms.insert(name, tfm))
    }

    fn vf(&self, name: &str) -> Result<Option<Arc<Vf>>, TexError> {
        let name = name.to_owned();
        if let Some(vf) = self.vfs.get(&name) {
            return Ok(vf);
        }
        let data = match self.finder.read(&format!("{name}.vf")) {
            Ok(data) => data,
            Err(TexError::NotFound { .. }) => return Ok(self.vfs.insert(name, None)),
            Err(err) => return Err(err),
        };
        let nested = LOADING_VF.with_borrow_mut(|loading| {
            if loading.contains(&name) || loading.len() >= MAX_VF_DEPTH {
                return Err(TexError::malformed(
                    TexFile::Vf,
                    format!("virtual font {name} refers to itself or nests too deeply"),
                ));
            }
            loading.push(name.clone());
            Ok(())
        });
        nested?;
        let vf = Vf::parse(&data, self);
        LOADING_VF.with_borrow_mut(Vec::pop);
        Ok(self.vfs.insert(name, Some(Arc::new(vf?))))
    }
}

/// Files held in memory, for tests.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct MemoryFinder(pub(crate) HashMap<String, Vec<u8>>);

#[cfg(test)]
impl FileFinder for MemoryFinder {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        if self.0.contains_key(name) {
            Ok(PathBuf::from(format!("/memory/{name}")))
        } else {
            Err(TexError::NotFound {
                name: name.to_owned(),
            })
        }
    }

    fn read(&self, name: &str) -> Result<Vec<u8>, TexError> {
        self.0.get(name).cloned().ok_or_else(|| TexError::NotFound {
            name: name.to_owned(),
        })
    }
}
