//! Errors of the pyplot functions that go beyond the methods they call.

use std::fmt;

use crate::axes::PlotError;
use crate::colorbar::ColorbarError;
use crate::colors::ColormapError;
use crate::figure::MosaicError;
use crate::gridspec::GridSpecError;

/// Why a pyplot function cannot act on the current figure or axes.
#[derive(Clone, Debug, PartialEq)]
pub enum PyplotError {
    /// No image or other colour-mapped artist was made current in the
    /// current figure (`colorbar`, `clim`).
    NoCurrentImage,
    /// The current axes is not polar (`polar`, `rgrids`, `thetagrids`).
    NotPolar,
    /// A function making figures was given the number of an open figure
    /// without being asked to clear it (`subplots_num`,
    /// `subplot_mosaic_num`).
    FigureExists(u32),
    /// An unknown colormap.
    Colormap(ColormapError),
    /// The colorbar cannot be made.
    Colorbar(ColorbarError),
    /// Invalid data or options for the plotting method.
    Plot(PlotError),
    /// An invalid grid of subplots.
    Grid(GridSpecError),
    /// An invalid mosaic of subplots.
    Mosaic(MosaicError),
}

impl fmt::Display for PyplotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCurrentImage => f.write_str(
                "no mappable was found: first define one such as an image (with imshow) \
                 or a contour set (with contourf)",
            ),
            Self::NotPolar => f.write_str("the current axes is not polar"),
            Self::FigureExists(num) => write!(
                f,
                "figure {num} already exists: clear it, or add to it through figure_num({num})"
            ),
            Self::Colormap(err) => err.fmt(f),
            Self::Colorbar(err) => err.fmt(f),
            Self::Plot(err) => err.fmt(f),
            Self::Grid(err) => err.fmt(f),
            Self::Mosaic(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for PyplotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colormap(err) => Some(err),
            Self::Colorbar(err) => Some(err),
            Self::Plot(err) => Some(err),
            Self::Grid(err) => Some(err),
            Self::Mosaic(err) => Some(err),
            Self::NoCurrentImage | Self::NotPolar | Self::FigureExists(_) => None,
        }
    }
}

impl From<ColormapError> for PyplotError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

impl From<ColorbarError> for PyplotError {
    fn from(err: ColorbarError) -> Self {
        Self::Colorbar(err)
    }
}

impl From<PlotError> for PyplotError {
    fn from(err: PlotError) -> Self {
        Self::Plot(err)
    }
}

impl From<GridSpecError> for PyplotError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

impl From<MosaicError> for PyplotError {
    fn from(err: MosaicError) -> Self {
        Self::Mosaic(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn errors_explain_themselves() {
        assert!(PyplotError::NoCurrentImage.to_string().contains("imshow"));
        assert!(PyplotError::NotPolar.source().is_none());
        let wrapped = PyplotError::from(PlotError::NotInAxes);
        assert_eq!(wrapped.to_string(), PlotError::NotInAxes.to_string());
        assert!(wrapped.source().is_some());
        assert!(
            PyplotError::FigureExists(3)
                .to_string()
                .contains("figure 3")
        );
        assert!(
            PyplotError::from(GridSpecError::EmptyGrid)
                .source()
                .is_some()
        );
    }
}
