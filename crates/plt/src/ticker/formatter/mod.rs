//! Tick formatters: what tick labels say.

mod eng;
mod log;
mod logit;
mod percent;
mod scalar;
mod simple;

use std::any::Any;
use std::fmt;

pub use eng::EngFormatter;
pub use log::{LogFormatter, LogLabelStyle};
pub use logit::LogitFormatter;
pub use percent::PercentFormatter;
pub use scalar::{OffsetMode, ScalarFormatter};
pub use simple::{
    FixedFormatter, FormatStrFormatter, FuncFormatter, NullFormatter, StrMethodFormatter,
};

use super::{TickContext, TickerError};

/// The labels for a set of ticks plus the axis offset text (e.g. `1e6` or
/// `+1e5`) that applies to all of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TickLabels {
    /// One label per tick location (possibly empty strings).
    pub labels: Vec<String>,
    /// The offset/scale text shown at the end of the axis.
    pub offset: String,
}

/// Turns tick values into label strings (matplotlib's `Formatter`).
///
/// Formatters hold only configuration: everything that matplotlib's
/// `set_locs` stores (offsets, precision, which minor ticks get labels) is
/// recomputed by [`Formatter::format_ticks`] from the full set of ticks.
/// Labels may contain mathtext (`$...$`).
pub trait Formatter: fmt::Debug + Any {
    /// Labels for all ticks at `locs` (matplotlib's `format_ticks` followed
    /// by `get_offset`).
    fn format_ticks(&self, locs: &[f64], ctx: &TickContext<'_>) -> Result<TickLabels, TickerError>;

    /// A context-independent representation of one value (used for
    /// offsets and scientific-notation strings).
    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError>;

    /// A short plain-text representation for the interactive cursor
    /// readout, with precision adapted to the view.
    fn format_data_short(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        self.format_data(value, ctx)
    }
}

/// matplotlib's `cbook.strip_math`: removes the `$...$` wrapper and the
/// common TeX commands from a fully-mathtext string.
pub fn strip_math(text: &str) -> String {
    let Some(inner) = text
        .strip_prefix('$')
        .and_then(|rest| rest.strip_suffix('$'))
    else {
        return text.to_owned();
    };
    let mut out = inner.replace(r"\times", "x");
    for command in [
        r"\mathdefault",
        r"\rm",
        r"\cal",
        r"\tt",
        r"\it",
        "\\",
        "{",
        "}",
    ] {
        out = out.replace(command, "");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_math_removes_markup() {
        assert_eq!(strip_math(r"$\mathdefault{10^{3}}$"), "10^3");
        assert_eq!(strip_math(r"$2\times10^{-1}$"), "2x10^-1");
        assert_eq!(strip_math("plain"), "plain");
        assert_eq!(strip_math("$"), "$");
    }
}
