//! Tick formatters: what tick labels say.

mod eng;
mod locale;
mod log;
mod logit;
mod percent;
mod scalar;
mod simple;

use std::any::Any;
use std::fmt;
use std::rc::Rc;

pub use eng::{EngFormatter, SI_PREFIXES};
pub use locale::{Grouping, NumericLocale};
pub use log::{LogFormatter, LogLabelStyle};
pub use logit::LogitFormatter;
pub use percent::PercentFormatter;
pub(crate) use scalar::g_sig_digits;
pub use scalar::{OffsetMode, ScalarFormatter};
pub use simple::{
    FixedFormatter, FormatStrFormatter, FuncFormatter, NullFormatter, StrMethodFormatter,
};

use super::{Locator, TickContext, TickerError};

/// The labels for a set of ticks plus the axis offset text (e.g. `1e6` or
/// `+1e5`) that applies to all of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormattedTicks {
    /// One label per tick location (possibly empty strings).
    pub labels: Vec<String>,
    /// The offset/scale text shown at the end of the axis.
    pub offset: String,
}

impl FormattedTicks {
    /// The labels `label` gives each of `locs` on its own, without an
    /// offset text.
    pub(crate) fn each(
        locs: &[f64],
        mut label: impl FnMut(f64) -> Result<String, TickerError>,
    ) -> Result<Self, TickerError> {
        Ok(Self {
            labels: locs.iter().map(|&x| label(x)).collect::<Result<_, _>>()?,
            offset: String::new(),
        })
    }
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
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError>;

    /// A context-independent representation of one value (used for
    /// offsets and scientific-notation strings).
    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError>;

    /// A short plain-text representation for the interactive cursor
    /// readout, with precision adapted to the view.
    fn format_data_short(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        self.format_data(value, ctx)
    }

    /// A copy of this formatter following `locator`, for formatters that
    /// base their labels on their axis' locator; `None` for the others,
    /// which are left as they are. An axis given a new locator hands it
    /// to its formatter (matplotlib's `Formatter._set_locator`, called by
    /// `Axis.set_major_locator`).
    fn following(&self, _locator: &Rc<dyn Locator>) -> Option<Rc<dyn Formatter>> {
        None
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

pltrs_core::archive_kinds!(dyn Formatter {
    "scalar" => ScalarFormatter,
    "eng" => EngFormatter,
    "logit" => LogitFormatter,
    "percent" => PercentFormatter,
    "null" => NullFormatter,
    "fixed" => FixedFormatter,
    "func" => FuncFormatter,
    "format-str" => FormatStrFormatter,
    "str-method" => StrMethodFormatter,
    "log" => LogFormatter,
    "theta" => crate::polar::ThetaFormatter,
    "category" => crate::units::CategoryFormatter,
    "geo-theta" => crate::geo::GeoThetaFormatter,
    "concise-date" => crate::dates::ConciseDateFormatter,
    "date" => crate::dates::DateFormatter,
    "auto-date" => crate::dates::AutoDateFormatter,
});

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
