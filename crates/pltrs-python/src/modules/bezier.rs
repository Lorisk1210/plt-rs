//! `_native.bezier`: Bézier curves (matplotlib's `bezier`), computed by
//! pltrs-core's `bezier` module.
//!
//! Control points cross as `(N, d)` arrays; a closed region is a Python
//! function of a point `(x, y)` telling whether it is inside, called
//! through a [`Callback`].

use pltrs_core::Point;
use pltrs_core::bezier::{
    BezierError, BezierSegment, Direction, Parallel, check_if_parallel,
    find_bezier_t_intersecting_with_closedpath, find_control_points, get_cos_sin, get_intersection,
    get_normal_points, get_parallels, make_wedged_bezier2, real_roots_in_01,
    split_bezier_intersecting_with_closedpath, split_path_inout,
};

use super::path::{PathArrays, PyPath, path_arrays};
use super::transforms::{points, points_array};
use crate::binding::prelude::*;
use crate::binding::registered_error;

native_value!(BezierSegment);

impl From<BezierError> for PyErr {
    /// matplotlib's `NonIntersectingPathException` (registered by
    /// `pltrs.bezier`) for curves that do not cross a region's boundary,
    /// `ValueError` otherwise.
    fn from(err: BezierError) -> Self {
        match err {
            BezierError::NotIntersecting => registered_error(
                "NonIntersectingPathException",
                "Both points are on the same side of the closed path",
            ),
            BezierError::ParallelLines => PyErr::value_error(
                "Given lines do not intersect. Please verify that the angles are not equal or \
                 differ by 180 degrees.",
            ),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// Three planar control points, `((x, y), (x, y), (x, y))`.
type Quadratic = ((f64, f64), (f64, f64), (f64, f64));

/// The points of three `(x, y)` pairs.
fn quadratic((a, b, c): Quadratic) -> [Point; 3] {
    [a, b, c].map(|(x, y)| Point::new(x, y))
}

/// Three points as `(x, y)` pairs.
fn pairs([a, b, c]: [Point; 3]) -> Quadratic {
    ((a.x, a.y), (b.x, b.y), (c.x, c.y))
}

/// Whether the region `inside` contains `p` (a raising function counts
/// as no).
fn region(inside: &Callback) -> impl Fn(Point) -> bool + '_ {
    |p: Point| inside.call::<bool>(((p.x, p.y),)).unwrap_or(false)
}

/// The control points of an `(N, d)` array as a curve.
fn segment(control_points: NdArray) -> PyResult<BezierSegment> {
    let dimension = match control_points.shape[..] {
        [_, d] => d,
        [_] => 1,
        _ => return Err(PyErr::value_error("control points are an (N, d) array")),
    };
    Ok(BezierSegment::new(dimension, control_points.data)?)
}

/// The rows of a curve's control points as an `(N, d)` array.
fn rows(curve: &BezierSegment) -> PyResult<NdArray> {
    let rows: Vec<&[f64]> = curve.control_points().collect();
    let dimension = rows.first().map_or(0, |row| row.len());
    NdArray::with_shape(rows.concat(), vec![rows.len(), dimension])
}

/// The curve at each parameter of `t`, as a `(len(t), d)` array.
fn segment_eval(curve: Native<BezierSegment>, t: NdArray) -> PyResult<NdArray> {
    let curve = curve.borrow();
    let values: Vec<Vec<f64>> = t.data.iter().map(|&t| curve.eval(t)).collect();
    let dimension = values.first().map_or(0, Vec::len);
    NdArray::with_shape(values.concat(), vec![values.len(), dimension])
}

/// The polynomial coefficients, one row per power of `t`.
fn segment_coefficients(curve: Native<BezierSegment>) -> PyResult<NdArray> {
    let coefficients = curve.borrow().polynomial_coefficients();
    let dimension = coefficients.first().map_or(0, Vec::len);
    NdArray::with_shape(coefficients.concat(), vec![coefficients.len(), dimension])
}

/// The dimensions and parameters of the curve's axis-aligned extrema.
fn segment_extrema(curve: Native<BezierSegment>) -> PyResult<(NdArray<i64>, NdArray)> {
    let (dims, roots) = curve.borrow().axis_aligned_extrema();
    let dims = dims
        .into_iter()
        .map(|d| i64::try_from(d).unwrap_or(i64::MAX))
        .collect();
    Ok((NdArray::from_vec(dims), NdArray::from_vec(roots)))
}

/// The curve's control points.
fn segment_points(curve: Native<BezierSegment>) -> PyResult<NdArray> {
    rows(&curve.borrow())
}

/// `split_de_casteljau(beta, t)`: the control points of both halves.
fn split_de_casteljau(beta: NdArray, t: f64) -> PyResult<(NdArray, NdArray)> {
    let (left, right) = segment(beta)?.split(t);
    Ok((rows(&left)?, rows(&right)?))
}

/// `_real_roots_in_01(coeffs)`: the real roots in `[0, 1]` of the
/// polynomial of increasing-degree coefficients.
fn roots_in_01(coefficients: Vec<f64>) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(real_roots_in_01(&coefficients)))
}

/// `find_bezier_t_intersecting_with_closedpath(point_at_t, inside, t0, t1,
/// tolerance)`: the parameters bracketing the crossing.
fn find_t_intersecting(
    point_at_t: Callback,
    inside: Callback,
    (t0, t1): (f64, f64),
    tolerance: f64,
) -> PyResult<(f64, f64)> {
    let point_at = |t: f64| {
        point_at_t
            .call::<(f64, f64)>((t,))
            .map_or(Point::new(f64::NAN, f64::NAN), |(x, y)| Point::new(x, y))
    };
    Ok(find_bezier_t_intersecting_with_closedpath(
        point_at,
        region(&inside),
        t0,
        t1,
        tolerance,
    )?)
}

/// `split_bezier_intersecting_with_closedpath(bezier, inside, tolerance)`.
fn split_intersecting(
    control_points: NdArray,
    inside: Callback,
    tolerance: f64,
) -> PyResult<(NdArray, NdArray)> {
    let (left, right) = split_bezier_intersecting_with_closedpath(
        &points(&control_points)?,
        region(&inside),
        tolerance,
    )?;
    let left_shape = vec![left.len(), 2];
    let right_shape = vec![right.len(), 2];
    Ok((
        points_array(&left, left_shape)?,
        points_array(&right, right_shape)?,
    ))
}

/// `split_path_inout(path, inside, tolerance, reorder_inout)`.
fn split_inout(
    py_path: PyPath,
    inside: Callback,
    tolerance: f64,
    reorder_inout: bool,
) -> PyResult<(PathArrays, PathArrays)> {
    let (first, second) =
        split_path_inout(&py_path.path, region(&inside), tolerance, reorder_inout)?;
    Ok((path_arrays(&first, false)?, path_arrays(&second, false)?))
}

/// `get_cos_sin(x0, y0, x1, y1)`.
fn cos_sin(p0: (f64, f64), p1: (f64, f64)) -> PyResult<(f64, f64)> {
    let Direction { cos, sin } = get_cos_sin(Point::new(p0.0, p0.1), Point::new(p1.0, p1.1));
    Ok((cos, sin))
}

/// `check_if_parallel(dx1, dy1, dx2, dy2, tolerance)`: 1, -1 or `False`.
fn parallel(d1: (f64, f64), d2: (f64, f64), tolerance: f64) -> PyResult<Option<i64>> {
    Ok(
        check_if_parallel(Point::new(d1.0, d1.1), Point::new(d2.0, d2.1), tolerance).map(
            |parallel| match parallel {
                Parallel::Same => 1,
                Parallel::Opposite => -1,
            },
        ),
    )
}

/// `get_intersection(cx1, cy1, cos_t1, sin_t1, cx2, cy2, cos_t2, sin_t2)`.
fn intersection(
    c1: (f64, f64),
    (cos1, sin1): (f64, f64),
    c2: (f64, f64),
    (cos2, sin2): (f64, f64),
) -> PyResult<(f64, f64)> {
    let p = get_intersection(
        Point::new(c1.0, c1.1),
        Direction {
            cos: cos1,
            sin: sin1,
        },
        Point::new(c2.0, c2.1),
        Direction {
            cos: cos2,
            sin: sin2,
        },
    )?;
    Ok((p.x, p.y))
}

/// `get_normal_points(cx, cy, cos_t, sin_t, length)`.
fn normal_points(
    c: (f64, f64),
    (cos, sin): (f64, f64),
    length: f64,
) -> PyResult<((f64, f64), (f64, f64))> {
    let (a, b) = get_normal_points(Point::new(c.0, c.1), Direction { cos, sin }, length);
    Ok(((a.x, a.y), (b.x, b.y)))
}

/// `find_control_points(c1x, c1y, mmx, mmy, c2x, c2y)`.
fn control_points(c1: (f64, f64), mid: (f64, f64), c2: (f64, f64)) -> PyResult<Quadratic> {
    let [c1, mid, c2] = quadratic((c1, mid, c2));
    Ok(pairs(find_control_points(c1, mid, c2)))
}

/// `get_parallels(bezier2, width)`.
fn parallels(bezier2: Quadratic, width: f64) -> PyResult<(Quadratic, Quadratic)> {
    let (left, right) = get_parallels(quadratic(bezier2), width);
    Ok((pairs(left), pairs(right)))
}

/// `make_wedged_bezier2(bezier2, width, w1, wm, w2)`.
fn wedged(
    bezier2: Quadratic,
    width: f64,
    (w1, wm, w2): (f64, f64, f64),
) -> PyResult<(Quadratic, Quadratic)> {
    let (left, right) = make_wedged_bezier2(quadratic(bezier2), width, w1, wm, w2);
    Ok((pairs(left), pairs(right)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("segment", segment)?;
    module.add("segment_eval", segment_eval)?;
    module.add("segment_coefficients", segment_coefficients)?;
    module.add("segment_extrema", segment_extrema)?;
    module.add("segment_points", segment_points)?;
    module.add("split_de_casteljau", split_de_casteljau)?;
    module.add("real_roots_in_01", roots_in_01)?;
    module.add("find_t_intersecting", find_t_intersecting)?;
    module.add("split_intersecting", split_intersecting)?;
    module.add("split_inout", split_inout)?;
    module.add("cos_sin", cos_sin)?;
    module.add("parallel", parallel)?;
    module.add("intersection", intersection)?;
    module.add("normal_points", normal_points)?;
    module.add("control_points", control_points)?;
    module.add("parallels", parallels)?;
    module.add("wedged", wedged)
}
