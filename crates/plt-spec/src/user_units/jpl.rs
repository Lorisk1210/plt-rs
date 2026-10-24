//! The parts of `matplotlib.testing.jpl_units` the units tests use:
//! `Duration`, `Epoch` and `UnitDbl` with the `EpochConverter`,
//! `UnitDblConverter` and `UnitDblFormatter` that `jpl_units.register()`
//! installs. The tests keep to ephemeris time and kilometres, so the other
//! frames and units (and the converters' cases for polar axes) are left
//! out.

use std::ops::{Add, Mul, Neg, Sub};
use std::rc::Rc;

use plt::axes::Axis;
use plt::dates::{AutoDateFormatter, AutoDateLocator, DateTime, get_epoch};
use plt::ticker::pyfmt::FormatSpec;
use plt::ticker::{
    FormattedTicks, Formatter, OffsetMode, ScalarFormatter, TickContext, TickerError,
    TickerSettings,
};
use plt::units::{AxisInfo, ConversionError, ConversionInterface, Convertible};

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

/// `jpl_units.EpochConverter` for epochs: date numbers, ticked as dates
/// and labelled with the frame.
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

    fn axisinfo(&self, unit: Option<&EphemerisTime>, _: &Axis) -> Option<AxisInfo> {
        Some(AxisInfo {
            major_locator: Some(Rc::new(AutoDateLocator::new())),
            major_formatter: Some(Rc::new(AutoDateFormatter::new(AutoDateLocator::new()))),
            label: unit.map(|_| "ET".to_owned()),
            ..AxisInfo::default()
        })
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
}

/// Kilometres, the unit of the tests' distances (`"km"`, the default unit
/// of distances).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Kilometres;

/// A distance in kilometres (`jpl_units.UnitDbl(value, "km")`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UnitDbl {
    km: f64,
}

impl UnitDbl {
    /// `km` kilometres (`km * units.km`).
    pub(crate) fn km(km: f64) -> Self {
        Self { km }
    }
}

/// `jpl_units.UnitDblConverter`: numbers in the axis unit, labelled with
/// it and formatted by a [`UnitDblFormatter`].
pub(crate) struct UnitDblConverter;

impl ConversionInterface for UnitDblConverter {
    type Value = UnitDbl;
    type Unit = Kilometres;

    fn convert(
        &self,
        values: &[UnitDbl],
        _: Option<&Kilometres>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        Ok(values.iter().map(|value| value.km).collect())
    }

    fn axisinfo(&self, unit: Option<&Kilometres>, _: &Axis) -> Option<AxisInfo> {
        Some(AxisInfo {
            major_formatter: Some(Rc::new(UnitDblFormatter::new())),
            label: unit.map(|_| "km".to_owned()),
            ..AxisInfo::default()
        })
    }

    fn default_units(&self, values: &[UnitDbl], _: &Axis) -> Option<Kilometres> {
        values.first().map(|_| Kilometres)
    }
}

impl Convertible for UnitDbl {
    type Converter = UnitDblConverter;

    fn converter() -> UnitDblConverter {
        UnitDblConverter
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
