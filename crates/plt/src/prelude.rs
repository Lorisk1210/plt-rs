//! The types and traits most programs need: `use plt::prelude::*;`.
//!
//! Figures, axes and the artists plotting returns; the option structs of
//! the common plot types (lines, scatter plots, bars, histograms, filled
//! areas, images); legends and colorbars; colours, colormaps and line
//! styles; and the numeric helpers that make data: ranges, meshes and
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
