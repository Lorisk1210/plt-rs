//! Curvilinear grids (matplotlib's `axisartist.grid_helper_curvelinear`):
//! the [`GridHelperCurveLinear`] and how its axis artists find their
//! lines, ticks and labels — fixed axes tick the grid lines crossing an
//! edge, floating axes run along a line of constant coordinate.

use plt_core::{Bbox, Point};

use super::aux_transform::AuxTransform;
use super::grid_finder::{
    Coordinate, GridError, GridFinder, GridFinderOptions, GridInfo, GridLevels,
};
use super::helper::TickInfo;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::ticker::pymath::{py_max, py_min};
use crate::transforms::{ResolvedTransform, interval_contains_close};

/// The relative tolerance of matplotlib's `_interval_contains_close`.
const CLOSE: f64 = 1e-10;

/// The number of points of a floating axis line.
const LINE_POINTS: usize = 100;

/// A curvilinear grid through an arbitrary transform (matplotlib's
/// `GridHelperCurveLinear`): its grid lines, and the ticks of the axis
/// artists of an [`Axes`](super::Axes) using it, come from the lines of
/// constant curvilinear coordinate.
///
/// ```
/// use plt::axisartist::grid_finder::GridFinderOptions;
/// use plt::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
/// use plt::axisartist::AuxTransform;
/// use plt::transforms::Bbox;
/// let shear = AuxTransform::from_fns(|x, y| (x, y - x), |x, y| (x, y + x));
/// let helper = GridHelperCurveLinear::new(shear, GridFinderOptions::default());
/// let info = helper.grid_finder().grid_info(Bbox::from_extents(0.0, 0.0, 10.0, 10.0))?;
/// assert!(!info.lon.lines.is_empty() && !info.lat.lines.is_empty());
/// # Ok::<(), plt::axisartist::grid_finder::GridError>(())
/// ```
#[derive(Debug)]
pub struct GridHelperCurveLinear {
    grid_finder: GridFinder,
}

impl GridHelperCurveLinear {
    /// A grid whose curvilinear coordinates map to data coordinates by
    /// `aux_trans`, located and labelled as `options` says.
    pub fn new(aux_trans: AuxTransform, options: GridFinderOptions) -> Self {
        Self {
            grid_finder: GridFinder::new(aux_trans, options),
        }
    }

    /// The grid finder computing lines and ticks.
    pub fn grid_finder(&self) -> &GridFinder {
        &self.grid_finder
    }
}

/// The grid lines of `info` drawn for gridlines over `axes` (`None` =
/// both coordinates; matplotlib's `get_gridlines`).
pub(crate) fn info_gridlines(info: &GridInfo, axis: Option<AxisName>) -> Vec<Vec<Point>> {
    let mut lines = Vec::new();
    if axis != Some(AxisName::Y) {
        lines.extend(info.lon.lines.iter().cloned());
    }
    if axis != Some(AxisName::X) {
        lines.extend(info.lat.lines.iter().cloned());
    }
    lines
}

/// The ticks of a fixed axis on `side` of the box: the crossings of both
/// coordinates' grid lines, labelled for coordinate `nth_ticks` only
/// (matplotlib's `FixedAxisArtistHelper.get_tick_iterators`). `inverted`
/// tells that the limits across the side are inverted, which swaps it.
pub(crate) fn fixed_ticks(
    info: &GridInfo,
    side: SpineSide,
    nth_ticks: AxisName,
    inverted: bool,
) -> Vec<TickInfo> {
    let side = if inverted {
        SpineSide::of(side.axis(), !side.is_high())
    } else {
        side
    };
    let angle_tangent = match side.axis() {
        AxisName::X => 0.0,
        AxisName::Y => 90.0,
    };
    let grid = |nth: AxisName| match nth {
        AxisName::X => &info.lon,
        AxisName::Y => &info.lat,
    };
    let other = match nth_ticks {
        AxisName::X => AxisName::Y,
        AxisName::Y => AxisName::X,
    };
    [(nth_ticks, true), (other, false)]
        .into_iter()
        .flat_map(|(nth, labelled)| {
            grid(nth).side_ticks(side).iter().map(move |tick| TickInfo {
                loc: tick.loc,
                angle_normal: tick.angle,
                angle_tangent,
                label: if labelled {
                    tick.label.clone()
                } else {
                    String::new()
                },
            })
        })
        .collect()
}

/// The levels and labels of both coordinates over some extremes, with
/// the line of an axis of constant coordinate (matplotlib's `_grid_info`
/// of floating axis helpers).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisGrid {
    /// The curvilinear extremes.
    pub(crate) extremes: Bbox,
    /// The levels of the first coordinate.
    pub(crate) lon: GridLevels,
    /// The levels of the second coordinate.
    pub(crate) lat: GridLevels,
    /// The labels of every level of the first coordinate.
    pub(crate) lon_labels: Vec<String>,
    /// The labels of every level of the second coordinate.
    pub(crate) lat_labels: Vec<String>,
}

impl AxisGrid {
    /// The levels and labels of `finder` over `extremes` (labelled as
    /// bottom ticks, like matplotlib).
    pub(crate) fn new(finder: &GridFinder, extremes: Bbox) -> Result<Self, GridError> {
        let (lon_min, lat_min, lon_max, lat_max) = extremes.extents();
        let lon = finder.levels(Coordinate::Lon, lon_min, lon_max)?;
        let lat = finder.levels(Coordinate::Lat, lat_min, lat_max)?;
        let lon_labels =
            finder.labels(Coordinate::Lon, SpineSide::Bottom, lon.factor, &lon.levels)?;
        let lat_labels =
            finder.labels(Coordinate::Lat, SpineSide::Bottom, lat.factor, &lat.levels)?;
        Ok(Self {
            extremes,
            lon,
            lat,
            lon_labels,
            lat_labels,
        })
    }
}

/// An axis along which curvilinear coordinate `nth` varies and the other
/// one equals `value`, limited across to `extremes` (matplotlib's
/// `FloatingAxisArtistHelper`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FloatingLine {
    /// The varying coordinate.
    pub(crate) nth: AxisName,
    /// The constant other coordinate.
    pub(crate) value: f64,
    /// The range of the varying coordinate (`-inf..inf` = unlimited).
    pub(crate) extremes: (f64, f64),
}

impl FloatingLine {
    /// The constant coordinate of the line as a point with `along`.
    fn point(self, along: f64) -> Point {
        match self.nth {
            AxisName::X => Point::new(self.value, along),
            AxisName::Y => Point::new(along, self.value),
        }
    }

    /// The levels of the grid over the view `view` (data coordinates)
    /// with the varying coordinate limited to the extremes, and the line
    /// in data coordinates (matplotlib's `update_lim`).
    pub(crate) fn grid(
        self,
        finder: &GridFinder,
        view: Bbox,
    ) -> Result<(AxisGrid, Vec<Point>), GridError> {
        let (mut lon_min, mut lat_min, mut lon_max, mut lat_max) = finder.extremes(view)?.extents();
        let (e_min, e_max) = self.extremes;
        match self.nth {
            AxisName::X => {
                lat_min = py_max(e_min, lat_min);
                lat_max = py_min(e_max, lat_max);
            }
            AxisName::Y => {
                lon_min = py_max(e_min, lon_min);
                lon_max = py_min(e_max, lon_max);
            }
        }
        let extremes = Bbox::from_extents(lon_min, lat_min, lon_max, lat_max);
        let (low, high) = match self.nth {
            AxisName::X => (lat_min, lat_max),
            AxisName::Y => (lon_min, lon_max),
        };
        let line = crate::numeric::linspace(low, high, LINE_POINTS)
            .into_iter()
            .map(|along| finder.transform().transform_point(self.point(along)))
            .collect();
        Ok((AxisGrid::new(finder, extremes)?, line))
    }
}

/// How the ticks of a curvilinear axis of constant coordinate are found.
pub(crate) struct TickSpec {
    /// The varying coordinate's range kept for ticks.
    pub(crate) mask: (f64, f64),
    /// The x limits of the numerical derivatives.
    pub(crate) xlim: (f64, f64),
    /// The y limits of the numerical derivatives.
    pub(crate) ylim: (f64, f64),
}

/// The ticks of an axis of constant coordinate in display pixels:
/// the levels of the varying coordinate inside `spec.mask`, with the
/// angles of the grid there, kept when inside the axes box (matplotlib's
/// `get_tick_iterators` of floating helpers). `to_display` maps
/// curvilinear coordinates to display pixels, `to_axes` display pixels to
/// axes fractions.
pub(crate) fn curve_ticks(
    grid: &AxisGrid,
    line: FloatingLine,
    spec: &TickSpec,
    to_display: &ResolvedTransform,
    to_axes: &ResolvedTransform,
) -> Vec<TickInfo> {
    let (levels, labels) = match line.nth {
        AxisName::X => (grid.lat.all_values(), &grid.lat_labels),
        AxisName::Y => (grid.lon.all_values(), &grid.lon_labels),
    };
    let (low, high) = spec.mask;
    let kept: Vec<(f64, &String)> = levels
        .into_iter()
        .zip(labels)
        .filter(|&(v, _)| low <= v && v <= high)
        .collect();
    let along: Vec<f64> = kept.iter().map(|&(v, _)| v).collect();
    let func = |p: Point| to_display.transform_point(p);
    let (points, (normals, tangents)) = match line.nth {
        AxisName::X => {
            let (points, dx, dy) =
                value_and_jac_angle(&func, &[line.value], &along, spec.xlim, spec.ylim);
            (points, (dx, dy))
        }
        AxisName::Y => {
            let (points, dx, dy) =
                value_and_jac_angle(&func, &along, &[line.value], spec.xlim, spec.ylim);
            (points, (dy, dx))
        }
    };
    let in_axes = |v: f64| interval_contains_close((0.0, 1.0), v, CLOSE);
    points
        .into_iter()
        .zip(normals.into_iter().zip(tangents))
        .zip(kept)
        .filter(|&((p, _), _)| {
            let q = to_axes.transform_point(p);
            in_axes(q.x) && in_axes(q.y)
        })
        .map(|((loc, (normal, tangent)), (_, label))| TickInfo {
            loc,
            angle_normal: normal.to_degrees(),
            angle_tangent: tangent.to_degrees(),
            label: label.clone(),
        })
        .collect()
}

/// Where the label of an axis of constant coordinate goes, in display
/// pixels, and the direction of the axis there in degrees: midway along
/// the extremes of the varying coordinate, if that is inside the axes
/// (matplotlib's `get_axislabel_pos_angle` of floating helpers).
pub(crate) fn curve_label_position(
    extremes: Bbox,
    line: FloatingLine,
    to_display: &ResolvedTransform,
    to_axes: &ResolvedTransform,
) -> Option<(Point, f64)> {
    let (xmin, ymin, xmax, ymax) = extremes.extents();
    let (x, y) = match line.nth {
        AxisName::X => (line.value, f64::midpoint(ymin, ymax)),
        AxisName::Y => (f64::midpoint(xmin, xmax), line.value),
    };
    let func = |p: Point| to_display.transform_point(p);
    let (points, dx, dy) = value_and_jac_angle(&func, &[x], &[y], (xmin, xmax), (ymin, ymax));
    let xy = points[0];
    let p = to_axes.transform_point(xy);
    let angle = match line.nth {
        AxisName::X => dy[0],
        AxisName::Y => dx[0],
    };
    ((0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y)).then(|| (xy, angle.to_degrees()))
}

/// numpy's `minimum`: NaN if either value is NaN.
fn np_minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// numpy's `maximum`: NaN if either value is NaN.
fn np_maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// The finite-difference step towards the farther of the bounds `lim`
/// for each of `values`, at most √ε, and the distance to that bound.
fn steps(values: &[f64], lim: (f64, f64)) -> (Vec<f64>, Vec<f64>) {
    let (lo, hi) = if lim.1 < lim.0 { (lim.1, lim.0) } else { lim };
    let eps0 = f64::EPSILON.sqrt();
    values
        .iter()
        .map(|&v| {
            let (dlo, dhi) = (v - lo, hi - v);
            let eps_max = np_maximum(dlo, dhi);
            let sign = if dhi >= dlo { 1.0 } else { -1.0 };
            (sign * np_minimum(eps0, eps_max), eps_max)
        })
        .unzip()
}

/// The angles (radians) of the numerical derivative of `eval` along one
/// coordinate: `eval(i, dp, dq)` is the function at point `i` moved by
/// `dp` along that coordinate and `dq` along the other. The step doubles
/// until the derivative is not null; a degenerate derivative is retried
/// after a step `eps_q` along the other coordinate (matplotlib's
/// `calc_thetas`, vectorized like numpy: all points share the loop).
fn thetas(
    eval: &dyn Fn(usize, f64, f64) -> Point,
    eps_p0: &[f64],
    eps_max: &[f64],
    eps_q: &[f64],
) -> Vec<f64> {
    let n = eps_p0.len();
    let mut thetas = vec![f64::NAN; n];
    let mut missing = vec![true; n];
    let mut eps_p = eps_p0.to_vec();
    for pass in [None, Some(eps_q)] {
        while missing.iter().any(|&m| m) && eps_p.iter().zip(eps_max).any(|(e, max)| e.abs() < *max)
        {
            if pass.is_none() && eps_p.iter().any(|&e| e > 1.0) {
                break;
            }
            for (e, &max) in eps_p.iter_mut().zip(eps_max) {
                *e = np_minimum(*e, max);
            }
            for i in 0..n {
                let dq = pass.map_or(0.0, |q| q[i]);
                let (moved, base) = (eval(i, eps_p[i], dq), eval(i, 0.0, dq));
                let df_x = (moved.x - base.x) / eps_p[i];
                let df_y = (moved.y - base.y) / eps_p[i];
                if missing[i] && (df_x != 0.0 || df_y != 0.0) {
                    thetas[i] = df_y.atan2(df_x);
                    missing[i] = false;
                }
            }
            for e in &mut eps_p {
                *e *= 2.0;
            }
        }
    }
    thetas
}

/// `func` at the points `(xs, ys)` (one of them may hold a single value,
/// broadcast) and the angles in radians of its numerical derivatives
/// along x and along y, stepping towards the farther of the bounds
/// `xlim` and `ylim` (matplotlib's `_value_and_jac_angle`).
pub(crate) fn value_and_jac_angle(
    func: &dyn Fn(Point) -> Point,
    xs: &[f64],
    ys: &[f64],
    xlim: (f64, f64),
    ylim: (f64, f64),
) -> (Vec<Point>, Vec<f64>, Vec<f64>) {
    let n = xs.len().max(ys.len());
    let broadcast = |values: &[f64]| -> Vec<f64> {
        match values {
            [single] => vec![*single; n],
            _ => values.to_vec(),
        }
    };
    let (xs, ys) = (broadcast(xs), broadcast(ys));
    let values = xs
        .iter()
        .zip(&ys)
        .map(|(&x, &y)| func(Point::new(x, y)))
        .collect();
    let (xeps, xeps_max) = steps(&xs, xlim);
    let (yeps, yeps_max) = steps(&ys, ylim);
    let along_x = |i: usize, dp: f64, dq: f64| func(Point::new(xs[i] + dp, ys[i] + dq));
    let along_y = |i: usize, dp: f64, dq: f64| func(Point::new(xs[i] + dq, ys[i] + dp));
    let thetas_dx = thetas(&along_x, &xeps, &xeps_max, &yeps);
    let thetas_dy = thetas(&along_y, &yeps, &yeps_max, &xeps);
    (values, thetas_dx, thetas_dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2 as VERTICAL;

    #[test]
    fn derivatives_of_a_shear() {
        let shear = |p: Point| Point::new(p.x, p.y - p.x);
        let (values, dx, dy) = value_and_jac_angle(
            &shear,
            &[3.0],
            &[1.0, 2.0],
            (f64::NEG_INFINITY, f64::INFINITY),
            (0.0, 10.0),
        );
        assert_eq!(values, vec![Point::new(3.0, -2.0), Point::new(3.0, -1.0)]);
        assert!(
            dx.iter()
                .all(|a| (a + std::f64::consts::FRAC_PI_4).abs() < 1e-6)
        );
        assert!(dy.iter().all(|a| (a - VERTICAL).abs() < 1e-6));
    }

    #[test]
    fn degenerate_derivatives_step_along_the_other_coordinate() {
        // The polar transform has no angular derivative at r = 0; the
        // values are matplotlib's.
        let polar = |p: Point| Point::new(p.y * p.x.cos(), p.y * p.x.sin());
        let unbounded = (f64::NEG_INFINITY, f64::INFINITY);
        let (_, dx, dy) = value_and_jac_angle(&polar, &[0.5], &[0.0], (0.0, 1.0), (0.0, 10.0));
        assert!(
            dx[0].is_nan(),
            "the step is capped before it may move along r"
        );
        assert!((dy[0] - 0.5).abs() < 1e-12);
        let (_, dx, _) = value_and_jac_angle(&polar, &[0.5], &[0.0], unbounded, (0.0, 10.0));
        assert!((dx[0] - 3.070_796_33).abs() < 1e-8);
        let (_, dx, _) = value_and_jac_angle(&polar, &[0.5, 1.0], &[0.0], unbounded, unbounded);
        assert!((dx[0] - 3.070_796_33).abs() < 1e-8 && (dx[1] + 2.712_388_98).abs() < 1e-8);
    }
}
