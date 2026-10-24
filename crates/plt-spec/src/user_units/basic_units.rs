//! The gallery's `basic_units` module: numbers tagged with units of
//! length, angle and time (`TaggedValue`, `BasicUnit`), their conversions
//! and `BasicUnitConverter`, which labels axes with the full name of their
//! unit and ticks angles in multiples of π/2 or in degrees.

use std::f64::consts::PI;
use std::ops::{Add, Mul};
use std::rc::Rc;

use plt::axes::Axis;
use plt::ticker::{AutoLocator, FormatStrFormatter, FuncFormatter, MultipleLocator};
use plt::units::{
    AxisDelta, AxisValue, ConversionError, ConversionInterface, Convertible, UnitAxisInfo,
};

/// A unit of `basic_units`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BasicUnit {
    Centimeters,
    Inches,
    Radians,
    Degrees,
    Seconds,
    Hertz,
    Minutes,
}

impl BasicUnit {
    /// The full name, which labels axes (`BasicUnit.fullname`).
    fn fullname(self) -> &'static str {
        match self {
            Self::Centimeters => "centimeters",
            Self::Inches => "inches",
            Self::Radians => "radians",
            Self::Degrees => "degrees",
            Self::Seconds => "seconds",
            Self::Hertz => "Hertz",
            Self::Minutes => "minutes",
        }
    }

    /// `value` of this unit in `to`, if `basic_units` converts between
    /// them (`BasicUnit.convert_value_to`).
    fn convert_value_to(self, value: f64, to: Self) -> Option<f64> {
        use BasicUnit::{Centimeters, Degrees, Hertz, Inches, Minutes, Radians, Seconds};
        match (self, to) {
            (Inches, Centimeters) => Some(value * 2.54),
            (Centimeters, Inches) => Some(value * (1.0 / 2.54)),
            (Radians, Degrees) => Some(value * (180.0 / PI)),
            (Degrees, Radians) => Some(value * (PI / 180.0)),
            (Seconds, Hertz) => Some(1.0 / value),
            (Seconds, Minutes) => Some(value * (1.0 / 60.0)),
            _ => None,
        }
    }
}

/// `basic_units.cm`.
pub(crate) const CM: BasicUnit = BasicUnit::Centimeters;
/// `basic_units.inch`.
pub(crate) const INCH: BasicUnit = BasicUnit::Inches;
/// `basic_units.radians`.
pub(crate) const RADIANS: BasicUnit = BasicUnit::Radians;
/// `basic_units.degrees`.
pub(crate) const DEGREES: BasicUnit = BasicUnit::Degrees;
/// `basic_units.secs`.
pub(crate) const SECS: BasicUnit = BasicUnit::Seconds;
/// `basic_units.hertz`.
pub(crate) const HERTZ: BasicUnit = BasicUnit::Hertz;
/// `basic_units.minutes`.
pub(crate) const MINUTES: BasicUnit = BasicUnit::Minutes;

/// A number in a unit (`basic_units.TaggedValue` of one number); masked
/// values are NaN.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TaggedValue {
    value: f64,
    unit: BasicUnit,
}

impl TaggedValue {
    /// The value in `unit` (`TaggedValue.convert_to`).
    fn convert_to(self, unit: BasicUnit) -> Result<Self, ConversionError> {
        if unit == self.unit {
            return Ok(self);
        }
        let value = self
            .unit
            .convert_value_to(self.value, unit)
            .ok_or_else(|| {
                ConversionError::Failed(format!(
                    "no conversion from {} to {}",
                    self.unit.fullname(),
                    unit.fullname()
                ))
            })?;
        Ok(Self { value, unit })
    }
}

/// `value * unit` (`BasicUnit.__rmul__`).
impl Mul<BasicUnit> for f64 {
    type Output = TaggedValue;

    fn mul(self, unit: BasicUnit) -> TaggedValue {
        TaggedValue { value: self, unit }
    }
}

/// The sum in the unit of the left value, the right one converted to it
/// (`TaggedValue.__add__`); values `basic_units` cannot add sum to NaN.
impl Add for TaggedValue {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let rhs = rhs.convert_to(self.unit).map_or(f64::NAN, |rhs| rhs.value);
        (self.value + rhs) * self.unit
    }
}

/// `basic_units.cos`: the cosines of angles.
pub(crate) fn cos(angles: &[TaggedValue]) -> Result<Vec<f64>, ConversionError> {
    angles
        .iter()
        .map(|angle| angle.convert_to(RADIANS).map(|angle| angle.value.cos()))
        .collect()
}

/// `basic_units.rad_fn`: multiples of π/2 in math text.
fn rad_fn(x: f64) -> String {
    let half_turns = (x / PI) * 2.0;
    // `int()` truncates towards zero.
    let n = if x >= 0.0 {
        (half_turns + 0.25).trunc()
    } else {
        (half_turns - 0.25).trunc()
    } as i64;
    match n {
        0 => "0".to_owned(),
        1 => r"$\pi/2$".to_owned(),
        2 => r"$\pi$".to_owned(),
        -1 => r"$-\pi/2$".to_owned(),
        -2 => r"$-\pi$".to_owned(),
        n if n % 2 == 0 => format!(r"${}\pi$", n.div_euclid(2)),
        n => format!(r"${n}\pi/2$"),
    }
}

/// `basic_units.BasicUnitConverter`.
pub(crate) struct BasicUnitConverter;

impl ConversionInterface for BasicUnitConverter {
    type Value = TaggedValue;
    type Unit = BasicUnit;

    fn convert(
        &self,
        values: &[TaggedValue],
        unit: Option<&BasicUnit>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        values
            .iter()
            .map(|value| match unit {
                Some(&unit) if !value.value.is_nan() => {
                    value.convert_to(unit).map(|value| value.value)
                }
                _ => Ok(value.value),
            })
            .collect()
    }

    fn axisinfo(&self, unit: Option<&BasicUnit>, _: &Axis) -> Option<UnitAxisInfo> {
        let unit = *unit?;
        let label = Some(unit.fullname().to_owned());
        Some(match unit {
            BasicUnit::Radians => UnitAxisInfo {
                majloc: Some(Rc::new(MultipleLocator::new(PI / 2.0).ok()?)),
                majfmt: Some(Rc::new(FuncFormatter::new(|x, _| rad_fn(x)))),
                label,
                ..UnitAxisInfo::default()
            },
            BasicUnit::Degrees => UnitAxisInfo {
                majloc: Some(Rc::new(AutoLocator::new())),
                majfmt: Some(Rc::new(FormatStrFormatter::new(r"$%i^\circ$").ok()?)),
                label,
                ..UnitAxisInfo::default()
            },
            _ => UnitAxisInfo {
                label,
                ..UnitAxisInfo::default()
            },
        })
    }

    fn default_units(&self, values: &[TaggedValue], _: &Axis) -> Option<BasicUnit> {
        values.first().map(|value| value.unit)
    }
}

impl Convertible for TaggedValue {
    type Converter = BasicUnitConverter;

    fn converter() -> BasicUnitConverter {
        BasicUnitConverter
    }

    /// The sum with a value that converts to this one's unit
    /// (`TaggedValue.__add__`, which refuses others).
    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        let delta = delta.downcast_ref::<Self>()?.convert_to(self.unit).ok()?;
        Some(((self.value + delta.value) * self.unit).into())
    }
}
