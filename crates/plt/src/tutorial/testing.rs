//! Chapter 12: testing the figures your program draws.
//!
//! A figure is output like any other, and can be tested at two levels.
//! *Structural* tests read the figure back through its handles: which
//! artists it holds, their data and colours, the limits and tick labels
//! of its axes. They are fast, exact and independent of fonts and
//! rendering, and they are what most of a program's figure tests should
//! be. *Image* tests compare rendered pixels with a reference, for what
//! only the picture shows; [`testing`](crate::testing) has matplotlib's
//! tools for them.
//!
//! # Structural tests
//!
//! Keep figure building in functions that return the figure, and test
//! those functions. Every getter used in this tutorial's `assert!` lines
//! is available to your tests.
//!
//! ```
//! use std::error::Error;
//!
//! use plt::axes::PlotError;
//! use plt::legend::LegendOptions;
//! use plt::prelude::*;
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
//! Pixels depend on the look, the fonts and the user's `pltrc`, so image
//! tests must not depend on the machine running them.
//! [`testing::setup`](crate::testing::setup) sets the configuration
//! matplotlib's own tests run in: matplotlib's look and defaults, the
//! bundled `DejaVu Sans` font, no window system, and no user configuration.
//! The look and the configuration change for the whole test binary, the
//! settings for the calling thread: call it at the start of each test,
//! since tests run on threads of their own. Pin the figure size and resolution,
//! and use data that is the same on every run.
//!
//! # Comparing two figures
//!
//! Often the clearest reference is a second figure drawn another way.
//! [`testing::check_figures_equal`](crate::testing::check_figures_equal)
//! builds a test figure and a reference figure with two closures, saves
//! both as PNG files beside the path given and compares them. The
//! returned [`Comparison`](crate::testing::Comparison) says whether they
//! match within the tolerance, an RMS difference of pixel values (0 asks
//! for identical images); its `Display` is the report for a failed test.
//!
//! ```
//! use plt::prelude::*;
//! use plt::testing;
//!
//! testing::setup();
//! let out = std::env::temp_dir().join(format!("plt-tutorial-testing-{}", std::process::id()));
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
//! For a figure that should not change, save it once, check the image by
//! eye, and commit it with the tests as a *baseline*. Each run then saves
//! the figure again and compares it with
//! [`testing::compare_images`](crate::testing::compare_images). When they
//! differ, it writes an image of the differences beside the output,
//! named after it with `-failed-diff`, to look at.
//!
//! ```
//! use plt::prelude::*;
//! use plt::testing;
//!
//! fn sine() -> Result<Figure, Box<dyn std::error::Error>> {
//!     let fig = Figure::with_size(4.0, 3.0, 100.0);
//!     let x = linspace(0.0, 6.0, 50);
//!     fig.subplot().plot(&x, x.iter().map(|x| x.sin()).collect::<Vec<_>>())?;
//!     Ok(fig)
//! }
//!
//! testing::setup();
//! let dir = std::env::temp_dir().join(format!("plt-tutorial-baseline-{}", std::process::id()));
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
//! Only PNG images can be compared: plt-rs writes SVG, PDF and
//! PostScript but cannot turn them back into pixels. Test vector output
//! structurally instead, by looking for what the document must contain.
//! The [`porting::configuration`](crate::porting::configuration) page
//! maps these functions to `matplotlib.testing`.
//!
//! That is the end of the tutorial. The [crate overview](crate) lists
//! every module, and the [`porting`](crate::porting) guide has the
//! methods of each topic in tables.
