//! [`ArrayError`]: why a collection refuses the values it is given to map
//! to colours.

use std::fmt;

use crate::colors::ColormapError;

/// Why [`Collection::set_array`](super::Collection::set_array) refuses
/// values.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayError {
    /// The collection maps no colours yet and `image.cmap` names no
    /// colormap.
    Colormap(ColormapError),
    /// A quad mesh takes one value per cell (flat shading) or per vertex
    /// (Gouraud shading).
    MeshSize {
        /// The number of cells or vertices.
        expected: usize,
        /// The number of values given.
        actual: usize,
    },
}

impl fmt::Display for ArrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Colormap(err) => err.fmt(f),
            Self::MeshSize { expected, actual } => write!(
                f,
                "the mesh takes {expected} values (one per cell, or per vertex when \
                 Gouraud-shaded), not {actual}"
            ),
        }
    }
}

impl std::error::Error for ArrayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colormap(err) => Some(err),
            Self::MeshSize { .. } => None,
        }
    }
}

impl From<ColormapError> for ArrayError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}
