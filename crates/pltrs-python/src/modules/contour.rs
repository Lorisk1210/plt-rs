//! `_native.contour`: contour plots (matplotlib's `contour`): making the
//! line and filled contour sets of a grid, and the contour set's levels,
//! paths, styles, nearest points and labels.

use pltrs::axes::Axes;
use pltrs::collections::Collection;
use pltrs::colors::{Color, Colormap, Extend, Norm};
use pltrs::contour::{
    ContourAlgorithm, ContourError, ContourLegendStyle, ContourOptions, ContourOrigin, ContourSet,
    GeneratorArguments, Levels, NearestIn, QuadCoords, QuadField,
};
use pltrs::image::ScalarKind;
use pltrs::numeric::Grid;
use pltrs::ticker::Locator;
use pltrs::transforms::Transform;
use pltrs::tri::Triangulation;
use pltrs_core::{Path, Point};

use super::axis::PyLocator;
use super::cm::NativeColormap;
use super::lines::PyLineStyle;
use super::path::{PathArrays, PyPath, path_arrays};
use crate::binding::prelude::*;

mod labels;

artist_handle!(ContourSet);

impl From<ContourError> for PyErr {
    /// matplotlib's errors for contour input and labels: `TypeError` for
    /// malformed grids, `IndexError` for a level out of range, the norm's
    /// own error for a mapping it refuses, else `ValueError`.
    fn from(err: ContourError) -> Self {
        let message = err.to_string();
        match err {
            ContourError::Norm(err) => err.into(),
            ContourError::GridTooSmall { .. }
            | ContourError::ShapeMismatch { .. }
            | ContourError::XLength { .. }
            | ContourError::YLength { .. } => PyErr::type_error(message),
            ContourError::LevelIndex { .. } | ContourError::VertexIndex { .. } => {
                PyErr::new(ExceptionKind::IndexError, message)
            }
            _ => PyErr::value_error(message),
        }
    }
}

/// A 2-D float array as a grid.
fn grid(array: NdArray) -> PyResult<Grid<f64>> {
    match array.shape[..] {
        [rows, cols] => Grid::from_vec(rows, cols, array.data)
            .map_err(|err| PyErr::value_error(err.to_string())),
        _ => Err(PyErr::type_error(format!(
            "Input z must be 2D, not {}D",
            array.shape.len()
        ))),
    }
}

/// Where an implicit grid lies: `(origin, extent)`, the origin
/// `"lower"`, `"upper"` or `"image"`.
type Placement = (Option<String>, Option<(f64, f64, f64, f64)>);

/// The field to contour: `(z, mask, x, y, placement)`, the mask and
/// coordinates optional, `x` and `y` 1-D (axes) or 2-D (a mesh).
type Field = (
    NdArray,
    Option<NdArray<bool>>,
    Option<NdArray>,
    Option<NdArray>,
    Placement,
);

/// How the contours are generated: `(corner_mask, nchunk, algorithm)`.
type Generation = (Option<bool>, usize, Option<String>);

/// The look: `(colors, (cmap, norm, vmin, vmax), alpha, linewidths,
/// linestyles, negative_linestyles)`.
pub(super) type Look = (
    Option<Vec<Color>>,
    super::plotting::mapping::Mapping,
    Option<f64>,
    Option<Vec<f64>>,
    Option<Vec<PyLineStyle>>,
    Option<PyLineStyle>,
);

/// The levels: `None` for automatic ones, a count, or the values.
fn levels(levels: Option<NdArray>) -> PyResult<Levels> {
    Ok(match levels {
        None => Levels::Auto,
        Some(levels) if levels.shape.is_empty() => {
            // The truncation is matplotlib's int(levels).
            #[allow(clippy::cast_possible_truncation, reason = "a level count")]
            let count = levels.data[0] as i64;
            Levels::Count(count)
        }
        Some(levels) => Levels::Values(levels.into_vec()?),
    })
}

/// The quad field of a [`Field`] generated as a [`Generation`] says.
fn quad_field(
    (z, mask, x, y, (origin, extent)): Field,
    (corner_mask, nchunk, algorithm): Generation,
) -> PyResult<QuadField> {
    let mut field = QuadField::new(grid(z)?);
    if let Some(mask) = mask {
        let (rows, cols) = (mask.shape[0], mask.shape.get(1).copied().unwrap_or(0));
        field = field.with_mask(
            Grid::from_vec(rows, cols, mask.data)
                .map_err(|err| PyErr::value_error(err.to_string()))?,
        );
    }
    field = match (x, y) {
        (Some(x), Some(y)) if x.shape.len() == 2 => field.with_coords(QuadCoords::Mesh {
            x: grid(x)?,
            y: grid(y)?,
        }),
        (Some(x), Some(y)) => field.with_coords(QuadCoords::Axes {
            x: x.into_vec()?,
            y: y.into_vec()?,
        }),
        _ => field.with_coords(QuadCoords::Implicit {
            origin: origin.map(|origin| match origin.as_str() {
                "upper" => ContourOrigin::Upper,
                "image" => ContourOrigin::Image,
                _ => ContourOrigin::Lower,
            }),
            extent: extent.map(|(x0, x1, y0, y1)| [x0, x1, y0, y1]),
        }),
    };
    field.corner_mask = corner_mask;
    field.nchunk = nchunk;
    if let Some(algorithm) = algorithm {
        field.algorithm = Some(
            algorithm
                .parse::<ContourAlgorithm>()
                .map_err(|err| PyErr::value_error(err.to_string()))?,
        );
    }
    Ok(field)
}

/// Line (or, `filled`, filled) contours of the field (matplotlib's
/// `contour` and `contourf`, whose arguments the Python side sorted out).
fn contour(
    axes: Axes,
    filled: bool,
    field: Field,
    generation: Generation,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
    let field = quad_field(field, generation)?;
    let options = contour_options(requested, look, extras)?;
    let set = if filled {
        axes.contourf(&field, options)?
    } else {
        axes.contour(&field, options)?
    };
    // What matplotlib's constructor warns about (non-positive values of a
    // log-scaled field masked).
    for warning in set.warnings() {
        pltrs::warnings::warn(&warning.to_string());
    }
    Ok(set)
}

/// What the contour generator of the field is made from, the defaults
/// resolved with the current rcParams: `(x, y, algorithm, corner_mask)`,
/// the coordinates of every point shaped like z (the arguments matplotlib
/// gives `contourpy.contour_generator`).
fn generator_arguments(
    field: Field,
    generation: Generation,
) -> PyResult<(NdArray, NdArray, String, bool)> {
    let GeneratorArguments {
        x,
        y,
        algorithm,
        corner_mask,
    } = quad_field(field, generation)?.generator_arguments(&pltrs::rc::params())?;
    let array = |grid: Grid<f64>| {
        let (rows, cols) = grid.shape();
        NdArray::with_shape(grid.into_vec(), vec![rows, cols])
    };
    Ok((
        array(x)?,
        array(y)?,
        algorithm.name().to_owned(),
        corner_mask,
    ))
}

/// Sets the edge colours of the contours, cycled over the paths of the
/// levels or bands.
fn set_edgecolors(set: ContourSet, colors: Vec<Color>) -> PyResult<()> {
    set.set_edgecolors(colors);
    Ok(())
}

/// Gives the other bands of a hatched set what its first band, which
/// the Python object sets its properties on, was given.
fn share_first_band_look(set: ContourSet) -> PyResult<()> {
    set.share_first_band_look();
    Ok(())
}

/// Line (or, `filled`, filled) contours of given segments: for each level
/// (band between consecutive levels), the paths of its segments
/// (matplotlib's `ContourSet(ax, levels, allsegs, allkinds)`).
fn contour_segments(
    axes: Axes,
    filled: bool,
    levels: Vec<f64>,
    segments: Vec<Vec<PyPath>>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
    let segments: Vec<Vec<Path>> = segments
        .into_iter()
        .map(|level| level.into_iter().map(|segment| segment.path).collect())
        .collect();
    let options = contour_options(None, look, extras)?;
    Ok(axes.contour_segments(&levels, &segments, filled, options)?)
}

/// The extend, hatches, antialiasing, level locator, transform and log
/// scaling of a contour set, the locator described as `_tick_helpers`
/// describes the helpers it installs: `(spec, True)` for a locator of
/// `ticker`, `(adapter, False)` for one of the program's; the transform
/// `None` for the axes' data coordinates; whether the set is log-scaled as
/// matplotlib decides it (by the classes of its norm and locator).
pub(super) type Extras = (
    String,
    Vec<Option<String>>,
    Option<bool>,
    Option<(Owned, bool)>,
    Option<Transform>,
    bool,
);

/// The locator a Python locator's description gives (see [`Extras`]).
fn level_locator((description, native): (Owned, bool)) -> PyResult<Box<dyn Locator>> {
    Python::with_gil(|py| {
        let description = description.bind(py);
        Ok(if native {
            super::ticker::locator(&description)?
        } else {
            Box::new(PyLocator::of(&description)?) as Box<dyn Locator>
        })
    })
}

/// The options of a contour set of the requested levels and look.
pub(super) fn contour_options(
    requested: Option<NdArray>,
    (colors, mapping, alpha, linewidths, linestyles, negative): Look,
    (extend, hatches, antialiased, locator, transform, log_scaled): Extras,
) -> PyResult<ContourOptions> {
    Ok(ContourOptions {
        levels: levels(requested)?,
        locator: locator.map(level_locator).transpose()?,
        extend: match extend.as_str() {
            "both" => Extend::Both,
            "min" => Extend::Min,
            "max" => Extend::Max,
            _ => Extend::Neither,
        },
        colors,
        colorizer: super::plotting::mapping::colorizer(mapping),
        alpha,
        linewidths,
        linestyles: linestyles.map(|styles| styles.into_iter().map(|s| s.0).collect()),
        negative_linestyles: negative.map(|style| style.0),
        antialiased,
        hatches,
        transform,
        log_scaled: Some(log_scaled),
    })
}

/// Line (or, `filled`, filled) contours of `z` on the triangulation
/// `tri` (matplotlib's `tricontour` and `tricontourf`).
fn tricontour(
    axes: Axes,
    filled: bool,
    tri: Native<Triangulation>,
    z: NdArray,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
    let tri = tri.borrow();
    let options = contour_options(requested, look, extras)?;
    let z = z.into_vec()?;
    Ok(if filled {
        axes.tricontourf(&tri, z, options)?
    } else {
        axes.tricontour(&tri, z, options)?
    })
}

/// The levels of the set.
fn get_levels(set: ContourSet) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(set.levels()))
}

/// The value each level or band is coloured by.
fn get_layers(set: ContourSet) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(set.layers()))
}

/// The values the colours are mapped from, and whether they are colour
/// indices (matplotlib's `cvalues`, mapped through a `NoNorm` with
/// explicit colours).
fn get_cvalues(set: ContourSet) -> PyResult<(NdArray, bool)> {
    let (cvalues, indexed) = set.cvalues();
    Ok((NdArray::from_vec(cvalues), indexed))
}

/// `(filled, zmin, zmax, extend)` of the set.
fn get_state(set: ContourSet) -> PyResult<(bool, Option<f64>, Option<f64>, &'static str)> {
    let extend = match set.extend() {
        Extend::Neither => "neither",
        Extend::Both => "both",
        Extend::Min => "min",
        Extend::Max => "max",
    };
    Ok((set.filled(), set.zmin(), set.zmax(), extend))
}

/// One compound path per level or band.
fn get_paths(set: ContourSet) -> PyResult<Vec<PathArrays>> {
    set.paths()
        .iter()
        .map(|path| path_arrays(path, false))
        .collect()
}

/// The line widths of the levels.
fn get_linewidths(set: ContourSet) -> PyResult<Vec<f64>> {
    Ok(set.linewidths())
}

/// The collections drawing the set.
fn get_collections(set: ContourSet) -> PyResult<Vec<Collection>> {
    Ok(set.collections().to_vec())
}

/// The point of the contours nearest `(x, y)` (in display pixels when
/// `pixel`), among the levels `indices` (all when `None`): `(level index,
/// segment index, vertex index, (x, y), squared distance)`.
type Nearest = Option<(usize, usize, usize, (f64, f64), f64)>;

/// matplotlib's `find_nearest_contour`.
fn find_nearest_contour(
    set: ContourSet,
    (x, y): (f64, f64),
    indices: Option<Vec<usize>>,
    pixel: bool,
) -> PyResult<Nearest> {
    let space = if pixel {
        NearestIn::Display
    } else {
        NearestIn::Coordinates
    };
    Ok(set
        .find_nearest_contour(Point::new(x, y), indices.as_deref(), space)?
        .map(|near| {
            (
                near.level,
                near.line,
                near.vertex,
                (near.point.x, near.point.y),
                near.distance_squared,
            )
        }))
}

/// Gives the bands of a hatched filled set drawn by a collection each
/// their value of `values` (`NaN` masked); `integers` when numpy holds
/// them as integers or booleans.
fn set_band_values(set: ContourSet, values: Vec<f64>, integers: bool) -> PyResult<()> {
    let values: Vec<Option<f64>> = values
        .into_iter()
        .map(|v| v.is_finite().then_some(v))
        .collect();
    let kind = if integers {
        ScalarKind::Integer
    } else {
        ScalarKind::Float
    };
    set.set_band_values(&values, kind);
    Ok(())
}

/// Sets the hatch patterns of the bands.
fn set_hatches(set: ContourSet, hatches: Vec<Option<String>>) -> PyResult<()> {
    set.set_hatches(hatches);
    Ok(())
}

/// Sets the line widths of the levels.
fn set_linewidths(set: ContourSet, widths: Vec<f64>) -> PyResult<()> {
    set.set_linewidths(&widths);
    Ok(())
}

/// Maps the levels through `cmap` and `norm` (the Python colorizer's,
/// handed over when it changes): a new norm when `replaced`, else the
/// set's norm with new parameters.
fn set_mapping(
    set: ContourSet,
    cmap: Colormap,
    norm: Box<dyn Norm>,
    replaced: bool,
) -> PyResult<()> {
    set.set_cmap(cmap);
    if replaced {
        set.set_norm(norm);
    } else {
        set.retune_norm(norm);
    }
    Ok(())
}

/// The colormap the levels are mapped through.
fn get_cmap(set: ContourSet) -> PyResult<Option<NativeColormap>> {
    Ok(set.cmap().map(NativeColormap))
}

/// The colour limits of the set's mapping.
fn get_clim(set: ContourSet) -> PyResult<(Option<f64>, Option<f64>)> {
    Ok(set.norm_limits())
}

/// A legend entry of a contour set as Python makes its artist of it:
/// `("line", rgba, linewidth, (offset, dashes))`, with `None` dashes for
/// a solid line as matplotlib's collections give them, or `("band", rgba,
/// hatch)`.
struct LegendEntry(ContourLegendStyle);

impl IntoPy for LegendEntry {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let rgba = |c: pltrs_core::Rgba| (c.r, c.g, c.b, c.a);
        match self.0 {
            ContourLegendStyle::Line {
                color,
                linewidth,
                dashes,
            } => (
                "line",
                rgba(color),
                linewidth,
                (dashes.offset, dashes.pattern),
            )
                .into_py(py),
            ContourLegendStyle::Band { color, hatch } => ("band", rgba(color), hatch).into_py(py),
        }
    }
}

/// The legend entries describing the set and their labels (matplotlib's
/// `legend_elements`), its numbers written by `str_format`.
fn legend_elements(
    set: ContourSet,
    variable_name: String,
    str_format: Callback,
) -> PyResult<(Vec<LegendEntry>, Vec<String>)> {
    let (styles, labels) = set.legend_styles(&variable_name, |value| {
        str_format.call::<String>((value,)).unwrap_or_default()
    });
    Ok((styles.into_iter().map(LegendEntry).collect(), labels))
}

/// Removes the set, its collections and its labels; warns, as matplotlib
/// does, when some labels had been removed one by one before.
fn remove(set: ContourSet) -> PyResult<()> {
    if let Some(warning) = set.remove() {
        pltrs::warnings::warn(&warning.to_string());
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    labels::register(module)?;
    module.add("contour", contour)?;
    module.add("tricontour", tricontour)?;
    module.add("contour_segments", contour_segments)?;
    module.add("set_edgecolors", set_edgecolors)?;
    module.add("share_first_band_look", share_first_band_look)?;
    module.add("generator_arguments", generator_arguments)?;
    module.add("get_levels", get_levels)?;
    module.add("get_layers", get_layers)?;
    module.add("get_cvalues", get_cvalues)?;
    module.add("get_state", get_state)?;
    module.add("get_paths", get_paths)?;
    module.add("get_linewidths", get_linewidths)?;
    module.add("get_collections", get_collections)?;
    module.add("find_nearest_contour", find_nearest_contour)?;
    module.add("set_linewidths", set_linewidths)?;
    module.add("set_hatches", set_hatches)?;
    module.add("set_band_values", set_band_values)?;
    module.add("get_clim", get_clim)?;
    module.add("set_mapping", set_mapping)?;
    module.add("get_cmap", get_cmap)?;
    module.add("legend_elements", legend_elements)?;
    module.add("remove", remove)
}
