//! What lists of plotting values are made of: the [`AxisItem`]s that
//! [`AxisValues`] collects from any collection or iterator.

use super::{AxisValues, Convertible, CustomValues};
use crate::dates::{DateTime, Datetime64, get_epoch};
use crate::numeric::{Number, with_numbers};

/// One item of a list of plotting values: a number of any primitive type
/// (`f64`, `f32`, `i32`, `u64`, `usize`, ...), a category name (`&str`,
/// `String`), a date ([`DateTime`], [`Datetime64`]) or a value of a
/// [`Convertible`] type. Numbers, names and dates count by value and by
/// reference, values of your own types by value.
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
/// conversion of each item.
macro_rules! items {
    ($variant:ident: $(|$item:ident: $type:ty| $convert:expr),+ $(,)?) => {$(
        impl AxisItem for $type {
            fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
                AxisValues::$variant(items.map(|$item| $convert).collect())
            }
        }
    )+};
}

items!(Categories:
    |name: String| name,
    |name: &String| name.clone(),
    |name: &str| name.to_owned(),
    |name: &&str| (*name).to_owned(),
);

// Naive date-times are taken as UTC, as matplotlib does by default.
items!(Dates:
    |date: DateTime| get_epoch().date2num(date),
    |date: &DateTime| get_epoch().date2num(*date),
    |date: Datetime64| get_epoch().datetime64_to_num(date),
    |date: &Datetime64| get_epoch().datetime64_to_num(*date),
);

impl<T: Convertible> AxisItem for T {
    fn collect(items: impl Iterator<Item = Self>) -> AxisValues {
        AxisValues::Custom(CustomValues::new(items.collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(AxisValues::from(&[day]), AxisValues::Dates(vec![1.0]));
        let instant = Datetime64::new(86_400, 0);
        assert_eq!(
            AxisValues::from(vec![instant]),
            AxisValues::Dates(vec![1.0])
        );

        assert!(AxisValues::from(Vec::<f64>::new()).is_empty());
    }
}
