//! The types and traits most programs need: `use plt::prelude::*;`.

pub use crate::artist::ArtistHandle;
pub use crate::axes::{Axes, MappedColors, ScatterColors, ScatterOptions};
pub use crate::colors::Color;
pub use crate::figure::Figure;
pub use crate::lines::{DrawStyle, Line2D, LineProps, LineStyle, MarkEvery};
pub use crate::markers::{FillStyle, Marker, MarkerSymbol};
pub use crate::numeric::{arange, linspace, logspace, meshgrid};
