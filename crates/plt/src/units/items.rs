//! What lists of plotting values are made of: the [`AxisItem`]s that
//! [`AxisValues`] collects from any collection or iterator.

use std::borrow::Borrow;

use super::scalar::zoned_date_number;
use super::{AxisValues, Convertible, CustomValues};
use crate::dates::{DateTime, Datetime64, ZonedDateTime, get_epoch};
use crate::numeric::{Number, with_numbers};

/// One item of a list of plotting values: a number of any primitive type
/// (`f64`, `f32`, `i32`, `u64`, `usize`, ...), a category name (`&str`,
/// `String`), a date ([`DateTime`], [`Datetime64`], [`ZonedDateTime`])
/// or a value of a [`Convertible`] type. Numbers, names and dates count
/// by value and by reference, values of your own types by value.
///
/// Zoned date-times become the date numbers of their instants, and the
/// list keeps the zone of the first, which an axis without a unit takes
/// as its unit, as matplotlib's date converter takes the `tzinfo` of the
/// first aware `datetime` (`default_units`) and converts them all to UTC
/// (`date2num`): a list mixing zones is in the first one's.
///
/// Any collection or iterator of one kind of item is [`AxisValues`]:
/// arrays, slices, `Vec`s and references to them, ranges, and iterators,
/// so that data computed on the fly goes straight into a plot.
///
/// ```
/// use plt::prelude::*;
/// let ax = Figure::new().subplot();
/// let counts: Vec<u32> = vec![3, 8, 5];
/// let line = ax.plot(0..3, &counts)?;
/// assert_eq!(line.ydata(), [3.0, 8.0, 5.0]);
///
/// let x = linspace(0.0, 1.0, 5);
/// let squares = ax.plot(&x, x.iter().map(|x| x * x))?;
/// assert_eq!(squares.ydata()[2], 0.25);
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
pub trait AxisItem: Sized {
    /// The values `items` make.
    fn collect(items: impl Iterator<Item = Self>) -> AxisValues;
}

impl<I> From<I> for AxisValues
where
    I: IntoIterator,
    I::Item: AxisItem,
{
    fn from(items: I) -> Self {
        AxisItem::collect(items.into_iter())
    }
}

/// Implements [`AxisItem`] for number types, by value and by reference.
macro_rules! number_items {
    ($($number:ty),+) => {$(
        impl AxisItem for $number {
            fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
                AxisValues::Numbers(items.map(Number::to_f64).collect())
            }
        }

        impl AxisItem for &$number {
            fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
                <$number>::collect(items.copied())
            }
        }
    )+};
}

with_numbers!(number_items!());

/// Implements [`AxisItem`] for a type and references to it, through a
/// conversion of each item and `$values` making the list of them.
macro_rules! items {
    ($values:path: $(|$item:ident: $type:ty| $convert:expr),+ $(,)?) => {$(
        impl AxisItem for $type {
            fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
                $values(items.map(|$item| $convert).collect())
            }
        }
    )+};
}

items!(AxisValues::Categories:
    |name: String| name,
    |name: &String| name.clone(),
    |name: &str| name.to_owned(),
    |name: &&str| (*name).to_owned(),
);

// Naive date-times are taken as UTC, as matplotlib does by default.
items!(naive_dates:
    |date: DateTime| get_epoch().date2num(date),
    |date: &DateTime| get_epoch().date2num(*date),
    |date: Datetime64| get_epoch().datetime64_to_num(date),
    |date: &Datetime64| get_epoch().datetime64_to_num(*date),
);

/// The date numbers of naive dates, which carry no zone.
fn naive_dates(numbers: Vec<f64>) -> AxisValues {
    AxisValues::Dates { numbers, tz: None }
}

impl AxisItem for ZonedDateTime {
    fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
        zoned_dates(items)
    }
}

impl AxisItem for &ZonedDateTime {
    fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
        zoned_dates(items)
    }
}

/// The date numbers of zoned date-times, with the zone of the first.
fn zoned_dates(items: impl Iterator<Item = impl Borrow<ZonedDateTime>>) -> AxisValues {
    let epoch = get_epoch();
    let mut tz = None;
    let numbers = items
        .map(|date| {
            let date = date.borrow();
            tz.get_or_insert_with(|| date.tz().clone());
            zoned_date_number(&epoch, date)
        })
        .collect();
    AxisValues::Dates { numbers, tz }
}

impl<T: Convertible> AxisItem for T {
    fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
        AxisValues::Custom(CustomValues::new(items.collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{TimeZone, UtcOffset};

    #[test]
    fn every_form_of_every_kind_converts() {
        let numbers = AxisValues::Numbers(vec![1.0, 2.0]);
        assert_eq!(AxisValues::from([1.0, 2.0]), numbers);
        assert_eq!(AxisValues::from(vec![1_i64, 2]), numbers);
        assert_eq!(AxisValues::from(&[1_u8, 2][..]), numbers);
        assert_eq!(AxisValues::from(&vec![1.0_f32, 2.0]), numbers);
        assert_eq!(AxisValues::from(1_usize..3), numbers);
        assert_eq!(
            AxisValues::from([0.5, 1.0].iter().map(|x| x * 2.0)),
            numbers
        );

        let names = AxisValues::Categories(vec!["a".into(), "b".into()]);
        assert_eq!(AxisValues::from(["a", "b"]), names);
        assert_eq!(AxisValues::from(&["a", "b"]), names);
        assert_eq!(AxisValues::from(vec![String::from("a"), "b".into()]), names);
        assert_eq!(AxisValues::from("ab".chars().map(String::from)), names);

        let day = DateTime::new(1970, 1, 2, 0, 0, 0, 0).unwrap();
        let naive = AxisValues::Dates {
            numbers: vec![1.0],
            tz: None,
        };
        assert_eq!(AxisValues::from(&[day]), naive);
        let instant = Datetime64::new(86_400, 0);
        assert_eq!(AxisValues::from(vec![instant]), naive);
        let utc = ZonedDateTime::utc(day);
        let zoned = AxisValues::Dates {
            numbers: vec![1.0],
            tz: Some(TimeZone::Utc),
        };
        assert_eq!(AxisValues::from(std::slice::from_ref(&utc)), zoned);
        assert_eq!(AxisValues::from([utc]), zoned);

        assert!(AxisValues::from(Vec::<f64>::new()).is_empty());
    }

    #[test]
    fn a_list_mixing_zones_is_in_the_first_ones_zone() {
        let hours = |h: i32| TimeZone::Fixed(UtcOffset::from_seconds(h * 3600).unwrap());
        let local = |h| DateTime::new(1970, 1, 2, h, 0, 0, 0).unwrap();
        let dates = [
            ZonedDateTime::from_local(local(1), hours(1)),
            ZonedDateTime::from_local(local(18), hours(-6)),
            ZonedDateTime::utc(local(12)),
        ];
        let values = AxisValues::from(&dates);
        let numbers = vec![1.0, 2.0, 1.5];
        let tz = Some(hours(1));
        assert_eq!(values, AxisValues::Dates { numbers, tz });
        let none = AxisValues::from(Vec::<ZonedDateTime>::new());
        assert_eq!(
            none,
            AxisValues::Dates {
                numbers: Vec::new(),
                tz: None
            }
        );
    }
}
