//! Curvilinear grids (matplotlib's `axisartist.grid_helper_curvelinear`
//! and `floating_axes`): pltrs (Rust) lays the grid lines, ticks and
//! floating axes out along an aux transform, asking the Python grid
//! finder's extreme finder, locators and formatters (`grid_finder`) for
//! the range, levels and labels.

use std::fmt;

use pltrs::axes::Axes;
use pltrs::axisartist::floating_axes::{FloatingAxes, FloatingGridHelper};
use pltrs::axisartist::grid_finder::{
    ExtremeFinder, GridError, GridFinderOptions, GridFormatter, GridLevels, GridLocator,
};
use pltrs::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use pltrs::axisartist::{AuxTransform, AxisArtist, AxisArtistAxes};
use pltrs::spines::SpineSide;
use pltrs::transforms::{Bbox, Point};

use super::super::transforms::{affine, map_xy};
use super::artists::axis_named;
use crate::binding::prelude::*;

/// An aux transform as the Python side describes it: `("affine",
/// matrix)`, `("polar",)`, `("then", first, second)`, or `("functions",
/// forward, inverse)`, two callables mapping the arrays `(xs, ys)` to a
/// `(2, N)` array, called once for all the points pltrs (Rust) needs.
struct PyAux(AuxTransform);

impl FromPy<'_> for PyAux {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let tag: String = object
            .call_method("__getitem__", &[0_i64.into_py(object.py())?])?
            .extract()?;
        Ok(Self(match tag.as_str() {
            "affine" => AuxTransform::affine(affine(&object.extract::<(String, NdArray)>()?.1)?),
            "polar" => AuxTransform::polar(),
            "then" => {
                let (_, first, second): (String, PyAux, PyAux) = object.extract()?;
                first.0.then(second.0)
            }
            "functions" => {
                let (_, forward, inverse): (String, Callback, Callback) = object.extract()?;
                let map =
                    |function: Callback| move |points: &mut [Point]| map_xy(&function, points);
                AuxTransform::from_array_fns(map(forward), map(inverse))
            }
            _ => return Err(PyErr::type_error(format!("unknown aux transform {tag:?}"))),
        }))
    }
}

/// The range of curvilinear coordinates covering a box, as the Python
/// grid finder's extreme finder finds it: called with the box's
/// `(x0, y0, x1, y1)`, it returns those of the range.
struct PyExtremeFinder(Callback);

impl fmt::Debug for PyExtremeFinder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PyExtremeFinder")
    }
}

impl ExtremeFinder for PyExtremeFinder {
    fn find_transformed_bbox(&self, _transform: &AuxTransform, bbox: Bbox) -> Bbox {
        let (x0, y0, x1, y1) = self
            .0
            .call((bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()))
            .unwrap_or((f64::NAN, f64::NAN, f64::NAN, f64::NAN));
        Bbox::from_extents(x0, y0, x1, y1)
    }
}

/// The levels of one coordinate, as a Python grid locator gives them:
/// `locator(v1, v2) -> (levels, n, factor)`.
struct PyGridLocator(Callback);

impl fmt::Debug for PyGridLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PyGridLocator")
    }
}

impl GridLocator for PyGridLocator {
    fn levels(&self, v1: f64, v2: f64) -> Result<GridLevels, GridError> {
        let (levels, n, factor): (Vec<f64>, usize, f64) = self
            .0
            .call((v1, v2))
            .ok_or_else(|| GridError::InvalidLevels("the grid locator failed".into()))?;
        Ok(GridLevels { levels, n, factor })
    }
}

/// The labels of one coordinate, as the Python grid finder formats them:
/// `formatter(direction, factor, values) -> labels`.
struct PyGridFormatter(Callback);

impl fmt::Debug for PyGridFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PyGridFormatter")
    }
}

impl GridFormatter for PyGridFormatter {
    fn labels(
        &self,
        direction: SpineSide,
        factor: f64,
        values: &[f64],
    ) -> Result<Vec<String>, GridError> {
        self.0
            .call((direction.name(), factor, values.to_vec()))
            .ok_or_else(|| GridError::InvalidLevels("the grid formatter failed".into()))
    }
}

/// The parts of the Python grid finder: its extreme finder, its two
/// locators and its two formatters, as callables.
type Finder = (Callback, (Callback, Callback), (Callback, Callback));

/// The grid finder options calling the Python grid finder's parts.
fn finder_options(
    (extreme, (locator1, locator2), (formatter1, formatter2)): Finder,
) -> GridFinderOptions {
    GridFinderOptions {
        extreme_finder: Some(Box::new(PyExtremeFinder(extreme))),
        grid_locator1: Some(Box::new(PyGridLocator(locator1))),
        grid_locator2: Some(Box::new(PyGridLocator(locator2))),
        tick_formatter1: Some(Box::new(PyGridFormatter(formatter1))),
        tick_formatter2: Some(Box::new(PyGridFormatter(formatter2))),
    }
}

/// Makes `axes` an axes drawn by axis artists along the curvilinear grid
/// of `aux` (`axislines.Axes(grid_helper=GridHelperCurveLinear(...))`).
fn install(axes: Axes, aux: PyAux, finder: Finder) -> PyResult<()> {
    AxisArtistAxes::with_grid_helper(
        axes,
        GridHelperCurveLinear::new(aux.0, finder_options(finder)),
    );
    Ok(())
}

/// Makes `axes` the floating axes showing the curvilinear range
/// `extremes` of `aux` (`FloatingAxes`).
fn install_floating(
    axes: Axes,
    aux: PyAux,
    extremes: (f64, f64, f64, f64),
    finder: Finder,
) -> PyResult<()> {
    let helper = FloatingGridHelper::new(aux.0, extremes, finder_options(finder));
    FloatingAxes::new(axes, helper).map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok(())
}

/// Gives the parasite `aux` the background patch of the floating axes
/// `host` (`aux_ax.patch = ax.patch`): it clips to the patch's outline and
/// draws the patch again among its children.
fn share_floating_patch(host: Axes, aux: Axes) -> PyResult<()> {
    FloatingAxes::of(host).share_patch(&aux);
    Ok(())
}

/// Clears the floating axes `axes` and fits it to its outline again
/// (`FloatingAxes.clear`).
fn clear_floating(axes: Axes) -> PyResult<()> {
    FloatingAxes::of(axes).clear();
    Ok(())
}

/// Fits the limits of the floating axes `axes` around its outline
/// (`FloatingAxes.adjust_axes_lim`).
fn adjust_floating_lim(axes: Axes) -> PyResult<()> {
    FloatingAxes::of(axes).adjust_axes_lim();
    Ok(())
}

/// The coordinate whose ticks an edge axis artist shows (`nth_coord_ticks`).
fn set_nth_coord_ticks(artist: AxisArtist, nth: String) -> PyResult<()> {
    artist.set_nth_coord_ticks(axis_named(&nth)?);
    Ok(())
}

/// The range a floating axis artist spans (`set_extremes`).
fn set_extremes(artist: AxisArtist, (min, max): (f64, f64)) -> PyResult<()> {
    artist.set_extremes(min, max);
    Ok(())
}

/// Has the curvilinear grid of `axes` computed anew, through `aux` from
/// now on when given (`update_grid_finder`).
fn update_grid_finder(axes: Axes, aux: Option<PyAux>) -> PyResult<()> {
    AxisArtistAxes::of(axes).update_grid_finder(aux.map(|aux| aux.0));
    Ok(())
}

/// A parasite of `host` plotting in the curvilinear coordinates of
/// `aux` (`get_aux_axes(tr)` of an axis-artist host).
fn aux_axes(host: Axes, aux: PyAux) -> PyResult<Axes> {
    Ok(AxisArtistAxes::of(host).aux_axes(&aux.0).axes().clone())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("install_curvilinear", install)?;
    module.add("update_grid_finder", update_grid_finder)?;
    module.add("install_floating", install_floating)?;
    module.add("clear_floating", clear_floating)?;
    module.add("share_floating_patch", share_floating_patch)?;
    module.add("adjust_floating_lim", adjust_floating_lim)?;
    module.add("set_nth_coord_ticks", set_nth_coord_ticks)?;
    module.add("set_extremes", set_extremes)?;
    module.add("aux_axes", aux_axes)
}
