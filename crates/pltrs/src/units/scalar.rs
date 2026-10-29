//! Single plotting values before unit conversion: a value along an axis
//! ([`AxisValue`]) and a point of two of them ([`DataPoint`]).

use pltrs_core::Point;

use super::{AxisDelta, AxisValues, Convertible, CustomValues};
use crate::dates::{DateTime, Datetime64, Epoch, TimeZone, ZonedDateTime, get_epoch};
use crate::numeric::{Number, with_numbers};

/// One plotting value along an axis before unit conversion: a number of
/// any primitive type, a category name, a date or a value of a
/// [`Convertible`] type (the scalars matplotlib passes through
/// `convert_xunits`).
///
/// ```
/// use pltrs::dates::{DateTime, TimeZone, ZonedDateTime};
/// use pltrs::units::AxisValue;
/// assert_eq!(AxisValue::from(2.5), AxisValue::Number(2.5));
/// assert_eq!(AxisValue::from(3_u32), AxisValue::Number(3.0));
/// let day = DateTime::new(1970, 1, 3, 12, 0, 0, 0)?;
/// assert_eq!(AxisValue::from(day), AxisValue::Date { number: 2.5, tz: None });
/// let noon = ZonedDateTime::utc(day);
/// let tz = Some(TimeZone::Utc);
/// assert_eq!(AxisValue::from(&noon), AxisValue::Date { number: 2.5, tz });
/// # Ok::<(), pltrs::dates::DateError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum AxisValue {
    /// A plain number.
    Number(f64),
    /// A category name.
    Category(String),
    /// A date, naive or in a zone.
    Date {
        /// The instant as a date number (days since the
        /// [epoch](crate::dates::get_epoch), in UTC).
        number: f64,
        /// The zone of a [`ZonedDateTime`], which an axis without a unit
        /// takes as its unit (matplotlib's date converter's
        /// `default_units`, the value's `tzinfo`); `None` for a naive
        /// date, which leaves the unit alone.
        tz: Option<TimeZone>,
    },
    /// A value of a user type: [`CustomValues`] holding one value.
    Custom(CustomValues),
}

impl AxisValue {
    /// The user value, if the value is one of type `T` (as
    /// [`AxisDelta::downcast_ref`] for extents).
    pub fn downcast_ref<T: Convertible>(&self) -> Option<&T> {
        match self {
            Self::Custom(values) => values.downcast_ref::<T>()?.first(),
            Self::Number(_) | Self::Category(_) | Self::Date { .. } => None,
        }
    }

    /// The value an extent `delta` reaches from this one, if the extent
    /// adds to it (Python's `value + delta`): a number to a number, a
    /// duration to a date (on the instant, as Python adds a `timedelta`
    /// to a `datetime`: [`Epoch::add_duration`](crate::dates::Epoch::add_duration),
    /// keeping its zone), and to a user value what its
    /// [`Convertible::add_delta`] adds.
    pub(crate) fn add_delta(&self, delta: &AxisDelta) -> Option<Self> {
        match (self, delta) {
            (Self::Number(value), AxisDelta::Number(delta)) => Some(Self::Number(value + delta)),
            (Self::Date { number, tz }, AxisDelta::Duration(duration)) => Some(Self::Date {
                number: get_epoch().add_duration(*number, *duration),
                tz: tz.clone(),
            }),
            (Self::Custom(value), delta) => value.add_delta(delta),
            _ => None,
        }
    }

    /// The value `factor` extents `delta` away, if the extent scales and
    /// adds to it (Python's `value + factor * delta`, matplotlib's ends
    /// of error bars).
    pub(crate) fn offset(&self, delta: &AxisDelta, factor: i64) -> Option<Self> {
        self.add_delta(&delta.scale(factor)?)
    }
}

impl<T: Convertible> From<T> for AxisValue {
    fn from(value: T) -> Self {
        Self::Custom(CustomValues::new(vec![value]))
    }
}

/// Implements `From` for each number type for a type holding a number in
/// its `Number` variant.
macro_rules! from_numbers {
    ($target:ident; $($number:ty),+) => {$(
        impl From<$number> for $target {
            fn from(value: $number) -> Self {
                Self::Number(value.to_f64())
            }
        }
    )+};
}

pub(super) use from_numbers;

with_numbers!(from_numbers!(AxisValue;));

impl From<DateTime> for AxisValue {
    /// Naive date-times are taken as UTC, as matplotlib does by default.
    fn from(date: DateTime) -> Self {
        naive_date(get_epoch().date2num(date))
    }
}

impl From<Datetime64> for AxisValue {
    fn from(date: Datetime64) -> Self {
        naive_date(get_epoch().datetime64_to_num(date))
    }
}

impl From<&ZonedDateTime> for AxisValue {
    /// The instant, and the zone an axis without a unit takes.
    fn from(date: &ZonedDateTime) -> Self {
        Self::Date {
            number: zoned_date_number(&get_epoch(), date),
            tz: Some(date.tz().clone()),
        }
    }
}

impl From<ZonedDateTime> for AxisValue {
    /// The instant, and the zone an axis without a unit takes.
    fn from(date: ZonedDateTime) -> Self {
        Self::from(&date)
    }
}

/// A date without a zone.
fn naive_date(number: f64) -> AxisValue {
    AxisValue::Date { number, tz: None }
}

/// The date number of the instant `date` stands for (matplotlib's
/// `date2num`, which converts aware date-times to UTC first); NaN when
/// that instant is outside years 1 to 9999 in UTC, where Python fails to
/// convert it.
pub(super) fn zoned_date_number(epoch: &Epoch, date: &ZonedDateTime) -> f64 {
    date.to_utc()
        .map_or(f64::NAN, |instant| epoch.date2num(instant))
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
            AxisValue::Date { number, tz } => Self::Dates {
                numbers: vec![number],
                tz,
            },
            AxisValue::Custom(value) => Self::Custom(value),
        }
    }
}

/// A point given by two plotting values before unit conversion, x then y
/// (matplotlib's `xy` arguments in data coordinates).
///
/// ```
/// use pltrs::dates::DateTime;
/// use pltrs::units::{AxisValue, DataPoint};
/// let point = DataPoint::from((DateTime::new(1970, 1, 2, 0, 0, 0, 0)?, 5.0));
/// let day = AxisValue::Date { number: 1.0, tz: None };
/// assert_eq!((point.x, point.y), (day, AxisValue::Number(5.0)));
/// # Ok::<(), pltrs::dates::DateError>(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{TimeDelta, UtcOffset};

    #[test]
    fn single_values_become_one_element_lists() {
        assert_eq!(
            AxisValues::from(AxisValue::from("a")),
            AxisValues::Categories(vec!["a".into()])
        );
        assert_eq!(
            AxisValues::from(AxisValue::from(Datetime64::new(86_400, 0))),
            AxisValues::Dates {
                numbers: vec![1.0],
                tz: None
            }
        );
        assert_eq!(
            AxisValues::from(AxisValue::from(3.0)),
            AxisValues::Numbers(vec![3.0])
        );
    }

    #[test]
    fn points_convert() {
        assert_eq!(
            DataPoint::from(Point::new(1.0, 2.0)),
            DataPoint::from([1.0, 2.0])
        );
        let point = DataPoint::from(("b", String::from("c")));
        assert_eq!(point.y, AxisValue::Category("c".into()));
    }

    #[test]
    fn extents_add_to_values_of_their_kind() {
        let day = TimeDelta::new(1, 43_200, 0).expect("a duration");
        let date = naive_date(10.0);
        assert_eq!(date.add_delta(&day.into()), Some(naive_date(11.5)));
        assert_eq!(date.add_delta(&AxisDelta::Number(1.0)), None);
        let number = AxisValue::Number(0.25);
        assert_eq!(
            number.add_delta(&AxisDelta::Number(0.5)),
            Some(AxisValue::Number(0.75))
        );
        assert_eq!(number.add_delta(&day.into()), None);
        assert_eq!(
            AxisValue::from("a").add_delta(&AxisDelta::Number(1.0)),
            None
        );
    }

    #[test]
    fn durations_add_to_the_instants_of_dates() {
        let at = |h, s| AxisValue::from(DateTime::new(2020, 5, 1, h, 0, s, 0).unwrap());
        let eight_hours = AxisDelta::from(TimeDelta::new(0, 8 * 3600, 0).unwrap());
        // Adding a third of a day to the date number rounds the other way.
        assert_eq!(at(0, 1).add_delta(&eight_hours), Some(at(8, 1)));
        // A date number of no whole microsecond adds as a number.
        let odd = naive_date(0.123_456_789_012_345);
        let day = AxisDelta::from(TimeDelta::new(1, 0, 0).unwrap());
        assert_eq!(odd.add_delta(&day), Some(naive_date(1.123_456_789_012_345)));
        assert!(matches!(
            naive_date(f64::NAN).add_delta(&day),
            Some(AxisValue::Date { number, .. }) if number.is_nan()
        ));
    }

    #[test]
    fn offsets_scale_the_extent_first() {
        let back = TimeDelta::new(0, 3600, 0).unwrap().into();
        let noon = AxisValue::from(DateTime::new(2020, 5, 1, 12, 0, 0, 0).unwrap());
        let eleven = AxisValue::from(DateTime::new(2020, 5, 1, 11, 0, 0, 0).unwrap());
        assert_eq!(noon.offset(&back, -1), Some(eleven));
        assert_eq!(noon.offset(&back, 0), Some(noon.clone()));
        assert_eq!(
            AxisValue::Number(1.0).offset(&AxisDelta::Number(0.25), -1),
            Some(AxisValue::Number(0.75))
        );
        assert_eq!(AxisValue::Number(1.0).offset(&back, 1), None);
    }

    #[test]
    fn zoned_dates_are_their_instants_in_their_zone() {
        let offset = UtcOffset::from_seconds(-5 * 3600).unwrap();
        let tz = TimeZone::Fixed(offset);
        let local = DateTime::new(1970, 1, 1, 19, 0, 0, 0).unwrap();
        let evening = AxisValue::from(ZonedDateTime::from_local(local, tz.clone()));
        let tz = Some(tz);
        assert_eq!(
            evening,
            AxisValue::Date {
                number: 1.0,
                tz: tz.clone()
            }
        );
        // Durations keep the zone, as Python's aware arithmetic does.
        let hour = AxisDelta::from(TimeDelta::from_hours(6));
        let later = evening.add_delta(&hour);
        assert_eq!(later, Some(AxisValue::Date { number: 1.25, tz }));
        // Before year 1 in UTC, where Python's astimezone fails.
        let first = DateTime::new(1, 1, 1, 0, 0, 0, 0).unwrap();
        let east = TimeZone::Fixed(UtcOffset::from_seconds(3600).unwrap());
        let early = AxisValue::from(ZonedDateTime::from_local(first, east));
        assert!(matches!(early, AxisValue::Date { number, tz: Some(_) } if number.is_nan()));
    }
}
