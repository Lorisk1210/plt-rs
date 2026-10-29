//! `_native.dviread`: TeX's files, as usetex finds and reads them
//! (matplotlib's `dviread`), through plt-text's kpathsea and DVI reader.

use plt_text::tex::{TexError, TexFiles};

use crate::binding::prelude::*;

impl From<TexError> for PyErr {
    /// `FileNotFoundError` with matplotlib's message for a file kpathsea
    /// cannot find, `OSError` for one that cannot be read, `ValueError`
    /// for malformed data, `RuntimeError` otherwise.
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
            other => PyErr::runtime_error(other.to_string()),
        }
    }
}

/// The files of the TeX installation, found through kpathsea.
fn tex_files() -> PyResult<&'static TexFiles> {
    let cache = plt::config::cache_dir().map_err(|err| PyErr::runtime_error(err.to_string()))?;
    Ok(TexFiles::kpathsea(&cache))
}

/// `find_tex_file(filename)`: the path kpathsea finds for a TeX file.
fn find_tex_file(filename: String) -> PyResult<String> {
    Ok(tex_files()?.find(&filename)?.display().to_string())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("find_tex_file", find_tex_file)
}
