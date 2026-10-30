//! `_native.dates`: dates as matplotlib's `dates` numbers them (days since
//! the epoch, `date.epoch`), by `pltrs::dates`.
//!
//! Instants cross as microseconds since 1970 (numpy's `datetime64[us]`)
//! one way, and as UTC calendar parts `((year, month, day), (hour, minute,
//! second, microsecond))` the other; Python attaches time zones. The
//! locators and recurrence rules are in [`locators`].

mod locators;

pub(super) use locators::{Zone, date_locator};
use pltrs::dates::{
    Date, DateError, Datetime64, TimeZone, datestr2num, datestr2num_with_default,
    datetime64_to_num, get_epoch, num2date, num2timedelta, reset_epoch, set_epoch, wrap_in_tex,
};

use crate::binding::prelude::*;

impl From<DateError> for PyErr {
    /// matplotlib's `ValueError` for dates it cannot convert, and its
    /// `RuntimeError` for an epoch set after dates were used.
    fn from(err: DateError) -> Self {
        match err {
            DateError::EpochAlreadySet => PyErr::runtime_error(err.to_string()),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// Whether `text` reads as a date and time (an ISO 8601 / numpy
/// `datetime64` string, `NaT` included).
fn is_datetime_string(text: String) -> PyResult<bool> {
    Ok(Datetime64::parse(&text).is_ok())
}

/// The date numbers of instants given as microseconds since 1970 (NaN
/// where `nat` is set).
fn micros_to_num(micros: NdArray<i64>, nat: NdArray<bool>) -> PyResult<NdArray> {
    let numbers = micros
        .data
        .iter()
        .zip(&nat.data)
        .map(|(&us, &nat)| {
            (!nat).then(|| {
                let (seconds, rest) = (us.div_euclid(1_000_000), us.rem_euclid(1_000_000));
                let nanos = u32::try_from(rest * 1_000).unwrap_or_default();
                Datetime64::new(seconds, nanos)
            })
        })
        .map(datetime64_to_num)
        .collect();
    NdArray::with_shape(numbers, micros.shape)
}

/// UTC calendar parts of a date.
type UtcParts = ((i32, u32, u32), (u32, u32, u32, u32));

/// The UTC dates of date numbers, as calendar parts; `ValueError` for
/// numbers outside the years 1–9999, with matplotlib's message.
fn num_to_utc(numbers: NdArray) -> PyResult<Vec<UtcParts>> {
    numbers
        .data
        .iter()
        .map(|&x| {
            let utc = num2date(x, &TimeZone::Utc).map_err(|_| {
                PyErr::value_error(format!(
                    "Date ordinal {x} converts to a date (using epoch {}), but Matplotlib \
                     dates must be between year 0001 and 9999.",
                    get_epoch()
                ))
            })?;
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

/// `datestr2num(text)` for an ISO 8601 date string.
fn datestr_to_num(text: String) -> PyResult<f64> {
    Ok(datestr2num(&text)?)
}

/// `datestr2num(text, default=date)`: the day or month a reduced ISO 8601
/// date leaves out comes from `(year, month, day)`.
fn datestr_to_num_with_default(text: String, (year, month, day): (i32, u32, u32)) -> PyResult<f64> {
    Ok(datestr2num_with_default(
        &text,
        Date::new(year, month, day)?,
    )?)
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
    locators::register(module)?;
    module.add("is_datetime_string", is_datetime_string)?;
    module.add("micros_to_num", micros_to_num)?;
    module.add("num_to_utc", num_to_utc)?;
    module.add("num_to_timedelta", num_to_timedelta)?;
    module.add("datestr2num", datestr_to_num)?;
    module.add("datestr2num_with_default", datestr_to_num_with_default)?;
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
        let micros = NdArray::from_vec(vec![43_200_000_000, -86_400_000_000, 0]);
        let nat = NdArray::from_vec(vec![false, false, true]);
        let days = micros_to_num(micros, nat).unwrap().data;
        assert_eq!(days[..2], [0.5, -1.0]);
        assert!(days[2].is_nan());
        let parts = num_to_utc(NdArray::from_vec(vec![0.5])).unwrap();
        assert_eq!(parts, [((1970, 1, 1), (12, 0, 0, 0))]);
        assert!(num_to_utc(NdArray::from_vec(vec![1e9])).is_err());
        let micros = num_to_timedelta(NdArray::from_vec(vec![1.5])).unwrap();
        assert_eq!(micros.data, [129_600_000_000]);
    }
}
