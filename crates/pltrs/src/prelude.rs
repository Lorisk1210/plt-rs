//! The types and traits that most programs need.
//!
//! Import them all with one line:
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.plot_with(linspace(0.0, 1.0, 5), [0.0, 1.0, 0.0, 1.0, 0.0], LineProps::default())?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The prelude exports these names:
//!
//! | Group | Names |
//! |-------|-------|
//! | Figures and axes | [`Figure`], [`Axes`] |
//! | Artists | [`ArtistHandle`], the trait that gives every artist its common methods |
//! | Lines and markers | [`Line2D`], [`LineProps`], [`LineStyle`], [`DrawStyle`], [`MarkEvery`], [`Marker`], [`MarkerSymbol`], [`FillStyle`] |
//! | Patches | [`PatchProps`] |
//! | Scatter plots | [`ScatterOptions`], [`ScatterColors`], [`MappedColors`] |
//! | Bars and histograms | [`BarOptions`], [`BarAlign`], [`HistPlotOptions`] |
//! | Error bars and areas | [`ErrorbarOptions`], [`FillBetweenOptions`] |
//! | Images | [`ImshowOptions`] |
//! | Legends and colorbars | [`LegendOptions`], [`ColorbarOptions`] |
//! | Colors | [`Color`], [`colormaps`], [`Colorize`], the trait that gives the colormap builders |
//! | Data | [`linspace`], [`arange`], [`logspace`], [`meshgrid`], [`Grid`] |
//!
//! Other names are in their modules. The [crate documentation](crate) lists
//! the modules.

pub use crate::artist::ArtistHandle;
pub use crate::axes::{
    Axes, BarAlign, BarOptions, ErrorbarOptions, FillBetweenOptions, HistPlotOptions,
    ImshowOptions, MappedColors, ScatterColors, ScatterOptions,
};
pub use crate::colorbar::ColorbarOptions;
pub use crate::colors::{Color, Colorize, colormaps};
pub use crate::figure::Figure;
pub use crate::legend::LegendOptions;
pub use crate::lines::{DrawStyle, Line2D, LineProps, LineStyle, MarkEvery};
pub use crate::markers::{FillStyle, Marker, MarkerSymbol};
pub use crate::numeric::{Grid, arange, linspace, logspace, meshgrid};
pub use crate::patches::PatchProps;
