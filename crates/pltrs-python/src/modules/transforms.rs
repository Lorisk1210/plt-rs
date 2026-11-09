//! `_native.transforms`: the arithmetic of matplotlib's `transforms`
//! (affine matrices, bounding boxes, intervals), done by pltrs-core's
//! `Affine2D` and `Bbox` and pltrs's transform helpers.
//!
//! Python keeps matplotlib's transform tree (its nodes, their invalidation
//! and their matrices as 3×3 numpy arrays, `_mtx`); every number those
//! nodes compute comes from here. A matrix crosses as a 3×3 array, a box
//! as its 2×2 array of points `[[x0, y0], [x1, y1]]`, points as an `(N, 2)`
//! array (or one `(2,)` point).

use pltrs::numeric::numpy_arith;
use pltrs::transforms::{
    Affine2D, Anchor, Bbox, Point, TrackedBbox, bbox_transform, bbox_transform_from,
    interval_contains, interval_contains_close, interval_contains_open, nonsingular, unit_to_bbox,
};

use crate::binding::prelude::*;
use crate::binding::{FloatArray, registered_error};

mod arrays;
mod computed;
mod geo;
mod leaves;
mod live;
pub(crate) use arrays::{map_points, map_values, map_xy};
pub(crate) use leaves::sync_watched;
pub(crate) use live::PyPointMap;

/// The affine map of a 3×3 matrix (its last row is taken as `0 0 1`).
pub(super) fn affine(matrix: &NdArray) -> PyResult<Affine2D> {
    match (&matrix.shape[..], &matrix.data[..]) {
        ([3, 3], [a, c, e, b, d, f, ..]) => Ok(Affine2D::from_values(*a, *b, *c, *d, *e, *f)),
        _ => Err(PyErr::value_error(format!(
            "an affine matrix is 3x3, not of shape {:?}",
            matrix.shape
        ))),
    }
}

/// The 3×3 matrix of an affine map.
pub(super) fn matrix(affine: Affine2D) -> PyResult<NdArray> {
    NdArray::with_shape(affine.matrix().concat(), vec![3, 3])
}

/// The points of an `(N, 2)` array, or of one `(2,)` point.
pub(super) fn points(array: &NdArray) -> PyResult<Vec<Point>> {
    match array.shape[..] {
        [2] | [_, 2] => Ok(array
            .data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[x, y]| Point::new(x, y))
            .collect()),
        [0] => Ok(Vec::new()),
        _ => Err(PyErr::value_error("Invalid vertices array.")),
    }
}

/// Points as an array of `shape` (`(2,)` or `(N, 2)`).
pub(super) fn points_array(points: &[Point], shape: Vec<usize>) -> PyResult<NdArray> {
    let data = points.iter().flat_map(|p| [p.x, p.y]).collect();
    NdArray::with_shape(data, shape)
}

/// The box of a 2×2 array of points.
pub(super) fn bbox(array: &NdArray) -> PyResult<Bbox> {
    match (&array.shape[..], &array.data[..]) {
        ([2, 2], [x0, y0, x1, y1]) => Ok(Bbox::from_extents(*x0, *y0, *x1, *y1)),
        _ => Err(PyErr::value_error(format!(
            "a bounding box is a 2x2 array of points, not of shape {:?}",
            array.shape
        ))),
    }
}

/// The 2×2 array of points of a box.
pub(super) fn bbox_array(bbox: Bbox) -> PyResult<NdArray> {
    NdArray::with_shape(vec![bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()], vec![2, 2])
}

/// The boxes of an `(N, 2, 2)` array.
fn bboxes(array: &NdArray) -> PyResult<Vec<Bbox>> {
    match array.shape[..] {
        [_, 2, 2] | [0] => Ok(array
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[x0, y0, x1, y1]| Bbox::from_extents(x0, y0, x1, y1))
            .collect()),
        _ => Err(PyErr::value_error(format!(
            "bounding boxes are an (N, 2, 2) array, not of shape {:?}",
            array.shape
        ))),
    }
}

/// `affine_transform(points, matrix)`: the points (anything
/// `numpy.asarray(points, float)` takes) mapped by the matrix, with the
/// checks and messages of matplotlib's C++ function, and pybind11's
/// `TypeError` for points numpy cannot make doubles of.
fn affine_transform(points: Owned, mtx: Owned) -> PyResult<NdArray> {
    let (values, mtx) = Python::with_gil(|py| {
        let (points, mtx) = (points.bind(py), mtx.bind(py));
        match FloatArray::from_py(&points) {
            Ok(FloatArray(values)) => Ok((values, NdArray::from_py(&mtx)?)),
            Err(_) => Err(PyErr::type_error(format!(
                "affine_transform(): incompatible function arguments. The following \
                 argument types are supported:\n    1. (points: typing.Annotated[\
                 numpy.typing.ArrayLike, numpy.float64], trans: trans_affine) -> object\n\n\
                 Invoked with: {}, {}",
                points.repr()?,
                mtx.repr()?
            ))),
        }
    })?;
    map_vertices(values, affine(&mtx)?)
}

/// `values`, an array of vertices, mapped by `affine` where they are, as
/// matplotlib's C++ `affine_transform` checks and maps them: one point, or
/// an `(N, 2)` array (any empty one mapping to an `(N, 2)` array).
fn map_vertices(mut values: NdArray, affine: Affine2D) -> PyResult<NdArray> {
    match values.shape[..] {
        [2] | [_, 2] => {}
        [_] => return Err(PyErr::runtime_error("Invalid vertices array.")),
        [rows, _] if values.data.is_empty() => {
            return NdArray::with_shape(Vec::new(), vec![rows, 2]);
        }
        [rows, columns] => {
            return Err(PyErr::value_error(format!(
                "vertices must have shape (N, 2), got ({rows}, {columns})"
            )));
        }
        ref shape => {
            return Err(PyErr::value_error(format!(
                "vertices must be 1D or 2D, not{}D",
                shape.len()
            )));
        }
    }
    for xy in values.data.as_chunks_mut::<2>().0 {
        let mapped = affine.transform_point(Point::new(xy[0], xy[1]));
        *xy = [mapped.x, mapped.y];
    }
    Ok(values)
}

/// The matrix applying `a`, then `b`.
fn compose(a: NdArray, b: NdArray) -> PyResult<NdArray> {
    let (first, then) = (affine(&a)?, affine(&b)?);
    if a.data.iter().chain(&b.data).all(|v| v.is_finite()) {
        return matrix(first.then(then));
    }
    // matplotlib's `np.dot(b, a)` of the whole matrices, which carries a
    // NaN into the bottom row and warns about infinities times zero.
    let rows = |m: &NdArray| -> [[f64; 3]; 3] {
        let mut rows = [[0.0; 3]; 3];
        for (row, values) in rows.iter_mut().zip(m.data.as_chunks::<3>().0) {
            *row = *values;
        }
        rows
    };
    let product = numpy_arith::dot3(&rows(&b), &rows(&a));
    NdArray::with_shape(product.concat(), vec![3, 3])
}

/// The inverse matrix; numpy's `LinAlgError` when it is singular.
fn invert(mtx: NdArray) -> PyResult<NdArray> {
    let inverse = affine(&mtx)?
        .inverted()
        .ok_or_else(|| registered_error("LinAlgError", "Singular matrix"))?;
    matrix(inverse)
}

/// The matrix taking the box `boxin` onto the box `boxout`
/// (`BboxTransform`), with the warnings numpy issues as matplotlib's
/// `get_matrix` computes it.
fn transform_between(boxin: NdArray, boxout: NdArray) -> PyResult<NdArray> {
    let (boxin, boxout) = (bbox(&boxin)?, bbox(&boxout)?);
    let (in_size, out_size) = (numpy_size(&boxin), numpy_size(&boxout));
    let scales = (
        numpy_arith::divide(out_size.0, in_size.0),
        numpy_arith::divide(out_size.1, in_size.1),
    );
    numpy_arith::add(numpy_arith::multiply(-boxin.x0(), scales.0), boxout.x0());
    numpy_arith::add(numpy_arith::multiply(-boxin.y0(), scales.1), boxout.y0());
    matrix(bbox_transform(&boxin, &boxout))
}

/// The matrix taking the unit square onto the box (`BboxTransformTo`),
/// with the warnings numpy issues as matplotlib's `get_matrix` computes
/// it.
fn transform_to(boxout: NdArray) -> PyResult<NdArray> {
    let boxout = bbox(&boxout)?;
    numpy_size(&boxout);
    matrix(unit_to_bbox(&boxout))
}

/// The matrix taking the box onto the unit square (`BboxTransformFrom`),
/// with the warnings numpy issues as matplotlib's `get_matrix` computes
/// it.
fn transform_from(boxin: NdArray) -> PyResult<NdArray> {
    let boxin = bbox(&boxin)?;
    let (width, height) = numpy_size(&boxin);
    let scales = (
        numpy_arith::divide(1.0, width),
        numpy_arith::divide(1.0, height),
    );
    numpy_arith::multiply(-boxin.x0(), scales.0);
    numpy_arith::multiply(-boxin.y0(), scales.1);
    matrix(bbox_transform_from(&boxin))
}

/// The width and height of `bbox` as matplotlib's box transforms take its
/// `bounds`, with numpy's warnings (infinite corners of one sign).
fn numpy_size(bbox: &Bbox) -> (f64, f64) {
    (
        numpy_arith::subtract(bbox.x1(), bbox.x0()),
        numpy_arith::subtract(bbox.y1(), bbox.y0()),
    )
}

/// `update_from_data_xy`: the points and `minpos` of the box `points`
/// grown to the finite points of `xy`, starting afresh when `ignore`.
fn update_from_points(
    points: NdArray,
    minpos: (f64, f64),
    ignore: bool,
    xy: NdArray,
    (update_x, update_y): (bool, bool),
) -> PyResult<(NdArray, (f64, f64))> {
    let mut tracked = TrackedBbox::new(bbox(&points)?);
    tracked.set_minpos(Point::new(minpos.0, minpos.1));
    tracked.set_ignore(ignore);
    tracked.update_from_points(&self::points(&xy)?, None, update_x, update_y);
    let minpos = tracked.minpos();
    Ok((bbox_array(tracked.bbox())?, (minpos.x, minpos.y)))
}

/// How many of `vertices` lie strictly inside the box.
fn count_contains(points: NdArray, vertices: NdArray) -> PyResult<usize> {
    Ok(bbox(&points)?.count_contains(&self::points(&vertices)?))
}

/// How many of `others` (an `(N, 2, 2)` array) overlap the box.
fn count_overlaps(points: NdArray, others: NdArray) -> PyResult<usize> {
    Ok(bbox(&points)?.count_overlaps(&bboxes(&others)?))
}

/// The box containing every box of `boxes`; `ValueError` for none.
fn union(boxes: NdArray) -> PyResult<NdArray> {
    let boxes = bboxes(&boxes)?;
    let union =
        Bbox::union_all(&boxes).ok_or_else(|| PyErr::value_error("'bboxes' cannot be empty"))?;
    bbox_array(union)
}

/// The intersection of two boxes, `None` when they are disjoint.
fn intersection(a: NdArray, b: NdArray) -> PyResult<Option<NdArray>> {
    bbox(&a)?
        .intersection(&bbox(&b)?)
        .map(bbox_array)
        .transpose()
}

/// Whether two boxes overlap, edges included (or, `fully`, excluded).
fn overlaps(a: NdArray, b: NdArray, fully: bool) -> PyResult<bool> {
    let (a, b) = (bbox(&a)?, bbox(&b)?);
    Ok(if fully {
        a.fully_overlaps(&b)
    } else {
        a.overlaps(&b)
    })
}

/// Whether the box contains `(x, y)`, edges included (or, `fully`,
/// excluded); a `None` coordinate is not checked.
fn contains(points: NdArray, x: Option<f64>, y: Option<f64>, fully: bool) -> PyResult<bool> {
    let bbox = bbox(&points)?;
    let inside_x = x.is_none_or(|x| {
        if fully {
            bbox.fully_contains_x(x)
        } else {
            bbox.contains_x(x)
        }
    });
    let inside_y = y.is_none_or(|y| {
        if fully {
            bbox.fully_contains_y(y)
        } else {
            bbox.contains_y(y)
        }
    });
    Ok(inside_x && inside_y)
}

/// An anchor: a code (`"C"`, `"SW"`...) or fractions `(cx, cy)`.
pub(super) fn anchor(object: &Bound<'_>) -> PyResult<Anchor> {
    if let Ok(code) = object.extract::<String>() {
        return Anchor::from_code(&code)
            .ok_or_else(|| PyErr::value_error(format!("Unknown anchor code {code:?}")));
    }
    let (cx, cy): (f64, f64) = object.extract()?;
    Ok(Anchor::Fraction(cx, cy))
}

/// The box moved to `c` within `container` (`anchored`).
fn anchored(points: NdArray, c: Owned, container: NdArray) -> PyResult<NdArray> {
    let anchor = Python::with_gil(|py| anchor(&c.bind(py)))?;
    bbox_array(bbox(&points)?.anchored(anchor, &bbox(&container)?))
}

/// The largest box of aspect `box_aspect` fitting in `container`
/// (`shrunk_to_aspect`).
fn shrunk_to_aspect(
    points: NdArray,
    box_aspect: f64,
    container: NdArray,
    fig_aspect: f64,
) -> PyResult<NdArray> {
    let shrunk = bbox(&points)?
        .shrunk_to_aspect(box_aspect, Some(&bbox(&container)?), fig_aspect)
        .ok_or_else(|| PyErr::value_error("'box_aspect' and 'fig_aspect' must be positive"))?;
    bbox_array(shrunk)
}

/// `nonsingular(vmin, vmax, expander, tiny, increasing)`.
fn nonsingular_interval(
    vmin: f64,
    vmax: f64,
    expander: f64,
    tiny: f64,
    increasing: bool,
) -> PyResult<(f64, f64)> {
    Ok(nonsingular(vmin, vmax, expander, tiny, increasing))
}

/// Whether `val` lies in the closed interval `(a, b)`; `open` excludes the
/// ends, `rtol` (when given) widens them by that fraction of the interval.
fn contains_value(interval: (f64, f64), val: f64, open: bool, rtol: Option<f64>) -> PyResult<bool> {
    Ok(match (open, rtol) {
        (true, _) => interval_contains_open(interval, val),
        (false, Some(rtol)) => interval_contains_close(interval, val, rtol),
        (false, None) => interval_contains(interval, val),
    })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("affine_transform", affine_transform)?;
    module.add("compose", compose)?;
    module.add("invert", invert)?;
    module.add("transform_between", transform_between)?;
    module.add("transform_to", transform_to)?;
    module.add("transform_from", transform_from)?;
    module.add("update_from_points", update_from_points)?;
    module.add("count_contains", count_contains)?;
    module.add("count_overlaps", count_overlaps)?;
    module.add("union", union)?;
    module.add("intersection", intersection)?;
    module.add("overlaps", overlaps)?;
    module.add("contains", contains)?;
    module.add("anchored", anchored)?;
    module.add("shrunk_to_aspect", shrunk_to_aspect)?;
    module.add("nonsingular", nonsingular_interval)?;
    module.add("interval_contains", contains_value)?;
    computed::register(module)?;
    geo::register(module)?;
    leaves::register(module)?;
    live::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn array(data: &[f64], shape: &[usize]) -> NdArray {
        NdArray::with_shape(data.to_vec(), shape.to_vec()).unwrap()
    }

    #[test]
    fn matrices_round_trip_and_compose_left_to_right() {
        let scale = matrix(Affine2D::scaling(2.0, 3.0)).unwrap();
        let shift = matrix(Affine2D::translation(1.0, 0.0)).unwrap();
        let both = compose(scale, shift).unwrap();
        assert_eq!(both.data, [2.0, 0.0, 1.0, 0.0, 3.0, 0.0, 0.0, 0.0, 1.0]);
        let mapped = map_vertices(array(&[1.0, 1.0], &[2]), affine(&both).unwrap()).unwrap();
        assert_eq!((mapped.data, mapped.shape), (vec![3.0, 3.0], vec![2]));
    }

    #[test]
    fn malformed_points_and_matrices_are_refused() {
        let identity = Affine2D::IDENTITY;
        assert!(map_vertices(array(&[], &[0]), identity).is_err());
        assert!(map_vertices(array(&[1.0, 2.0, 3.0], &[3]), identity).is_err());
        assert!(map_vertices(array(&[1.0; 6], &[2, 3]), identity).is_err());
        assert!(map_vertices(array(&[1.0; 8], &[2, 2, 2]), identity).is_err());
        assert!(map_vertices(array(&[1.0], &[]), identity).is_err());
        assert!(affine(&array(&[1.0; 4], &[2, 2])).is_err());
        assert!(bbox(&array(&[0.0; 3], &[3])).is_err());
    }

    #[test]
    fn empty_vertex_arrays_map_to_pairs() {
        let mapped = map_vertices(array(&[], &[0, 3]), Affine2D::IDENTITY).unwrap();
        assert_eq!((mapped.data.len(), mapped.shape), (0, vec![0, 2]));
    }

    #[test]
    fn boxes_union_intersect_and_count() {
        let unit = array(&[0.0, 0.0, 1.0, 1.0], &[2, 2]);
        let far = array(&[2.0, 2.0, 3.0, 3.0], &[2, 2]);
        assert!(intersection(unit.clone(), far.clone()).unwrap().is_none());
        assert!(!overlaps(unit.clone(), far, false).unwrap());
        let both = array(&[0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0], &[2, 2, 2]);
        assert_eq!(union(both.clone()).unwrap().data, [0.0, 0.0, 3.0, 3.0]);
        assert_eq!(count_overlaps(unit.clone(), both).unwrap(), 1);
        let inside = array(&[0.5, 0.5, 1.0, 1.0, f64::NAN, 0.5], &[3, 2]);
        assert_eq!(count_contains(unit.clone(), inside).unwrap(), 1);
        assert!(contains(unit.clone(), Some(1.0), None, false).unwrap());
        assert!(!contains(unit, Some(1.0), None, true).unwrap());
    }

    #[test]
    fn intervals_and_nonsingular_ranges() {
        assert_eq!(
            nonsingular_interval(0.0, 0.0, 0.1, 1e-15, true).unwrap(),
            (-0.1, 0.1)
        );
        assert!(contains_value((0.0, 1.0), 1.0, false, None).unwrap());
        assert!(!contains_value((0.0, 1.0), 1.0, true, None).unwrap());
        assert!(contains_value((0.0, 1.0), 1.0 + 1e-12, false, Some(1e-10)).unwrap());
    }
}
