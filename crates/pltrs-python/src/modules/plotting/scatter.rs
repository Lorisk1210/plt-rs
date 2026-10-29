//! `scatter`.

use pltrs::axes::{Axes, MappedColors, ScatterColors, ScatterOptions};
use pltrs::collections::Collection;
use pltrs::colors::{Alpha, Color};

use super::super::collections::{edge_colors, rgba_rows};
use super::super::lines::PyMarker;
use super::mapping::{Mapping, colorizer};
use super::values::PyAxisValues;
use crate::binding::prelude::*;

/// The face colours: RGBA rows, or values mapped through a colormap.
type Faces = (Option<NdArray>, Option<(NdArray, Mapping)>);

/// The opacity: one for all markers, or one each.
type Opacity = (Option<f64>, Option<Vec<f64>>);

/// The next colour of the patch property cycle (matplotlib's
/// `_get_patches_for_fill.get_next_color`).
pub(super) fn next_patch_color(axes: Axes) -> PyResult<Color> {
    Ok(axes.next_patch_color())
}

/// A scatter plot of `y` against `x` (matplotlib's `scatter`, whose
/// colour arguments the Python side resolved with
/// `_parse_scatter_color_args`): marker areas in points², the face
/// colours, the marker, the edge colours (`None` for `scatter.edgecolors`)
/// and widths, the opacity, and whether non-finite mapped values are drawn
/// in the colormap's bad colour.
pub(super) fn scatter(
    axes: Axes,
    (x, y): (PyAxisValues, PyAxisValues),
    sizes: NdArray,
    (rgba, mapped): Faces,
    marker: PyMarker,
    (edgecolors, linewidths): (Owned, Option<Vec<f64>>),
    (alpha, alphas): Opacity,
    plot_nonfinite: bool,
) -> PyResult<Collection> {
    let colors = match (rgba, mapped) {
        (Some(rgba), _) => ScatterColors::PerPoint(rgba_rows(&rgba)?),
        (None, Some((values, mapping))) => MappedColors {
            values: values.into_vec()?,
            colorizer: colorizer(mapping)?,
        }
        .into(),
        (None, None) => ScatterColors::Cycle,
    };
    let edgecolors = Python::with_gil(|py| {
        let edges = edgecolors.bind(py);
        if edges.is_none() {
            Ok(None)
        } else {
            edge_colors(&edges).map(Some)
        }
    })?;
    let options = ScatterOptions {
        sizes: Some(sizes.into_vec()?),
        colors,
        marker: Some(marker.style()?),
        edgecolors,
        linewidths,
        alpha: match (alpha, alphas) {
            (_, Some(alphas)) => Alpha::Each(alphas),
            (Some(alpha), None) => Alpha::Uniform(alpha),
            (None, None) => Alpha::Keep,
        },
        plot_nonfinite,
        offset_transform: None,
    };
    Ok(axes.scatter_with(x.0, y.0, options)?)
}
