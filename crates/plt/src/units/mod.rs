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
//! 3-D axes take plotting values the same way: their point-plotting
//! methods ([`Axes3D::plot`], `scatter`, `text`, `bar3d`, ...) convert
//! each list on the axis it lies along ([`Axis3D::convert_units`]),
//! converters get that axis as a [`UnitAxis`], and the lines and texts
//! of 3-D axes convert their user values again when a unit changes.
//!
//! [`Axis::set_units`]: crate::axes::Axis::set_units
//! [`Axes3D::plot`]: crate::mplot3d::Axes3D::plot
//! [`Axis3D::convert_units`]: crate::mplot3d::Axis3D::convert_units

mod axis;
mod category;
mod converting;
mod custom;
mod date_converter;
mod deltas;
mod interface;
mod items;
mod lists;
mod scalar;
mod sources;

use std::rc::Rc;

use crate::dates::{AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, TimeZone};
use crate::ticker::{Formatter, Locator};

pub use axis::UnitAxis;
pub use category::{Categories, CategoryFormatter, CategoryLocator};
pub(crate) use converting::{AxisUnits, UnitData, UnitHandle, UnitState};
pub use custom::{CustomConverter, CustomValues, Unit};
pub use date_converter::DateConverter;
pub use deltas::{AxisDelta, AxisDeltas};
pub use interface::{ConversionError, ConversionInterface, Convertible, UnitAxisInfo};
pub use items::AxisItem;
pub(crate) use lists::from_value_lists;
pub use scalar::{AxisValue, DataPoint};
pub(crate) use sources::UnitSources;

/// The tick helpers of an axis of dates in `tz` that `converter` gives,
/// if it converts dates (matplotlib's `DateConverter.axisinfo` and
/// `ConciseDateConverter.axisinfo`): an `AutoDateLocator` and an
/// `AutoDateFormatter` or a `ConciseDateFormatter`, as the kind of the
/// converter says. The switchable converter takes its kind from
/// `date.converter` and its locator's `interval_multiples` from
/// `date.interval_multiples` now (matplotlib's `_SwitchableDateConverter`,
/// which makes a converter of either kind from them); a converter of a
/// given kind ticks at interval multiples, the default of matplotlib's
/// constructors. Without `tz` (the axis unit is no zone) they take the
/// `timezone` rcParam as matplotlib's do given `tz=None`: the locator
/// now, the formatter whenever it formats.
pub(crate) fn date_tickers(
    converter: &UnitConverter,
    tz: Option<&TimeZone>,
) -> Option<(Rc<dyn Locator>, Rc<dyn Formatter>)> {
    let (kind, interval_multiples) = match *converter {
        UnitConverter::Dates => {
            let rc = crate::rc::params();
            (rc.date_converter, rc.date_interval_multiples)
        }
        UnitConverter::DatesBy(kind) => (kind, true),
        UnitConverter::Categories | UnitConverter::Custom(_) => return None,
    };
    let locator = || {
        let locator = AutoDateLocator::new().with_interval_multiples(interval_multiples);
        with_optional_tz(locator, tz, AutoDateLocator::with_tz)
    };
    let formatter: Rc<dyn Formatter> = match kind {
        DateConverter::Concise => Rc::new(with_optional_tz(
            ConciseDateFormatter::new(),
            tz,
            ConciseDateFormatter::with_tz,
        )),
        DateConverter::Auto => Rc::new(with_optional_tz(
            AutoDateFormatter::new(locator()),
            tz,
            AutoDateFormatter::with_tz,
        )),
    };
    Some((Rc::new(locator()), formatter))
}

/// `ticker` given `tz` by `with_tz` if there is one.
fn with_optional_tz<T>(ticker: T, tz: Option<&TimeZone>, with_tz: fn(T, TimeZone) -> T) -> T {
    match tz {
        Some(tz) => with_tz(ticker, tz.clone()),
        None => ticker,
    }
}

/// The unit converter of an axis (matplotlib's `ConversionInterface`s
/// registered for dates and strings, and those of users): what plotted
/// values were converted from, which decides the axis' default tick
/// helpers.
///
/// Dates have two: the one plotting dates gives an axis, whose kind of
/// ticks follows `date.converter`, and one of a kind of its own, which an
/// axis is given with [`Axis::set_converter`] and which converts from
/// that kind ([`DateConverter`]). They compare unequal, as converters of
/// different kinds do.
///
/// ```
/// use plt::dates::TimeZone;
/// use plt::prelude::*;
/// use plt::units::{DateConverter, UnitConverter};
/// let ax = Figure::new().subplot();
/// ax.xaxis().set_converter(DateConverter::Concise)?;
/// ax.xaxis_date(&TimeZone::Utc);
/// let converter = ax.xaxis().converter().expect("a converter");
/// assert_eq!(converter, UnitConverter::DatesBy(DateConverter::Concise));
/// assert!(converter.is_dates() && converter != UnitConverter::Dates);
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
///
/// [`Axis::set_converter`]: crate::axes::Axis::set_converter
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnitConverter {
    /// Dates, as date numbers with date ticks of the kind `date.converter`
    /// names whenever the axis takes them (matplotlib's
    /// `_SwitchableDateConverter`, the converter of dates in its
    /// `units.registry`): the converter plotting dates gives an axis.
    Dates,
    /// Dates, as date numbers with date ticks of one kind whatever
    /// `date.converter` says (matplotlib's `DateConverter()` or
    /// `ConciseDateConverter()`, given to `set_converter`).
    DatesBy(DateConverter),
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
            Self::Dates | Self::DatesBy(_) | Self::Categories => None,
        }
    }

    /// Whether the converter is one of dates, of the kind `date.converter`
    /// names or of a kind of its own.
    pub fn is_dates(&self) -> bool {
        matches!(self, Self::Dates | Self::DatesBy(_))
    }
}

impl From<DateConverter> for UnitConverter {
    fn from(kind: DateConverter) -> Self {
        Self::DatesBy(kind)
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
    fn date_ticks_are_in_the_timezone_rc_param_unless_the_unit_is_a_zone() {
        use crate::dates::TimeZone;

        if TimeZone::named("America/New_York").is_err() {
            return; // no system zoneinfo database
        }
        let labels = |converter: &str, unit: Option<TimeZone>| {
            crate::rc::context(|| {
                crate::rc::set("timezone", "America/New_York").unwrap();
                crate::rc::set("date.converter", converter).unwrap();
                let ax = Figure::new().subplot();
                if let Some(zone) = unit {
                    ax.xaxis().set_units(zone);
                }
                // Across 07:00 UTC, when New York's clocks skip from 02:00
                // to 03:00 (the rule's 02:00 and 02:30 land an hour early).
                let hour = |h| DateTime::new(2020, 3, 8, h, 0, 0, 0).unwrap();
                ax.plot(vec![hour(6), hour(9)], [0.0, 1.0]).unwrap();
                ax.xaxis().ticklabels(false).unwrap()
            })
        };
        // matplotlib 3.11 labels these the same way.
        assert_eq!(
            labels("auto", None),
            [
                "08 01:00", "08 01:30", "08 01:00", "08 01:30", "08 03:00", "08 03:30", "08 04:00",
                "08 04:30", "08 05:00"
            ]
        );
        assert_eq!(
            labels("concise", None),
            [
                "01:00", "01:30", "01:00", "01:30", "03:00", "03:30", "04:00", "04:30", "05:00"
            ]
        );
        assert_eq!(
            labels("auto", Some(TimeZone::Utc)),
            [
                "08 06:00", "08 06:30", "08 07:00", "08 07:30", "08 08:00", "08 08:30", "08 09:00"
            ]
        );
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
