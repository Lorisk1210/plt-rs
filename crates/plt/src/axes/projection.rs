//! The coordinate system of an axes (matplotlib's projections).

use super::{Axes, AxesState};
use crate::geo::GeoState;
use crate::polar::PolarState;

/// The kind of coordinate system an axes draws in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectionKind {
    /// Cartesian x and y.
    #[default]
    Rectilinear,
    /// Angle and radius (see [`crate::polar`]).
    Polar,
    /// Longitude and latitude through the Aitoff projection (see
    /// [`crate::geo`]).
    Aitoff,
    /// Longitude and latitude through the Hammer projection.
    Hammer,
    /// Longitude and latitude through the Lambert azimuthal equal-area
    /// projection.
    Lambert,
    /// Longitude and latitude through the Mollweide projection.
    Mollweide,
}

impl ProjectionKind {
    /// matplotlib's projection name (`ax.name`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rectilinear => "rectilinear",
            Self::Polar => "polar",
            Self::Aitoff => "aitoff",
            Self::Hammer => "hammer",
            Self::Lambert => "lambert",
            Self::Mollweide => "mollweide",
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
    /// Longitude and latitude through a map projection.
    Geo(GeoState),
}

impl Projection {
    /// The kind of projection.
    pub(crate) fn kind(&self) -> ProjectionKind {
        match self {
            Self::Rectilinear => ProjectionKind::Rectilinear,
            Self::Polar(_) => ProjectionKind::Polar,
            Self::Geo(geo) => geo.projection.kind(),
        }
    }

    /// Whether the axes is a rectangle framed by side spines; curved
    /// projections draw their own outline, keep a data ratio of 1 and
    /// clip to their patch.
    pub(crate) fn is_rectilinear(&self) -> bool {
        matches!(self, Self::Rectilinear)
    }
}

impl Axes {
    /// The kind of coordinate system this axes draws in.
    pub fn projection(&self) -> ProjectionKind {
        self.raw.with::<AxesState, _>(|axes| axes.projection.kind())
    }
}
