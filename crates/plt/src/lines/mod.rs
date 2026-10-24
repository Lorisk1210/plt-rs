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

pub(crate) use axline::InfiniteLine;
pub use axline::{AxLineError, AxLineThrough};
pub use format::{FormatString, FormatStringError};
pub use line::{Line2D, LineProps};
pub(crate) use line::{LineArtist, PolarMarkerRotation};
pub use line_markers::MarkerIndicesError;
pub use linestyle::{LineStyle, LineStyleError, inverse_dashes};
pub use markevery::{MarkEvery, MarkEveryError};
pub use plt_render::{CapStyle, JoinStyle};
pub use steps::{
    DrawStyle, StepError, StepWhere, line_vertices, pts_to_midstep, pts_to_poststep, pts_to_prestep,
};
