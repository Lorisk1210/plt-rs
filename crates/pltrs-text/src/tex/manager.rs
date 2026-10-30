//! matplotlib's `TexManager`: TeX strings typeset by `latex` into DVI files
//! and rasterized by `dvipng`, both cached under the cache directory's
//! `tex.cache`, and the extents of the typeset text.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use pltrs_codec::sha256::sha256_hex;
use pltrs_core::pyfmt;

use super::command::{on_path, run_checked};
use super::dvi::DviPage;
use super::error::{TexError, TexFile};
use super::files::TexFiles;
use super::source::{TexFonts, tex_source};
use super::tempdir::TemporaryDirectory;
use crate::TextError;

/// How Agg-based backends draw TeX's output (`text.latex.engine`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TexEngine {
    /// Glyph by glyph from the DVI file, as every backend does (`latex`,
    /// matplotlib 3.11's default).
    #[default]
    Latex,
    /// As an image `dvipng` rasterizes (`latex+dvipng`).
    LatexDvipng,
}

pltrs_core::archive_enum!(TexEngine { Latex, LatexDvipng });

impl TexEngine {
    /// The engine's `text.latex.engine` name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Latex => "latex",
            Self::LatexDvipng => "latex+dvipng",
        }
    }
}

impl std::fmt::Display for TexEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for TexEngine {
    type Err = TextError;

    /// Parses the engine's name ([`TexEngine::name`]).
    fn from_str(name: &str) -> Result<Self, TextError> {
        [Self::Latex, Self::LatexDvipng]
            .into_iter()
            .find(|engine| engine.name() == name)
            .ok_or_else(|| TextError::unknown_name("TeX engine", name))
    }
}

/// The coverage of text `dvipng` rasterized, as matplotlib's Agg backend
/// blits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreyImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Coverage, row by row from the top.
    pub data: Vec<u8>,
}

/// What [`TexManager`] typesets with.
#[derive(Debug, PartialEq)]
struct Settings {
    cache_dir: PathBuf,
    fonts: TexFonts,
    preamble: String,
    engine: TexEngine,
}

/// Typesets TeX strings with `latex` (matplotlib's `TexManager`), with
/// the fonts and preamble of the rcParams it was made from. Cheap to
/// clone.
#[derive(Clone, Debug, PartialEq)]
pub struct TexManager(Arc<Settings>);

/// Makes the directory `path` and its parents.
fn make_dirs(path: &Path) -> Result<(), TexError> {
    std::fs::create_dir_all(path).map_err(|err| TexError::io(path, &err))
}

/// A fresh directory in `parent` for TeX's files.
fn temporary_directory(parent: &Path) -> Result<TemporaryDirectory, TexError> {
    TemporaryDirectory::new(parent).map_err(|err| TexError::io(parent, &err))
}

/// Moves `from` to `to` (`Path.replace`).
fn replace(from: &Path, to: &Path) -> Result<(), TexError> {
    std::fs::rename(from, to).map_err(|err| TexError::io(from, &err))
}

/// How many coverage images are kept before the cache starts afresh.
const GREY_CACHE_LIMIT: usize = 1024;

/// The coverage images read so far (`_grey_arrayd`), by document and
/// resolution.
type GreyCache = Mutex<HashMap<(String, u64), Arc<GreyImage>>>;

fn grey_cache() -> &'static GreyCache {
    static CACHE: OnceLock<GreyCache> = OnceLock::new();
    CACHE.get_or_init(GreyCache::default)
}

impl TexManager {
    /// A manager caching under `cache_dir/tex.cache`, typesetting with
    /// `fonts`, the user's `preamble` (`text.latex.preamble`) and
    /// rasterizing for Agg with `engine`.
    pub fn new(cache_dir: &Path, fonts: TexFonts, preamble: &str, engine: TexEngine) -> Self {
        Self(Arc::new(Settings {
            cache_dir: cache_dir.join("tex.cache"),
            fonts,
            preamble: preamble.to_owned(),
            engine,
        }))
    }

    /// Whether usetex can run here (matplotlib's `_checkdep_usetex`,
    /// without the Ghostscript the PostScript backend also needs): `tex`,
    /// `latex` and `dvipng` are installed.
    ///
    /// # Errors
    ///
    /// Why not, naming the missing program as matplotlib's warning does.
    pub fn check_installation() -> Result<(), TexError> {
        for (program, needed) in [("tex", "TeX"), ("latex", "LaTeX"), ("dvipng", "dvipng")] {
            if !on_path(program) {
                return Err(TexError::NotInstalled {
                    program: needed.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// The `tex.cache` directory.
    pub fn cache_dir(&self) -> &Path {
        &self.0.cache_dir
    }

    /// How Agg-based backends draw the output.
    pub fn engine(&self) -> TexEngine {
        self.0.engine
    }

    /// The fonts the text is typeset with.
    pub fn fonts(&self) -> &TexFonts {
        &self.0.fonts
    }

    /// The files of the texmf tree the DVI files refer to.
    pub fn files(&self) -> &'static TexFiles {
        TexFiles::kpathsea(self.0.cache_dir.parent().unwrap_or(&self.0.cache_dir))
    }

    /// The document typesetting `tex` at `fontsize` points
    /// (`_get_tex_source`).
    pub fn source(&self, tex: &str, fontsize: f64) -> String {
        tex_source(tex, fontsize, &self.0.fonts, &self.0.preamble)
    }

    /// `_get_base_path`: the cache path (without extension) of `tex` at
    /// `fontsize` and `dpi`, named after the SHA-256 of the document and
    /// the resolution, two levels of directories deep.
    ///
    /// # Errors
    ///
    /// When the directories cannot be made.
    pub fn base_path(
        &self,
        tex: &str,
        fontsize: f64,
        dpi: Option<f64>,
    ) -> Result<PathBuf, TexError> {
        let dpi = dpi.map_or_else(|| "None".to_owned(), pyfmt::repr);
        let hash = sha256_hex(format!("{}{dpi}", self.source(tex, fontsize)).as_bytes());
        let dir = self.0.cache_dir.join(&hash[0..2]).join(&hash[2..4]);
        make_dirs(&dir)?;
        Ok(dir.join(hash))
    }

    /// `make_tex`: writes the document to the cache; returns its path.
    ///
    /// # Errors
    ///
    /// When it cannot be written.
    pub fn make_tex(&self, tex: &str, fontsize: f64) -> Result<PathBuf, TexError> {
        let path = self.base_path(tex, fontsize, None)?.with_extension("tex");
        std::fs::write(&path, self.source(tex, fontsize))
            .map_err(|err| TexError::io(&path, &err))?;
        Ok(path)
    }

    /// `make_dvi`: typesets `tex` at `fontsize` with `latex` unless it is
    /// cached; returns the DVI file's path.
    ///
    /// # Errors
    ///
    /// When `latex` is missing or fails (with its output), or the cache
    /// cannot be written.
    pub fn make_dvi(&self, tex: &str, fontsize: f64) -> Result<PathBuf, TexError> {
        let dvi = self.base_path(tex, fontsize, None)?.with_extension("dvi");
        if dvi.exists() {
            return Ok(dvi);
        }
        let dir = dvi.parent().unwrap_or(&self.0.cache_dir);
        let tmp = temporary_directory(dir)?;
        let source = tmp.path().join("file.tex");
        std::fs::write(&source, self.source(tex, fontsize))
            .map_err(|err| TexError::io(&source, &err))?;
        let args = [
            "-interaction=nonstopmode",
            "-halt-on-error",
            "-no-shell-escape",
            "file.tex",
        ]
        .map(OsStr::new);
        run_checked("latex", &args, tex, tmp.path())?;
        replace(&tmp.path().join("file.dvi"), &dvi)?;
        replace(&source, &dvi.with_extension("tex"))?;
        Ok(dvi)
    }

    /// `make_png`: rasterizes `tex` at `fontsize` and `dpi` with
    /// `dvipng` unless it is cached; returns the image's path.
    ///
    /// # Errors
    ///
    /// As [`make_dvi`](Self::make_dvi), and when `dvipng` is missing or
    /// fails.
    pub fn make_png(&self, tex: &str, fontsize: f64, dpi: f64) -> Result<PathBuf, TexError> {
        let png = self
            .base_path(tex, fontsize, Some(dpi))?
            .with_extension("png");
        if png.exists() {
            return Ok(png);
        }
        let dvi = self.make_dvi(tex, fontsize)?;
        let tmp = temporary_directory(png.parent().unwrap_or(&self.0.cache_dir))?;
        let dpi = pyfmt::repr(dpi);
        let args = [
            OsStr::new("-bg"),
            OsStr::new("Transparent"),
            OsStr::new("-D"),
            OsStr::new(&dpi),
            OsStr::new("-T"),
            OsStr::new("tight"),
            OsStr::new("-o"),
            OsStr::new("file.png"),
            dvi.as_os_str(),
        ];
        run_checked("dvipng", &args, tex, tmp.path())?;
        replace(&tmp.path().join("file.png"), &png)?;
        Ok(png)
    }

    /// `get_grey`, as the Agg backend uses it: the alpha channel of the
    /// image `dvipng` makes of `tex` at `fontsize` and `dpi`, read as
    /// matplotlib reads it (`imread`, a float in `[0, 1]`) and turned back
    /// into bytes (`(Z * 0xff).astype(np.uint8)`). Kept for the process.
    ///
    /// # Errors
    ///
    /// As [`make_png`](Self::make_png), and when the image is malformed.
    pub fn grey(&self, tex: &str, fontsize: f64, dpi: f64) -> Result<Arc<GreyImage>, TexError> {
        let key = (self.source(tex, fontsize), dpi.to_bits());
        let cache = grey_cache();
        if let Some(grey) = cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
        {
            return Ok(Arc::clone(grey));
        }
        let path = self.make_png(tex, fontsize, dpi)?;
        let data = std::fs::read(&path).map_err(|err| TexError::io(&path, &err))?;
        let image = pltrs_codec::png::decode(&data)
            .map_err(|err| TexError::malformed(TexFile::Png, err.to_string()))?;
        let rgba = image.to_rgba8();
        let data = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[_, _, _, alpha]| {
                let alpha = f32::from(alpha) / 255.0;
                (alpha * 255.0) as u8
            })
            .collect();
        let grey = Arc::new(GreyImage {
            width: image.width(),
            height: image.height(),
            data,
        });
        let mut images = cache.lock().unwrap_or_else(PoisonError::into_inner);
        if images.len() >= GREY_CACHE_LIMIT {
            images.clear();
        }
        images.insert(key, Arc::clone(&grey));
        Ok(grey)
    }

    /// The page `latex` typesets `tex` on at `fontsize`, in pixels at
    /// `dpi` (`Dvi(make_dvi(tex, fontsize), dpi)` and its only page).
    ///
    /// # Errors
    ///
    /// As [`make_dvi`](Self::make_dvi), when the DVI file is malformed or
    /// uses a font that cannot be found, or when it does not hold exactly
    /// one page.
    pub fn page(&self, tex: &str, fontsize: f64, dpi: f64) -> Result<DviPage, TexError> {
        let dvi = self.make_dvi(tex, fontsize)?;
        let pages = self.files().dvi_pages(&dvi)?;
        match pages.as_slice() {
            [page] => page.at(dpi),
            pages => Err(TexError::malformed(
                TexFile::Dvi,
                format!("expected 1 page, found {}", pages.len()),
            )),
        }
    }

    /// `get_text_width_height_descent`: the width, height (descent
    /// included) and descent of `tex` at `fontsize` points, in pixels of a
    /// renderer whose point is `pixels_per_point` pixels (all zero for
    /// blank text).
    ///
    /// # Errors
    ///
    /// As [`page`](Self::page).
    pub fn text_width_height_descent(
        &self,
        tex: &str,
        fontsize: f64,
        pixels_per_point: f64,
    ) -> Result<(f64, f64, f64), TexError> {
        if tex.trim().is_empty() {
            return Ok((0.0, 0.0, 0.0));
        }
        let page = self.page(tex, fontsize, 72.0 * pixels_per_point)?;
        Ok((page.width, page.height + page.descent, page.descent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager(dir: &Path) -> TexManager {
        let fonts = TexFonts {
            family: vec!["serif".into()],
            ..TexFonts::default()
        };
        TexManager::new(dir, fonts, "", TexEngine::Latex)
    }

    #[test]
    fn cache_paths_hash_the_document_and_resolution() {
        let dir = std::env::temp_dir().join(format!("pltrs-texmanager-{}", std::process::id()));
        let tex = manager(&dir);
        let base = tex.base_path("x", 10.0, None).unwrap();
        let hash = sha256_hex(format!("{}None", tex.source("x", 10.0)).as_bytes());
        assert_eq!(
            base,
            dir.join("tex.cache")
                .join(&hash[..2])
                .join(&hash[2..4])
                .join(&hash)
        );
        assert!(base.parent().unwrap().is_dir());
        let png = tex.base_path("x", 10.0, Some(100.0)).unwrap();
        let hash = sha256_hex(format!("{}100.0", tex.source("x", 10.0)).as_bytes());
        assert!(png.ends_with(&hash));
        let written = tex.make_tex("x", 10.0).unwrap();
        assert_eq!(
            std::fs::read_to_string(written).unwrap(),
            tex.source("x", 10.0)
        );
        assert_eq!(
            tex.text_width_height_descent("  ", 10.0, 1.0),
            Ok((0.0, 0.0, 0.0))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn engines_parse_from_the_rc_values() {
        assert_eq!("latex".parse(), Ok(TexEngine::Latex));
        assert_eq!("latex+dvipng".parse(), Ok(TexEngine::LatexDvipng));
        assert_eq!(
            "dvipng".parse::<TexEngine>(),
            Err(TextError::unknown_name("TeX engine", "dvipng"))
        );
        assert_eq!(TexEngine::LatexDvipng.to_string(), "latex+dvipng");
    }
}
