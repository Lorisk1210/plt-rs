//! Configuration files and tests of your figures: the matplotlib
//! configuration directory, `matplotlibrc`, the user style library, and
//! `matplotlib.testing`.
//!
//! pltrs reads the same files as matplotlib, under pltrs names:
//!
//! * The rc file is `pltrsrc`. It uses the matplotlib syntax, so you can
//!   copy a `matplotlibrc`.
//! * The configuration directory is `$PLTRSCONFIGDIR` or `~/.config/pltrs`.
//! * User style sheets go in its directory `stylelib`.
//!
//! Threads start from the parameters of the look, with the rc file on top.
//! pltrs does not log the lines of the file that it skips. To read them,
//! call [`config::user_rc_file`]. [`config`] tells where pltrs looks for
//! each file.
//!
//! [`testing`] has the parts of `matplotlib.testing` that test your own
//! figures:
//!
//! * [`testing::setup`] sets the configuration of the matplotlib tests.
//! * [`testing::compare_images`] and [`testing::calculate_rms`] compare
//!   images.
//! * [`testing::check_figures_equal`] and [`testing::image_comparison`]
//!   replace the decorators. They take the drawing code as closures.
//!
//! As in matplotlib, Ghostscript rasterizes PDF and PostScript files, and
//! Inkscape rasterizes SVG files, before the comparison
//! ([`testing::convert`]).
//!
//! # Equivalences
//!
//! | matplotlib                                       | pltrs                                          |
//! |--------------------------------------------------|------------------------------------------------|
//! | `MPLCONFIGDIR`                                   | `PLTRSCONFIGDIR` ([`config::CONFIG_DIR_VAR`])  |
//! | `MATPLOTLIBRC`                                   | `PLTRSRC` ([`config::RC_FILE_VAR`])            |
//! | `matplotlibrc`                                   | `pltrsrc` ([`config::RC_FILE_NAME`])           |
//! | `~/.config/matplotlib`, `~/.cache/matplotlib`    | `~/.config/pltrs`, `~/.cache/pltrs`            |
//! | `mpl.get_configdir()`, `mpl.get_cachedir()`      | [`config::config_dir`], [`config::cache_dir`]  |
//! | `mpl.matplotlib_fname()`                         | [`config::rc_file_path`] (`None` without a user file) |
//! | warnings logged about bad lines of `matplotlibrc` | [`config::user_rc_file`]`()?` and its `warnings` |
//! | `mpl.rcParamsOrig`                               | [`RcParams::startup`]                          |
//! | `mpl.rc_file_defaults()`, `mpl.rcdefaults()`     | [`rc::restore_startup`], [`rc::reset`]         |
//! | `style.USER_LIBRARY_PATHS`                       | [`style::user_library_paths`], [`style::set_user_library_paths`] |
//! | `style.reload_library()`                         | [`style::reload_library`]                      |
//! | `matplotlib.testing.setup()`                     | [`testing::setup`]                             |
//! | a separate `MPLCONFIGDIR` for tests              | [`config::ignore_user_config`] (which [`testing::setup`] calls) |
//! | `compare_images(expected, actual, tol)`          | [`testing::compare_images`], returning a [`Comparison`] |
//! | `calculate_rms(expected, actual)`                | [`testing::calculate_rms`]                     |
//! | `comparable_formats()`                           | [`testing::comparable_formats`]: PNG, GIF, and PDF, EPS and SVG when their converters are installed |
//! | `convert(filename, cache)`, `get_file_hash(path)` | [`testing::convert`], [`testing::get_file_hash`] |
//! | `make_test_filename(fname, purpose)`, `crop_to_same(...)` | [`testing::make_test_filename`], [`testing::crop_to_same`] |
//! | `save_diff_image(expected, actual, output)`      | [`testing::save_diff_image`]                   |
//! | `@check_figures_equal(extensions=["png"], tol=0)` | [`testing::check_figures_equal`]`(path, tol, test, reference)` |
//! | `@image_comparison(baseline_images, extensions=, tol=, remove_text=)` | [`testing::image_comparison`]`(&options, names, build)` with [`ImageComparison`] |
//! | `remove_ticks_and_titles(fig)`                   | [`testing::remove_ticks_and_titles`]           |
//! | `set_font_settings_for_testing()`, `set_reproducibility_for_testing()` | [`testing::setup`] does both          |
//! | `ImageComparisonFailure`                         | a [`Comparison`] that does not match, or a [`ComparisonError`] |
//!
//! # Configuration
//!
//! ```text
//! # ~/.config/pltrs/pltrsrc, as ~/.config/matplotlib/matplotlibrc
//! lines.linewidth: 2
//! axes.grid: True
//! ```
//!
//! ```python
//! import matplotlib as mpl
//! print(mpl.get_configdir(), mpl.matplotlib_fname())
//! mpl.rcParams["lines.linewidth"] = 4
//! mpl.rc_file_defaults()   # back to what matplotlibrc says
//! mpl.rcdefaults()         # the matplotlib defaults
//! assert mpl.rcParams["lines.linewidth"] == 1.5
//! ```
//!
//! ```
//! use pltrs::{config, rc};
//!
//! let dir = config::config_dir()?;
//! assert!(dir.is_dir());
//! let rc_file: Option<std::path::PathBuf> = config::rc_file_path();
//! assert!(rc_file.is_none_or(|path| path.ends_with("pltrsrc")));
//! rc::set("lines.linewidth", "4")?;
//! rc::restore_startup(); // back to the look and what pltrsrc says
//! rc::reset(); // the matplotlib defaults
//! assert_eq!(rc::params().lines_linewidth, 1.5);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Image comparison tests
//!
//! ```python
//! from matplotlib.testing.compare import compare_images
//! from matplotlib.testing.decorators import check_figures_equal
//!
//! @check_figures_equal(extensions=["png"])
//! def test_bar_color(fig_test, fig_ref):
//!     fig_test.subplots().bar([1, 2], [3, 4], color="C1")
//!     fig_ref.subplots().bar([1, 2], [3, 4], color="tab:orange")
//!
//! def test_against_baseline():
//!     fig, ax = plt.subplots()
//!     ax.bar([1, 2], [3, 4], color="C1")
//!     fig.savefig("result_images/bars.png")
//!     assert compare_images("baseline_images/bars.png", "result_images/bars.png", tol=0) is None
//! ```
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::testing;
//!
//! testing::setup();
//! let out = std::env::temp_dir().join(format!("pltrs-porting-testing-{}", std::process::id()));
//! let bars = |color: &str| BarOptions { color: vec![color.into()], ..BarOptions::default() };
//! let comparison = testing::check_figures_equal(
//!     out.join("bar_color.png"),
//!     0.0,
//!     |fig| {
//!         fig.subplot().bar(vec![1.0, 2.0], [3.0, 4.0], bars("C1"))?;
//!         Ok(())
//!     },
//!     |fig| {
//!         fig.subplot().bar(vec![1.0, 2.0], [3.0, 4.0], bars("tab:orange"))?;
//!         Ok(())
//!     },
//! )?;
//! assert!(comparison.matches(), "{comparison}");
//!
//! // The test figure saved above takes the place of a baseline image.
//! let fig = Figure::new();
//! fig.subplot().bar(vec![1.0, 2.0], [3.0, 4.0], bars("C1"))?;
//! fig.savefig(out.join("bars.png"))?;
//! let comparison = testing::compare_images(out.join("bar_color.png"), out.join("bars.png"), 0.0)?;
//! assert!(comparison.matches(), "{comparison}");
//! std::fs::remove_dir_all(&out)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`RcParams::startup`]: crate::rc::RcParams::startup
//! [`rc::restore_startup`]: crate::rc::restore_startup
//! [`rc::reset`]: crate::rc::reset
//! [`style::user_library_paths`]: crate::style::user_library_paths
//! [`style::set_user_library_paths`]: crate::style::set_user_library_paths
//! [`style::reload_library`]: crate::style::reload_library
//! [`config`]: crate::config
//! [`config::CONFIG_DIR_VAR`]: crate::config::CONFIG_DIR_VAR
//! [`config::RC_FILE_VAR`]: crate::config::RC_FILE_VAR
//! [`config::RC_FILE_NAME`]: crate::config::RC_FILE_NAME
//! [`config::config_dir`]: crate::config::config_dir
//! [`config::cache_dir`]: crate::config::cache_dir
//! [`config::rc_file_path`]: crate::config::rc_file_path
//! [`config::user_rc_file`]: crate::config::user_rc_file
//! [`config::ignore_user_config`]: crate::config::ignore_user_config
//! [`testing`]: crate::testing
//! [`testing::setup`]: crate::testing::setup
//! [`testing::compare_images`]: crate::testing::compare_images
//! [`testing::calculate_rms`]: crate::testing::calculate_rms
//! [`testing::comparable_formats`]: crate::testing::comparable_formats
//! [`testing::check_figures_equal`]: crate::testing::check_figures_equal
//! [`Comparison`]: crate::testing::Comparison
//! [`ComparisonError`]: crate::testing::ComparisonError
//! [`testing::image_comparison`]: crate::testing::image_comparison
//! [`testing::convert`]: crate::testing::convert
//! [`testing::get_file_hash`]: crate::testing::get_file_hash
//! [`testing::make_test_filename`]: crate::testing::make_test_filename
//! [`testing::crop_to_same`]: crate::testing::crop_to_same
//! [`testing::save_diff_image`]: crate::testing::save_diff_image
//! [`testing::remove_ticks_and_titles`]: crate::testing::remove_ticks_and_titles
//! [`ImageComparison`]: crate::testing::ImageComparison
