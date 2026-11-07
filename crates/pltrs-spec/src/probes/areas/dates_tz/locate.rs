//! `dates_tz.locate`: a date locator in a zone, through an axis or with
//! the dates themselves.

use pltrs::dates::{AutoDateLocator, AutoFrequency, DateLocator, RRuleLocator, TimeZone, num2date};
use pltrs::ticker::{AxisInfo, TickContext, TickerSettings};

use super::support::{When, described, in_rc, whens, zone_arg};
use crate::json::Json;
use crate::probes::areas::dates::{FixedLocator, auto_locator, fixed_locator, rrule_spec};
use crate::probes::{Args, Outcome, ProbeError, made, malformed};

/// The probe's locator.
enum Made {
    /// `YearLocator` ... `MicrosecondLocator`.
    Fixed(FixedLocator),
    /// `RRuleLocator(rrulewrapper(...))`.
    Rule(RRuleLocator),
    /// `AutoDateLocator`.
    Auto(AutoDateLocator),
}

impl Made {
    /// `<locator>(**params, tz=tz)`; an `RRuleLocator`'s `params` are
    /// `{rule}`, the keywords of its `rrulewrapper` (`tzinfo` a zone spec).
    fn of(args: Args<'_>) -> Result<Self, ProbeError> {
        let tz = zone_arg(args, "tz")?;
        Ok(match args.str("locator")? {
            "AutoDateLocator" => {
                let locator = auto_locator(args.get("params"))?;
                Self::Auto(match tz {
                    Some(tz) => locator.with_tz(tz),
                    None => locator,
                })
            }
            "RRuleLocator" => {
                let rule = Args::new(Args::new(args.json("params")?).json("rule")?);
                let mut locator = RRuleLocator::new(rrule_spec(rule)?);
                if let Some(rule_tz) = zone_arg(rule, "tzinfo")? {
                    locator = locator.with_rule_tz(rule_tz);
                }
                Self::Rule(match tz {
                    Some(tz) => locator.with_tz(tz),
                    None => locator,
                })
            }
            class => {
                let locator = fixed_locator(class, args)?;
                Self::Fixed(match tz {
                    Some(tz) => locator.with_tz(tz),
                    None => locator,
                })
            }
        })
    }

    fn as_date_locator(&self) -> &dyn DateLocator {
        match self {
            Self::Fixed(locator) => locator.as_date_locator(),
            Self::Rule(locator) => locator,
            Self::Auto(locator) => locator,
        }
    }

    /// The zone naive dates are read in: the rule's own, else UTC.
    fn naive_zone(&self) -> TimeZone {
        match self {
            Self::Rule(locator) => locator.rule_tz().cloned().unwrap_or_default(),
            Self::Fixed(_) | Self::Auto(_) => TimeZone::Utc,
        }
    }
}

/// `{ticks, iso[, unit]}` of the probe's locator on its view: through an
/// axis (`via = "axis"`) or with the dates themselves (`tick_values`).
pub(super) fn locate(args: Args<'_>) -> Outcome {
    in_rc(
        args,
        || Made::of(args),
        |made_locator| {
            let view = whens(args.json("view")?)?;
            let [first, last] = view.as_slice() else {
                return Err(malformed("view"));
            };
            let locator = made_locator.as_date_locator();
            let settings = TickerSettings::default();
            let (ticks, unit) = if args.str("via")? == "axis" {
                let axis = AxisInfo::with_view(first.number()?, last.number()?);
                let ctx = TickContext::new(&axis, &settings);
                (made(locator.locate(&ctx))?, locator.unit(&ctx))
            } else {
                tick_values(&made_locator, first, last)?
            };
            let tz = made(locator.tz())?;
            let iso = ticks
                .iter()
                .map(|&t| made(num2date(t, tz)).map(|dt| described(&dt)))
                .collect::<Result<Vec<_>, _>>()?;
            let mut fields = vec![
                ("ticks".to_owned(), Json::from(ticks)),
                ("iso".to_owned(), Json::Array(iso)),
            ];
            if matches!(made_locator, Made::Auto(_)) {
                fields.push(("unit".to_owned(), Json::from(unit)));
            }
            Ok(Json::Object(fields))
        },
    )
}

/// `locator.tick_values(first, last)`, and the locator's unit after it
/// (an `AutoDateLocator`'s that of the frequency it picked, yearly for
/// none, as matplotlib's fresh locator keeps).
fn tick_values(
    made_locator: &Made,
    first: &When,
    last: &When,
) -> Result<(Vec<f64>, f64), ProbeError> {
    let naive = made_locator.naive_zone();
    let (dmin, dmax) = (first.zoned(&naive)?, last.zoned(&naive)?);
    let locator = made_locator.as_date_locator();
    let ticks = made(locator.tick_dates(&dmin, &dmax))?;
    let unit = match made_locator {
        Made::Auto(auto) => made(auto.get_locator(&dmin, &dmax))?
            .frequency
            .unwrap_or(AutoFrequency::Yearly)
            .unit_days(),
        Made::Fixed(_) | Made::Rule(_) => locator.unit(&TickContext::default()),
    };
    Ok((ticks, unit))
}
