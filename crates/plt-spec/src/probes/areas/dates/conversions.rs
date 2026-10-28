//! Conversions between dates and Matplotlib date numbers: the epoch,
//! `date2num`, `num2date`, `num2timedelta` and `drange`.

use plt::dates::{Datetime64, TimeDelta, datetime64_to_num, drange, get_epoch, num2timedelta};

use super::support::{date_num, iso, zone_arg, zoned};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

pub(super) fn epoch(_: Args<'_>) -> Outcome {
    Ok(Json::from(get_epoch().to_string()))
}

pub(crate) fn date2num_op(args: Args<'_>) -> Outcome {
    let d = args.json("d")?;
    match (d.as_str(), d.as_array()) {
        (Some(text), _) => Ok(Json::from(date_num(text)?)),
        (_, Some(items)) => items
            .iter()
            .map(|item| {
                let text = item
                    .as_str()
                    .ok_or_else(|| ProbeError::BadArgs("d must hold strings".into()))?;
                date_num(text)
            })
            .collect::<Result<Vec<f64>, _>>()
            .map(Json::from),
        _ => Err(ProbeError::BadArgs(
            "d must be a string or a list of strings".into(),
        )),
    }
}

pub(super) fn date2num_datetime64(args: Args<'_>) -> Outcome {
    let items = args
        .json("d")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("d must be a list".into()))?;
    items
        .iter()
        .map(|item| {
            let text = item
                .as_str()
                .ok_or_else(|| ProbeError::BadArgs("d must hold strings".into()))?;
            Datetime64::parse(text)
                .map(datetime64_to_num)
                .map_err(ProbeError::rejected)
        })
        .collect::<Result<Vec<f64>, _>>()
        .map(Json::from)
}

/// `num2date(x, tz).isoformat()` for a number or a list of numbers, in
/// the zone the probe's `tz` names or else the `timezone` rcParam's.
pub(crate) fn num2date_op(args: Args<'_>) -> Outcome {
    let tz = zone_arg(args)?;
    let x = args.json("x")?;
    match (x.as_f64(), x.as_f64_vec()) {
        (Some(x), _) => iso(x, &tz),
        (_, Some(values)) => values
            .into_iter()
            .map(|x| iso(x, &tz))
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        _ => Err(ProbeError::BadArgs(
            "x must be a number or a list of numbers".into(),
        )),
    }
}

pub(crate) fn num2timedelta_op(args: Args<'_>) -> Outcome {
    let parts = |x: f64| -> Result<Json, ProbeError> {
        let delta = num2timedelta(x).map_err(ProbeError::rejected)?;
        Ok(Json::from(vec![
            delta.days() as f64,
            delta.seconds() as f64,
            delta.microseconds() as f64,
        ]))
    };
    let x = args.json("x")?;
    match (x.as_f64(), x.as_f64_vec()) {
        (Some(x), _) => parts(x),
        (_, Some(values)) => values
            .into_iter()
            .map(parts)
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        _ => Err(ProbeError::BadArgs(
            "x must be a number or a list of numbers".into(),
        )),
    }
}

pub(crate) fn drange_op(args: Args<'_>) -> Outcome {
    let delta = Args::new(args.json("delta")?);
    let part = |key: &str| delta.opt_i64(key).map(Option::unwrap_or_default);
    let seconds = part("hours")? * 3600 + part("minutes")? * 60 + part("seconds")?;
    let step = TimeDelta::new(part("days")?, seconds, part("microseconds")?)
        .map_err(ProbeError::rejected)?;
    drange(
        &zoned(args.str("dstart")?)?,
        &zoned(args.str("dend")?)?,
        step,
    )
    .map(Json::from)
    .map_err(ProbeError::rejected)
}
