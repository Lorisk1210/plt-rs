//! The zones and dates of the `dates_tz` probes, the `timezone` rcParam
//! they set, and the shape of the date-times they record.

use std::sync::Arc;

use pltrs::dates::iso::{Datetime64, fromisoformat};
use pltrs::dates::{
    Date, DateTime, TimeZone, UtcOffset, ZoneInfo, ZonedDateTime, date2num, datetime64_to_num,
};

use crate::json::Json;
use crate::probes::{Args, ProbeError, made, malformed};

/// The zone a zone spec names: `None` for `null` (the zone of the
/// `timezone` rcParam); a string as `dateutil.tz.gettz` reads it
/// (`"UTC"` is UTC); `{"kind": ...}` for `datetime.timezone` objects
/// (`utc`, `offset`), `zoneinfo.ZoneInfo` (`zoneinfo`), `dateutil.tz`
/// (`dateutil`, `tzutc`).
pub(super) fn zone(spec: &Json) -> Result<Option<TimeZone>, ProbeError> {
    if spec.is_null() {
        return Ok(None);
    }
    if let Some(name) = spec.as_str() {
        return made(TimeZone::named(name)).map(Some);
    }
    let spec = Args::new(spec);
    let seconds = || {
        let seconds = i32::try_from(spec.i64("seconds")?).map_err(|_| malformed("seconds"))?;
        made(UtcOffset::from_seconds(seconds))
    };
    Ok(Some(match spec.str("kind")? {
        "utc" | "tzutc" => TimeZone::Utc,
        "offset" => TimeZone::Fixed(seconds()?),
        "zoneinfo" => TimeZone::Info(Arc::new(made(ZoneInfo::find(spec.str("name")?))?)),
        "dateutil" => made(TimeZone::named(spec.str("name")?))?,
        _ => return Err(malformed("zone kind")),
    }))
}

/// The zone of the probe's argument `key` (absent: `None`, as `null`).
pub(super) fn zone_arg(args: Args<'_>, key: &str) -> Result<Option<TimeZone>, ProbeError> {
    args.get(key).map_or(Ok(None), zone)
}

/// The zone of `tz`, or that of the `timezone` rcParam for `None`
/// (matplotlib's `_get_tzinfo`).
pub(super) fn zone_or_rc(tz: Option<TimeZone>) -> Result<TimeZone, ProbeError> {
    tz.map_or_else(|| made(TimeZone::from_rc()), Ok)
}

/// What a date spec stands for.
pub(super) enum When {
    /// A naive date-time.
    Naive(DateTime),
    /// An aware one.
    Aware(ZonedDateTime),
    /// A `datetime.date`.
    Day(Date),
    /// A numpy `datetime64`, `None` for `NaT`.
    Instant(Option<Datetime64>),
}

impl When {
    /// The date-time, a naive one (or a day's midnight) read on the wall
    /// clock of `naive`.
    pub(super) fn zoned(&self, naive: &TimeZone) -> Result<ZonedDateTime, ProbeError> {
        match self {
            Self::Naive(local) => Ok(ZonedDateTime::from_local(*local, naive.clone())),
            Self::Aware(zoned) => Ok(zoned.clone()),
            Self::Day(date) => Ok(ZonedDateTime::from_local(
                DateTime::midnight(*date),
                naive.clone(),
            )),
            Self::Instant(_) => Err(ProbeError::BadArgs("a datetime64 is no date-time".into())),
        }
    }

    /// Its date number (`date2num`: naive dates are UTC).
    pub(super) fn number(&self) -> Result<f64, ProbeError> {
        match self {
            Self::Instant(instant) => Ok(datetime64_to_num(*instant)),
            other => made(date2num(&other.zoned(&TimeZone::Utc)?)),
        }
    }

    /// Whether the date is naive (a `datetime64` and a day are).
    pub(super) const fn is_naive(&self) -> bool {
        !matches!(self, Self::Aware(_))
    }
}

/// The date of a date spec: an ISO string (naive, or with a fixed offset),
/// `{"at", "tz", "fold"}` for a wall-clock time in a zone, `{"date"}` or
/// `{"datetime64"}`.
pub(super) fn when(spec: &Json) -> Result<When, ProbeError> {
    if let Some(text) = spec.as_str() {
        let (local, offset) = made(fromisoformat(text))?;
        return Ok(match offset {
            None => When::Naive(local),
            Some(offset) => When::Aware(ZonedDateTime::from_local(local, TimeZone::Fixed(offset))),
        });
    }
    let spec = Args::new(spec);
    if let Some(day) = spec.opt_str("date")? {
        let (local, _) = made(fromisoformat(day))?;
        return Ok(When::Day(local.date()));
    }
    if let Some(text) = spec.opt_str("datetime64")? {
        return Ok(When::Instant(made(Datetime64::parse(text))?));
    }
    let (local, _) = made(fromisoformat(spec.str("at")?))?;
    let tz = zone(spec.json("tz")?)?.ok_or_else(|| malformed("the zone of a date"))?;
    let fold = spec.opt_i64("fold")?.unwrap_or(0) != 0;
    Ok(When::Aware(
        ZonedDateTime::from_local(local, tz).with_fold(fold),
    ))
}

/// The dates of a list of date specs.
pub(super) fn whens(spec: &Json) -> Result<Vec<When>, ProbeError> {
    spec.as_array()
        .ok_or_else(|| malformed("a list of dates"))?
        .iter()
        .map(when)
        .collect()
}

/// A date-time as the probes record it: `[isoformat, tzname, fold]`.
pub(super) fn described(dt: &ZonedDateTime) -> Json {
    Json::Array(vec![
        Json::from(dt.isoformat()),
        Json::from(dt.tz_name()),
        Json::from(f64::from(u8::from(dt.fold()))),
    ])
}

/// Sets the `timezone` rcParam to `name`, when given.
fn set_timezone(name: Option<&str>) -> Result<(), ProbeError> {
    match name {
        Some(name) => made(pltrs::rc::set("timezone", name)),
        None => Ok(()),
    }
}

/// `make` with the `timezone` rcParam the probe's `rc` names, then `then`
/// of what it made with the one its `rc_then` names (else still `rc`'s),
/// the rcParams restored afterwards.
pub(super) fn in_rc<T, U>(
    args: Args<'_>,
    make: impl FnOnce() -> Result<T, ProbeError>,
    then: impl FnOnce(T) -> Result<U, ProbeError>,
) -> Result<U, ProbeError> {
    pltrs::rc::context(|| {
        set_timezone(args.opt_str("rc")?)?;
        let made = make()?;
        set_timezone(args.opt_str("rc_then")?)?;
        then(made)
    })
}
