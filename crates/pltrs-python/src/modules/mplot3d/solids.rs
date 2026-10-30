//! The 3-D plotting methods drawing boxes, arrows and triangulated
//! surfaces (matplotlib's `Axes3D.bar3d`, `quiver` and `plot_trisurf`),
//! whose arguments the Python side resolved as matplotlib does.

use pltrs::collections::{Collection, CollectionProps, EdgeColors};
use pltrs::colors::Color;
use pltrs::mplot3d::{
    Axes3D, Bar3DOptions, Boxes, FaceColors, Quiver3DOptions, TrisurfOptions, Vec3,
};
use pltrs::quiver::Pivot;
use pltrs::tri::Triangulation;

use super::super::collections::rgba_rows;
use super::super::plotting::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

/// The corners and the sizes of boxes, each one per box or one for all.
type PyBoxes = (
    (Vec<f64>, Vec<f64>, Vec<f64>),
    (Vec<f64>, Vec<f64>, Vec<f64>),
);

/// Boxes with corners at `(x, y, z)` and sizes `(dx, dy, dz)` (`bar3d`):
/// coloured `colors` (RGBA rows: one for all, one per box or per face;
/// none for the cycle colour), shaded when `shade`, hiding the points
/// outside the view limits when `axlim_clip`.
fn bar3d(
    axes: Axes3D,
    ((x, y, z), (dx, dy, dz)): PyBoxes,
    colors: Option<NdArray>,
    shade: bool,
    axlim_clip: bool,
) -> PyResult<Collection> {
    let options = Bar3DOptions {
        color: colors
            .as_ref()
            .map(rgba_rows)
            .transpose()?
            .unwrap_or_default(),
        shade,
        axlim_clip,
        ..Bar3DOptions::default()
    };
    Ok(axes.bar3d(Boxes::new(x, y, z, dx, dy, dz), options)?)
}

/// `(N, 3)` rows of an array.
fn rows3(array: &NdArray) -> Vec<Vec3> {
    array.data.as_chunks::<3>().0.to_vec()
}

/// How arrows are drawn: their length factor, head ratio, pivot
/// (`"tail"`, `"middle"`, `"tip"`) and whether vectors are normalised.
type ArrowShape = (f64, f64, String, bool);

/// Arrows from `positions` along `vectors` (`(N, 3)` arrays; `quiver`),
/// shaped as `shape` says, in `color` (one colour, or RGBA rows one per
/// line), hiding the points outside the view limits when `axlim_clip`.
fn quiver(
    axes: Axes3D,
    (positions, vectors): (NdArray, NdArray),
    (length, arrow_length_ratio, pivot, normalize): ArrowShape,
    (color, colors): (Option<Color>, Option<NdArray>),
    axlim_clip: bool,
) -> PyResult<Collection> {
    let pivot = match pivot.as_str() {
        "tail" => Pivot::Tail,
        "middle" => Pivot::Middle,
        "tip" => Pivot::Tip,
        _ => {
            return Err(PyErr::value_error(format!(
                "{pivot:?} is not a valid value for pivot; supported values are 'tail', \
                 'middle', 'tip'"
            )));
        }
    };
    let props = CollectionProps {
        color,
        edgecolors: colors
            .as_ref()
            .map(rgba_rows)
            .transpose()?
            .map(EdgeColors::Colors),
        ..CollectionProps::default()
    };
    let options = Quiver3DOptions {
        length,
        arrow_length_ratio,
        pivot,
        normalize,
        props,
        axlim_clip,
    };
    Ok(axes.quiver(&rows3(&positions), &rows3(&vectors), options)?)
}

/// A triangulated surface: the points `(x, y)`, their triangles (`None`:
/// Delaunay's) and the mask of the triangles not drawn.
type PyTriangulation = (
    Vec<f64>,
    Vec<f64>,
    Option<Vec<(usize, usize, usize)>>,
    Option<Vec<bool>>,
);

/// The surface of the triangulation lifted to `z` (`plot_trisurf`):
/// coloured `color`, or its heights through a colormap (`None` for the
/// cycle colour), shaded when `shade` (`None`: unless colour-mapped).
fn plot_trisurf(
    axes: Axes3D,
    (x, y, triangles, mask): PyTriangulation,
    z: Vec<f64>,
    (color, mapping, shade): (Option<Color>, Option<Mapping>, Option<bool>),
    axlim_clip: bool,
) -> PyResult<Collection> {
    let mut tri = match triangles {
        Some(triangles) => Triangulation::new(
            &x,
            &y,
            triangles.into_iter().map(|(a, b, c)| [a, b, c]).collect(),
        ),
        None => Triangulation::delaunay(&x, &y),
    }
    .map_err(|err| PyErr::value_error(err.to_string()))?;
    if let Some(mask) = mask {
        tri.set_mask(Some(mask))
            .map_err(|err| PyErr::value_error(err.to_string()))?;
    }
    let colors = match (mapping, color) {
        (Some(mapping), _) => FaceColors::from(colorizer(mapping)?),
        (None, Some(color)) => FaceColors::Single(color),
        (None, None) => FaceColors::Cycle,
    };
    let options = TrisurfOptions {
        colors,
        shade,
        axlim_clip,
        ..TrisurfOptions::default()
    };
    Ok(axes.plot_trisurf(&tri, &z, options)?)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("bar3d", bar3d)?;
    module.add("quiver", quiver)?;
    module.add("plot_trisurf", plot_trisurf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_of_three_are_points() {
        let array = NdArray::with_shape(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0], vec![2, 3]).unwrap();
        assert_eq!(rows3(&array), [[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]]);
    }
}
