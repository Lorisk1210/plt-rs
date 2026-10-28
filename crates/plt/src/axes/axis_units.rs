//! Unit conversion on an axis: turning plotted dates and category names
//! into numbers, and the converter that remembers which kind of values
//! the axis shows (matplotlib's `Axis.convert_units`, `update_units`,
//! `get_converter`, `set_converter`, `have_units` and `axis_date`, and
//! `Axes.convert_xunits` and `convert_yunits`). The conversions are those
//! [`UnitHandle`] shares with 3-D axes; this module gives the 2-D axis
//! its part of them.

use plt_core::Point;

use super::axis::AxisState;
use super::axis_handle::Axis;
use super::{Axes, AxesState, PlotError};
use crate::artist::{ArtistHandle, RawHandle};
use crate::dates::TimeZone;
use crate::units::{
    AxisValue, AxisValues, Categories, DataPoint, UnitAxis, UnitConverter, UnitHandle, UnitState,
};

impl UnitHandle for Axis {
    type State = AxisState;

    fn unit_axis(&self) -> UnitAxis {
        UnitAxis::Axes(self.clone())
    }

    fn axes_raw(&self) -> &RawHandle {
        self.axes.raw()
    }

    fn read_state<R>(&self, f: impl FnOnce(&AxisState) -> R) -> R {
        self.read(f)
    }

    fn update_state<R>(&self, f: impl FnOnce(&mut AxisState) -> R) -> R {
        self.update(f)
    }

    fn update_shared_states(&self, f: impl FnMut(&mut AxisState)) {
        self.update_shared(f);
    }

    fn sharing(&self) -> Vec<Self> {
        (self.axes.shared_axes(self.name).iter())
            .map(|axes| axes.axis_of(self.name))
            .collect()
    }

    fn units_changed(&self) {
        self.axes.units_changed(self.name);
    }

    fn set_default_intervals(&self, limits: (f64, f64)) {
        let name = self.name;
        self.axes.raw().with_mut::<AxesState, _>(|axes| {
            let ((low, high), _) = axes.data_interval(name);
            if !(low.is_finite() || high.is_finite() || axes.view_mutated(name)) {
                axes.set_view_interval(name, limits);
            }
        });
    }
}

impl Axis {
    /// Converts plotting values to axis numbers, registering new category
    /// names (matplotlib's `update_units` + `convert_units`). The axes
    /// sharing this axis take over its converter, category names and the
    /// tick helpers they bring, as matplotlib's shared axes share their
    /// units and tickers.
    ///
    /// Values of user types convert through their
    /// [`ConversionInterface`](crate::units::ConversionInterface), the axis
    /// picking its unit, tick helpers and label from them first if it has
    /// no unit (matplotlib's `_process_unit_info`); values that fail to
    /// convert become NaN, and the figure reports why when it next draws.
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
        self.convert_values(values.into())
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
        self.convert_one(value.into())
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
        self.current_converter()
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
    pub fn set_converter(&self, converter: impl Into<UnitConverter>) -> Result<&Self, PlotError> {
        let converter = converter.into();
        if self.update(|axis| axis.units_mut().set_explicit(converter)) {
            Ok(self)
        } else {
            Err(PlotError::ExplicitConverter(self.name))
        }
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

    /// Whether the axis has a unit converter or a unit (matplotlib's
    /// `have_units`).
    pub fn have_units(&self) -> bool {
        self.has_units()
    }

    /// Ticks the axis as an axis of dates without plotting any
    /// (matplotlib's `axis_date`, as `Axis3D::axis_date` does for 3-D
    /// axes): the axis takes the date converter, unless another one was
    /// set explicitly, and an `AutoDateLocator` with the formatter
    /// `date.converter` chooses in place of the major locator and
    /// formatter that were not set explicitly; the axes sharing the axis
    /// follow. A zone `tz` becomes the axis unit if it has none, so later
    /// plots keep it; the ticks are in the unit's zone, else (`None`) in
    /// the `timezone` rcParam's. Before any data is plotted or limits are
    /// set, the view becomes the first day of 1970, the default limits of
    /// matplotlib's date converter (its `set_default_intervals`).
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
    pub fn axis_date<'a>(&self, tz: impl Into<Option<&'a TimeZone>>) -> &Self {
        self.make_date_axis(tz.into());
        self
    }
}

impl Axes {
    /// Ticks the x axis as an axis of dates (matplotlib's
    /// `xaxis_date`; see [`Axis::axis_date`]).
    pub fn xaxis_date<'a>(&self, tz: impl Into<Option<&'a TimeZone>>) -> &Self {
        self.xaxis().axis_date(tz);
        self
    }

    /// Ticks the y axis as an axis of dates (matplotlib's
    /// `yaxis_date`; see [`Axis::axis_date`]).
    pub fn yaxis_date<'a>(&self, tz: impl Into<Option<&'a TimeZone>>) -> &Self {
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
    use crate::ticker::{AxisName, MultipleLocator};

    #[test]
    fn axis_date_installs_date_tickers_where_defaults_remain() {
        let ax = Figure::new().subplot();
        ax.yaxis()
            .set_major_locator(MultipleLocator::new(0.5).expect("a positive base"));
        ax.xaxis_date(None).yaxis_date(None);
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
        // A zone becomes the unit, and the unit change autoscales, as in
        // matplotlib (`xaxis_date("UTC")`).
        ax.xaxis_date(&TimeZone::Utc);
        assert_eq!(ax.xlim(), (-0.05, 1.05));
    }

    /// matplotlib's `axis_date(tz)` updates the units with a date in `tz`:
    /// the zone becomes the unit, which later plots keep, while no zone
    /// leaves the ticks in the `timezone` rcParam's.
    #[test]
    fn axis_date_keeps_its_zone_for_later_plots() {
        crate::rc::context(|| {
            crate::rc::set("timezone", "Europe/Berlin").expect("a valid rcParam");
            let Ok(berlin) = TimeZone::from_rc() else {
                return; // no zoneinfo on this system
            };
            let at = |hour| crate::dates::DateTime::new(2020, 1, 1, hour, 0, 0, 0).expect("a date");
            let labels = |tz: Option<&TimeZone>| {
                let ax = Figure::new().subplot();
                ax.xaxis_date(tz);
                ax.plot(vec![at(12), at(18)], [0.0, 1.0]).expect("a line");
                let unit = ax.xaxis().units();
                assert_eq!(unit.as_ref().and_then(|u| u.downcast_ref::<TimeZone>()), tz);
                ax.xaxis().ticklabels(false).expect("ticks")
            };
            // matplotlib: "01-01 12" ... in UTC, "01-01 13" ... in Berlin.
            assert_eq!(labels(Some(&TimeZone::Utc))[0], "01-01 12");
            assert_eq!(labels(None)[0], "01-01 13");
            assert_eq!(labels(None), labels(Some(&berlin)));
        });
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
