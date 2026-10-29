//! `_native.dviread`: TeX's files, as usetex finds and reads them
//! (matplotlib's `dviread`), through pltrs-text's kpathsea, font maps and
//! DVI reader.

use std::path::{Path, PathBuf};

use pltrs_text::tex::{FileFinder, PsfontsMap, TexError, TexFiles};

use crate::binding::new_dict;
use crate::binding::prelude::*;

impl From<TexError> for PyErr {
    /// `FileNotFoundError` with matplotlib's message for a file kpathsea
    /// cannot find, `OSError` for one that cannot be read, `ValueError`
    /// for malformed data, `LookupError` for a font a font map lacks,
    /// `RuntimeError` otherwise.
    fn from(err: TexError) -> Self {
        match err {
            TexError::NotFound { name } => PyErr::new(
                ExceptionKind::FileNotFoundError,
                format!(
                    "Matplotlib's TeX implementation searched for a file named {name:?} in \
                     your texmf tree, but could not find it"
                ),
            ),
            TexError::Io { .. } => PyErr::new(ExceptionKind::OSError, err.to_string()),
            TexError::Malformed { .. } => PyErr::value_error(err.to_string()),
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
fn tex_files() -> PyResult<&'static TexFiles> {
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
    let effects = Python::with_gil(|py| -> PyResult<Owned> {
        let effects = new_dict(py)?;
        if let Some(slant) = font.effects.slant {
            effects.set_item("slant", slant)?;
        }
        if let Some(extend) = font.effects.extend {
            effects.set_item("extend", extend)?;
        }
        Ok(effects.unbind())
    })?;
    Ok((
        Bytes(font.texname),
        Bytes(font.psname),
        effects,
        path_text(font.encoding.as_deref()),
        path_text(font.filename.as_deref()),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("find_tex_file", find_tex_file)?;
    module.add("psfonts_map", psfonts_map)?;
    module.add("psfonts_map_get", psfonts_map_get)
}
