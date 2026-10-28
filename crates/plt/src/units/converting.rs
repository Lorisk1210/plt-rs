//! Unit conversion on an axis, shared by the x and y axes of 2-D axes and
//! the three axes of 3-D axes: the converter and unit an axis holds,
//! picking them from plotted values, the tick helpers and label they
//! bring, and the values converted again when the unit changes
//! (matplotlib's `Axis.update_units`, `convert_units`, `set_units`,
//! `set_converter`, `axis_date` and `_update_axisinfo`, and `Axes`'s
//! `_process_unit_info` and `_convert_dx`).
//!
//! An axis state implements [`UnitState`] and an axis handle
//! [`UnitHandle`]; the provided methods of both are the conversions.

use std::rc::Rc;

use super::{
    AxisDelta, AxisDeltas, AxisValue, AxisValues, Categories, CategoryFormatter, CategoryLocator,
    ConversionError, CustomValues, Unit, UnitAxis, UnitAxisInfo, UnitConverter, first_finite,
};
use crate::artist::{DrawError, RawHandle};
use crate::dates::{Datetime64, TimeZone, get_epoch};
use crate::ticker::{Formatter, Locator};

/// Seconds in a day: the length of the default view of a date axis.
const SECONDS_PER_DAY: i64 = 86_400;

/// The unit converter of an axis, how it was chosen, and the unit.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AxisUnits {
    /// The converter, once values with units were converted or one was
    /// set (matplotlib's `_converter`).
    pub(crate) converter: Option<UnitConverter>,
    /// Whether the converter was set explicitly, so that converted values
    /// do not replace it (matplotlib's `_converter_is_explicit`).
    pub(crate) explicit: bool,
    /// The unit, once one was set or picked for user values (matplotlib's
    /// `units`).
    pub(crate) unit: Option<Unit>,
}

impl AxisUnits {
    /// Takes `converter` unless another one was set explicitly
    /// (matplotlib's `_set_converter` from `update_units`); returns whether
    /// the converter changed (`update_units`' `neednew`), which is when
    /// its tick helpers are set up.
    pub(crate) fn adopt(&mut self, converter: UnitConverter) -> bool {
        if self.explicit || self.converter.as_ref() == Some(&converter) {
            return false;
        }
        self.converter = Some(converter);
        true
    }

    /// The converter, which is `converter` when there was none
    /// (matplotlib's `Axis.convert_units`, which sets the converter of
    /// the values it converts only on an axis without one).
    pub(crate) fn converter_or(&mut self, converter: UnitConverter) -> UnitConverter {
        self.converter.get_or_insert(converter).clone()
    }

    /// The time zone of date ticks when the unit is one, as matplotlib's
    /// date converter takes it; without, date ticks are in the zone the
    /// `timezone` rcParam names (the converter's `tz=None`).
    pub(crate) fn time_zone(&self) -> Option<TimeZone> {
        self.unit
            .as_ref()
            .and_then(Unit::downcast_ref::<TimeZone>)
            .cloned()
    }

    /// Takes `converter` explicitly (matplotlib's `set_converter`), unless
    /// another one was already set explicitly; returns whether it did.
    pub(crate) fn set_explicit(&mut self, converter: UnitConverter) -> bool {
        if self.explicit && self.converter.as_ref() != Some(&converter) {
            return false;
        }
        self.converter = Some(converter);
        self.explicit = true;
        true
    }
}

/// The unit data an axis shares with the axes sharing it: its converter
/// and unit, and its category names.
pub(crate) type UnitData = (AxisUnits, Option<Categories>);

/// The state of an axis that converts values: its unit data and the tick
/// helpers and label its converter may replace while they are the
/// defaults.
pub(crate) trait UnitState {
    /// The converter, how it was chosen, and the unit.
    fn units(&self) -> &AxisUnits;

    /// The same, to be changed.
    fn units_mut(&mut self) -> &mut AxisUnits;

    /// The category names placed on the axis, if any.
    fn categories(&self) -> Option<&Categories>;

    /// The same, to be changed.
    fn categories_mut(&mut self) -> &mut Option<Categories>;

    /// Installs `locator` and `formatter` as the major tick helpers where
    /// the defaults are still in place, as a converter's tick helpers
    /// (matplotlib's `_update_axisinfo` with the date and category
    /// converters).
    fn use_unit_tickers(&mut self, locator: Rc<dyn Locator>, formatter: Rc<dyn Formatter>);

    /// Installs the tick helpers of `info` where the defaults are still
    /// in place, which they stay (matplotlib's `_update_axisinfo` with a
    /// user converter).
    fn use_axis_info_tickers(&mut self, info: &UnitAxisInfo);

    /// Takes `label` as the axis label unless a label was set
    /// (matplotlib's `isDefault_label`).
    fn use_default_label(&mut self, label: &str);

    /// Installs the tick helpers of dates in `tz`, or in the zone of the
    /// `timezone` rcParam, that `converter` gives where the defaults are
    /// still in place (matplotlib's date converters' `axisinfo`); a
    /// converter of other values gives none here.
    fn use_date_tickers(&mut self, converter: &UnitConverter, tz: Option<&TimeZone>) {
        if let Some((locator, formatter)) = super::date_tickers(converter, tz) {
            self.use_unit_tickers(locator, formatter);
        }
    }

    /// Installs the tick helpers of the category names where the
    /// defaults are still in place (matplotlib's category converter's
    /// `axisinfo`).
    fn use_category_tickers(&mut self) {
        let Some(categories) = self.categories() else {
            return;
        };
        let locator: Rc<dyn Locator> = Rc::new(CategoryLocator::new(categories));
        let formatter: Rc<dyn Formatter> = Rc::new(CategoryFormatter::new(categories));
        self.use_unit_tickers(locator, formatter);
    }

    /// Takes date numbers, giving the axis the converter of dates that
    /// follows `date.converter` and its date ticks, in the time zone of
    /// the axis unit or else of the `timezone` rcParam, unless tick
    /// helpers were chosen explicitly (matplotlib's `update_units` +
    /// `convert_units` for dates, whose converter in `units.registry` is
    /// the `_SwitchableDateConverter`).
    fn convert_dates(&mut self, numbers: Vec<f64>) -> Vec<f64> {
        self.adopt_dates(UnitConverter::Dates);
        numbers
    }

    /// Takes `converter`, a converter of dates, unless another one was set
    /// explicitly, and its date ticks only when it is new to the axis
    /// (matplotlib's `update_units`, which updates the axis info when the
    /// converter changes): ticks set up by earlier dates stay, and an
    /// axis given a converter explicitly keeps its tick helpers.
    fn adopt_dates(&mut self, converter: UnitConverter) {
        if self.units_mut().adopt(converter.clone()) {
            let tz = self.units().time_zone();
            self.use_date_tickers(&converter, tz.as_ref());
        }
    }

    /// Converts category names to positions, registering new names and
    /// labelling the ticks with them unless tick helpers were chosen
    /// explicitly (matplotlib's `update_units` + `convert_units` for
    /// strings).
    fn convert_categories(&mut self, names: &[String]) -> Vec<f64> {
        self.units_mut().adopt(UnitConverter::Categories);
        let positions = self
            .categories_mut()
            .get_or_insert_with(Categories::default)
            .convert(names);
        self.use_category_tickers();
        positions
    }

    /// The unit converter and category names, which an axis sharing the
    /// units takes over.
    fn unit_data(&self) -> UnitData {
        (self.units().clone(), self.categories().cloned())
    }

    /// Takes over the [`unit_data`](Self::unit_data) of an axis it is
    /// shared with, after values were converted there, with the tick
    /// helpers they bring where the defaults are in place: matplotlib's
    /// shared axes hold one `units` object (category names placed on one
    /// are placed on all) and share their tickers. A converter of dates
    /// is taken as it is, of the kind `date.converter` names or of a kind
    /// of its own. Converters of user values are not shared: matplotlib
    /// shares their units through `set_units` alone.
    fn share_units(&mut self, (units, categories): &UnitData) {
        match &units.converter {
            Some(converter @ (UnitConverter::Dates | UnitConverter::DatesBy(_))) => {
                self.adopt_dates(converter.clone());
            }
            Some(UnitConverter::Categories) => {
                self.units_mut().adopt(UnitConverter::Categories);
                self.categories_mut().clone_from(categories);
                self.use_category_tickers();
            }
            Some(UnitConverter::Custom(_)) | None => {}
        }
    }
}

/// A handle to an axis that converts plotting values. The required
/// methods reach the axis state, the axes sharing the axis and the
/// artists on it; the provided ones are the conversions.
///
/// Converters are user code that may use the figure: the provided
/// methods never call one while the figure is borrowed.
pub(crate) trait UnitHandle: Sized {
    /// The state of the axis.
    type State: UnitState;

    /// The axis as converters get it.
    fn unit_axis(&self) -> UnitAxis;

    /// The axes the axis belongs to, which reports failed conversions.
    fn axes_raw(&self) -> &RawHandle;

    /// Reads the state of the axis.
    fn read_state<R>(&self, f: impl FnOnce(&Self::State) -> R) -> R;

    /// Changes the state of the axis.
    fn update_state<R>(&self, f: impl FnOnce(&mut Self::State) -> R) -> R;

    /// Changes the state of the axis and of the axes sharing it.
    fn update_shared_states(&self, f: impl FnMut(&mut Self::State));

    /// This axis on every axes sharing it, this one included.
    fn sharing(&self) -> Vec<Self>;

    /// Follows a change of unit of the axis: the artists keeping user
    /// values convert them again and the axis autoscales (matplotlib's
    /// `Axes._unit_change_handler`).
    fn units_changed(&self);

    /// Makes `limits` the view when neither the data nor the view limits
    /// of the axis changed yet (matplotlib's `set_default_intervals`).
    fn set_default_intervals(&self, limits: (f64, f64));

    /// The unit, once one was set or picked (matplotlib's `get_units`).
    fn unit(&self) -> Option<Unit> {
        self.read_state(|state| state.units().unit.clone())
    }

    /// The unit converter (matplotlib's `get_converter`).
    fn current_converter(&self) -> Option<UnitConverter> {
        self.read_state(|state| state.units().converter.clone())
    }

    /// Whether the axis has a unit converter or a unit (matplotlib's
    /// `have_units`).
    fn has_units(&self) -> bool {
        self.read_state(|state| state.units().converter.is_some() || state.units().unit.is_some())
    }

    /// Lets the axis pick its converter and unit from `values`
    /// (matplotlib's `update_units`); returns whether the values have
    /// units, numbers having none.
    fn update_units_from(&self, values: AxisValues) -> bool {
        match values {
            AxisValues::Numbers(_) => false,
            AxisValues::Custom(values) => {
                self.update_custom_units(&values);
                true
            }
            values => {
                self.convert_values(values);
                true
            }
        }
    }

    /// [`update_units_from`](Self::update_units_from) for user values:
    /// they give the axis their converter, unless one was set explicitly,
    /// and the unit the converter picks for them if the axis has none; a
    /// new converter brings its tick helpers and label.
    fn update_custom_units(&self, values: &CustomValues) {
        let (current, explicit) =
            self.read_state(|state| (state.units().converter.clone(), state.units().explicit));
        let converter = match current.filter(|_| explicit) {
            Some(converter) => converter,
            None => UnitConverter::Custom(values.converter()),
        };
        let new = self.update_state(|state| state.units_mut().adopt(converter.clone()));
        let default = converter
            .custom()
            .and_then(|converter| values.default_units(converter, &self.unit_axis()));
        match default {
            Some(unit) if self.unit().is_none() => self.set_unit(unit),
            _ if new => self.update_axisinfo(),
            _ => {}
        }
    }

    /// Converts plotting values to axis numbers, registering new category
    /// names; the axes sharing this axis take over its converter,
    /// category names and the tick helpers they bring (matplotlib's
    /// `update_units` + `convert_units`, as `_process_unit_info` does).
    /// User values set the axis up first if it has no unit; values that
    /// fail to convert become NaN, and the figure reports why when it
    /// next draws.
    fn convert_values(&self, values: AxisValues) -> Vec<f64> {
        let positions = match values {
            // Numbers need no converter, so the axis is left untouched.
            AxisValues::Numbers(numbers) => return numbers,
            AxisValues::Custom(values) => {
                if !self.has_units() {
                    self.update_custom_units(&values);
                }
                return self.convert_custom(&values);
            }
            AxisValues::Dates(numbers) => self.update_state(|state| state.convert_dates(numbers)),
            AxisValues::Categories(names) => {
                self.update_state(|state| state.convert_categories(&names))
            }
        };
        let units = self.read_state(UnitState::unit_data);
        self.update_shared_states(|state| state.share_units(&units));
        positions
    }

    /// Converts plotting values as matplotlib's `plot` and `fill` do: user
    /// values update the axis units first even when it has some
    /// (`_plot_args` calling `update_units`).
    fn convert_plot_units(&self, values: AxisValues) -> Vec<f64> {
        if let AxisValues::Custom(values) = &values {
            self.update_custom_units(values);
        }
        self.convert_values(values)
    }

    /// Converts user values in the axis unit, without letting them change
    /// the unit (matplotlib's `Axis.convert_units`): an axis without a
    /// converter takes theirs, without its tick helpers. Values that fail
    /// to convert become NaN, and the figure reports why when it next
    /// draws.
    fn convert_custom(&self, values: &CustomValues) -> Vec<f64> {
        self.try_convert_custom(values).unwrap_or_else(|error| {
            self.report(&error);
            vec![f64::NAN; values.len()]
        })
    }

    /// [`convert_custom`](Self::convert_custom), failing instead of
    /// reporting.
    fn try_convert_custom(&self, values: &CustomValues) -> Result<Vec<f64>, ConversionError> {
        let fallback = UnitConverter::Custom(values.converter());
        self.update_state(|state| state.units_mut().converter_or(fallback));
        self.try_convert_custom_as_is(values)
    }

    /// User values in the axis unit, converted by the axis converter if it
    /// is of their converter's type and by their own otherwise, leaving
    /// the axis as it is.
    fn try_convert_custom_as_is(&self, values: &CustomValues) -> Result<Vec<f64>, ConversionError> {
        let (converter, unit) =
            self.read_state(|state| (state.units().converter.clone(), state.units().unit.clone()));
        let converter = converter.as_ref().and_then(UnitConverter::custom);
        values.convert(converter, unit.as_ref(), &self.unit_axis())
    }

    /// Reports a failed conversion when the figure next draws.
    fn report(&self, error: &ConversionError) {
        let raw = self.axes_raw();
        raw.figure_state()
            .borrow_mut()
            .pending_errors
            .push(DrawError {
                artist: raw.id(),
                message: error.to_string(),
            });
    }

    /// Sets the unit of the axis and of the axes sharing it (matplotlib's
    /// `set_units`): each takes the tick helpers and label its converter
    /// gives for the unit, and its artists keeping user values convert
    /// them again.
    fn set_unit(&self, unit: Unit) {
        if self.unit().as_ref() == Some(&unit) {
            return;
        }
        for axis in self.sharing() {
            axis.update_state(|state| state.units_mut().unit = Some(unit.clone()));
            axis.update_axisinfo();
            axis.units_changed();
        }
    }

    /// Takes the tick helpers, label and default view the converter gives
    /// for the unit (matplotlib's `_update_axisinfo`); the tick helpers
    /// are shared with the axes sharing the axis.
    fn update_axisinfo(&self) {
        let (converter, unit) =
            self.read_state(|state| (state.units().converter.clone(), state.units().unit.clone()));
        match converter {
            Some(UnitConverter::Custom(converter)) => {
                if let Some(info) = converter.axisinfo(unit.as_ref(), &self.unit_axis()) {
                    self.use_axis_info(&info);
                }
            }
            Some(converter @ (UnitConverter::Dates | UnitConverter::DatesBy(_))) => {
                if let Some(tz) = unit.as_ref().and_then(Unit::downcast_ref::<TimeZone>) {
                    self.use_date_axisinfo(&converter, Some(tz));
                }
            }
            Some(UnitConverter::Categories) | None => {}
        }
    }

    /// Installs what `info` gives.
    fn use_axis_info(&self, info: &UnitAxisInfo) {
        self.update_shared_states(|state| state.use_axis_info_tickers(info));
        if let Some(label) = &info.label {
            self.update_state(|state| state.use_default_label(label));
        }
        if let Some(limits) = info.default_limits {
            self.set_default_intervals(limits);
        }
    }

    /// Takes the converter of dates that follows `date.converter`, unless
    /// another one was set explicitly, with the tick helpers and default
    /// view of dates its converter gives (matplotlib's `axis_date`, which
    /// updates the units with a date in `tz`): a given zone becomes the
    /// unit of an axis without one, which sets them up in that zone;
    /// otherwise they come only with a converter new to the axis, in the
    /// unit's zone, else in the `timezone` rcParam's. So the ticks earlier
    /// dates set up stay, and an axis given a converter of dates
    /// explicitly keeps its tick helpers until it gets a zone, then takes
    /// those of that converter: one of a kind of its own ignores
    /// `date.converter`.
    fn make_date_axis(&self, tz: Option<&TimeZone>) {
        let (converter, new) = self.update_state(|state| {
            let units = state.units_mut();
            let new = units.adopt(UnitConverter::Dates);
            (units.converter.clone().filter(UnitConverter::is_dates), new)
        });
        let Some(converter) = converter else {
            return;
        };
        match tz {
            Some(tz) if self.unit().is_none() => self.set_unit(Unit::new(tz.clone())),
            _ if new => {
                let zone = self.read_state(|state| state.units().time_zone());
                self.use_date_axisinfo(&converter, zone.as_ref());
            }
            _ => {}
        }
    }

    /// The tick helpers and default view of dates in `tz` that
    /// `converter`, a converter of dates, gives (matplotlib's
    /// `_update_axisinfo` with a date converter's `axisinfo`): the axes
    /// sharing the axis take the tick helpers too.
    fn use_date_axisinfo(&self, converter: &UnitConverter, tz: Option<&TimeZone>) {
        self.update_shared_states(|state| state.use_date_tickers(converter, tz));
        let epoch = get_epoch();
        let start = epoch.datetime64_to_num(Datetime64::new(0, 0));
        let end = epoch.datetime64_to_num(Datetime64::new(SECONDS_PER_DAY, 0));
        self.set_default_intervals((start, end));
    }

    /// Extents converted to axis numbers by themselves (matplotlib's
    /// `convert(dx)`): numbers as they are, durations in days and user
    /// values by the axis converter, which an axis without one takes from
    /// them.
    fn convert_deltas(&self, deltas: &AxisDeltas) -> Vec<f64> {
        deltas
            .to_numbers()
            .unwrap_or_else(|values| self.convert_custom(values))
    }

    /// Extents from `origin`, whose values convert to `converted`, in axis
    /// numbers (matplotlib's `Axes._convert_dx`): the converted far end of
    /// each extent from the first finite origin less the first finite
    /// converted origin. When there is no origin, or an extent does not
    /// add to it or its far end fails to convert, the extents are
    /// converted by themselves instead.
    ///
    /// Numbers round through their origin as in matplotlib: a bar one
    /// unit high from 1e16 is no height at all.
    fn convert_dx(&self, deltas: &AxisDeltas, origin: &AxisValues, converted: &[f64]) -> Vec<f64> {
        let (Some(start), Some(converted_start)) = (origin.first_finite(), first_finite(converted))
        else {
            return self.convert_deltas(deltas);
        };
        let ends: Option<Vec<f64>> = deltas
            .iter()
            .map(|delta| {
                let end = start.add_delta(&delta)?;
                self.try_convert_value(&end).ok()
            })
            .collect();
        match ends {
            Some(ends) => ends.into_iter().map(|end| end - converted_start).collect(),
            None => self.convert_deltas(deltas),
        }
    }

    /// The axis numbers of `start` and of the far end of the extent
    /// `delta` from it, which plotting methods placing spans compute as
    /// matplotlib's `broken_barh` does (`convert((x0, x0 + dx))`). An
    /// extent that does not add to its start, where matplotlib fails, is
    /// converted by itself and added to the converted start.
    fn convert_span(&self, start: AxisValue, delta: &AxisDelta) -> (f64, f64) {
        let end = start.add_delta(delta);
        let start_number = self.convert_one(start);
        let end_number = match end {
            Some(end) => self.convert_without_update(&end),
            None => start_number + one_number(self.convert_deltas(&delta.clone().into())),
        };
        (start_number, end_number)
    }

    /// Converts one plotting value as [`convert_values`](Self::convert_values)
    /// converts a list.
    fn convert_one(&self, value: AxisValue) -> f64 {
        one_number(self.convert_values(AxisValues::from(value)))
    }

    /// Converts `value` as an artist converts its own values (matplotlib's
    /// `Axis.convert_units`, which `convert_xunits` calls): a date or a
    /// user value without updating the axis, which only takes the
    /// converter of the value if it has none, without its tick helpers,
    /// label or unit. Category names register as when they are plotted. A
    /// failed conversion gives NaN and is reported when the figure next
    /// draws.
    fn convert_without_update(&self, value: &AxisValue) -> f64 {
        self.try_convert_value(value).unwrap_or_else(|error| {
            self.report(&error);
            f64::NAN
        })
    }

    /// Converts `value` as texts and annotations convert their positions,
    /// which matplotlib does only when drawing them, after the plotting
    /// calls that set the axis up: without changing the axis. Dates are
    /// their numbers, user values convert in the axis unit as
    /// [`convert_custom`](Self::convert_custom) does but without giving
    /// the axis their converter, and category names register as when they
    /// are plotted on an axis with categories and fail on one without
    /// (matplotlib's `StrCategoryConverter.convert`, which has no unit to
    /// add the name to there). A failed conversion gives NaN and is
    /// reported when the figure next draws.
    fn convert_when_drawn(&self, value: &AxisValue) -> f64 {
        let converted = match value {
            &AxisValue::Number(number) | &AxisValue::Date(number) => Ok(number),
            AxisValue::Category(name) => {
                if self.read_state(|state| state.categories().is_some()) {
                    Ok(self.convert_one(value.clone()))
                } else {
                    Err(ConversionError::NoCategories(name.clone()))
                }
            }
            AxisValue::Custom(values) => self.try_convert_custom_as_is(values).map(one_number),
        };
        converted.unwrap_or_else(|error| {
            self.report(&error);
            f64::NAN
        })
    }

    /// [`convert_without_update`](Self::convert_without_update), failing
    /// instead of reporting.
    fn try_convert_value(&self, value: &AxisValue) -> Result<f64, ConversionError> {
        match value {
            &AxisValue::Number(number) => Ok(number),
            &AxisValue::Date(number) => {
                self.update_state(|state| state.units_mut().converter_or(UnitConverter::Dates));
                Ok(number)
            }
            AxisValue::Category(_) => Ok(self.convert_one(value.clone())),
            AxisValue::Custom(values) => self.try_convert_custom(values).map(one_number),
        }
    }
}

/// The number one value converted to: NaN for a user value holding no
/// value at all, as for a value that fails to convert.
fn one_number(numbers: Vec<f64>) -> f64 {
    numbers.into_iter().next().unwrap_or(f64::NAN)
}
