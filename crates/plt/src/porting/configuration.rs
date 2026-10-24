//! Configuration files and testing your figures: matplotlib's
//! configuration directory, `matplotlibrc` and user style library, and
//! `matplotlib.testing`.
//!
//! **Configuration.** plt-rs reads the files matplotlib reads, under names
//! of its own: the rc file is `pltrc` (in matplotlib's syntax, so a
//! `matplotlibrc` can be copied over), the configuration directory is
//! `$PLTCONFIGDIR` or `~/.config/plt-rs`, and user style sheets go in its
//! `stylelib` directory. Threads start from the look's parameters with the
//! rc file on top, the way matplotlib's `matplotlibrc` applies over its
//! defaults; to read what plt-rs skipped in the file, ask
//! [`config::user_rc_file`] rather than a log. [`config`] tells where each
//! file is looked for.
//!
//! **Testing.** [`testing`] has the parts of `matplotlib.testing` that
//! test your own figures: [`testing::setup`] for the configuration
//! matplotlib's tests run in, [`testing::compare_images`] and
//! [`testing::calculate_rms`], and [`testing::check_figures_equal`], a
//! function taking the test and reference figures' drawing as two
//! closures instead of a decorator. Only PNG files can be compared: plt-rs
//! cannot rasterise PDF, SVG or PostScript again, which matplotlib needs
//! Ghostscript or Inkscape for. A matplotlib test using
//! `@image_comparison` saves its figure and calls [`testing::compare_images`]
//! with its baseline image.
//!
//! # Equivalences
//!
//! | matplotlib                                       | plt-rs                                         |
//! |--------------------------------------------------|------------------------------------------------|
//! | `MPLCONFIGDIR`                                   | `PLTCONFIGDIR` ([`config::CONFIG_DIR_VAR`])    |
//! | `MATPLOTLIBRC`                                   | `PLTRC` ([`config::RC_FILE_VAR`])              |
//! | `matplotlibrc`                                   | `pltrc` ([`config::RC_FILE_NAME`])             |
//! | `~/.config/matplotlib`, `~/.cache/matplotlib`    | `~/.config/plt-rs`, `~/.cache/plt-rs`          |
//! | `mpl.get_configdir()`, `mpl.get_cachedir()`      | [`config::config_dir`], [`config::cache_dir`]  |
//! | `mpl.matplotlib_fname()`                         | [`config::rc_file_path`] (`None` without a user file) |
//! | warnings logged about bad lines of `matplotlibrc` | [`config::user_rc_file`]`()?` and its `warnings` |
//! | `mpl.rcParamsOrig`                               | [`RcParams::startup`]                          |
//! | `mpl.rc_file_defaults()`, `mpl.rcdefaults()`     | [`rc::restore_startup`], [`rc::reset`]         |
//! | `style.USER_LIBRARY_PATHS`                       | [`style::user_library_paths`], [`style::set_user_library_paths`] |
//! | `style.reload_library()`                         | [`style::reload_library`]                      |
//! | `matplotlib.testing.setup()`                     | [`testing::setup`]                             |
//! | a test environment's own `MPLCONFIGDIR`          | [`config::ignore_user_config`] (which [`testing::setup`] calls) |
//! | `compare_images(expected, actual, tol)`          | [`testing::compare_images`], returning a [`Comparison`] |
//! | `calculate_rms(expected, actual)`                | [`testing::calculate_rms`]                     |
//! | `comparable_formats()`                           | [`testing::comparable_formats`]: PNG           |
//! | `@check_figures_equal(extensions=["png"], tol=0)` | [`testing::check_figures_equal`]`(path, tol, test, reference)` |
//! | `ImageComparisonFailure`                         | a [`Comparison`] that does not match, or a [`ComparisonError`] |
//!
//! # Configuration
//!
//! ```text
//! # ~/.config/plt-rs/pltrc, as ~/.config/matplotlib/matplotlibrc
//! lines.linewidth: 2
//! axes.grid: True
//! ```
//!
//! ```python
//! import matplotlib as mpl
//! print(mpl.get_configdir(), mpl.matplotlib_fname())
//! mpl.rcParams["lines.linewidth"] = 4
//! mpl.rc_file_defaults()   # back to what matplotlibrc says
//! mpl.rcdefaults()         # matplotlib's defaults
//! assert mpl.rcParams["lines.linewidth"] == 1.5
//! ```
//!
//! ```
//! use plt::{config, rc};
//!
//! let dir = config::config_dir()?;
//! assert!(dir.is_dir());
//! let rc_file: Option<std::path::PathBuf> = config::rc_file_path();
//! assert!(rc_file.is_none_or(|path| path.ends_with("pltrc")));
//! rc::set("lines.linewidth", "4")?;
//! rc::restore_startup(); // back to the look and what pltrc says
//! rc::reset(); // matplotlib's defaults
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
//! use plt::prelude::*;
//! use plt::testing;
//!
//! testing::setup();
//! let out = std::env::temp_dir().join(format!("plt-porting-testing-{}", std::process::id()));
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
//! // The test figure saved above stands in for a baseline image.
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
