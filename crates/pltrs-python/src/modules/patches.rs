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
use pltrs::mplot3d::{ProjectedOutline, ProjectedPath};
use pltrs::patches::{
    Annulus, Arc, Arrow, Circle, CirclePolygon, ConnectionPatch, Ellipse, FancyArrow,
    FancyArrowPatch, FancyBboxPatch, Patch, PatchError, PatchProps, PatchShape, PathShape, Polygon,
    Rectangle, RegularPolygon, RotationPoint, Shadow, ShadowError, StepBaseline, StepPatch, Wedge,
};
use pltrs_core::{Affine2D, Path, Point};

use super::lines::{PyLineStyle, cap_style, join_style};
use super::path::{PathArrays, PyPath, path_arrays};
use super::transforms::{matrix, points, points_array};
use crate::binding::dict_items;
use crate::binding::prelude::*;

mod arrow_route;
mod background;
mod connection;
mod fancy;
mod geometry;
mod new;
mod style;

use style::StyleOps;

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
    Patch<Shadow<PathShape>>,
);

/// What every patch handle does, whatever its shape (matplotlib's `Patch`
/// base class).
trait PatchOps: StyleOps {
    /// The outline in the shape's own coordinates.
    fn path(&self) -> Path;
    /// The map of the outline into the patch's coordinates.
    fn patch_transform(&self) -> Affine2D;
    /// How many pieces each segment of the outline is split into before a
    /// curved projection bends it.
    fn interpolation_steps(&self) -> usize;
    /// Sets [`PatchOps::interpolation_steps`].
    fn set_interpolation_steps(&self, steps: usize);
    /// The patch as an item of a patch collection.
    fn collection_patch(&self) -> CollectionPatch;
    /// The patch's shadow, offset by `(ox, oy)` points and darkened by
    /// `shade` (see [`Shadow::outline_of`]).
    fn shadow(&self, ox: f64, oy: f64, shade: f64) -> Result<Shadow<PathShape>, ShadowError>;
    /// The outlines in display pixels in the figure as it stands.
    fn display_paths_as_is(&self) -> Result<Vec<Path>, PatchError>;
}

impl<S: PatchShape> PatchOps for Patch<S> {
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

    fn collection_patch(&self) -> CollectionPatch {
        CollectionPatch {
            path: self.shape().placed_path(),
            style: Patch::style(self),
            alpha: self.alpha(),
            edge_default: S::EDGE_DEFAULT,
        }
    }

    fn shadow(&self, ox: f64, oy: f64, shade: f64) -> Result<Shadow<PathShape>, ShadowError> {
        Shadow::outline_of(self, ox, oy, shade)
    }

    fn display_paths_as_is(&self) -> Result<Vec<Path>, PatchError> {
        Patch::display_paths_as_is(self)
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

impl AnyPatch {
    /// The patch as an artist painted as a patch.
    fn into_style(self) -> Box<dyn StyleOps> {
        self.0
    }
}

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
            BboxPatch, BboxConnector, BboxConnectorPatch,
            ProjectedOutline, ProjectedPath, Shadow<PathShape>, Shadow<Wedge>,
        )
        .map(Self)
        .ok_or_else(|| PyErr::type_error("expected a native patch handle"))
    }
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

/// The outline in the shape's own coordinates, as `(vertices, codes)`
/// (matplotlib's `get_path`).
fn get_path(patch: AnyPatch) -> PyResult<PathArrays> {
    path_arrays(&patch.0.path(), false)
}

/// The outlines in display pixels, as one path, in the figure as it
/// stands (matplotlib's `FancyArrowPatch._get_path_in_displaycoord`, which
/// places the arrow through `transData` with the views not autoscaled
/// yet); a patch that cannot be placed raises `ValueError`.
fn display_path(patch: AnyPatch) -> PyResult<PathArrays> {
    let paths = patch
        .0
        .display_paths_as_is()
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    path_arrays(&Path::compound(&paths), false)
}

/// Fails as matplotlib's `add_patch` fails to measure the patch (an arrow
/// whose connection cannot be made), in the figure as it stands.
fn check_measurable(patch: AnyPatch) -> PyResult<()> {
    patch
        .0
        .display_paths_as_is()
        .map(drop)
        .map_err(|err| PyErr::value_error(err.to_string()))
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

/// The interpolation steps all rectangles draw with, if set (matplotlib's
/// `Path.unit_rectangle()._interpolation_steps`, which every
/// `Rectangle.get_path()` is).
fn get_shared_rectangle_steps() -> PyResult<Option<usize>> {
    Ok(pltrs::patches::shared_rectangle_steps())
}

/// Sets the interpolation steps all rectangles draw with.
fn set_shared_rectangle_steps(steps: usize) -> PyResult<()> {
    pltrs::patches::set_shared_rectangle_steps(Some(steps));
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
type EllipseFields = (f64, f64, f64, f64, f64);

/// The [`EllipseFields`] of `ellipse`.
fn ellipse_fields(ellipse: &Ellipse) -> EllipseFields {
    (
        ellipse.center.x,
        ellipse.center.y,
        ellipse.width,
        ellipse.height,
        ellipse.angle,
    )
}

/// Gives `ellipse` the centre `(x, y)`, the axes `width` and `height` and
/// the `angle`.
fn place_ellipse(ellipse: &mut Ellipse, (x, y): (f64, f64), width: f64, height: f64, angle: f64) {
    ellipse.center = Point::new(x, y);
    ellipse.width = width;
    ellipse.height = height;
    ellipse.angle = angle;
}

/// `(x, y, width, height, angle)` of an ellipse.
fn ellipse(patch: Patch<Ellipse>) -> PyResult<EllipseFields> {
    Ok(ellipse_fields(&patch.shape()))
}

/// Sets an ellipse's geometry.
fn set_ellipse(
    patch: Patch<Ellipse>,
    center: (f64, f64),
    width: f64,
    height: f64,
    angle: f64,
) -> PyResult<()> {
    patch.update_shape(|shape| place_ellipse(shape, center, width, height, angle));
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

/// Sets the start and end angles of an arc, in degrees (what a class of
/// the program's computes when it is drawn).
fn set_arc_angles(patch: Patch<Arc>, theta1: f64, theta2: f64) -> PyResult<()> {
    patch.update_shape(|shape| {
        shape.theta1 = theta1;
        shape.theta2 = theta2;
    });
    Ok(())
}

/// `(x, y, width, height, angle)` of the ellipse a circle is.
fn circle(patch: Patch<Circle>) -> PyResult<EllipseFields> {
    Ok(ellipse_fields(&patch.shape().ellipse))
}

/// Sets the geometry of the ellipse a circle is.
fn set_circle(
    patch: Patch<Circle>,
    center: (f64, f64),
    width: f64,
    height: f64,
    angle: f64,
) -> PyResult<()> {
    patch.update_shape(|shape| place_ellipse(&mut shape.ellipse, center, width, height, angle));
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

/// Whether the steps of a step patch run along y (`'horizontal'`) rather
/// than x (matplotlib's `StepPatch.orientation`).
fn step_patch_horizontal(patch: Patch<StepPatch>) -> PyResult<bool> {
    Ok(patch.shape().orientation() == pltrs::orientation::Orientation::Horizontal)
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

/// Draws a step patch as `path` until its data are set again (matplotlib's
/// `set_path`, which `StepPatch` inherits from `PathPatch`).
fn set_step_path(patch: Patch<StepPatch>, path: PyPath) -> PyResult<()> {
    patch.set_shape(patch.shape().with_path(path.path));
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
    background::register(module)?;
    module.add("update", style::update)?;
    module.add("get", style::get)?;
    module.add("get_path", get_path)?;
    module.add("display_path", display_path)?;
    module.add("check_measurable", check_measurable)?;
    module.add("get_interpolation_steps", get_interpolation_steps)?;
    module.add("set_interpolation_steps", set_interpolation_steps)?;
    module.add("get_shared_rectangle_steps", get_shared_rectangle_steps)?;
    module.add("set_shared_rectangle_steps", set_shared_rectangle_steps)?;
    module.add("get_patch_transform", get_patch_transform)?;
    module.add("rectangle", rectangle)?;
    module.add("set_rectangle", set_rectangle)?;
    module.add("ellipse", ellipse)?;
    module.add("set_ellipse", set_ellipse)?;
    module.add("set_arc", set_arc)?;
    module.add("set_arc_angles", set_arc_angles)?;
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
    module.add("set_step_path", set_step_path)?;
    module.add("step_patch_horizontal", step_patch_horizontal)?;
    module.add("fancy_bbox", fancy_bbox)?;
    module.add("set_fancy_bbox", set_fancy_bbox)?;
    arrow_route::register(module)?;
    connection::register(module)?;
    fancy::register(module)?;
    geometry::register(module)?;
    new::register(module)
}
