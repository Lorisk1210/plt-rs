//! Lines: [`Line2D`] with its line styles, draw styles, markers and marker
//! subsampling (matplotlib.lines).

mod line;
mod linestyle;
mod markevery;
mod steps;

pub use line::Line2D;
pub(crate) use line::LineArtist;
pub use linestyle::{LineStyle, LineStyleError, inverse_dashes};
pub use markevery::{MarkEvery, MarkEveryError};
pub use steps::{
    DrawStyle, StepError, line_vertices, pts_to_midstep, pts_to_poststep, pts_to_prestep,
};
