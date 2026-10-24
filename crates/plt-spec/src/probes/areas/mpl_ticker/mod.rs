//! Probe operations for the `mpl_ticker` area: tick locators and
//! formatters ported from matplotlib's `tests/test_ticker.py` (see
//! `spec/reference/probes/test_mpl_ticker.py`).
//!
//! Locators and formatters used on their own are the `ticker` area's
//! ops plus the calls of [`standalone`]; the tests that read the ticks of
//! an axes are replayed by [`axes`].

mod axes;
mod standalone;

pub(crate) use axes::styled;

use super::ticker;
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("ticker.MaxNLocator.tick_values", ticker::maxn_tick_values),
    Op::new("ticker.MaxNLocator.view_limits", ticker::maxn_view_limits),
    Op::new("ticker.MaxNLocator._steps", standalone::max_n_steps),
    Op::new("ticker.AutoLocator._steps", standalone::auto_steps),
    Op::new(
        "ticker.MultipleLocator.tick_values",
        ticker::multiple_tick_values,
    ),
    Op::new(
        "ticker.MultipleLocator.view_limits",
        ticker::multiple_view_limits,
    ),
    Op::new("ticker.LinearLocator.tick_values", standalone::linear),
    Op::new("ticker.IndexLocator.tick_values", ticker::index_tick_values),
    Op::new("ticker.LogLocator.tick_values", ticker::log_tick_values),
    Op::new("ticker.LogitLocator.tick_values", ticker::logit_tick_values),
    Op::new("ticker.LogitLocator.nonsingular", ticker::logit_nonsingular),
    Op::new(
        "ticker.SymmetricalLogLocator.tick_values",
        ticker::symlog_tick_values,
    ),
    Op::new(
        "ticker.SymmetricalLogLocator.view_limits",
        ticker::symlog_view_limits,
    ),
    Op::new(
        "ticker.SymmetricalLogLocator.__call__",
        standalone::symlog_call,
    ),
    Op::new("ticker.AsinhLocator.tick_values", ticker::asinh_tick_values),
    Op::new("ticker.AsinhLocator.__call__", ticker::asinh_call),
    Op::new(
        "ticker.ScalarFormatter.format_data",
        ticker::scalar_format_data,
    ),
    Op::new(
        "ticker.ScalarFormatter.format_data_short",
        ticker::scalar_format_data_short,
    ),
    Op::new(
        "ticker.ScalarFormatter.get_useOffset",
        standalone::scalar_use_offset,
    ),
    Op::new(
        "ticker.ScalarFormatter.get_useMathText",
        standalone::scalar_use_mathtext,
    ),
    Op::new("ticker.LogFormatter.__call__", standalone::log_plain_call),
    Op::new(
        "ticker.LogFormatterExponent.__call__",
        standalone::log_exponent_call,
    ),
    Op::new(
        "ticker.LogFormatterMathtext.__call__",
        standalone::log_mathtext_call,
    ),
    Op::new(
        "ticker.LogFormatterSciNotation.__call__",
        standalone::log_sci_call,
    ),
    Op::new("ticker.LogFormatter._pprint_val", standalone::pprint_val),
    Op::new(
        "ticker.LogitFormatter.format_ticks",
        ticker::logit_format_ticks,
    ),
    Op::new("ticker.LogitFormatter.__call__", standalone::logit_call),
    Op::new(
        "ticker.LogitFormatter.format_data_short",
        standalone::logit_data_short,
    ),
    Op::new(
        "ticker.FormatStrFormatter.format_ticks",
        ticker::format_str_format_ticks,
    ),
    Op::new("ticker.EngFormatter.__call__", standalone::eng_call),
    Op::new(
        "ticker.PercentFormatter.format_pct",
        standalone::percent_format_pct,
    ),
    Op::new("ticker.NullFormatter.__call__", standalone::null_call),
    Op::new("ticker.NullFormatter.format_data", standalone::null_data),
    Op::new(
        "ticker.NullFormatter.format_data_short",
        standalone::null_data_short,
    ),
    Op::new("ticker.FixedFormatter.get_offset", standalone::fixed_offset),
    Op::new("ticker.FuncFormatter.get_offset", standalone::func_offset),
    Op::new("axis.Axis.get_ticklocs", axes::ticklocs),
    Op::new("axis.Axis.get_ticklabels", axes::ticklabels),
    Op::new("axis.Formatter.format_data", axes::format_data),
    Op::new("axis.Formatter.format_data_short", axes::format_data_short),
    Op::new("axes.Axes.get_yticks", axes::shared_hist),
    Op::new("axis.Axis.get_minorticklocs", axes::remove_overlap),
];
