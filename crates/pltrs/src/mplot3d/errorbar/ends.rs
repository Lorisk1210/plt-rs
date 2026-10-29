//! The arithmetic of 3-D error bars: where each bar ends, in the units
//! of the values, which points get bars, and the extremes they reach.

use crate::axes::{ErrorSize, PlotError};
use crate::lines::MarkEvery;
use crate::mplot3d::coords::Axis3;
use crate::mplot3d::error::{Axes3DError, check_len};
use crate::mplot3d::math::Vec3;
use crate::units::{AxisDelta, AxisDeltas, AxisValue, AxisValues};

/// The lower and upper bar ends along `axis` of every point, in the
/// units of `values`, the coordinates of the points along it: the value
/// itself on the limited side and for points without bars, else the value
/// less or plus its error (matplotlib's `_extract_errs`, which computes
/// `data - err` and `data + err` on the data as given).
pub(super) fn end_values(
    values: &AxisValues,
    axis: Axis3,
    err: &ErrorSize,
    (lolims, uplims): (&[bool], &[bool]),
    every: &[bool],
) -> Result<(Vec<AxisValue>, Vec<AxisValue>), Axes3DError> {
    let items: Vec<AxisValue> = (0..values.len()).filter_map(|j| values.get(j)).collect();
    let n = items.len();
    let (lower, upper) = match err {
        ErrorSize::Symmetric(errors) => (errors, errors),
        ErrorSize::Asymmetric { lower, upper } => (lower, upper),
    };
    let per_point = |errors: &AxisDeltas| -> Result<Vec<AxisDelta>, Axes3DError> {
        let errors: Vec<AxisDelta> = errors.iter().collect();
        if let [single] = errors.as_slice() {
            return Ok(vec![single.clone(); n]);
        }
        check_len("errors", n, errors.len())?;
        Ok(errors)
    };
    let (lower, upper) = (per_point(lower)?, per_point(upper)?);
    let end = |j: usize, error: &AxisDelta, factor: i64, keep: bool| {
        if keep {
            return Ok(items[j].clone());
        }
        items[j]
            .offset(error, factor)
            .ok_or(Axes3DError::UnaddableErrors(axis))
    };
    let lows = (0..n)
        .map(|j| end(j, &lower[j], -1, lolims[j] || !every[j]))
        .collect::<Result<_, _>>()?;
    let highs = (0..n)
        .map(|j| end(j, &upper[j], 1, uplims[j] || !every[j]))
        .collect::<Result<_, _>>()?;
    Ok((lows, highs))
}

/// One flag per point from one flag or one per point.
pub(super) fn broadcast_flags(
    name: &'static str,
    flags: &[bool],
    n: usize,
) -> Result<Vec<bool>, Axes3DError> {
    match flags {
        [] => Ok(vec![false; n]),
        [single] => Ok(vec![*single; n]),
        _ => {
            check_len(name, n, flags.len())?;
            Ok(flags.to_vec())
        }
    }
}

/// The points getting error bars (matplotlib's `_errorevery_to_mask`).
pub(super) fn every_mask(every: Option<&MarkEvery>, n: usize) -> Result<Vec<bool>, Axes3DError> {
    let Some(every) = every else {
        return Ok(vec![true; n]);
    };
    let mut mask = vec![false; n];
    for i in every.indices(n).map_err(PlotError::ErrorEvery)? {
        mask[i] = true;
    }
    Ok(mask)
}

/// Element-wise `a & b`.
pub(super) fn and(a: &[bool], b: &[bool]) -> Vec<bool> {
    a.iter().zip(b).map(|(&a, &b)| a && b).collect()
}

/// The points where `mask` is set.
pub(super) fn masked(points: &[Vec3], mask: &[bool]) -> Vec<Vec3> {
    points
        .iter()
        .zip(mask)
        .filter_map(|(p, &keep)| keep.then_some(*p))
        .collect()
}

/// The smallest and largest non-NaN value, as matplotlib's `nanmin` and
/// `nanmax` (none when every value is NaN).
pub(super) fn nan_extremes(values: &[f64]) -> Vec<f64> {
    let finite = values.iter().copied().filter(|v| !v.is_nan());
    let low = finite.clone().reduce(f64::min);
    let high = finite.reduce(f64::max);
    low.into_iter().chain(high).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_drop_one_side_and_every_skips_points() {
        let z = AxisValues::from([1.0, 2.0, 3.0]);
        let err = ErrorSize::Asymmetric {
            lower: 0.5.into(),
            upper: 1.0.into(),
        };
        let (lows, highs) = end_values(
            &z,
            Axis3::Z,
            &err,
            (&[true, false, false], &[false, false, false]),
            &[true, true, false],
        )
        .unwrap();
        let number = AxisValue::Number;
        assert_eq!(lows[0], number(1.0), "a lower limit keeps its low end");
        assert_eq!(highs[0], number(2.0));
        assert_eq!((&lows[1], &highs[1]), (&number(1.5), &number(3.0)));
        assert_eq!(
            (&lows[2], &highs[2]),
            (&number(3.0), &number(3.0)),
            "no bar"
        );
    }

    #[test]
    fn errors_with_units_are_refused_as_in_matplotlib() {
        use crate::dates::TimeDelta;
        use crate::units::{ConversionError, ConversionInterface, Convertible, UnitAxis};

        /// A length in some unit.
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Length(f64);

        struct LengthConverter;

        impl ConversionInterface for LengthConverter {
            type Value = Length;
            type Unit = ();
            fn convert(
                &self,
                values: &[Length],
                _: Option<&()>,
                _: &UnitAxis,
            ) -> Result<Vec<f64>, ConversionError> {
                Ok(values.iter().map(|length| length.0).collect())
            }
        }

        impl Convertible for Length {
            type Converter = LengthConverter;
            fn converter() -> LengthConverter {
                LengthConverter
            }
        }

        let values = AxisValues::from([0.0]);
        let flags: (&[bool], &[bool]) = (&[false], &[false]);
        let half_day = TimeDelta::new(0, 43_200, 0).unwrap();
        assert_eq!(
            end_values(&values, Axis3::X, &half_day.into(), flags, &[true]).err(),
            Some(Axes3DError::UnaddableErrors(Axis3::X)),
            "a number plus a duration fails in Python"
        );
        let lengths = ErrorSize::from(AxisDeltas::from(vec![Length(1.0)]));
        assert_eq!(
            end_values(&values, Axis3::Y, &lengths, flags, &[true]).err(),
            Some(Axes3DError::UnaddableErrors(Axis3::Y))
        );
    }

    #[test]
    fn nan_extremes_ignore_nan() {
        assert_eq!(nan_extremes(&[f64::NAN, 3.0, -1.0]), [-1.0, 3.0]);
        assert_eq!(nan_extremes(&[f64::NAN]).len(), 0);
    }
}
