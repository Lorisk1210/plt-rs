//! Tick locating and labelling (matplotlib's `ticker`).
//!
//! A [`Locator`] decides where ticks go and a [`Formatter`] turns tick
//! values into labels. Both are configured once and then evaluated against
//! a [`TickContext`]: the [`AxisInfo`] they serve (view and data limits,
//! smallest positive datum, tick space, current major ticks) plus the
//! [`TickerSettings`] (the relevant rcParams). This replaces matplotlib's
//! back-pointer from tick helpers to their `Axis`.
//!
//! ```
//! use pltrs::ticker::{AutoLocator, AxisInfo, Formatter, Locator, ScalarFormatter,
//!                   TickContext, TickerSettings};
//!
//! let axis = AxisInfo::with_view(0.0, 1e6);
//! let settings = TickerSettings::default();
//! let ctx = TickContext::new(&axis, &settings);
//! let ticks = AutoLocator::new().locate(&ctx).unwrap();
//! assert_eq!(ticks.len(), 6);
//! let labels = ScalarFormatter::new().format_ticks(&ticks, &ctx).unwrap();
//! assert_eq!(labels.labels[1], "0.2");
//! assert_eq!(labels.offset, "1e6");
//! ```
//!
//! Python's float formatting is reproduced exactly by [`pyfmt`] (from
//! [`pltrs_core::pyfmt`], which the backends share), which other modules may
//! use for matplotlib-compatible number formatting.
#![expect(
    clippy::float_cmp,
    reason = "matplotlib's tick algorithms compare floats exactly; the ports keep those comparisons"
)]

mod context;
mod error;
mod formatter;
mod label_format;
mod locator;
mod log_base;
pub(crate) mod pymath;
mod settings;

pub use context::{AxisInfo, AxisName, ScaleHint, TickContext};
pub use error::TickerError;
pub(crate) use formatter::g_sig_digits;
pub use formatter::{
    EngFormatter, FixedFormatter, FormatStrFormatter, FormattedTicks, Formatter, FuncFormatter,
    LogFormatter, LogLabelStyle, LogitFormatter, NullFormatter, OffsetMode, PercentFormatter,
    SI_PREFIXES, ScalarFormatter, StrMethodFormatter, strip_math,
};
pub use label_format::{LabelFormat, ValueFn};
pub(crate) use locator::drawable_ticks;
pub use locator::{
    AsinhLocator, AutoLocator, AutoMinorLocator, FixedLocator, IndexLocator, LinearLocator,
    Locator, LogLocator, LogSubs, LogitLocator, MAX_DRAWN_TICKS, MAXTICKS, MaxNLocator,
    MultipleLocator, Nbins, NullLocator, NumTicks, Prune, SymmetricalLogLocator, exceeds_max_ticks,
    scale_range,
};
pub use log_base::LogBase;
pub use pltrs_core::pyfmt;
pub use settings::{AutolimitMode, MinorDivisions, TickerSettings};
