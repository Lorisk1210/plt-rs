//! `_native.testing`: comparing images for matplotlib's tests
//! (`testing.compare`).

use std::path::PathBuf;

use plt::testing::ComparisonError;

use crate::binding::lookup;
use crate::binding::prelude::*;

impl From<ComparisonError> for PyErr {
    /// matplotlib's exceptions: `OSError` for missing files, its
    /// `ImageComparisonFailure` (registered by `pltrs.testing.exceptions`)
    /// for images of different sizes, `ValueError` otherwise.
    fn from(err: ComparisonError) -> Self {
        let message = err.to_string();
        match err {
            ComparisonError::MissingOutput(_)
            | ComparisonError::MissingBaseline(_)
            | ComparisonError::Unwritable { .. } => PyErr::new(ExceptionKind::OSError, message),
            ComparisonError::Shape(_) => Python::with_gil(|py| {
                lookup(py, "ImageComparisonFailure")
                    .and_then(|class| class.call(&[message.as_str().into_py(py)?], &[]))
                    .map_or_else(|err| err, PyErr::from_instance)
            }),
            _ => PyErr::value_error(message),
        }
    }
}

/// Compares two PNG files: the RMS of their difference, and the
/// difference image written beside `actual` when it exceeds `tol`.
fn compare_images(expected: PathBuf, actual: PathBuf, tol: f64) -> PyResult<(f64, Option<String>)> {
    let comparison = plt::testing::compare_images(expected, actual, tol)?;
    let diff = comparison.diff.map(|path| path.display().to_string());
    Ok((comparison.rms, diff))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("compare_images", compare_images)
}
