//! Python transforms called with arrays, as matplotlib calls them: pltrs
//! (Rust) gathers the values a path, a set of ticks or a limit needs and
//! hands them over at once (`AxisMap::forward_all`,
//! `PointMap::forward_all`), as a 1-D array of values or an `(N, 2)`
//! array of points (nothing is called for none). A masked result reads as
//! NaN where it is masked; a callable that raises, or answers with another
//! number of values, gives NaN (its exception goes to the native call
//! running).

use pltrs::scale::ArrayLayout;
use pltrs::transforms::Point;

use super::{points, points_array};
use crate::binding::prelude::*;

/// The numbers a Python transform answered with, masked ones as NaN (as
/// matplotlib's paths treat masked vertices), in row-major order.
struct Mapped(Vec<f64>);

impl FromPy<'_> for Mapped {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let filled = if object.getattr("mask").is_ok() {
            object.call_method("filled", &[f64::NAN.into_py(object.py())?])?
        } else {
            object.clone()
        };
        Ok(Self(filled.extract::<NdArray>()?.data))
    }
}

/// Replaces `values` by what `callback` answers for them (as a 1-D array,
/// or a column `(N, 1)` for [`ArrayLayout::Column`]): one call for all of
/// them. Returns whether it answered.
pub(crate) fn map_values(callback: &Callback, values: &mut [f64], layout: ArrayLayout) -> bool {
    if values.is_empty() {
        return true;
    }
    let shape = match layout {
        ArrayLayout::Row => vec![values.len()],
        ArrayLayout::Column => vec![values.len(), 1],
    };
    let array = NdArray::with_shape(values.to_vec(), shape).expect("the shape holds the values");
    let mapped = callback.call::<Mapped>((array,)).map(|mapped| mapped.0);
    let answered = mapped.is_some();
    write_back(values, mapped);
    answered
}

/// Replaces `points` by what `callback` answers for them (as an `(N, 2)`
/// array): one call for all of them.
pub(crate) fn map_points(callback: &Callback, points_in: &mut [Point]) {
    if points_in.is_empty() {
        return;
    }
    let mapped = points_array(points_in, vec![points_in.len(), 2])
        .ok()
        .and_then(|array| callback.call::<NdArray>((array,)))
        .and_then(|array| points(&array).ok());
    match mapped {
        Some(mapped) if mapped.len() == points_in.len() => points_in.copy_from_slice(&mapped),
        _ => points_in.fill(Point::new(f64::NAN, f64::NAN)),
    }
}

/// Replaces `points` by what `callback` answers for their coordinates
/// (`callback(xs, ys)`, two 1-D arrays, as matplotlib calls the functions
/// of a curvilinear grid), a `(2, N)` array: one call for all of them.
pub(crate) fn map_xy(callback: &Callback, points_in: &mut [Point]) {
    if points_in.is_empty() {
        return;
    }
    let xs = NdArray::from_vec(points_in.iter().map(|p| p.x).collect());
    let ys = NdArray::from_vec(points_in.iter().map(|p| p.y).collect());
    let mapped = callback.call::<NdArray>((xs, ys));
    match mapped {
        Some(mapped) if mapped.shape == [2, points_in.len()] => {
            let (us, vs) = mapped.data.split_at(points_in.len());
            for ((point, &u), &v) in points_in.iter_mut().zip(us).zip(vs) {
                *point = Point::new(u, v);
            }
        }
        _ => points_in.fill(Point::new(f64::NAN, f64::NAN)),
    }
}

/// `values` replaced by `mapped`, one number for each (or one for all of
/// them, as numpy broadcasts a scalar), else by NaN.
fn write_back(values: &mut [f64], mapped: Option<Vec<f64>>) {
    match mapped.as_deref() {
        Some(mapped) if mapped.len() == values.len() => values.copy_from_slice(mapped),
        Some(&[one]) => values.fill(one),
        _ => values.fill(f64::NAN),
    }
}

#[cfg(test)]
mod tests {
    // The arrays cross as numpy arrays, which the embedded interpreter of
    // the unit tests lacks: python/tests/test_colorbar_and_callbacks.py
    // calls transforms of the program's with arrays.
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn nothing_is_called_for_no_values() {
        with_python(|py| {
            let raising: Callback = eval(py, "", "lambda *args: 1 / 0").extract().unwrap();
            assert!(map_values(&raising, &mut [], ArrayLayout::Row));
            map_points(&raising, &mut []);
            map_xy(&raising, &mut []);
        });
    }

    #[test]
    fn answers_broadcast_or_fail_as_nan() {
        let mut values = [1.0, 2.0];
        write_back(&mut values, Some(vec![3.0, 4.0]));
        assert_eq!(values, [3.0, 4.0]);
        write_back(&mut values, Some(vec![7.0]));
        assert_eq!(values, [7.0, 7.0], "a scalar broadcasts");
        write_back(&mut values, Some(vec![1.0, 2.0, 3.0]));
        assert!(
            values.iter().all(|v| v.is_nan()),
            "another number of values"
        );
        write_back(&mut values, None);
        assert!(values.iter().all(|v| v.is_nan()), "the callable raised");
    }
}
