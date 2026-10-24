//! The interface of user unit converters (matplotlib's
//! `ConversionInterface`, `AxisInfo` and `ConversionError`) and the link
//! from a type of plotting value to its converter (matplotlib's
//! `units.registry`).

use std::fmt;
use std::rc::Rc;

use super::{AxisDelta, AxisValue};
use crate::axes::Axis;
use crate::ticker::{Formatter, Locator};

/// A converter of plotting values of a user type to axis numbers
/// (matplotlib's `ConversionInterface`): implement it, and
/// [`Convertible`] for the values, to plot them like numbers, dates and
/// category names.
///
/// An axis converting such values picks a unit from the first values it
/// sees ([`default_units`](Self::default_units)) unless it has one, takes
/// its tick helpers, label and default limits from
/// [`axisinfo`](Self::axisinfo), and converts every value in its unit
/// ([`convert`](Self::convert)).
///
/// ```
/// use plt::axes::Axis;
/// use plt::prelude::*;
/// use plt::units::{AxisInfo, ConversionError, ConversionInterface, Convertible};
///
/// /// A length in metres or feet.
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// struct Length { value: f64, unit: Unit }
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Unit { Metres, Feet }
///
/// struct LengthConverter;
///
/// impl ConversionInterface for LengthConverter {
///     type Value = Length;
///     type Unit = Unit;
///
///     fn convert(&self, values: &[Length], unit: Option<&Unit>, _: &Axis)
///         -> Result<Vec<f64>, ConversionError> {
///         let metres_per = |unit| if unit == Unit::Feet { 0.3048 } else { 1.0 };
///         Ok(values.iter().map(|length| {
///             let to = unit.copied().unwrap_or(length.unit);
///             length.value * metres_per(length.unit) / metres_per(to)
///         }).collect())
///     }
///
///     fn axisinfo(&self, unit: Option<&Unit>, _: &Axis) -> Option<AxisInfo> {
///         let label = format!("{:?}", unit?).to_lowercase();
///         Some(AxisInfo { label: Some(label), ..AxisInfo::default() })
///     }
///
///     fn default_units(&self, values: &[Length], _: &Axis) -> Option<Unit> {
///         values.first().map(|length| length.unit)
///     }
/// }
///
/// impl Convertible for Length {
///     type Converter = LengthConverter;
///     fn converter() -> LengthConverter { LengthConverter }
/// }
///
/// let ax = Figure::new().subplot();
/// let feet = |value| Length { value, unit: Unit::Feet };
/// let line = ax.plot(vec![feet(0.0), feet(10.0)], [1.0, 2.0])?;
/// assert_eq!(ax.xaxis().label_text(), "feet");
/// ax.xaxis().set_units(Unit::Metres);
/// assert_eq!(line.xdata(), [0.0, 3.048]);
/// assert_eq!(ax.xaxis().label_text(), "metres");
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
pub trait ConversionInterface: 'static {
    /// The values converted (the type matplotlib registers the converter
    /// for).
    type Value;
    /// The units an axis of these values can be in (matplotlib's
    /// `Axis.units`).
    type Unit: PartialEq + fmt::Debug + 'static;

    /// The axis numbers of `values` in `unit`, one per value (`None` when
    /// the axis has no unit), on `axis` (matplotlib's `convert`).
    ///
    /// # Errors
    ///
    /// When the values cannot be expressed in `unit`; the axis reports it
    /// when its figure next draws, and the values become NaN.
    fn convert(
        &self,
        values: &[Self::Value],
        unit: Option<&Self::Unit>,
        axis: &Axis,
    ) -> Result<Vec<f64>, ConversionError>;

    /// The tick helpers, label and default limits of `axis` in `unit`,
    /// if the converter has any (matplotlib's `axisinfo`; the default has
    /// none).
    fn axisinfo(&self, unit: Option<&Self::Unit>, axis: &Axis) -> Option<AxisInfo> {
        let _ = (unit, axis);
        None
    }

    /// The unit `axis` takes from `values` when it has none yet
    /// (matplotlib's `default_units`; the default picks none).
    fn default_units(&self, values: &[Self::Value], axis: &Axis) -> Option<Self::Unit> {
        let _ = (values, axis);
        None
    }
}

/// A type of plotting values converted by a [`ConversionInterface`]
/// (matplotlib's registering a converter for a class in
/// `units.registry`): lists of such values, and single ones, are
/// [`AxisValues`](super::AxisValues) and [`AxisValue`](super::AxisValue)
/// that every plotting method taking those converts on its axis.
pub trait Convertible: Clone + PartialEq + fmt::Debug + 'static {
    /// The converter of these values.
    type Converter: ConversionInterface<Value = Self>;

    /// The converter an axis uses for these values unless another one of
    /// the same type was set on it ([`Axis::set_converter`]).
    fn converter() -> Self::Converter;

    /// This value moved by the extent `delta`, if extents of that kind
    /// add to it (Python's `value + delta`). An extent from a value, such
    /// as the height of a bar from its bottom or the width of a
    /// [`DataRectangle`](crate::patches::DataRectangle) from its corner,
    /// becomes axis numbers as the converted far end less the converted
    /// value (matplotlib's `Axes._convert_dx` and
    /// `Rectangle._convert_units`). The default adds nothing, so that
    /// extents from these values are converted by themselves, as
    /// `_convert_dx` falls back to.
    ///
    /// ```
    /// use plt::axes::{Axis, BarOptions};
    /// use plt::prelude::*;
    /// use plt::units::{AxisDelta, AxisValue, ConversionError, ConversionInterface, Convertible};
    ///
    /// /// A day of a year.
    /// #[derive(Clone, Copy, Debug, PartialEq)]
    /// struct Day(f64);
    ///
    /// /// A number of days.
    /// #[derive(Clone, Copy, Debug, PartialEq)]
    /// struct Days(f64);
    ///
    /// struct DayConverter;
    ///
    /// impl ConversionInterface for DayConverter {
    ///     type Value = Day;
    ///     type Unit = ();
    ///     fn convert(&self, days: &[Day], _: Option<&()>, _: &Axis)
    ///         -> Result<Vec<f64>, ConversionError> {
    ///         Ok(days.iter().map(|day| day.0).collect())
    ///     }
    /// }
    ///
    /// impl Convertible for Day {
    ///     type Converter = DayConverter;
    ///     fn converter() -> DayConverter { DayConverter }
    ///     fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
    ///         delta.downcast_ref::<Days>().map(|days| Day(self.0 + days.0).into())
    ///     }
    /// }
    ///
    /// struct DaysConverter;
    ///
    /// impl ConversionInterface for DaysConverter {
    ///     type Value = Days;
    ///     type Unit = ();
    ///     fn convert(&self, days: &[Days], _: Option<&()>, _: &Axis)
    ///         -> Result<Vec<f64>, ConversionError> {
    ///         Ok(days.iter().map(|days| days.0).collect())
    ///     }
    /// }
    ///
    /// impl Convertible for Days {
    ///     type Converter = DaysConverter;
    ///     fn converter() -> DaysConverter { DaysConverter }
    /// }
    ///
    /// let ax = Figure::new().subplot();
    /// let from_day_10 = BarOptions { base: vec![Day(10.0)].into(), ..BarOptions::default() };
    /// let bars = ax.bar([1.0, 2.0], vec![Days(3.0), Days(5.0)], from_day_10)?;
    /// assert_eq!(bars.tops(), [13.0, 15.0]);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        let _ = delta;
        None
    }
}

/// The unit type of the converter of values of type `T`.
pub(crate) type UnitOf<T> = <<T as Convertible>::Converter as ConversionInterface>::Unit;

/// What a converter says about an axis in a unit (matplotlib's
/// `units.AxisInfo`; [`ticker::AxisInfo`](crate::ticker::AxisInfo) is
/// the axis state tick helpers read). The tick helpers replace only those
/// still at their defaults, the label only a label that was not set, and
/// the default limits, in axis numbers, become the view while neither
/// data nor limits changed it.
#[derive(Clone, Debug, Default)]
pub struct AxisInfo {
    /// The locator of the major ticks.
    pub major_locator: Option<Rc<dyn Locator>>,
    /// The locator of the minor ticks.
    pub minor_locator: Option<Rc<dyn Locator>>,
    /// The formatter of the major ticks.
    pub major_formatter: Option<Rc<dyn Formatter>>,
    /// The formatter of the minor ticks.
    pub minor_formatter: Option<Rc<dyn Formatter>>,
    /// The axis label.
    pub label: Option<String>,
    /// The view of the axis before any data is plotted, in axis numbers.
    pub default_limits: Option<(f64, f64)>,
}

/// Why values could not be converted to axis numbers (matplotlib's
/// `ConversionError`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionError {
    /// The converter could not convert the values, for the reason given.
    Failed(String),
    /// The axis unit is not a unit of the converter.
    ForeignUnit {
        /// The unit, as `Debug` shows it.
        unit: String,
        /// The type of the converter.
        converter: &'static str,
    },
    /// The converter gave another number of numbers than it was given
    /// values.
    Count {
        /// The number of values.
        values: usize,
        /// The number of numbers.
        numbers: usize,
    },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to convert values to axis units: ")?;
        match self {
            Self::Failed(reason) => f.write_str(reason),
            Self::ForeignUnit { unit, converter } => {
                write!(f, "{unit} is not a unit of {converter}")
            }
            Self::Count { values, numbers } => {
                write!(f, "{values} values converted to {numbers} numbers")
            }
        }
    }
}

impl std::error::Error for ConversionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_say_what_failed() {
        let failed = ConversionError::Failed("feet are not seconds".into());
        assert_eq!(
            failed.to_string(),
            "failed to convert values to axis units: feet are not seconds"
        );
        let count = ConversionError::Count {
            values: 2,
            numbers: 1,
        };
        assert!(
            count
                .to_string()
                .ends_with("2 values converted to 1 numbers")
        );
    }
}
