//! `_native.patches`: shapes with a face and an edge (matplotlib's
//! `patches`).
//!
//! pltrs (Rust) types a patch handle by its shape (`Patch<Rectangle>`,
//! `Patch<Wedge>`, ...). The functions common to every patch take an
//! [`AnyPatch`], which accepts the native object of a patch of any shape;
//! each shape has a getter and a setter of its geometry for its Python
//! class.

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes_grid1::{BboxConnector, BboxConnectorPatch, BboxPatch};
use pltrs::collections::CollectionPatch;
use pltrs::colors::{Color, ColorError};
use pltrs::inset::InsetIndicator;
use pltrs::mplot3d::{ProjectedOutline, ProjectedPath};
use pltrs::patches::{
    Annulus, Arc, Arrow, Circle, CirclePolygon, ConnectionPatch, Ellipse, FancyArrow,
    FancyArrowPatch, FancyBboxPatch, Patch, PatchColors, PatchProps, PatchShape, PatchStyle,
    PathShape, Polygon, Rectangle, RegularPolygon, RotationPoint, StepBaseline, StepPatch, Wedge,
};
use pltrs::rc::ColorOrEdge;
use pltrs_core::{Affine2D, Path, Point};

use super::lines::{PyLineStyle, cap_style, join_style};
use super::path::{PathArrays, PyPath, path_arrays};
use super::transforms::{matrix, points, points_array};
use crate::binding::dict_items;
use crate::binding::prelude::*;

mod arrow_route;
mod new;

artist_handle!(
    Patch<Rectangle>,
    Patch<Ellipse>,
    Patch<Circle>,
    Patch<Polygon>,
    Patch<Wedge>,
    Patch<PathShape>,
    Patch<RegularPolygon>,
    Patch<CirclePolygon>,
    Patch<StepPatch>,
    Patch<FancyArrow>,
    Patch<Arrow>,
    Patch<Arc>,
    Patch<Annulus>,
    Patch<FancyBboxPatch>,
    Patch<FancyArrowPatch>,
    Patch<ConnectionPatch>,
    Patch<ProjectedOutline>,
    Patch<ProjectedPath>,
);

/// What every patch handle does, whatever its shape (matplotlib's `Patch`
/// base class).
trait PatchOps {
    /// The handle common to all artists.
    fn raw_handle(&self) -> &RawHandle;
    /// A copy of the edge/face/hatch properties.
    fn style(&self) -> PatchStyle;
    /// Changes the edge/face/hatch properties.
    fn update_style(&self, f: &mut dyn FnMut(&mut PatchStyle));
    /// Sets the properties of a property set.
    fn update_props(&self, props: PatchProps);
    /// The edge width, defaults applied.
    fn linewidth(&self) -> f64;
    /// The colours painted, defaults and alpha applied.
    fn colors(&self) -> Result<PatchColors, ColorError>;
    /// The outline in the shape's own coordinates.
    fn path(&self) -> Path;
    /// The map of the outline into the patch's coordinates.
    fn patch_transform(&self) -> Affine2D;
    /// How many pieces each segment of the outline is split into before a
    /// curved projection bends it.
    fn interpolation_steps(&self) -> usize;
    /// Sets [`PatchOps::interpolation_steps`].
    fn set_interpolation_steps(&self, steps: usize);
    /// Sets the hatch line width.
    fn set_hatch_linewidth(&self, width: f64);
    /// The patch as an item of a patch collection.
    fn collection_patch(&self) -> CollectionPatch;
}

impl<S: PatchShape> PatchOps for Patch<S> {
    fn raw_handle(&self) -> &RawHandle {
        self.raw()
    }

    fn style(&self) -> PatchStyle {
        Patch::style(self)
    }

    fn update_style(&self, f: &mut dyn FnMut(&mut PatchStyle)) {
        Patch::update_style(self, f);
    }

    fn update_props(&self, props: PatchProps) {
        Patch::update_props(self, props);
    }

    fn linewidth(&self) -> f64 {
        Patch::linewidth(self)
    }

    fn colors(&self) -> Result<PatchColors, ColorError> {
        Patch::colors(self)
    }

    fn path(&self) -> Path {
        self.shape().path()
    }

    fn patch_transform(&self) -> Affine2D {
        self.shape().patch_transform()
    }

    fn interpolation_steps(&self) -> usize {
        Patch::interpolation_steps(self)
    }

    fn set_interpolation_steps(&self, steps: usize) {
        Patch::set_interpolation_steps(self, steps);
    }

    fn set_hatch_linewidth(&self, width: f64) {
        Patch::set_hatch_linewidth(self, width);
    }

    fn collection_patch(&self) -> CollectionPatch {
        CollectionPatch {
            path: self.shape().placed_path(),
            style: Patch::style(self),
            alpha: self.alpha(),
            edge_default: S::EDGE_DEFAULT,
        }
    }
}

/// The patches of `objects` (native patch objects of any shape) as the
/// items of a patch collection (matplotlib's `PatchCollection`).
pub(super) fn collection_patches(objects: &[Owned]) -> PyResult<Vec<CollectionPatch>> {
    Python::with_gil(|py| {
        objects
            .iter()
            .map(|object| Ok(object.bind(py).extract::<AnyPatch>()?.0.collection_patch()))
            .collect()
    })
}

/// The native object of a patch of any shape.
struct AnyPatch(Box<dyn PatchOps>);

/// Tries the patch shapes in turn on a raw handle.
macro_rules! downcast_patch {
    ($raw:expr; $($shape:ty),* $(,)?) => {{
        let raw = $raw;
        None$(.or_else(|| raw.downcast::<Patch<$shape>>().map(|p| Box::new(p) as Box<dyn PatchOps>)))*
    }};
}

impl FromPy<'_> for AnyPatch {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let raw: RawHandle = object.extract()?;
        downcast_patch!(&raw;
            Rectangle, Ellipse, Circle, Polygon, Wedge, PathShape, RegularPolygon,
            CirclePolygon, StepPatch, FancyArrow, Arrow, Arc, Annulus, FancyBboxPatch,
            FancyArrowPatch, ConnectionPatch,
            BboxPatch, BboxConnector, BboxConnectorPatch, InsetIndicator,
            ProjectedOutline, ProjectedPath,
        )
        .map(Self)
        .ok_or_else(|| PyErr::type_error("expected a native patch handle"))
    }
}

/// A colour, `None` (the default) when the Python value is `None`.
fn optional_color(value: &Bound<'_>) -> PyResult<Option<Color>> {
    value.extract()
}

/// Applies the patch property `key` set to `value` (a resolved name of
/// matplotlib's `Patch.set`) to `patch`.
fn set_property(patch: &dyn PatchOps, key: &str, value: &Bound<'_>) -> PyResult<()> {
    let mut props = PatchProps::default();
    match key {
        "color" => {
            let color = optional_color(value)?;
            patch.update_style(&mut |style| {
                style.facecolor.clone_from(&color);
                style.edgecolor.clone_from(&color);
            });
        }
        "facecolor" => {
            let color = optional_color(value)?;
            patch.update_style(&mut |style| style.facecolor.clone_from(&color));
        }
        "edgecolor" => {
            let color = optional_color(value)?;
            patch.update_style(&mut |style| style.edgecolor.clone_from(&color));
        }
        "linewidth" => {
            let width: Option<f64> = value.extract()?;
            patch.update_style(&mut |style| style.linewidth = width);
        }
        "linestyle" => {
            let PyLineStyle(linestyle) = value.extract()?;
            patch.update_style(&mut |style| style.linestyle = linestyle.clone());
        }
        "fill" => {
            let fill: bool = value.extract()?;
            patch.update_style(&mut |style| style.fill = fill);
        }
        "hatch" => {
            let hatch: Option<String> = value.extract()?;
            patch.update_style(&mut |style| style.hatch.clone_from(&hatch));
        }
        "hatchcolor" => {
            let color = match value.extract::<Option<String>>() {
                Ok(Some(edge)) if edge == "edge" => ColorOrEdge::Edge,
                _ => ColorOrEdge::Color(value.extract()?),
            };
            patch.update_style(&mut |style| style.hatchcolor = color.clone());
        }
        "hatch_linewidth" => patch.set_hatch_linewidth(value.extract()?),
        "edgegapcolor" => {
            let color = optional_color(value)?.unwrap_or_else(Color::none);
            patch.update_style(&mut |style| style.edgegapcolor = color.clone());
        }
        "antialiased" => {
            let antialiased: Option<bool> = value.extract()?;
            patch.update_style(&mut |style| style.antialiased = antialiased);
        }
        "capstyle" => {
            let cap = cap_style(value)?;
            patch.update_style(&mut |style| style.capstyle = cap);
        }
        "joinstyle" => {
            let join = join_style(value)?;
            patch.update_style(&mut |style| style.joinstyle = join);
        }
        "label" => {
            props.base.label = Some(value.str()?);
            patch.update_props(props);
        }
        "zorder" => {
            props.base.zorder = Some(value.extract()?);
            patch.update_props(props);
        }
        "visible" => {
            props.base.visible = Some(value.extract()?);
            patch.update_props(props);
        }
        "clip_on" => {
            props.base.clip_on = Some(value.extract()?);
            patch.update_props(props);
        }
        "alpha" => {
            patch
                .raw_handle()
                .set_alpha(value.extract::<Option<f64>>()?);
        }
        _ => {
            return Err(PyErr::new(
                ExceptionKind::AttributeError,
                format!("Patch.set() got an unexpected keyword argument '{key}'"),
            ));
        }
    }
    Ok(())
}

/// The patch properties of a dict of keyword arguments whose aliases the
/// Python side resolved, for patches made with them (a plotting method's
/// `boxprops`, ...).
#[derive(Clone, Debug, Default)]
pub(super) struct PatchKwargs(pub(super) PatchProps);

impl FromPy<'_> for PatchKwargs {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut props = PatchProps::default();
        for (key, value) in dict_items(object)? {
            let key: String = key.extract()?;
            match key.as_str() {
                "color" => props.color = value.extract()?,
                "facecolor" => props.facecolor = value.extract()?,
                "edgecolor" => props.edgecolor = value.extract()?,
                "linewidth" => props.linewidth = value.extract()?,
                "linestyle" => props.linestyle = Some(value.extract::<PyLineStyle>()?.0),
                "fill" => props.fill = Some(value.extract()?),
                "hatch" => props.hatch = value.extract()?,
                "hatchcolor" => props.hatchcolor = value.extract()?,
                "antialiased" => props.antialiased = value.extract()?,
                "capstyle" => props.capstyle = Some(cap_style(&value)?),
                "joinstyle" => props.joinstyle = Some(join_style(&value)?),
                "alpha" => props.base.alpha = value.extract()?,
                "zorder" => props.base.zorder = Some(value.extract()?),
                "label" => props.base.label = Some(value.str()?),
                "visible" => props.base.visible = Some(value.extract()?),
                "clip_on" => props.base.clip_on = Some(value.extract()?),
                _ => {
                    return Err(PyErr::new(
                        ExceptionKind::AttributeError,
                        format!("PathPatch.set() got an unexpected keyword argument '{key}'"),
                    ));
                }
            }
        }
        Ok(Self(props))
    }
}

/// Sets the properties of a dict of keyword arguments whose aliases the
/// Python side resolved (matplotlib's `Patch.set`), in order.
fn update(patch: AnyPatch, kwargs: Owned) -> PyResult<()> {
    Python::with_gil(|py| {
        for (key, value) in dict_items(&kwargs.bind(py))? {
            let key: String = key.extract()?;
            set_property(patch.0.as_ref(), &key, &value)?;
        }
        Ok(())
    })
}

/// `(r, g, b, a)`.
fn rgba_tuple(rgba: pltrs::colors::Rgba) -> (f64, f64, f64, f64) {
    (rgba.r, rgba.g, rgba.b, rgba.a)
}

/// The patch property `name` as matplotlib's `get_<name>` returns it:
/// colours as RGBA tuples with the defaults and alpha applied, styles
/// by name.
fn get(patch: AnyPatch, name: String) -> PyResult<Owned> {
    let patch = patch.0;
    Python::with_gil(|py| {
        let style = patch.style();
        let value = match name.as_str() {
            "facecolor" => rgba_tuple(patch.colors()?.face).into_py(py)?,
            "edgecolor" => rgba_tuple(patch.colors()?.edge).into_py(py)?,
            "hatchcolor" => rgba_tuple(patch.colors()?.hatch).into_py(py)?,
            "linewidth" => patch.linewidth().into_py(py)?,
            "linestyle" => match style.linestyle {
                pltrs::lines::LineStyle::Dashes { offset, pattern } => {
                    (offset, pattern).into_py(py)?
                }
                named => named.to_string().into_py(py)?,
            },
            "fill" => style.fill.into_py(py)?,
            "hatch" => style.hatch.into_py(py)?,
            "hatch_linewidth" => style.hatch_linewidth.into_py(py)?,
            "edgegapcolor" => (!style.edgegapcolor.is_none())
                .then_some(style.edgegapcolor)
                .into_py(py)?,
            "antialiased" => style.antialiased.into_py(py)?,
            "capstyle" => style.capstyle.name().into_py(py)?,
            "joinstyle" => style.joinstyle.name().into_py(py)?,
            _ => {
                return Err(PyErr::new(
                    ExceptionKind::AttributeError,
                    format!("'Patch' object has no property '{name}'"),
                ));
            }
        };
        Ok(value.unbind())
    })
}

/// The outline in the shape's own coordinates, as `(vertices, codes)`
/// (matplotlib's `get_path`).
fn get_path(patch: AnyPatch) -> PyResult<PathArrays> {
    path_arrays(&patch.0.path(), false)
}

/// How many pieces each segment of the outline is split into before a
/// curved projection bends it (matplotlib's
/// `get_path()._interpolation_steps`).
fn get_interpolation_steps(patch: AnyPatch) -> PyResult<usize> {
    Ok(patch.0.interpolation_steps())
}

/// Sets how many pieces each segment of the outline is split into.
fn set_interpolation_steps(patch: AnyPatch, steps: usize) -> PyResult<()> {
    patch.0.set_interpolation_steps(steps);
    Ok(())
}

/// The matrix mapping the outline into the patch's coordinates
/// (matplotlib's `get_patch_transform`).
fn get_patch_transform(patch: AnyPatch) -> PyResult<NdArray> {
    matrix(patch.0.patch_transform())
}

/// `(x, y, width, height, angle, rotation_point)` of a rectangle, the
/// rotation point `"xy"`, `"center"` or a point.
type RectangleFields = (f64, f64, f64, f64, f64, Owned);

/// A rectangle's geometry.
fn rectangle(patch: Patch<Rectangle>) -> PyResult<RectangleFields> {
    let shape = patch.shape();
    let point = Python::with_gil(|py| {
        Ok::<_, PyErr>(
            match shape.rotation_point {
                RotationPoint::Xy => "xy".into_py(py),
                RotationPoint::Center => "center".into_py(py),
                RotationPoint::At(at) => (at.x, at.y).into_py(py),
            }?
            .unbind(),
        )
    })?;
    Ok((
        shape.xy.x,
        shape.xy.y,
        shape.width,
        shape.height,
        shape.angle,
        point,
    ))
}

/// Sets a rectangle's geometry (the fields of [`rectangle`]).
fn set_rectangle(
    patch: Patch<Rectangle>,
    (x, y): (f64, f64),
    (width, height): (f64, f64),
    angle: f64,
    rotation_point: Owned,
) -> PyResult<()> {
    let point = Python::with_gil(|py| rotation(&rotation_point.bind(py)))?;
    patch.update_shape(|shape| {
        shape.xy = Point::new(x, y);
        shape.width = width;
        shape.height = height;
        shape.angle = angle;
        shape.rotation_point = point;
    });
    Ok(())
}

/// A rectangle's rotation point: `"xy"`, `"center"` or a point.
fn rotation(rotation_point: &Bound<'_>) -> PyResult<RotationPoint> {
    Ok(match rotation_point.extract::<String>() {
        Ok(name) if name == "xy" => RotationPoint::Xy,
        Ok(name) if name == "center" => RotationPoint::Center,
        Ok(_) => {
            return Err(PyErr::value_error(
                "rotation_point must be one of {'xy', 'center', (number, number)}",
            ));
        }
        Err(_) => {
            let (px, py): (f64, f64) = rotation_point.extract().map_err(|_| {
                PyErr::value_error(
                    "rotation_point must be one of {'xy', 'center', (number, number)}",
                )
            })?;
            RotationPoint::At(Point::new(px, py))
        }
    })
}

/// `(x, y, width, height, angle)` of an ellipse.
fn ellipse(patch: Patch<Ellipse>) -> PyResult<(f64, f64, f64, f64, f64)> {
    let shape = patch.shape();
    Ok((
        shape.center.x,
        shape.center.y,
        shape.width,
        shape.height,
        shape.angle,
    ))
}

/// Sets an ellipse's geometry.
fn set_ellipse(
    patch: Patch<Ellipse>,
    (x, y): (f64, f64),
    width: f64,
    height: f64,
    angle: f64,
) -> PyResult<()> {
    patch.update_shape(|shape| {
        shape.center = Point::new(x, y);
        shape.width = width;
        shape.height = height;
        shape.angle = angle;
    });
    Ok(())
}

/// Gives `patch` the edge, face and hatch properties of `other`
/// (matplotlib's `Patch.update_from`).
fn update_style_from(patch: AnyPatch, other: AnyPatch) -> PyResult<()> {
    let style = other.0.style();
    patch.0.update_style(&mut |own| own.clone_from(&style));
    Ok(())
}

/// Sets the geometry of an arc's ellipse, its angles kept.
fn set_arc(
    patch: Patch<Arc>,
    (x, y): (f64, f64),
    width: f64,
    height: f64,
    angle: f64,
) -> PyResult<()> {
    patch.update_shape(|shape| {
        shape.center = Point::new(x, y);
        shape.width = width;
        shape.height = height;
        shape.angle = angle;
    });
    Ok(())
}

/// `(x, y, radius)` of a circle.
fn circle(patch: Patch<Circle>) -> PyResult<(f64, f64, f64)> {
    let shape = patch.shape();
    Ok((shape.center.x, shape.center.y, shape.radius))
}

/// Sets a circle's geometry.
fn set_circle(patch: Patch<Circle>, (x, y): (f64, f64), radius: f64) -> PyResult<()> {
    patch.update_shape(|shape| {
        shape.center = Point::new(x, y);
        shape.radius = radius;
    });
    Ok(())
}

/// The vertices (`(N, 2)`, closed polygons repeating the first) and
/// whether the polygon is closed.
fn polygon(patch: Patch<Polygon>) -> PyResult<(NdArray, bool)> {
    let shape = patch.shape();
    let vertices = shape.vertices();
    Ok((
        points_array(vertices, vec![vertices.len(), 2])?,
        shape.is_closed(),
    ))
}

/// Sets a polygon's vertices and closedness.
fn set_polygon(patch: Patch<Polygon>, vertices: NdArray, closed: bool) -> PyResult<()> {
    patch.set_shape(Polygon::new(points(&vertices)?, closed));
    Ok(())
}

/// `(x, y, r, theta1, theta2, width)` of a wedge.
type WedgeFields = (f64, f64, f64, f64, f64, Option<f64>);

/// A wedge's geometry.
fn wedge(patch: Patch<Wedge>) -> PyResult<WedgeFields> {
    let shape = patch.shape();
    Ok((
        shape.center.x,
        shape.center.y,
        shape.r,
        shape.theta1,
        shape.theta2,
        shape.width,
    ))
}

/// Sets a wedge's geometry.
fn set_wedge(
    patch: Patch<Wedge>,
    (x, y): (f64, f64),
    r: f64,
    (theta1, theta2): (f64, f64),
    width: Option<f64>,
) -> PyResult<()> {
    patch.update_shape(|shape| {
        shape.center = Point::new(x, y);
        shape.r = r;
        shape.theta1 = theta1;
        shape.theta2 = theta2;
        shape.width = width;
    });
    Ok(())
}

/// Replaces the path of a path patch.
fn set_path_shape(patch: Patch<PathShape>, path: PyPath) -> PyResult<()> {
    patch.set_shape(PathShape(path.path));
    Ok(())
}

/// `(x, y, sides, radius, orientation)` of a regular polygon.
fn regular_polygon(patch: Patch<RegularPolygon>) -> PyResult<(f64, f64, usize, f64, f64)> {
    let shape = patch.shape();
    Ok((
        shape.center.x,
        shape.center.y,
        shape.sides,
        shape.radius,
        shape.orientation,
    ))
}

/// `((x, y, width, height), (angle, theta1, theta2))` of an arc.
type ArcFields = ((f64, f64, f64, f64), (f64, f64, f64));

/// An arc's geometry.
fn arc(patch: Patch<Arc>) -> PyResult<ArcFields> {
    let shape = patch.shape();
    Ok((
        (shape.center.x, shape.center.y, shape.width, shape.height),
        (shape.angle, shape.theta1, shape.theta2),
    ))
}

/// The steps of a step patch: `(values, edges, baseline)`, the baseline
/// `None`, a number or an array.
fn step_patch(patch: Patch<StepPatch>) -> PyResult<(NdArray, NdArray, Owned)> {
    let shape = patch.shape();
    let baseline = Python::with_gil(|py| {
        Ok::<_, PyErr>(
            match shape.baseline() {
                StepBaseline::None => Ok(Bound::none(py)),
                StepBaseline::Value(value) => value.into_py(py),
                StepBaseline::PerStep(values) => NdArray::from_vec(values.clone()).into_py(py),
            }?
            .unbind(),
        )
    })?;
    Ok((
        NdArray::from_vec(shape.values().to_vec()),
        NdArray::from_vec(shape.edges().to_vec()),
        baseline,
    ))
}

/// Replaces the values, edges and baseline of a step patch; `None`
/// keeps a part, and `keep_baseline` keeps the baseline.
fn set_step_patch(
    patch: Patch<StepPatch>,
    values: Option<NdArray>,
    edges: Option<NdArray>,
    (keep_baseline, baseline): (bool, Option<NdArray>),
) -> PyResult<()> {
    let shape = patch.shape();
    let baseline = if keep_baseline {
        shape.baseline().clone()
    } else {
        match baseline {
            None => StepBaseline::None,
            Some(base) if base.shape.is_empty() => StepBaseline::Value(base.data[0]),
            Some(base) => StepBaseline::PerStep(base.into_vec()?),
        }
    };
    let orientation = shape.orientation();
    let updated = shape
        .with_data(
            values.map(NdArray::into_vec).transpose()?,
            edges.map(NdArray::into_vec).transpose()?,
        )
        .and_then(|shape| shape.with_baseline(baseline))
        .map_err(|err| PyErr::value_error(err.to_string()))?
        .with_orientation(orientation);
    patch.set_shape(updated);
    Ok(())
}

/// `(x, y, width, height)` of a fancy box.
fn fancy_bbox(patch: Patch<FancyBboxPatch>) -> PyResult<(f64, f64, f64, f64)> {
    let shape = patch.shape();
    Ok((shape.xy.x, shape.xy.y, shape.width, shape.height))
}

/// Sets the box of a fancy box.
fn set_fancy_bbox(
    patch: Patch<FancyBboxPatch>,
    (x, y): (f64, f64),
    width: f64,
    height: f64,
) -> PyResult<()> {
    patch.update_shape(|shape| {
        shape.xy = Point::new(x, y);
        shape.width = width;
        shape.height = height;
    });
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("update", update)?;
    module.add("get", get)?;
    module.add("get_path", get_path)?;
    module.add("get_interpolation_steps", get_interpolation_steps)?;
    module.add("set_interpolation_steps", set_interpolation_steps)?;
    module.add("get_patch_transform", get_patch_transform)?;
    module.add("rectangle", rectangle)?;
    module.add("set_rectangle", set_rectangle)?;
    module.add("ellipse", ellipse)?;
    module.add("set_ellipse", set_ellipse)?;
    module.add("set_arc", set_arc)?;
    module.add("update_style_from", update_style_from)?;
    module.add("circle", circle)?;
    module.add("set_circle", set_circle)?;
    module.add("polygon", polygon)?;
    module.add("set_polygon", set_polygon)?;
    module.add("wedge", wedge)?;
    module.add("set_wedge", set_wedge)?;
    module.add("set_path_shape", set_path_shape)?;
    module.add("regular_polygon", regular_polygon)?;
    module.add("arc", arc)?;
    module.add("step_patch", step_patch)?;
    module.add("set_step_patch", set_step_patch)?;
    module.add("fancy_bbox", fancy_bbox)?;
    module.add("set_fancy_bbox", set_fancy_bbox)?;
    arrow_route::register(module)?;
    new::register(module)
}
