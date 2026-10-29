//! `_native.transforms`: the arithmetic of matplotlib's `transforms`
//! (affine matrices, bounding boxes, intervals), done by plt-core's
//! `Affine2D` and `Bbox` and plt's transform helpers.
//!
//! Python keeps matplotlib's transform tree (its nodes, their invalidation
//! and their matrices as 3×3 numpy arrays, `_mtx`); every number those
//! nodes compute comes from here. A matrix crosses as a 3×3 array, a box
//! as its 2×2 array of points `[[x0, y0], [x1, y1]]`, points as an `(N, 2)`
//! array (or one `(2,)` point).

use plt::transforms::{
    Affine2D, Anchor, Bbox, Point, TrackedBbox, bbox_transform, bbox_transform_from,
    interval_contains, interval_contains_close, interval_contains_open, nonsingular, unit_to_bbox,
};

use crate::binding::prelude::*;
use crate::binding::registered_error;

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

/// `affine_transform(points, matrix)`: the points mapped by the matrix.
fn affine_transform(values: NdArray, mtx: NdArray) -> PyResult<NdArray> {
    let affine = affine(&mtx)?;
    if let [len] = values.shape[..]
        && len != 2
    {
        // matplotlib's C++ check of a single point's length.
        return Err(PyErr::runtime_error("Invalid vertices array."));
    }
    let mut mapped = points(&values)?;
    affine.transform_in_place(&mut mapped);
    points_array(&mapped, values.shape)
}

/// The matrix applying `a`, then `b`.
fn compose(a: NdArray, b: NdArray) -> PyResult<NdArray> {
    matrix(affine(&a)?.then(affine(&b)?))
}

/// The inverse matrix; numpy's `LinAlgError` when it is singular.
fn invert(mtx: NdArray) -> PyResult<NdArray> {
    let inverse = affine(&mtx)?
        .inverted()
        .ok_or_else(|| registered_error("LinAlgError", "Singular matrix"))?;
    matrix(inverse)
}

/// The matrix followed by a rotation of `theta` radians.
fn rotate(mtx: NdArray, theta: f64) -> PyResult<NdArray> {
    matrix(affine(&mtx)?.rotate(theta))
}

/// The matrix followed by a translation.
fn translate(mtx: NdArray, tx: f64, ty: f64) -> PyResult<NdArray> {
    matrix(affine(&mtx)?.translate(tx, ty))
}

/// The matrix followed by a scaling.
fn scale(mtx: NdArray, sx: f64, sy: f64) -> PyResult<NdArray> {
    matrix(affine(&mtx)?.scale(sx, sy))
}

/// The matrix followed by a skew of angles in radians.
fn skew(mtx: NdArray, x_shear: f64, y_shear: f64) -> PyResult<NdArray> {
    matrix(affine(&mtx)?.skew(x_shear, y_shear))
}

/// The matrix taking the box `boxin` onto the box `boxout`
/// (`BboxTransform`).
fn transform_between(boxin: NdArray, boxout: NdArray) -> PyResult<NdArray> {
    matrix(bbox_transform(&bbox(&boxin)?, &bbox(&boxout)?))
}

/// The matrix taking the unit square onto the box (`BboxTransformTo`).
fn transform_to(boxout: NdArray) -> PyResult<NdArray> {
    matrix(unit_to_bbox(&bbox(&boxout)?))
}

/// The matrix taking the box onto the unit square (`BboxTransformFrom`).
fn transform_from(boxin: NdArray) -> PyResult<NdArray> {
    matrix(bbox_transform_from(&bbox(&boxin)?))
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
fn anchor(object: &Bound<'_>) -> PyResult<Anchor> {
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
    module.add("rotate", rotate)?;
    module.add("translate", translate)?;
    module.add("scale", scale)?;
    module.add("skew", skew)?;
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
    module.add("interval_contains", contains_value)
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
        let mapped = affine_transform(array(&[1.0, 1.0], &[2]), both).unwrap();
        assert_eq!((mapped.data, mapped.shape), (vec![3.0, 3.0], vec![2]));
    }

    #[test]
    fn malformed_points_and_matrices_are_refused() {
        let identity = matrix(Affine2D::IDENTITY).unwrap();
        assert!(affine_transform(array(&[], &[0]), identity.clone()).is_err());
        assert!(affine_transform(array(&[1.0, 2.0, 3.0], &[3]), identity).is_err());
        assert!(affine(&array(&[1.0; 4], &[2, 2])).is_err());
        assert!(bbox(&array(&[0.0; 3], &[3])).is_err());
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
