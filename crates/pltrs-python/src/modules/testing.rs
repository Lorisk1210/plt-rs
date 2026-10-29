//! `_native.testing`: comparing images for matplotlib's tests (the
//! `calculate_rms_and_diff` of matplotlib's native `_image`, which
//! `testing.compare` uses), and the bundled fonts its SVG converter hands
//! Inkscape.

use std::path::PathBuf;

use pltrs::numeric::Grid;
use pltrs::testing::{ImageShapeError, calculate_rms_and_diff};

use crate::binding::prelude::*;
use crate::binding::registered_error;

/// matplotlib's `ImageComparisonFailure` (registered by
/// `pltrs.testing.exceptions`).
fn comparison_failure(message: &str) -> PyErr {
    registered_error("ImageComparisonFailure", message)
}

impl From<ImageShapeError> for PyErr {
    fn from(err: ImageShapeError) -> Self {
        comparison_failure(&err.to_string())
    }
}

/// The pixels of an image array of shape `(rows, cols, N)`.
fn pixels<const N: usize>(image: &NdArray<u8>) -> PyResult<Grid<[u8; N]>> {
    let cells = image.data.as_chunks::<N>().0.to_vec();
    Grid::from_vec(image.shape[0], image.shape[1], cells)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The RMS of the difference of two images of shape `(rows, cols, 3|4)`
/// and the difference image, ten times as strong (matplotlib's
/// `calculate_rms_and_diff`).
fn rms_and_diff(expected: NdArray<u8>, actual: NdArray<u8>) -> PyResult<(f64, NdArray<u8>)> {
    for (image, name) in [(&expected, "Expected"), (&actual, "Actual")] {
        if image.shape.len() != 3 {
            return Err(comparison_failure(&format!(
                "{name} image must be 3-dimensional, but is {}-dimensional",
                image.shape.len()
            )));
        }
    }
    let depth = expected.shape[2];
    if depth != 3 && depth != 4 {
        return Err(comparison_failure(&format!(
            "Image must be RGB or RGBA but has depth {depth}"
        )));
    }
    if expected.shape != actual.shape {
        let shape = |image: &NdArray<u8>| (image.shape[0], image.shape[1], image.shape[2]);
        return Err(ImageShapeError {
            expected: shape(&expected),
            actual: shape(&actual),
        }
        .into());
    }
    let (rms, diff) = if depth == 3 {
        calculate_rms_and_diff(&pixels::<3>(&expected)?, &pixels::<3>(&actual)?)?
    } else {
        calculate_rms_and_diff(&pixels::<4>(&expected)?, &pixels::<4>(&actual)?)?
    };
    let shape = vec![diff.rows(), diff.cols(), 3];
    Ok((rms, NdArray::with_shape(diff.into_vec().concat(), shape)?))
}

/// Writes the font files pltrs bundles (matplotlib's `mpl-data/fonts/ttf`)
/// into the directory `dir`, made if missing, for Inkscape to find them.
fn write_bundled_fonts(dir: PathBuf) -> PyResult<()> {
    let os_error = |err: std::io::Error| PyErr::new(ExceptionKind::OSError, err.to_string());
    std::fs::create_dir_all(&dir).map_err(os_error)?;
    for (name, data) in pltrs_font::bundled_font_files() {
        std::fs::write(dir.join(name), data).map_err(os_error)?;
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("calculate_rms_and_diff", rms_and_diff)?;
    module.add("write_bundled_fonts", write_bundled_fonts)
}
