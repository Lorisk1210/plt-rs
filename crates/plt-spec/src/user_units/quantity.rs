//! `test_units.Quantity` and the `quantity_converter` fixture: magnitudes
//! in units of time and length, converted by a fixed table of factors.

use plt::axes::Axis;
use plt::units::{ConversionError, ConversionInterface, Convertible, UnitAxisInfo};

/// A unit of a [`Quantity`] (the test's unit strings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuantityUnit {
    Hours,
    Minutes,
    Seconds,
    Miles,
    Feet,
    Inches,
    Meters,
}

impl QuantityUnit {
    /// The test's name of the unit, which labels axes.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Hours => "hours",
            Self::Minutes => "minutes",
            Self::Seconds => "seconds",
            Self::Miles => "miles",
            Self::Feet => "feet",
            Self::Inches => "inches",
            Self::Meters => "meters",
        }
    }

    /// The unit named `name`.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        [
            Self::Hours,
            Self::Minutes,
            Self::Seconds,
            Self::Miles,
            Self::Feet,
            Self::Inches,
            Self::Meters,
        ]
        .into_iter()
        .find(|unit| unit.name() == name)
    }

    /// What one of this unit is in `to`, if the test's table says
    /// (`Quantity.to`'s `factors`).
    fn factor(self, to: Self) -> Option<f64> {
        use QuantityUnit::{Feet, Hours, Inches, Miles, Minutes, Seconds};
        match (self, to) {
            (Hours, Seconds) => Some(3600.0),
            (Minutes, Hours) => Some(1.0 / 60.0),
            (Minutes, Seconds) => Some(60.0),
            (Feet, Miles) => Some(1.0 / 5280.0),
            (Feet, Inches) => Some(12.0),
            (Miles, Inches) => Some(12.0 * 5280.0),
            _ => None,
        }
    }
}

/// A magnitude in a unit (`test_units.Quantity` holding one number).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quantity {
    /// The number.
    pub(crate) magnitude: f64,
    /// Its unit.
    pub(crate) units: QuantityUnit,
}

impl Quantity {
    /// `magnitude` in `units`.
    pub(crate) fn new(magnitude: f64, units: QuantityUnit) -> Self {
        Self { magnitude, units }
    }

    /// The quantity in `units` (`Quantity.to`).
    fn to(self, units: QuantityUnit) -> Result<Self, ConversionError> {
        if self.units == units {
            return Ok(self);
        }
        let factor = self.units.factor(units).ok_or_else(|| {
            ConversionError::Failed(format!(
                "no factor from {} to {}",
                self.units.name(),
                units.name()
            ))
        })?;
        Ok(Self::new(factor * self.magnitude, units))
    }
}

/// Magnitudes in one unit (`Quantity(array, units)`).
pub(crate) fn quantities(
    magnitudes: impl IntoIterator<Item = f64>,
    units: QuantityUnit,
) -> Vec<Quantity> {
    magnitudes
        .into_iter()
        .map(|magnitude| Quantity::new(magnitude, units))
        .collect()
}

/// The `quantity_converter` fixture: labels axes with their unit and
/// views 0 to 100 before any data.
pub(crate) struct QuantityConverter;

impl ConversionInterface for QuantityConverter {
    type Value = Quantity;
    type Unit = QuantityUnit;

    fn convert(
        &self,
        values: &[Quantity],
        unit: Option<&QuantityUnit>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        values
            .iter()
            .map(|value| match unit {
                Some(&unit) => value.to(unit).map(|value| value.magnitude),
                None => Ok(value.magnitude),
            })
            .collect()
    }

    fn axisinfo(&self, unit: Option<&QuantityUnit>, _: &Axis) -> Option<UnitAxisInfo> {
        Some(UnitAxisInfo {
            label: unit.map(|unit| unit.name().to_owned()),
            default_limits: Some((0.0, 100.0)),
            ..UnitAxisInfo::default()
        })
    }

    fn default_units(&self, values: &[Quantity], _: &Axis) -> Option<QuantityUnit> {
        values.first().map(|value| value.units)
    }
}

impl Convertible for Quantity {
    type Converter = QuantityConverter;

    fn converter() -> QuantityConverter {
        QuantityConverter
    }
}
