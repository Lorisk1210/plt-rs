//! Comparing images (matplotlib's `testing.compare`): the RMS metric, and
//! `compare_images` with the difference image it writes on failure.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use pltrs_codec::png::{self, ColorType, ImageRef};

use crate::figure::{SaveError, SaveFormat};
use crate::numeric::Grid;

/// The formats [`compare_images`] compares (matplotlib's
/// `comparable_formats`): PNG only, as for matplotlib without Ghostscript
/// and Inkscape to rasterise PDF, PostScript and SVG.
pub fn comparable_formats() -> &'static [SaveFormat] {
    &[SaveFormat::Png]
}

/// Whether [`compare_images`] can compare the file `path`: whether its
/// extension is `png`.
pub(super) fn is_comparable(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "png")
}

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

/// An image file as the comparison loads it (matplotlib's `_load_image`):
/// RGB images as RGB, others as RGBA unless every pixel is opaque.
#[derive(Clone, Debug, PartialEq)]
enum Loaded {
    /// Opaque pixels.
    Rgb(Grid<[u8; 3]>),
    /// Pixels with transparency.
    Rgba(Grid<[u8; 4]>),
}

impl Loaded {
    /// Reads the PNG file at `path`.
    fn read(path: &Path) -> Result<Self, ComparisonError> {
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
    fn shape(&self) -> (usize, usize, usize) {
        match self {
            Self::Rgb(grid) => (grid.rows(), grid.cols(), 3),
            Self::Rgba(grid) => (grid.rows(), grid.cols(), 4),
        }
    }
}

/// The grid of `rows × cols` decoded pixels.
fn filled_grid<T>(rows: usize, cols: usize, pixels: Vec<T>) -> Grid<T> {
    Grid::from_vec(rows, cols, pixels).expect("a decoded image has a pixel per cell")
}

/// The RMS of two loaded images and their difference image (matplotlib's
/// `calculate_rms_and_diff`).
fn rms_and_diff(
    expected: &Loaded,
    actual: &Loaded,
) -> Result<(f64, Grid<[u8; 3]>), ImageShapeError> {
    match (expected, actual) {
        (Loaded::Rgb(expected), Loaded::Rgb(actual)) => rms_and_diff_of(expected, actual),
        (Loaded::Rgba(expected), Loaded::Rgba(actual)) => rms_and_diff_of(expected, actual),
        _ => Err(ImageShapeError {
            expected: expected.shape(),
            actual: actual.shape(),
        }),
    }
}

/// The RMS of two images and their difference image: the colour channels
/// of each pixel differ ten times as much, clipped to 255, and alpha is
/// left out.
fn rms_and_diff_of<const N: usize>(
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
pub(super) fn test_filename(path: &Path, purpose: &str) -> PathBuf {
    let mut name = path.file_stem().unwrap_or_default().to_owned();
    name.push("-");
    name.push(purpose);
    if let Some(extension) = path.extension() {
        name.push(".");
        name.push(extension);
    }
    path.with_file_name(name)
}

/// Writes `image` to `path` as an RGB PNG.
fn write_rgb_png(path: &Path, image: &Grid<[u8; 3]>) -> Result<(), ComparisonError> {
    let unwritable = |reason: String| ComparisonError::Unwritable {
        path: path.to_owned(),
        reason,
    };
    let side = |n: usize| u32::try_from(n).map_err(|err| unwritable(err.to_string()));
    let png = ImageRef::rgb8(
        side(image.cols())?,
        side(image.rows())?,
        image.as_slice().as_flattened(),
    )
    .and_then(png::encode)
    .map_err(|err| unwritable(err.to_string()))?;
    fs::write(path, png).map_err(|err| unwritable(err.to_string()))
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
    /// Images of this file's format cannot be compared (only PNG can, see
    /// [`comparable_formats`]).
    NotComparable(PathBuf),
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
                write!(f, "Don't know how to convert {extension} files to png")
            }
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

/// Compares the PNG image `actual` with the baseline image `expected`
/// (matplotlib's `compare_images`). They match when the RMS of their
/// channel differences ([`calculate_rms`], with images whose pixels are
/// all opaque compared as RGB) is at most `tol`, or when `tol` is not
/// positive and they are identical. Otherwise matplotlib's difference
/// image is written beside `actual`, named with `-failed-diff` before
/// the extension: ten times the colour differences.
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
/// a `.png` file, a file cannot be read as a PNG, the images differ in
/// size or channels, or the difference image cannot be written.
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
    if !is_comparable(expected) {
        return Err(ComparisonError::NotComparable(expected.to_owned()));
    }
    let expected_image = Loaded::read(expected)?;
    let actual_image = Loaded::read(actual)?;
    let mut comparison = Comparison {
        rms: 0.0,
        tol,
        expected: expected.to_owned(),
        actual: actual.to_owned(),
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
    let diff_path = test_filename(actual, "failed-diff");
    write_rgb_png(&diff_path, &diff)?;
    comparison.diff = Some(diff_path);
    Ok(comparison)
}

#[cfg(test)]
mod tests {
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
            test_filename(path, "failed-diff"),
            Path::new("out/a.b-failed-diff.png")
        );
        assert_eq!(
            test_filename(Path::new("plain"), "expected"),
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
            "Don't know how to convert svg files to png"
        );
        assert!(matches!(
            error(&png_file, &svg),
            ComparisonError::Unreadable { .. }
        ));
        assert_eq!(comparable_formats(), [SaveFormat::Png]);
        fs::remove_dir_all(&dir).unwrap();
    }
}
