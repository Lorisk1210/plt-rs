//! The images comparisons work on (matplotlib's `_load_image`,
//! `crop_to_same` and `save_diff_image`): PNG files loaded as RGB or RGBA
//! pixels, cropped alike when an EPS image meets a PDF one, and the
//! difference image of two of them.

use std::fs;
use std::ops::Range;
use std::path::Path;

use pltrs_codec::png::{self, ColorType, ImageRef};

use super::compare::{ComparisonError, ImageShapeError};
use crate::numeric::Grid;

/// An image file as the comparison loads it (matplotlib's `_load_image`):
/// RGB images as RGB, others as RGBA unless every pixel is opaque.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Loaded {
    /// Opaque pixels.
    Rgb(Grid<[u8; 3]>),
    /// Pixels with transparency.
    Rgba(Grid<[u8; 4]>),
}

impl Loaded {
    /// Reads the PNG file at `path`.
    pub(super) fn read(path: &Path) -> Result<Self, ComparisonError> {
        let unreadable = |reason: String| ComparisonError::Unreadable {
            path: path.to_owned(),
            reason,
        };
        let bytes = fs::read(path).map_err(|err| unreadable(err.to_string()))?;
        let image = png::decode(&bytes).map_err(|err| unreadable(err.to_string()))?;
        let (rows, cols) = (image.height() as usize, image.width() as usize);
        let pixels = image.to_rgba8().as_chunks::<4>().0.to_vec();
        let opaque = pixels.iter().all(|pixel| pixel[3] == u8::MAX);
        Ok(if image.color_type() == ColorType::Rgb || opaque {
            let colors = pixels.iter().map(|&[r, g, b, _]| [r, g, b]).collect();
            Self::Rgb(filled_grid(rows, cols, colors))
        } else {
            Self::Rgba(filled_grid(rows, cols, pixels))
        })
    }

    /// (rows, columns, channels).
    pub(super) fn shape(&self) -> (usize, usize, usize) {
        match self {
            Self::Rgb(grid) => (grid.rows(), grid.cols(), 3),
            Self::Rgba(grid) => (grid.rows(), grid.cols(), 4),
        }
    }

    /// The middle `rows × cols` of the image (see [`crop_to_same`]).
    fn cropped(self, rows: usize, cols: usize) -> Self {
        match self {
            Self::Rgb(grid) => Self::Rgb(centred(&grid, rows, cols)),
            Self::Rgba(grid) => Self::Rgba(centred(&grid, rows, cols)),
        }
    }
}

/// The grid of `rows × cols` decoded pixels.
pub(super) fn filled_grid<T>(rows: usize, cols: usize, pixels: Vec<T>) -> Grid<T> {
    Grid::from_vec(rows, cols, pixels).expect("a decoded image has a pixel per cell")
}

/// Whether `path`, a converted image, was made from a file with the
/// extension `extension` (`plot_eps.png` from `plot.eps`): matplotlib's
/// `path[-7:-4] == extension`.
fn converted_from(path: &Path, extension: &str) -> bool {
    let name = path.as_os_str().as_encoded_bytes();
    name.len() >= 7 && &name[name.len() - 7..name.len() - 4] == extension.as_bytes()
}

/// The indices of `start..stop` of a sequence of `len` items, as a Python
/// slice takes them (negative bounds count from the end).
fn python_slice(start: isize, stop: isize, len: usize) -> Range<usize> {
    let len = isize::try_from(len).unwrap_or(isize::MAX);
    let bound = |index: isize| {
        let index = if index < 0 { index + len } else { index };
        usize::try_from(index.clamp(0, len)).unwrap_or(0)
    };
    let (start, stop) = (bound(start), bound(stop));
    start..stop.max(start)
}

/// The middle `rows × cols` of `grid`, as matplotlib slices it: rows
/// `int(r/2 - rows/2):int(r/2 + rows/2)`, columns alike.
fn centred<T: Clone>(grid: &Grid<T>, rows: usize, cols: usize) -> Grid<T> {
    let range = |len: usize, wanted: usize| {
        let (half, wanted) = (len as f64 / 2.0, wanted as f64 / 2.0);
        python_slice((half - wanted) as isize, (half + wanted) as isize, len)
    };
    let (row_range, col_range) = (range(grid.rows(), rows), range(grid.cols(), cols));
    Grid::from_fn(row_range.len(), col_range.len(), |r, c| {
        grid.get(row_range.start + r, col_range.start + c)
            .expect("the crop lies inside the grid")
            .clone()
    })
}

/// The actual and expected images, the actual one cropped to the size of
/// the expected one when it was converted from an EPS file and the
/// expected one from a PDF file (matplotlib's `crop_to_same`, which only
/// this comparison needs: an EPS page is cropped to its bounding box).
/// The paths are those of the converted images (`plot_eps.png`).
///
/// ```
/// use std::path::Path;
/// use pltrs::numeric::Grid;
/// use pltrs::testing::crop_to_same;
/// let actual = Grid::from_fn(4, 4, |r, c| r * 4 + c);
/// let expected = Grid::from_fn(2, 2, |_, _| 0);
/// let (cropped, _) = crop_to_same(Path::new("a_eps.png"), actual.clone(), Path::new("b_pdf.png"), expected.clone());
/// assert_eq!(cropped.as_slice(), [5, 6, 9, 10]);
/// let (kept, _) = crop_to_same(Path::new("a_png.png"), actual.clone(), Path::new("b_pdf.png"), expected);
/// assert_eq!(kept, actual);
/// ```
pub fn crop_to_same<T: Clone>(
    actual_path: &Path,
    actual: Grid<T>,
    expected_path: &Path,
    expected: Grid<T>,
) -> (Grid<T>, Grid<T>) {
    if converted_from(actual_path, "eps") && converted_from(expected_path, "pdf") {
        let cropped = centred(&actual, expected.rows(), expected.cols());
        (cropped, expected)
    } else {
        (actual, expected)
    }
}

/// [`crop_to_same`] of loaded images.
pub(super) fn crop_loaded(
    actual_path: &Path,
    actual: Loaded,
    expected_path: &Path,
    expected: Loaded,
) -> (Loaded, Loaded) {
    if converted_from(actual_path, "eps") && converted_from(expected_path, "pdf") {
        let (rows, cols, _) = expected.shape();
        (actual.cropped(rows, cols), expected)
    } else {
        (actual, expected)
    }
}

/// The difference of two images of the same shape, ten times as strong
/// and clipped, with alpha (when there is one) opaque.
fn amplified<const N: usize>(
    expected: &Grid<[u8; N]>,
    actual: &Grid<[u8; N]>,
) -> Result<Grid<[u8; N]>, ImageShapeError> {
    if expected.shape() != actual.shape() {
        let shape = |grid: &Grid<[u8; N]>| (grid.rows(), grid.cols(), N);
        return Err(ImageShapeError {
            expected: shape(expected),
            actual: shape(actual),
        });
    }
    let pixels = expected
        .as_slice()
        .iter()
        .zip(actual.as_slice())
        .map(|(e, a)| {
            let mut difference = [0; N];
            for (channel, (e, a)) in difference.iter_mut().zip(e.iter().zip(a)) {
                *channel = e.abs_diff(*a).saturating_mul(10);
            }
            if N == 4 {
                difference[3] = u8::MAX;
            }
            difference
        })
        .collect();
    Ok(filled_grid(expected.rows(), expected.cols(), pixels))
}

/// Writes the difference of the PNG images `expected` and `actual` to
/// `output` as a PNG image (matplotlib's `save_diff_image`): ten times the
/// difference of each channel, clipped to 255, alpha opaque; the actual
/// image is cropped first as [`crop_to_same`] says.
///
/// ```
/// use pltrs::prelude::*;
/// use pltrs::testing::save_diff_image;
/// let dir = std::env::temp_dir().join(format!("pltrs-save-diff-doc-{}", std::process::id()));
/// std::fs::create_dir_all(&dir)?;
/// let fig = Figure::with_size(1.0, 1.0, 20.0);
/// fig.savefig(dir.join("empty.png"))?;
/// fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
/// fig.savefig(dir.join("line.png"))?;
/// save_diff_image(dir.join("empty.png"), dir.join("line.png"), dir.join("diff.png"))?;
/// assert!(dir.join("diff.png").exists());
/// std::fs::remove_dir_all(&dir)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When an image cannot be read as a PNG, the images differ in shape, or
/// `output` cannot be written.
pub fn save_diff_image(
    expected: impl AsRef<Path>,
    actual: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<(), ComparisonError> {
    let (expected_path, actual_path) = (expected.as_ref(), actual.as_ref());
    let (actual, expected) = crop_loaded(
        actual_path,
        Loaded::read(actual_path)?,
        expected_path,
        Loaded::read(expected_path)?,
    );
    let output = output.as_ref();
    match (&expected, &actual) {
        (Loaded::Rgb(e), Loaded::Rgb(a)) => write_png(output, &amplified(e, a)?),
        (Loaded::Rgba(e), Loaded::Rgba(a)) => write_png(output, &amplified(e, a)?),
        _ => Err(ImageShapeError {
            expected: expected.shape(),
            actual: actual.shape(),
        }
        .into()),
    }
}

/// Writes `image` to `path` as an RGB or RGBA PNG.
pub(super) fn write_png<const N: usize>(
    path: &Path,
    image: &Grid<[u8; N]>,
) -> Result<(), ComparisonError> {
    let unwritable = |reason: String| ComparisonError::Unwritable {
        path: path.to_owned(),
        reason,
    };
    let side = |n: usize| u32::try_from(n).map_err(|err| unwritable(err.to_string()));
    let (width, height) = (side(image.cols())?, side(image.rows())?);
    let data = image.as_slice().as_flattened();
    let png = if N == 4 {
        ImageRef::rgba8(width, height, data)
    } else {
        ImageRef::rgb8(width, height, data)
    }
    .and_then(png::encode)
    .map_err(|err| unwritable(err.to_string()))?;
    fs::write(path, png).map_err(|err| unwritable(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    fn write_rgba(path: &Path, width: u32, pixels: &[[u8; 4]]) {
        let height = (pixels.len() as u32) / width;
        let png = png::encode(ImageRef::rgba8(width, height, pixels.as_flattened()).unwrap());
        fs::write(path, png.unwrap()).unwrap();
    }

    #[test]
    fn slices_follow_python() {
        assert_eq!(python_slice(1, 3, 5), 1..3);
        assert_eq!(python_slice(-2, 5, 5), 3..5);
        assert_eq!(python_slice(0, 9, 5), 0..5);
        assert_eq!(python_slice(4, 2, 5), 4..4);
    }

    #[test]
    fn only_eps_against_pdf_is_cropped() {
        let actual = Grid::from_fn(5, 6, |r, c| (r, c));
        let expected = Grid::from_fn(3, 2, |_, _| (0, 0));
        let (cropped, _) = crop_to_same(
            Path::new("dir/x_eps.png"),
            actual.clone(),
            Path::new("dir/x_pdf.png"),
            expected.clone(),
        );
        // int(2.5 - 1.5) = 1 to int(2.5 + 1.5) = 4; int(3 - 1) = 2 to 4.
        assert_eq!(cropped.shape(), (3, 2));
        assert_eq!(cropped.get(0, 0), Some(&(1, 2)));
        let (kept, _) = crop_to_same(
            Path::new("x_pdf.png"),
            actual.clone(),
            Path::new("x_eps.png"),
            expected,
        );
        assert_eq!(kept, actual);
    }

    #[test]
    fn diff_images_amplify_and_keep_alpha_opaque() {
        let scratch = Scratch::new("save-diff");
        let dir = scratch.path("");
        let (expected, actual) = (dir.join("e.png"), dir.join("a.png"));
        write_rgba(&expected, 2, &[[10, 20, 30, 100], [0, 0, 0, 255]]);
        write_rgba(&actual, 2, &[[16, 20, 30, 50], [0, 0, 100, 255]]);
        let output = dir.join("diff.png");
        save_diff_image(&expected, &actual, &output).unwrap();
        let diff = png::decode(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(diff.color_type(), ColorType::Rgba);
        assert_eq!(diff.to_rgba8(), [60, 0, 0, 255, 0, 0, 255, 255]);
        let opaque = dir.join("o.png");
        write_rgba(&opaque, 2, &[[0, 0, 0, 255]; 2]);
        let mismatch = save_diff_image(&opaque, &actual, &output).unwrap_err();
        assert!(matches!(mismatch, ComparisonError::Shape(_)));
    }
}
