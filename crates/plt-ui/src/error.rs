//! The crate-wide error type.

use std::fmt;

use plt_raster::RasterError;
use plt_text::TextError;

/// Everything that can go wrong while laying out, painting or showing a UI.
#[derive(Debug)]
#[non_exhaustive]
pub enum UiError {
    /// Painting into the framebuffer failed.
    Raster(RasterError),
    /// Text could not be laid out (e.g. no usable font).
    Text(TextError),
    /// The windowing system reported an error.
    Platform(plt_platform::Error),
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raster(error) => write!(f, "painting failed: {error}"),
            Self::Text(error) => write!(f, "text layout failed: {error}"),
            Self::Platform(error) => write!(f, "windowing failed: {error}"),
        }
    }
}

impl std::error::Error for UiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Raster(error) => Some(error),
            Self::Text(error) => Some(error),
            Self::Platform(error) => Some(error),
        }
    }
}

impl From<RasterError> for UiError {
    fn from(error: RasterError) -> Self {
        Self::Raster(error)
    }
}

impl From<TextError> for UiError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}

impl From<plt_platform::Error> for UiError {
    fn from(error: plt_platform::Error) -> Self {
        Self::Platform(error)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn wraps_and_describes_sources() {
        let error = UiError::from(plt_platform::Error::EventLoopClosed);
        assert!(error.to_string().starts_with("windowing failed"));
        assert!(error.source().is_some());
    }
}
