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
//! Option fields hold axis numbers: convert dates for them with
//! [`Axis::convert_value`](crate::axes::Axis::convert_value) or
//! [`Axis::convert_units`](crate::axes::Axis::convert_units), which give
//! the axis its date ticks too, and durations with [`AxisDelta`].
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

mod category;
mod scalar;

use std::rc::Rc;

use crate::dates::{
    AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, DateTime, Datetime64, TimeZone,
    get_epoch,
};
use crate::ticker::{Formatter, Locator};

pub use category::{Categories, CategoryFormatter, CategoryLocator};
pub use scalar::{AxisDelta, AxisValue, DataPoint};

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

/// Extents along an axis re-expressed through the axis origin, the first
/// finite position (or the first position when none is finite):
/// `(origin + delta) - origin`. This is matplotlib's `_convert_dx` for
/// numbers, which rounds the extents of bars and error bars; without
/// positions the deltas are kept.
///
/// ```
/// use plt::units::convert_dx;
/// // 1e16 + 1 rounds back to 1e16.
/// assert_eq!(convert_dx(&[f64::NAN, 1e16], &[1.0]), [0.0]);
/// assert_eq!(convert_dx(&[], &[0.1]), [0.1]);
/// ```
pub fn convert_dx(positions: &[f64], deltas: &[f64]) -> Vec<f64> {
    let origin = positions
        .iter()
        .copied()
        .find(|p| p.is_finite())
        .or_else(|| positions.first().copied());
    match origin {
        Some(origin) => deltas.iter().map(|&d| (origin + d) - origin).collect(),
        None => deltas.to_vec(),
    }
}

/// The unit converter of an axis (matplotlib's `ConversionInterface`s
/// registered for dates and strings): what plotted values were converted
/// from, which decides the axis' default tick helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitConverter {
    /// Dates, as date numbers with date ticks (matplotlib's
    /// `DateConverter`, or `ConciseDateConverter` as `date.converter`
    /// says).
    Dates,
    /// Category names, placed at 0, 1, 2, ... (matplotlib's
    /// `StrCategoryConverter`).
    Categories,
}

/// Plotting values along one axis before unit conversion.
#[derive(Clone, Debug, PartialEq)]
pub enum AxisValues {
    /// Plain numbers.
    Numbers(Vec<f64>),
    /// Category names.
    Categories(Vec<String>),
    /// Dates as date numbers (days since the [epoch](crate::dates::get_epoch)).
    Dates(Vec<f64>),
}

impl AxisValues {
    /// The number of values.
    pub fn len(&self) -> usize {
        match self {
            Self::Numbers(values) | Self::Dates(values) => values.len(),
            Self::Categories(names) => names.len(),
        }
    }

    /// Whether there are no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<&[DateTime]> for AxisValues {
    /// Naive date-times are taken as UTC, as matplotlib does by default.
    fn from(dates: &[DateTime]) -> Self {
        let epoch = get_epoch();
        Self::Dates(dates.iter().map(|&d| epoch.date2num(d)).collect())
    }
}

impl From<Vec<DateTime>> for AxisValues {
    fn from(dates: Vec<DateTime>) -> Self {
        Self::from(dates.as_slice())
    }
}

impl From<&[Datetime64]> for AxisValues {
    fn from(dates: &[Datetime64]) -> Self {
        let epoch = get_epoch();
        Self::Dates(dates.iter().map(|&d| epoch.datetime64_to_num(d)).collect())
    }
}

impl From<Vec<Datetime64>> for AxisValues {
    fn from(dates: Vec<Datetime64>) -> Self {
        Self::from(dates.as_slice())
    }
}

impl From<Vec<f64>> for AxisValues {
    fn from(values: Vec<f64>) -> Self {
        Self::Numbers(values)
    }
}

impl From<&Vec<f64>> for AxisValues {
    fn from(values: &Vec<f64>) -> Self {
        Self::Numbers(values.clone())
    }
}

impl From<&[f64]> for AxisValues {
    fn from(values: &[f64]) -> Self {
        Self::Numbers(values.to_vec())
    }
}

impl<const N: usize> From<[f64; N]> for AxisValues {
    fn from(values: [f64; N]) -> Self {
        Self::Numbers(values.to_vec())
    }
}

impl<const N: usize> From<&[f64; N]> for AxisValues {
    fn from(values: &[f64; N]) -> Self {
        Self::Numbers(values.to_vec())
    }
}

impl From<Vec<String>> for AxisValues {
    fn from(names: Vec<String>) -> Self {
        Self::Categories(names)
    }
}

impl From<&[String]> for AxisValues {
    fn from(names: &[String]) -> Self {
        Self::Categories(names.to_vec())
    }
}

impl From<Vec<&str>> for AxisValues {
    fn from(names: Vec<&str>) -> Self {
        names.as_slice().into()
    }
}

impl From<&[&str]> for AxisValues {
    fn from(names: &[&str]) -> Self {
        Self::Categories(names.iter().map(|&name| name.to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for AxisValues {
    fn from(names: [&str; N]) -> Self {
        names.as_slice().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisName;
    use crate::figure::Figure;

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
