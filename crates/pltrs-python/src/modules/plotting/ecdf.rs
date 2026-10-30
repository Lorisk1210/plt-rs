//! Empirical cumulative distribution functions: `ecdf`.

use pltrs::axes::{Axes, EcdfError, EcdfOptions};
use pltrs::lines::Line2D;
use pltrs::orientation::Orientation;

use super::super::lines::LineKwargs;
use crate::binding::prelude::*;

impl From<EcdfError> for PyErr {
    /// matplotlib's `ValueError` for values it cannot sort, and its
    /// `TypeError` for a draw style.
    fn from(err: EcdfError) -> Self {
        match err {
            EcdfError::DrawStyle => PyErr::type_error(err.to_string()),
            err => PyErr::value_error(err.to_string()),
        }
    }
}

/// How the distribution is drawn: `(complementary, horizontal,
/// compress)`.
type EcdfLook = (bool, bool, bool);

/// The empirical cumulative distribution of `x`, weighted by `weights`
/// (matplotlib's `ecdf`), drawn as a step line styled by `line`.
pub(super) fn ecdf(
    axes: Axes,
    x: NdArray,
    weights: Option<NdArray>,
    (complementary, horizontal, compress): EcdfLook,
    line: LineKwargs,
) -> PyResult<Line2D> {
    let options = EcdfOptions {
        weights: weights.map(NdArray::into_vec).transpose()?,
        complementary,
        orientation: if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        },
        compress,
        line: line.0,
    };
    Ok(axes.ecdf(x.into_vec()?, options)?)
}
