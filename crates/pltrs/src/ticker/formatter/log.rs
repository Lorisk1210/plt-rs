//! The `LogFormatter` family: labels for log and symlog axes.

use pltrs_core::numeric::{py_max, py_min};

use super::{FormattedTicks, Formatter, strip_math};
use crate::ticker::pyfmt::{exponential, fixed, general, integer, repr};
use crate::ticker::pymath::{
    arange, floor_int, is_close_to_int, linspace, log_base, pow10, round_int,
};
use crate::ticker::{ScaleHint, TickContext, TickerError, TickerSettings};
use crate::transforms::nonsingular;

/// How a [`LogFormatter`] writes the labels it keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogLabelStyle {
    /// Plain numbers: `1`, `10`, `1e+04` (matplotlib's `LogFormatter`).
    Plain,
    /// The exponent only: `0`, `1`, `4` (`LogFormatterExponent`).
    Exponent,
    /// Powers in mathtext: `$\mathdefault{10^{4}}$`, non-decades as
    /// `10^{0.30}` (`LogFormatterMathtext`).
    Mathtext,
    /// Powers in mathtext, non-decades as `2\times10^{0}`
    /// (`LogFormatterSciNotation`), the default for log axes.
    #[default]
    SciNotation,
}

pltrs_core::archive_enum!(LogLabelStyle {
    Plain,
    Exponent,
    Mathtext,
    SciNotation,
});

/// Labels for logarithmic axes: all decades, plus a subset of the ticks
/// in between when few decades are visible (matplotlib's `LogFormatter`
/// family, selected by [`LogLabelStyle`]).
#[derive(Clone, Debug, PartialEq)]
pub struct LogFormatter {
    style: LogLabelStyle,
    base: f64,
    label_only_base: bool,
    minor_thresholds: (f64, f64),
    linthresh: Option<f64>,
}

pltrs_core::archive_struct!(LogFormatter {
    style,
    base,
    label_only_base,
    minor_thresholds,
    linthresh
});

impl LogFormatter {
    /// A formatter for powers of `base`, configured from matplotlib's
    /// default rcParams.
    pub fn new(style: LogLabelStyle, base: f64) -> Self {
        Self::from_settings(style, base, &TickerSettings::DEFAULT)
    }

    /// Like [`LogFormatter::new`]; classic mode disables labels between
    /// decades (`minor_thresholds = (0, 0)`).
    pub fn from_settings(style: LogLabelStyle, base: f64, settings: &TickerSettings) -> Self {
        Self {
            style,
            base,
            label_only_base: false,
            minor_thresholds: if settings.classic_mode {
                (0.0, 0.0)
            } else {
                (1.0, 0.4)
            },
            linthresh: None,
        }
    }

    /// Only label integer powers of the base.
    #[must_use]
    pub fn with_label_only_base(mut self, label_only_base: bool) -> Self {
        self.label_only_base = label_only_base;
        self
    }

    /// `(subset, all)`: ticks between decades are labelled when at most
    /// `subset` decades are labelled, all of them when the view spans at
    /// most `all` decades. `(inf, inf)` labels every tick.
    #[must_use]
    pub fn with_minor_thresholds(mut self, subset: f64, all: f64) -> Self {
        self.minor_thresholds = (subset, all);
        self
    }

    /// The `linthresh` of a symlog axis (otherwise taken from the axis).
    #[must_use]
    pub fn with_linthresh(mut self, linthresh: Option<f64>) -> Self {
        self.linthresh = linthresh;
        self
    }

    /// The label style.
    pub fn style(&self) -> LogLabelStyle {
        self.style
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// Whether only powers of the base are labelled.
    pub fn label_only_base(&self) -> bool {
        self.label_only_base
    }

    /// The `(subset, all)` minor thresholds.
    pub fn minor_thresholds(&self) -> (f64, f64) {
        self.minor_thresholds
    }

    /// The configured symlog threshold.
    pub fn linthresh(&self) -> Option<f64> {
        self.linthresh
    }

    /// The short form a plain log label gives `value` on an axis spanning
    /// `domain` (matplotlib's `LogFormatter._pprint_val`): integers below
    /// 10⁴ as they are, otherwise fixed or exponential notation with fewer
    /// digits the wider the domain, trailing zeros removed.
    ///
    /// ```
    /// use pltrs::ticker::LogFormatter;
    /// assert_eq!(LogFormatter::pprint_val(31.41592654, 0.001), "3.142e1");
    /// assert_eq!(LogFormatter::pprint_val(31.41592654, 5.0), "31.42");
    /// ```
    pub fn pprint_val(value: f64, domain: f64) -> String {
        pprint_val(value, domain)
    }

    /// The coefficients of the ticks that get labels on the axis of `ctx`,
    /// `None` when every tick does (what matplotlib's `set_locs` keeps as
    /// `_sublabels`).
    ///
    /// ```
    /// use pltrs::ticker::{AxisInfo, LogFormatter, TickContext, TickerSettings};
    /// let axis = AxisInfo::with_view(1.0, 1e6);
    /// let settings = TickerSettings::default();
    /// let ctx = TickContext::new(&axis, &settings);
    /// assert_eq!(LogFormatter::default().sublabels(&ctx)?, Some(vec![1.0]));
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the view is not finite.
    pub fn sublabels(&self, ctx: &TickContext<'_>) -> Result<Option<Vec<f64>>, TickerError> {
        let (subset, all) = self.minor_thresholds;
        if subset.is_infinite() {
            return Ok(None);
        }
        let axis = ctx.axis_or_dummy();
        let linthresh = self.linthresh.or(match axis.scale {
            ScaleHint::SymLog { linthresh } => Some(linthresh),
            _ => None,
        });
        let (v0, v1) = axis.view_interval;
        let (vmin, vmax) = if v0 > v1 { (v1, v0) } else { (v0, v1) };
        if linthresh.is_none() && vmin <= 0.0 {
            // Probably a colorbar: label only the powers of the base.
            return Ok(Some(vec![1.0]));
        }
        let b = self.base;
        let decade_floor = |x: f64| floor_int(log_base(x, b), "log of the view");
        let below = |x: f64| floor_int(log_base(x, b).next_down(), "log of the view");
        let (numticks, numdec) = if let Some(lt) = linthresh {
            // Only count the logarithmic parts of a symlog axis.
            let (mut numticks, mut numdec) = (0, 0.0);
            if vmin < -lt {
                let rhs = py_min(vmax, -lt);
                numticks += decade_floor(rhs.abs())? - below(vmin.abs())?;
                numdec += log_base(vmin / rhs, b);
            }
            if vmax > lt {
                let lhs = py_max(vmin, lt);
                numticks += decade_floor(vmax)? - below(lhs)?;
                numdec += log_base(vmax / lhs, b);
            }
            (numticks, numdec)
        } else {
            let (lmin, lmax) = (log_base(vmin, b), log_base(vmax, b));
            let numticks = floor_int(lmax, "log of the view")?
                - floor_int(lmin.next_down(), "log of the view")?;
            (numticks, (lmax - lmin).abs())
        };
        Ok(Some(if numticks as f64 > subset {
            vec![1.0]
        } else if numdec > all {
            // Log-spaced coefficients, e.g. 1, 2, 3, 4, 6, 10 for base 10.
            geomspace_coefficients(b)?
        } else {
            arange(1.0, b + 1.0, 1.0)?
        }))
    }

    /// The label of `x` when only the ticks with the coefficients
    /// `sublabels` get labels (all of them for `None`), as matplotlib's
    /// `__call__` writes it after a `set_locs` that kept them.
    ///
    /// ```
    /// use pltrs::ticker::{LogFormatter, TickContext};
    /// let ctx = TickContext::default();
    /// let f = LogFormatter::default();
    /// assert_eq!(f.call(2.0, None, &ctx)?, r"$\mathdefault{2\times10^{0}}$");
    /// assert_eq!(f.call(2.0, Some(&[1.0]), &ctx)?, "");
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` has no finite logarithm (NaN or infinite), as Python's
    /// `round` of NaN fails in matplotlib.
    pub fn call(
        &self,
        x: f64,
        sublabels: Option<&[f64]>,
        ctx: &TickContext<'_>,
    ) -> Result<String, TickerError> {
        self.label(x, sublabels, self.label_only_base, ctx)
    }

    /// One label (matplotlib's `__call__`).
    fn label(
        &self,
        x: f64,
        sublabels: Option<&[f64]>,
        label_only_base: bool,
        ctx: &TickContext<'_>,
    ) -> Result<String, TickerError> {
        let math = matches!(
            self.style,
            LogLabelStyle::Mathtext | LogLabelStyle::SciNotation
        );
        if x == 0.0 {
            return Ok(if math { r"$\mathdefault{0}$" } else { "0" }.to_owned());
        }
        let sign = if x < 0.0 { "-" } else { "" };
        let x = x.abs();
        let b = self.base;
        let fx = x.ln() / b.ln();
        if !fx.is_finite() {
            // matplotlib's `round` of the coefficient, `b ** (fx - exponent)`,
            // which is NaN for a value of no finite logarithm.
            return Err(TickerError::NotAnInteger { infinite: false });
        }
        let is_decade = is_close_to_int(fx);
        let exponent = if is_decade {
            fx.round_ties_even()
        } else {
            fx.floor()
        };
        let coeff = round_int(pltrs_core::numeric::pow(b, fx - exponent), "coefficient")? as f64;
        if label_only_base && !is_decade {
            return Ok(String::new());
        }
        if sublabels.is_some_and(|subs| !subs.contains(&coeff)) {
            return Ok(String::new());
        }
        if !math {
            let (vmin, vmax) = ctx.view_interval();
            let (vmin, vmax) = nonsingular(vmin, vmax, 0.05, 1e-15, true);
            let text = if self.style == LogLabelStyle::Exponent {
                self.exponent_text(x, vmax - vmin)
            } else if (1.0..=10000.0).contains(&x) {
                pprint_val(x, vmax - vmin)
            } else {
                exponential(x, 0)
            };
            return Ok(ctx.settings().fix_minus(&text));
        }
        let fx = if is_decade { fx.round_ties_even() } else { fx };
        let base = if b % 1.0 == 0.0 { integer(b) } else { repr(b) };
        Ok(
            if fx.abs() < f64::from(ctx.settings().axes_formatter_min_exponent) {
                format!("$\\mathdefault{{{sign}{}}}$", general(x, 6))
            } else if !is_decade {
                self.non_decade(sign, &base, fx)
            } else {
                format!("$\\mathdefault{{{sign}{base}^{{{}}}}}$", integer(fx))
            },
        )
    }

    /// `LogFormatterExponent._num_to_string`.
    fn exponent_text(&self, x: f64, span: f64) -> String {
        let fx = x.ln() / self.base.ln();
        if (1.0..=10000.0).contains(&fx.abs()) {
            pprint_val(fx, span.ln() / self.base.ln())
        } else {
            general(fx, 0)
        }
    }

    /// The label of a tick between decades.
    fn non_decade(&self, sign: &str, base: &str, fx: f64) -> String {
        if self.style == LogLabelStyle::Mathtext {
            return format!("$\\mathdefault{{{sign}{base}^{{{}}}}}$", fixed(fx, 2));
        }
        let exponent = fx.floor();
        let mut coeff = pltrs_core::numeric::pow(self.base, fx - exponent);
        if is_close_to_int(coeff) {
            coeff = coeff.round_ties_even();
        }
        format!(
            "$\\mathdefault{{{sign}{}\\times{base}^{{{}}}}}$",
            general(coeff, 6),
            integer(exponent)
        )
    }
}

impl Default for LogFormatter {
    /// `LogFormatterSciNotation(base=10)`.
    fn default() -> Self {
        Self::new(LogLabelStyle::SciNotation, 10.0)
    }
}

/// `set(np.round(np.geomspace(1, b, int(b) // 2 + 1)))`, refusing a base
/// so large that numpy could not hold the coefficients.
fn geomspace_coefficients(b: f64) -> Result<Vec<f64>, TickerError> {
    let num = (b.trunc() as usize) / 2 + 1;
    let exponents = linspace(0.0, b.log10(), num)?;
    let mut coeffs: Vec<f64> = exponents.iter().map(|&e| pow10(e)).collect();
    if let Some(first) = coeffs.first_mut() {
        *first = 1.0;
    }
    if num > 1
        && let Some(last) = coeffs.last_mut()
    {
        *last = b;
    }
    Ok(coeffs.iter().map(|c| c.round_ties_even()).collect())
}

/// `LogFormatter._pprint_val`: a short representation of `x` given the
/// span `d` of the view.
fn pprint_val(x: f64, d: f64) -> String {
    if x.abs() < 1e4 && x == x.trunc() {
        return integer(x);
    }
    let text = if d < 1e-2 {
        exponential(x, 3)
    } else if d <= 1.0 {
        fixed(x, 3)
    } else if d <= 10.0 {
        fixed(x, 2)
    } else if d <= 1e5 {
        fixed(x, 1)
    } else {
        exponential(x, 1)
    };
    let strip = |s: &str| s.trim_end_matches('0').trim_end_matches('.').to_owned();
    match text.split_once('e') {
        Some((mantissa, exponent)) => {
            let mantissa = strip(mantissa);
            match exponent.parse::<i32>() {
                Ok(0) | Err(_) => mantissa,
                Ok(e) => format!("{mantissa}e{e}"),
            }
        }
        None => strip(&text),
    }
}

impl Formatter for LogFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let sublabels = self.sublabels(ctx)?;
        FormattedTicks::each(locs, |x| {
            self.label(x, sublabels.as_deref(), self.label_only_base, ctx)
        })
    }

    /// The full label of any value, without mathtext markup.
    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(strip_math(&self.label(value, None, false, ctx)?))
    }

    /// `%g` of the value.
    fn format_data_short(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(general(value, 6))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::AxisInfo;

    fn labels(formatter: &LogFormatter, locs: &[f64], view: (f64, f64)) -> Vec<String> {
        let axis = AxisInfo::with_view(view.0, view.1);
        let settings = TickerSettings::default();
        formatter
            .format_ticks(locs, &TickContext::new(&axis, &settings))
            .unwrap()
            .labels
    }

    #[test]
    fn sci_notation_labels() {
        let f = LogFormatter::default();
        assert_eq!(
            labels(&f, &[1.0, 10.0, 100.0, 1000.0, 2.0], (1.0, 1000.0)),
            [
                r"$\mathdefault{10^{0}}$",
                r"$\mathdefault{10^{1}}$",
                r"$\mathdefault{10^{2}}$",
                r"$\mathdefault{10^{3}}$",
                ""
            ]
        );
        assert_eq!(
            labels(&f, &[2.0, 3.0], (1.0, 5.0)),
            [
                r"$\mathdefault{2\times10^{0}}$",
                r"$\mathdefault{3\times10^{0}}$"
            ]
        );
    }

    #[test]
    fn plain_and_exponent_labels() {
        let plain = LogFormatter::new(LogLabelStyle::Plain, 10.0);
        assert_eq!(
            labels(&plain, &[0.001, 1.0, 1e5], (0.001, 1e5)),
            ["1e\u{2212}03", "1", "1e+05"]
        );
        let exponent = LogFormatter::new(LogLabelStyle::Exponent, 10.0);
        assert_eq!(
            labels(&exponent, &[0.001, 1.0, 1e5], (0.001, 1e5)),
            ["\u{2212}3", "0", "5"]
        );
    }

    #[test]
    fn helpers() {
        assert_eq!(
            geomspace_coefficients(10.0).expect("coefficients"),
            [1.0, 2.0, 3.0, 4.0, 6.0, 10.0]
        );
        // Found by fuzzing: a base of 10^12 asked for half as many
        // coefficients, and the allocation's size overflowed.
        assert!(matches!(
            geomspace_coefficients(1e12),
            Err(TickerError::TooManyTicks(_))
        ));
        assert!(geomspace_coefficients(1e300).is_err());
        assert_eq!(pprint_val(2.5, 5.0), "2.5");
        assert_eq!(pprint_val(12345.0, 1e6), "1.2e4");
        assert_eq!(pprint_val(3.0, 1.0), "3");
    }
}
