//! The parts of `matplotlib.testing.jpl_units` the tests use: `Duration`,
//! `Epoch` and `UnitDbl` with the converters `jpl_units.register()`
//! installs (`EpochConverter` for epochs and durations,
//! `UnitDblConverter`) and the `UnitDblFormatter`. The tests keep to
//! ephemeris time and to metres, kilometres, miles, radians, degrees,
//! seconds and hours, so the UTC frame and minutes are left out.

use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use plt::axes::{Axis, ProjectionKind};
use plt::dates::{AutoDateFormatter, AutoDateLocator, DateTime, get_epoch};
use plt::polar::ThetaFormatter;
use plt::ticker::pyfmt::FormatSpec;
use plt::ticker::{
    FormattedTicks, Formatter, OffsetMode, ScalarFormatter, TickContext, TickerError,
    TickerSettings,
};
use plt::units::{
    AxisDelta, AxisValue, ConversionError, ConversionInterface, Convertible, UnitAxisInfo,
};

/// Seconds in a day.
const SECONDS_PER_DAY: f64 = 86_400.0;

/// The Julian date of matplotlib's date 0 (`EpochConverter.jdRef`).
const JD_REF: f64 = 1_721_425.5;

/// Ephemeris time, the frame of the tests' epochs and durations (`"ET"`),
/// and the unit of axes of epochs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EphemerisTime;

/// A length of time in ephemeris time (`jpl_units.Duration("ET", ...)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Duration {
    seconds: f64,
}

impl Duration {
    /// `seconds` long.
    pub(crate) fn seconds(seconds: f64) -> Self {
        Self { seconds }
    }
}

impl Neg for Duration {
    type Output = Self;

    fn neg(self) -> Self {
        Self::seconds(-self.seconds)
    }
}

/// `scale * duration` (`Duration.__rmul__`).
impl Mul<Duration> for f64 {
    type Output = Duration;

    fn mul(self, duration: Duration) -> Duration {
        Duration::seconds(duration.seconds * self)
    }
}

/// A moment in ephemeris time: whole Julian days and the seconds past
/// them (`jpl_units.Epoch("ET", ...)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Epoch {
    jd: f64,
    seconds: f64,
}

impl Epoch {
    /// `seconds` past Julian date `jd`, normalised to fewer seconds than a
    /// day (`Epoch("ET", sec=..., jd=...)`).
    fn from_seconds(seconds: f64, jd: f64) -> Self {
        let days = (seconds / SECONDS_PER_DAY).floor();
        Self {
            jd: jd + days,
            seconds: seconds - days * SECONDS_PER_DAY,
        }
    }

    /// The moment of `date` (`Epoch("ET", dt=...)`).
    pub(crate) fn from_date(date: DateTime) -> Self {
        let jd = get_epoch().date2num(date) + JD_REF;
        let whole = jd.floor();
        Self {
            jd: whole,
            seconds: (jd - whole) * SECONDS_PER_DAY,
        }
    }

    /// The Julian date (`Epoch.julianDate("ET")`).
    fn julian_date(self) -> f64 {
        self.jd + self.seconds / SECONDS_PER_DAY
    }
}

impl Add<Duration> for Epoch {
    type Output = Self;

    fn add(self, duration: Duration) -> Self {
        Self::from_seconds(self.seconds + duration.seconds, self.jd)
    }
}

impl Sub<Duration> for Epoch {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self {
        self + -duration
    }
}

/// What `EpochConverter.axisinfo` gives every axis of epochs or
/// durations: date ticks, labelled with the frame.
fn epoch_axisinfo(unit: Option<&EphemerisTime>) -> UnitAxisInfo {
    UnitAxisInfo {
        majloc: Some(Rc::new(AutoDateLocator::new())),
        majfmt: Some(Rc::new(AutoDateFormatter::new(AutoDateLocator::new()))),
        label: unit.map(|_| "ET".to_owned()),
        ..UnitAxisInfo::default()
    }
}

/// `jpl_units.EpochConverter` for epochs: date numbers.
pub(crate) struct EpochConverter;

impl ConversionInterface for EpochConverter {
    type Value = Epoch;
    type Unit = EphemerisTime;

    fn convert(
        &self,
        values: &[Epoch],
        _: Option<&EphemerisTime>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        Ok(values
            .iter()
            .map(|epoch| epoch.julian_date() - JD_REF)
            .collect())
    }

    fn axisinfo(&self, unit: Option<&EphemerisTime>, _: &Axis) -> Option<UnitAxisInfo> {
        Some(epoch_axisinfo(unit))
    }

    fn default_units(&self, values: &[Epoch], _: &Axis) -> Option<EphemerisTime> {
        values.first().map(|_| EphemerisTime)
    }
}

impl Convertible for Epoch {
    type Converter = EpochConverter;

    fn converter() -> EpochConverter {
        EpochConverter
    }

    /// `epoch + duration` (`Epoch.__add__`); nothing else adds to epochs.
    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        delta
            .downcast_ref::<Duration>()
            .map(|&duration| (*self + duration).into())
    }
}

/// `jpl_units.EpochConverter` for durations, which `jpl_units.register()`
/// installs for them too: days.
pub(crate) struct DurationConverter;

impl ConversionInterface for DurationConverter {
    type Value = Duration;
    type Unit = EphemerisTime;

    fn convert(
        &self,
        values: &[Duration],
        _: Option<&EphemerisTime>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        Ok(values
            .iter()
            .map(|duration| duration.seconds / SECONDS_PER_DAY)
            .collect())
    }

    fn axisinfo(&self, unit: Option<&EphemerisTime>, _: &Axis) -> Option<UnitAxisInfo> {
        Some(epoch_axisinfo(unit))
    }

    fn default_units(&self, values: &[Duration], _: &Axis) -> Option<EphemerisTime> {
        values.first().map(|_| EphemerisTime)
    }
}

impl Convertible for Duration {
    type Converter = DurationConverter;

    fn converter() -> DurationConverter {
        DurationConverter
    }
}

/// A unit a [`UnitDbl`] can be given in and converted to
/// (`UnitDbl.allowed`); its name labels axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnitName {
    Metres,
    Kilometres,
    Miles,
    Radians,
    Degrees,
    Seconds,
    Hours,
}

impl UnitName {
    /// What one of this unit is in its base unit, and the base unit
    /// (`UnitDbl.allowed[unit]`).
    fn scale(self) -> (f64, Self) {
        match self {
            Self::Metres => (0.001, Self::Kilometres),
            Self::Kilometres => (1.0, Self::Kilometres),
            Self::Miles => (1.609_344, Self::Kilometres),
            Self::Radians => (1.0, Self::Radians),
            Self::Degrees => (1.745_329_251_994_33e-2, Self::Radians),
            Self::Seconds => (1.0, Self::Seconds),
            Self::Hours => (3600.0, Self::Seconds),
        }
    }

    /// The unit's name (`"km"`, `"deg"`, ...).
    fn name(self) -> &'static str {
        match self {
            Self::Metres => "m",
            Self::Kilometres => "km",
            Self::Miles => "mile",
            Self::Radians => "rad",
            Self::Degrees => "deg",
            Self::Seconds => "sec",
            Self::Hours => "hour",
        }
    }
}

/// A distance, angle or time in a unit (`jpl_units.UnitDbl`), held in the
/// base unit of its kind: kilometres, radians or seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UnitDbl {
    value: f64,
    unit: UnitName,
}

impl UnitDbl {
    /// `value` in `unit` (`UnitDbl(value, unit)`).
    pub(crate) fn new(value: f64, unit: UnitName) -> Self {
        let (factor, base) = unit.scale();
        Self {
            value: value * factor,
            unit: base,
        }
    }

    /// The value in `unit`, of the same kind (`UnitDbl.convert`).
    pub(crate) fn convert(self, unit: UnitName) -> Result<f64, ConversionError> {
        if self.unit == unit {
            return Ok(self.value);
        }
        let (factor, base) = unit.scale();
        if base == self.unit {
            Ok(self.value / factor)
        } else {
            Err(ConversionError::Failed(format!(
                "cannot convert {} to {}",
                self.unit.name(),
                unit.name()
            )))
        }
    }

    /// The unit values of this kind are plotted in unless the axis has one
    /// (`UnitDblConverter.defaults`).
    fn default_unit(self) -> UnitName {
        match self.unit {
            UnitName::Radians => UnitName::Degrees,
            unit => unit,
        }
    }

    /// Whether the value is an angle (`UnitDbl.type() == "angle"`).
    fn is_angle(self) -> bool {
        self.unit == UnitName::Radians
    }
}

/// `scale * value` (`UnitDbl.__rmul__`).
impl Mul<UnitDbl> for f64 {
    type Output = UnitDbl;

    fn mul(self, value: UnitDbl) -> UnitDbl {
        UnitDbl {
            value: value.value * self,
            unit: value.unit,
        }
    }
}

/// `jpl_units.UnitDblConverter`: numbers in the axis unit (angles in
/// radians on polar axes), labelled with the unit and formatted by a
/// [`UnitDblFormatter`], or by polar axes' `ThetaFormatter` for degrees
/// on polar axes.
pub(crate) struct UnitDblConverter;

/// Whether `axis` belongs to polar axes (`isinstance(axis.axes,
/// PolarAxes)`).
fn on_polar_axes(axis: &Axis) -> bool {
    axis.axes().projection() == ProjectionKind::Polar
}

impl ConversionInterface for UnitDblConverter {
    type Value = UnitDbl;
    type Unit = UnitName;

    fn convert(
        &self,
        values: &[UnitDbl],
        unit: Option<&UnitName>,
        axis: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        let polar = on_polar_axes(axis);
        values
            .iter()
            .map(|&value| {
                if polar && value.is_angle() {
                    value.convert(UnitName::Radians)
                } else {
                    value.convert(unit.copied().unwrap_or_else(|| value.default_unit()))
                }
            })
            .collect()
    }

    fn axisinfo(&self, unit: Option<&UnitName>, axis: &Axis) -> Option<UnitAxisInfo> {
        let major_formatter: Rc<dyn Formatter> =
            if unit == Some(&UnitName::Degrees) && on_polar_axes(axis) {
                Rc::new(ThetaFormatter)
            } else {
                Rc::new(UnitDblFormatter::new())
            };
        Some(UnitAxisInfo {
            majfmt: Some(major_formatter),
            label: unit.map(|unit| unit.name().to_owned()),
            ..UnitAxisInfo::default()
        })
    }

    fn default_units(&self, values: &[UnitDbl], _: &Axis) -> Option<UnitName> {
        values.first().map(|value| value.default_unit())
    }
}

impl Convertible for UnitDbl {
    type Converter = UnitDblConverter;

    fn converter() -> UnitDblConverter {
        UnitDblConverter
    }

    /// The sum of two values of one unit (`UnitDbl.__add__`).
    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        let delta = delta.downcast_ref::<Self>()?;
        (delta.unit == self.unit).then(|| {
            Self {
                value: self.value + delta.value,
                unit: self.unit,
            }
            .into()
        })
    }
}

/// `jpl_units.UnitDblFormatter`: a `ScalarFormatter` without offset whose
/// labels are the values to twelve significant digits (`f"{x:.12}"`).
#[derive(Clone, Debug)]
pub(crate) struct UnitDblFormatter {
    scalar: ScalarFormatter,
    digits: FormatSpec,
}

impl UnitDblFormatter {
    /// A formatter set up from the current rc parameters.
    fn new() -> Self {
        let settings = TickerSettings::from(&*plt::rc::params());
        Self {
            scalar: ScalarFormatter::from_settings(&settings).with_use_offset(OffsetMode::Off),
            digits: FormatSpec::parse(".12").expect("a valid format spec"),
        }
    }

    /// `f"{value:.12}"`.
    fn label(&self, value: f64) -> Result<String, TickerError> {
        self.digits.format_float(value).map_err(TickerError::Format)
    }
}

impl Formatter for UnitDblFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let offset = self.scalar.format_ticks(locs, ctx)?.offset;
        let labels = locs
            .iter()
            .map(|&value| self.label(value))
            .collect::<Result<_, _>>()?;
        Ok(FormattedTicks { labels, offset })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        self.label(value)
    }
}
