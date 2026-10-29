//! The types and traits most programs need: `use pltrs::prelude::*;`.
//!
//! Figures, axes, lines and the [`ArtistHandle`] trait every artist's
//! common methods come from; the option structs of the common plot types
//! (lines, scatter plots, bars, histograms, filled areas, images); the
//! options of legends and colorbars; colours, colormaps, line styles and
//! markers; and the numeric helpers that make data: ranges, meshes and
//! [`Grid`]s.

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
