//! Testing your own figures (`matplotlib.testing`): the configuration
//! image tests run under, image comparison and figure comparison.
//!
//! * [`setup`] puts the process and the current thread in the state
//!   matplotlib's tests run in: matplotlib's look and defaults, the
//!   `DejaVu Sans` font, no display, and no user configuration.
//! * [`compare_images`] compares a PNG with a baseline PNG by matplotlib's
//!   RMS metric ([`calculate_rms`]) and, when they differ by more than the
//!   tolerance, writes matplotlib's `-failed-diff` image beside the
//!   output.
//! * [`check_figures_equal`] builds a test figure and a reference figure,
//!   saves both and compares them (matplotlib's `@check_figures_equal`).
//!
//! Only PNG files can be compared ([`comparable_formats`]): pltrs writes
//! PDF, SVG and PostScript but cannot rasterise them again, the way
//! matplotlib needs Ghostscript or Inkscape for them.
//!
//! A test in the style of matplotlib's, checking that a line without a
//! colour takes the first colour of the cycle:
//!
//! ```python
//! @check_figures_equal()
//! def test_default_color(fig_test, fig_ref):
//!     fig_test.subplots().plot([0, 1], [0, 1])
//!     fig_ref.subplots().plot([0, 1], [0, 1], color="C0")
//! ```
//!
//! ```
//! use pltrs::lines::LineProps;
//! use pltrs::testing;
//!
//! testing::setup();
//! let out = std::env::temp_dir().join(format!("pltrs-testing-doc-{}", std::process::id()));
//! let comparison = testing::check_figures_equal(
//!     out.join("default_color.png"),
//!     0.0,
//!     |fig| {
//!         fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
//!         Ok(())
//!     },
//!     |fig| {
//!         let props = LineProps::default().color("C0");
//!         fig.subplot().plot_with([0.0, 1.0], [0.0, 1.0], props)?;
//!         Ok(())
//!     },
//! )?;
//! assert!(comparison.matches(), "{comparison}");
//! std::fs::remove_dir_all(&out)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod compare;
mod figures;
#[cfg(test)]
mod support;

pub use compare::{
    Comparison, ComparisonError, ImageShapeError, calculate_rms, comparable_formats, compare_images,
};
pub use figures::check_figures_equal;
#[cfg(test)]
pub(crate) use support::*;

use crate::look::{self, Appearance};
use crate::pyplot::{self, Backend};
use crate::rc::RcParams;

/// Prepares image tests as matplotlib's `matplotlib.testing.setup` does:
/// for the whole process, matplotlib's [look] and no user
/// configuration ([`config::ignore_user_config`](crate::config::ignore_user_config));
/// for the current thread, pyplot without a display ([`Backend::Agg`])
/// and the parameters matplotlib's tests start from: matplotlib's
/// defaults with the font family `DejaVu Sans` and `svg.hashsalt:
/// matplotlib` (`text.hinting`, which matplotlib's `setup` also sets,
/// keeps its default, `default`). Call it at the start of each test, as
/// matplotlib's test fixture does: tests run on threads of their own.
///
/// ```
/// use pltrs::{rc, testing};
/// rc::set("lines.linewidth", "4")?;
/// testing::setup();
/// let params = rc::params();
/// assert_eq!(params.lines_linewidth, 1.5);
/// assert_eq!(params.font_family, ["DejaVu Sans"]);
/// assert!(pltrs::config::user_config_ignored());
/// # Ok::<(), pltrs::rc::RcError>(())
/// ```
pub fn setup() {
    crate::config::ignore_user_config();
    look::set_appearance(Appearance::MATPLOTLIB);
    pyplot::switch_backend(Backend::Agg);
    let mut params = RcParams::matplotlib();
    params.font_family = vec!["DejaVu Sans".to_owned()];
    params.svg_hashsalt = Some("matplotlib".to_owned());
    crate::rc::replace(params);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_starts_from_matplotlibs_test_configuration() {
        crate::rc::context(|| {
            crate::rc::set("font.size", "30").unwrap();
            setup();
            let params = crate::rc::params();
            let mut expected = RcParams::matplotlib();
            expected.set("font.family", "DejaVu Sans").unwrap();
            expected.set("svg.hashsalt", "matplotlib").unwrap();
            assert_eq!(*params, expected);
            assert_eq!(look::appearance(), Appearance::MATPLOTLIB);
            assert_eq!(pyplot::get_backend(), Backend::Agg);
            assert!(crate::config::user_config_ignored());
        });
    }
}
