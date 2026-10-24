//! Single plotting values before unit conversion: a value along an axis
//! ([`AxisValue`]), a point of two of them ([`DataPoint`]) and an extent
//! along an axis ([`AxisDelta`]).

use plt_core::Point;

use super::AxisValues;
use crate::dates::{DateTime, Datetime64, TimeDelta, get_epoch};

/// Microseconds in a day: a duration of that many microseconds spans one
/// unit of a date axis.
const MICROS_PER_DAY: f64 = 86_400_000_000.0;

/// One plotting value along an axis before unit conversion: a number, a
/// category name or a date (the scalars matplotlib passes through
/// `convert_xunits`).
///
/// ```
/// use plt::dates::DateTime;
/// use plt::units::AxisValue;
/// assert_eq!(AxisValue::from(2.5), AxisValue::Number(2.5));
/// let day = DateTime::new(1970, 1, 3, 12, 0, 0, 0)?;
/// assert_eq!(AxisValue::from(day), AxisValue::Date(2.5));
/// # Ok::<(), plt::dates::DateError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum AxisValue {
    /// A plain number.
    Number(f64),
    /// A category name.
    Category(String),
    /// A date as a date number (days since the
    /// [epoch](crate::dates::get_epoch)).
    Date(f64),
}

impl From<f64> for AxisValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<DateTime> for AxisValue {
    /// Naive date-times are taken as UTC, as matplotlib does by default.
    fn from(date: DateTime) -> Self {
        Self::Date(get_epoch().date2num(date))
    }
}

impl From<Datetime64> for AxisValue {
    fn from(date: Datetime64) -> Self {
        Self::Date(get_epoch().datetime64_to_num(date))
    }
}

impl From<&str> for AxisValue {
    fn from(name: &str) -> Self {
        Self::Category(name.to_owned())
    }
}

impl From<String> for AxisValue {
    fn from(name: String) -> Self {
        Self::Category(name)
    }
}

impl From<AxisValue> for AxisValues {
    /// The one value.
    fn from(value: AxisValue) -> Self {
        match value {
            AxisValue::Number(value) => Self::Numbers(vec![value]),
            AxisValue::Category(name) => Self::Categories(vec![name]),
            AxisValue::Date(value) => Self::Dates(vec![value]),
        }
    }
}

/// A point given by two plotting values before unit conversion, x then y
/// (matplotlib's `xy` arguments in data coordinates).
///
/// ```
/// use plt::dates::DateTime;
/// use plt::units::{AxisValue, DataPoint};
/// let point = DataPoint::from((DateTime::new(1970, 1, 2, 0, 0, 0, 0)?, 5.0));
/// assert_eq!((point.x, point.y), (AxisValue::Date(1.0), AxisValue::Number(5.0)));
/// # Ok::<(), plt::dates::DateError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DataPoint {
    /// The x value.
    pub x: AxisValue,
    /// The y value.
    pub y: AxisValue,
}

impl<X: Into<AxisValue>, Y: Into<AxisValue>> From<(X, Y)> for DataPoint {
    fn from((x, y): (X, Y)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }
}

impl From<Point> for DataPoint {
    fn from(point: Point) -> Self {
        (point.x, point.y).into()
    }
}

impl From<[f64; 2]> for DataPoint {
    fn from([x, y]: [f64; 2]) -> Self {
        (x, y).into()
    }
}

/// An extent along an axis before unit conversion: a number, or a
/// duration on a date axis (matplotlib's `timedelta` widths, heights and
/// error sizes).
///
/// ```
/// use plt::dates::TimeDelta;
/// use plt::units::AxisDelta;
/// assert_eq!(AxisDelta::from(0.8).to_number(), 0.8);
/// assert_eq!(AxisDelta::from(TimeDelta::new(1, 43_200, 0)?).to_number(), 1.5);
/// # Ok::<(), plt::dates::DateError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisDelta {
    /// A plain number.
    Number(f64),
    /// A duration.
    Duration(TimeDelta),
}

impl AxisDelta {
    /// The extent in axis units: durations in days, the unit of date axes.
    pub fn to_number(self) -> f64 {
        match self {
            Self::Number(delta) => delta,
            Self::Duration(duration) => duration.total_micros() as f64 / MICROS_PER_DAY,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_values_become_one_element_lists() {
        assert_eq!(
            AxisValues::from(AxisValue::from("a")),
            AxisValues::Categories(vec!["a".into()])
        );
        assert_eq!(
            AxisValues::from(AxisValue::from(Datetime64::new(86_400, 0))),
            AxisValues::Dates(vec![1.0])
        );
        assert_eq!(
            AxisValues::from(AxisValue::from(3.0)),
            AxisValues::Numbers(vec![3.0])
        );
    }

    #[test]
    fn points_and_extents_convert() {
        assert_eq!(
            DataPoint::from(Point::new(1.0, 2.0)),
            DataPoint::from([1.0, 2.0])
        );
        let point = DataPoint::from(("b", String::from("c")));
        assert_eq!(point.y, AxisValue::Category("c".into()));
        let back = TimeDelta::new(-2, 0, 0).expect("a duration");
        assert_eq!(AxisDelta::from(back).to_number(), -2.0);
    }
}
