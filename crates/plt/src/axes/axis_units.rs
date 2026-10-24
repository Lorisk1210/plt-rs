//! Unit conversion on an axis: turning plotted dates and category names
//! into numbers, and the converter that remembers which kind of values
//! the axis shows (matplotlib's `Axis.convert_units`, `update_units`,
//! `get_converter`, `set_converter`, `have_units` and `axis_date`, and
//! `Axes.convert_xunits` and `convert_yunits`).

use plt_core::Point;

use super::axis::AxisState;
use super::axis_handle::Axis;
use super::{Axes, AxesState, PlotError};
use crate::artist::ArtistHandle;
use crate::dates::{Datetime64, TimeZone, get_epoch};
use crate::ticker::AxisName;
use crate::units::{AxisValue, AxisValues, Categories, DataPoint, UnitConverter};

/// Seconds in a day: the length of the default view of a date axis.
const SECONDS_PER_DAY: i64 = 86_400;

/// The unit converter of an axis and how it was chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AxisUnits {
    /// The converter, once values with units were converted or one was
    /// set (matplotlib's `_converter`).
    pub(crate) converter: Option<UnitConverter>,
    /// Whether the converter was set explicitly, so that converted values
    /// do not replace it (matplotlib's `_converter_is_explicit`).
    pub(crate) explicit: bool,
}

impl AxisUnits {
    /// Takes `converter` unless another one was set explicitly
    /// (matplotlib's `_set_converter` from `update_units`).
    pub(crate) fn adopt(&mut self, converter: UnitConverter) {
        if !self.explicit {
            self.converter = Some(converter);
        }
    }
}

impl Axis {
    /// Converts plotting values to axis numbers, registering new category
    /// names (matplotlib's `update_units` + `convert_units`). The axes
    /// sharing this axis take over its converter, category names and the
    /// tick helpers they bring, as matplotlib's shared axes share their
    /// units and tickers.
    ///
    /// ```
    /// use plt::figure::{Figure, Share};
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(1, 2, Share::All, Share::None)?;
    /// axs[(0, 0)].plot(vec!["a", "b"], [1.0, 2.0])?;
    /// let line = axs[(0, 1)].plot(vec!["c", "a"], [3.0, 4.0])?;
    /// assert_eq!(line.xdata(), [2.0, 0.0]);
    /// let names = axs[(0, 0)].xaxis().categories().expect("categories");
    /// assert_eq!(names.names(), ["a", "b", "c"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn convert_units(&self, values: impl Into<AxisValues>) -> Vec<f64> {
        match values.into() {
            // Numbers need no converter, so the axis is left untouched.
            AxisValues::Numbers(numbers) => numbers,
            values => {
                let positions = self.update(|axis| axis.convert_units(values));
                let units = self.read(AxisState::unit_data);
                self.update_shared(|axis| axis.share_units(&units));
                positions
            }
        }
    }

    /// Converts one plotting value to an axis number, as
    /// [`convert_units`](Self::convert_units) converts a list: how option
    /// fields, which hold axis numbers, take dates.
    ///
    /// ```
    /// use plt::dates::DateTime;
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let day = DateTime::new(1970, 1, 3, 0, 0, 0, 0)?;
    /// assert_eq!(ax.yaxis().convert_value(day), 2.0);
    /// assert!(ax.yaxis().have_units());
    /// # Ok::<(), plt::dates::DateError>(())
    /// ```
    pub fn convert_value(&self, value: impl Into<AxisValue>) -> f64 {
        self.convert_units(AxisValues::from(value.into()))
            .into_iter()
            .next()
            .expect("one value converts to one number")
    }

    /// The unit converter: the one set with
    /// [`set_converter`](Self::set_converter), else that of the dates or
    /// category names plotted on the axis, if any (matplotlib's
    /// `get_converter`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::units::UnitConverter;
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.xaxis().converter(), None);
    /// ax.bar(["apple", "banana"], [4.0, 6.0], Default::default())?;
    /// assert_eq!(ax.xaxis().converter(), Some(UnitConverter::Categories));
    /// assert!(ax.xaxis().have_units() && !ax.yaxis().have_units());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn converter(&self) -> Option<UnitConverter> {
        self.read(|axis| axis.units.converter)
    }

    /// Sets the unit converter explicitly (matplotlib's `set_converter`):
    /// values plotted later no longer change it, though each is still
    /// converted as its own kind. Like matplotlib, this leaves the tick
    /// helpers alone; [`axis_date`](Self::axis_date) installs those of
    /// dates.
    ///
    /// # Errors
    ///
    /// When another converter was already set explicitly.
    pub fn set_converter(&self, converter: UnitConverter) -> Result<&Self, PlotError> {
        self.update(|axis| {
            let units = &mut axis.units;
            if units.explicit && units.converter != Some(converter) {
                return Err(PlotError::ExplicitConverter(axis.name));
            }
            units.converter = Some(converter);
            units.explicit = true;
            Ok(())
        })?;
        Ok(self)
    }

    /// The category names plotted on the axis, in position order, if any
    /// were (the mapping of matplotlib's `axis.units` on a categorical
    /// axis, a `UnitData`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot(vec!["b", "a", "b"], [1.0, 2.0, 3.0])?;
    /// let categories = ax.xaxis().categories().expect("a categorical axis");
    /// assert_eq!(categories.names(), ["b", "a"]);
    /// assert!(ax.yaxis().categories().is_none());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn categories(&self) -> Option<Categories> {
        self.read(|axis| axis.categories().cloned())
    }

    /// Whether the axis has a unit converter (matplotlib's `have_units`).
    pub fn have_units(&self) -> bool {
        self.converter().is_some()
    }

    /// Ticks the axis as an axis of dates in `tz` without plotting any
    /// (matplotlib's `axis_date`, as `Axis3D::axis_date` does for 3-D
    /// axes): the axis takes the date converter, unless another one was
    /// set explicitly, and an `AutoDateLocator` with the formatter
    /// `date.converter` chooses in place of the major locator and
    /// formatter that were not set explicitly; the axes sharing the axis
    /// follow. Before any data is plotted or limits are set, the view
    /// becomes the first day of 1970, the default limits of matplotlib's
    /// date converter (its `set_default_intervals`).
    ///
    /// ```
    /// use plt::dates::TimeZone;
    /// use plt::prelude::*;
    /// use plt::units::UnitConverter;
    /// let ax = Figure::new().subplot();
    /// ax.plot([19_000.0, 19_010.0], [0.0, 1.0])?;
    /// ax.xaxis_date(&TimeZone::Utc);
    /// assert_eq!(ax.xaxis().converter(), Some(UnitConverter::Dates));
    /// assert_eq!(ax.xaxis().ticklabels(false)?[0], "2022-01-09");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn axis_date(&self, tz: &TimeZone) -> &Self {
        let dated = self.update(|axis| {
            axis.units.adopt(UnitConverter::Dates);
            axis.units.converter == Some(UnitConverter::Dates)
        });
        if dated {
            self.update_shared(|axis| axis.use_date_tickers(tz));
            let name = self.name;
            self.axes
                .raw()
                .with_mut::<AxesState, _>(|axes| set_default_date_view(axes, name));
        }
        self
    }
}

/// matplotlib's `set_default_intervals` for dates: the view of axis
/// `name` becomes 1970-01-01 to 1970-01-02 when neither its data nor its
/// view limits changed yet.
fn set_default_date_view(axes: &mut AxesState, name: AxisName) {
    let ((low, high), _) = axes.data_interval(name);
    if low.is_finite() || high.is_finite() || axes.view_mutated(name) {
        return;
    }
    let epoch = get_epoch();
    let start = epoch.datetime64_to_num(Datetime64::new(0, 0));
    let end = epoch.datetime64_to_num(Datetime64::new(SECONDS_PER_DAY, 0));
    axes.set_view_interval(name, (start, end));
}

impl Axes {
    /// Ticks the x axis as an axis of dates in `tz` (matplotlib's
    /// `xaxis_date`; see [`Axis::axis_date`]).
    pub fn xaxis_date(&self, tz: &TimeZone) -> &Self {
        self.xaxis().axis_date(tz);
        self
    }

    /// Ticks the y axis as an axis of dates in `tz` (matplotlib's
    /// `yaxis_date`; see [`Axis::axis_date`]).
    pub fn yaxis_date(&self, tz: &TimeZone) -> &Self {
        self.yaxis().axis_date(tz);
        self
    }

    /// Converts a data point to axis numbers, its x on the x axis and its
    /// y on the y axis (matplotlib's `convert_xunits` and
    /// `convert_yunits`).
    ///
    /// ```
    /// use plt::dates::DateTime;
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let p = ax.convert_point((DateTime::new(1970, 1, 2, 0, 0, 0, 0)?, 3.0));
    /// assert_eq!((p.x, p.y), (1.0, 3.0));
    /// # Ok::<(), plt::dates::DateError>(())
    /// ```
    pub fn convert_point(&self, point: impl Into<DataPoint>) -> Point {
        let DataPoint { x, y } = point.into();
        Point::new(self.xaxis().convert_value(x), self.yaxis().convert_value(y))
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;

    use super::*;
    use crate::dates::{AutoDateFormatter, AutoDateLocator};
    use crate::figure::{Figure, Share};
    use crate::ticker::MultipleLocator;

    #[test]
    fn axis_date_installs_date_tickers_where_defaults_remain() {
        let ax = Figure::new().subplot();
        ax.yaxis()
            .set_major_locator(MultipleLocator::new(0.5).expect("a positive base"));
        ax.xaxis_date(&TimeZone::Utc).yaxis_date(&TimeZone::Utc);
        let is = |axis: &Axis| {
            (
                (axis.major_locator().as_ref() as &dyn Any).is::<AutoDateLocator>(),
                (axis.major_formatter().as_ref() as &dyn Any).is::<AutoDateFormatter>(),
            )
        };
        assert_eq!(is(&ax.xaxis()), (true, true));
        assert_eq!(is(&ax.yaxis()), (false, true), "the chosen locator stays");
        assert!(ax.xaxis().have_units() && ax.yaxis().have_units());
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 1.0), (0.0, 1.0)));
    }

    #[test]
    fn the_default_date_view_leaves_data_and_set_limits_alone() {
        let ax = Figure::new().subplot();
        ax.set_ylim(2.0, 3.0).expect("finite limits");
        ax.plot([5.0, 6.0], [2.5, 2.5]).expect("a line");
        ax.xaxis_date(&TimeZone::Utc).yaxis_date(&TimeZone::Utc);
        assert_eq!(ax.xlim(), (4.95, 6.05), "the data's view");
        assert_eq!(ax.ylim(), (2.0, 3.0), "set limits stay");
    }

    #[test]
    fn axes_sharing_a_date_axis_follow() {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(2, 1, Share::All, Share::None)
            .expect("a grid");
        axs[(0, 0)].xaxis_date(&TimeZone::Utc);
        let below = axs[(1, 0)].xaxis().major_locator();
        assert!((below.as_ref() as &dyn Any).is::<AutoDateLocator>());
    }

    #[test]
    fn explicit_converters_are_kept() {
        let ax = Figure::new().subplot();
        let xaxis = ax.xaxis();
        xaxis
            .set_converter(UnitConverter::Categories)
            .expect("no converter yet");
        assert!(xaxis.set_converter(UnitConverter::Categories).is_ok());
        assert_eq!(
            xaxis.set_converter(UnitConverter::Dates).err(),
            Some(PlotError::ExplicitConverter(AxisName::X))
        );
        xaxis.axis_date(&TimeZone::Utc);
        assert_eq!(xaxis.converter(), Some(UnitConverter::Categories));
        assert!(
            !(xaxis.major_locator().as_ref() as &dyn Any).is::<AutoDateLocator>(),
            "matplotlib asks the explicit converter for tick helpers"
        );
        assert_eq!(xaxis.convert_units(vec![3.0]), [3.0]);
        let dates = xaxis.convert_units(AxisValues::Dates(vec![10.0]));
        assert_eq!(dates, [10.0], "values convert as their own kind");
        assert_eq!(xaxis.converter(), Some(UnitConverter::Categories));
    }

    #[test]
    fn shared_axes_take_over_converted_units() {
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 1, Share::All, Share::None).unwrap();
        let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
        top.xaxis()
            .convert_units(AxisValues::Dates(vec![0.0, 30.0]));
        top.set_xlim(0.0, 30.0).expect("finite limits");
        assert_eq!(bottom.xaxis().converter(), Some(UnitConverter::Dates));
        let ticks = bottom.ticks(AxisName::X).unwrap();
        assert!(ticks.major[0].label.starts_with("1970-"), "date ticks");
        assert!(!bottom.yaxis().have_units(), "only the shared axis");
    }
}
