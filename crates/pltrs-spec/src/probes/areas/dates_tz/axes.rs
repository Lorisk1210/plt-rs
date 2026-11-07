//! `dates_tz.format` and `dates_tz.plot`: the labels the date formatters
//! give in zones, on the ticks of an `AutoDateLocator` and on the x axis
//! of plotted dates.

use pltrs::axes::AxisName;
use pltrs::dates::{
    AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, DateFormatter, TimeZone,
    ZonedDateTime,
};
use pltrs::figure::Figure;
use pltrs::ticker::{AxisInfo, Formatter, Locator, TickContext, TickerSettings};
use pltrs::units::AxisValues;

use super::support::{When, in_rc, whens, zone_arg};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, made, malformed};

/// The probe's formatter, `<formatter>(locator, **params, tz=tz)`
/// (`DateFormatter(fmt, tz=tz)`).
fn formatter(
    args: Args<'_>,
    locator: &AutoDateLocator,
    tz: Option<TimeZone>,
) -> Result<Box<dyn Formatter>, ProbeError> {
    static EMPTY: Json = Json::Object(Vec::new());
    let params = Args::new(args.get("params").unwrap_or(&EMPTY));
    Ok(match (args.str("formatter")?, tz) {
        ("DateFormatter", tz) => {
            let formatter = DateFormatter::new(params.str("fmt")?);
            Box::new(match tz {
                Some(tz) => formatter.with_tz(tz),
                None => formatter,
            })
        }
        ("AutoDateFormatter", tz) => {
            let formatter = AutoDateFormatter::new(locator.clone());
            Box::new(match tz {
                Some(tz) => formatter.with_tz(tz),
                None => formatter,
            })
        }
        ("ConciseDateFormatter", tz) => {
            let formatter = ConciseDateFormatter::new()
                .with_show_offset(params.opt_bool("show_offset")?.unwrap_or(true));
            Box::new(match tz {
                Some(tz) => formatter.with_tz(tz),
                None => formatter,
            })
        }
        (other, _) => return Err(malformed(&format!("formatter {other}"))),
    })
}

/// `{labels[, offset, short]}`: the probe's formatter labelling the ticks
/// of an `AutoDateLocator(tz=tz)` on its view.
pub(super) fn format(args: Args<'_>) -> Outcome {
    in_rc(
        args,
        || {
            let tz = zone_arg(args, "tz")?;
            let locator = match &tz {
                Some(tz) => AutoDateLocator::new().with_tz(tz.clone()),
                None => AutoDateLocator::new(),
            };
            let formatter = formatter(args, &locator, tz)?;
            Ok((locator, formatter))
        },
        |(locator, formatter)| {
            let view = whens(args.json("view")?)?;
            let [first, last] = view.as_slice() else {
                return Err(malformed("view"));
            };
            let axis = AxisInfo::with_view(first.number()?, last.number()?);
            let settings = TickerSettings::default();
            let ctx = TickContext::new(&axis, &settings);
            let ticks = made(locator.locate(&ctx))?;
            let labels = made(formatter.format_ticks(&ticks, &ctx))?;
            let mut fields = vec![("labels".to_owned(), Json::from(labels.labels))];
            if args.str("formatter")? == "ConciseDateFormatter" {
                let first_tick = *ticks
                    .first()
                    .ok_or_else(|| malformed("a view without ticks"))?;
                fields.push(("offset".to_owned(), Json::from(labels.offset)));
                fields.push((
                    "short".to_owned(),
                    Json::from(made(formatter.format_data_short(first_tick, &ctx))?),
                ));
            }
            Ok(Json::Object(fields))
        },
    )
}

/// Plotted dates: naive ones (days and `datetime64` values too) as date
/// numbers without a zone, aware ones with the zone of the first.
fn plotted(dates: &[When]) -> Result<AxisValues, ProbeError> {
    if dates.iter().all(When::is_naive) {
        let numbers = dates
            .iter()
            .map(When::number)
            .collect::<Result<Vec<f64>, _>>()?;
        return Ok(AxisValues::Dates { numbers, tz: None });
    }
    let zoned = dates
        .iter()
        .map(|d| match d {
            When::Aware(_) => d.zoned(&TimeZone::Utc),
            _ => Err(ProbeError::BadArgs(
                "naive dates after aware ones read in the machine's zone".into(),
            )),
        })
        .collect::<Result<Vec<ZonedDateTime>, _>>()?;
    Ok(AxisValues::from(zoned))
}

/// `{ticks, labels, offset}` of the x axis of `ax.plot(d, range(len(d)))`
/// on a new figure, plotted with the `date.converter` rcParam the probe's
/// `converter` names, after drawing.
pub(super) fn plot(args: Args<'_>) -> Outcome {
    in_rc(
        args,
        || {
            let dates = whens(args.json("d")?)?;
            let counts: Vec<f64> = (0..dates.len()).map(|i| i as f64).collect();
            let figure = Figure::new();
            let ax = figure.subplot();
            pltrs::rc::context(|| {
                if let Some(converter) = args.opt_str("converter")? {
                    made(pltrs::rc::set("date.converter", converter))?;
                }
                made(ax.plot(plotted(&dates)?, counts))
            })?;
            Ok((figure, ax))
        },
        |(figure, ax)| {
            figure.layout();
            let axis = ax.xaxis();
            Ok(Json::Object(vec![
                ("ticks".to_owned(), Json::from(made(axis.ticklocs(false))?)),
                (
                    "labels".to_owned(),
                    Json::from(made(axis.ticklabels(false))?),
                ),
                (
                    "offset".to_owned(),
                    Json::from(made(ax.ticks(AxisName::X))?.offset),
                ),
            ]))
        },
    )
}
