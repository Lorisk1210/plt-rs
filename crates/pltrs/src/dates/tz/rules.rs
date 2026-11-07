//! What pltrs asks of a time zone ([`ZoneRules`]): the zones it models
//! answer it, and so can zones a program defines ([`ProgramZone`]), as
//! matplotlib only ever calls the methods of a `tzinfo` (any zone a
//! Python program writes, `dateutil.tz.tzlocal()`, `tzstr`).

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder, KindOf};

use super::UtcOffset;
use crate::dates::{DateError, DateTime};

/// What pltrs asks of a time zone, Python's `tzinfo` protocol: the offset
/// from UTC at a wall-clock time (`utcoffset`), the abbreviation there
/// (`tzname`) and the wall-clock time of a UTC instant (`fromutc`).
///
/// UTC, fixed offsets and the IANA zones implement it; a program gives
/// pltrs a zone of its own as a [`ProgramZone`] ([`TimeZone::Program`]).
///
/// ```
/// use pltrs::dates::{DateError, DateTime, ProgramZone, TimeZone, UtcOffset, ZoneRules};
///
/// /// Two hours east of UTC, called "EET".
/// #[derive(Debug)]
/// struct Eastern;
///
/// impl ZoneRules for Eastern {
///     fn offset_at_local(&self, _: DateTime, _: bool) -> UtcOffset {
///         UtcOffset::from_seconds(7200).expect("within a day")
///     }
///     fn name_at_local(&self, _: DateTime, _: bool) -> String {
///         "EET".to_owned()
///     }
///     fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
///         Ok((utc.checked_add(UtcOffset::from_seconds(7200)?.as_timedelta())?, false))
///     }
/// }
///
/// let zone = TimeZone::Program(ProgramZone::new(Eastern));
/// let noon = DateTime::new(2020, 1, 1, 12, 0, 0, 0)?;
/// assert_eq!(zone.name_at_local(noon, false), "EET");
/// assert_eq!(zone.local_from_utc(noon)?.0.hour(), 14);
/// # Ok::<(), DateError>(())
/// ```
///
/// [`TimeZone::Program`]: super::TimeZone::Program
pub trait ZoneRules: fmt::Debug + Send + Sync + Any {
    /// The offset from UTC in effect at the wall-clock time `local`;
    /// `fold` selects the second occurrence of a repeated wall time.
    fn offset_at_local(&self, local: DateTime, fold: bool) -> UtcOffset;

    /// The offsets of the wall-clock times `locals` (with their folds), as
    /// [`offset_at_local`](Self::offset_at_local) gives them one by one; a
    /// zone answering through code of the program's answers them in one
    /// call.
    fn offsets_at_local(&self, locals: &[(DateTime, bool)]) -> Vec<UtcOffset> {
        locals
            .iter()
            .map(|&(local, fold)| self.offset_at_local(local, fold))
            .collect()
    }

    /// The abbreviation of the zone at the wall-clock time `local`.
    fn name_at_local(&self, local: DateTime, fold: bool) -> String;

    /// The wall-clock time of the UTC instant `utc` here, and its fold.
    ///
    /// # Errors
    ///
    /// When that time is outside the years 1–9999.
    fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError>;

    /// Whether `other` is this very zone: date-times of one zone compare
    /// and subtract on their wall clocks, as Python's aware date-times of
    /// one `tzinfo` object do. By default, whether it is the same value.
    fn same_zone(&self, other: &dyn ZoneRules) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// A time zone the program defines (see [`ZoneRules`]), shared by the
/// date-times and tick helpers it is given to.
#[derive(Clone, Debug)]
pub struct ProgramZone(Arc<dyn ZoneRules>);

impl ProgramZone {
    /// The zone `rules` defines.
    pub fn new(rules: impl ZoneRules) -> Self {
        Self(Arc::new(rules))
    }

    /// What answers for the zone.
    pub fn rules(&self) -> &dyn ZoneRules {
        &*self.0
    }
}

impl PartialEq for ProgramZone {
    /// Whether both are the same zone ([`ZoneRules::same_zone`]).
    fn eq(&self, other: &Self) -> bool {
        self.0.same_zone(other.rules())
    }
}

/// The kind under which a program archives its zones.
fn kind() -> KindOf {
    KindOf::of::<dyn ZoneRules>("ZoneRules")
}

impl Archive for ProgramZone {
    /// The program archives its zones (see
    /// [`ProgramKinds`](pltrs_core::archive::ProgramKinds)).
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        let rules: &dyn Any = self.rules();
        encoder.write_program_kind(kind(), rules)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let rules: Box<dyn ZoneRules> = decoder.read_program_kind(kind())?;
        Ok(Self(Arc::from(rules)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{TimeZone, ZonedDateTime};
    use pltrs_core::archive::{NoProgram, from_bytes, to_bytes};

    /// A zone one hour east of UTC, with a summer an hour later still.
    #[derive(Debug)]
    struct Summery;

    impl Summery {
        fn summer(local: DateTime) -> bool {
            (4..=9).contains(&local.month())
        }
    }

    impl ZoneRules for Summery {
        fn offset_at_local(&self, local: DateTime, _: bool) -> UtcOffset {
            let hours = if Self::summer(local) { 2 } else { 1 };
            UtcOffset::from_seconds(hours * 3600).expect("within a day")
        }

        fn name_at_local(&self, local: DateTime, _: bool) -> String {
            (if Self::summer(local) { "SUM" } else { "WIN" }).to_owned()
        }

        fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
            let standard = utc.checked_add(UtcOffset::from_seconds(3600)?.as_timedelta())?;
            Ok((
                utc.checked_add(self.offset_at_local(standard, false).as_timedelta())?,
                false,
            ))
        }
    }

    #[test]
    fn program_zones_answer_and_compare_by_identity() {
        let zone = ProgramZone::new(Summery);
        let tz = TimeZone::Program(zone.clone());
        let july = DateTime::new(2020, 7, 1, 0, 0, 0, 0).unwrap();
        let january = DateTime::new(2020, 1, 1, 0, 0, 0, 0).unwrap();
        assert_eq!(tz.offset_at_local(july, false).to_string(), "+02:00");
        assert_eq!(tz.name_at_local(january, false), "WIN");
        assert_eq!(
            zone.rules()
                .offsets_at_local(&[(july, false), (january, true)]),
            [
                UtcOffset::from_seconds(7200).unwrap(),
                UtcOffset::from_seconds(3600).unwrap()
            ]
        );
        assert_eq!(tz, TimeZone::Program(zone));
        assert_ne!(tz, TimeZone::Program(ProgramZone::new(Summery)));
        assert_ne!(tz, TimeZone::Utc);
    }

    /// A zone counting how often it is asked its name.
    #[derive(Debug, Default)]
    struct Named(std::sync::atomic::AtomicUsize);

    impl ZoneRules for Named {
        fn offset_at_local(&self, _: DateTime, _: bool) -> UtcOffset {
            UtcOffset::UTC
        }

        fn name_at_local(&self, _: DateTime, _: bool) -> String {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            "NAMED".to_owned()
        }

        fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
            Ok((utc, false))
        }
    }

    #[test]
    fn formats_ask_a_zone_its_name_only_where_they_write_it() {
        let zone = ProgramZone::new(Named::default());
        let noon = DateTime::new(2020, 1, 1, 12, 0, 0, 0).unwrap();
        let at = ZonedDateTime::from_local(noon, TimeZone::Program(zone.clone()));
        let asked = || {
            let rules: &dyn Any = zone.rules();
            let named = rules.downcast_ref::<Named>().expect("a named zone");
            named.0.load(std::sync::atomic::Ordering::Relaxed)
        };
        assert_eq!(at.strftime("%H:%M %%Z"), "12:00 %Z");
        assert_eq!(asked(), 0);
        assert_eq!(at.strftime("%Z %H %^Z"), "NAMED 12 NAMED");
        assert_eq!(asked(), 1);
    }

    #[test]
    fn a_program_without_zones_cannot_archive_them() {
        let zone = TimeZone::Program(ProgramZone::new(Summery));
        let err = to_bytes(&zone, &NoProgram).expect_err("pure Rust programs archive none");
        assert!(matches!(err, ArchiveError::Unsupported(_)), "{err:?}");
        let (bytes, externals) = to_bytes(&TimeZone::Utc, &NoProgram).unwrap();
        let back: TimeZone = from_bytes(&bytes, &externals, &NoProgram).unwrap();
        assert_eq!(back, TimeZone::Utc);
    }
}
