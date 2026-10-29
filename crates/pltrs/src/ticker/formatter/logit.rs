//! `LogitFormatter`: probability labels such as `10^{-3}`, `1/2`, `1-10^{-2}`.

use std::cmp::Ordering;

use super::{FormattedTicks, Formatter};
use crate::ticker::pyfmt::{exponential, fixed};
use crate::ticker::pymath::{
    ceil_int, floor_int, is_close_to_int, np_isclose, pow10_int, round_int,
};
use crate::ticker::{TickContext, TickerError};

/// matplotlib's `_is_decade(x, rtol=1e-7)` for base 10.
fn is_decade(x: f64) -> bool {
    if !x.is_finite() {
        return false;
    }
    if x == 0.0 {
        return true;
    }
    let lx = x.abs().ln() / 10.0_f64.ln();
    np_isclose(lx, lx.round_ties_even(), 1e-7, 1e-8)
}

/// Whether `2x` is close to the integer 1, i.e. `x` is one half.
fn is_one_half(x: f64) -> bool {
    x.is_finite() && is_close_to_int(2.0 * x) && (2.0 * x).round_ties_even() == 1.0
}

/// Labels for a logit axis (matplotlib's `LogitFormatter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogitFormatter {
    use_overline: bool,
    one_half: String,
    minor: bool,
    minor_threshold: usize,
    minor_number: usize,
}

impl Default for LogitFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogitFormatter {
    /// A major-tick formatter writing `1-x` for probabilities above one half
    /// and `\frac{1}{2}` for one half.
    pub fn new() -> Self {
        Self {
            use_overline: false,
            one_half: r"\frac{1}{2}".to_owned(),
            minor: false,
            minor_threshold: 25,
            minor_number: 6,
        }
    }

    /// Writes `\overline{x}` instead of `1-x` above one half.
    #[must_use]
    pub fn with_use_overline(mut self, use_overline: bool) -> Self {
        self.use_overline = use_overline;
        self
    }

    /// The text used for one half.
    #[must_use]
    pub fn with_one_half(mut self, one_half: impl Into<String>) -> Self {
        self.one_half = one_half.into();
        self
    }

    /// Formats minor ticks: only a few, well-separated ones get labels.
    #[must_use]
    pub fn with_minor(mut self, minor: bool) -> Self {
        self.minor = minor;
        self
    }

    /// Minor ticks are only labelled when there are fewer than this many.
    #[must_use]
    pub fn with_minor_threshold(mut self, minor_threshold: usize) -> Self {
        self.minor_threshold = minor_threshold;
        self
    }

    /// How many minor ticks get labels.
    #[must_use]
    pub fn with_minor_number(mut self, minor_number: usize) -> Self {
        self.minor_number = minor_number;
        self
    }

    /// Whether `\overline` notation is used.
    pub fn use_overline(&self) -> bool {
        self.use_overline
    }

    /// The text for one half.
    pub fn one_half(&self) -> &str {
        &self.one_half
    }

    /// Whether this formats minor ticks.
    pub fn minor(&self) -> bool {
        self.minor
    }

    /// The minor-tick threshold.
    pub fn minor_threshold(&self) -> usize {
        self.minor_threshold
    }

    /// The number of labelled minor ticks.
    pub fn minor_number(&self) -> usize {
        self.minor_number
    }

    /// matplotlib's `set_locs`: the minor ticks that get labels.
    fn labelled(&self, locs: &[f64]) -> Vec<f64> {
        let ideal = |x: f64| is_decade(x) || is_decade(1.0 - x) || is_one_half(x);
        if locs.iter().all(|&x| ideal(x)) || locs.len() >= self.minor_threshold {
            // Minor ticks subsampled from the ideal ones get no labels.
            return Vec::new();
        }
        if locs.len() < self.minor_number {
            return locs.to_vec();
        }
        // Label the ticks with the most room around them (smallest gap to
        // a neighbour, then the sum of both gaps).
        let logits: Vec<f64> = locs.iter().map(|x| -(1.0 / x - 1.0).ln()).collect();
        let diff: Vec<f64> = logits.windows(2).map(|w| w[1] - w[0]).collect();
        let gap = |i: usize, fallback: f64| -> (f64, f64) {
            let left = i.checked_sub(1).map_or(fallback, |j| diff[j]);
            let right = diff.get(i).copied().unwrap_or(fallback);
            (left, right)
        };
        let key = |i: usize| {
            let (left, right) = gap(i, f64::INFINITY);
            let pessimistic = if left.is_nan() || right.is_nan() {
                f64::NAN
            } else {
                left.min(right)
            };
            let (left, right) = gap(i, 0.0);
            (pessimistic, left + right)
        };
        let mut order: Vec<usize> = (0..locs.len()).collect();
        order.sort_by(|&a, &b| {
            let (ka, kb) = (key(a), key(b));
            ka.0.partial_cmp(&kb.0)
                .unwrap_or(Ordering::Equal)
                .then(ka.1.partial_cmp(&kb.1).unwrap_or(Ordering::Equal))
        });
        // Python's `[-0:]` keeps everything.
        let start = order.len().saturating_sub(self.minor_number);
        let start = if self.minor_number == 0 { 0 } else { start };
        order[start..].iter().map(|&i| locs[i]).collect()
    }

    fn one_minus(&self, text: &str) -> String {
        if self.use_overline {
            format!("\\overline{{{text}}}")
        } else {
            format!("1-{text}")
        }
    }

    /// One label (matplotlib's `__call__`).
    fn label(&self, x: f64, locs: &[f64], labelled: &[f64]) -> Result<String, TickerError> {
        if self.minor && !labelled.contains(&x) {
            return Ok(String::new());
        }
        if x <= 0.0 || x >= 1.0 {
            return Ok(String::new());
        }
        if !x.is_finite() {
            return Err(TickerError::NonFinite("tick value"));
        }
        let text = if is_one_half(x) {
            self.one_half.clone()
        } else if x < 0.5 && is_decade(x) {
            format!("10^{{{}}}", round_int(x.log10(), "decade")?)
        } else if x > 0.5 && is_decade(1.0 - x) {
            self.one_minus(&format!(
                "10^{{{}}}",
                round_int((1.0 - x).log10(), "decade")?
            ))
        } else if x < 0.1 {
            format_value(x, locs, true)?
        } else if x > 0.9 {
            let complements: Vec<f64> = locs.iter().map(|l| 1.0 - l).collect();
            self.one_minus(&format_value(1.0 - x, &complements, true)?)
        } else {
            format_value(x, locs, false)?
        };
        Ok(format!("$\\mathdefault{{{text}}}$"))
    }
}

/// matplotlib's `LogitFormatter._format_value`: enough digits to tell `x`
/// from its nearest neighbour in `locs`.
fn format_value(x: f64, locs: &[f64], sci_notation: bool) -> Result<String, TickerError> {
    let (exponent, min_precision) = if sci_notation {
        (floor_int(x.log10(), "tick value")?, 0)
    } else {
        (0, 1)
    };
    let exponent_i32 =
        i32::try_from(exponent).map_err(|_| TickerError::NonFinite("tick exponent"))?;
    let value = x * pow10_int(-exponent_i32);
    let precision = if locs.len() < 2 {
        min_precision
    } else {
        let mut distances: Vec<f64> = locs.iter().map(|l| (l - x).abs()).collect();
        distances.sort_by(f64::total_cmp);
        let p = -distances[1].log10() + exponent as f64;
        if !p.is_finite() {
            return Err(TickerError::NonFinite("label precision"));
        }
        let p = if is_close_to_int(p) {
            round_int(p, "label precision")?
        } else {
            ceil_int(p, "label precision")?
        };
        p.max(min_precision)
    };
    let mantissa = fixed(value, usize::try_from(precision).unwrap_or(0));
    Ok(if sci_notation {
        format!("{mantissa}\\cdot10^{{{exponent}}}")
    } else {
        mantissa
    })
}

impl Formatter for LogitFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let labelled = if self.minor {
            self.labelled(locs)
        } else {
            Vec::new()
        };
        FormattedTicks::each(locs, |x| self.label(x, locs, &labelled))
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        self.label(value, &[], &[])
    }

    /// Scientific notation for probabilities near 0 or 1.
    fn format_data_short(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(if value < 0.1 {
            exponential(value, 6)
        } else if value < 0.9 {
            fixed(value, 6)
        } else {
            format!("1-{}", exponential(1.0 - value, 6))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(formatter: &LogitFormatter, locs: &[f64]) -> Vec<String> {
        formatter
            .format_ticks(locs, &TickContext::default())
            .unwrap()
            .labels
    }

    #[test]
    fn ideal_ticks() {
        let f = LogitFormatter::new();
        assert_eq!(
            labels(&f, &[0.01, 0.5, 0.99, 0.0]),
            [
                r"$\mathdefault{10^{-2}}$",
                r"$\mathdefault{\frac{1}{2}}$",
                r"$\mathdefault{1-10^{-2}}$",
                ""
            ]
        );
        let overline = LogitFormatter::new().with_use_overline(true);
        assert_eq!(
            labels(&overline, &[0.999]),
            [r"$\mathdefault{\overline{10^{-3}}}$"]
        );
    }

    #[test]
    fn non_ideal_ticks() {
        let f = LogitFormatter::new();
        assert_eq!(
            labels(&f, &[0.001, 0.002, 0.003]),
            [
                r"$\mathdefault{10^{-3}}$",
                r"$\mathdefault{2\cdot10^{-3}}$",
                r"$\mathdefault{3\cdot10^{-3}}$"
            ]
        );
        assert_eq!(labels(&f, &[0.25]), [r"$\mathdefault{0.2}$"]);
        let minor = LogitFormatter::new().with_minor(true);
        assert_eq!(
            labels(&minor, &[0.2, 0.3]),
            [r"$\mathdefault{0.2}$", r"$\mathdefault{0.3}$"]
        );
    }

    #[test]
    fn short_format() {
        let f = LogitFormatter::new();
        let ctx = TickContext::default();
        assert_eq!(f.format_data_short(0.5, &ctx).unwrap(), "0.500000");
        assert_eq!(f.format_data_short(0.01, &ctx).unwrap(), "1.000000e-02");
        assert_eq!(
            f.format_data_short(0.99, &ctx).unwrap(),
            format!("1-{}", exponential(1.0 - 0.99, 6))
        );
    }
}
