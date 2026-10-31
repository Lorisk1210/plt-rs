//! Unit conversion on the x, y and z axes of 3-D axes (matplotlib's
//! `Axis.convert_units`, `update_units`, `set_units`, `get_units`,
//! `set_converter` and `Axes3D.convert_zunits` on `axis3d.Axis`): dates,
//! category names and user values become axis numbers on the axis they
//! lie along, which takes the tick helpers and label their converter
//! gives. The conversions are those [`UnitHandle`] shares with 2-D axes.

use std::fmt;
use std::rc::Rc;

use super::axis_handle::Axis3D;
use super::axis3d::Axis3DState;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::share::ShareKey;
use super::state::Axes3DState;
use crate::artist::{ArtistHandle, ArtistId, DrawError, RawHandle};
use crate::ticker::{Formatter, Locator};
use crate::units::{
    AxisDeltas, AxisUnits, AxisValue, AxisValues, Categories, CustomValues, Unit, UnitAxis,
    UnitAxisInfo, UnitConverter, UnitHandle, UnitState,
};

/// The user values the 3-D geometry of an artist came from along x, y
/// and z, which it converts again when an axis changes unit: those of
/// lines and texts, as matplotlib's convert theirs when drawn (the other
/// 3-D artists keep the numbers they were made with).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Sources3D([Option<CustomValues>; 3]);

pltrs_core::archive_struct!(Sources3D(v0));

impl Sources3D {
    /// The user values among `values`, given along x, y and z.
    pub(crate) fn of(values: [&AxisValues; 3]) -> Self {
        Self(values.map(|values| values.custom().cloned()))
    }

    /// The coordinates of `position` that are user values.
    pub(crate) fn of_position(position: &[AxisValue; 3]) -> Self {
        Self(position.each_ref().map(|value| match value {
            AxisValue::Custom(values) => Some(values.clone()),
            AxisValue::Number(_) | AxisValue::Category(_) | AxisValue::Date { .. } => None,
        }))
    }

    /// The user values along `dir`, if any.
    pub(crate) fn along(&self, dir: Axis3) -> Option<&CustomValues> {
        self.0[dir.index()].as_ref()
    }
}

impl UnitState for Axis3DState {
    fn units(&self) -> &AxisUnits {
        &self.units
    }

    fn units_mut(&mut self) -> &mut AxisUnits {
        &mut self.units
    }

    fn categories(&self) -> Option<&Categories> {
        self.categories.as_ref()
    }

    fn categories_mut(&mut self) -> &mut Option<Categories> {
        &mut self.categories
    }

    fn use_unit_tickers(&mut self, locator: Rc<dyn Locator>, formatter: Rc<dyn Formatter>) {
        let tickers = &mut self.tickers;
        if tickers.defaults.major.locator {
            tickers.major_locator = locator;
        }
        if tickers.defaults.major.formatter {
            tickers.major_formatter = formatter;
        }
    }

    fn use_axis_info_tickers(&mut self, info: &UnitAxisInfo) {
        let tickers = &mut self.tickers;
        let defaults = tickers.defaults;
        if let Some(locator) = info.majloc.as_ref().filter(|_| defaults.major.locator) {
            tickers.major_locator = Rc::clone(locator);
        }
        if let Some(locator) = info.minloc.as_ref().filter(|_| defaults.minor.locator) {
            tickers.minor_locator = Rc::clone(locator);
        }
        if let Some(formatter) = info.majfmt.as_ref().filter(|_| defaults.major.formatter) {
            tickers.major_formatter = Rc::clone(formatter);
        }
        if let Some(formatter) = info.minfmt.as_ref().filter(|_| defaults.minor.formatter) {
            tickers.minor_formatter = Rc::clone(formatter);
        }
    }

    fn use_default_label(&mut self, label: &str) {
        if self.label.is_default {
            label.clone_into(&mut self.label.text);
        }
    }
}

impl UnitHandle for Axis3D {
    type State = Axis3DState;

    fn unit_axis(&self) -> UnitAxis {
        UnitAxis::Axes3D(self.clone())
    }

    fn axes_raw(&self) -> &RawHandle {
        self.axes.raw()
    }

    fn read_state<R>(&self, f: impl FnOnce(&Axis3DState) -> R) -> R {
        self.axes.with(|axes| f(axes.axis(self.dir)))
    }

    fn update_state<R>(&self, f: impl FnOnce(&mut Axis3DState) -> R) -> R {
        self.axes.with_mut(|axes| f(axes.axis_mut(self.dir)))
    }

    fn update_shared_states(&self, mut f: impl FnMut(&mut Axis3DState)) {
        let dir = self.dir;
        self.axes
            .update_group(ShareKey::Axis(dir), |axes| f(axes.axis_mut(dir)));
    }

    fn sharing(&self) -> Vec<Self> {
        (self.axes.shared_axes(self.dir).iter())
            .map(|axes| axes.axis(self.dir))
            .collect()
    }

    /// Converts again the user values the lines and texts of the axes
    /// keep along this axis, then recomputes the data limits and
    /// autoscales; a failure to autoscale is reported when the figure
    /// next draws.
    fn units_changed(&self) {
        let dir = self.dir;
        let sources = self.axes.with(|axes| axes.unit_sources(dir));
        let converted: Vec<_> = (sources.into_iter())
            .map(|(id, values)| (id, self.convert_custom(&values)))
            .collect();
        let autoscaled = self.axes.with_mut(|axes| axes.reconvert(dir, &converted));
        let figure = self.axes.raw().figure_state();
        let mut state = figure.borrow_mut();
        state.stale = true;
        if let Err(error) = autoscaled {
            state
                .pending_errors
                .push(DrawError::new(self.axes.id(), error.to_string()));
        }
    }

    /// Nothing: 3-D axes keep their view, as matplotlib's do (their 2-D
    /// view limits, which matplotlib checks, were set when they were
    /// made).
    fn set_default_intervals(&self, _limits: (f64, f64)) {}
}

impl Axis3D {
    /// Converts plotting values to numbers of this axis, registering new
    /// category names (matplotlib's `update_units` + `convert_units`, as
    /// `_process_unit_info` and `convert_zunits` do): dates become date
    /// numbers and give the axis date ticks, names their positions with
    /// a tick at each, and user values the numbers their converter gives
    /// in the axis unit, the axis taking the unit, tick helpers and label
    /// of the converter if it has no unit. The axes sharing this axis
    /// take over its converter and category names. Values that fail to
    /// convert become NaN, and the figure reports why when it next draws.
    ///
    /// ```
    /// use pltrs::dates::DateTime;
    /// use pltrs::figure::Figure;
    /// use pltrs::units::UnitConverter;
    /// let ax = Figure::new().subplot_3d();
    /// let day = |d| DateTime::new(2020, 1, d, 0, 0, 0, 0);
    /// let z = ax.zaxis().convert_units(vec![day(1)?, day(2)?]);
    /// assert_eq!(z, [18_262.0, 18_263.0]);
    /// assert_eq!(ax.zaxis().converter(), Some(UnitConverter::Dates));
    /// # Ok::<(), pltrs::dates::DateError>(())
    /// ```
    pub fn convert_units(&self, values: impl Into<AxisValues>) -> Vec<f64> {
        self.convert_values(values.into())
    }

    /// Converts one plotting value to a number of this axis, as
    /// [`convert_units`](Self::convert_units) converts a list.
    pub fn convert_value(&self, value: impl Into<AxisValue>) -> f64 {
        self.convert_one(value.into())
    }

    /// Lets the axis pick its converter and unit from `values`
    /// (matplotlib's `update_units`), as [`Axis::update_units`] does;
    /// returns whether the values have units, numbers having none.
    ///
    /// [`Axis::update_units`]: crate::axes::Axis::update_units
    pub fn update_units(&self, values: impl Into<AxisValues>) -> bool {
        self.update_units_from(values.into())
    }

    /// The unit converter: the one set with
    /// [`set_converter`](Self::set_converter), else that of the values
    /// converted on the axis, if any (matplotlib's `get_converter`).
    pub fn converter(&self) -> Option<UnitConverter> {
        self.current_converter()
    }

    /// Sets the unit converter explicitly (matplotlib's `set_converter`):
    /// values converted later no longer change it. A
    /// [`DateConverter`](crate::units::DateConverter) gives the axis dates
    /// of that kind whatever `date.converter` says, as
    /// [`Axis::set_converter`] does.
    ///
    /// # Errors
    ///
    /// When another converter was already set explicitly; converters of
    /// dates of different kinds are different converters.
    ///
    /// [`Axis::set_converter`]: crate::axes::Axis::set_converter
    pub fn set_converter(&self, converter: impl Into<UnitConverter>) -> Result<&Self, Axes3DError> {
        let converter = converter.into();
        if self.update_state(|axis| axis.units.set_explicit(converter)) {
            Ok(self)
        } else {
            Err(Axes3DError::ExplicitConverter(self.dir))
        }
    }

    /// Sets the unit of the axis and of the axes sharing it (matplotlib's
    /// `set_units`): each takes the tick helpers and label its converter
    /// gives for the unit, and the lines and texts keeping user values
    /// along the axis convert them again, after which the data limits are
    /// recomputed from the data artists and the axis autoscales. The
    /// other artists keep the numbers they were made with, as 2-D bars,
    /// fills and scatters do.
    ///
    /// ```
    /// use pltrs::dates::{DateTime, TimeZone, UtcOffset};
    /// use pltrs::figure::Figure;
    /// use pltrs::units::Unit;
    /// let ax = Figure::new().subplot_3d();
    /// let hour = |h| DateTime::new(2020, 1, 1, h, 0, 0, 0);
    /// ax.plot([0.0, 1.0], [0.0, 1.0], vec![hour(0)?, hour(12)?])?;
    /// let utc = ax.zaxis().ticklabels(false)?;
    /// let five = TimeZone::Fixed(UtcOffset::from_seconds(5 * 3600)?);
    /// ax.zaxis().set_units(five.clone());
    /// assert_eq!(ax.zaxis().units(), Some(Unit::new(five)));
    /// assert_ne!(ax.zaxis().ticklabels(false)?, utc, "dates ticked five hours on");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
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

    /// Whether the axis has a unit converter or a unit (matplotlib's
    /// `have_units`).
    pub fn have_units(&self) -> bool {
        self.has_units()
    }

    /// The category names placed on the axis, in position order, if any
    /// were (matplotlib's `axis.units` on a categorical axis).
    pub fn categories(&self) -> Option<Categories> {
        self.read_state(|axis| axis.categories.clone())
    }

    /// Extents from `origin`, which converted to `converted`, in numbers
    /// of this axis: numbers as they are, other extents as their far end
    /// converted less the converted origin (matplotlib's `_convert_dx`).
    pub(crate) fn convert_extents(
        &self,
        deltas: &AxisDeltas,
        origin: &AxisValues,
        converted: &[f64],
    ) -> Vec<f64> {
        match deltas {
            AxisDeltas::Numbers(numbers) => numbers.clone(),
            AxisDeltas::Durations(_) | AxisDeltas::Custom(_) => {
                self.convert_dx(deltas, origin, converted)
            }
        }
    }
}

impl Axes3D {
    /// Converts plotting values along x, y and z to axis numbers, each on
    /// its axis and in that order (matplotlib's `_process_unit_info` with
    /// the three datasets: user values set up an axis that has no unit,
    /// then convert in the axis unit).
    pub(crate) fn convert_xyz(&self, values: [AxisValues; 3]) -> [Vec<f64>; 3] {
        let [x, y, z] = values;
        let x = self.xaxis().convert_values(x);
        let y = self.yaxis().convert_values(y);
        [x, y, self.zaxis().convert_values(z)]
    }

    /// Converts `x`, `y` and `z` on the axes they lie along once the 2-D
    /// points `(x, y)` are placed in the plane normal to `zdir`
    /// (matplotlib's `juggle_axes`), returning them in their own order.
    pub(crate) fn convert_juggled(&self, values: [AxisValues; 3], zdir: Axis3) -> [Vec<f64>; 3] {
        zdir.unjuggle(self.convert_xyz(zdir.juggle(values)))
    }

    /// Converts one position as texts convert theirs, without changing
    /// the axes (matplotlib converts text positions when drawing them).
    pub(crate) fn convert_position(&self, position: [AxisValue; 3]) -> [f64; 3] {
        let [x, y, z] = position;
        [
            self.xaxis().convert_when_drawn(&x),
            self.yaxis().convert_when_drawn(&y),
            self.zaxis().convert_when_drawn(&z),
        ]
    }
}

impl Axes3DState {
    /// The user values the artists keep along `dir`, with their ids.
    fn unit_sources(&self, dir: Axis3) -> Vec<(ArtistId, CustomValues)> {
        (self.artists.iter())
            .filter_map(|artist| Some((artist.id, artist.sources.along(dir)?.clone())))
            .collect()
    }

    /// Writes the numbers the user values along `dir` converted to into
    /// the geometry of their artists, recomputes the data limits and
    /// autoscales.
    fn reconvert(
        &mut self,
        dir: Axis3,
        converted: &[(ArtistId, Vec<f64>)],
    ) -> Result<(), Axes3DError> {
        for (id, numbers) in converted {
            if let Some(artist) = self.artists.iter_mut().find(|a| a.id == *id) {
                artist.art.reconvert(dir, numbers);
            }
        }
        self.relim();
        self.autoscale_view()
    }
}
