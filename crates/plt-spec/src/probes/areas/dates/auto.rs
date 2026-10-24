//! `AutoDateLocator`: built from the probe's parameters, called on the
//! dummy axis and described the way the reference module records it.

use plt::dates::{AutoDateLocator, AutoFrequency, ChosenLocator, DateLocator, TimeZone};
use plt::ticker::{Locator, TickContext, TickerSettings};

use super::support::{ObjectBuilder, nonsingular, tick_fields, view_axis};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// `AutoDateLocator(**params)`; `maxticks` is an int or `{freq name: int}`
/// and `tz` a zone name.
pub(crate) fn auto_locator(params: Option<&Json>) -> Result<AutoDateLocator, ProbeError> {
    static EMPTY: Json = Json::Object(Vec::new());
    let params = Args::new(params.unwrap_or(&EMPTY));
    let mut locator = AutoDateLocator::new();
    if let Some(tz) = params.opt_str("tz")? {
        locator = locator.with_tz(TimeZone::named(tz).map_err(ProbeError::rejected)?);
    }
    if let Some(multiples) = params.opt_bool("interval_multiples")? {
        locator = locator.with_interval_multiples(multiples);
    }
    if let Some(minticks) = params.opt_i64("minticks")? {
        locator = locator.with_minticks(
            u32::try_from(minticks).map_err(|_| ProbeError::BadArgs("minticks".into()))?,
        );
    }
    match params.get("maxticks") {
        None => {}
        Some(Json::Object(entries)) => {
            for (name, value) in entries {
                let frequency = AutoFrequency::from_name(name)
                    .ok_or_else(|| ProbeError::BadArgs(format!("frequency {name}")))?;
                let count = value
                    .as_i64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| {
                        ProbeError::BadArgs("maxticks values must be integers".into())
                    })?;
                locator = locator.with_maxticks_for(frequency, count);
            }
        }
        Some(value) => {
            let count = value
                .as_i64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| {
                    ProbeError::BadArgs("maxticks must be an integer or an object".into())
                })?;
            locator = locator.with_maxticks(count);
        }
    }
    Ok(locator)
}

/// A `by*` list as the reference records it: `null` when not given.
fn optional_list<T: Copy + Into<f64>>(values: &[T]) -> Json {
    if values.is_empty() {
        Json::Null
    } else {
        Json::from(values.iter().map(|&v| v.into()).collect::<Vec<f64>>())
    }
}

/// The reference module's `describe_locator`.
fn describe(locator: &ChosenLocator) -> Json {
    match locator {
        ChosenLocator::Year(year) => ObjectBuilder::default()
            .field("class", "YearLocator")
            .field("base", f64::from(year.base()))
            .field("month", f64::from(year.month()))
            .field("day", f64::from(year.day()))
            .into(),
        ChosenLocator::RRule(rrule) => {
            let rule = rrule.rule();
            ObjectBuilder::default()
                .field("class", "RRuleLocator")
                .field("freq", rule.freq.name())
                .field("interval", f64::from(rule.interval))
                .field("bymonth", optional_list(&rule.bymonth))
                .field("bymonthday", optional_list(&rule.bymonthday))
                .field("byhour", optional_list(&rule.byhour))
                .field("byminute", optional_list(&rule.byminute))
                .field("bysecond", optional_list(&rule.bysecond))
                .into()
        }
        ChosenLocator::Microsecond(micro) => ObjectBuilder::default()
            .field("class", "MicrosecondLocator")
            .field("interval", f64::from(micro.micros()))
            .into(),
    }
}

pub(crate) fn auto_call(args: Args<'_>) -> Outcome {
    let locator = auto_locator(args.get("params"))?;
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    let ctx = TickContext::new(&axis, &settings);
    let found = locator.locate(&ctx).map_err(ProbeError::rejected)?;
    let (vmin, vmax) = axis.view_interval;
    let choice = locator.choose(vmin, vmax).map_err(ProbeError::rejected)?;
    Ok(tick_fields(found)?
        .field("unit", locator.unit(&ctx))
        .field("locator", describe(&choice.locator))
        .into())
}

pub(super) fn auto_nonsingular(args: Args<'_>) -> Outcome {
    nonsingular(&AutoDateLocator::new(), args)
}
