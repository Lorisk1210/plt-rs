//! Errors of the pyplot functions that go beyond the methods they call.

use std::fmt;

use crate::axes::PlotError;
use crate::colorbar::ColorbarError;
use crate::colors::ColormapError;

/// Why a pyplot function cannot act on the current figure or axes.
#[derive(Clone, Debug, PartialEq)]
pub enum PyplotError {
    /// No image or other colour-mapped artist was made current in the
    /// current figure (`colorbar`, `clim`).
    NoCurrentImage,
    /// The current image is a contour set, whose colours are fixed when it
    /// is made (`clim`, `set_cmap` and the colormap shortcuts).
    FixedContourColors,
    /// The current axes is not polar (`polar`, `rgrids`, `thetagrids`).
    NotPolar,
    /// An unknown colormap.
    Colormap(ColormapError),
    /// The colorbar cannot be made.
    Colorbar(ColorbarError),
    /// Invalid data or options for the plotting method.
    Plot(PlotError),
}

impl fmt::Display for PyplotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCurrentImage => f.write_str(
                "no mappable was found: first define one such as an image (with imshow) \
                 or a contour set (with contourf)",
            ),
            Self::FixedContourColors => f.write_str(
                "the current image is a contour set, whose colormap and limits cannot change",
            ),
            Self::NotPolar => f.write_str("the current axes is not polar"),
            Self::Colormap(err) => err.fmt(f),
            Self::Colorbar(err) => err.fmt(f),
            Self::Plot(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for PyplotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colormap(err) => Some(err),
            Self::Colorbar(err) => Some(err),
            Self::Plot(err) => Some(err),
            Self::NoCurrentImage | Self::FixedContourColors | Self::NotPolar => None,
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
    }
}
