//! Matplotlib date numbers: days since an epoch, as floats.
//!
//! [`Epoch`] does the arithmetic; the free functions use the process-wide
//! epoch managed by [`get_epoch`] / [`set_epoch`], exactly like
//! `matplotlib.dates`. The epoch is latched the first time it is read, after
//! which it can no longer change.

use std::fmt;
use std::sync::{Mutex, PoisonError};

use super::datetime::{MAX_YEAR, MIN_YEAR};
use super::iso::Datetime64;
use super::timedelta::{MICROS_PER_DAY, MICROS_PER_SECOND};
use super::tz::{TimeZone, ZonedDateTime};
use super::{Date, DateError, DateTime, TimeDelta, calendar};
use crate::numeric::linspace;

/// Seconds per day.
const SECONDS_PER_DAY: f64 = 86_400.0;
/// Microseconds per day as a float (matplotlib's `MUSECONDS_PER_DAY`).
pub(crate) const MICROSECONDS_PER_DAY: f64 = 86_400e6;
/// Beyond this many days from the epoch, `num2date` rounds to 20 µs.
const PRECISE_DAYS: f64 = 70.0 * 365.0;

/// The origin of Matplotlib date numbers (`rcParams["date.epoch"]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Epoch {
    text: String,
    instant: Datetime64,
}

impl Default for Epoch {
    /// `1970-01-01T00:00:00`, matplotlib's default since 3.3.
    fn default() -> Self {
        Self {
            text: "1970-01-01T00:00:00".to_owned(),
            instant: Datetime64::new(0, 0),
        }
    }
}

impl fmt::Display for Epoch {
    /// The epoch string as given.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl Epoch {
    /// An epoch from a numpy `datetime64` string (UTC, no zone), e.g.
    /// `"0000-12-31T00:00:00"` for matplotlib's pre-3.3 convention.
    pub fn parse(text: &str) -> Result<Self, DateError> {
        let instant = Datetime64::parse(text)?
            .ok_or_else(|| DateError::parse(text, "the epoch cannot be NaT"))?;
        Ok(Self {
            text: text.to_owned(),
            instant,
        })
    }

    /// The date number of a naive UTC date-time.
    ///
    /// ```
    /// use plt::dates::{DateTime, Epoch};
    /// let noon = DateTime::new(1970, 1, 1, 12, 0, 0, 0).unwrap();
    /// assert_eq!(Epoch::default().date2num(noon), 0.5);
    /// ```
    pub fn date2num(&self, utc: DateTime) -> f64 {
        let micros = utc.timestamp_micros();
        self.datetime64_to_num(Datetime64::new(
            micros.div_euclid(MICROS_PER_SECOND),
            (micros.rem_euclid(MICROS_PER_SECOND) * 1000) as u32,
        ))
    }

    /// The date number of a numpy `datetime64` value: whole seconds from
    /// the (second-truncated) epoch plus the nanosecond remainder, as
    /// matplotlib's `_dt64_to_ordinalf` computes it.
    pub fn datetime64_to_num(&self, value: Datetime64) -> f64 {
        let seconds = (value.seconds() - self.instant.seconds()) as f64;
        (seconds + f64::from(value.subsec_nanos()) / 1e9) / SECONDS_PER_DAY
    }

    /// The date-time in `tz` of date number `x` (matplotlib's
    /// `_from_ordinalf`): rounded to the microsecond, and to 20 µs when
    /// more than about 70 years from the epoch.
    pub fn num2date(&self, x: f64, tz: &TimeZone) -> Result<ZonedDateTime, DateError> {
        let scaled = (x * MICROSECONDS_PER_DAY).round_ties_even();
        if scaled.is_nan() {
            return Err(DateError::NonFinite(x));
        }
        if scaled.abs() >= 9.223_372_036_854_775e18 {
            return Err(DateError::Overflow);
        }
        let micros = self
            .instant
            .timestamp_micros()
            .checked_add(scaled as i64)
            .ok_or(DateError::Overflow)?;
        let lowest = calendar::days_from_civil(i64::from(MIN_YEAR), 1, 1) * MICROS_PER_DAY;
        let beyond = calendar::days_from_civil(i64::from(MAX_YEAR) + 1, 1, 1) * MICROS_PER_DAY;
        if !(lowest..beyond).contains(&micros) {
            return Err(DateError::DateNumberOutOfRange(x));
        }
        let zoned = ZonedDateTime::from_utc(DateTime::from_timestamp_micros(micros)?, tz.clone())?;
        if x.abs() <= PRECISE_DAYS {
            return Ok(zoned);
        }
        let local = zoned.local();
        let rounded = (f64::from(local.microsecond()) / 20.0).round_ties_even() as u32 * 20;
        if rounded == 1_000_000 {
            let next = local
                .with_microsecond(0)?
                .checked_add(TimeDelta::from_micros(MICROS_PER_SECOND))?;
            Ok(zoned.with_local(next))
        } else {
            Ok(zoned.replace_local(local.with_microsecond(rounded)?))
        }
    }
}

static EPOCH: Mutex<Option<Epoch>> = Mutex::new(None);

/// The epoch used by the module-level conversions (`dates.get_epoch`);
/// reading it latches `date.epoch` if none was set.
pub fn get_epoch() -> Epoch {
    let mut epoch = EPOCH.lock().unwrap_or_else(PoisonError::into_inner);
    epoch
        .get_or_insert_with(|| rc_epoch(&crate::rc::params().date_epoch))
        .clone()
}

/// The epoch an rc `date.epoch` value names, or the default when it does
/// not parse.
fn rc_epoch(text: &str) -> Epoch {
    Epoch::parse(text).unwrap_or_default()
}

/// Sets the epoch (`dates.set_epoch`); fails once the epoch has been used.
pub fn set_epoch(epoch: &str) -> Result<(), DateError> {
    let parsed = Epoch::parse(epoch)?;
    let mut current = EPOCH.lock().unwrap_or_else(PoisonError::into_inner);
    if current.is_some() {
        return Err(DateError::EpochAlreadySet);
    }
    *current = Some(parsed);
    Ok(())
}

/// Forgets the epoch so it can be set again (matplotlib's
/// `_reset_epoch_test_example`); meant for tests and examples.
pub fn reset_epoch() {
    *EPOCH.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

/// The date number of an aware date-time (`dates.date2num`).
pub fn date2num(dt: &ZonedDateTime) -> Result<f64, DateError> {
    Ok(get_epoch().date2num(dt.to_utc()?))
}

/// The date number of a numpy `datetime64` value; `NaT` (`None`) gives NaN.
pub fn datetime64_to_num(value: Option<Datetime64>) -> f64 {
    value.map_or(f64::NAN, |v| get_epoch().datetime64_to_num(v))
}

/// The date-time in `tz` of a date number (`dates.num2date`).
///
/// ```
/// use plt::dates::{TimeZone, num2date};
/// let dt = num2date(18_262.5, &TimeZone::Utc).unwrap();
/// assert_eq!(dt.isoformat(), "2020-01-01T12:00:00+00:00");
/// ```
pub fn num2date(x: f64, tz: &TimeZone) -> Result<ZonedDateTime, DateError> {
    get_epoch().num2date(x, tz)
}

/// A number of days as a duration (`dates.num2timedelta`).
pub fn num2timedelta(x: f64) -> Result<TimeDelta, DateError> {
    TimeDelta::from_days_f64(x)
}

/// Date numbers from `start` up to, not including, `end` in steps of
/// `delta` (`dates.drange`).
///
/// ```
/// use plt::dates::{TimeDelta, ZonedDateTime, drange};
/// let start = ZonedDateTime::parse_iso("2020-01-01").unwrap();
/// let end = ZonedDateTime::parse_iso("2020-01-02").unwrap();
/// let six_hours = TimeDelta::new(0, 6 * 3600, 0).unwrap();
/// assert_eq!(drange(&start, &end, six_hours).unwrap(), [18262.0, 18262.25, 18262.5, 18262.75]);
/// ```
pub fn drange(
    start: &ZonedDateTime,
    end: &ZonedDateTime,
    delta: TimeDelta,
) -> Result<Vec<f64>, DateError> {
    let first = date2num(start)?;
    let last = date2num(end)?;
    let step = delta.total_seconds() / SECONDS_PER_DAY;
    let steps = ((last - first) / step).ceil();
    if !steps.is_finite() || steps.abs() > 9e15 {
        return Err(DateError::InvalidArgument(
            "drange: the number of steps is not finite".to_owned(),
        ));
    }
    let mut count = steps as i64;
    let shift = delta.checked_mul(count).ok_or(DateError::Overflow)?;
    let mut interval_end = start.with_local(start.local().checked_add(shift)?);
    if interval_end.to_utc()? >= end.to_utc()? {
        interval_end = interval_end.with_local(interval_end.local().checked_sub(delta)?);
        count -= 1;
    }
    let samples = usize::try_from(count + 1).map_err(|_| {
        DateError::InvalidArgument(format!(
            "drange: number of samples, {}, is negative",
            count + 1
        ))
    })?;
    Ok(linspace(first, date2num(&interval_end)?, samples))
}

/// The date number of an ISO 8601 string (`dates.datestr2num`); strings
/// without an offset are UTC.
///
/// Only ISO 8601 is accepted, not the free-form syntax of
/// `dateutil.parser`.
pub fn datestr2num(text: &str) -> Result<f64, DateError> {
    date2num(&ZonedDateTime::parse_iso(text)?)
}

/// Midnight of `date` in UTC as a date number.
pub(crate) fn date_to_num(date: Date) -> f64 {
    get_epoch().date2num(DateTime::midnight(date))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(text: &str) -> DateTime {
        ZonedDateTime::parse_iso(text).unwrap().to_utc().unwrap()
    }

    #[test]
    fn date_numbers_match_matplotlib() {
        let epoch = Epoch::default();
        assert_eq!(
            epoch.date2num(utc("1969-12-31T23:59:59.999999")),
            -1.157_407_407_440_689_4e-11
        );
        assert_eq!(
            epoch.date2num(utc("2020-03-05T14:07:09.123456")),
            18_326.588_300_04
        );
        assert_eq!(
            epoch.date2num(utc("9999-12-31T23:59:59.999999")),
            2_932_897.0
        );
        assert_eq!(
            epoch.date2num(utc("2020-06-15T12:00:00+02:00")),
            18_428.416_666_666_668
        );
        let old = Epoch::parse("0000-12-31T00:00:00").unwrap();
        assert_eq!(old.date2num(utc("0001-01-01T00:00:00")), 1.0);
        let ns = Datetime64::parse("2020-01-01T00:00:00.123456789")
            .unwrap()
            .unwrap();
        assert_eq!(epoch.datetime64_to_num(ns), 18_262.000_001_428_896);
    }

    #[test]
    fn num2date_rounds_like_matplotlib() {
        let epoch = Epoch::default();
        let iso = |x: f64| epoch.num2date(x, &TimeZone::Utc).map(|d| d.isoformat());
        assert_eq!(iso(-0.5).unwrap(), "1969-12-31T12:00:00+00:00");
        assert_eq!(
            iso(0.6 / 86_400e6).unwrap(),
            "1970-01-01T00:00:00.000001+00:00"
        );
        assert_eq!(
            iso(-62_090.999_998_571_11).unwrap(),
            "1800-01-01T00:00:00.123460+00:00"
        );
        assert_eq!(
            iso(2_932_896.999_999_9).unwrap(),
            "9999-12-31T23:59:59.991360+00:00"
        );
        assert_eq!(
            iso(-719_163.0),
            Err(DateError::DateNumberOutOfRange(-719_163.0))
        );
        assert_eq!(
            iso(2_932_897.0),
            Err(DateError::DateNumberOutOfRange(2_932_897.0))
        );
        assert_eq!(iso(1e10), Err(DateError::Overflow));
        assert_eq!(iso(f64::INFINITY), Err(DateError::Overflow));
        assert!(iso(f64::NAN).is_err());
    }

    #[test]
    fn named_zones_follow_dateutil() {
        let Ok(tz) = TimeZone::named("America/New_York") else {
            return; // no system zoneinfo database
        };
        let epoch = Epoch::default();
        let cases = [
            ("2020-03-08T06:59:59", "2020-03-08T01:59:59-05:00", "EST"),
            ("2020-03-08T07:00:00", "2020-03-08T03:00:00-04:00", "EDT"),
            ("2020-11-01T05:30:00", "2020-11-01T01:30:00-04:00", "EDT"),
            ("2020-11-01T06:30:00", "2020-11-01T01:30:00-05:00", "EST"),
            ("2040-07-01T12:00:00", "2040-07-01T07:00:00-05:00", "EST"),
            ("1890-01-01T00:00:00", "1889-12-31T19:03:58-04:56:02", "LMT"),
        ];
        for (instant, local, name) in cases {
            let x = epoch.date2num(utc(instant));
            let zoned = epoch.num2date(x, &tz).unwrap();
            assert_eq!(zoned.isoformat(), local);
            assert_eq!(zoned.strftime("%Z"), name);
            assert_eq!(epoch.date2num(zoned.to_utc().unwrap()), x);
        }
    }

    #[test]
    fn epochs_come_from_the_rc_value() {
        assert_eq!(rc_epoch("2000-01-01T00:00:00").text, "2000-01-01T00:00:00");
        assert_eq!(rc_epoch("not a date"), Epoch::default());
    }

    #[test]
    fn epoch_is_latched_once_read() {
        let _ = get_epoch();
        assert_eq!(set_epoch("2000-01-01"), Err(DateError::EpochAlreadySet));
        assert!(Epoch::parse("NaT").is_err());
    }

    #[test]
    fn drange_is_half_open() {
        let start = ZonedDateTime::parse_iso("2020-01-01").unwrap();
        let day = TimeDelta::new(1, 0, 0).unwrap();
        assert_eq!(drange(&start, &start, day).unwrap(), Vec::<f64>::new());
        let before = ZonedDateTime::parse_iso("2019-12-25").unwrap();
        assert!(drange(&start, &before, day).is_err());
        assert!(drange(&start, &before, TimeDelta::ZERO).is_err());
        assert_eq!(datestr2num("1970-01-02T00:00:00Z"), Ok(1.0));
    }
}
