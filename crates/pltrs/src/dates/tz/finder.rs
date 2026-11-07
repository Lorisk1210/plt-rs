//! Zones named as no zone pltrs knows is named, found by the program
//! ([`set_zone_finder`]): matplotlib reads the `timezone` rcParam with
//! `dateutil.tz.gettz`, which also reads POSIX `TZ` strings
//! (`dateutil.tz.tzstr`) and takes an empty name for the system's local
//! zone (`tzlocal`); pltrs's Python package hands that very function.

use std::cell::RefCell;
use std::rc::Rc;

use super::TimeZone;

/// What finds the zones [`TimeZone::named`] finds none of.
pub trait ZoneFinder {
    /// The zone `name` names, `None` for none.
    fn find(&self, name: &str) -> Option<TimeZone>;
}

pltrs_core::thread_state! {
    /// The thread's finder.
    static FINDER: RefCell<Option<Rc<dyn ZoneFinder>>> = const { RefCell::new(None) };
}

/// Has `finder` find the zones named as no zone pltrs knows is named
/// (see [`TimeZone::named`]) from now on.
///
/// ```
/// use pltrs::dates::{TimeZone, UtcOffset, ZoneFinder, clear_zone_finder, set_zone_finder};
///
/// /// Names of the form `+H` for whole hours east of UTC.
/// struct Hours;
///
/// impl ZoneFinder for Hours {
///     fn find(&self, name: &str) -> Option<TimeZone> {
///         let hours: i32 = name.strip_prefix('+')?.parse().ok()?;
///         UtcOffset::from_seconds(hours * 3600).ok().map(TimeZone::Fixed)
///     }
/// }
///
/// assert!(TimeZone::named("+3").is_err());
/// set_zone_finder(Hours);
/// assert_eq!(TimeZone::named("+3")?, TimeZone::Fixed(UtcOffset::from_seconds(10_800)?));
/// clear_zone_finder();
/// # Ok::<(), pltrs::dates::DateError>(())
/// ```
pub fn set_zone_finder(finder: impl ZoneFinder + 'static) {
    FINDER.set(Some(Rc::new(finder)));
}

/// Finds no other zones than those pltrs knows from now on.
pub fn clear_zone_finder() {
    FINDER.set(None);
}

/// The zone the program's finder finds for `name`, on a copy of the
/// finder, which may set another meanwhile.
pub(super) fn find(name: &str) -> Option<TimeZone> {
    let finder = FINDER.with(|slot| slot.borrow().clone());
    finder?.find(name)
}
