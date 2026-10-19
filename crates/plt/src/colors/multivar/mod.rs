//! Colormaps of several variates (matplotlib's `MultivarColormap`,
//! `BivarColormap` and the `multivar_colormaps` and `bivar_colormaps`
//! registries).
//!
//! * A [`MultivarColormap`] holds one ordinary colormap per variate and
//!   mixes their colours, adding them (brighter) or their complements
//!   (darker): `2VarAddA`, `2VarSubA` and `3VarAddA`.
//! * A [`BivarColormap`] looks colours up from value pairs in a 2-D table:
//!   `BiPeak`, `BiOrangeBlue` and `BiCone`.
//! * A [`MultiColormap`] is either kind, and a [`MultiMappable`] maps data
//!   of several variates through a [`MultiNorm`](crate::colors::MultiNorm)
//!   and one.
//!
//! Values are given as [`Normalized`](crate::colors::Normalized): fractions
//! in `[0, 1]`, indices into the tables, or masked.

mod bivariate;
mod colormap;
mod data;
mod mappable;
mod multivariate;

use std::fmt;

pub use bivariate::{BivarAxis, BivarColormap, BivarShape};
pub use colormap::MultiColormap;
pub use mappable::MultiMappable;
pub use multivariate::{CombinationMode, MultivarColormap};

use crate::colors::{ColorError, Colormap, ColormapError, NormError};
use crate::numeric::Grid;

/// Why a multivariate colormap could not be made or used.
#[derive(Clone, Debug, PartialEq)]
pub enum MultivarError {
    /// A multivariate colormap needs at least two colormaps.
    TooFewColormaps(usize),
    /// The number of values (or colours, or sizes) does not match the
    /// number of variates.
    VariateCount {
        /// The number of variates.
        expected: usize,
        /// The number given.
        given: usize,
    },
    /// A bivariate shape name other than `square`, `circle`, `ignore` and
    /// `circleignore`.
    UnknownShape(String),
    /// Circular bivariate colormaps take fractions only.
    CircleNeedsFractions,
    /// A table needs at least one entry.
    EmptyTable,
    /// No colormap is registered under this name.
    Unknown(String),
    /// A colour is invalid.
    Color(ColorError),
    /// A component colormap could not be made.
    Colormap(ColormapError),
    /// The norms cannot map the data.
    Norm(NormError),
    /// The number of norms differs from the number of variates.
    NormCount {
        /// The variates of the colormap.
        variates: usize,
        /// The norms given.
        norms: usize,
    },
    /// The variates of the data have different lengths.
    Lengths {
        /// The length of the first variate.
        first: usize,
        /// The length of another one.
        other: usize,
    },
}

impl fmt::Display for MultivarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewColormaps(n) => {
                write!(
                    f,
                    "a multivariate colormap needs at least two colormaps, got {n}"
                )
            }
            Self::VariateCount { expected, given } => {
                write!(f, "expected {expected} variates, got {given}")
            }
            Self::UnknownShape(name) => write!(f, "{name:?} is not a bivariate colormap shape"),
            Self::CircleNeedsFractions => {
                f.write_str("circular bivariate colormaps are only implemented for fractions")
            }
            Self::EmptyTable => f.write_str("a colormap table needs at least one entry"),
            Self::Unknown(name) => write!(f, "no colormap is registered as {name:?}"),
            Self::Color(err) => err.fmt(f),
            Self::Colormap(err) => err.fmt(f),
            Self::Norm(err) => err.fmt(f),
            Self::NormCount { variates, norms } => write!(
                f,
                "a colormap of {variates} variates needs {variates} norms, got {norms}"
            ),
            Self::Lengths { first, other } => write!(
                f,
                "every variate needs the same number of values, got {first} and {other}"
            ),
        }
    }
}

impl std::error::Error for MultivarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Color(err) => Some(err),
            Self::Colormap(err) => Some(err),
            Self::Norm(err) => Some(err),
            _ => None,
        }
    }
}

impl From<NormError> for MultivarError {
    fn from(err: NormError) -> Self {
        Self::Norm(err)
    }
}

impl From<ColorError> for MultivarError {
    fn from(err: ColorError) -> Self {
        Self::Color(err)
    }
}

impl From<ColormapError> for MultivarError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

/// The names of the registered multivariate colormaps (matplotlib's
/// `multivar_colormaps`).
pub fn multivar_colormap_names() -> [&'static str; 3] {
    ["2VarAddA", "2VarSubA", "3VarAddA"]
}

/// The registered multivariate colormap `name`.
pub fn multivar_colormap(name: &str) -> Result<MultivarColormap, MultivarError> {
    let (components, mode): (&[(&str, &[f64])], _) = match name {
        "2VarAddA" => (
            &[
                ("2VarAddA0", data::VAR2_ADDA0),
                ("2VarAddA1", data::VAR2_ADDA1),
            ],
            CombinationMode::Add,
        ),
        "2VarSubA" => (
            &[
                ("2VarSubA0", data::VAR2_SUBA0),
                ("2VarSubA1", data::VAR2_SUBA1),
            ],
            CombinationMode::Sub,
        ),
        "3VarAddA" => (
            &[
                ("3VarAddA0", data::VAR3_ADDA0),
                ("3VarAddA1", data::VAR3_ADDA1),
                ("3VarAddA2", data::VAR3_ADDA2),
            ],
            CombinationMode::Add,
        ),
        other => return Err(MultivarError::Unknown(other.to_owned())),
    };
    let lutsize =
        usize::try_from(crate::rc::params().image_lut).map_err(|_| ColormapError::ZeroEntries)?;
    let colormaps = components
        .iter()
        .map(|(component, colors)| {
            let colors: Vec<_> = rgb(colors).into_iter().map(plt_core::Rgba::from).collect();
            Colormap::from_list(*component, colors, lutsize, 1.0)
        })
        .collect::<Result<_, _>>()?;
    MultivarColormap::new(name, colormaps, mode)
}

/// The names of the registered bivariate colormaps (matplotlib's
/// `bivar_colormaps`).
pub fn bivar_colormap_names() -> [&'static str; 3] {
    ["BiPeak", "BiOrangeBlue", "BiCone"]
}

/// The registered bivariate colormap `name`.
pub fn bivar_colormap(name: &str) -> Result<BivarColormap, MultivarError> {
    let (patch, side, shape, origin) = match name {
        "BiPeak" => (data::BI_PEAK, 65, BivarShape::Square, (0.5, 0.5)),
        "BiOrangeBlue" => (data::BI_ORANGE_BLUE, 2, BivarShape::Square, (0.0, 0.0)),
        "BiCone" => (data::BI_PEAK, 65, BivarShape::Circle, (0.5, 0.5)),
        other => return Err(MultivarError::Unknown(other.to_owned())),
    };
    let rows: Vec<Vec<[f64; 3]>> = rgb(patch)
        .chunks(side)
        .map(|row| row.iter().map(|&[r, g, b, _]| [r, g, b]).collect())
        .collect();
    let patch = Grid::from_rows(rows).map_err(|_| MultivarError::EmptyTable)?;
    BivarColormap::segmented(name, &patch, 256, shape, origin)
}

/// Opaque colours from consecutive RGB triples.
fn rgb(values: &[f64]) -> Vec<[f64; 4]> {
    values
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[r, g, b]| [r, g, b, 1.0])
        .collect()
}

#[cfg(test)]
mod tests;
