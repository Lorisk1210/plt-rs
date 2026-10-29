//! `_native.held`: artists Python makes before adding them (matplotlib's
//! `Rectangle(...)`, `Line2D(...)`, `LineCollection(...)` before
//! `ax.add_patch`, `ax.add_line`, `ax.add_collection`), and their adding.
//!
//! Each thread has a holding figure, never drawn, in which these artists
//! are made (see [`pltrs::figure::Figure::hold_patch`]); their handles
//! work as any other. Adding one moves it into the axes
//! ([`pltrs::axes::Axes::adopt_held`]), and the Python object then wraps
//! the handle returned.

use pltrs::artist::{AttachError, RawHandle};
use pltrs::axes::Axes;
use pltrs::collections::{Collection, CollectionSpec, EllipseUnits};
use pltrs::figure::{AdoptAs, Figure};
use pltrs::lines::Line2D;
use pltrs::orientation::Orientation;
use pltrs::patches::{
    Annulus, Arc, Arrow, ArrowShape, ArrowStyle, BoxStyle, Circle, CirclePolygon, ConnectionStyle,
    Ellipse, FancyArrow, FancyArrowPatch, FancyBboxPatch, Patch, PathShape, Polygon, Rectangle,
    RegularPolygon, StepBaseline, StepPatch, Wedge,
};
use pltrs_core::Point;

use super::lines::LineKwargs;
use super::patch_styles::style_error;
use super::patches::collection_patches;
use super::path::PyPath;
use super::transforms::points;
use crate::binding::prelude::*;

thread_local! {
    /// The holding figure of the thread.
    static HOLDER: Figure = Figure::new();
}

/// The holding figure of this thread.
fn holder() -> Figure {
    HOLDER.with(Figure::clone)
}

/// matplotlib's `ValueError` for an artist that cannot be added.
fn attach_error(err: AttachError) -> PyErr {
    PyErr::value_error(match err {
        AttachError::NotRemoved => {
            "Can not reset the Axes. You are probably trying to re-use an artist in more \
             than one Axes which is not supported"
                .to_owned()
        }
        other => other.to_string(),
    })
}

/// Adds the held (or removed) `artist` to `axes` as `how` names
/// matplotlib's adding method: `"patch"`, `"line"`, `"collection"`
/// (`autolim`), or `"artist"`; returns the handle that replaces the
/// artist's old one.
fn adopt(axes: Axes, artist: RawHandle, how: String, autolim: bool) -> PyResult<RawHandle> {
    let how = match how.as_str() {
        "patch" => AdoptAs::Patch,
        "line" => AdoptAs::Line,
        "collection" => AdoptAs::Collection { autolim },
        _ => AdoptAs::Artist,
    };
    axes.adopt_held(&artist, how).map_err(attach_error)
}

/// A held rectangle (matplotlib's `Rectangle(xy, width, height, angle=)`).
fn rectangle(
    xy: (f64, f64),
    (width, height): (f64, f64),
    angle: f64,
) -> PyResult<Patch<Rectangle>> {
    let mut shape = Rectangle::new(xy, width, height);
    shape.angle = angle;
    Ok(holder().hold_patch(shape))
}

/// A held ellipse.
fn ellipse(xy: (f64, f64), (width, height): (f64, f64), angle: f64) -> PyResult<Patch<Ellipse>> {
    let mut shape = Ellipse::new(xy, width, height);
    shape.angle = angle;
    Ok(holder().hold_patch(shape))
}

/// A held circle.
fn circle(xy: (f64, f64), radius: f64) -> PyResult<Patch<Circle>> {
    Ok(holder().hold_patch(Circle::new(xy, radius)))
}

/// A held polygon through `(N, 2)` vertices.
fn polygon(vertices: NdArray, closed: bool) -> PyResult<Patch<Polygon>> {
    Ok(holder().hold_patch(Polygon::new(points(&vertices)?, closed)))
}

/// A held wedge.
fn wedge(
    center: (f64, f64),
    r: f64,
    (theta1, theta2): (f64, f64),
    width: Option<f64>,
) -> PyResult<Patch<Wedge>> {
    let mut shape = Wedge::new(center, r, theta1, theta2);
    shape.width = width;
    Ok(holder().hold_patch(shape))
}

/// A held path patch.
fn path_patch(path: PyPath) -> PyResult<Patch<PathShape>> {
    Ok(holder().hold_patch(PathShape(path.path)))
}

/// A held regular polygon.
fn regular_polygon(
    xy: (f64, f64),
    sides: usize,
    radius: f64,
    orientation: f64,
) -> PyResult<Patch<RegularPolygon>> {
    let mut shape = RegularPolygon::new(xy, sides, radius);
    shape.orientation = orientation;
    Ok(holder().hold_patch(shape))
}

/// A held polygonal circle of `resolution` sides.
fn circle_polygon(
    xy: (f64, f64),
    radius: f64,
    resolution: usize,
) -> PyResult<Patch<CirclePolygon>> {
    Ok(holder().hold_patch(CirclePolygon::new(xy, radius).with_resolution(resolution)))
}

/// A held elliptical arc from `theta1` to `theta2` degrees.
fn arc(
    xy: (f64, f64),
    (width, height): (f64, f64),
    angle: f64,
    (theta1, theta2): (f64, f64),
) -> PyResult<Patch<Arc>> {
    let mut shape = Arc::new(xy, width, height).with_angle(angle);
    shape.theta1 = theta1;
    shape.theta2 = theta2;
    Ok(holder().hold_patch(shape))
}

/// A held elliptical ring of outer semi-axes `(a, b)`.
fn annulus(
    xy: (f64, f64),
    semi_axes: (f64, f64),
    width: f64,
    angle: f64,
) -> PyResult<Patch<Annulus>> {
    let shape = Annulus::elliptical(xy, semi_axes, width, angle).map_err(|_| {
        PyErr::value_error("Width of annulus must be less than or equal to semi-minor axis")
    })?;
    Ok(holder().hold_patch(shape))
}

/// A held arrow (matplotlib's `Arrow`).
fn arrow((x, y): (f64, f64), (dx, dy): (f64, f64), width: f64) -> PyResult<Patch<Arrow>> {
    let mut shape = Arrow::new(x, y, dx, dy);
    shape.width = width;
    Ok(holder().hold_patch(shape))
}

/// The head of a fancy arrow: `(head_width, head_length, shape, overhang,
/// length_includes_head, head_starts_at_zero)`.
type ArrowHead = (Option<f64>, Option<f64>, String, f64, bool, bool);

/// A held fancy arrow (matplotlib's `FancyArrow`).
fn fancy_arrow(
    (x, y): (f64, f64),
    (dx, dy): (f64, f64),
    width: f64,
    (head_width, head_length, shape, overhang, includes_head, starts_at_zero): ArrowHead,
) -> PyResult<Patch<FancyArrow>> {
    let arrow = FancyArrow {
        width,
        head_width,
        head_length,
        shape: match shape.as_str() {
            "left" => ArrowShape::Left,
            "right" => ArrowShape::Right,
            "full" => ArrowShape::Full,
            other => {
                return Err(PyErr::value_error(format!("Got unknown shape: {other:?}")));
            }
        },
        overhang,
        length_includes_head: includes_head,
        head_starts_at_zero: starts_at_zero,
        ..FancyArrow::new(x, y, dx, dy)
    };
    Ok(holder().hold_patch(arrow))
}

/// A held step patch: the values, edges, baseline (`None`, one or one per
/// step) and orientation.
fn step_patch(
    values: NdArray,
    edges: NdArray,
    baseline: Option<NdArray>,
    horizontal: bool,
) -> PyResult<Patch<StepPatch>> {
    let baseline = match baseline {
        None => StepBaseline::None,
        Some(base) if base.shape.is_empty() => StepBaseline::Value(base.data[0]),
        Some(base) => StepBaseline::PerStep(base.into_vec()?),
    };
    let shape = StepPatch::new(values.into_vec()?, edges.into_vec()?)
        .and_then(|shape| shape.with_baseline(baseline))
        .map_err(|err| PyErr::value_error(err.to_string()))?
        .with_orientation(if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        });
    Ok(holder().hold_patch(shape))
}

/// A held fancy box (matplotlib's `FancyBboxPatch`): the box style as its
/// description string, the mutation scale and aspect.
fn fancy_bbox(
    xy: (f64, f64),
    (width, height): (f64, f64),
    boxstyle: String,
    (mutation_scale, mutation_aspect): (f64, f64),
) -> PyResult<Patch<FancyBboxPatch>> {
    let mut shape = FancyBboxPatch::new(
        xy,
        width,
        height,
        boxstyle.parse::<BoxStyle>().map_err(style_error)?,
    );
    shape.mutation_scale = mutation_scale;
    shape.mutation_aspect = mutation_aspect;
    Ok(holder().hold_patch(shape))
}

/// The route of a fancy arrow: between two points, or along a path.
type ArrowEnds = (Option<((f64, f64), (f64, f64))>, Option<PyPath>);

/// The styles of a fancy arrow: `(arrowstyle, connectionstyle, shrinkA,
/// shrinkB, mutation_scale, mutation_aspect)`, the styles as description
/// strings.
type ArrowStyles = (String, String, f64, f64, f64, f64);

/// A held fancy arrow patch (matplotlib's `FancyArrowPatch`).
fn fancy_arrow_patch(
    (ends, path): ArrowEnds,
    (arrowstyle, connectionstyle, shrink_a, shrink_b, scale, aspect): ArrowStyles,
) -> PyResult<Patch<FancyArrowPatch>> {
    let mut shape = match (ends, path) {
        (_, Some(path)) => FancyArrowPatch::along(path.path),
        (Some((a, b)), None) => FancyArrowPatch::between(a, b),
        (None, None) => {
            return Err(PyErr::value_error(
                "Either posA and posB, or path need to provided",
            ));
        }
    }
    .with_arrowstyle(arrowstyle.parse::<ArrowStyle>().map_err(style_error)?)
    .with_connectionstyle(
        connectionstyle
            .parse::<ConnectionStyle>()
            .map_err(style_error)?,
    )
    .with_shrink(shrink_a, shrink_b)
    .with_mutation_scale(scale);
    shape.spec.mutation_aspect = aspect;
    Ok(holder().hold_patch(shape))
}

/// A held line through `(x, y)` with the keyword arguments of `Line2D`.
fn line(x: NdArray, y: NdArray, kwargs: LineKwargs) -> PyResult<Line2D> {
    Ok(holder().hold_line(x.into_vec()?, y.into_vec()?, kwargs.0))
}

/// `(N, 2)` arrays as point lists.
fn point_lists(arrays: &[NdArray]) -> PyResult<Vec<Vec<Point>>> {
    arrays.iter().map(points).collect()
}

/// A held line collection of polylines (matplotlib's `LineCollection`).
fn line_collection(segments: Vec<NdArray>) -> PyResult<Collection> {
    Ok(holder().hold_collection(CollectionSpec::lines(point_lists(&segments)?)))
}

/// A held collection of closed polygons (matplotlib's `PolyCollection`).
fn poly_collection(verts: Vec<NdArray>) -> PyResult<Collection> {
    Ok(holder().hold_collection(CollectionSpec::polygons(point_lists(&verts)?)))
}

/// A held collection of paths (matplotlib's `PathCollection`).
fn path_collection(paths: Vec<PyPath>) -> PyResult<Collection> {
    let paths = paths.into_iter().map(|path| path.path).collect();
    Ok(holder().hold_collection(CollectionSpec::paths(paths)))
}

/// A held event collection (matplotlib's `EventCollection`): event lines
/// at `positions` along x (along y when `vertical`), centred on
/// `lineoffset` and `linelength` long.
fn event_collection(
    positions: NdArray,
    vertical: bool,
    lineoffset: f64,
    linelength: f64,
) -> PyResult<Collection> {
    let orientation = if vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    };
    let spec = CollectionSpec::events(&positions.into_vec()?, orientation, lineoffset, linelength);
    Ok(holder().hold_collection(spec))
}

/// A held collection of the outlines of `patches` (native patch objects;
/// matplotlib's `PatchCollection`), each keeping its own look when
/// `match_original`.
fn patch_collection(patches: Vec<Owned>, match_original: bool) -> PyResult<Collection> {
    let items = collection_patches(&patches)?;
    let spec = CollectionSpec::patches(&items, match_original)?;
    Ok(holder().hold_collection(spec))
}

/// A held collection of circles of areas `sizes` in points²
/// (matplotlib's `CircleCollection`).
fn circle_collection(sizes: NdArray) -> PyResult<Collection> {
    Ok(holder().hold_collection(CollectionSpec::circles(sizes.into_vec()?)))
}

/// A held collection of regular polygons (matplotlib's
/// `RegularPolyCollection`).
fn regular_poly_collection(numsides: usize, rotation: f64, sizes: NdArray) -> PyResult<Collection> {
    let spec = CollectionSpec::regular_polygons(numsides, rotation, sizes.into_vec()?);
    Ok(holder().hold_collection(spec))
}

/// A held collection of ellipses (matplotlib's `EllipseCollection`), with
/// the units of their sizes.
fn ellipse_collection(
    (widths, heights, angles): (NdArray, NdArray, NdArray),
    units: String,
) -> PyResult<Collection> {
    let units = match units.as_str() {
        "xy" => EllipseUnits::Xy,
        "x" => EllipseUnits::X,
        "y" => EllipseUnits::Y,
        "inches" => EllipseUnits::Inches,
        "width" => EllipseUnits::Width,
        "height" => EllipseUnits::Height,
        "dots" => EllipseUnits::Dots,
        _ => EllipseUnits::Points,
    };
    let spec = CollectionSpec::ellipses(
        &widths.into_vec()?,
        &heights.into_vec()?,
        &angles.into_vec()?,
        units,
    );
    Ok(holder().hold_collection(spec))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("adopt", adopt)?;
    module.add("rectangle", rectangle)?;
    module.add("ellipse", ellipse)?;
    module.add("circle", circle)?;
    module.add("polygon", polygon)?;
    module.add("wedge", wedge)?;
    module.add("path_patch", path_patch)?;
    module.add("regular_polygon", regular_polygon)?;
    module.add("circle_polygon", circle_polygon)?;
    module.add("arc", arc)?;
    module.add("annulus", annulus)?;
    module.add("arrow", arrow)?;
    module.add("fancy_arrow", fancy_arrow)?;
    module.add("step_patch", step_patch)?;
    module.add("fancy_bbox", fancy_bbox)?;
    module.add("fancy_arrow_patch", fancy_arrow_patch)?;
    module.add("line", line)?;
    module.add("line_collection", line_collection)?;
    module.add("poly_collection", poly_collection)?;
    module.add("path_collection", path_collection)?;
    module.add("event_collection", event_collection)?;
    module.add("patch_collection", patch_collection)?;
    module.add("circle_collection", circle_collection)?;
    module.add("regular_poly_collection", regular_poly_collection)?;
    module.add("ellipse_collection", ellipse_collection)
}
