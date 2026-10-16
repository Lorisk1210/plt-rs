//! The coordinate system of an axes (matplotlib's projections).

use super::{Axes, AxesState};
use crate::polar::PolarState;

/// The kind of coordinate system an axes draws in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectionKind {
    /// Cartesian x and y.
    #[default]
    Rectilinear,
    /// Angle and radius (see [`crate::polar`]).
    Polar,
}

impl ProjectionKind {
    /// matplotlib's projection name (`ax.name`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rectilinear => "rectilinear",
            Self::Polar => "polar",
        }
    }
}

/// The projection of an axes with its settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Projection {
    /// Cartesian x and y.
    #[default]
    Rectilinear,
    /// Angle and radius.
    Polar(PolarState),
}

impl Projection {
    /// The kind of projection.
    pub(crate) fn kind(&self) -> ProjectionKind {
        match self {
            Self::Rectilinear => ProjectionKind::Rectilinear,
            Self::Polar(_) => ProjectionKind::Polar,
        }
    }
}

impl Axes {
    /// The kind of coordinate system this axes draws in.
    pub fn projection(&self) -> ProjectionKind {
        self.raw.with::<AxesState, _>(|axes| axes.projection.kind())
    }
}
