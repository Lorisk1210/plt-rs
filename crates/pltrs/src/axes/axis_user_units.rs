//! User converters on an axis: picking a unit from plotted values,
//! setting one, taking the converter's tick helpers, label and default
//! view, and converting the plotted lines again when the unit changes
//! (matplotlib's `Axis.update_units`, `set_units`, `get_units`,
//! `_update_axisinfo` and `Axes._unit_change_handler`). The conversions
//! are those [`UnitHandle`] shares with 3-D axes.

use std::fmt;

use super::axis_handle::Axis;
use super::{Axes, AxesState};
use crate::artist::{ArtistHandle, ArtistLookup};
use crate::ticker::AxisName;
use crate::units::{AxisValues, Unit, UnitHandle};

impl Axis {
    /// Lets the axis pick its converter and unit from `values`
    /// (matplotlib's `update_units`): user values give it their converter,
    /// unless one was set explicitly, and the unit the converter picks
    /// for them if the axis has none; a new converter brings its tick
    /// helpers and label. Dates and category names register as when they
    /// are converted, and [`ZonedDateTime`](crate::dates::ZonedDateTime)s
    /// make the zone of the first the unit if the axis has none, as
    /// plotting them does. Returns whether the values have units, numbers
    /// having none.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(!ax.xaxis().update_units(vec![1.0, 2.0]));
    /// assert!(ax.xaxis().update_units(vec!["a", "b"]));
    /// assert_eq!(ax.xaxis().categories().expect("names").names(), ["a", "b"]);
    /// ```
    pub fn update_units(&self, values: impl Into<AxisValues>) -> bool {
        self.update_units_from(values.into())
    }

    /// Sets the unit of the axis and of the axes sharing it (matplotlib's
    /// `set_units`): each takes the tick helpers and label its converter
    /// gives for the unit, and the artists on it that keep user values
    /// (lines, texts, annotations, rectangles of a
    /// [`DataRectangle`](crate::patches::DataRectangle)) convert them
    /// again. Before any user value is converted the axis has no
    /// converter to ask, so the unit only decides how values convert
    /// later. Dates follow a [`TimeZone`](crate::dates::TimeZone) unit;
    /// category axes keep their names.
    ///
    /// ```
    /// use pltrs::dates::TimeZone;
    /// use pltrs::prelude::*;
    /// use pltrs::units::Unit;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_units(TimeZone::Utc);
    /// assert_eq!(ax.xaxis().units(), Some(Unit::new(TimeZone::Utc)));
    /// assert!(ax.xaxis().have_units() && ax.xaxis().converter().is_none());
    /// ```
    pub fn set_units<U: PartialEq + fmt::Debug + 'static>(&self, unit: U) -> &Self {
        self.set_unit(Unit::new(unit));
        self
    }

    /// The unit of the axis, once one was set or picked for user values
    /// (matplotlib's `get_units`).
    pub fn units(&self) -> Option<Unit> {
        self.unit()
    }
}

impl Axes {
    /// Follows a change of unit of axis `name` (matplotlib's
    /// `_unit_change_handler`, and its artists converting their values
    /// when drawn): the artists that keep user values
    /// ([`Artist::unit_sources`](crate::artist::Artist::unit_sources))
    /// convert them again, the data limits are recomputed and the axis
    /// autoscales.
    pub(super) fn units_changed(&self, name: AxisName) {
        let axis = self.axis_of(name);
        let figure = self.raw().figure_state();
        let children = self
            .raw()
            .with::<AxesState, _>(|axes| axes.children.clone());
        for id in children {
            let sources = figure
                .borrow()
                .artist(id)
                .map(|artist| artist.unit_sources(name))
                .unwrap_or_default();
            if sources.is_empty() {
                continue;
            }
            let numbers = sources
                .iter()
                .map(|values| axis.convert_custom(values))
                .collect();
            let mut state = figure.borrow_mut();
            if let Some(artist) = state.artist_mut(id) {
                artist.reconvert_units(name, numbers);
                state.stale = true;
            }
        }
        self.relim(false);
        self.raw().with_mut::<AxesState, _>(|axes| {
            axes.request_autoscale(name == AxisName::X, name == AxisName::Y);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::rc::Rc;

    use super::*;
    use crate::dates::TimeZone;
    use crate::figure::{Figure, Share};
    use crate::lines::LineProps;
    use crate::ticker::{FixedLocator, MultipleLocator};
    use crate::units::{
        ConversionError, ConversionInterface, Convertible, CustomConverter, UnitAxis, UnitAxisInfo,
    };

    /// A distance in kilometres or metres.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Distance {
        metres: f64,
        unit: DistanceUnit,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum DistanceUnit {
        Metres,
        Kilometres,
        /// A unit the converter refuses.
        Furlongs,
    }

    impl DistanceUnit {
        fn metres(self) -> f64 {
            match self {
                Self::Metres => 1.0,
                Self::Kilometres => 1000.0,
                Self::Furlongs => 201.168,
            }
        }
    }

    fn km(value: f64) -> Distance {
        Distance {
            metres: value * 1000.0,
            unit: DistanceUnit::Kilometres,
        }
    }

    /// Converts distances; `scale` multiplies the numbers, telling the
    /// converters apart.
    #[derive(Default)]
    struct DistanceConverter {
        scale: f64,
    }

    impl ConversionInterface for DistanceConverter {
        type Value = Distance;
        type Unit = DistanceUnit;

        fn convert(
            &self,
            values: &[Distance],
            unit: Option<&DistanceUnit>,
            _: &UnitAxis,
        ) -> Result<Vec<f64>, ConversionError> {
            let unit = unit.copied().unwrap_or(DistanceUnit::Metres);
            if unit == DistanceUnit::Furlongs {
                return Err(ConversionError::Failed("no furlongs".into()));
            }
            Ok(values
                .iter()
                .map(|d| self.scale * d.metres / unit.metres())
                .collect())
        }

        fn axisinfo(&self, unit: Option<&DistanceUnit>, _: &UnitAxis) -> Option<UnitAxisInfo> {
            Some(UnitAxisInfo {
                majloc: Some(Rc::new(MultipleLocator::new(0.5).expect("positive"))),
                label: Some(format!("{:?}", unit?)),
                default_limits: Some((0.0, 100.0)),
                ..UnitAxisInfo::default()
            })
        }

        fn default_units(&self, values: &[Distance], _: &UnitAxis) -> Option<DistanceUnit> {
            values.first().map(|d| d.unit)
        }
    }

    impl Convertible for Distance {
        type Converter = DistanceConverter;
        fn converter() -> DistanceConverter {
            DistanceConverter { scale: 1.0 }
        }
    }

    fn is_multiple_locator(axis: &Axis) -> bool {
        (axis.major_locator().as_ref() as &dyn Any).is::<MultipleLocator>()
    }

    #[test]
    fn values_bring_their_unit_label_ticks_and_default_view() {
        let ax = Figure::new().subplot();
        let xaxis = ax.xaxis();
        assert!(xaxis.update_units(vec![km(10.0)]));
        assert_eq!(xaxis.units(), Some(Unit::new(DistanceUnit::Kilometres)));
        let converter = xaxis.converter().expect("a converter");
        assert!(
            converter
                .custom()
                .is_some_and(CustomConverter::is::<DistanceConverter>)
        );
        assert_eq!(ax.label(AxisName::X), "Kilometres");
        assert!(is_multiple_locator(&xaxis));
        assert_eq!(ax.xlim(), (0.0, 100.0), "the default limits");
        assert_eq!(xaxis.convert_value(km(2.5)), 2.5);
    }

    #[test]
    fn data_keeps_its_view() {
        let ax = Figure::new().subplot();
        ax.axvline(30.0, LineProps::default());
        ax.plot(vec![km(0.0), km(2.0)], vec![km(0.0), km(4.0)])
            .expect("a line");
        assert_eq!(ax.xlim().1, 31.5, "the line at 30 counts as data");
        assert_eq!(ax.ylim(), (-0.2, 4.2), "no data in y before the line");
    }

    #[test]
    fn a_new_unit_converts_the_lines_again() {
        let ax = Figure::new().subplot();
        let line = ax.plot(vec![km(1.0), km(2.0)], [0.0, 1.0]).expect("a line");
        let mark = ax.axvline(km(1.5), LineProps::default());
        let replaced = ax.plot(vec![km(1.0)], [2.0]).expect("a line");
        replaced.set_xdata(vec![7.0]);
        ax.xaxis().set_units(DistanceUnit::Metres);
        assert_eq!(line.xdata(), [1000.0, 2000.0]);
        assert_eq!(mark.xdata(), [1500.0, 1500.0]);
        assert_eq!(replaced.xdata(), [7.0], "replaced data stays");
        assert_eq!(ax.label(AxisName::X), "Metres");
        assert_eq!(ax.data_lim().bbox.x1(), 2000.0);
        assert_eq!(line.ydata(), [0.0, 1.0]);
    }

    #[test]
    fn units_set_before_any_value_leave_the_axis_unlabelled() {
        let ax = Figure::new().subplot();
        ax.xaxis().set_units(DistanceUnit::Metres);
        assert!(ax.xaxis().have_units() && ax.xaxis().converter().is_none());
        ax.set_xlim(km(0.0), km(1.0)).expect("finite limits");
        assert_eq!(ax.xlim(), (0.0, 1000.0));
        assert_eq!(
            ax.label(AxisName::X),
            "",
            "no axis info without a converter"
        );
        assert!(ax.xaxis().converter().is_some(), "but now a converter");
    }

    #[test]
    fn plotting_asks_for_axis_info_even_with_units() {
        let ax = Figure::new().subplot();
        ax.yaxis().set_units(DistanceUnit::Metres);
        ax.plot([0.0, 1.0], vec![km(1.0), km(2.0)]).expect("a line");
        assert_eq!(ax.label(AxisName::Y), "Metres");
        assert_eq!(ax.ylim().1, 2050.0);
    }

    #[test]
    fn labels_and_tick_helpers_set_by_hand_stay() {
        let ax = Figure::new().subplot();
        ax.set_xlabel("distance");
        ax.xaxis().set_major_locator(FixedLocator::new(vec![1.0]));
        ax.plot(vec![km(1.0)], [1.0]).expect("a line");
        assert_eq!(ax.label(AxisName::X), "distance");
        assert!(!is_multiple_locator(&ax.xaxis()));
    }

    #[test]
    fn shared_axes_share_the_unit() {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(2, 1, Share::All, Share::None)
            .expect("a grid");
        let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
        top.plot(vec![km(1.0)], [0.0]).expect("a line");
        assert_eq!(
            bottom.xaxis().units(),
            Some(Unit::new(DistanceUnit::Kilometres))
        );
        bottom.xaxis().set_label_text("");
        let below = bottom.plot(vec![km(3.0)], [0.0]).expect("a line");
        assert!(is_multiple_locator(&bottom.xaxis()), "shared tick helpers");
        top.xaxis().set_units(DistanceUnit::Metres);
        assert_eq!(below.xdata(), [3000.0]);
        assert_eq!(top.label(AxisName::X), "", "inner labels go on a grid");
        assert_eq!(bottom.label(AxisName::X), "", "a label set by hand");
    }

    #[test]
    fn an_explicit_converter_of_the_values_type_converts_them() {
        let ax = Figure::new().subplot();
        ax.xaxis()
            .set_converter(DistanceConverter { scale: 2.0 })
            .expect("no converter yet");
        // Having a converter, the axis picks no unit: metres, doubled.
        assert_eq!(ax.xaxis().convert_value(km(1.0)), 2000.0);
        assert!(
            ax.xaxis()
                .set_converter(DistanceConverter::default())
                .is_ok(),
            "converters of one type are one converter"
        );
    }

    #[test]
    fn failed_conversions_are_reported_when_drawing() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.xaxis().set_units(DistanceUnit::Furlongs);
        let line = ax.plot(vec![km(1.0)], [1.0]).expect("a line");
        assert!(line.xdata()[0].is_nan());
        let error = fig.to_png().expect_err("a pending error");
        assert!(error.to_string().contains("no furlongs"), "{error}");
    }

    #[test]
    fn plotting_methods_convert_user_values() {
        use crate::axes::{BarOptions, ErrorbarOptions, FillBetweenOptions};
        use crate::text::AnnotateOptions;

        let ax = Figure::new().subplot();
        let points = ax
            .scatter(vec![km(1.0), km(2.0)], [0.0, 1.0])
            .expect("points");
        assert_eq!(points.offsets()[1].x, 2.0);
        let bars = ax
            .bar(vec![km(3.0)], [1.0], BarOptions::default())
            .expect("bars");
        assert_eq!(bars.position_centers(), [3.0]);
        ax.fill_between(
            vec![km(0.0), km(1.0)],
            [0.0],
            [1.0],
            FillBetweenOptions::default(),
        )
        .expect("a fill");
        ax.errorbar(vec![km(4.0)], [0.5], ErrorbarOptions::default())
            .expect("error bars");
        let span = ax.axvspan(km(5.0), km(6.0));
        assert_eq!((span.shape().xy.x, span.shape().width), (5.0, 1.0));
        let segments = ax
            .hlines([0.5], vec![km(1.0)], vec![km(7.0)])
            .expect("segments");
        assert_eq!(segments.paths()[0].vertices()[1].x, 7.0);
        let note = ax.text(km(2.5), 0.5, "note");
        assert_eq!(note.position().x, 2.5);
        let arrow = ax
            .annotate("a", (km(3.5), 0.5), AnnotateOptions::default())
            .expect("an annotation");
        assert_eq!(arrow.xy().x, 3.5);
        ax.set_xlim(km(0.0), km(8.0)).expect("finite limits");
        assert_eq!(ax.xlim(), (0.0, 8.0));
        assert!(!ax.yaxis().have_units(), "numbers leave an axis alone");
    }

    #[test]
    fn date_axes_tick_in_a_time_zone_unit() {
        use crate::dates::{DateTime, UtcOffset};
        let ax = Figure::new().subplot();
        let hour = |h| DateTime::new(2020, 1, 1, h, 0, 0, 0).expect("a date");
        ax.plot(vec![hour(0), hour(12)], [0.0, 1.0])
            .expect("a line");
        assert_eq!(ax.xaxis().ticklabels(false).expect("ticks")[0], "01-01 00");
        let five = TimeZone::Fixed(UtcOffset::from_seconds(5 * 3600).expect("an offset"));
        ax.xaxis().set_units(five.clone());
        ax.plot(vec![hour(6)], [0.5]).expect("a line");
        assert_eq!(ax.xaxis().ticklabels(false).expect("ticks")[0], "01-01 06");
        assert_eq!(ax.xaxis().units(), Some(Unit::new(five)));
    }
}
