//! Probe operations for the `dates` area.
//!
//! Dates arrive as ISO 8601 strings (naive ones are UTC). Locators are
//! evaluated like the reference module does: attached to a dummy axis whose
//! view interval is `[date2num(vmin), date2num(vmax)]` and then called.
//!
//! The operations are grouped as [`conversions`], the [`auto`] locator,
//! the [`fixed`]-frequency and rrule locators and the [`formatters`], on
//! top of the argument helpers in [`support`].

mod auto;
mod conversions;
mod fixed;
mod formatters;
mod support;

pub(crate) use auto::{auto_call, auto_locator};
pub(crate) use conversions::{date2num_op, drange_op, num2timedelta_op};
pub(crate) use fixed::{rrule_spec, year_call};
pub(crate) use formatters::{auto_format_ticks, concise_format_ticks, concise_formatter};
pub(crate) use support::{date_num, ticks, zoned};

use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("dates.get_epoch", conversions::epoch),
    Op::new("dates.date2num", date2num_op),
    Op::new(
        "dates.date2num.datetime64",
        conversions::date2num_datetime64,
    ),
    Op::new("dates.num2date", conversions::num2date_op),
    Op::new("dates.num2timedelta", num2timedelta_op),
    Op::new("dates.drange", drange_op),
    Op::new("dates.AutoDateLocator.__call__", auto_call),
    Op::new("dates.AutoDateLocator.nonsingular", auto::auto_nonsingular),
    Op::new("dates.AutoDateFormatter.format_ticks", auto_format_ticks),
    Op::new(
        "dates.ConciseDateFormatter.format_ticks",
        concise_format_ticks,
    ),
    Op::new("dates.YearLocator.__call__", year_call),
    Op::new("dates.YearLocator.nonsingular", fixed::year_nonsingular),
    Op::new("dates.MonthLocator.__call__", fixed::month_call),
    Op::new("dates.MonthLocator.nonsingular", fixed::month_nonsingular),
    Op::new("dates.WeekdayLocator.__call__", fixed::weekday_call),
    Op::new(
        "dates.WeekdayLocator.nonsingular",
        fixed::weekday_nonsingular,
    ),
    Op::new("dates.DayLocator.__call__", fixed::day_call),
    Op::new("dates.DayLocator.nonsingular", fixed::day_nonsingular),
    Op::new("dates.HourLocator.__call__", fixed::hour_call),
    Op::new("dates.HourLocator.nonsingular", fixed::hour_nonsingular),
    Op::new("dates.MinuteLocator.__call__", fixed::minute_call),
    Op::new("dates.MinuteLocator.nonsingular", fixed::minute_nonsingular),
    Op::new("dates.SecondLocator.__call__", fixed::second_call),
    Op::new("dates.SecondLocator.nonsingular", fixed::second_nonsingular),
    Op::new("dates.MicrosecondLocator.__call__", fixed::microsecond_call),
    Op::new(
        "dates.MicrosecondLocator.nonsingular",
        fixed::microsecond_nonsingular,
    ),
    Op::new("dates.RRuleLocator.__call__", fixed::rrule_call),
    Op::new(
        "dates.DateFormatter.__call__",
        formatters::date_formatter_call,
    ),
];
