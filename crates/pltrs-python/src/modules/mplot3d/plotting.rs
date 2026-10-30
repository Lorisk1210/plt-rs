//! The 3-D plotting methods (matplotlib's `Axes3D.plot`, `scatter`,
//! `plot_surface`, `plot_wireframe` and `text`), whose arguments the
//! Python side resolved as matplotlib does.

use pltrs::axes::{MappedColors, ScatterColors, ScatterOptions};
use pltrs::collections::{Collection, CollectionProps};
use pltrs::colors::{Alpha, Color};
use pltrs::lines::{Line2D, LineProps};
use pltrs::mplot3d::{
    Axes3D, FaceColors, Sampling, Scatter3DOptions, SurfaceOptions, WireframeOptions,
};
use pltrs::numeric::Grid;
use pltrs::text::Text;

use super::super::collections::{edge_colors, rgba_rows};
use super::super::lines::{LineKwargs, PyMarker};
use super::super::plotting::mapping::{Mapping, colorizer};
use super::axis3;
use crate::binding::prelude::*;

/// A 2-D float array as a grid; `name` names it in the error.
fn grid(array: &NdArray, name: &str) -> PyResult<Grid<f64>> {
    match array.shape[..] {
        [rows, cols] => Grid::from_vec(rows, cols, array.data.clone())
            .map_err(|err| PyErr::value_error(err.to_string())),
        _ => Err(PyErr::value_error(format!(
            "Argument {name} must be 2-dimensional."
        ))),
    }
}

/// A line through the points `(x, y, z)`, `z` measured along `zdir`,
/// styled by the format string and the keyword arguments on top
/// (`plot`); `axlim_clip` hides the points outside the view limits.
fn plot(
    axes: Axes3D,
    (x, y, z): (NdArray, NdArray, NdArray),
    zdir: String,
    fmt: Option<String>,
    kwargs: LineKwargs,
    axlim_clip: bool,
) -> PyResult<Line2D> {
    let base = match fmt {
        Some(fmt) => LineProps::fmt(&fmt)?,
        None => LineProps::default(),
    };
    let line = axes.plot_with(
        x.into_vec()?,
        y.into_vec()?,
        z.into_vec()?,
        axis3(&zdir)?,
        base.overlay(kwargs.0),
    )?;
    if axlim_clip {
        axes.set_axlim_clip(&line, true)?;
    }
    Ok(line)
}

/// The face colours of a scatter plot: RGBA rows, or values mapped
/// through a colormap.
type Faces = (Option<NdArray>, Option<(NdArray, Mapping)>);

/// The edges of a scatter plot: their colours (`None` for
/// `scatter.edgecolors`) and widths.
type Edges = (Owned, Option<Vec<f64>>);

/// The opacity of a scatter plot: one for all markers, or one each.
type Opacity = (Option<f64>, Option<Vec<f64>>);

/// The depth settings of a 3-D scatter plot: `zdir`, `depthshade` and
/// `depthshade_minalpha` (`None`: the `axes3d.*` rcParams) and
/// `axlim_clip`.
type Depth = (String, Option<bool>, Option<f64>, bool);

/// A scatter plot of the points `(x, y, z)` (`scatter`): marker areas in
/// points² and marker, faces, edges and opacity as the 2-D `scatter`
/// takes them, then the depth settings.
fn scatter(
    axes: Axes3D,
    (x, y, z): (NdArray, NdArray, NdArray),
    (sizes, marker): (NdArray, PyMarker),
    (rgba, mapped): Faces,
    (edgecolors, linewidths): Edges,
    (alpha, alphas): Opacity,
    (zdir, depthshade, depthshade_minalpha, axlim_clip): Depth,
) -> PyResult<Collection> {
    let colors = match (rgba, mapped) {
        (Some(rgba), _) => ScatterColors::PerPoint(rgba_rows(&rgba)?),
        (None, Some((values, mapping))) => MappedColors {
            values: values.into_vec()?,
            colorizer: colorizer(mapping),
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
    let options = Scatter3DOptions {
        zdir: axis3(&zdir)?,
        depthshade,
        depthshade_minalpha,
        axlim_clip,
        scatter: ScatterOptions {
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
            plot_nonfinite: false,
            offset_transform: None,
        },
    };
    Ok(axes.scatter_with(x.into_vec()?, y.into_vec()?, z.into_vec()?, options)?)
}

/// Which grid points a surface or wireframe samples: `("count", rows,
/// cols)`, `("stride", rows, cols)` or `None` for matplotlib's default.
type PySampling = Option<(String, usize, usize)>;

/// The sampling `spec` names.
fn sampling(spec: PySampling) -> PyResult<Sampling> {
    match spec {
        None => Ok(Sampling::Auto),
        Some((kind, rows, cols)) => match kind.as_str() {
            "count" => Ok(Sampling::Counts { rows, cols }),
            "stride" => Ok(Sampling::Strides { rows, cols }),
            _ => Err(PyErr::value_error(format!("no sampling named {kind:?}"))),
        },
    }
}

/// How a surface is coloured: one colour, or its heights through a
/// colormap (`None` for the next colour of the cycle), and whether it is
/// shaded (`None`: unless colour-mapped).
type SurfaceColors = (Option<Color>, Option<Mapping>, Option<bool>);

/// A surface through the grids `(x, y, z)` (`plot_surface`), sampled as
/// `spec` says and coloured as `colors` says; `axlim_clip` hides the
/// points outside the view limits.
fn plot_surface(
    axes: Axes3D,
    (x, y, z): (NdArray, NdArray, NdArray),
    spec: PySampling,
    (color, mapping, shade): SurfaceColors,
    axlim_clip: bool,
) -> PyResult<Collection> {
    let colors = match (mapping, color) {
        (Some(mapping), _) => FaceColors::from(colorizer(mapping)),
        (None, Some(color)) => FaceColors::Single(color),
        (None, None) => FaceColors::Cycle,
    };
    let options = SurfaceOptions {
        sampling: sampling(spec)?,
        colors,
        shade,
        axlim_clip,
        ..SurfaceOptions::default()
    };
    Ok(axes.plot_surface(&grid(&x, "X")?, &grid(&y, "Y")?, &grid(&z, "Z")?, options)?)
}

/// A wireframe through the grids `(x, y, z)` (`plot_wireframe`), sampled
/// as `spec` says, in `color` (`None`: the next colour of the cycle).
fn plot_wireframe(
    axes: Axes3D,
    (x, y, z): (NdArray, NdArray, NdArray),
    spec: PySampling,
    color: Option<Color>,
    axlim_clip: bool,
) -> PyResult<Collection> {
    let options = WireframeOptions {
        sampling: sampling(spec)?,
        props: CollectionProps {
            color,
            ..CollectionProps::default()
        },
        axlim_clip,
    };
    Ok(axes.plot_wireframe(&grid(&x, "X")?, &grid(&y, "Y")?, &grid(&z, "Z")?, options)?)
}

/// The text `text` at `(x, y, z)`, its baseline along `zdir` (`None`:
/// facing the viewer; `text`).
fn text(
    axes: Axes3D,
    (x, y, z): (f64, f64, f64),
    text: String,
    zdir: Option<String>,
) -> PyResult<Text> {
    let zdir = zdir.as_deref().map(axis3).transpose()?;
    Ok(axes.text(x, y, z, text, zdir))
}

/// matplotlib's test data set, `(X, Y, Z)` grids sampled every `delta`
/// (`get_test_data`).
fn test_data(delta: f64) -> PyResult<(NdArray, NdArray, NdArray)> {
    let (x, y, z) = pltrs::mplot3d::test_data(delta);
    let array = |grid: Grid<f64>| {
        let (rows, cols) = (grid.rows(), grid.cols());
        NdArray::with_shape(grid.into_vec(), vec![rows, cols])
    };
    Ok((array(x)?, array(y)?, array(z)?))
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("plot", plot)?;
    module.add("scatter", scatter)?;
    module.add("plot_surface", plot_surface)?;
    module.add("plot_wireframe", plot_wireframe)?;
    module.add("text", text)?;
    module.add("test_data", test_data)
}
