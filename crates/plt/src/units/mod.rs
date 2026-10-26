//! Unit conversion of plotting values (matplotlib's `units`, `category`
//! and date converters): numbers pass through unchanged, category names
//! are placed at 0, 1, 2, ... in order of their first appearance on an
//! axis, and dates become date numbers on an axis with date ticks.
//!
//! ```
//! use plt::prelude::*;
//! let ax = Figure::new().subplot();
//! let bars = ax.bar(["apple", "banana", "cherry"], [4.0, 6.0, 3.0], Default::default())?;
//! assert_eq!(bars.patches[1].shape().xy.x, 0.6);
//! # Ok::<(), plt::axes::PlotError>(())
//! ```
//!
//! The plotting methods take the values they place as [`AxisValues`] (or
//! one [`AxisValue`], or a [`DataPoint`] of two) and convert them on the
//! axis they go along, as matplotlib converts its `x` and `y` arguments.
//! Extents, such as bar heights and widths, error sizes and the sides of
//! a [`DataRectangle`](crate::patches::DataRectangle), are [`AxisDeltas`]
//! (or one [`AxisDelta`]): numbers, durations or user values, converted
//! through the values they extend from (matplotlib's `_convert_dx`).
//! Other option fields hold axis numbers: convert dates for them with
//! [`Axis::convert_value`](crate::axes::Axis::convert_value) or
//! [`Axis::convert_units`](crate::axes::Axis::convert_units), which give
//! the axis its date ticks too.
//!
//! ```
//! use plt::dates::DateTime;
//! use plt::prelude::*;
//! use plt::units::UnitConverter;
//! let ax = Figure::new().subplot();
//! let day = |d| DateTime::new(2020, 6, d, 0, 0, 0, 0);
//! ax.plot(vec![day(1)?, day(9)?], [1.0, 2.0])?;
//! ax.axvline(day(5)?, LineProps::default());
//! let note = ax.text(day(2)?, 1.5, "a note");
//! assert_eq!(note.position().x, ax.xaxis().convert_value(day(2)?));
//! assert_eq!(ax.xaxis().converter(), Some(UnitConverter::Dates));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Values of your own types plot the same way once a
//! [`ConversionInterface`] converts them and they are [`Convertible`]
//! (matplotlib's `units.registry`): lists of them are [`AxisValues`], one
//! of them is an [`AxisValue`]. An axis picks its [`Unit`] from the first
//! such values it converts, unless [`Axis::set_units`] chose one, and
//! takes its tick helpers, label and default view from the converter.
//! When its unit changes, the artists that matplotlib converts when
//! drawing convert their user values again: lines, texts, annotations and
//! rectangles of a [`DataRectangle`](crate::patches::DataRectangle); spans,
//! bars, fills, scatters and the bars of error bars keep the numbers they
//! were made with, as in matplotlib. An extent from a user value converts
//! as the far end less the start when [`Convertible::add_delta`] adds it
//! to the value, and by itself otherwise; error bars end at the values
//! less and plus their errors ([`Convertible::scale`]).
//! [`ConversionInterface`] shows a converter.
//!
//! [`Axis::set_units`]: crate::axes::Axis::set_units

mod category;
mod custom;
mod deltas;
mod interface;
mod items;
mod lists;
mod scalar;
mod sources;

use std::rc::Rc;

use crate::dates::{AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, TimeZone};
use crate::ticker::{Formatter, Locator};

pub use category::{Categories, CategoryFormatter, CategoryLocator};
pub use custom::{CustomConverter, CustomValues, Unit};
pub use deltas::{AxisDelta, AxisDeltas};
pub use interface::{ConversionError, ConversionInterface, Convertible, UnitAxisInfo};
pub use items::AxisItem;
pub(crate) use lists::from_value_lists;
pub use scalar::{AxisValue, DataPoint};
pub(crate) use sources::UnitSources;

/// The tick helpers of an axis of dates in `tz` (matplotlib's
/// `DateConverter.axisinfo`, or `ConciseDateConverter`'s when
/// `date.converter` says so): an `AutoDateLocator` following
/// `date.interval_multiples`, and an `AutoDateFormatter` or a
/// `ConciseDateFormatter`.
pub(crate) fn date_tickers(tz: &TimeZone) -> (Rc<dyn Locator>, Rc<dyn Formatter>) {
    let rc = crate::rc::params();
    let locator = || {
        AutoDateLocator::new()
            .with_interval_multiples(rc.date_interval_multiples)
            .with_tz(tz.clone())
    };
    let formatter: Rc<dyn Formatter> = if rc.date_converter == "concise" {
        Rc::new(ConciseDateFormatter::new().with_tz(tz.clone()))
    } else {
        Rc::new(AutoDateFormatter::new(locator()).with_tz(tz.clone()))
    };
    (Rc::new(locator()), formatter)
}

/// The unit converter of an axis (matplotlib's `ConversionInterface`s
/// registered for dates and strings, and those of users): what plotted
/// values were converted from, which decides the axis' default tick
/// helpers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnitConverter {
    /// Dates, as date numbers with date ticks (matplotlib's
    /// `DateConverter`, or `ConciseDateConverter` as `date.converter`
    /// says).
    Dates,
    /// Category names, placed at 0, 1, 2, ... (matplotlib's
    /// `StrCategoryConverter`).
    Categories,
    /// A converter of user values.
    Custom(CustomConverter),
}

impl UnitConverter {
    /// The converter of user values, if it is one.
    pub fn custom(&self) -> Option<&CustomConverter> {
        match self {
            Self::Custom(converter) => Some(converter),
            Self::Dates | Self::Categories => None,
        }
    }
}

impl<C: ConversionInterface> From<C> for UnitConverter {
    fn from(converter: C) -> Self {
        Self::Custom(CustomConverter::new(converter))
    }
}

/// Plotting values along one axis before unit conversion. Any collection
/// or iterator of [`AxisItem`]s converts into them: numbers of any
/// primitive type, names, dates or values of a [`Convertible`] type. One
/// value is a list of one (`[value]`).
#[derive(Clone, Debug, PartialEq)]
pub enum AxisValues {
    /// Plain numbers.
    Numbers(Vec<f64>),
    /// Category names.
    Categories(Vec<String>),
    /// Dates as date numbers (days since the [epoch](crate::dates::get_epoch)).
    Dates(Vec<f64>),
    /// Values of a user type, converted by its [`ConversionInterface`].
    Custom(CustomValues),
}

impl AxisValues {
    /// The number of values.
    pub fn len(&self) -> usize {
        match self {
            Self::Numbers(values) | Self::Dates(values) => values.len(),
            Self::Categories(names) => names.len(),
            Self::Custom(values) => values.len(),
        }
    }

    /// Whether there are no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The values as axis numbers where that needs no axis: numbers as
    /// they are and dates as date numbers. Names and user values need an
    /// axis to place them, and become NaN.
    pub(crate) fn into_unplaced_numbers(self) -> Vec<f64> {
        match self {
            Self::Numbers(numbers) | Self::Dates(numbers) => numbers,
            Self::Categories(_) | Self::Custom(_) => vec![f64::NAN; self.len()],
        }
    }

    /// Value `index`, if there is one.
    pub(crate) fn get(&self, index: usize) -> Option<AxisValue> {
        match self {
            Self::Numbers(values) => values.get(index).copied().map(AxisValue::Number),
            Self::Dates(values) => values.get(index).copied().map(AxisValue::Date),
            Self::Categories(names) => names.get(index).cloned().map(AxisValue::Category),
            Self::Custom(values) => values.get(index).map(AxisValue::Custom),
        }
    }

    /// The values of a user type, if they are.
    pub fn custom(&self) -> Option<&CustomValues> {
        match self {
            Self::Custom(values) => Some(values),
            Self::Numbers(_) | Self::Categories(_) | Self::Dates(_) => None,
        }
    }

    /// The first finite value, or the first one when none is
    /// (matplotlib's `cbook._safe_first_finite`); names and user values
    /// count as finite.
    pub(crate) fn first_finite(&self) -> Option<AxisValue> {
        match self {
            Self::Numbers(values) => first_finite(values).map(AxisValue::Number),
            Self::Dates(values) => first_finite(values).map(AxisValue::Date),
            Self::Categories(names) => names.first().cloned().map(AxisValue::Category),
            Self::Custom(values) => values.get(0).map(AxisValue::Custom),
        }
    }
}

/// The first finite number, or the first one when none is (matplotlib's
/// `cbook._safe_first_finite`).
pub(crate) fn first_finite(numbers: &[f64]) -> Option<f64> {
    numbers
        .iter()
        .copied()
        .find(|number| number.is_finite())
        .or_else(|| numbers.first().copied())
}

impl From<CustomValues> for AxisValues {
    fn from(values: CustomValues) -> Self {
        Self::Custom(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisName;
    use crate::dates::{DateTime, Datetime64};
    use crate::figure::Figure;

    #[test]
    fn the_first_finite_value_leads() {
        assert_eq!(first_finite(&[f64::NAN, 2.0]), Some(2.0));
        assert!(first_finite(&[f64::NAN]).is_some_and(f64::is_nan));
        assert_eq!(first_finite(&[]), None);
        let dates = AxisValues::Dates(vec![f64::INFINITY, 3.0]);
        assert_eq!(dates.first_finite(), Some(AxisValue::Date(3.0)));
        let names = AxisValues::from(["b", "a"]);
        assert_eq!(names.first_finite(), Some(AxisValue::from("b")));
        assert_eq!(AxisValues::Numbers(Vec::new()).first_finite(), None);
    }

    #[test]
    fn dates_become_date_numbers_on_a_date_axis() {
        let days = [
            DateTime::new(1970, 1, 2, 0, 0, 0, 0).unwrap(),
            DateTime::new(1970, 1, 3, 12, 0, 0, 0).unwrap(),
        ];
        assert_eq!(
            AxisValues::from(&days[..]),
            AxisValues::Dates(vec![1.0, 2.5])
        );
        let instants = [Datetime64::new(86_400, 0)];
        assert_eq!(
            AxisValues::from(&instants[..]),
            AxisValues::Dates(vec![1.0])
        );

        let ax = Figure::new().subplot();
        ax.plot(days.to_vec(), [1.0, 2.0]).unwrap();
        let ticks = ax.ticks(AxisName::X).unwrap();
        let labels: Vec<&str> = ticks.major.iter().map(|t| t.label.as_str()).collect();
        // matplotlib 3.11 labels this span the same way.
        assert_eq!(labels[..3], ["01-02 00", "01-02 04", "01-02 08"]);
        assert_eq!(labels.last(), Some(&"01-03 12"));
    }

    #[test]
    fn plotting_methods_place_dates_and_durations() {
        use crate::artist::ArtistHandle;
        use crate::axes::{BandAlign, ErrorbarOptions, FillBetweenOptions};
        use crate::dates::TimeDelta;
        use crate::lines::LineProps;
        use crate::text::AnnotateOptions;

        let day = |d| DateTime::new(1970, 1, d, 0, 0, 0, 0).unwrap();
        let ax = Figure::new().subplot();
        ax.set_xlim(day(1), day(11)).expect("finite limits");
        assert_eq!(ax.xlim(), (0.0, 10.0));
        assert_eq!(ax.xaxis().converter(), Some(UnitConverter::Dates));
        assert!(!ax.yaxis().have_units(), "numbers leave an axis alone");

        let line = ax.axvline(day(5), LineProps::default());
        assert_eq!(line.xdata(), [4.0, 4.0]);
        let note = ax.text(day(2), 0.5, "note");
        assert_eq!(note.position().x, 1.0);
        let arrow = ax
            .annotate("a", (day(3), 0.5), AnnotateOptions::default())
            .unwrap();
        assert_eq!(arrow.xy().x, 2.0);

        let span = ax.axvspan(day(6), day(8));
        assert_eq!((span.shape().xy.x, span.shape().width), (5.0, 2.0));
        let two_days = TimeDelta::new(2, 0, 0).unwrap();
        let band = ax.broken_barh(&[(day(4), two_days)], (0.0, 1.0), BandAlign::Bottom);
        let corners = band.paths()[0].vertices().to_vec();
        assert_eq!((corners[0].x, corners[2].x), (3.0, 5.0));

        let fill = ax
            .fill_between(
                vec![day(1), day(3)],
                [0.0],
                [1.0],
                FillBetweenOptions::default(),
            )
            .unwrap();
        assert!(
            fill.paths()[0]
                .vertices()
                .iter()
                .all(|p| (0.0..=2.0).contains(&p.x))
        );
        let segments = ax.hlines([0.5], vec![day(9)], vec![day(10)]).unwrap();
        assert_eq!(segments.paths()[0].vertices()[1].x, 9.0);
        let one_day = ErrorbarOptions {
            xerr: Some(TimeDelta::new(1, 0, 0).unwrap().into()),
            ..ErrorbarOptions::default()
        };
        let errors = ax.errorbar(vec![day(2)], [0.25], one_day).unwrap();
        let bar = &errors.barlinecols[0].paths()[0];
        assert_eq!((bar.vertices()[0].x, bar.vertices()[1].x), (0.0, 2.0));
        assert!(ax.texts().iter().any(|text| text.id() == note.id()));
    }
}
