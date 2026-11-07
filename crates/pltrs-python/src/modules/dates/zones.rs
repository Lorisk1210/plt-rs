//! Time zones of the program's: a `tzinfo` pltrs (Rust) has no model of
//! (`dateutil.tz.tzlocal()`, `tzstr`, a class the program writes), asked
//! what pltrs (Rust) needs through the adapter `_program_zones.ProgramZone`
//! makes of it, as matplotlib's code asks the zone: through `astimezone`
//! and `tzname` of `datetime`s in it. The names pltrs (Rust) finds no zone
//! of are read by dateutil's `gettz`, as matplotlib reads the `timezone`
//! rcParam ([`GettzFinder`]).

use std::any::Any;

use pltrs::dates::{DateError, DateTime, TimeZone, UtcOffset, ZoneFinder, ZoneRules};

use super::locators::{Parts, Zone, parts};
use crate::binding::lookup;
use crate::binding::prelude::*;

/// A zone of the program's, through its adapter: `offsets(times)` gives
/// the offsets in microseconds of wall-clock times given as `[(parts,
/// fold)]`, `name(parts, fold)` the abbreviation at one, `local(parts)`
/// the wall-clock parts and fold of a UTC instant.
#[derive(Debug)]
pub(crate) struct PyZone {
    /// The adapter.
    adapter: Owned,
    /// The program's `tzinfo`, the zone's identity.
    tz: Owned,
    offsets: Callback,
    name: Callback,
    local: Callback,
}

impl PyZone {
    /// The zone behind `adapter`.
    pub(crate) fn of(adapter: &Bound<'_>) -> PyResult<Self> {
        Ok(Self {
            adapter: adapter.clone().unbind(),
            tz: adapter.getattr("tz")?.unbind(),
            offsets: adapter.getattr("offsets")?.extract()?,
            name: adapter.getattr("name")?.extract()?,
            local: adapter.getattr("local")?.extract()?,
        })
    }

    /// The adapter, which archives the zone.
    pub(crate) fn adapter(&self) -> &Owned {
        &self.adapter
    }
}

/// The date-time of calendar parts, as `datetime` has them.
fn datetime(
    ((year, month, day), (hour, minute, second, micro)): Parts,
) -> Result<DateTime, DateError> {
    DateTime::new(year, month, day, hour, minute, second, micro)
}

impl ZoneRules for PyZone {
    fn offset_at_local(&self, local: DateTime, fold: bool) -> UtcOffset {
        self.offsets_at_local(&[(local, fold)])
            .first()
            .copied()
            .unwrap_or(UtcOffset::UTC)
    }

    /// The offsets, asked in one call (UTC's when the zone raised, whose
    /// exception the running native call raises).
    fn offsets_at_local(&self, locals: &[(DateTime, bool)]) -> Vec<UtcOffset> {
        let times: Vec<(Parts, bool)> = locals
            .iter()
            .map(|&(local, fold)| (parts(local), fold))
            .collect();
        let micros: Vec<i64> = self.offsets.call((times,)).unwrap_or_default();
        if micros.len() != locals.len() {
            return vec![UtcOffset::UTC; locals.len()];
        }
        micros
            .into_iter()
            .map(|micros| UtcOffset::from_micros(micros).unwrap_or(UtcOffset::UTC))
            .collect()
    }

    fn name_at_local(&self, local: DateTime, fold: bool) -> String {
        self.name.call((parts(local), fold)).unwrap_or_default()
    }

    /// The wall-clock time, the UTC instant itself when the zone raised.
    fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
        match self.local.call::<(Parts, bool)>((parts(utc),)) {
            Some((local, fold)) => Ok((datetime(local)?, fold)),
            None => Ok((utc, false)),
        }
    }

    /// Whether `other` is a zone of the same `tzinfo` object, as Python's
    /// aware date-times compare their zones.
    fn same_zone(&self, other: &dyn ZoneRules) -> bool {
        let other: &dyn Any = other;
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| other.tz.as_ptr() == self.tz.as_ptr())
    }
}

/// The zones dateutil's `gettz` finds where pltrs (Rust) finds none (a
/// POSIX `TZ` string, the empty name of the system's local zone), through
/// `_program_zones.find`, which the package registers as `find_zone`.
pub(crate) struct GettzFinder;

impl ZoneFinder for GettzFinder {
    fn find(&self, name: &str) -> Option<TimeZone> {
        let find: Callback = Python::with_gil(|py| lookup(py, "find_zone")?.extract()).ok()?;
        let found: Option<Zone> = find.call((name.to_owned(),))?;
        found.map(|Zone(tz)| tz)
    }
}
