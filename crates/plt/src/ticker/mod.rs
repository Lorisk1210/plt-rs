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
//! use plt::ticker::{AutoLocator, AxisInfo, Formatter, Locator, ScalarFormatter,
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
//! Python's float formatting is reproduced exactly by [`pyfmt`], which
//! other modules may use for matplotlib-compatible number formatting.
#![allow(
    clippy::float_cmp,
    reason = "matplotlib's tick algorithms compare floats exactly; the ports keep those comparisons"
)]

mod context;
mod error;
mod formatter;
mod locator;
pub mod pyfmt;
pub(crate) mod pymath;
mod settings;

pub use context::{AxisInfo, AxisName, ScaleHint, TickContext};
pub use error::TickerError;
pub use formatter::{
    EngFormatter, FixedFormatter, FormatStrFormatter, Formatter, FuncFormatter, LogFormatter,
    LogLabelStyle, LogitFormatter, NullFormatter, OffsetMode, PercentFormatter, ScalarFormatter,
    StrMethodFormatter, TickLabels, strip_math,
};
pub use locator::{
    AsinhLocator, AutoLocator, AutoMinorLocator, FixedLocator, IndexLocator, LinearLocator,
    Locator, LogLocator, LogSubs, LogitLocator, MAXTICKS, MaxNLocator, MultipleLocator, Nbins,
    NullLocator, NumTicks, Prune, SymmetricalLogLocator, exceeds_max_ticks, scale_range,
};
pub use settings::{AutolimitMode, MinorDivisions, TickerSettings};
