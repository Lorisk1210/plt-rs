//! Extents along an axis before unit conversion: one ([`AxisDelta`]) or a
//! list of them ([`AxisDeltas`]), such as bar heights, bar widths and
//! error sizes (matplotlib's `dx` arguments, which may be `timedelta`s or
//! values of user types).

use super::{Convertible, CustomValues, from_value_lists};
use crate::dates::TimeDelta;

/// Microseconds in a day: a duration of that many microseconds spans one
/// unit of a date axis.
const MICROS_PER_DAY: f64 = 86_400_000_000.0;

/// The days `duration` lasts, the unit of date axes.
pub(crate) fn days(duration: TimeDelta) -> f64 {
    duration.total_micros() as f64 / MICROS_PER_DAY
}

/// An extent along an axis before unit conversion: a number, a duration
/// on a date axis (matplotlib's `timedelta` widths, heights and error
/// sizes) or a value of a [`Convertible`] type.
///
/// ```
/// use plt::dates::TimeDelta;
/// use plt::units::AxisDelta;
/// assert_eq!(AxisDelta::from(0.8).to_number(), Some(0.8));
/// assert_eq!(AxisDelta::from(TimeDelta::new(1, 43_200, 0)?).to_number(), Some(1.5));
/// # Ok::<(), plt::dates::DateError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum AxisDelta {
    /// A plain number.
    Number(f64),
    /// A duration.
    Duration(TimeDelta),
    /// A value of a user type: [`CustomValues`] holding one value.
    Custom(CustomValues),
}

impl AxisDelta {
    /// The extent in axis units where that needs no converter: numbers as
    /// they are, durations in days, the unit of date axes. A user value
    /// has none until an axis converts it.
    pub fn to_number(&self) -> Option<f64> {
        match self {
            Self::Number(delta) => Some(*delta),
            Self::Duration(duration) => Some(days(*duration)),
            Self::Custom(_) => None,
        }
    }

    /// The user value, if the extent is one of type `T`: how
    /// [`Convertible::add_delta`] looks at the extents it is given.
    pub fn downcast_ref<T: Convertible>(&self) -> Option<&T> {
        match self {
            Self::Custom(values) => values.downcast_ref::<T>()?.first(),
            Self::Number(_) | Self::Duration(_) => None,
        }
    }

    /// The extent `factor` times, if it scales (Python's `factor *
    /// delta`): numbers and durations do, user values as their
    /// [`Convertible::scale`] says.
    pub(crate) fn scale(&self, factor: i64) -> Option<Self> {
        match self {
            Self::Number(delta) => Some(Self::Number(factor as f64 * delta)),
            Self::Duration(duration) => duration.checked_mul(factor).map(Self::Duration),
            Self::Custom(value) => value.scale(factor).map(Self::Custom),
        }
    }

    /// Whether the extent is below zero, which error sizes may not be:
    /// numbers and durations can be, user values have no order here
    /// (matplotlib compares them with `err < -err`).
    pub(crate) fn is_negative(&self) -> bool {
        match self {
            Self::Number(delta) => *delta < 0.0,
            Self::Duration(duration) => duration.total_micros() < 0,
            Self::Custom(_) => false,
        }
    }
}

impl From<f64> for AxisDelta {
    fn from(delta: f64) -> Self {
        Self::Number(delta)
    }
}

impl From<TimeDelta> for AxisDelta {
    fn from(duration: TimeDelta) -> Self {
        Self::Duration(duration)
    }
}

impl<T: Convertible> From<T> for AxisDelta {
    fn from(value: T) -> Self {
        Self::Custom(CustomValues::new(vec![value]))
    }
}

/// Extents along an axis before unit conversion, one or one per item:
/// numbers, durations or values of a [`Convertible`] type (matplotlib's
/// bar heights and widths and error sizes, which convert through the
/// values they extend from, as its `_convert_dx` converts them).
///
/// ```
/// use plt::dates::TimeDelta;
/// use plt::units::AxisDeltas;
/// assert_eq!(AxisDeltas::from([0.5, 1.0]), AxisDeltas::Numbers(vec![0.5, 1.0]));
/// let day = TimeDelta::new(1, 0, 0)?;
/// assert_eq!(AxisDeltas::from(day).len(), 1);
/// # Ok::<(), plt::dates::DateError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum AxisDeltas {
    /// Plain numbers.
    Numbers(Vec<f64>),
    /// Durations, in days on a date axis.
    Durations(Vec<TimeDelta>),
    /// Values of a user type.
    Custom(CustomValues),
}

impl AxisDeltas {
    /// The number of extents.
    pub fn len(&self) -> usize {
        match self {
            Self::Numbers(deltas) => deltas.len(),
            Self::Durations(durations) => durations.len(),
            Self::Custom(values) => values.len(),
        }
    }

    /// Whether there are no extents.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The extents of a user type, if they are.
    pub fn custom(&self) -> Option<&CustomValues> {
        match self {
            Self::Custom(values) => Some(values),
            Self::Numbers(_) | Self::Durations(_) => None,
        }
    }

    /// The extents one by one.
    pub(crate) fn iter(&self) -> impl Iterator<Item = AxisDelta> + '_ {
        (0..self.len()).filter_map(|index| self.get(index))
    }

    /// Extent `index`.
    fn get(&self, index: usize) -> Option<AxisDelta> {
        match self {
            Self::Numbers(deltas) => deltas.get(index).copied().map(AxisDelta::Number),
            Self::Durations(durations) => durations.get(index).copied().map(AxisDelta::Duration),
            Self::Custom(values) => values.get(index).map(AxisDelta::Custom),
        }
    }

    /// The extents in axis units where that needs no converter (see
    /// [`AxisDelta::to_number`]), or the user values, which need one.
    pub(crate) fn to_numbers(&self) -> Result<Vec<f64>, &CustomValues> {
        match self {
            Self::Numbers(deltas) => Ok(deltas.clone()),
            Self::Durations(durations) => Ok(durations.iter().copied().map(days).collect()),
            Self::Custom(values) => Err(values),
        }
    }
}

impl From<AxisDelta> for AxisDeltas {
    /// The one extent.
    fn from(delta: AxisDelta) -> Self {
        match delta {
            AxisDelta::Number(delta) => Self::Numbers(vec![delta]),
            AxisDelta::Duration(duration) => Self::Durations(vec![duration]),
            AxisDelta::Custom(value) => Self::Custom(value),
        }
    }
}

impl From<Vec<f64>> for AxisDeltas {
    fn from(deltas: Vec<f64>) -> Self {
        Self::Numbers(deltas)
    }
}

impl From<Vec<TimeDelta>> for AxisDeltas {
    fn from(durations: Vec<TimeDelta>) -> Self {
        Self::Durations(durations)
    }
}

impl From<CustomValues> for AxisDeltas {
    fn from(values: CustomValues) -> Self {
        Self::Custom(values)
    }
}

impl<T: Convertible> From<Vec<T>> for AxisDeltas {
    fn from(values: Vec<T>) -> Self {
        Self::Custom(CustomValues::new(values))
    }
}

from_value_lists!(AxisDeltas: f64, TimeDelta);
from_value_lists!(impl<T: Convertible> AxisDeltas: T);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::Axis;
    use crate::units::{ConversionError, ConversionInterface};

    /// A length in metres.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Metres(f64);

    struct MetreConverter;

    impl ConversionInterface for MetreConverter {
        type Value = Metres;
        type Unit = ();

        fn convert(
            &self,
            values: &[Metres],
            _: Option<&()>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            Ok(values.iter().map(|m| m.0).collect())
        }
    }

    impl Convertible for Metres {
        type Converter = MetreConverter;
        fn converter() -> MetreConverter {
            MetreConverter
        }
    }

    #[test]
    fn single_extents_become_one_element_lists() {
        let back = TimeDelta::new(-2, 0, 0).expect("a duration");
        assert_eq!(AxisDelta::from(back).to_number(), Some(-2.0));
        assert_eq!(
            AxisDeltas::from(AxisDelta::from(back)),
            AxisDeltas::Durations(vec![back])
        );
        assert_eq!(AxisDeltas::from(0.5), AxisDeltas::Numbers(vec![0.5]));
        let metres = AxisDelta::from(Metres(3.0));
        assert_eq!(metres.to_number(), None, "user values need a converter");
        assert_eq!(metres.downcast_ref::<Metres>(), Some(&Metres(3.0)));
        assert_eq!(AxisDelta::from(1.0).downcast_ref::<Metres>(), None);
        assert_eq!(AxisDeltas::from(metres).len(), 1);
    }

    #[test]
    fn lists_hold_one_kind_of_extent() {
        let numbers = vec![1.0, 2.0];
        assert_eq!(AxisDeltas::from(&numbers), AxisDeltas::from(&[1.0, 2.0]));
        assert_eq!(
            AxisDeltas::from(numbers.as_slice()).to_numbers(),
            Ok(numbers)
        );
        let day = TimeDelta::new(1, 0, 0).expect("a duration");
        let durations = AxisDeltas::from([day, day]);
        assert_eq!(durations, AxisDeltas::from(vec![day, day]));
        assert_eq!(durations.to_numbers(), Ok(vec![1.0, 1.0]));
        assert_eq!(AxisDeltas::from(&[day][..]).len(), 1);
        let lengths = AxisDeltas::from([Metres(1.0), Metres(2.0)]);
        assert_eq!(lengths, AxisDeltas::from(&[Metres(1.0), Metres(2.0)][..]));
        assert!(
            lengths.to_numbers().is_err(),
            "user values need a converter"
        );
        let second = lengths.iter().nth(1).expect("two extents");
        assert_eq!(second.downcast_ref::<Metres>(), Some(&Metres(2.0)));
        assert!(AxisDeltas::from(Vec::<f64>::new()).is_empty());
        assert_eq!(
            AxisDeltas::from(CustomValues::new(vec![Metres(1.0)])).len(),
            1
        );
    }

    #[test]
    fn values_and_extents_come_in_the_same_forms() {
        use crate::dates::DateTime;
        use crate::units::{AxisValue, AxisValues};

        let lengths = vec![Metres(1.0), Metres(2.0)];
        let custom = AxisDeltas::from(lengths.clone());
        assert_eq!(AxisDeltas::from(&lengths), custom);
        assert_eq!(AxisDeltas::from(&[Metres(1.0), Metres(2.0)]), custom);
        assert_eq!(AxisDeltas::from(Metres(3.0)).len(), 1);
        assert_eq!(
            custom
                .custom()
                .and_then(CustomValues::downcast_ref::<Metres>),
            Some(lengths.as_slice())
        );
        assert_eq!(AxisDeltas::from(0.5).custom(), None);
        let day = TimeDelta::new(1, 0, 0).expect("a duration");
        assert_eq!(AxisDeltas::from(&vec![day]), AxisDeltas::from(&[day]));

        let values = AxisValues::from(lengths.clone());
        assert_eq!(AxisValues::from(&lengths), values);
        assert_eq!(AxisValues::from(Metres(1.0)).len(), 1);
        assert_eq!(AxisValues::from(2.5), AxisValues::Numbers(vec![2.5]));
        assert_eq!(AxisValues::from("a"), AxisValues::from(["a"]));
        let names = vec![String::from("a")];
        assert_eq!(AxisValues::from(&names), AxisValues::from(names.clone()));
        let date = DateTime::new(1970, 1, 2, 0, 0, 0, 0).expect("a date");
        assert_eq!(AxisValues::from([date]), AxisValues::Dates(vec![1.0]));
        assert_eq!(AxisValues::from(&[date]), AxisValues::from(date));

        let metres = AxisValue::from(Metres(4.0));
        assert_eq!(metres.downcast_ref::<Metres>(), Some(&Metres(4.0)));
        assert_eq!(AxisValue::from(4.0).downcast_ref::<Metres>(), None);
    }

    #[test]
    fn extents_scale_and_tell_their_sign() {
        let hour = TimeDelta::new(0, 3600, 0).expect("a duration");
        assert_eq!(
            AxisDelta::from(hour).scale(-2),
            Some(AxisDelta::from(
                TimeDelta::new(0, -7200, 0).expect("a duration")
            ))
        );
        assert_eq!(
            AxisDelta::from(0.5).scale(-1),
            Some(AxisDelta::Number(-0.5))
        );
        assert_eq!(
            AxisDelta::from(Metres(1.0)).scale(2),
            None,
            "no scale given"
        );
        assert!(AxisDelta::from(-0.5).is_negative());
        assert!(!AxisDelta::from(f64::NAN).is_negative());
        assert!(AxisDelta::from(TimeDelta::new(-1, 0, 0).expect("a duration")).is_negative());
        assert!(
            !AxisDelta::from(Metres(-1.0)).is_negative(),
            "user values have no order"
        );
    }
}
