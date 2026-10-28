//! Time zones: UTC, fixed offsets and IANA zones read from `TZif` files.
//!
//! The model is Python's: a [`ZonedDateTime`] is a local wall-clock
//! [`DateTime`] plus a [`TimeZone`]; arithmetic within one zone happens on
//! the wall clock and the UTC offset is looked up from the local time (with
//! a `fold` flag disambiguating repeated wall times).

mod tzfile;
mod zoned;

use std::fmt::{self, Write as _};
use std::sync::Arc;

pub use tzfile::TzFile;
pub use zoned::ZonedDateTime;

use super::timedelta::{MICROS_PER_DAY, MICROS_PER_SECOND};
use super::{DateError, DateTime, TimeDelta};

/// A fixed offset from UTC, strictly between -24 h and +24 h.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcOffset {
    micros: i64,
}

impl UtcOffset {
    /// The zero offset.
    pub const UTC: Self = Self { micros: 0 };

    /// An offset of `seconds` east of UTC.
    pub fn from_seconds(seconds: i32) -> Result<Self, DateError> {
        Self::from_micros(i64::from(seconds) * MICROS_PER_SECOND)
    }

    /// An offset of `micros` microseconds east of UTC.
    pub fn from_micros(micros: i64) -> Result<Self, DateError> {
        if micros.abs() >= MICROS_PER_DAY {
            return Err(DateError::field("utc offset", micros));
        }
        Ok(Self { micros })
    }

    /// The offset as a duration (positive east of UTC).
    pub const fn as_timedelta(self) -> TimeDelta {
        TimeDelta::from_micros(self.micros)
    }

    /// Python's `_format_offset`: `±HH<sep>MM[<sep>SS[.ffffff]]`.
    pub fn format(self, separator: &str) -> String {
        let sign = if self.micros < 0 { '-' } else { '+' };
        let micros = self.micros.abs();
        let seconds = micros / MICROS_PER_SECOND;
        let mut text = format!(
            "{sign}{:02}{separator}{:02}",
            seconds / 3600,
            seconds / 60 % 60
        );
        let (sec, frac) = (seconds % 60, micros % MICROS_PER_SECOND);
        if sec != 0 || frac != 0 {
            let _ = write!(text, "{separator}{sec:02}");
            if frac != 0 {
                let _ = write!(text, ".{frac:06}");
            }
        }
        text
    }
}

impl fmt::Display for UtcOffset {
    /// `±HH:MM`, as in ISO 8601 strings.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format(":"))
    }
}

/// A time zone.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum TimeZone {
    /// Coordinated Universal Time (`datetime.timezone.utc`).
    #[default]
    Utc,
    /// A constant offset (`datetime.timezone(offset)`).
    Fixed(UtcOffset),
    /// An IANA zone with historical transitions (`dateutil.tz.tzfile`).
    Zone(Arc<TzFile>),
}

impl TimeZone {
    /// Resolves a zone name the way matplotlib's `_get_tzinfo` does:
    /// `"UTC"` is UTC; anything else is looked up in the system zoneinfo
    /// database (`/usr/share/zoneinfo` and friends), like
    /// `dateutil.tz.gettz`. POSIX `TZ` rule strings are not supported.
    pub fn named(name: &str) -> Result<Self, DateError> {
        if name == "UTC" {
            return Ok(Self::Utc);
        }
        TzFile::find(name).map(|zone| Self::Zone(Arc::new(zone)))
    }

    /// The zone the `timezone` rcParam names, resolved by
    /// [`named`](Self::named) (matplotlib's `_get_tzinfo(None)`): the zone
    /// of date locators, formatters and date axes given none, and the one
    /// to pass [`num2date`](super::num2date) for matplotlib's default.
    ///
    /// ```
    /// use plt::dates::TimeZone;
    /// plt::rc::context(|| {
    ///     plt::rc::set("timezone", "UTC")?;
    ///     assert_eq!(TimeZone::from_rc(), Ok(TimeZone::Utc));
    ///     plt::rc::set("timezone", "Nowhere/Atlantis")?;
    ///     assert!(TimeZone::from_rc().is_err());
    ///     Ok::<(), plt::rc::RcError>(())
    /// })?;
    /// # Ok::<(), plt::rc::RcError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`DateError::UnknownTimeZone`] when the system knows no zone of
    /// that name (matplotlib's `ValueError`).
    pub fn from_rc() -> Result<Self, DateError> {
        Self::named(&crate::rc::params().timezone)
    }

    /// The offset in effect at the wall-clock time `local`; `fold` selects
    /// the second occurrence of a repeated wall time.
    pub fn offset_at_local(&self, local: DateTime, fold: bool) -> UtcOffset {
        match self {
            Self::Utc => UtcOffset::UTC,
            Self::Fixed(offset) => *offset,
            Self::Zone(zone) => zone.offset_at_local(local, fold),
        }
    }

    /// The zone abbreviation in effect at `local` (Python's `tzname()`).
    pub fn name_at_local(&self, local: DateTime, fold: bool) -> String {
        match self {
            Self::Utc => "UTC".to_owned(),
            Self::Fixed(offset) if *offset == UtcOffset::UTC => "UTC".to_owned(),
            Self::Fixed(offset) => format!("UTC{offset}"),
            Self::Zone(zone) => zone.name_at_local(local, fold).to_owned(),
        }
    }

    /// Converts a UTC instant to this zone's wall clock, returning the local
    /// time and its `fold`.
    pub fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
        match self {
            Self::Utc => Ok((utc, false)),
            Self::Fixed(offset) => Ok((utc.checked_add(offset.as_timedelta())?, false)),
            Self::Zone(zone) => zone.local_from_utc(utc),
        }
    }
}

/// The zone a date locator or formatter settles when it is made: the one
/// given, or the `timezone` rcParam's (matplotlib's `self.tz =
/// _get_tzinfo(tz)`). A name the system does not know is kept as the
/// error that using the zone gives, so that the constructors need not
/// fail and a zone given later replaces it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TickZone(Result<TimeZone, DateError>);

impl TickZone {
    /// The zone the `timezone` rcParam names now.
    pub(crate) fn from_rc() -> Self {
        Self(TimeZone::from_rc())
    }

    /// The zone, or why the rcParam named none.
    pub(crate) fn get(&self) -> Result<&TimeZone, DateError> {
        self.0.as_ref().map_err(Clone::clone)
    }
}

impl From<TimeZone> for TickZone {
    fn from(tz: TimeZone) -> Self {
        Self(Ok(tz))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_zones_settle_the_rc_param_when_made() {
        let (utc, unknown) = crate::rc::context(|| {
            crate::rc::set("timezone", "Nowhere/Atlantis").unwrap();
            let unknown = TickZone::from_rc();
            crate::rc::set("timezone", "UTC").unwrap();
            (TickZone::from_rc(), unknown)
        });
        assert_eq!(utc.get(), Ok(&TimeZone::Utc));
        assert_eq!(
            unknown.get(),
            Err(DateError::UnknownTimeZone("Nowhere/Atlantis".to_owned()))
        );
        let fixed = TimeZone::Fixed(UtcOffset::from_seconds(3600).unwrap());
        assert_eq!(TickZone::from(fixed.clone()).get(), Ok(&fixed));
    }

    #[test]
    fn offsets_format_like_python() {
        let off = UtcOffset::from_seconds(-(5 * 3600 + 30 * 60)).unwrap();
        assert_eq!(off.to_string(), "-05:30");
        assert_eq!(off.format(""), "-0530");
        let odd = UtcOffset::from_micros(3_723_000_001).unwrap();
        assert_eq!(odd.format(":"), "+01:02:03.000001");
        assert!(UtcOffset::from_seconds(86_400).is_err());
        let zone = TimeZone::Fixed(UtcOffset::from_seconds(7200).unwrap());
        let local = DateTime::new(2020, 1, 1, 0, 0, 0, 0).unwrap();
        assert_eq!(zone.name_at_local(local, false), "UTC+02:00");
        assert_eq!(TimeZone::Utc.name_at_local(local, false), "UTC");
        assert_eq!(TimeZone::named("UTC"), Ok(TimeZone::Utc));
    }
}
