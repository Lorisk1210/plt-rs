//! Argument helpers of the `dates` area: dates and integer lists from the
//! probe arguments, the dummy axis locators run on, and the shape of the
//! recorded results.

use pltrs::dates::{TimeZone, Weekday, ZonedDateTime, date2num, num2date};
use pltrs::ticker::{AxisInfo, Locator, TickContext, TickerSettings};

use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// `_dt(iso)`: naive strings are UTC.
pub(crate) fn zoned(text: &str) -> Result<ZonedDateTime, ProbeError> {
    ZonedDateTime::parse_iso(text).map_err(ProbeError::rejected)
}

pub(crate) fn date_num(text: &str) -> Result<f64, ProbeError> {
    date2num(&zoned(text)?).map_err(ProbeError::rejected)
}

/// The zone the probe's `tz` names, or `num2date`'s default without one:
/// the zone of the `timezone` rcParam.
pub(crate) fn zone_arg(args: Args<'_>) -> Result<TimeZone, ProbeError> {
    args.opt_str("tz")?
        .map_or_else(TimeZone::from_rc, TimeZone::named)
        .map_err(ProbeError::rejected)
}

/// `num2date(x, tz).isoformat()`.
pub(super) fn iso(x: f64, tz: &TimeZone) -> Result<Json, ProbeError> {
    num2date(x, tz)
        .map(|d| Json::from(d.isoformat()))
        .map_err(ProbeError::rejected)
}

/// An integer or a list of integers (`None` when absent), like dateutil's
/// `by*` arguments.
pub(super) fn int_list(value: Option<&Json>) -> Result<Option<Vec<i64>>, ProbeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let items = match value.as_array() {
        Some(items) => items.iter().map(Json::as_i64).collect::<Option<Vec<_>>>(),
        None => value.as_i64().map(|n| vec![n]),
    };
    items
        .map(Some)
        .ok_or_else(|| ProbeError::BadArgs("expected an integer or a list of integers".into()))
}

pub(super) fn convert_all<T: TryFrom<i64>>(values: Vec<i64>) -> Result<Vec<T>, ProbeError> {
    values
        .into_iter()
        .map(|v| T::try_from(v).map_err(|_| ProbeError::Rejected(format!("{v} is out of range"))))
        .collect()
}

/// An integer list argument converted to `T`; empty when absent.
pub(super) fn by_list<T: TryFrom<i64>>(args: Args<'_>, key: &str) -> Result<Vec<T>, ProbeError> {
    convert_all(int_list(args.get(key))?.unwrap_or_default())
}

/// A positive integer argument; matplotlib rejects fractional and
/// non-positive intervals.
pub(super) fn positive_int(args: Args<'_>, key: &str, default: u32) -> Result<u32, ProbeError> {
    let Some(value) = args.opt_f64(key)? else {
        return Ok(default);
    };
    if value.fract() != 0.0 || value < 1.0 || value > f64::from(u32::MAX) {
        return Err(ProbeError::Rejected(format!(
            "{key} must be an integer greater than 0"
        )));
    }
    Ok(value as u32)
}

pub(super) fn weekdays(values: Vec<i64>) -> Vec<Weekday> {
    values.into_iter().map(Weekday::from_index).collect()
}

/// The dummy axis of `_attach`: view (and data) interval
/// `[date2num(vmin), date2num(vmax)]`.
pub(super) fn view_axis(args: Args<'_>) -> Result<AxisInfo, ProbeError> {
    Ok(AxisInfo::with_view(
        date_num(args.str("vmin")?)?,
        date_num(args.str("vmax")?)?,
    ))
}

/// `locator()` on the dummy axis.
pub(super) fn located(locator: &dyn Locator, args: Args<'_>) -> Result<Vec<f64>, ProbeError> {
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    locator
        .locate(&TickContext::new(&axis, &settings))
        .map_err(ProbeError::rejected)
}

/// `{ticks, iso}`.
pub(crate) fn ticks(ticks: Vec<f64>) -> Outcome {
    Ok(tick_fields(ticks)?.into())
}

/// The `{ticks, iso}` fields as object entries, the dates in the zone of
/// the `timezone` rcParam.
pub(super) fn tick_fields(ticks: Vec<f64>) -> Result<ObjectBuilder, ProbeError> {
    let tz = TimeZone::from_rc().map_err(ProbeError::rejected)?;
    let iso = ticks
        .iter()
        .map(|&t| iso(t, &tz))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ObjectBuilder::default()
        .field("ticks", ticks)
        .field("iso", Json::Array(iso)))
}

/// Builds a JSON object field by field.
#[derive(Default)]
pub(super) struct ObjectBuilder(Vec<(String, Json)>);

impl ObjectBuilder {
    pub(super) fn field(mut self, key: &str, value: impl Into<Json>) -> Self {
        self.0.push((key.to_owned(), value.into()));
        self
    }
}

impl From<ObjectBuilder> for Json {
    fn from(builder: ObjectBuilder) -> Self {
        Self::Object(builder.0)
    }
}

pub(super) fn nonsingular(locator: &dyn Locator, args: Args<'_>) -> Outcome {
    let (lo, hi) = locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &TickContext::default(),
    );
    Ok(Json::from(vec![lo, hi]))
}
