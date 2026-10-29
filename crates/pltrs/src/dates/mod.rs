//! Dates on axes (matplotlib's `dates`), on a self-contained date/time layer.
//!
//! Matplotlib represents dates as floating-point *date numbers*: days since
//! an [`Epoch`] (default `1970-01-01T00:00:00` UTC). This module provides
//!
//! * the civil date/time types — [`Date`], [`Time`], naive [`DateTime`]
//!   and zone-aware [`ZonedDateTime`] (years 1-9999, microsecond
//!   resolution, proleptic Gregorian [`calendar`]), [`TimeDelta`] and the
//!   month-aware [`RelativeDelta`];
//! * time zones: UTC, fixed offsets and IANA zones read from the system's
//!   `TZif` files ([`TimeZone::named`], with `dateutil.tz.tzfile` semantics);
//! * parsing ([`iso`]) and `strftime` formatting
//!   ([`DateTime::strftime`]) with Python's and the C library's semantics;
//! * the conversions [`date2num`], [`num2date`], [`num2timedelta`],
//!   [`drange`] and [`datestr2num`], and evenly spaced date-times
//!   ([`DateRange`]);
//! * recurrence rules ([`rrule`], dateutil semantics);
//! * the tick locators — [`AutoDateLocator`], [`RRuleLocator`] (with the
//!   `MonthLocator`-style constructors), [`YearLocator`],
//!   [`MicrosecondLocator`] — and formatters — [`DateFormatter`],
//!   [`AutoDateFormatter`], [`ConciseDateFormatter`] — implementing the
//!   [`ticker`](crate::ticker) traits.
//!
//! Locators, formatters and date axes given no zone work in the one the
//! `timezone` rcParam names ([`TimeZone::from_rc`]), read when matplotlib
//! reads it: when locators (those of date axes too) and [`DateFormatter`]s
//! are made, and whenever [`AutoDateFormatter`] and
//! [`ConciseDateFormatter`] format. A name the system does not know makes
//! locating and formatting fail with [`DateError::UnknownTimeZone`]; a
//! zone given with `with_tz` replaces it.
//!
//! ```
//! use pltrs::dates::{AutoDateLocator, ConciseDateFormatter, datestr2num};
//! use pltrs::ticker::{AxisInfo, Formatter, Locator, TickContext, TickerSettings};
//!
//! let view = (datestr2num("2020-01-15T13:47:12").unwrap(), datestr2num("2020-03-02").unwrap());
//! let axis = AxisInfo::with_view(view.0, view.1);
//! let settings = TickerSettings::default();
//! let ctx = TickContext::new(&axis, &settings);
//! let ticks = AutoDateLocator::new().locate(&ctx).unwrap();
//! let labels = ConciseDateFormatter::new().format_ticks(&ticks, &ctx).unwrap();
//! assert_eq!(labels.labels, ["22", "Feb", "08", "15", "22", "Mar"]);
//! assert_eq!(labels.offset, "2020-Mar");
//! ```

pub mod calendar;
mod convert;
mod datetime;
mod error;
mod formatters;
pub mod iso;
mod locators;
mod range;
mod relativedelta;
pub mod rrule;
mod strftime;
mod timedelta;
mod tz;

pub use calendar::Weekday;
pub use convert::{
    Epoch, date2num, datestr2num, datestr2num_with_default, datetime64_to_num, drange, get_epoch,
    num2date, num2timedelta, reset_epoch, set_epoch,
};
pub use datetime::{Date, DateTime, MAX_YEAR, MIN_YEAR, Time};
pub use error::DateError;
pub use formatters::{AutoDateFormatter, ConciseDateFormatter, DateFormatter, wrap_in_tex};
pub use iso::Datetime64;
pub use locators::{
    AutoDateLocator, AutoFrequency, ChosenLocator, DateLocator, LocatorChoice, MicrosecondLocator,
    RRuleLocator, YearLocator,
};
pub use range::{DateRange, MAX_RANGE_DATES};
pub use relativedelta::RelativeDelta;
pub use rrule::{Frequency, NthWeekday, RRule, RRuleSpec};
pub use timedelta::TimeDelta;
pub use tz::{TimeZone, TzFile, UtcOffset, ZonedDateTime};
