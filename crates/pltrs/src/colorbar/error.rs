//! Errors of colorbar creation.

use std::fmt;

use crate::axes::PlotError;
use crate::colors::{ColormapError, NormError};
use crate::figure::AxesRectError;
use crate::gridspec::GridSpecError;
use crate::image::ImageError;
use crate::orientation::Orientation;
use crate::spines::SpineSide;
use crate::ticker::TickerError;

/// Invalid options given to [`Figure::colorbar`](crate::figure::Figure::colorbar)
/// or a colorbar method.
#[derive(Clone, Debug, PartialEq)]
pub enum ColorbarError {
    /// No axes to draw in or steal space from: the mappable is not drawn
    /// in an axes and neither parents nor `cax` were given.
    NoAxes,
    /// The parent axes belong to different figures.
    MixedFigures,
    /// The parent axes whose grid cell a colorbar was to take space from
    /// is on no grid.
    NoSubplotSpec,
    /// `location` and `orientation` disagree.
    LocationOrientation {
        /// The requested location.
        location: SpineSide,
        /// The requested orientation.
        orientation: Orientation,
    },
    /// The tick location is not a side along the colorbar.
    TickLocation {
        /// The requested tick location.
        ticklocation: SpineSide,
        /// The colorbar's orientation.
        orientation: Orientation,
    },
    /// An extension fraction is NaN.
    InvalidExtendFrac,
    /// Explicit values without boundaries need at least two entries.
    TooFewValues(usize),
    /// A colorbar needs at least two block edges inside its extensions.
    TooFewBoundaries(usize),
    /// Lines were requested from a filled contour set.
    FilledContours,
    /// The contour set maps no colours (no levels or no colours given).
    NoColors,
    /// The mappable's artist is not in its figure's state, drawn or
    /// removed.
    RemovedMappable,
    /// The default colormap is unknown.
    Colormap(ColormapError),
    /// The norm could not map the colorbar's values.
    Norm(NormError),
    /// The parent's grid cell could not be subdivided.
    GridSpec(GridSpecError),
    /// The colour mesh could not be created.
    Image(ImageError),
    /// An invalid tick format.
    Ticker(TickerError),
    /// An axis could not be configured.
    Plot(PlotError),
    /// The space taken for the colorbar is not a valid axes rectangle
    /// (`fraction`, `shrink` or `pad` not finite, or `shrink` negative).
    Rect(AxesRectError),
}

impl fmt::Display for ColorbarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAxes => f.write_str(
                "unable to determine the axes to steal space for the colorbar: give a cax, \
                 parent axes, or a mappable drawn in an axes",
            ),
            Self::MixedFigures => f.write_str(
                "unable to create a colorbar axes as not all parents share the same figure",
            ),
            Self::NoSubplotSpec => f.write_str("the parent axes is on no grid"),
            Self::LocationOrientation {
                location,
                orientation,
            } => write!(
                f,
                "location {:?} and orientation {orientation:?} are mutually exclusive",
                location.name()
            ),
            Self::TickLocation {
                ticklocation,
                orientation,
            } => write!(
                f,
                "ticklocation {:?} does not fit a {orientation:?} colorbar",
                ticklocation.name()
            ),
            Self::InvalidExtendFrac => f.write_str("invalid value for extendfrac"),
            Self::TooFewValues(n) => write!(
                f,
                "at least 2 values are needed to derive the boundaries, got {n}"
            ),
            Self::TooFewBoundaries(n) => write!(
                f,
                "at least 2 boundaries inside the extensions are needed, got {n}"
            ),
            Self::FilledContours => f.write_str(
                "if a single artist is passed to add_lines, it must be a ContourSet of lines",
            ),
            Self::NoColors => f.write_str("the contour set maps no colours"),
            Self::RemovedMappable => f.write_str("the mappable was removed from its figure"),
            Self::Colormap(err) => write!(f, "{err}"),
            Self::Norm(err) => write!(f, "{err}"),
            Self::GridSpec(err) => write!(f, "{err}"),
            Self::Image(err) => write!(f, "{err}"),
            Self::Ticker(err) => write!(f, "{err}"),
            Self::Plot(err) => write!(f, "{err}"),
            Self::Rect(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ColorbarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colormap(err) => Some(err),
            Self::Norm(err) => Some(err),
            Self::GridSpec(err) => Some(err),
            Self::Image(err) => Some(err),
            Self::Ticker(err) => Some(err),
            Self::Plot(err) => Some(err),
            Self::Rect(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ColormapError> for ColorbarError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

impl From<NormError> for ColorbarError {
    fn from(err: NormError) -> Self {
        Self::Norm(err)
    }
}

impl From<GridSpecError> for ColorbarError {
    fn from(err: GridSpecError) -> Self {
        Self::GridSpec(err)
    }
}

impl From<ImageError> for ColorbarError {
    fn from(err: ImageError) -> Self {
        Self::Image(err)
    }
}

impl From<TickerError> for ColorbarError {
    fn from(err: TickerError) -> Self {
        Self::Ticker(err)
    }
}

impl From<PlotError> for ColorbarError {
    fn from(err: PlotError) -> Self {
        Self::Plot(err)
    }
}

impl From<AxesRectError> for ColorbarError {
    fn from(err: AxesRectError) -> Self {
        Self::Rect(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_problem() {
        let err = ColorbarError::LocationOrientation {
            location: SpineSide::Top,
            orientation: Orientation::Vertical,
        };
        assert!(err.to_string().contains("\"top\""));
        assert!(ColorbarError::TooFewValues(1).to_string().contains("got 1"));
        let norm = ColorbarError::from(NormError::Unscaled);
        assert!(std::error::Error::source(&norm).is_some());
        assert!(std::error::Error::source(&ColorbarError::NoAxes).is_none());
    }
}
