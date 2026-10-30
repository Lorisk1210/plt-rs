//! `_native.contour`: contour plots (matplotlib's `contour`): making the
//! line and filled contour sets of a grid, and the contour set's levels,
//! paths, styles, nearest points and labels.

use pltrs::axes::Axes;
use pltrs::collections::Collection;
use pltrs::colors::{Color, Colormap, Extend, Norm};
use pltrs::contour::{
    ClabelFormat, ClabelOptions, ContourAlgorithm, ContourError, ContourLegendStyle,
    ContourOptions, ContourOrigin, ContourSet, Levels, NearestIn, QuadCoords, QuadField,
};
use pltrs::fonts::FontSize;
use pltrs::numeric::Grid;
use pltrs::text::Text;
use pltrs::ticker::LabelFormat;
use pltrs::tri::Triangulation;
use pltrs_core::Point;

use super::cm::NativeColormap;
use super::lines::PyLineStyle;
use super::path::{PathArrays, path_arrays};
use crate::binding::prelude::*;

artist_handle!(ContourSet);

impl From<ContourError> for PyErr {
    /// matplotlib's errors for contour input and labels: `TypeError` for
    /// malformed grids, `IndexError` for a level out of range, else
    /// `ValueError`.
    fn from(err: ContourError) -> Self {
        let message = err.to_string();
        match err {
            ContourError::GridTooSmall { .. }
            | ContourError::ShapeMismatch { .. }
            | ContourError::XLength { .. }
            | ContourError::YLength { .. } => PyErr::type_error(message),
            ContourError::LevelIndex { .. } => PyErr::new(ExceptionKind::IndexError, message),
            _ => PyErr::value_error(message),
        }
    }
}

/// A 2-D float array as a grid.
fn grid(array: &NdArray) -> PyResult<Grid<f64>> {
    match array.shape[..] {
        [rows, cols] => Grid::from_vec(rows, cols, array.data.clone())
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
type Look = (
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

/// Line (or, `filled`, filled) contours of the field (matplotlib's
/// `contour` and `contourf`, whose arguments the Python side sorted out).
fn contour(
    axes: Axes,
    filled: bool,
    (z, mask, x, y, (origin, extent)): Field,
    (corner_mask, nchunk, algorithm): Generation,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
    let mut field = QuadField::new(grid(&z)?);
    if let Some(mask) = mask {
        let (rows, cols) = (mask.shape[0], mask.shape.get(1).copied().unwrap_or(0));
        field = field.with_mask(
            Grid::from_vec(rows, cols, mask.data)
                .map_err(|err| PyErr::value_error(err.to_string()))?,
        );
    }
    field = match (x, y) {
        (Some(x), Some(y)) if x.shape.len() == 2 => field.with_coords(QuadCoords::Mesh {
            x: grid(&x)?,
            y: grid(&y)?,
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
    let options = contour_options(requested, look, extras)?;
    Ok(if filled {
        axes.contourf(&field, options)?
    } else {
        axes.contour(&field, options)?
    })
}

/// The extend, hatches and antialiasing of a contour set.
type Extras = (String, Vec<Option<String>>, Option<bool>);

/// The options of a contour set of the requested levels and look.
fn contour_options(
    requested: Option<NdArray>,
    (colors, mapping, alpha, linewidths, linestyles, negative): Look,
    (extend, hatches, antialiased): Extras,
) -> PyResult<ContourOptions> {
    Ok(ContourOptions {
        levels: levels(requested)?,
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
        ..ContourOptions::default()
    })
}

/// Line (or, `filled`, filled) contours of `z` on the triangulation
/// `tri` (matplotlib's `tricontour` and `tricontourf`).
fn tricontour(
    axes: Axes,
    filled: bool,
    tri: Triangulation,
    z: NdArray,
    requested: Option<NdArray>,
    look: Look,
    extras: Extras,
) -> PyResult<ContourSet> {
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

/// The colours of the levels or bands, as an `(N, 4)` array.
fn get_colors(set: ContourSet) -> PyResult<NdArray> {
    let colors = set.colors();
    let data = colors.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect();
    NdArray::with_shape(data, vec![colors.len(), 4])
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

/// How labels are written: a template or callable, or `{level: text}`
/// pairs, or `None` for matplotlib's default formatter.
fn clabel_format(fmt: Option<Owned>) -> PyResult<ClabelFormat> {
    let Some(fmt) = fmt else {
        return Ok(ClabelFormat::Scalar);
    };
    Python::with_gil(|py| {
        let fmt = fmt.bind(py);
        if let Ok(template) = fmt.extract::<String>() {
            return Ok(ClabelFormat::Label(LabelFormat::Template(template)));
        }
        if let Ok(pairs) = fmt.extract::<Vec<(f64, String)>>() {
            return Ok(ClabelFormat::Map(pairs));
        }
        let callback: Callback = fmt.extract()?;
        Ok(ClabelFormat::Label(LabelFormat::func(move |level| {
            callback.call::<String>((level,)).unwrap_or_default()
        })))
    })
}

/// The label options: `(levels, fontsize, inline, inline_spacing,
/// colors, manual positions)`.
type ClabelLayout = (
    Option<Vec<f64>>,
    Option<f64>,
    bool,
    f64,
    Option<Vec<Color>>,
    Option<Vec<(f64, f64)>>,
);

/// Labels the contour lines (matplotlib's `clabel`): the label texts.
fn clabel(
    set: ContourSet,
    (levels, fontsize, inline, inline_spacing, colors, manual): ClabelLayout,
    fmt: Option<Owned>,
    (rightside_up, zorder, use_clabeltext): (bool, Option<f64>, bool),
) -> PyResult<Vec<Text>> {
    let options = ClabelOptions {
        levels,
        fontsize: fontsize.map(FontSize::Points),
        inline,
        inline_spacing,
        fmt: clabel_format(fmt)?,
        colors,
        manual,
        rightside_up,
        zorder,
        use_clabeltext,
    };
    Ok(set.clabel(options)?)
}

/// The texts of the set's labels.
fn label_texts(set: ContourSet) -> PyResult<Vec<Text>> {
    Ok(set.labels().into_iter().map(|label| label.artist).collect())
}

/// Sets the line widths of the levels.
fn set_linewidths(set: ContourSet, widths: Vec<f64>) -> PyResult<()> {
    set.set_linewidths(&widths);
    Ok(())
}

/// Maps the levels through `cmap` and `norm` (the Python colorizer's,
/// handed over when it changes).
fn set_mapping(set: ContourSet, cmap: Colormap, norm: Box<dyn Norm>) -> PyResult<()> {
    set.set_cmap(cmap).set_norm(norm);
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

/// Removes the set, its collections and its labels.
fn remove(set: ContourSet) -> PyResult<()> {
    set.remove();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("contour", contour)?;
    module.add("tricontour", tricontour)?;
    module.add("get_levels", get_levels)?;
    module.add("get_layers", get_layers)?;
    module.add("get_cvalues", get_cvalues)?;
    module.add("get_state", get_state)?;
    module.add("get_paths", get_paths)?;
    module.add("get_linewidths", get_linewidths)?;
    module.add("get_colors", get_colors)?;
    module.add("get_collections", get_collections)?;
    module.add("find_nearest_contour", find_nearest_contour)?;
    module.add("clabel", clabel)?;
    module.add("label_texts", label_texts)?;
    module.add("set_linewidths", set_linewidths)?;
    module.add("get_clim", get_clim)?;
    module.add("set_mapping", set_mapping)?;
    module.add("get_cmap", get_cmap)?;
    module.add("legend_elements", legend_elements)?;
    module.add("remove", remove)
}
