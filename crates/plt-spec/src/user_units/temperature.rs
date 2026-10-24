//! The spec's `Temperature` (`spec_support.units`): degrees Celsius
//! plotted in degrees Celsius, degrees Fahrenheit or kelvin, which add and
//! scale like numbers, so that temperatures serve as their own error
//! sizes, and convert with an offset.

use plt::axes::Axis;
use plt::units::{
    AxisDelta, AxisValue, ConversionError, ConversionInterface, Convertible, UnitAxisInfo,
};

/// The unit of an axis of temperatures (`"C"`, `"F"` or `"K"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

impl TemperatureUnit {
    /// The unit's name, which labels axes.
    fn name(self) -> &'static str {
        match self {
            Self::Celsius => "C",
            Self::Fahrenheit => "F",
            Self::Kelvin => "K",
        }
    }
}

/// A temperature in degrees Celsius (`Temperature`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Temperature(pub(crate) f64);

impl Temperature {
    /// The temperature in `unit` (`Temperature.to`).
    fn to(self, unit: TemperatureUnit) -> f64 {
        match unit {
            TemperatureUnit::Celsius => self.0,
            TemperatureUnit::Fahrenheit => self.0 * 9.0 / 5.0 + 32.0,
            TemperatureUnit::Kelvin => self.0 + 273.15,
        }
    }
}

/// `TemperatureConverter`: temperatures in the axis unit, Celsius
/// without one, labelled with the unit.
pub(crate) struct TemperatureConverter;

impl ConversionInterface for TemperatureConverter {
    type Value = Temperature;
    type Unit = TemperatureUnit;

    fn convert(
        &self,
        values: &[Temperature],
        unit: Option<&TemperatureUnit>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        let unit = unit.copied().unwrap_or(TemperatureUnit::Celsius);
        Ok(values.iter().map(|value| value.to(unit)).collect())
    }

    fn axisinfo(&self, unit: Option<&TemperatureUnit>, _: &Axis) -> Option<UnitAxisInfo> {
        let name = unit.map_or("None", |unit| unit.name());
        Some(UnitAxisInfo {
            label: Some(format!("temperature ({name})")),
            ..UnitAxisInfo::default()
        })
    }

    fn default_units(&self, _: &[Temperature], _: &Axis) -> Option<TemperatureUnit> {
        Some(TemperatureUnit::Celsius)
    }
}

impl Convertible for Temperature {
    type Converter = TemperatureConverter;

    fn converter() -> TemperatureConverter {
        TemperatureConverter
    }

    /// `temperature + temperature` (`Temperature.__add__`).
    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        delta
            .downcast_ref::<Self>()
            .map(|delta| Self(self.0 + delta.0).into())
    }

    /// `factor * temperature` (`Temperature.__rmul__`).
    fn scale(&self, factor: i64) -> Option<Self> {
        Some(Self(self.0 * factor as f64))
    }
}
