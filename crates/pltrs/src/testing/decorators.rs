//! Comparing figures with baseline images (matplotlib's
//! `@image_comparison` decorator and its `remove_ticks_and_titles`).

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use pltrs_render::Metadata;

use super::compare::{Comparison, ComparisonError, compare_images, make_test_filename};
use crate::figure::{Figure, SaveFormat, SaveOptions};
use crate::ticker::NullFormatter;

/// How [`image_comparison`] compares figures with baseline images
/// (matplotlib's arguments of `@image_comparison`).
#[derive(Clone, Debug)]
pub struct ImageComparison {
    /// The directory of the baseline images (matplotlib's
    /// `baseline_images/<test module>`).
    pub baseline_dir: PathBuf,
    /// The directory the figures and copies of their baseline images are
    /// written to (matplotlib's `result_images/<test module>`), made if
    /// missing.
    pub result_dir: PathBuf,
    /// The formats each figure is saved and compared in (`extensions`).
    pub extensions: Vec<SaveFormat>,
    /// The largest RMS accepted (`tol`).
    pub tol: f64,
    /// Whether titles and tick labels are removed before the figures are
    /// saved (`remove_text`, see [`remove_ticks_and_titles`]).
    pub remove_text: bool,
    /// How the figures are saved (`savefig_kwarg`); PDF files are saved
    /// without creator, producer and date unless metadata is given.
    pub save_options: SaveOptions,
}

impl Default for ImageComparison {
    /// matplotlib's defaults: baseline images in `baseline_images`,
    /// results in `result_images`, compared as PNG, PDF and SVG without
    /// tolerance, with their text.
    fn default() -> Self {
        Self {
            baseline_dir: PathBuf::from("baseline_images"),
            result_dir: PathBuf::from("result_images"),
            extensions: vec![SaveFormat::Png, SaveFormat::Pdf, SaveFormat::Svg],
            tol: 0.0,
            remove_text: false,
            save_options: SaveOptions::default(),
        }
    }
}

/// Compares the figures `build` makes with the baseline images
/// `baseline_images` names, in order (matplotlib's `@image_comparison`):
/// for each format of [`ImageComparison::extensions`], `build` makes the
/// figures afresh, each is saved in the result directory under its
/// baseline's name (`<name>.<ext>`), its baseline image is copied beside
/// it (`<name>-expected.<ext>`) and [`compare_images`] compares them.
/// Returns the comparisons, format by format; formats that cannot be
/// compared here ([`comparable_formats`](super::comparable_formats)) fail
/// with [`ComparisonError::NotComparable`], so leave them out where their
/// converters may be missing. Run [`setup`](super::setup) first, as
/// matplotlib's tests do.
///
/// ```
/// use pltrs::figure::SaveFormat;
/// use pltrs::prelude::*;
/// use pltrs::testing::{self, ImageComparison};
///
/// testing::setup();
/// let dir = std::env::temp_dir().join(format!("pltrs-image-comparison-{}", std::process::id()));
/// let line = || -> Result<Vec<Figure>, Box<dyn std::error::Error>> {
///     let fig = Figure::with_size(2.0, 2.0, 50.0);
///     fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
///     Ok(vec![fig])
/// };
/// // The baseline image, made once.
/// std::fs::create_dir_all(dir.join("baseline"))?;
/// line()?[0].savefig(dir.join("baseline/line.png"))?;
/// let options = ImageComparison {
///     baseline_dir: dir.join("baseline"),
///     result_dir: dir.join("result"),
///     extensions: vec![SaveFormat::Png],
///     ..ImageComparison::default()
/// };
/// let comparisons = testing::image_comparison(&options, &["line"], line)?;
/// assert!(comparisons[0].matches(), "{}", comparisons[0]);
/// assert!(dir.join("result/line-expected.png").exists());
/// std::fs::remove_dir_all(&dir)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When `build` fails or makes another number of figures than there are
/// baseline images, a baseline image is missing, a figure cannot be
/// saved, and as [`compare_images`].
pub fn image_comparison(
    options: &ImageComparison,
    baseline_images: &[&str],
    mut build: impl FnMut() -> Result<Vec<Figure>, Box<dyn Error>>,
) -> Result<Vec<Comparison>, ComparisonError> {
    let unwritable = |path: &PathBuf, err: std::io::Error| ComparisonError::Unwritable {
        path: path.clone(),
        reason: err.to_string(),
    };
    fs::create_dir_all(&options.result_dir).map_err(|err| unwritable(&options.result_dir, err))?;
    let mut comparisons = Vec::new();
    for &format in &options.extensions {
        let figures = build().map_err(ComparisonError::Build)?;
        if figures.len() != baseline_images.len() {
            return Err(ComparisonError::FigureCount {
                expected: baseline_images.len(),
                actual: figures.len(),
            });
        }
        let extension = format.extension();
        for (figure, name) in figures.iter().zip(baseline_images) {
            if options.remove_text {
                remove_ticks_and_titles(figure);
            }
            let file = format!("{name}.{extension}");
            let actual = options.result_dir.join(&file);
            figure
                .savefig_with(&actual, save_options(options, format))
                .map_err(ComparisonError::Save)?;
            let baseline = options.baseline_dir.join(&file);
            if !baseline.exists() {
                return Err(ComparisonError::MissingBaseline(baseline));
            }
            let expected = make_test_filename(&actual, "expected");
            fs::copy(&baseline, &expected).map_err(|err| unwritable(&expected, err))?;
            comparisons.push(compare_images(&expected, &actual, options.tol)?);
        }
    }
    Ok(comparisons)
}

/// The options a figure is saved with in `format`: matplotlib leaves the
/// creator, producer and date out of PDF files unless metadata is given.
fn save_options(options: &ImageComparison, format: SaveFormat) -> SaveOptions {
    let mut save = options.save_options.clone();
    save.format = Some(format);
    if format == SaveFormat::Pdf && save.metadata.is_none() {
        save.metadata = Some(Metadata {
            creator: None,
            producer: None,
            date: None,
            ..Metadata::default()
        });
    }
    save
}

/// Removes the titles and tick labels of `figure` (matplotlib's
/// `remove_ticks_and_titles`, which `@image_comparison(remove_text=True)`
/// applies): its super title and the title of every axes are emptied,
/// and the ticks of every axis, 3-D ones included, and of the axes inside
/// axes, are labelled by a [`NullFormatter`]. Tick marks stay.
///
/// ```
/// use pltrs::prelude::*;
/// use pltrs::testing::remove_ticks_and_titles;
/// let fig = Figure::new();
/// fig.suptitle("Results");
/// let ax = fig.subplot();
/// ax.set_title("Run 1");
/// ax.plot([0.0, 1.0], [0.0, 1.0])?;
/// remove_ticks_and_titles(&fig);
/// assert_eq!(fig.suptitle_text(), "");
/// assert!(ax.xaxis().ticklabels(false)?.iter().all(String::is_empty));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn remove_ticks_and_titles(figure: &Figure) {
    figure.suptitle("");
    let mut pending = figure.axes();
    while let Some(ax) = pending.pop() {
        ax.set_title("");
        for axis in [ax.xaxis(), ax.yaxis()] {
            axis.set_major_formatter(NullFormatter);
            axis.set_minor_formatter(NullFormatter);
        }
        pending.extend(ax.child_axes());
    }
    for ax in figure.axes3d() {
        ax.set_title("");
        for axis in [ax.xaxis(), ax.yaxis(), ax.zaxis()] {
            axis.set_major_formatter(NullFormatter);
            axis.set_minor_formatter(NullFormatter);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::TitleLocation;
    use crate::mplot3d::Axes3DOptions;
    use crate::scratch::Scratch;

    fn one_line() -> Result<Vec<Figure>, Box<dyn Error>> {
        let fig = Figure::with_size(2.0, 2.0, 40.0);
        fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
        Ok(vec![fig])
    }

    #[test]
    fn figures_are_compared_with_their_baselines() {
        crate::testing::setup();
        let scratch = Scratch::new("image-comparison");
        let dir = scratch.path("");
        let baseline = dir.join("baseline");
        fs::create_dir_all(&baseline).unwrap();
        one_line().unwrap()[0]
            .savefig(baseline.join("line.png"))
            .unwrap();
        let options = ImageComparison {
            baseline_dir: baseline,
            result_dir: dir.join("result"),
            extensions: vec![SaveFormat::Png],
            ..ImageComparison::default()
        };
        let comparisons = image_comparison(&options, &["line"], one_line).unwrap();
        assert_eq!(comparisons.len(), 1);
        assert!(comparisons[0].matches());
        // The text removed, the figure differs from its baseline.
        let without_text = ImageComparison {
            remove_text: true,
            ..options.clone()
        };
        let comparisons = image_comparison(&without_text, &["line"], one_line).unwrap();
        assert!(!comparisons[0].matches());
        let count = image_comparison(&options, &["line", "other"], one_line).unwrap_err();
        assert_eq!(
            count.to_string(),
            "Test generated 1 images but there are 2 baseline images"
        );
        let missing = image_comparison(&options, &["absent"], one_line).unwrap_err();
        assert!(matches!(missing, ComparisonError::MissingBaseline(_)));
    }

    #[test]
    fn pdf_files_are_saved_without_dates() {
        let options = ImageComparison::default();
        let pdf = save_options(&options, SaveFormat::Pdf);
        let metadata = pdf.metadata.expect("metadata for PDF files");
        assert_eq!(
            (metadata.creator, metadata.producer, metadata.date),
            (None, None, None)
        );
        assert!(save_options(&options, SaveFormat::Png).metadata.is_none());
    }

    #[test]
    fn titles_and_tick_labels_go_but_tick_marks_stay() {
        let fig = Figure::new();
        fig.suptitle("t");
        let ax = fig.subplot();
        ax.set_title("a");
        ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let inset = ax.inset_axes([0.5, 0.5, 0.4, 0.4]);
        inset.set_title("inset");
        let ax3 = fig
            .add_axes_3d([0.0, 0.0, 0.3, 0.3], Axes3DOptions::default())
            .unwrap();
        ax3.set_title("3-D");
        let ticks = ax.xaxis().ticklocs(false).unwrap();
        remove_ticks_and_titles(&fig);
        assert_eq!(fig.suptitle_text(), "");
        for axes in [&ax, &inset] {
            assert_eq!(axes.title(TitleLocation::Center), "");
            let labels = axes.yaxis().ticklabels(false).unwrap();
            assert!(labels.iter().all(String::is_empty));
        }
        assert_eq!(ax.xaxis().ticklocs(false).unwrap(), ticks);
        assert_eq!(ax3.title(), "");
        fig.layout();
        let labels = ax3.zaxis().ticklabels(false).unwrap();
        assert!(labels.iter().all(String::is_empty));
    }
}
