//! Lines: [`Line2D`] with its line styles, draw styles, markers and marker
//! subsampling (matplotlib.lines).

mod axline;
mod edge;
mod format;
mod hit_region;
mod line;
mod line_markers;
mod linestyle;
mod markevery;
mod steps;
mod subslice;

pub use axline::AxLineThrough;
pub(crate) use axline::InfiniteLine;
pub use format::{FormatError, FormatString};
pub use line::{Line2D, LineProps};
pub(crate) use line::{LineArtist, PolarMarkerRotation};
pub use line_markers::MarkerError;
pub use linestyle::{LineStyle, LineStyleError, inverse_dashes};
pub use markevery::{MarkEvery, MarkEveryError};
pub use steps::{
    DrawStyle, StepError, StepWhere, line_vertices, pts_to_midstep, pts_to_poststep, pts_to_prestep,
};
