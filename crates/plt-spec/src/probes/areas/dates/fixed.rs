//! The fixed-frequency locators (`YearLocator` … `MicrosecondLocator`)
//! and `RRuleLocator` over the rule an `rrulewrapper` describes.

use plt::dates::{
    DateLocator, Frequency, MicrosecondLocator, NthWeekday, RRuleLocator, RRuleSpec, YearLocator,
};

use super::support::{
    by_list, convert_all, int_list, located, nonsingular, positive_int, ticks, weekdays,
};
use crate::probes::areas::ticker::Params;
use crate::probes::{Args, Outcome, ProbeError};

/// Generates the `__call__` and `nonsingular` adapters of the fixed-frequency
/// locators.
macro_rules! fixed_locator_ops {
    ($($class:literal => $call:ident, $nonsingular:ident;)*) => {
        $(
            pub(crate) fn $call(args: Args<'_>) -> Outcome {
                let locator = fixed_locator($class, args)?;
                ticks(located(locator.as_ref(), args)?)
            }

            pub(super) fn $nonsingular(args: Args<'_>) -> Outcome {
                let locator = fixed_locator($class, args)?;
                nonsingular(locator.as_ref(), args)
            }
        )*
    };
}

fixed_locator_ops! {
    "YearLocator" => year_call, year_nonsingular;
    "MonthLocator" => month_call, month_nonsingular;
    "WeekdayLocator" => weekday_call, weekday_nonsingular;
    "DayLocator" => day_call, day_nonsingular;
    "HourLocator" => hour_call, hour_nonsingular;
    "MinuteLocator" => minute_call, minute_nonsingular;
    "SecondLocator" => second_call, second_nonsingular;
    "MicrosecondLocator" => microsecond_call, microsecond_nonsingular;
}

/// `LOCATORS[class](**params)`.
fn fixed_locator(class: &str, args: Args<'_>) -> Result<Box<dyn DateLocator>, ProbeError> {
    let interval = |params: Args<'_>| positive_int(params, "interval", 1);
    Ok(match class {
        "YearLocator" => {
            let params = Params::of(args, &["base", "month", "day"])?.args();
            let get = |key: &str| positive_int(params, key, 1);
            Box::new(
                YearLocator::new(get("base")?, get("month")?, get("day")?)
                    .map_err(ProbeError::rejected)?,
            )
        }
        "MonthLocator" => {
            let params = Params::of(args, &["bymonth", "bymonthday", "interval"])?.args();
            let monthday = int_list(params.get("bymonthday"))?.unwrap_or_else(|| vec![1]);
            Box::new(RRuleLocator::months(
                &by_list(params, "bymonth")?,
                &convert_all(monthday)?,
                interval(params)?,
            ))
        }
        "WeekdayLocator" => {
            let params = Params::of(args, &["byweekday", "interval"])?.args();
            let days = int_list(params.get("byweekday"))?.unwrap_or_else(|| vec![1]);
            Box::new(RRuleLocator::weekdays(&weekdays(days), interval(params)?))
        }
        "DayLocator" => {
            let params = Params::of(args, &["bymonthday", "interval"])?.args();
            Box::new(
                RRuleLocator::days(&by_list(params, "bymonthday")?, interval(params)?)
                    .map_err(ProbeError::rejected)?,
            )
        }
        "HourLocator" => {
            let params = Params::of(args, &["byhour", "interval"])?.args();
            Box::new(RRuleLocator::hours(
                &by_list(params, "byhour")?,
                interval(params)?,
            ))
        }
        "MinuteLocator" => {
            let params = Params::of(args, &["byminute", "interval"])?.args();
            Box::new(RRuleLocator::minutes(
                &by_list(params, "byminute")?,
                interval(params)?,
            ))
        }
        "SecondLocator" => {
            let params = Params::of(args, &["bysecond", "interval"])?.args();
            Box::new(RRuleLocator::seconds(
                &by_list(params, "bysecond")?,
                interval(params)?,
            ))
        }
        "MicrosecondLocator" => {
            let params = Params::of(args, &["interval"])?.args();
            Box::new(MicrosecondLocator::new(interval(params)?).map_err(ProbeError::rejected)?)
        }
        other => return Err(ProbeError::BadArgs(format!("unknown locator {other}"))),
    })
}

pub(super) fn rrule_call(args: Args<'_>) -> Outcome {
    let spec = rrule_spec(Args::new(args.json("rule")?))?;
    ticks(located(&RRuleLocator::new(spec), args)?)
}

/// The rule `rrulewrapper(freq, **rule)` describes.
pub(crate) fn rrule_spec(rule: Args<'_>) -> Result<RRuleSpec, ProbeError> {
    let freq = Frequency::from_name(rule.str("freq")?)
        .ok_or_else(|| ProbeError::BadArgs("unknown frequency".into()))?;
    Ok(RRuleSpec {
        interval: positive_int(rule, "interval", 1)?,
        bysetpos: by_list(rule, "bysetpos")?,
        bymonth: by_list(rule, "bymonth")?,
        bymonthday: by_list(rule, "bymonthday")?,
        byyearday: by_list(rule, "byyearday")?,
        byweekday: weekdays(int_list(rule.get("byweekday"))?.unwrap_or_default())
            .into_iter()
            .map(NthWeekday::from)
            .collect(),
        byhour: by_list(rule, "byhour")?,
        byminute: by_list(rule, "byminute")?,
        bysecond: by_list(rule, "bysecond")?,
        ..RRuleSpec::new(freq)
    })
}
