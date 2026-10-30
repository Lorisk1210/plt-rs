//! The error matplotlib raises while drawing a line whose `markevery`
//! does not fit it (`_mark_every_path`): pltrs (Rust) reports the line
//! among the artists it could not draw, and the selection, asked again,
//! tells why.

use pltrs::artist::{ArtistHandle, FigureRef};
use pltrs::figure::{DrawErrors, Figure};
use pltrs::lines::{Line2D, MarkerIndicesError};

use crate::binding::prelude::*;

/// matplotlib's `ValueError` for the first of the artists of `figure`
/// that `problems` lists whose `markevery` selection does not fit it, if
/// any.
pub(in crate::modules) fn markevery_error(figure: &Figure, problems: &DrawErrors) -> Option<PyErr> {
    let figure = FigureRef::of(figure.raw());
    problems.0.iter().find_map(|problem| {
        let line = figure.handle(problem.artist).downcast::<Line2D>()?;
        match line.marker_indices() {
            Err(MarkerIndicesError::MarkEvery(err)) => Some(err.into()),
            _ => None,
        }
    })
}
