//! Comparing images (matplotlib's `testing.compare`): the RMS metric, and
//! `compare_images` with the difference image it writes on failure.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use super::convert::convert;
use super::images::{Loaded, crop_loaded, filled_grid, write_png};
use crate::figure::SaveError;
use crate::numeric::Grid;

/// Two images of different shapes, each given as (rows, columns,
/// channels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageShapeError {
    /// The shape of the expected image.
    pub expected: (usize, usize, usize),
    /// The shape of the actual image.
    pub actual: (usize, usize, usize),
}

impl fmt::Display for ImageShapeError {
    /// matplotlib's message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Image sizes do not match expected size: {:?} actual size {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for ImageShapeError {}

/// The root mean square of the differences between the channel values of
/// two images of the same shape (matplotlib's `calculate_rms`): 0 for
/// equal images, up to 255 for 8-bit channels; NaN for empty images.
/// The images are grids of pixels of `N` channels, such as the RGB and
/// RGBA bytes of [`ImageData`](crate::image::ImageData).
///
/// ```
/// use pltrs::numeric::Grid;
/// use pltrs::testing::calculate_rms;
/// let expected = Grid::from_vec(1, 2, vec![[10_u8, 20, 30], [0, 0, 0]])?;
/// let actual = Grid::from_vec(1, 2, vec![[16_u8, 20, 30], [0, 0, 0]])?;
/// // One channel value of six is off by 6.
/// assert_eq!(calculate_rms(&expected, &actual)?, 6.0_f64.sqrt());
/// let column = Grid::from_vec(2, 1, vec![[0_u8; 3]; 2])?;
/// let err = calculate_rms(&expected, &column).unwrap_err();
/// assert_eq!(
///     err.to_string(),
///     "Image sizes do not match expected size: (1, 2, 3) actual size (2, 1, 3)"
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`ImageShapeError`] when the images differ in size.
pub fn calculate_rms<T, const N: usize>(
    expected: &Grid<[T; N]>,
    actual: &Grid<[T; N]>,
) -> Result<f64, ImageShapeError>
where
    T: Copy + Into<f64>,
{
    if expected.shape() != actual.shape() {
        let shape = |grid: &Grid<[T; N]>| (grid.rows(), grid.cols(), N);
        return Err(ImageShapeError {
            expected: shape(expected),
            actual: shape(actual),
        });
    }
    let sum: f64 = expected
        .as_slice()
        .iter()
        .zip(actual.as_slice())
        .flat_map(|(e, a)| e.iter().zip(a))
        .map(|(&e, &a)| {
            let difference = e.into() - a.into();
            difference * difference
        })
        .sum();
    Ok((sum / (expected.as_slice().len() * N) as f64).sqrt())
}

/// The RMS of two loaded images and their difference image (matplotlib's
/// `calculate_rms_and_diff`).
fn rms_and_diff(
    expected: &Loaded,
    actual: &Loaded,
) -> Result<(f64, Grid<[u8; 3]>), ImageShapeError> {
    match (expected, actual) {
        (Loaded::Rgb(expected), Loaded::Rgb(actual)) => calculate_rms_and_diff(expected, actual),
        (Loaded::Rgba(expected), Loaded::Rgba(actual)) => calculate_rms_and_diff(expected, actual),
        _ => Err(ImageShapeError {
            expected: expected.shape(),
            actual: actual.shape(),
        }),
    }
}

/// The RMS of two images and their difference image (matplotlib's
/// `_image.calculate_rms_and_diff`): the colour channels of each pixel
/// differ ten times as much, clipped to 255, and alpha is left out.
///
/// ```
/// use pltrs::numeric::Grid;
/// use pltrs::testing::calculate_rms_and_diff;
/// let expected = Grid::from_vec(1, 1, vec![[10_u8, 20, 30, 255]])?;
/// let actual = Grid::from_vec(1, 1, vec![[10_u8, 50, 30, 255]])?;
/// let (rms, diff) = calculate_rms_and_diff(&expected, &actual)?;
/// assert_eq!(rms, 15.0);
/// assert_eq!(diff.as_slice(), [[0, 255, 0]]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// [`ImageShapeError`] when the images differ in size.
pub fn calculate_rms_and_diff<const N: usize>(
    expected: &Grid<[u8; N]>,
    actual: &Grid<[u8; N]>,
) -> Result<(f64, Grid<[u8; 3]>), ImageShapeError> {
    let rms = calculate_rms(expected, actual)?;
    let pixels = expected
        .as_slice()
        .iter()
        .zip(actual.as_slice())
        .map(|(e, a)| {
            let mut difference = [0; 3];
            for (channel, (e, a)) in difference.iter_mut().zip(e.iter().zip(a)) {
                *channel = e.abs_diff(*a).saturating_mul(10);
            }
            difference
        })
        .collect();
    Ok((rms, filled_grid(expected.rows(), expected.cols(), pixels)))
}

/// `path` with `-purpose` before its extension (matplotlib's
/// `make_test_filename`).
///
/// ```
/// use std::path::Path;
/// use pltrs::testing::make_test_filename;
/// assert_eq!(make_test_filename("out/plot.png", "expected"), Path::new("out/plot-expected.png"));
/// ```
pub fn make_test_filename(path: impl AsRef<Path>, purpose: &str) -> PathBuf {
    let path = path.as_ref();
    let mut name = path.file_stem().unwrap_or_default().to_owned();
    name.push("-");
    name.push(purpose);
    if let Some(extension) = path.extension() {
        name.push(".");
        name.push(extension);
    }
    path.with_file_name(name)
}

/// How an output image compares with its baseline image.
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// The RMS of the channel differences ([`calculate_rms`]).
    pub rms: f64,
    /// The largest RMS accepted.
    pub tol: f64,
    /// The baseline image.
    pub expected: PathBuf,
    /// The output image.
    pub actual: PathBuf,
    /// The difference image written beside the output image, `None`
    /// exactly when the images match.
    pub diff: Option<PathBuf>,
}

impl Comparison {
    /// Whether the images agree within the tolerance.
    pub fn matches(&self) -> bool {
        self.diff.is_none()
    }
}

impl fmt::Display for Comparison {
    /// matplotlib's report of a failed comparison, or a line saying the
    /// images match.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(diff) = &self.diff else {
            return write!(
                f,
                "Image files match: RMS Value {} within tolerance {}",
                self.rms, self.tol
            );
        };
        write!(
            f,
            "Error: Image files did not match.\n  RMS Value: {}\n  Expected:  \n    {}\n  \
             Actual:    \n    {}\n  Difference:\n    {}\n  Tolerance: \n    {}",
            self.rms,
            self.expected.display(),
            self.actual.display(),
            diff.display(),
            self.tol
        )
    }
}

/// What stops a comparison of images or figures.
#[derive(Debug)]
pub enum ComparisonError {
    /// The output image does not exist.
    MissingOutput(PathBuf),
    /// The output image is an empty file.
    EmptyOutput(PathBuf),
    /// The baseline image does not exist.
    MissingBaseline(PathBuf),
    /// Images of this file's format cannot be compared here: no converter
    /// installed rasterises it (see
    /// [`comparable_formats`](super::comparable_formats)).
    NotComparable(PathBuf),
    /// The converter rasterising a file for comparison failed (matplotlib's
    /// `ImageComparisonFailure` with the converter's output).
    Convert {
        /// The file converted.
        path: PathBuf,
        /// What the converter said.
        reason: String,
    },
    /// An image comparison was given another number of figures than of
    /// baseline images (see [`image_comparison`](super::image_comparison)).
    FigureCount {
        /// The number of baseline images.
        expected: usize,
        /// The number of figures made.
        actual: usize,
    },
    /// A file cannot be read as a PNG image.
    Unreadable {
        /// The file.
        path: PathBuf,
        /// Why it cannot be read.
        reason: String,
    },
    /// A file or directory cannot be written.
    Unwritable {
        /// The file or directory.
        path: PathBuf,
        /// Why it cannot be written.
        reason: String,
    },
    /// The images differ in size.
    Shape(ImageShapeError),
    /// A figure of [`check_figures_equal`](super::check_figures_equal)
    /// could not be built.
    Build(Box<dyn Error>),
    /// A figure of [`check_figures_equal`](super::check_figures_equal)
    /// could not be saved.
    Save(SaveError),
}

impl fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOutput(path) => {
                write!(f, "Output image {} does not exist.", path.display())
            }
            Self::EmptyOutput(path) => write!(f, "Output image file {} is empty.", path.display()),
            Self::MissingBaseline(path) => {
                write!(f, "Baseline image {:?} does not exist.", path.display())
            }
            Self::NotComparable(path) => {
                let extension = path.extension().unwrap_or_default().display();
                write!(f, "Don't know how to convert .{extension} files to png")
            }
            Self::Convert { path, reason } => {
                write!(f, "cannot convert {} to png: {reason}", path.display())
            }
            Self::FigureCount { expected, actual } => write!(
                f,
                "Test generated {actual} images but there are {expected} baseline images"
            ),
            Self::Unreadable { path, reason } => {
                write!(f, "cannot read {}: {reason}", path.display())
            }
            Self::Unwritable { path, reason } => {
                write!(f, "cannot write {}: {reason}", path.display())
            }
            Self::Shape(err) => err.fmt(f),
            Self::Build(err) => write!(f, "cannot build the figure: {err}"),
            Self::Save(err) => write!(f, "cannot save the figure: {err}"),
        }
    }
}

impl Error for ComparisonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Shape(err) => Some(err),
            Self::Build(err) => Some(err.as_ref()),
            Self::Save(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ImageShapeError> for ComparisonError {
    fn from(err: ImageShapeError) -> Self {
        Self::Shape(err)
    }
}

/// Compares the image `actual` with the baseline image `expected`
/// (matplotlib's `compare_images`). They match when the RMS of their
/// channel differences ([`calculate_rms`], with images whose pixels are
/// all opaque compared as RGB) is at most `tol`, or when `tol` is not
/// positive and they are identical. Otherwise matplotlib's difference
/// image is written beside `actual`, named with `-failed-diff` before
/// the extension: ten times the colour differences.
///
/// When `expected` is not a PNG file, both files are rasterised first
/// with [`convert`](super::convert) (cached), and compared as the PNG
/// images beside them; an EPS image is cropped to the size of a PDF
/// baseline ([`crop_to_same`](super::crop_to_same)).
///
/// ```
/// use pltrs::prelude::*;
/// use pltrs::testing::{self, ComparisonError};
///
/// testing::setup();
/// let dir = std::env::temp_dir().join(format!("pltrs-compare-doc-{}", std::process::id()));
/// std::fs::create_dir_all(&dir)?;
/// let fig = Figure::new();
/// fig.subplot().plot([0.0, 1.0], [1.0, 0.0])?;
/// fig.savefig(dir.join("expected.png"))?;
/// fig.savefig(dir.join("actual.png"))?;
/// let comparison = testing::compare_images(dir.join("expected.png"), dir.join("actual.png"), 0.001)?;
/// assert!(comparison.matches(), "{comparison}");
/// assert_eq!(comparison.rms, 0.0);
/// let missing = testing::compare_images(dir.join("expected.png"), dir.join("nothing.png"), 0.001);
/// assert!(matches!(missing, Err(ComparisonError::MissingOutput(_))));
/// std::fs::remove_dir_all(&dir)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When either file is missing (or `actual` is empty), `expected` is not
/// a PNG file and no converter installed here rasterises it, a converter
/// fails, a file cannot be read as a PNG, the images differ in size or
/// channels, or the difference image cannot be written.
pub fn compare_images(
    expected: impl AsRef<Path>,
    actual: impl AsRef<Path>,
    tol: f64,
) -> Result<Comparison, ComparisonError> {
    let (expected, actual) = (expected.as_ref(), actual.as_ref());
    match fs::metadata(actual) {
        Err(_) => return Err(ComparisonError::MissingOutput(actual.to_owned())),
        Ok(metadata) if metadata.len() == 0 => {
            return Err(ComparisonError::EmptyOutput(actual.to_owned()));
        }
        Ok(_) => {}
    }
    if !expected.exists() {
        return Err(ComparisonError::MissingBaseline(expected.to_owned()));
    }
    let (expected, actual) = if expected.extension().is_some_and(|ext| ext == "png") {
        (expected.to_owned(), actual.to_owned())
    } else {
        let actual = convert(actual, true)?;
        (convert(expected, true)?, actual)
    };
    let (actual_image, expected_image) = crop_loaded(
        &actual,
        Loaded::read(&actual)?,
        &expected,
        Loaded::read(&expected)?,
    );
    let diff_path = make_test_filename(&actual, "failed-diff");
    let mut comparison = Comparison {
        rms: 0.0,
        tol,
        expected,
        actual,
        diff: None,
    };
    if tol <= 0.0 && expected_image == actual_image {
        return Ok(comparison);
    }
    let (rms, diff) = rms_and_diff(&expected_image, &actual_image)?;
    comparison.rms = rms;
    if rms <= tol {
        return Ok(comparison);
    }
    write_png(&diff_path, &diff)?;
    comparison.diff = Some(diff_path);
    Ok(comparison)
}

#[cfg(test)]
mod tests {
    use pltrs_codec::png::{self, ColorType, ImageRef};

    use super::*;

    /// A fresh temporary directory for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pltrs-compare-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_rgba(path: &Path, width: u32, pixels: &[[u8; 4]]) {
        let data = pixels.as_flattened();
        let height = (pixels.len() as u32) / width;
        let png = png::encode(ImageRef::rgba8(width, height, data).unwrap()).unwrap();
        fs::write(path, png).unwrap();
    }

    #[test]
    fn rms_follows_matplotlibs_definition() {
        let expected = Grid::from_vec(1, 2, vec![[0_u8, 0, 0, 0], [255, 255, 255, 255]]).unwrap();
        let actual = Grid::from_vec(1, 2, vec![[0_u8, 0, 0, 4], [255, 255, 255, 255]]).unwrap();
        // One value of eight is off by 4: alpha counts.
        assert_eq!(calculate_rms(&expected, &actual), Ok(2.0_f64.sqrt()));
        assert_eq!(
            calculate_rms(&actual, &expected),
            Ok(2.0_f64.sqrt()),
            "symmetric"
        );
        assert_eq!(calculate_rms(&expected, &expected), Ok(0.0));
        let floats = Grid::from_vec(1, 1, vec![[0.5_f32, 0.25]]).unwrap();
        let zeros = Grid::from_vec(1, 1, vec![[0.0_f32, 0.0]]).unwrap();
        let rms = calculate_rms(&floats, &zeros).unwrap();
        assert!((rms - (0.3125_f64 / 2.0).sqrt()).abs() < 1e-12);
        let empty: Grid<[u8; 3]> = Grid::from_vec(0, 0, Vec::new()).unwrap();
        assert!(calculate_rms(&empty, &empty).unwrap().is_nan());
    }

    #[test]
    fn test_filenames_insert_their_purpose() {
        let path = Path::new("out/a.b.png");
        assert_eq!(
            make_test_filename(path, "failed-diff"),
            Path::new("out/a.b-failed-diff.png")
        );
        assert_eq!(
            make_test_filename(Path::new("plain"), "expected"),
            Path::new("plain-expected")
        );
    }

    #[test]
    fn mismatches_write_the_difference_image() {
        let dir = scratch("diff");
        let (expected, actual) = (dir.join("expected.png"), dir.join("actual.png"));
        write_rgba(&expected, 2, &[[10, 20, 30, 255], [0, 0, 0, 255]]);
        write_rgba(&actual, 2, &[[16, 20, 30, 255], [0, 0, 100, 255]]);
        let comparison = compare_images(&expected, &actual, 1.0).unwrap();
        assert!(!comparison.matches());
        let rms = ((36.0_f64 + 10_000.0) / 6.0).sqrt();
        assert!((comparison.rms - rms).abs() < 1e-12);
        let diff_path = dir.join("actual-failed-diff.png");
        assert_eq!(comparison.diff.as_deref(), Some(diff_path.as_path()));
        let diff = png::decode(&fs::read(&diff_path).unwrap()).unwrap();
        assert_eq!(diff.color_type(), ColorType::Rgb);
        assert_eq!(diff.to_rgba8(), [60, 0, 0, 255, 0, 0, 255, 255]);
        let report = comparison.to_string();
        assert!(report.starts_with("Error: Image files did not match.\n  RMS Value: "));
        assert!(report.ends_with("Tolerance: \n    1"), "{report}");

        let lenient = compare_images(&expected, &actual, 100.0).unwrap();
        assert!(lenient.matches() && lenient.rms > 0.0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn identical_images_match_without_tolerance() {
        let dir = scratch("identical");
        let path = dir.join("a.png");
        write_rgba(&path, 1, &[[1, 2, 3, 128]]);
        let comparison = compare_images(&path, &path, 0.0).unwrap();
        assert!(comparison.matches());
        assert_eq!(comparison.rms, 0.0);
        assert!(comparison.to_string().starts_with("Image files match"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn opaque_images_compare_as_rgb_and_translucent_ones_as_rgba() {
        let dir = scratch("alpha");
        let (opaque, translucent) = (dir.join("opaque.png"), dir.join("translucent.png"));
        write_rgba(&opaque, 1, &[[0, 0, 0, 255]]);
        write_rgba(&translucent, 1, &[[0, 0, 0, 254]]);
        let err = compare_images(&opaque, &translucent, 1.0).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Image sizes do not match expected size: (1, 1, 3) actual size (1, 1, 4)"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_empty_and_foreign_files_are_errors() {
        let dir = scratch("errors");
        let (png_file, empty, svg) = (dir.join("a.png"), dir.join("e.png"), dir.join("a.svg"));
        write_rgba(&png_file, 1, &[[0, 0, 0, 255]]);
        fs::write(&empty, "").unwrap();
        fs::write(&svg, "<svg/>").unwrap();
        let missing = dir.join("missing.png");
        let error =
            |expected: &Path, actual: &Path| compare_images(expected, actual, 0.0).unwrap_err();
        assert!(matches!(
            error(&png_file, &missing),
            ComparisonError::MissingOutput(_)
        ));
        assert!(matches!(
            error(&png_file, &empty),
            ComparisonError::EmptyOutput(_)
        ));
        assert!(matches!(
            error(&missing, &png_file),
            ComparisonError::MissingBaseline(_)
        ));
        let foreign = error(&svg, &png_file);
        assert_eq!(
            foreign.to_string(),
            "Don't know how to convert .png files to png"
        );
        assert!(matches!(
            error(&png_file, &svg),
            ComparisonError::Unreadable { .. }
        ));
        fs::remove_dir_all(&dir).unwrap();
    }
}
