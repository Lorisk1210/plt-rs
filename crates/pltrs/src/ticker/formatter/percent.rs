//! `PercentFormatter`: values as percentages of a reference value.

use super::{FormattedTicks, Formatter};
use crate::ticker::pyfmt::fixed;
use crate::ticker::pymath::ceil_int;
use crate::ticker::{TickContext, TickerError, TickerSettings};

/// Labels `x` as `100 * x / xmax` followed by a symbol (matplotlib's
/// `PercentFormatter`).
///
/// ```
/// use pltrs::ticker::{Formatter, PercentFormatter, TickContext, AxisInfo, TickerSettings};
/// let f = PercentFormatter::new(1.0).with_decimals(Some(0));
/// let axis = AxisInfo::with_view(0.0, 1.0);
/// let settings = TickerSettings::default();
/// let ctx = TickContext::new(&axis, &settings);
/// assert_eq!(f.format_ticks(&[0.25], &ctx).unwrap().labels, ["25%"]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PercentFormatter {
    xmax: f64,
    decimals: Option<i32>,
    symbol: Option<String>,
    is_latex: bool,
}

impl Default for PercentFormatter {
    /// Data already in percent (`xmax = 100`).
    fn default() -> Self {
        Self::new(100.0)
    }
}

impl PercentFormatter {
    /// `xmax` is the data value that corresponds to 100%.
    pub fn new(xmax: f64) -> Self {
        Self {
            xmax,
            decimals: None,
            symbol: Some("%".to_owned()),
            is_latex: false,
        }
    }

    /// Digits after the decimal point (`None`: derived from the view).
    #[must_use]
    pub fn with_decimals(mut self, decimals: Option<i32>) -> Self {
        self.decimals = decimals;
        self
    }

    /// The appended symbol (`None` or empty for none).
    #[must_use]
    pub fn with_symbol(mut self, symbol: Option<String>) -> Self {
        self.symbol = symbol;
        self
    }

    /// Whether the symbol is already valid LaTeX (otherwise its special
    /// characters are escaped when `text.usetex` is on).
    #[must_use]
    pub fn with_is_latex(mut self, is_latex: bool) -> Self {
        self.is_latex = is_latex;
        self
    }

    /// The value that corresponds to 100%.
    pub fn xmax(&self) -> f64 {
        self.xmax
    }

    /// The configured number of decimals.
    pub fn decimals(&self) -> Option<i32> {
        self.decimals
    }

    /// The symbol as it is appended under `settings`.
    pub fn symbol(&self, settings: &TickerSettings) -> String {
        let Some(symbol) = self.symbol.as_deref().filter(|s| !s.is_empty()) else {
            return String::new();
        };
        if self.is_latex || !settings.text_usetex {
            return symbol.to_owned();
        }
        // Backslash first, so the escapes added later are kept.
        r"\#$%&~_^{}"
            .chars()
            .fold(symbol.to_owned(), |text, special| {
                text.replace(special, &format!("\\{special}"))
            })
    }

    fn to_percent(&self, x: f64) -> f64 {
        100.0 * (x / self.xmax)
    }

    /// Decimals for a view spanning `display_range`: 0 above 50%, 1 above
    /// 5%, ... up to 5.
    fn automatic_decimals(&self, display_range: f64) -> Result<i64, TickerError> {
        let scaled_range = self.to_percent(display_range);
        if scaled_range <= 0.0 {
            return Ok(0);
        }
        Ok(ceil_int(2.0 - (2.0 * scaled_range).log10(), "percent range")?.clamp(0, 5))
    }

    /// The label of `x` on an axis spanning `display_range` data units,
    /// before minus signs are replaced (matplotlib's `format_pct`): `x` as
    /// a percentage of `xmax` with the configured decimals, or with as
    /// many as the range needs, followed by the symbol.
    ///
    /// ```
    /// use pltrs::ticker::{PercentFormatter, TickerSettings};
    /// let settings = TickerSettings::default();
    /// let pct = PercentFormatter::new(17.0);
    /// assert_eq!(pct.format_pct(1.0, 8.4, &settings).unwrap(), "5.9%");
    /// ```
    ///
    /// # Errors
    ///
    /// When the decimals cannot be determined (a non-finite range).
    pub fn format_pct(
        &self,
        x: f64,
        display_range: f64,
        settings: &TickerSettings,
    ) -> Result<String, TickerError> {
        let decimals = match self.decimals {
            Some(decimals) => i64::from(decimals),
            None => self.automatic_decimals(display_range)?,
        };
        let decimals = usize::try_from(decimals)
            .map_err(|_| TickerError::invalid("decimals", "must not be negative"))?;
        Ok(fixed(self.to_percent(x), decimals) + &self.symbol(settings))
    }

    fn label(&self, x: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        let (a, b) = ctx.view_interval();
        let settings = ctx.settings();
        Ok(settings.fix_minus(&self.format_pct(x, (b - a).abs(), settings)?))
    }
}

impl Formatter for PercentFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        FormattedTicks::each(locs, |x| self.label(x, ctx))
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        self.label(value, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::AxisInfo;

    fn labels(f: &PercentFormatter, locs: &[f64], view: (f64, f64), usetex: bool) -> Vec<String> {
        let axis = AxisInfo::with_view(view.0, view.1);
        let settings = TickerSettings {
            text_usetex: usetex,
            ..TickerSettings::DEFAULT
        };
        f.format_ticks(locs, &TickContext::new(&axis, &settings))
            .unwrap()
            .labels
    }

    #[test]
    fn automatic_decimals() {
        let f = PercentFormatter::default();
        assert_eq!(labels(&f, &[34.5], (0.0, 100.0), false), ["34%"]);
        assert_eq!(labels(&f, &[34.5], (30.0, 40.0), false), ["34.5%"]);
        assert_eq!(labels(&f, &[-50.0], (-50.0, 50.0), false), ["\u{2212}50%"]);
        assert_eq!(labels(&f, &[1.0], (1.0, 1.0), false), ["1%"]);
    }

    #[test]
    fn symbols_and_latex() {
        let f = PercentFormatter::default().with_symbol(Some("50% $".into()));
        assert_eq!(labels(&f, &[10.0], (0.0, 100.0), true), ["1050\\% \\$"]);
        let latex = f.clone().with_is_latex(true);
        assert_eq!(labels(&latex, &[10.0], (0.0, 100.0), true), ["1050% $"]);
        let none = PercentFormatter::default().with_symbol(None);
        assert_eq!(labels(&none, &[10.0], (0.0, 100.0), false), ["10"]);
    }
}
