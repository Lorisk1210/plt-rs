//! The 3-D plotting methods (matplotlib's `Axes3D.plot`, `scatter`,
//! `plot_surface`, `plot_wireframe` and `text`), whose arguments the
//! Python side resolved as matplotlib does.

use pltrs::axes::{MappedColors, ScatterColors, ScatterOptions};
use pltrs::collections::{Collection, CollectionProps};
use pltrs::colors::{Alpha, Color, LightSource};
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
pub(super) fn grid(array: &NdArray, name: &str) -> PyResult<Grid<f64>> {
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
/// colormap (`None` for the next colour of the cycle), or one colour per
/// grid point (RGBA rows shaped like the grid, matplotlib's
/// `facecolors`).
type SurfaceColors = (Option<Color>, Option<Mapping>, Option<NdArray>);

/// Whether a surface is shaded (`None`: unless colour-mapped) and the
/// light at `(azdeg, altdeg)` shading it (`None`: mplot3d's).
pub(super) type Shading = (Option<bool>, Option<(f64, f64)>);

/// The light at `(azdeg, altdeg)` of a [`Shading`].
pub(super) fn light((azdeg, altdeg): (f64, f64)) -> LightSource {
    LightSource::new(azdeg, altdeg)
}

/// One colour per grid point from RGBA rows of shape `(rows, cols, 4)`.
fn grid_colors(array: &NdArray) -> PyResult<Grid<Color>> {
    let [rows, cols, 4] = array.shape[..] else {
        return Err(PyErr::value_error(
            "facecolors must be one RGBA colour per grid point",
        ));
    };
    let flat = NdArray::with_shape(array.data.clone(), vec![rows * cols, 4])?;
    Grid::from_vec(rows, cols, rgba_rows(&flat)?).map_err(|err| PyErr::value_error(err.to_string()))
}

/// A surface through the grids `(x, y, z)` (`plot_surface`), sampled as
/// `spec` says, coloured and shaded as `colors` and `shading` say;
/// `axlim_clip` hides the points outside the view limits.
fn plot_surface(
    axes: Axes3D,
    (x, y, z): (NdArray, NdArray, NdArray),
    spec: PySampling,
    (color, mapping, cell_colors): SurfaceColors,
    (shade, lightsource): Shading,
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
        cell_colors: cell_colors.as_ref().map(grid_colors).transpose()?,
        shade,
        lightsource: lightsource.map(light),
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

/// The text `text` at `(x, y, z)`, its baseline along `(dx, dy, dz)` as
/// seen on screen (zero: horizontal), hidden outside the view limits with
/// `axlim_clip` (`text`).
fn text(
    axes: Axes3D,
    (x, y, z): (f64, f64, f64),
    text: String,
    (dx, dy, dz): (f64, f64, f64),
    axlim_clip: bool,
) -> PyResult<Text> {
    let text = axes.text_along(x, y, z, text, [dx, dy, dz]);
    if axlim_clip {
        axes.set_axlim_clip(&text, true)?;
    }
    Ok(text)
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
