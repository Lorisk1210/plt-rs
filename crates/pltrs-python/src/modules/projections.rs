//! `_native.projections`: projections the program defines (matplotlib's
//! `register_projection` of an `Axes` subclass with transforms, axes,
//! ticks and spines of its own, such as the SkewT-logP example's
//! `SkewXAxes`).
//!
//! Such an axes is a pltrs (Rust) axes drawn through a [`Projection`]
//! whose hooks ask the Python axes, through the adapter
//! `projections._program` makes for it: the transforms its
//! `_set_lim_and_transforms` built, its `Axis.get_view_interval`, its
//! `Tick.draw` and its `Spine._adjust_location`. While the figure is laid
//! out or drawn its axes cannot be asked about themselves, so every hook
//! hands the adapter the view limits pltrs (Rust) works with, which the
//! Python axes answers from meanwhile (its `viewLim`). A hook that raises
//! gives what rectilinear axes would, and its exception surfaces from the
//! native call that drew.

use std::rc::Rc;

use pltrs::axes::{Axes, AxisName, Sides};
use pltrs::projections::{Projection, ProjectionError, ProjectionView};
use pltrs::spines::SpineSide;
use pltrs::transforms::{Affine2D, ResolvedTransform, Step};

use super::transforms::{PyPointMap, affine};
use crate::binding::dict_value;
use crate::binding::prelude::*;

impl From<ProjectionError> for PyErr {
    fn from(err: ProjectionError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// A transform the adapter evaluated at a view: the matrix of its affine
/// part and, for a non-affine transform, the map of its non-affine part
/// (matplotlib's `transform_non_affine`, then `get_affine()`).
struct Evaluated {
    affine: Affine2D,
    non_affine: Option<PyPointMap>,
}

impl FromPy<'_> for Evaluated {
    /// `(matrix, map)`, the map `None` for an affine transform.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (matrix, map): (NdArray, Option<Bound<'_>>) = object.extract()?;
        Ok(Self {
            affine: affine(&matrix)?,
            non_affine: map.as_ref().map(PyPointMap::new).transpose()?,
        })
    }
}

impl Evaluated {
    /// The transform as pltrs (Rust) maps points through it.
    fn resolved(self) -> ResolvedTransform {
        match self.non_affine {
            None => ResolvedTransform::affine(self.affine),
            Some(map) => ResolvedTransform::with_step(Step::Plane(Rc::new(map)), self.affine),
        }
    }
}

/// The hooks the program's classes override, by axis or side; the others
/// are those of rectilinear axes and never call Python.
#[derive(Debug, Default)]
struct Overrides {
    /// Axes whose gridlines, ticks and spines have a transform of their
    /// own (`get_xaxis_transform`).
    axis_transform: Vec<AxisName>,
    /// Axes ticked over an interval of their own (`Axis.get_view_interval`).
    tick_interval: Vec<AxisName>,
    /// Axes whose ticks hide some of their parts (`Tick.draw`).
    tick_sides: Vec<AxisName>,
    /// Sides whose spine spans an interval of its own
    /// (`Spine._adjust_location`).
    spine_interval: Vec<SpineSide>,
}

/// The axis called `name`.
fn axis_name(name: &str) -> PyResult<AxisName> {
    match name {
        "x" => Ok(AxisName::X),
        "y" => Ok(AxisName::Y),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not an axis of 2-D axes"
        ))),
    }
}

/// The spine side called `name`.
fn spine_side(name: &str) -> PyResult<SpineSide> {
    SpineSide::from_name(name)
        .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a side of 2-D axes")))
}

impl FromPy<'_> for Overrides {
    /// A dict of the names of the axes or sides each hook is overridden
    /// for: `"axis_transform"`, `"tick_interval"`, `"tick_sides"` (axis
    /// names) and `"spine_interval"` (side names).
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let axes = |key: &str| -> PyResult<Vec<AxisName>> {
            dict_value::<Vec<String>>(object, key)?
                .iter()
                .map(|name| axis_name(name))
                .collect()
        };
        Ok(Self {
            axis_transform: axes("axis_transform")?,
            tick_interval: axes("tick_interval")?,
            tick_sides: axes("tick_sides")?,
            spine_interval: dict_value::<Vec<String>>(object, "spine_interval")?
                .iter()
                .map(|name| spine_side(name))
                .collect::<PyResult<_>>()?,
        })
    }
}

/// The projection of an axes of a program's projection class: its hooks
/// call the methods of the class' adapter, as which it is archived.
#[derive(Debug)]
pub(super) struct PyProjection {
    /// The adapter.
    adapter: Owned,
    /// The class' `name`.
    name: &'static str,
    /// The order of the spines of `_gen_axes_spines`.
    spine_order: [SpineSide; 4],
    /// The ratio the class' `get_data_ratio` gives, when it overrides it.
    data_ratio: Option<f64>,
    overrides: Overrides,
    set_up: Callback,
    data_to_axes: Callback,
    axis_transform: Callback,
    tick_interval: Callback,
    tick_sides: Callback,
    spine_interval: Callback,
}

/// The view limits as the adapter takes them: `(x0, y0, x1, y1)`, an
/// inverted axis the other way round.
fn limits(view: &ProjectionView<'_>) -> (f64, f64, f64, f64) {
    let limits = view.limits();
    (limits.p0.x, limits.p0.y, limits.p1.x, limits.p1.y)
}

impl PyProjection {
    /// The projection asking `adapter`, which gives its `name`, its
    /// `spine_order` (the four side names), its `data_ratio` (`None` for
    /// the rectilinear one) and its `overrides`.
    pub(super) fn of(adapter: &Bound<'_>) -> PyResult<Self> {
        let method = |name: &str| -> PyResult<Callback> { adapter.getattr(name)?.extract() };
        let order: Vec<String> = adapter.getattr("spine_order")?.extract()?;
        let spine_order = order
            .iter()
            .map(|name| spine_side(name))
            .collect::<PyResult<Vec<_>>>()?
            .try_into()
            .map_err(|sides: Vec<SpineSide>| {
                PyErr::value_error(format!("{} spines where axes have 4", sides.len()))
            })?;
        Ok(Self {
            adapter: adapter.clone().unbind(),
            name: pltrs_core::interned(&adapter.getattr("name")?.extract::<String>()?),
            spine_order,
            data_ratio: adapter.getattr("data_ratio")?.extract()?,
            overrides: adapter.getattr("overrides")?.extract()?,
            set_up: method("set_up")?,
            data_to_axes: method("data_to_axes")?,
            axis_transform: method("axis_transform")?,
            tick_interval: method("tick_interval")?,
            tick_sides: method("tick_sides")?,
            spine_interval: method("spine_interval")?,
        })
    }

    /// The adapter the projection asks.
    pub(super) fn adapter(&self) -> &Owned {
        &self.adapter
    }
}

impl Projection for PyProjection {
    fn name(&self) -> &'static str {
        self.name
    }

    /// `transData` without its final `transAxes`, at the view.
    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        self.data_to_axes
            .call::<Evaluated>((limits(view),))
            .map_or_else(|| view.rectilinear(), Evaluated::resolved)
    }

    /// The adapter connects the axes again (it runs whenever the axes is
    /// cleared, which disconnects what was connected to it).
    fn set_up(&mut self, _axes: &Axes) -> Result<(), Box<dyn std::error::Error>> {
        match self.set_up.call::<Owned>(()) {
            Some(_) => Ok(()),
            None => Err("the projection's set-up raised".into()),
        }
    }

    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        if !self.overrides.axis_transform.contains(&axis) {
            return None;
        }
        self.axis_transform
            .call::<Evaluated>((limits(view), axis.name()))
            .map(Evaluated::resolved)
    }

    fn tick_interval(&self, view: &ProjectionView<'_>, axis: AxisName) -> (f64, f64) {
        if !self.overrides.tick_interval.contains(&axis) {
            return view.interval(axis);
        }
        self.tick_interval
            .call((limits(view), axis.name()))
            .unwrap_or_else(|| view.interval(axis))
    }

    fn tick_sides(&self, view: &ProjectionView<'_>, axis: AxisName, loc: f64) -> Sides {
        if !self.overrides.tick_sides.contains(&axis) {
            return Sides::BOTH;
        }
        self.tick_sides
            .call((limits(view), axis.name(), loc))
            .map_or(Sides::BOTH, |(first, second)| Sides { first, second })
    }

    fn spine_order(&self) -> [SpineSide; 4] {
        self.spine_order
    }

    fn data_ratio(&self) -> Option<f64> {
        self.data_ratio
    }

    fn spine_interval(&self, view: &ProjectionView<'_>, side: SpineSide) -> Option<(f64, f64)> {
        if !self.overrides.spine_interval.contains(&side) {
            return None;
        }
        self.spine_interval.call((limits(view), side.name()))
    }
}

/// Hands the fresh rectilinear `axes` over to the projection asking
/// `adapter` (see [`PyProjection::of`]), which sets it up.
fn install(axes: Axes, adapter: Owned) -> PyResult<()> {
    let projection = Python::with_gil(|py| PyProjection::of(&adapter.bind(py)))?;
    axes.install_projection(projection)?;
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("install", install)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};
    use pltrs::figure::Figure;
    use pltrs::transforms::Point;

    /// An adapter leaning the x axis by a shear of 1 and ticking it over
    /// twice its view, which records the calls it gets.
    const ADAPTER: &str = r"
class Adapter:
    name = 'leaning'
    spine_order = ['top', 'bottom', 'left', 'right']
    data_ratio = None
    overrides = {'axis_transform': [], 'tick_interval': ['x'],
                 'tick_sides': ['x'], 'spine_interval': ['top']}

    def __init__(self):
        self.calls = []

    def set_up(self):
        self.calls.append('set_up')

    def data_to_axes(self, limits):
        x0, y0, x1, y1 = limits
        sx, sy = 1 / (x1 - x0), 1 / (y1 - y0)
        # The limits to the unit square, then the shear.
        return [[sx, sy, -x0 * sx - y0 * sy], [0, sy, -y0 * sy], [0, 0, 1.0]], None

    def axis_transform(self, limits, name):
        raise AssertionError('not overridden')

    def tick_interval(self, limits, name):
        x0, y0, x1, y1 = limits
        return 2 * x0 - x1, x1

    def tick_sides(self, limits, name, loc):
        return loc >= 0, True

    def spine_interval(self, limits, side):
        return limits[0] / 2, limits[2]
";

    #[test]
    fn hooks_ask_the_adapter_at_the_view() {
        with_python(|py| {
            let adapter = eval(py, ADAPTER, "Adapter()");
            let figure = Figure::with_size(4.0, 4.0, 100.0);
            let axes = figure.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
            axes.set_xlim(0.0, 10.0)
                .unwrap()
                .set_ylim(0.0, 10.0)
                .unwrap();
            install(axes.clone(), adapter.clone().unbind()).unwrap();
            assert_eq!(axes.projection().name(), "leaning");
            assert_eq!(
                adapter.getattr("calls").unwrap().repr().unwrap(),
                "['set_up']"
            );
            let data = figure.resolve_transform(&axes.trans_data()).unwrap();
            assert_eq!(
                data.transform_point(Point::new(0.0, 10.0)),
                Point::new(400.0, 400.0)
            );
            let ticks = axes.ticks(AxisName::X).unwrap();
            assert!(ticks.major_locs.contains(&-10.0), "{:?}", ticks.major_locs);
            assert!(
                install(axes, adapter.unbind()).is_err(),
                "only rectilinear axes"
            );
        });
    }

    #[test]
    fn adapters_name_four_sides() {
        with_python(|py| {
            let three = eval(
                py,
                ADAPTER,
                "type('Three', (Adapter,), {'spine_order': ['top'] * 3})()",
            );
            assert!(PyProjection::of(&three).is_err());
            let wrong = eval(
                py,
                ADAPTER,
                "type('Wrong', (Adapter,), {'overrides': {**Adapter.overrides, 'tick_sides': ['z']}})()",
            );
            assert!(PyProjection::of(&wrong).is_err());
        });
    }
}
