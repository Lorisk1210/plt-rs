//! Probe operations for the `dates_tz` area: the conversions, locators,
//! formatters and date axes of `matplotlib.dates` in time zones, with the
//! argument conventions of `spec/reference/probes/_dates_tz.py`.
//!
//! Zones and dates are read in [`support`] (the zones the probes write as
//! `tzinfo` classes in [`program`]); the operations are the
//! [`conversions`], [`locate`] and the formatters and plots of [`axes`].
//! Naive dates have no type of their own in pltrs (Rust): they are UTC,
//! as `date2num` reads them, except where a recurrence rule with a zone
//! of its own reads them on its wall clock.

mod axes;
mod conversions;
mod locate;
mod program;
mod support;

use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("dates_tz.date2num", conversions::date2num),
    Op::new("dates_tz.num2date", conversions::num2date),
    Op::new("dates_tz.drange", conversions::drange),
    Op::new("dates_tz.datetime64", conversions::datetime64),
    Op::new("dates_tz.epoch", conversions::epoch),
    Op::new("dates_tz.locate", locate::locate),
    Op::new("dates_tz.format", axes::format),
    Op::new("dates_tz.plot", axes::plot),
];
