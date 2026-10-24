//! Error sizes of error bars (matplotlib's `xerr` and `yerr`) and the ends
//! of the bars they give, computed in the units of the data.

use std::convert::Infallible;

use super::PlotError;
use super::broadcast::broadcast;
use crate::dates::TimeDelta;
use crate::units::{AxisDelta, AxisDeltas, AxisValue, Convertible, CustomValues, from_value_lists};

/// Error sizes along one axis (matplotlib's `xerr`/`yerr`): numbers,
/// durations or user values ([`AxisDeltas`]), every list holding one
/// value or one per point. Whatever makes [`AxisDeltas`] makes symmetric
/// error sizes.
///
/// ```
/// use plt::axes::ErrorSize;
/// use plt::dates::TimeDelta;
/// use plt::units::AxisDeltas;
/// assert_eq!(ErrorSize::from([0.1, 0.2]), ErrorSize::Symmetric(AxisDeltas::from([0.1, 0.2])));
/// let hour = TimeDelta::new(0, 3600, 0)?;
/// assert_eq!(ErrorSize::from(hour), ErrorSize::Symmetric(AxisDeltas::Durations(vec![hour])));
/// # Ok::<(), plt::dates::DateError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorSize {
    /// The same error below and above.
    Symmetric(AxisDeltas),
    /// Separate errors below and above.
    Asymmetric {
        /// Errors below the values.
        lower: AxisDeltas,
        /// Errors above the values.
        upper: AxisDeltas,
    },
}

impl From<AxisDeltas> for ErrorSize {
    fn from(errors: AxisDeltas) -> Self {
        Self::Symmetric(errors)
    }
}

impl From<AxisDelta> for ErrorSize {
    fn from(error: AxisDelta) -> Self {
        Self::Symmetric(error.into())
    }
}

impl From<CustomValues> for ErrorSize {
    fn from(errors: CustomValues) -> Self {
        Self::Symmetric(errors.into())
    }
}

impl From<Vec<f64>> for ErrorSize {
    fn from(errors: Vec<f64>) -> Self {
        Self::Symmetric(errors.into())
    }
}

impl From<Vec<TimeDelta>> for ErrorSize {
    /// Durations below and above, on a date axis (matplotlib's
    /// `timedelta` errors).
    fn from(errors: Vec<TimeDelta>) -> Self {
        Self::Symmetric(errors.into())
    }
}

impl<T: Convertible> From<Vec<T>> for ErrorSize {
    fn from(errors: Vec<T>) -> Self {
        Self::Symmetric(errors.into())
    }
}

from_value_lists!(ErrorSize: f64, TimeDelta);
from_value_lists!(impl<T: Convertible> ErrorSize: T);

/// The lower and upper ends of error bars, one of each per point, in the
/// units of the data.
pub(crate) type BarEnds = (Vec<AxisValue>, Vec<AxisValue>);

impl ErrorSize {
    /// The errors below and above, each list converted by `convert`.
    pub(crate) fn sides<E>(
        &self,
        mut convert: impl FnMut(&AxisDeltas) -> Result<Vec<f64>, E>,
    ) -> Result<(Vec<f64>, Vec<f64>), E> {
        match self {
            Self::Symmetric(errors) => {
                let errors = convert(errors)?;
                Ok((errors.clone(), errors))
            }
            Self::Asymmetric { lower, upper } => Ok((convert(lower)?, convert(upper)?)),
        }
    }

    /// The sizes in axis numbers, each list converted by `convert`.
    pub(crate) fn converted(&self, mut convert: impl FnMut(&AxisDeltas) -> Vec<f64>) -> Self {
        let Ok((lower, upper)) = self.sides(|errors| Ok::<_, Infallible>(convert(errors)));
        match self {
            Self::Symmetric(_) => Self::Symmetric(lower.into()),
            Self::Asymmetric { .. } => Self::Asymmetric {
                lower: lower.into(),
                upper: upper.into(),
            },
        }
    }

    /// The ends of the error bars of `values`, given as `argument`, in
    /// their units: each value less its lower error unless it is a lower
    /// limit (`lolims`), and plus its upper error unless it is an upper
    /// limit (matplotlib's `dep + np.vstack([-(1 - lolims), 1 - uplims]) *
    /// err`, which keeps the ends of NaN errors NaN).
    ///
    /// # Errors
    ///
    /// When the errors do not broadcast to the values, are negative, or do
    /// not scale or add to the values (matplotlib's `TypeError`).
    pub(crate) fn ends(
        &self,
        argument: &'static str,
        values: &[AxisValue],
        lolims: &[bool],
        uplims: &[bool],
    ) -> Result<BarEnds, PlotError> {
        let n = values.len();
        let (lower, upper) = match self {
            Self::Symmetric(errors) => (errors, errors),
            Self::Asymmetric { lower, upper } => (lower, upper),
        };
        let lower = broadcast(argument, &lower.iter().collect::<Vec<_>>(), n)?;
        let upper = broadcast(argument, &upper.iter().collect::<Vec<_>>(), n)?;
        if lower.iter().chain(&upper).any(AxisDelta::is_negative) {
            return Err(PlotError::NegativeErrors(argument));
        }
        let end = |index: usize, error: &AxisDelta, factor: i64| {
            values[index]
                .offset(error, factor)
                .ok_or(PlotError::UnaddableErrors(argument))
        };
        let low = (0..n)
            .map(|i| end(i, &lower[i], -(1 - i64::from(lolims[i]))))
            .collect::<Result<_, _>>()?;
        let high = (0..n)
            .map(|i| end(i, &upper[i], 1 - i64::from(uplims[i])))
            .collect::<Result<_, _>>()?;
        Ok((low, high))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::DateTime;

    fn numbers(values: &[f64]) -> Vec<AxisValue> {
        values.iter().copied().map(AxisValue::Number).collect()
    }

    #[test]
    fn ends_extend_from_the_values_in_their_units() {
        let values = numbers(&[1.0, f64::NAN, 3.0]);
        let errors = ErrorSize::Asymmetric {
            lower: vec![0.5, 0.5, f64::NAN].into(),
            upper: 0.25.into(),
        };
        let flags = [false; 3];
        let (low, high) = errors.ends("yerr", &values, &flags, &flags).unwrap();
        assert_eq!(low[0], AxisValue::Number(0.5));
        assert!(matches!(low[1], AxisValue::Number(v) if v.is_nan()));
        assert!(
            matches!(low[2], AxisValue::Number(v) if v.is_nan()),
            "NaN stays NaN"
        );
        assert_eq!(high[2], AxisValue::Number(3.25));
        let (low, high) = errors
            .ends(
                "yerr",
                &values,
                &[true, false, false],
                &[false, false, true],
            )
            .unwrap();
        assert_eq!(
            low[0],
            AxisValue::Number(1.0),
            "a lower limit has no lower bar"
        );
        assert_eq!(
            high[2],
            AxisValue::Number(3.0),
            "an upper limit no upper bar"
        );
    }

    #[test]
    fn durations_add_to_dates_alone() {
        let day = |d| DateTime::new(2020, 5, d, 0, 0, 0, 0).unwrap();
        let dates = [AxisValue::from(day(2))];
        let half_day = ErrorSize::from(TimeDelta::new(0, 43_200, 0).unwrap());
        let (low, high) = half_day.ends("xerr", &dates, &[false], &[false]).unwrap();
        let at = |d, h| AxisValue::from(DateTime::new(2020, 5, d, h, 0, 0, 0).unwrap());
        assert_eq!((&low[0], &high[0]), (&at(1, 12), &at(2, 12)));
        assert_eq!(
            half_day.ends("xerr", &numbers(&[1.0]), &[false], &[false]),
            Err(PlotError::UnaddableErrors("xerr")),
            "a number plus a duration fails, as in Python"
        );
        assert_eq!(
            ErrorSize::from(0.5).ends("yerr", &dates, &[false], &[false]),
            Err(PlotError::UnaddableErrors("yerr"))
        );
    }

    #[test]
    fn errors_must_broadcast_and_not_be_negative() {
        let values = numbers(&[1.0, 2.0, 3.0]);
        let flags = [false; 3];
        assert_eq!(
            ErrorSize::from(vec![1.0, 2.0]).ends("xerr", &values, &flags, &flags),
            Err(PlotError::Broadcast {
                argument: "xerr",
                len: 2,
                points: 3
            })
        );
        assert_eq!(
            ErrorSize::from(-1.0).ends("xerr", &values, &flags, &flags),
            Err(PlotError::NegativeErrors("xerr"))
        );
        let back = TimeDelta::new(-1, 0, 0).unwrap();
        assert_eq!(
            ErrorSize::from(back).ends("xerr", &values, &flags, &flags),
            Err(PlotError::NegativeErrors("xerr"))
        );
    }
}
