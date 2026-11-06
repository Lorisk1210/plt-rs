//! `_native.dviread`: TeX's files, as usetex finds and reads them
//! (matplotlib's `dviread`), through pltrs-text's kpathsea, font maps and
//! DVI reader.

mod fonts;
mod pages;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use pltrs_font::Type1Effects;
use pltrs_text::tex::{
    FileFinder, FontMapLookup, PsFont, PsfontsMap, TexError, TexFiles, set_font_map_lookup,
};

use crate::binding::new_dict;
use crate::binding::prelude::*;

impl From<TexError> for PyErr {
    /// `FileNotFoundError` with matplotlib's message for a file kpathsea
    /// cannot find, `OSError` for one that cannot be read, `ValueError`
    /// for malformed data (with matplotlib's message, which does not name
    /// the kind of file) and for a font without a Type 1 file,
    /// `LookupError` for a font a font map lacks, `RuntimeError`
    /// otherwise.
    fn from(err: TexError) -> Self {
        match err {
            TexError::NotFound { name } => PyErr::new(
                ExceptionKind::FileNotFoundError,
                format!(
                    "Matplotlib's TeX implementation searched for a file named {} in your \
                     texmf tree, but could not find it",
                    pltrs_core::pyfmt::str_repr(&name)
                ),
            ),
            TexError::Io { .. } => PyErr::new(ExceptionKind::OSError, err.to_string()),
            TexError::Malformed { message, .. } => PyErr::value_error(message),
            TexError::NoType1Font { .. } => PyErr::value_error(err.to_string()),
            TexError::UnmappedFont { .. } => {
                PyErr::new(ExceptionKind::LookupError, err.to_string())
            }
            other => PyErr::runtime_error(other.to_string()),
        }
    }
}

impl IntoPy for PsfontsMap {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// The files of the TeX installation, found through kpathsea.
pub(super) fn tex_files() -> PyResult<&'static TexFiles> {
    let cache = pltrs::config::cache_dir().map_err(|err| PyErr::runtime_error(err.to_string()))?;
    Ok(TexFiles::kpathsea(&cache))
}

/// `find_tex_file(filename)`: the path kpathsea finds for a TeX file.
fn find_tex_file(filename: String) -> PyResult<String> {
    Ok(tex_files()?.find(&filename)?.display().to_string())
}

/// The module's `find_tex_file` as font maps call it: with the name as
/// bytes, as matplotlib does, so that a replacement installed by a test
/// is the one called.
#[derive(Debug)]
struct PythonFinder(Callback);

impl FileFinder for PythonFinder {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        self.0
            .call::<PathBuf>((Bytes(name.as_bytes().to_vec()),))
            .ok_or_else(|| TexError::NotFound {
                name: name.to_owned(),
            })
    }
}

/// A path as matplotlib's font map entries hold it: a `str`.
fn path_text(path: Option<&Path>) -> Option<String> {
    path.map(|path| path.to_string_lossy().into_owned())
}

/// `PsfontsMap(filename)`: the font map in `data`, the contents of the
/// file called `name`, its lines parsed on demand.
fn psfonts_map(data: Bytes, name: String) -> PyResult<PsfontsMap> {
    Ok(PsfontsMap::parse(&data.0, &name))
}

/// matplotlib's effects dict of a Type 1 font: `slant` and `extend`, as
/// far as the font map gives them.
fn type1_effects(effects: Type1Effects) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let dict = new_dict(py)?;
        if let Some(slant) = effects.slant {
            dict.set_item("slant", slant)?;
        }
        if let Some(extend) = effects.extend {
            dict.set_item("extend", extend)?;
        }
        Ok(dict.unbind())
    })
}

/// `PsfontsMap[texname]`: the TeX name, PostScript name, effects (a dict
/// of `slant` and `extend`), encoding file and font file of a TeX font,
/// the files found with `find_tex_file`.
#[expect(
    clippy::type_complexity,
    reason = "the fields of matplotlib's PsFont tuple"
)]
fn psfonts_map_get(
    map: Native<PsfontsMap>,
    texname: Bytes,
    find_tex_file: Callback,
) -> PyResult<(Bytes, Bytes, Owned, Option<String>, Option<String>)> {
    let font = map.borrow().get(&texname.0, &PythonFinder(find_tex_file))?;
    Ok((
        Bytes(font.texname),
        Bytes(font.psname),
        type1_effects(font.effects)?,
        path_text(font.encoding.as_deref()),
        path_text(font.filename.as_deref()),
    ))
}

/// `_parse_enc(path)`: the glyph names of the PostScript encoding file
/// whose contents are `data`.
fn parse_enc(data: Bytes) -> PyResult<Vec<String>> {
    Ok(pltrs_text::tex::parse_enc(&data.0)?)
}

/// The fields of matplotlib's `PsFont` tuple as a replaced
/// `PsfontsMap.__getitem__` returns them: the effects may be None.
type PsFontFields = (
    Bytes,
    Bytes,
    Option<BTreeMap<String, f64>>,
    Option<PathBuf>,
    Option<PathBuf>,
);

/// Fonts looked up by the program's `PsfontsMap.__getitem__`:
/// `lookup(texname)` gives the fields of a `PsFont`.
struct PythonLookup(Callback);

impl FontMapLookup for PythonLookup {
    fn lookup(&self, texname: &[u8]) -> Result<PsFont, TexError> {
        let unmapped = || TexError::UnmappedFont {
            map: "pdftex.map".to_owned(),
            texname: String::from_utf8_lossy(texname).into_owned(),
        };
        let (texname, psname, effects, encoding, filename) = self
            .0
            .call::<PsFontFields>((Bytes(texname.to_vec()),))
            .ok_or_else(unmapped)?;
        let effects = effects.unwrap_or_default();
        Ok(PsFont {
            texname: texname.0,
            psname: psname.0,
            effects: Type1Effects {
                slant: effects.get("slant").copied(),
                extend: effects.get("extend").copied(),
            },
            encoding,
            filename,
        })
    }
}

/// Looks every TeX font usetex draws on this thread up with `lookup` (a
/// callable giving the fields of matplotlib's `PsFont` for a TeX name),
/// or in `pdftex.map` again with None.
fn set_font_lookup(lookup: Option<Callback>) -> PyResult<()> {
    set_font_map_lookup(
        lookup.map(|lookup| Rc::new(PythonLookup(lookup)) as Rc<dyn FontMapLookup>),
    );
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("find_tex_file", find_tex_file)?;
    module.add("psfonts_map", psfonts_map)?;
    module.add("psfonts_map_get", psfonts_map_get)?;
    module.add("parse_enc", parse_enc)?;
    module.add("set_font_lookup", set_font_lookup)?;
    fonts::register(module)?;
    pages::register(module)
}
