//! The constructors of matplotlib's patch classes (`Rectangle(...)`,
//! `Circle(...)`, ...): each makes a loose pltrs (Rust) patch
//! ([`Patch::loose`]), styled by the rc parameters, which an axes or a
//! figure then adopts (`_native.artist.adopt_into_axes`).

use pltrs::orientation::Orientation;
use pltrs::patches::{
    Annulus, Arc, Arrow, ArrowShape, ArrowStyle, BoxStyle, Circle, CirclePolygon, ConnectionStyle,
    Ellipse, FancyArrow, FancyArrowPatch, FancyBboxPatch, Patch, PatchProps, PathShape, Polygon,
    Rectangle, RegularPolygon, Shadow, ShadowError, StepBaseline, StepPatch, Wedge,
};

use super::super::patch_styles::style_error;
use super::super::path::PyPath;
use super::super::transforms::points;
use super::AnyPatch;
use super::geometry::annulus_too_wide;
use crate::binding::prelude::*;

/// A loose rectangle (matplotlib's `Rectangle(xy, width, height, angle=)`).
fn rectangle(
    xy: (f64, f64),
    (width, height): (f64, f64),
    angle: f64,
) -> PyResult<Patch<Rectangle>> {
    let mut shape = Rectangle::new(xy, width, height);
    shape.angle = angle;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose ellipse.
fn ellipse(xy: (f64, f64), (width, height): (f64, f64), angle: f64) -> PyResult<Patch<Ellipse>> {
    let mut shape = Ellipse::new(xy, width, height);
    shape.angle = angle;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose circle.
fn circle(xy: (f64, f64), radius: f64) -> PyResult<Patch<Circle>> {
    Ok(Patch::loose(Circle::new(xy, radius), PatchProps::default()))
}

/// A loose polygon through `(N, 2)` vertices.
fn polygon(vertices: NdArray, closed: bool) -> PyResult<Patch<Polygon>> {
    Ok(Patch::loose(
        Polygon::new(points(&vertices)?, closed),
        PatchProps::default(),
    ))
}

/// A loose wedge.
fn wedge(
    center: (f64, f64),
    r: f64,
    (theta1, theta2): (f64, f64),
    width: Option<f64>,
) -> PyResult<Patch<Wedge>> {
    let mut shape = Wedge::new(center, r, theta1, theta2);
    shape.width = width;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose path patch.
fn path_patch(path: PyPath) -> PyResult<Patch<PathShape>> {
    Ok(Patch::loose(PathShape(path.path), PatchProps::default()))
}

/// A loose regular polygon.
fn regular_polygon(
    xy: (f64, f64),
    sides: usize,
    radius: f64,
    orientation: f64,
) -> PyResult<Patch<RegularPolygon>> {
    let mut shape = RegularPolygon::new(xy, sides, radius);
    shape.orientation = orientation;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose polygonal circle of `resolution` sides.
fn circle_polygon(
    xy: (f64, f64),
    radius: f64,
    resolution: usize,
) -> PyResult<Patch<CirclePolygon>> {
    Ok(Patch::loose(
        CirclePolygon::new(xy, radius).with_resolution(resolution),
        PatchProps::default(),
    ))
}

/// A loose elliptical arc from `theta1` to `theta2` degrees.
fn arc(
    xy: (f64, f64),
    (width, height): (f64, f64),
    angle: f64,
    (theta1, theta2): (f64, f64),
) -> PyResult<Patch<Arc>> {
    let mut shape = Arc::new(xy, width, height).with_angle(angle);
    shape.theta1 = theta1;
    shape.theta2 = theta2;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose elliptical ring of outer semi-axes `(a, b)`.
fn annulus(
    xy: (f64, f64),
    semi_axes: (f64, f64),
    width: f64,
    angle: f64,
) -> PyResult<Patch<Annulus>> {
    let shape = Annulus::elliptical(xy, semi_axes, width, angle).map_err(|_| annulus_too_wide())?;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose arrow (matplotlib's `Arrow`).
fn arrow((x, y): (f64, f64), (dx, dy): (f64, f64), width: f64) -> PyResult<Patch<Arrow>> {
    let mut shape = Arrow::new(x, y, dx, dy);
    shape.width = width;
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose shadow of `patch`, a patch of any shape, offset by `(ox, oy)`
/// points and darkened by `shade` (matplotlib's `Shadow(patch, ox, oy,
/// shade=)`).
fn shadow(patch: AnyPatch, (ox, oy): (f64, f64), shade: f64) -> PyResult<Patch<Shadow<PathShape>>> {
    let shadow = patch.0.shadow(ox, oy, shade).map_err(|err| match err {
        ShadowError::Shape(_) => PyErr::value_error("shade must be between 0 and 1."),
        ShadowError::Color(err) => PyErr::value_error(err.to_string()),
    })?;
    Ok(Patch::loose(shadow, PatchProps::default()))
}

/// The head of a fancy arrow: `(head_width, head_length, shape, overhang,
/// length_includes_head, head_starts_at_zero)`.
type ArrowHead = (Option<f64>, Option<f64>, String, f64, bool, bool);

/// A loose fancy arrow (matplotlib's `FancyArrow`).
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
                // Quoted as Python's `repr` does, as matplotlib's message.
                let quoted = Python::with_gil(|py| other.into_py(py)?.repr())?;
                return Err(PyErr::value_error(format!("Got unknown shape: {quoted}")));
            }
        },
        overhang,
        length_includes_head: includes_head,
        head_starts_at_zero: starts_at_zero,
        ..FancyArrow::new(x, y, dx, dy)
    };
    Ok(Patch::loose(arrow, PatchProps::default()))
}

/// A loose step patch: the values, edges, baseline (`None`, one or one per
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
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// A loose fancy box (matplotlib's `FancyBboxPatch`): the box style as its
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
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// The route of a fancy arrow: between two points, or along a path.
type ArrowEnds = (Option<((f64, f64), (f64, f64))>, Option<PyPath>);

/// The styles of a fancy arrow: `(arrowstyle, connectionstyle, shrinkA,
/// shrinkB, mutation_scale, mutation_aspect)`, the styles as description
/// strings.
type ArrowStyles = (String, String, f64, f64, f64, f64);

/// A loose fancy arrow patch (matplotlib's `FancyArrowPatch`).
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
    Ok(Patch::loose(shape, PatchProps::default()))
}

/// Registers the constructors, as `new_<shape>`.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_rectangle", rectangle)?;
    module.add("new_ellipse", ellipse)?;
    module.add("new_circle", circle)?;
    module.add("new_polygon", polygon)?;
    module.add("new_wedge", wedge)?;
    module.add("new_path_patch", path_patch)?;
    module.add("new_regular_polygon", regular_polygon)?;
    module.add("new_circle_polygon", circle_polygon)?;
    module.add("new_arc", arc)?;
    module.add("new_annulus", annulus)?;
    module.add("new_arrow", arrow)?;
    module.add("new_fancy_arrow", fancy_arrow)?;
    module.add("new_step_patch", step_patch)?;
    module.add("new_fancy_bbox", fancy_bbox)?;
    module.add("new_fancy_arrow_patch", fancy_arrow_patch)?;
    module.add("new_shadow", shadow)?;
    Ok(())
}
