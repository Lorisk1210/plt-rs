//! Chapter 12: testing the figures that your program draws.
//!
//! A structural test reads the figure through its handles: artists, data,
//! colors, limits and tick labels. Structural tests are fast and exact, and
//! do not depend on fonts, so write most tests this way. An image test
//! compares the pixels with a reference. [`testing`](crate::testing) has
//! the matplotlib tools for image tests.
//!
//! # Structural tests
//!
//! Build figures in functions that return the figure, and test those
//! functions with getters, as the `assert!` lines of this tutorial do.
//!
//! ```
//! use std::error::Error;
//!
//! use pltrs::axes::PlotError;
//! use pltrs::prelude::*;
//!
//! /// Monthly sales as bars, with the target as a dashed line.
//! fn sales_chart(months: &[&str], sales: &[f64], target: f64) -> Result<Figure, Box<dyn Error>> {
//!     let fig = Figure::with_size(6.0, 4.0, 100.0);
//!     let ax = fig.subplot();
//!     ax.bar(months, sales, BarOptions { label: Some("sales".into()), ..BarOptions::default() })?;
//!     ax.axhline(target, LineProps::default().linestyle(LineStyle::Dashed).label("target"));
//!     ax.legend(LegendOptions::default())?;
//!     Ok(fig)
//! }
//!
//! // In a #[test] function:
//! let fig = sales_chart(&["jan", "feb", "mar"], &[12.0, 15.0, 9.0], 12.5)?;
//! let ax = &fig.axes()[0];
//! assert_eq!(ax.patches().len(), 3, "one bar per month");
//! assert_eq!(ax.lines()[0].ydata(), [12.5, 12.5]);
//! assert_eq!(ax.current_legend().ok_or("a legend")?.labels(), ["target", "sales"]);
//! assert_eq!(ax.xaxis().ticklabels(false)?, ["jan", "feb", "mar"]);
//!
//! // Bad input is an error to test for, not a panic.
//! let error = sales_chart(&["jan", "feb", "mar"], &[1.0, 2.0], 0.0).unwrap_err();
//! assert!(error.is::<PlotError>());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # A stable configuration
//!
//! Pixels depend on the look, the fonts and the `pltrsrc` of the user.
//! [`testing::setup`](crate::testing::setup) sets the configuration of the
//! matplotlib tests: the matplotlib look and defaults, the font
//! `DejaVu Sans`, no window system and no user configuration. It sets the
//! look and the configuration for the whole test binary, but the settings
//! only for the calling thread. Thus, call it at the start of each test. Use a fixed figure
//! size, resolution and data.
//!
//! # Comparing two figures
//!
//! The best reference is often a second figure that you draw in another
//! way. [`testing::check_figures_equal`](crate::testing::check_figures_equal)
//! builds two figures with two closures, saves them as PNG files next to
//! the given path and compares them. The returned
//! [`Comparison`](crate::testing::Comparison) tells whether they match
//! within the tolerance, an RMS difference of pixel values. A tolerance of
//! 0 requires identical images. Its `Display` is the failure report.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::testing;
//!
//! testing::setup();
//! let out = std::env::temp_dir().join(format!("pltrs-tutorial-testing-{}", std::process::id()));
//! let comparison = testing::check_figures_equal(
//!     out.join("first_cycle_colour.png"),
//!     0.0,
//!     |fig| {
//!         fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
//!         Ok(())
//!     },
//!     |fig| {
//!         fig.subplot().plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().color("C0"))?;
//!         Ok(())
//!     },
//! )?;
//! assert!(comparison.matches(), "{comparison}");
//! # std::fs::remove_dir_all(&out)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Comparing with a baseline image
//!
//! If a figure must not change, save it once, check it and commit it as a
//! *baseline*. Each test run saves the figure again and compares it with
//! [`testing::compare_images`](crate::testing::compare_images). If they
//! differ, it writes an image of the differences next to the output, with
//! `-failed-diff` in the name.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::testing;
//!
//! fn sine() -> Result<Figure, Box<dyn std::error::Error>> {
//!     let fig = Figure::with_size(4.0, 3.0, 100.0);
//!     let x = linspace(0.0, 6.0, 50);
//!     fig.subplot().plot(&x, x.iter().map(|x| x.sin()))?;
//!     Ok(fig)
//! }
//!
//! testing::setup();
//! let dir = std::env::temp_dir().join(format!("pltrs-tutorial-baseline-{}", std::process::id()));
//! std::fs::create_dir_all(&dir)?;
//! let baseline = dir.join("sine-expected.png"); // in your repository, e.g. tests/baseline/
//! sine()?.savefig(&baseline)?; // once, when the figure is right
//!
//! let output = dir.join("sine.png");
//! sine()?.savefig(&output)?; // on every run
//! let comparison = testing::compare_images(&baseline, &output, 0.0)?;
//! assert!(comparison.matches(), "{comparison}");
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! pltrs compares only PNG images, because it cannot render SVG, PDF or
//! PostScript back into pixels. Test vector output for the content that the
//! document must have. [`porting::configuration`](crate::porting::configuration)
//! maps these functions to `matplotlib.testing`.
//!
//! This is the end of the tutorial. The [crate overview](crate) lists all
//! modules, and the [`porting`](crate::porting) guide has tables of the
//! methods for each topic.
