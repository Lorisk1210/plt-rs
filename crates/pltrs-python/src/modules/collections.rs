//! `_native.collections`: collections of artists drawn together
//! (matplotlib's `collections`).
//!
//! Colours cross as the Python side normalises them, as matplotlib keeps
//! them: `(N, 4)` RGBA arrays, `None` for the default, and the edge
//! colour `"face"`. Widths and line styles cross as lists.

use pltrs::artist::{ArtistHandle, RawHandle, TypedHandle};
use pltrs::collections::{
    Collection, EdgeColors, EllipseCollection, EventCollection, HatchColors, MeshShading,
};
use pltrs::colors::{Color, Colormap, Norm};
use pltrs::image::ScalarKind;
use pltrs::orientation::Orientation;
use pltrs::transforms::Transform;
use pltrs_core::{Affine2D, Point};
use pltrs_render::{CapStyle, JoinStyle};

use super::cm::NativeColormap;
use super::lines::{PyLineStyle, cap_style, join_style};
use super::path::{PathArrays, PyPath, path_arrays};
use super::transforms::{points, points_array};
use crate::binding::prelude::*;

mod copy;
mod held;
mod mapped;
mod new;
mod tri_mesh;

artist_handle!(Collection, EllipseCollection, EventCollection);

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

/// Sets the face colours (`None`: the kind's default again), as the
/// field `_facecolors` is assigned.
fn set_facecolors(collection: Collection, colors: Option<NdArray>) -> PyResult<()> {
    match colors {
        Some(colors) => collection.set_facecolors(rgba_rows(&colors)?),
        None => collection.reset_facecolor(),
    };
    Ok(())
}

/// Sets the edge colours (`None`: the default rule; `"face"`; colours),
/// as the field `_edgecolors` is assigned.
fn set_edgecolors(collection: Collection, colors: Owned) -> PyResult<()> {
    let edges = Python::with_gil(|py| edge_colors(&colors.bind(py)))?;
    collection.set_edgecolors(edges);
    Ok(())
}

/// Gives the face colours as `set_facecolor` does (`None`: the kind's
/// default again); returns numpy's `ValueError` message when per-item
/// alphas cannot be broadcast onto them, for the setter to raise once it
/// has noted them (matplotlib keeps the colour given before it fails to
/// resolve it, and the colours it had).
fn give_facecolors(collection: Collection, colors: Option<NdArray>) -> PyResult<Option<String>> {
    if let Some(colors) = colors {
        return Ok(collection.try_set_facecolors(rgba_rows(&colors)?).err());
    }
    collection.reset_facecolor();
    Ok(None)
}

/// Gives the edge colours as `set_edgecolor` does; returns the message of
/// alphas that do not fit them, as [`give_facecolors`] does.
fn give_edgecolors(collection: Collection, colors: Owned) -> PyResult<Option<String>> {
    let edges = Python::with_gil(|py| edge_colors(&colors.bind(py)))?;
    Ok(collection.try_set_edgecolors(edges).err())
}

/// Has the default edge rule decide as if the faces were `unfilled` or
/// not (matplotlib's `set_edgecolor(None)`, deciding from the face colour
/// given last).
fn decide_default_edges_as(collection: Collection, unfilled: bool) -> PyResult<()> {
    collection.decide_default_edges_as(unfilled);
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

/// Sets the edge line styles, which leaves the collection as fresh as it
/// was (matplotlib's `Collection.set_linestyle` makes nothing stale).
fn set_linestyles(collection: Collection, styles: Vec<PyLineStyle>) -> PyResult<()> {
    let styles = styles.into_iter().map(|style| style.0).collect();
    pltrs::figure::unseen(|| collection.set_linestyles(styles));
    Ok(())
}

/// Sets per-item antialiasing.
fn set_antialiaseds(collection: Collection, antialiased: Vec<bool>) -> PyResult<()> {
    collection.set_antialiased(antialiased);
    Ok(())
}

/// Sets one opacity per item, given as an array of `shape`; raises
/// matplotlib's `ValueError` (the alphas set all the same) when they cannot
/// be broadcast onto the colours given.
fn set_alphas(collection: Collection, alphas: Vec<f64>, shape: Vec<usize>) -> PyResult<()> {
    collection
        .try_set_alpha_array(alphas, Some(shape))
        .map(drop)
        .map_err(PyErr::value_error)
}

/// Sets (or clears) one opacity for every item of a collection (or of a
/// triangle mesh, whose colours are mapped as it draws), the colours of
/// a collection as set until the next draw maps the values again
/// (matplotlib's `set_alpha` with a single value).
fn set_alpha(artist: RawHandle, alpha: Option<f64>) -> PyResult<()> {
    match Collection::from_raw_handle(&artist) {
        Some(collection) => {
            collection.set_uniform_alpha(alpha);
        }
        None => {
            artist.set_alpha(alpha);
        }
    }
    Ok(())
}

/// Maps the values onto the colours the getters report, as a draw does
/// (matplotlib's `update_scalarmappable`).
fn update_scalarmappable(collection: Collection) -> PyResult<()> {
    collection
        .update_scalarmappable()
        .map(drop)
        .map_err(PyErr::value_error)
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

/// Sets the cap style of the edges, which leaves the collection as fresh
/// as it was (as matplotlib's `set_capstyle` does).
fn set_capstyle(collection: Collection, name: Owned) -> PyResult<()> {
    let cap = Python::with_gil(|py| cap_style(&name.bind(py)))?;
    pltrs::figure::unseen(|| collection.set_capstyle(cap));
    Ok(())
}

/// Sets the join style of the edges, which leaves the collection as fresh
/// as it was (as matplotlib's `set_joinstyle` does).
fn set_joinstyle(collection: Collection, name: Owned) -> PyResult<()> {
    let join = Python::with_gil(|py| join_style(&name.bind(py)))?;
    pltrs::figure::unseen(|| collection.set_joinstyle(join));
    Ok(())
}

/// Sets the pick radius in pixels, which changes nothing drawn.
fn set_pickradius(collection: Collection, radius: f64) -> PyResult<()> {
    pltrs::figure::unseen(|| collection.set_pickradius(radius));
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
/// edge colours as `(N, 4)` RGBA arrays after defaults, alpha and the
/// mapping the last draw applied (matplotlib's `get_facecolor` and
/// `get_edgecolor`, see `Collection::reported_colors`), widths and line
/// styles as lists, the hatch, cap and join styles by name;
/// `edgecolor_setting` and `hatchcolor_setting` say whether the edges take
/// the faces' colours (`"face"`) and the hatches the edges' (`"edge"`).
fn get(collection: Collection, name: String) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let style = collection.style();
        let colors = || {
            collection
                .reported_colors()
                .map_err(|err| PyErr::value_error(err.to_string()))
        };
        let value = match name.as_str() {
            "facecolor" => rgba_array(&colors()?.faces)?.into_py(py)?,
            "edgecolor" => rgba_array(&colors()?.edges)?.into_py(py)?,
            "hatchcolor" => rgba_array(&colors()?.hatches)?.into_py(py)?,
            "hatchcolor_setting" => match style.hatchcolors {
                HatchColors::Edge => "edge".into_py(py)?,
                HatchColors::Colors(_) => Bound::none(py),
            },
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

/// The coordinates of the offsets, `None` when the items are placed
/// without offsets.
fn get_offset_transform(collection: Collection) -> PyResult<Option<Transform>> {
    Ok(collection.offset_transform())
}

/// Places the offsets in the coordinates `transform`.
fn set_offset_transform(collection: Collection, transform: Transform) -> PyResult<()> {
    collection.set_offset_transform(Some(transform));
    Ok(())
}

/// Replaces the curves of a fill between curves, keeping its steps and
/// interpolation (matplotlib's `FillBetweenPolyCollection.set_data`).
fn set_fill_between_data(
    collection: Collection,
    (t, f1, f2): (NdArray, NdArray, NdArray),
    mask: Option<Vec<bool>>,
) -> PyResult<()> {
    collection.set_fill_between_data(&t.data, &f1.data, &f2.data, mask.as_deref())?;
    Ok(())
}

/// The box of the points a fill between curves fills, `(x0, y0, x1,
/// y1)`, and their smallest positive coordinates (matplotlib's
/// `FillBetweenPolyCollection._bbox` and its `minpos`).
type FilledLimits = ((f64, f64, f64, f64), (f64, f64));

/// The [`FilledLimits`] of a fill between curves, `None` for other
/// collections.
fn fill_between_limits(collection: Collection) -> PyResult<Option<FilledLimits>> {
    Ok(collection.fill_between_limits().map(|limits| {
        let bbox = limits.bbox;
        (
            (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()),
            (limits.minpos.x, limits.minpos.y),
        )
    }))
}

/// The vertex grid of a quad mesh as a `(rows, columns, 2)` array, `None`
/// for other collections (matplotlib's `get_coordinates`).
fn get_coordinates(collection: Collection) -> PyResult<Option<NdArray>> {
    collection
        .mesh_coordinates()
        .map(|grid| {
            let (rows, cols) = grid.shape();
            let data = grid.as_slice().iter().flat_map(|p| [p.x, p.y]).collect();
            NdArray::with_shape(data, vec![rows, cols, 2])
        })
        .transpose()
}

/// Gives a quad mesh the vertices `coordinates`, a `(rows, columns, 2)`
/// array of the shape it has (the array `get_coordinates` handed out,
/// changed in place); nothing for another shape or another collection.
fn set_coordinates(collection: Collection, coordinates: NdArray) -> PyResult<()> {
    collection.set_mesh_coordinates(new::vertex_grid(&coordinates)?);
    Ok(())
}

/// The `(rows, columns)` of a mesh's vertex grid, without copying it
/// (`get_coordinates().shape[:2]`); `None` for other collections.
fn get_mesh_shape(collection: Collection) -> PyResult<Option<(usize, usize)>> {
    Ok(collection.mesh_shape())
}

/// How the cells of a mesh are coloured, `"flat"` or `"gouraud"`; `None`
/// for other collections.
fn get_mesh_shading(collection: Collection) -> PyResult<Option<&'static str>> {
    Ok(collection.mesh_shading().map(|shading| match shading {
        MeshShading::Flat => "flat",
        MeshShading::Gouraud => "gouraud",
    }))
}

/// The positions of the items, as an `(n, 2)` array (NaN where masked),
/// and whether they are a masked array.
fn get_offsets(collection: Collection) -> PyResult<(NdArray, bool)> {
    let offsets = collection.offsets();
    Ok((
        points_array(&offsets, vec![offsets.len(), 2])?,
        collection.offsets_masked(),
    ))
}

/// Moves the items to `offsets` (an `(n, 2)` array, NaN where masked);
/// `masked` tells whether they are a masked array.
fn set_offsets(collection: Collection, offsets: NdArray, masked: bool) -> PyResult<()> {
    collection.set_offsets_masked(points(&offsets)?, masked);
    Ok(())
}

/// The shape transform of each item drawn at `dpi`, as an `(n, 3, 3)`
/// array (matplotlib's `get_transforms`).
fn get_transforms(collection: Collection, dpi: f64) -> PyResult<NdArray> {
    let transforms = collection.item_transforms(dpi);
    let data = transforms
        .iter()
        .flat_map(|t| t.matrix().concat())
        .collect();
    NdArray::with_shape(data, vec![transforms.len(), 3, 3])
}

/// Replaces the shape transform of each item with those of an `(n, 3, 3)`
/// array (matplotlib's `_transforms` assigned).
fn set_transforms(collection: Collection, matrices: NdArray) -> PyResult<()> {
    let transforms = match matrices.shape[..] {
        [_, 3, 3] => matrices
            .data
            .as_chunks::<9>()
            .0
            .iter()
            .map(|m| Affine2D::from_values(m[0], m[3], m[1], m[4], m[2], m[5]))
            .collect(),
        [0] => Vec::new(),
        _ => {
            return Err(PyErr::value_error(format!(
                "transforms are an (n, 3, 3) array, not of shape {:?}",
                matrices.shape
            )));
        }
    };
    collection.set_item_transforms(transforms);
    Ok(())
}

/// Sets the colours drawn in the gaps of dashed edges (an `(N, 4)` RGBA
/// array), or leaves the gaps empty with `None`.
fn set_gapcolors(collection: Collection, colors: Option<NdArray>) -> PyResult<()> {
    collection.set_gapcolors(colors.map(|colors| rgba_rows(&colors)).transpose()?);
    Ok(())
}

/// The colours drawn in the gaps, the collection's alpha applied, as an
/// `(N, 4)` RGBA array; `None` when the gaps stay empty.
fn get_gapcolors(collection: Collection) -> PyResult<Option<NdArray>> {
    collection
        .gapcolors()
        .map_err(|err| PyErr::value_error(err.to_string()))?
        .map(|colors| rgba_array(&colors))
        .transpose()
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

/// Gives the collection one path where a sequence of paths is expected
/// (matplotlib's `set_paths(path)`), which drawing refuses.
fn set_single_path(collection: Collection, path: PyPath) -> PyResult<()> {
    collection.set_single_path(path.path);
    Ok(())
}

/// Whether the collection's paths are one path given in place of a
/// sequence.
fn has_single_path(collection: Collection) -> PyResult<bool> {
    Ok(collection.has_single_path())
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
    Ok(collection.mapped_values(|values| {
        NdArray::from_vec(
            values
                .iter()
                .map(|value| value.unwrap_or(f64::NAN))
                .collect(),
        )
    }))
}

/// Maps `values` (`NaN` masked) to colours; `None` drops the mapping.
fn set_array(collection: Collection, values: Option<NdArray>) -> PyResult<()> {
    match values {
        Some(values) => {
            collection
                .set_array(masked(values)?)
                .map_err(|err| PyErr::value_error(err.to_string()))?;
        }
        None => {
            collection.clear_array();
        }
    }
    Ok(())
}

/// Maps `values` (`NaN` masked) to colours, through `cmap` if the
/// collection maps none yet: the colormap of its colorizer, which
/// matplotlib resolves (`image.cmap` by default) in the registry the
/// program may have replaced (ultraplot's); `integers` when numpy holds
/// them as integers or booleans, which an identity norm indexes the
/// colormap with.
fn set_mapped_array(
    collection: Collection,
    values: NdArray,
    cmap: Colormap,
    integers: bool,
) -> PyResult<()> {
    let collection = collection
        .set_array_mapped_by(masked(values)?, cmap)
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    if integers {
        collection.set_value_kind(ScalarKind::Integer);
    }
    Ok(())
}

/// `values` with the values that are not finite masked.
fn masked(values: NdArray) -> PyResult<Vec<Option<f64>>> {
    Ok(values
        .into_vec()?
        .into_iter()
        .map(|value| value.is_finite().then_some(value))
        .collect())
}

/// Has the values mapped be integers or booleans, as numpy holds them,
/// which an identity norm indexes the colormap with.
fn set_integer_values(collection: Collection) -> PyResult<()> {
    collection.set_value_kind(ScalarKind::Integer);
    Ok(())
}

/// The colour limits of the mapping.
fn get_clim(collection: Collection) -> PyResult<(Option<f64>, Option<f64>)> {
    Ok(collection.clim())
}

/// The colormap of the mapping, `None` without a colour mapping.
fn get_cmap(collection: Collection) -> PyResult<Option<NativeColormap>> {
    Ok(collection.cmap().map(NativeColormap))
}

/// Maps the values of the collection, if it maps any, through `cmap` and
/// `norm` (the Python colorizer's, handed over when it changes): a new
/// norm when `replaced`, else the collection's norm with new parameters.
fn set_mapping(
    collection: Collection,
    cmap: Colormap,
    norm: Box<dyn Norm>,
    replaced: bool,
) -> PyResult<()> {
    collection.set_cmap(cmap);
    if replaced {
        collection.set_norm(norm);
    } else {
        collection.retune_norm(norm);
    }
    Ok(())
}

/// Whether the collection is in its figure.
fn is_attached(collection: Collection) -> PyResult<bool> {
    Ok(collection.raw().is_attached())
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

/// The full widths, the full heights and the angles (degrees) of the
/// ellipses of an ellipse collection (matplotlib's `get_widths`,
/// `get_heights` and `get_angles`).
fn ellipse_sizes(ellipses: EllipseCollection) -> PyResult<(NdArray, NdArray, NdArray)> {
    Ok((
        NdArray::from_vec(ellipses.widths()),
        NdArray::from_vec(ellipses.heights()),
        NdArray::from_vec(ellipses.angles()),
    ))
}

/// Sets those of the full widths, full heights and angles (degrees) of the
/// ellipses of an ellipse collection that are given.
fn set_ellipse_sizes(
    ellipses: EllipseCollection,
    (widths, heights, angles): (Option<NdArray>, Option<NdArray>, Option<NdArray>),
) -> PyResult<()> {
    if let Some(widths) = widths {
        ellipses.set_widths(&widths.into_vec()?);
    }
    if let Some(heights) = heights {
        ellipses.set_heights(&heights.into_vec()?);
    }
    if let Some(angles) = angles {
        ellipses.set_angles(&angles.into_vec()?);
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("ellipse_sizes", ellipse_sizes)?;
    module.add("set_ellipse_sizes", set_ellipse_sizes)?;
    module.add("get", get)?;
    module.add("set_facecolors", set_facecolors)?;
    module.add("give_facecolors", give_facecolors)?;
    module.add("give_edgecolors", give_edgecolors)?;
    module.add("set_edgecolors", set_edgecolors)?;
    module.add("decide_default_edges_as", decide_default_edges_as)?;
    module.add("set_hatchcolors", set_hatchcolors)?;
    module.add("set_linewidths", set_linewidths)?;
    module.add("set_linestyles", set_linestyles)?;
    module.add("set_antialiaseds", set_antialiaseds)?;
    module.add("set_alphas", set_alphas)?;
    module.add("set_alpha", set_alpha)?;
    module.add("update_scalarmappable", update_scalarmappable)?;
    module.add("set_hatch", set_hatch)?;
    module.add("set_hatch_linewidth", set_hatch_linewidth)?;
    module.add("set_capstyle", set_capstyle)?;
    module.add("set_joinstyle", set_joinstyle)?;
    module.add("set_pickradius", set_pickradius)?;
    module.add("get_pickradius", get_pickradius)?;
    module.add("set_urls", set_urls)?;
    module.add("get_urls", get_urls)?;
    module.add("get_offset_transform", get_offset_transform)?;
    module.add("set_offset_transform", set_offset_transform)?;
    module.add("set_fill_between_data", set_fill_between_data)?;
    module.add("fill_between_limits", fill_between_limits)?;
    module.add("get_coordinates", get_coordinates)?;
    module.add("set_coordinates", set_coordinates)?;
    module.add("get_mesh_shape", get_mesh_shape)?;
    module.add("get_mesh_shading", get_mesh_shading)?;
    module.add("get_offsets", get_offsets)?;
    module.add("set_offsets", set_offsets)?;
    module.add("get_transforms", get_transforms)?;
    module.add("set_transforms", set_transforms)?;
    module.add("set_gapcolors", set_gapcolors)?;
    module.add("get_gapcolors", get_gapcolors)?;
    module.add("get_paths", get_paths)?;
    module.add("set_paths", set_paths)?;
    module.add("set_single_path", set_single_path)?;
    module.add("has_single_path", has_single_path)?;
    module.add("set_segments", set_segments)?;
    module.add("set_verts", set_verts)?;
    module.add("get_sizes", get_sizes)?;
    module.add("set_sizes", set_sizes)?;
    module.add("get_array", get_array)?;
    module.add("set_array", set_array)?;
    module.add("set_mapped_array", set_mapped_array)?;
    module.add("set_integer_values", set_integer_values)?;
    module.add("get_clim", get_clim)?;
    module.add("get_cmap", get_cmap)?;
    module.add("set_mapping", set_mapping)?;
    module.add("is_attached", is_attached)?;
    module.add("set_event_positions", set_event_positions)?;
    module.add("event_orientation", event_orientation)?;
    module.add("set_event_orientation", set_event_orientation)?;
    module.add("event_lines", event_lines)?;
    module.add("set_event_lines", set_event_lines)?;
    copy::register(module)?;
    held::register(module)?;
    mapped::register(module)?;
    new::register(module)?;
    tri_mesh::register(module)
}
