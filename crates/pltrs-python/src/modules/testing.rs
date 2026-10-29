//! `_native.testing`: comparing images for matplotlib's tests
//! (`testing.compare`).

use std::path::PathBuf;

use pltrs::testing::ComparisonError;

use crate::binding::prelude::*;
use crate::binding::registered_error;

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
            ComparisonError::Shape(_) => registered_error("ImageComparisonFailure", &message),
            _ => PyErr::value_error(message),
        }
    }
}

/// Compares two PNG files: the RMS of their difference, and the
/// difference image written beside `actual` when it exceeds `tol`.
fn compare_images(expected: PathBuf, actual: PathBuf, tol: f64) -> PyResult<(f64, Option<String>)> {
    let comparison = pltrs::testing::compare_images(expected, actual, tol)?;
    let diff = comparison.diff.map(|path| path.display().to_string());
    Ok((comparison.rms, diff))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("compare_images", compare_images)
}
