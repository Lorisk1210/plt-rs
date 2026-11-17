//! `_native.dates`: dates as matplotlib's `dates` numbers them (days since
//! the epoch, `date.epoch`), by `pltrs::dates`.
//!
//! Instants cross as microseconds since 1970 (numpy's `datetime64[us]`)
//! one way, and as UTC calendar parts `((year, month, day), (hour, minute,
//! second, microsecond))` the other; Python attaches time zones. The
//! locators and recurrence rules are in [`locators`], the zones of the
//! program's in [`zones`].

mod locators;
mod zones;

pub(super) use locators::{Zone, date_locator};
use pltrs::dates::{
    DateError, Datetime64, MICROSECONDS_PER_DAY, TimeZone, datetime64_to_num, get_epoch, num2date,
    num2timedelta, ordinal_message_written, reset_epoch, set_epoch, wrap_in_tex,
};
pub(super) use zones::PyZone;

use crate::binding::prelude::*;

impl From<DateError> for PyErr {
    /// matplotlib's `ValueError` for dates it cannot convert, Python's
    /// `OverflowError` for date arithmetic beyond the year 9999, and
    /// matplotlib's `RuntimeError` for an epoch set after dates were used.
    fn from(err: DateError) -> Self {
        match err {
            DateError::EpochAlreadySet => PyErr::runtime_error(err.to_string()),
            DateError::Overflow => {
                PyErr::new(ExceptionKind::OverflowError, "date value out of range")
            }
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// Whether `text` reads as a date and time (an ISO 8601 / numpy
/// `datetime64` string, `NaT` included).
fn is_datetime_string(text: String) -> PyResult<bool> {
    Ok(Datetime64::parse(&text).is_ok())
}

/// The date numbers of instants given as whole seconds since 1970 and
/// nanoseconds beyond them (NaN where `nat` is set), as numpy splits a
/// `datetime64` array for matplotlib's `_dt64_to_ordinalf`.
fn instants_to_num(
    seconds: NdArray<i64>,
    nanos: NdArray<i64>,
    nat: NdArray<bool>,
) -> PyResult<NdArray> {
    let numbers = seconds
        .data
        .iter()
        .zip(&nanos.data)
        .zip(&nat.data)
        .map(|((&seconds, &nanos), &nat)| {
            (!nat).then(|| Datetime64::new(seconds, u32::try_from(nanos).unwrap_or_default()))
        })
        .map(datetime64_to_num)
        .collect();
    NdArray::with_shape(numbers, seconds.shape)
}

/// UTC calendar parts of a date.
type UtcParts = ((i32, u32, u32), (u32, u32, u32, u32));

/// The error matplotlib's `num2date` raises for date number `x`, which no
/// date of the years 1–9999 has: as it rounds the number of microseconds
/// to a Python integer (`ValueError` for NaN, `OverflowError` for an
/// infinity), makes it a numpy `timedelta64` (`OverflowError` beyond 64
/// bits), and checks the year (`ValueError`, with the number as Python
/// writes it: an integer's digits when the numbers were given as
/// `integers`).
fn ordinal_error(x: f64, integers: bool) -> PyErr {
    let micros = (x * MICROSECONDS_PER_DAY).round_ties_even();
    if x.is_nan() {
        PyErr::value_error("cannot convert float NaN to integer")
    } else if x.is_infinite() {
        PyErr::new(
            ExceptionKind::OverflowError,
            "cannot convert float infinity to integer",
        )
    } else if micros.abs() >= 2f64.powi(63) {
        PyErr::new(ExceptionKind::OverflowError, "int too big to convert")
    } else {
        let written = if integers {
            format!("{x:.0}")
        } else {
            pltrs_core::pyfmt::repr(x)
        };
        PyErr::value_error(ordinal_message_written(&written, x, &get_epoch()))
    }
}

/// The UTC dates of date numbers, as calendar parts (given as `integers`,
/// or as floats); for numbers outside the years 1–9999 matplotlib's error
/// ([`ordinal_error`]).
fn num_to_utc(numbers: NdArray, integers: bool) -> PyResult<Vec<UtcParts>> {
    numbers
        .data
        .iter()
        .map(|&x| {
            let utc = num2date(x, &TimeZone::Utc).map_err(|_| ordinal_error(x, integers))?;
            let local = utc.local();
            let (date, time) = (local.date(), local.time());
            Ok((
                (date.year(), date.month(), date.day()),
                (
                    time.hour(),
                    time.minute(),
                    time.second(),
                    time.microsecond(),
                ),
            ))
        })
        .collect()
}

/// The durations of numbers of days, in microseconds.
fn num_to_timedelta(numbers: NdArray) -> PyResult<NdArray<i64>> {
    let micros = numbers
        .data
        .iter()
        .map(|&x| Ok(num2timedelta(x)?.total_micros()))
        .collect::<PyResult<Vec<i64>>>()?;
    NdArray::with_shape(micros, numbers.shape)
}

/// The epoch, as given (`get_epoch`).
fn epoch() -> PyResult<String> {
    Ok(get_epoch().to_string())
}

/// Sets the epoch (`set_epoch`).
fn change_epoch(text: String) -> PyResult<()> {
    Ok(set_epoch(&text)?)
}

/// Restores the epoch of `date.epoch` (`_reset_epoch_test_example`).
fn restore_epoch() -> PyResult<()> {
    reset_epoch();
    Ok(())
}

/// `_wrap_in_tex(text)`: a date label set in TeX's math mode.
fn tex(text: String) -> PyResult<String> {
    Ok(wrap_in_tex(&text))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    pltrs::dates::set_zone_finder(zones::GettzFinder);
    locators::register(module)?;
    module.add("is_datetime_string", is_datetime_string)?;
    module.add("instants_to_num", instants_to_num)?;
    module.add("num_to_utc", num_to_utc)?;
    module.add("num_to_timedelta", num_to_timedelta)?;
    module.add("get_epoch", epoch)?;
    module.add("set_epoch", change_epoch)?;
    module.add("reset_epoch", restore_epoch)?;
    module.add("wrap_in_tex", tex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instants_number_as_days_since_the_epoch() {
        pltrs::rc::restore_startup();
        let seconds = NdArray::from_vec(vec![43_200, -86_401, 0]);
        let nanos = NdArray::from_vec(vec![0, 999_999_999, 0]);
        let nat = NdArray::from_vec(vec![false, false, true]);
        let days = instants_to_num(seconds, nanos, nat).unwrap().data;
        assert_eq!(days[0], 0.5);
        assert_eq!(days[1], (-86_401.0 + 0.999_999_999) / 86_400.0);
        assert!(days[2].is_nan());
        let parts = num_to_utc(NdArray::from_vec(vec![0.5]), false).unwrap();
        assert_eq!(parts, [((1970, 1, 1), (12, 0, 0, 0))]);
        assert!(num_to_utc(NdArray::from_vec(vec![1e9]), true).is_err());
        let micros = num_to_timedelta(NdArray::from_vec(vec![1.5])).unwrap();
        assert_eq!(micros.data, [129_600_000_000]);
    }
}
