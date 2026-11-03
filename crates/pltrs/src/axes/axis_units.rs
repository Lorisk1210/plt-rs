//! Unit conversion on an axis: turning plotted dates and category names
//! into numbers, and the converter that remembers which kind of values
//! the axis shows (matplotlib's `Axis.convert_units`, `update_units`,
//! `get_converter`, `set_converter`, `have_units` and `axis_date`, and
//! `Axes.convert_xunits` and `convert_yunits`). The conversions are those
//! [`UnitHandle`] shares with 3-D axes; this module gives the 2-D axis
//! its part of them.

use pltrs_core::Point;

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

    /// Each axis sharing the axis, in turn, takes the category ticks
    /// (matplotlib's `_update_axisinfo`) and follows the change of unit
    /// (`_unit_change_handler`, which recomputes the data limits and asks
    /// for autoscaling); then the axis takes its ticks again, as
    /// `update_units` updates the axis info of a new converter once more.
    /// Taking the ticks reads the view limits, bringing the view up to
    /// date, while the axes has no data in the axis' direction (what
    /// `set_default_intervals` checks first).
    fn set_up_categories(&self) {
        for axis in self.sharing() {
            axis.default_intervals_read();
            axis.update(|state| {
                state.units_mut().adopt(UnitConverter::Categories);
                state
                    .categories_mut()
                    .get_or_insert_with(Categories::default);
                state.use_category_tickers();
            });
            axis.units_changed();
        }
        self.default_intervals_read();
    }
}

impl Axis {
    /// Brings the view up to date while no data was added to the axes
    /// in the axis' direction, as matplotlib's `set_default_intervals`
    /// reads the view limits only then (after `dataLim.mutatedx()`).
    fn default_intervals_read(&self) {
        let name = self.name;
        let has_data = self.axes.raw().with::<AxesState, _>(|axes| {
            let ((low, high), _) = axes.data_interval(name);
            low.is_finite() || high.is_finite()
        });
        if !has_data {
            self.axes.unstale_view();
        }
    }

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
    /// use pltrs::figure::{Figure, Share};
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
    /// use pltrs::dates::DateTime;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let day = DateTime::new(1970, 1, 3, 0, 0, 0, 0)?;
    /// assert_eq!(ax.yaxis().convert_value(day), 2.0);
    /// assert!(ax.yaxis().have_units());
    /// # Ok::<(), pltrs::dates::DateError>(())
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
    /// use pltrs::prelude::*;
    /// use pltrs::units::UnitConverter;
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.xaxis().converter(), None);
    /// ax.bar(["apple", "banana"], [4.0, 6.0], Default::default())?;
    /// assert_eq!(ax.xaxis().converter(), Some(UnitConverter::Categories));
    /// assert!(ax.xaxis().have_units() && !ax.yaxis().have_units());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn converter(&self) -> Option<UnitConverter> {
        self.current_converter()
    }

    /// Sets the unit converter explicitly (matplotlib's `set_converter`):
    /// values plotted later no longer change it, though each is still
    /// converted as its own kind. Like matplotlib, this leaves the tick
    /// helpers alone, and so do values plotted later;
    /// [`axis_date`](Self::axis_date) with a zone installs those of
    /// dates that the converter gives.
    ///
    /// A [`DateConverter`] gives an axis dates of that kind
    /// ([`UnitConverter::DatesBy`], matplotlib's `DateConverter()` or
    /// `ConciseDateConverter()`), whose ticks are of that kind whatever
    /// `date.converter` says, while [`UnitConverter::Dates`] is the
    /// converter plotting dates gives, whose kind follows the rcParam.
    ///
    /// ```
    /// use std::any::Any;
    /// use pltrs::dates::{ConciseDateFormatter, TimeZone};
    /// use pltrs::prelude::*;
    /// use pltrs::units::DateConverter;
    /// let ax = Figure::new().subplot();
    /// // matplotlib: ax.xaxis.set_converter(mdates.ConciseDateConverter())
    /// ax.xaxis().set_converter(DateConverter::Concise)?;
    /// ax.xaxis_date(&TimeZone::Utc); // date.converter is "auto"
    /// let formatter = ax.xaxis().major_formatter();
    /// assert!((formatter.as_ref() as &dyn Any).is::<ConciseDateFormatter>());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When another converter was already set explicitly; converters of
    /// dates of different kinds are different converters.
    ///
    /// [`DateConverter`]: crate::units::DateConverter
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
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot(vec!["b", "a", "b"], [1.0, 2.0, 3.0])?;
    /// let categories = ax.xaxis().categories().expect("a categorical axis");
    /// assert_eq!(categories.names(), ["b", "a"]);
    /// assert!(ax.yaxis().categories().is_none());
    /// # Ok::<(), pltrs::axes::PlotError>(())
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
    /// axes): the axis takes the converter plotting dates gives
    /// ([`UnitConverter::Dates`]), unless another one was set explicitly,
    /// and an `AutoDateLocator` with the formatter of its converter in
    /// place of the major locator and formatter that were not set
    /// explicitly: the one `date.converter` chooses, or that of the kind
    /// a converter set with [`set_converter`](Self::set_converter) has
    /// ([`UnitConverter::DatesBy`]). The axes sharing the axis follow. A
    /// zone `tz` becomes the axis unit if it has none, so later plots
    /// keep it; the ticks are in the unit's zone, else (`None`) in the
    /// `timezone` rcParam's. As in matplotlib (`update_units`), the date
    /// ticks come with that new unit or with a converter new to the axis;
    /// with neither, the axis keeps its tick helpers, so an explicit
    /// converter of dates brings its ticks only with a zone. Before any
    /// data is plotted or limits are set, the view becomes the first day
    /// of 1970, the default limits of matplotlib's date converter (its
    /// `set_default_intervals`).
    ///
    /// ```
    /// use pltrs::dates::TimeZone;
    /// use pltrs::prelude::*;
    /// use pltrs::units::UnitConverter;
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
    /// use pltrs::dates::DateTime;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let p = ax.convert_point((DateTime::new(1970, 1, 2, 0, 0, 0, 0)?, 3.0));
    /// assert_eq!((p.x, p.y), (1.0, 3.0));
    /// # Ok::<(), pltrs::dates::DateError>(())
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
    use crate::dates::{AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, DateTime};
    use crate::figure::{Figure, Share};
    use crate::ticker::{AxisName, MultipleLocator};
    use crate::units::DateConverter;

    /// 2020-01-01 at `hour` (UTC).
    fn at(hour: u32) -> DateTime {
        DateTime::new(2020, 1, 1, hour, 0, 0, 0).expect("a date")
    }

    /// The kind of date formatter `axis` has as its major formatter, if
    /// it has one.
    fn date_formatter(axis: &Axis) -> Option<DateConverter> {
        let formatter = axis.major_formatter();
        let formatter = formatter.as_ref() as &dyn Any;
        if formatter.is::<AutoDateFormatter>() {
            Some(DateConverter::Auto)
        } else {
            (formatter.is::<ConciseDateFormatter>()).then_some(DateConverter::Concise)
        }
    }

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

    /// matplotlib's `update_units` sets the date ticks up only when the
    /// axis takes the date converter or a unit: later dates and
    /// `axis_date()` keep them.
    #[test]
    fn date_ticks_are_set_up_once() {
        crate::rc::context(|| {
            let ax = Figure::new().subplot();
            ax.plot(vec![at(12), at(18)], [0.0, 1.0]).expect("a line");
            crate::rc::set("date.converter", "concise").expect("a valid rcParam");
            ax.plot(vec![at(12), at(18)], [1.0, 0.0]).expect("a line");
            ax.xaxis_date(None);
            assert_eq!(
                date_formatter(&ax.xaxis()),
                Some(DateConverter::Auto),
                "the first converter's formatter stays"
            );
        });
    }

    /// An explicit converter of dates brings no ticks until `axis_date`
    /// gives the axis a zone, and then those of its kind: matplotlib's
    /// `DateConverter()` and `ConciseDateConverter()` keep theirs whatever
    /// `date.converter` says, while its `_SwitchableDateConverter` takes
    /// the rcParam's kind when it ticks.
    #[test]
    fn explicit_date_converters_tick_as_their_kind() {
        use DateConverter::{Auto, Concise};
        let cases = [
            (UnitConverter::Dates, "concise", Concise),
            (UnitConverter::Dates, "auto", Auto),
            (Auto.into(), "concise", Auto),
            (Concise.into(), "auto", Concise),
        ];
        for (converter, ticked, kind) in cases {
            crate::rc::context(|| {
                let ax = Figure::new().subplot();
                ax.xaxis()
                    .set_converter(converter.clone())
                    .expect("no converter yet");
                ax.plot(vec![at(12), at(18)], [0.0, 1.0]).expect("a line");
                ax.xaxis_date(None);
                assert_eq!(date_formatter(&ax.xaxis()), None, "{converter:?}");
                crate::rc::set("date.converter", ticked).expect("a valid rcParam");
                ax.xaxis_date(&TimeZone::Utc);
                assert_eq!(date_formatter(&ax.xaxis()), Some(kind), "{converter:?}");
                assert_eq!(ax.xaxis().converter(), Some(converter));
            });
        }
    }

    /// matplotlib's `set_converter` after plotted dates replaces their
    /// converter without touching the ticks, which change only when
    /// `axis_date` gives the axis a zone.
    #[test]
    fn an_explicit_date_converter_after_plotted_dates_ticks_with_a_zone() {
        crate::rc::context(|| {
            crate::rc::set("date.converter", "concise").expect("a valid rcParam");
            let ax = Figure::new().subplot();
            ax.plot(vec![at(12), at(18)], [0.0, 1.0]).expect("a line");
            ax.xaxis()
                .set_converter(DateConverter::Auto)
                .expect("no explicit converter yet");
            ax.xaxis_date(None);
            let concise = Some(DateConverter::Concise);
            assert_eq!(date_formatter(&ax.xaxis()), concise, "the plotted ticks");
            ax.xaxis_date(&TimeZone::Utc);
            assert_eq!(date_formatter(&ax.xaxis()), Some(DateConverter::Auto));
        });
    }

    /// Converters of dates of different kinds are different converters,
    /// so an axis keeps the first one set explicitly.
    #[test]
    fn date_converters_of_different_kinds_differ() {
        let ax = Figure::new().subplot();
        let xaxis = ax.xaxis();
        xaxis
            .set_converter(DateConverter::Auto)
            .expect("no converter yet");
        assert!(xaxis.set_converter(DateConverter::Auto).is_ok());
        for other in [UnitConverter::Dates, DateConverter::Concise.into()] {
            assert_eq!(
                xaxis.set_converter(other).err(),
                Some(PlotError::ExplicitConverter(AxisName::X))
            );
        }
        let auto = UnitConverter::DatesBy(DateConverter::Auto);
        assert_eq!(xaxis.converter(), Some(auto));
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

        let axs = fig
            .subplots_shared(2, 1, Share::All, Share::None)
            .expect("a grid");
        (axs[(0, 0)].xaxis())
            .set_converter(DateConverter::Concise)
            .expect("no converter yet");
        axs[(0, 0)].xaxis_date(&TimeZone::Utc);
        let below = date_formatter(&axs[(1, 0)].xaxis());
        assert_eq!(below, Some(DateConverter::Concise), "the converter's kind");
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
        let dates = xaxis.convert_units(AxisValues::Dates {
            numbers: vec![10.0],
            tz: Some(TimeZone::Utc),
        });
        assert_eq!(dates, [10.0], "values convert as their own kind");
        assert_eq!(xaxis.converter(), Some(UnitConverter::Categories));
    }

    #[test]
    fn shared_axes_take_over_converted_units() {
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 1, Share::All, Share::None).unwrap();
        let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
        top.xaxis().convert_units(AxisValues::Dates {
            numbers: vec![0.0, 30.0],
            tz: None,
        });
        top.set_xlim(0.0, 30.0).expect("finite limits");
        assert_eq!(bottom.xaxis().converter(), Some(UnitConverter::Dates));
        let ticks = bottom.ticks(AxisName::X).unwrap();
        assert!(ticks.major[0].label.starts_with("1970-"), "date ticks");
        assert!(!bottom.yaxis().have_units(), "only the shared axis");
    }
}
