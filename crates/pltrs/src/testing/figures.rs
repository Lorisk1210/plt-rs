//! Comparing a figure with a reference figure drawn another way
//! (matplotlib's `@check_figures_equal`).

use std::error::Error;
use std::fs;
use std::path::Path;

use super::compare::{Comparison, ComparisonError, compare_images, is_comparable, test_filename};
use crate::figure::Figure;

/// Builds a test figure and a reference figure, saves both as PNG and
/// compares them with [`compare_images`] (matplotlib's
/// `check_figures_equal` decorator). `test` draws the test figure, saved
/// at `path`; `reference` draws the reference figure, saved beside it
/// with `-expected` before the extension. When they differ by more than
/// `tol` (0: when they differ at all), the difference image goes beside
/// them too. Both figures are made at the size and resolution of the
/// current parameters, and changes `test` and `reference` make to the
/// parameters last until both figures are saved, but not beyond.
///
/// ```
/// use pltrs::testing;
///
/// testing::setup();
/// let out = std::env::temp_dir().join(format!("pltrs-figures-equal-{}", std::process::id()));
/// let comparison = testing::check_figures_equal(
///     out.join("segments.png"),
///     0.0,
///     |fig| {
///         fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
///         Ok(())
///     },
///     |fig| {
///         let ax = fig.subplot();
///         ax.plot([0.0, 0.5], [0.0, 0.5])?;
///         ax.plot([0.5, 1.0], [0.5, 1.0])?;
///         Ok(())
///     },
/// )?;
/// // The second segment takes the next colour of the cycle.
/// assert!(!comparison.matches());
/// assert_eq!(comparison.expected, out.join("segments-expected.png"));
/// assert_eq!(comparison.diff, Some(out.join("segments-failed-diff.png")));
/// assert!(comparison.to_string().starts_with("Error: Image files did not match."));
/// std::fs::remove_dir_all(&out)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When `path` is not a `.png` file, a figure cannot be built or saved,
/// and as [`compare_images`].
pub fn check_figures_equal(
    path: impl AsRef<Path>,
    tol: f64,
    test: impl FnOnce(&Figure) -> Result<(), Box<dyn Error>>,
    reference: impl FnOnce(&Figure) -> Result<(), Box<dyn Error>>,
) -> Result<Comparison, ComparisonError> {
    let path = path.as_ref();
    if !is_comparable(path) {
        return Err(ComparisonError::NotComparable(path.to_owned()));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|err| ComparisonError::Unwritable {
            path: dir.to_owned(),
            reason: err.to_string(),
        })?;
    }
    let expected = test_filename(path, "expected");
    crate::rc::context(|| {
        let (test_figure, reference_figure) = (Figure::new(), Figure::new());
        test(&test_figure).map_err(ComparisonError::Build)?;
        reference(&reference_figure).map_err(ComparisonError::Build)?;
        test_figure.savefig(path).map_err(ComparisonError::Save)?;
        reference_figure
            .savefig(&expected)
            .map_err(ComparisonError::Save)
    })?;
    compare_images(&expected, path, tol)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pltrs-figures-equal-{name}-{}", std::process::id()))
    }

    #[test]
    fn equal_figures_match_and_parameters_are_restored() {
        let out = out_dir("equal");
        let width = crate::rc::params().lines_linewidth;
        let comparison = check_figures_equal(
            out.join("lw.png"),
            0.0,
            |fig| {
                crate::rc::set("lines.linewidth", "4")?;
                fig.subplot().plot([0.0, 1.0], [1.0, 0.0])?;
                Ok(())
            },
            |fig| {
                let props = crate::lines::LineProps::default().linewidth(4.0);
                fig.subplot().plot_with([0.0, 1.0], [1.0, 0.0], props)?;
                Ok(())
            },
        )
        .unwrap();
        assert!(comparison.matches(), "{comparison}");
        assert!(out.join("lw.png").exists() && out.join("lw-expected.png").exists());
        assert_eq!(crate::rc::params().lines_linewidth, width);
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn failures_to_build_or_compare_are_errors() {
        let out = out_dir("errors");
        let fails = |_: &Figure| -> Result<(), Box<dyn Error>> { Err("no figure".into()) };
        let fine = |_: &Figure| -> Result<(), Box<dyn Error>> { Ok(()) };
        let err = check_figures_equal(out.join("a.png"), 0.0, fine, fails).unwrap_err();
        assert_eq!(err.to_string(), "cannot build the figure: no figure");
        let err = check_figures_equal(out.join("a.svg"), 0.0, fine, fine).unwrap_err();
        assert!(matches!(err, ComparisonError::NotComparable(_)));
        assert!(!out.join("a.svg").exists(), "nothing is drawn");
        let _ = fs::remove_dir_all(&out);
    }
}
