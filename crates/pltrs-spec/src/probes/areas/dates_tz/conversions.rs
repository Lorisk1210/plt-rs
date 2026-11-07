//! `date2num`, `num2date`, `drange`, numpy `datetime64` of every unit and
//! other epochs, in zones.

use pltrs::dates::iso::Datetime64;
use pltrs::dates::{
    TimeDelta, TimeUnit, TimeZone, datetime64_to_num, num2date as to_date, reset_epoch, set_epoch,
};

use super::support::{When, described, in_rc, when, whens, zone_arg, zone_or_rc};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, made, malformed};

/// `date2num(d)` of the probe's `d`: one date or a list of them.
fn numbers(d: &Json) -> Outcome {
    if d.as_array().is_some() {
        let numbers = whens(d)?
            .iter()
            .map(When::number)
            .collect::<Result<Vec<f64>, _>>()?;
        return Ok(Json::from(numbers));
    }
    Ok(Json::from(when(d)?.number()?))
}

/// `date2num(d)`.
pub(super) fn date2num(args: Args<'_>) -> Outcome {
    numbers(args.json("d")?)
}

/// The date-times of date numbers `x` (one or a list) in `tz`.
fn dates_of(x: &Json, tz: &TimeZone) -> Outcome {
    let date = |x: &Json| {
        let x = x.as_f64().ok_or_else(|| malformed("x"))?;
        made(to_date(x, tz)).map(|dt| described(&dt))
    };
    match x.as_array() {
        Some(items) => Ok(Json::Array(
            items.iter().map(date).collect::<Result<_, _>>()?,
        )),
        None => date(x),
    }
}

/// `num2date(x, tz)`, with the `timezone` rcParam of `rc`.
pub(super) fn num2date(args: Args<'_>) -> Outcome {
    in_rc(
        args,
        || zone_or_rc(zone_arg(args, "tz")?),
        |tz| dates_of(args.json("x")?, &tz),
    )
}

/// The `datetime.timedelta` of the keywords `days`, `hours`, `minutes`,
/// `seconds` and `microseconds`.
fn timedelta(spec: Args<'_>) -> Result<TimeDelta, ProbeError> {
    let part = |key: &str| Ok::<i64, ProbeError>(spec.opt_i64(key)?.unwrap_or(0));
    let seconds = part("hours")? * 3600 + part("minutes")? * 60 + part("seconds")?;
    made(TimeDelta::new(
        part("days")?,
        seconds,
        part("microseconds")?,
    ))
}

/// `drange(dstart, dend, delta)`.
pub(super) fn drange(args: Args<'_>) -> Outcome {
    let start = when(args.json("dstart")?)?.zoned(&TimeZone::Utc)?;
    let end = when(args.json("dend")?)?.zoned(&TimeZone::Utc)?;
    let delta = timedelta(Args::new(args.json("delta")?))?;
    made(pltrs::dates::drange(&start, &end, delta)).map(Json::from)
}

/// `date2num(np.array(counts, dtype=f"datetime64[{unit}]"))`, `null`
/// counts being `NaT`.
pub(super) fn datetime64(args: Args<'_>) -> Outcome {
    let unit: TimeUnit = made(args.str("unit")?.parse())?;
    let counts = args
        .json("counts")?
        .as_array()
        .ok_or_else(|| malformed("counts"))?;
    let numbers = counts
        .iter()
        .map(|count| {
            let instant = match count.as_i64() {
                Some(count) => Some(made(Datetime64::from_count(count, unit))?),
                None if count.is_null() => None,
                None => return Err(malformed("a count")),
            };
            Ok(datetime64_to_num(instant))
        })
        .collect::<Result<Vec<f64>, ProbeError>>()?;
    Ok(Json::from(numbers))
}

/// `{nums, dates}`: `date2num(d)` and `num2date(x)` with
/// `set_epoch(epoch)`, the default epoch restored afterwards.
pub(super) fn epoch(args: Args<'_>) -> Outcome {
    reset_epoch();
    let result = made(set_epoch(args.str("epoch")?)).and_then(|()| {
        let nums = numbers(args.json("d")?)?;
        let tz = made(TimeZone::from_rc())?;
        let dates = dates_of(args.json("x")?, &tz)?;
        Ok(Json::Object(vec![
            ("nums".to_owned(), nums),
            ("dates".to_owned(), dates),
        ]))
    });
    reset_epoch();
    result
}
