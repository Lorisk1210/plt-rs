//! `_native.collections`: collections of artists drawn together
//! (matplotlib's `collections`).
//!
//! Colours cross as the Python side normalises them, as matplotlib keeps
//! them: `(N, 4)` RGBA arrays, `None` for the default, and the edge
//! colour `"face"`. Widths and line styles cross as lists.

use pltrs::artist::ArtistHandle;
use pltrs::collections::{Collection, EdgeColors, EventCollection, HatchColors};
use pltrs::colors::Color;
use pltrs::orientation::Orientation;
use pltrs_core::Point;
use pltrs_render::{CapStyle, JoinStyle};

use super::lines::{PyLineStyle, cap_style, join_style};
use super::path::{PathArrays, PyPath, path_arrays};
use super::transforms::{points, points_array};
use crate::binding::prelude::*;

mod new;

artist_handle!(Collection, EventCollection);

/// Colours as an `(N, 4)` RGBA array.
pub(super) fn rgba_rows(array: &NdArray) -> PyResult<Vec<Color>> {
    if array.data.is_empty() {
        return Ok(Vec::new());
    }
    if array.shape.len() != 2 || array.shape[1] != 4 {
        return Err(PyErr::value_error(format!(
            "expected an (N, 4) array of RGBA colours, got shape {:?}",
            array.shape
        )));
    }
    Ok(array
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&rgba| Color::from(rgba))
        .collect())
}

/// Colours back as an `(N, 4)` RGBA array.
fn rgba_array(colors: &[pltrs::colors::Rgba]) -> PyResult<NdArray> {
    let data = colors.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect();
    NdArray::with_shape(data, vec![colors.len(), 4])
}

/// Edge colours: `"face"`, or an RGBA array.
pub(super) fn edge_colors(value: &Bound<'_>) -> PyResult<EdgeColors> {
    if value.is_none() {
        return Ok(EdgeColors::Default);
    }
    if let Ok(name) = value.extract::<String>() {
        if name == "face" {
            return Ok(EdgeColors::Face);
        }
        return Ok(EdgeColors::Colors(vec![Color::from(name)]));
    }
    Ok(EdgeColors::Colors(rgba_rows(&value.extract()?)?))
}

/// Hatch colours: `"edge"`, or an RGBA array.
fn hatch_colors(value: &Bound<'_>) -> PyResult<HatchColors> {
    if let Ok(name) = value.extract::<String>()
        && name == "edge"
    {
        return Ok(HatchColors::Edge);
    }
    Ok(HatchColors::Colors(rgba_rows(&value.extract()?)?))
}

/// Sets the face colours (`None`: the kind's default again).
fn set_facecolors(collection: Collection, colors: Option<NdArray>) -> PyResult<()> {
    match colors {
        Some(colors) => collection.set_facecolors(rgba_rows(&colors)?),
        None => collection.reset_facecolor(),
    };
    Ok(())
}

/// Sets the edge colours (`None`: the default rule; `"face"`; colours).
fn set_edgecolors(collection: Collection, colors: Owned) -> PyResult<()> {
    let edges = Python::with_gil(|py| edge_colors(&colors.bind(py)))?;
    collection.set_edgecolors(edges);
    Ok(())
}

/// Sets the hatch colours (`"edge"` or colours).
fn set_hatchcolors(collection: Collection, colors: Owned) -> PyResult<()> {
    let hatches = Python::with_gil(|py| hatch_colors(&colors.bind(py)))?;
    collection.set_hatchcolors(hatches);
    Ok(())
}

/// Sets the edge widths in points.
fn set_linewidths(collection: Collection, widths: Vec<f64>) -> PyResult<()> {
    collection.set_linewidths(widths);
    Ok(())
}

/// Sets the edge line styles.
fn set_linestyles(collection: Collection, styles: Vec<PyLineStyle>) -> PyResult<()> {
    collection.set_linestyles(styles.into_iter().map(|style| style.0).collect());
    Ok(())
}

/// Sets per-item antialiasing.
fn set_antialiaseds(collection: Collection, antialiased: Vec<bool>) -> PyResult<()> {
    collection.set_antialiased(antialiased);
    Ok(())
}

/// Sets one opacity per item.
fn set_alphas(collection: Collection, alphas: Vec<f64>) -> PyResult<()> {
    collection.set_alphas(alphas);
    Ok(())
}

/// Sets the hatch pattern.
fn set_hatch(collection: Collection, hatch: Option<String>) -> PyResult<()> {
    match hatch {
        Some(hatch) => collection.set_hatch(hatch),
        None => collection.clear_hatch(),
    };
    Ok(())
}

/// Sets the hatch line width in points.
fn set_hatch_linewidth(collection: Collection, width: f64) -> PyResult<()> {
    collection.set_hatch_linewidth(width);
    Ok(())
}

/// Sets the cap style of the edges.
fn set_capstyle(collection: Collection, name: Owned) -> PyResult<()> {
    let cap = Python::with_gil(|py| cap_style(&name.bind(py)))?;
    collection.set_capstyle(cap);
    Ok(())
}

/// Sets the join style of the edges.
fn set_joinstyle(collection: Collection, name: Owned) -> PyResult<()> {
    let join = Python::with_gil(|py| join_style(&name.bind(py)))?;
    collection.set_joinstyle(join);
    Ok(())
}

/// Sets the pick radius in pixels.
fn set_pickradius(collection: Collection, radius: f64) -> PyResult<()> {
    collection.set_pickradius(radius);
    Ok(())
}

/// The pick radius in pixels.
fn get_pickradius(collection: Collection) -> PyResult<f64> {
    Ok(collection.pickradius())
}

/// Sets the links of the items.
fn set_urls(collection: Collection, urls: Vec<Option<String>>) -> PyResult<()> {
    collection.set_urls(urls);
    Ok(())
}

/// The links of the items.
fn get_urls(collection: Collection) -> PyResult<Vec<Option<String>>> {
    Ok(collection.urls())
}

/// The style property `name` as matplotlib's getter returns it: face and
/// edge colours as `(N, 4)` RGBA arrays after defaults, mapping and alpha
/// (matplotlib's `get_facecolor` and `get_edgecolor`), widths and line
/// styles as lists, the hatch, cap and join styles by name.
fn get(collection: Collection, name: String) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let style = collection.style();
        let colors = || {
            collection
                .colors()
                .map_err(|err| PyErr::value_error(err.to_string()))
        };
        let value = match name.as_str() {
            "facecolor" => rgba_array(&colors()?.faces)?.into_py(py)?,
            "edgecolor" => rgba_array(&colors()?.edges)?.into_py(py)?,
            "hatchcolor" => rgba_array(&colors()?.hatches)?.into_py(py)?,
            "edgecolor_setting" => match style.edgecolors {
                EdgeColors::Face => "face".into_py(py)?,
                EdgeColors::Default => Bound::none(py),
                EdgeColors::Colors(_) => rgba_array(&colors()?.edges)?.into_py(py)?,
            },
            "linewidth" => collection.linewidths().into_py(py)?,
            "linestyle" => style
                .linestyles
                .into_iter()
                .map(PyLineStyle)
                .collect::<Vec<_>>()
                .into_py(py)?,
            "hatch" => style.hatch.into_py(py)?,
            "hatch_linewidth" => style.hatch_linewidth.into_py(py)?,
            "antialiased" => style.antialiased.into_py(py)?,
            "alpha_array" => style.alphas.into_py(py)?,
            "capstyle" => style.capstyle.map(CapStyle::name).into_py(py)?,
            "joinstyle" => style.joinstyle.map(JoinStyle::name).into_py(py)?,
            _ => {
                return Err(PyErr::new(
                    ExceptionKind::AttributeError,
                    format!("'Collection' object has no property '{name}'"),
                ));
            }
        };
        Ok(value.unbind())
    })
}

/// The positions of the items, as an `(n, 2)` array.
fn get_offsets(collection: Collection) -> PyResult<NdArray> {
    let offsets = collection.offsets();
    points_array(&offsets, vec![offsets.len(), 2])
}

/// Moves the items to `offsets` (an `(n, 2)` array).
fn set_offsets(collection: Collection, offsets: NdArray) -> PyResult<()> {
    collection.set_offsets(points(&offsets)?);
    Ok(())
}

/// The item shapes, as `(vertices, codes)` pairs.
fn get_paths(collection: Collection) -> PyResult<Vec<PathArrays>> {
    collection
        .paths()
        .iter()
        .map(|path| path_arrays(path, false))
        .collect()
}

/// Replaces the item shapes.
fn set_paths(collection: Collection, paths: Vec<PyPath>) -> PyResult<()> {
    collection.set_paths(paths.into_iter().map(|path| path.path).collect());
    Ok(())
}

/// Replaces the items with polylines through each segment's points.
fn set_segments(collection: Collection, segments: Vec<NdArray>) -> PyResult<()> {
    let segments = segments
        .iter()
        .map(points)
        .collect::<PyResult<Vec<Vec<Point>>>>()?;
    collection.set_segments(segments);
    Ok(())
}

/// Replaces the items with closed polygons through each polygon's points.
fn set_verts(collection: Collection, polygons: Vec<NdArray>) -> PyResult<()> {
    let polygons = polygons
        .iter()
        .map(points)
        .collect::<PyResult<Vec<Vec<Point>>>>()?;
    collection.set_verts(polygons);
    Ok(())
}

/// The marker areas in points², `None` when unset.
fn get_sizes(collection: Collection) -> PyResult<Option<NdArray>> {
    Ok(collection.sizes().map(NdArray::from_vec))
}

/// Sets the marker areas in points².
fn set_sizes(collection: Collection, sizes: NdArray) -> PyResult<()> {
    collection.set_sizes(sizes.into_vec()?);
    Ok(())
}

/// The values mapped to colours (`NaN` where masked), `None` without a
/// colour mapping.
fn get_array(collection: Collection) -> PyResult<Option<NdArray>> {
    Ok(collection.array().map(|values| {
        NdArray::from_vec(
            values
                .into_iter()
                .map(|value| value.unwrap_or(f64::NAN))
                .collect(),
        )
    }))
}

/// Maps `values` (`NaN` masked) to colours; `None` drops the mapping.
fn set_array(collection: Collection, values: Option<NdArray>) -> PyResult<()> {
    match values {
        Some(values) => {
            let values = values
                .into_vec()?
                .into_iter()
                .map(|value| value.is_finite().then_some(value))
                .collect();
            collection
                .set_array(values)
                .map_err(|err| PyErr::value_error(err.to_string()))?;
        }
        None => {
            collection.clear_array();
        }
    }
    Ok(())
}

/// The colour limits of the mapping.
fn get_clim(collection: Collection) -> PyResult<(Option<f64>, Option<f64>)> {
    Ok(collection.clim())
}

/// Sets the colour limits of the mapping.
fn set_clim(collection: Collection, vmin: Option<f64>, vmax: Option<f64>) -> PyResult<()> {
    collection.set_clim(vmin, vmax);
    Ok(())
}

/// The name of the colormap, `None` without a colour mapping.
fn get_cmap_name(collection: Collection) -> PyResult<Option<String>> {
    Ok(collection.cmap().map(|cmap| cmap.name().to_owned()))
}

/// Whether the collection is in its figure.
fn is_attached(collection: Collection) -> PyResult<bool> {
    Ok(collection.raw().is_attached())
}

/// The positions of the events.
fn event_positions(events: EventCollection) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(events.positions()))
}

/// Replaces the positions of the events.
fn set_event_positions(events: EventCollection, positions: NdArray) -> PyResult<()> {
    events.set_positions(&positions.into_vec()?);
    Ok(())
}

/// The orientation of the events, `"horizontal"` or `"vertical"`.
fn event_orientation(events: EventCollection) -> PyResult<&'static str> {
    Ok(match events.orientation() {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    })
}

/// Turns the events to `"horizontal"` or `"vertical"`.
fn set_event_orientation(events: EventCollection, orientation: String) -> PyResult<()> {
    events.set_orientation(if orientation == "vertical" {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    });
    Ok(())
}

/// `(lineoffset, linelength)` of the event lines.
fn event_lines(events: EventCollection) -> PyResult<(f64, f64)> {
    Ok((events.lineoffset(), events.linelength()))
}

/// Sets the offset and length of the event lines (`None` keeps one).
fn set_event_lines(
    events: EventCollection,
    lineoffset: Option<f64>,
    linelength: Option<f64>,
) -> PyResult<()> {
    if let Some(lineoffset) = lineoffset {
        events.set_lineoffset(lineoffset);
    }
    if let Some(linelength) = linelength {
        events.set_linelength(linelength);
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get", get)?;
    module.add("set_facecolors", set_facecolors)?;
    module.add("set_edgecolors", set_edgecolors)?;
    module.add("set_hatchcolors", set_hatchcolors)?;
    module.add("set_linewidths", set_linewidths)?;
    module.add("set_linestyles", set_linestyles)?;
    module.add("set_antialiaseds", set_antialiaseds)?;
    module.add("set_alphas", set_alphas)?;
    module.add("set_hatch", set_hatch)?;
    module.add("set_hatch_linewidth", set_hatch_linewidth)?;
    module.add("set_capstyle", set_capstyle)?;
    module.add("set_joinstyle", set_joinstyle)?;
    module.add("set_pickradius", set_pickradius)?;
    module.add("get_pickradius", get_pickradius)?;
    module.add("set_urls", set_urls)?;
    module.add("get_urls", get_urls)?;
    module.add("get_offsets", get_offsets)?;
    module.add("set_offsets", set_offsets)?;
    module.add("get_paths", get_paths)?;
    module.add("set_paths", set_paths)?;
    module.add("set_segments", set_segments)?;
    module.add("set_verts", set_verts)?;
    module.add("get_sizes", get_sizes)?;
    module.add("set_sizes", set_sizes)?;
    module.add("get_array", get_array)?;
    module.add("set_array", set_array)?;
    module.add("get_clim", get_clim)?;
    module.add("set_clim", set_clim)?;
    module.add("get_cmap_name", get_cmap_name)?;
    module.add("is_attached", is_attached)?;
    module.add("event_positions", event_positions)?;
    module.add("set_event_positions", set_event_positions)?;
    module.add("event_orientation", event_orientation)?;
    module.add("set_event_orientation", set_event_orientation)?;
    module.add("event_lines", event_lines)?;
    module.add("set_event_lines", set_event_lines)?;
    new::register(module)
}
