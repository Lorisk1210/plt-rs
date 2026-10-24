//! Why a projection could not be registered or axes made with it.

use std::fmt;

use crate::axes::AspectError;
use crate::figure::AxesRectError;
use crate::gridspec::GridSpecError;

/// Why a projection could not be registered, or axes made with it.
#[derive(Clone, Debug, PartialEq)]
pub enum ProjectionError {
    /// No projection is registered under this name on this thread.
    Unknown(String),
    /// A projection may not take the name of a built-in one.
    Builtin(&'static str),
    /// The options given are not of the type the projection is made with
    /// (matplotlib's `TypeError` for keywords its `__init__` does not
    /// take).
    Options {
        /// The name of the projection.
        projection: &'static str,
        /// The type of options it takes, `()` for none.
        expected: &'static str,
        /// The type of the options given.
        given: &'static str,
    },
    /// The projection could not set up the axes; they were removed.
    SetUp {
        /// The name of the projection.
        projection: &'static str,
        /// What went wrong.
        message: String,
    },
    /// The subplot does not fit its grid.
    Grid(GridSpecError),
    /// The axes rectangle is invalid.
    Rect(AxesRectError),
    /// Twinned axes adjust their data limits to an aspect, which a
    /// projection fixing its data ratio refuses (matplotlib's
    /// `set_adjustable('datalim')` raising in `twinx`).
    Aspect(AspectError),
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(f, "no projection is registered as {name:?}"),
            Self::Builtin(name) => write!(f, "{name:?} is the name of a built-in projection"),
            Self::Options {
                projection,
                expected,
                given,
            } => write!(
                f,
                "the {projection:?} projection is made with options of type {expected}, \
                 not {given}"
            ),
            Self::SetUp {
                projection,
                message,
            } => write!(
                f,
                "the {projection:?} projection could not set up its axes: {message}"
            ),
            Self::Grid(err) => err.fmt(f),
            Self::Rect(err) => err.fmt(f),
            Self::Aspect(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Grid(err) => Some(err),
            Self::Rect(err) => Some(err),
            Self::Aspect(err) => Some(err),
            Self::Unknown(_) | Self::Builtin(_) | Self::Options { .. } | Self::SetUp { .. } => None,
        }
    }
}

impl From<GridSpecError> for ProjectionError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

impl From<AxesRectError> for ProjectionError {
    fn from(err: AxesRectError) -> Self {
        Self::Rect(err)
    }
}

impl From<AspectError> for ProjectionError {
    fn from(err: AspectError) -> Self {
        Self::Aspect(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ProjectionKind;

    #[test]
    fn messages_name_the_projection() {
        assert_eq!(
            ProjectionError::Unknown("skewx".into()).to_string(),
            "no projection is registered as \"skewx\""
        );
        assert_eq!(
            ProjectionError::Builtin("polar").to_string(),
            "\"polar\" is the name of a built-in projection"
        );
        let set_up = ProjectionError::SetUp {
            projection: "skewx",
            message: "no".into(),
        };
        assert!(set_up.to_string().ends_with("its axes: no"));
        let options = ProjectionError::Options {
            projection: "skewx",
            expected: "()",
            given: "f64",
        };
        assert_eq!(
            options.to_string(),
            "the \"skewx\" projection is made with options of type (), not f64"
        );
        let grid = ProjectionError::from(GridSpecError::EmptyGrid);
        assert!(std::error::Error::source(&grid).is_some());
        let aspect = ProjectionError::from(AspectError::FixedDataRatio(ProjectionKind::Polar));
        assert!(aspect.to_string().contains("polar"));
    }
}
