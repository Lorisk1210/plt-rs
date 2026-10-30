//! `ScalarFormatter`: plain numbers with automatic offset and scientific
//! notation.

use pltrs_core::numeric::{floor_div, remainder};

use super::{FormattedTicks, Formatter};
use crate::ticker::pyfmt::{fixed, general, general_alternate, integer};
use crate::ticker::pymath::{floor_int, np_round, pow10, pow10_int, py_round, spacing};
use crate::ticker::{TickContext, TickerError, TickerSettings};

/// Whether a [`ScalarFormatter`] subtracts an offset from tick values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OffsetMode {
    /// Use an offset when it saves at least `axes.formatter.offset_threshold`
    /// digits (matplotlib's `useOffset=True`).
    Auto,
    /// Never use an offset.
    Off,
    /// Always subtract this offset.
    Fixed(f64),
}

impl OffsetMode {
    /// matplotlib's `useOffset` attribute: `true` only in automatic mode.
    pub fn is_auto(self) -> bool {
        self == Self::Auto
    }

    /// The configured fixed offset (0 unless [`OffsetMode::Fixed`]).
    pub fn fixed_offset(self) -> f64 {
        match self {
            Self::Fixed(offset) => offset,
            Self::Auto | Self::Off => 0.0,
        }
    }
}

/// How a set of ticks is written: `(x - offset) / 10^oom` with `decimals`
/// digits after the point (matplotlib's `set_locs` state).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ScalarLayout {
    pub(super) offset: f64,
    pub(super) oom: i32,
    pub(super) decimals: usize,
}

impl ScalarLayout {
    /// One tick label (matplotlib's `ScalarFormatter.__call__`).
    pub(super) fn label(&self, x: f64, math: bool, settings: &TickerSettings) -> String {
        let mut xp = (x - self.offset) / pow10(f64::from(self.oom));
        if xp.abs() < 1e-8 {
            xp = 0.0;
        }
        let text = fixed(xp, self.decimals);
        settings.fix_minus(&if math {
            format!("$\\mathdefault{{{text}}}$")
        } else {
            text
        })
    }
}

/// The ticks inside the (sorted) view interval.
fn visible(locs: &[f64], view: (f64, f64)) -> impl Iterator<Item = f64> + '_ {
    let (vmin, vmax) = if view.1 < view.0 {
        (view.1, view.0)
    } else {
        view
    };
    locs.iter()
        .copied()
        .filter(move |&l| vmin <= l && l <= vmax)
}

/// matplotlib's `_compute_offset`: an offset that saves at least
/// `threshold` digits, or 0.
pub(super) fn compute_offset(locs: &[f64], view: (f64, f64), threshold: i32) -> f64 {
    let (lmin, lmax) = visible(locs, view)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), l| {
            (lo.min(l), hi.max(l))
        });
    // No visible ticks, a single one, or ticks of both signs: no offset.
    if lmin >= lmax || (lmin <= 0.0 && 0.0 <= lmax) {
        return 0.0;
    }
    let (abs_min, abs_max) = if lmin.abs() <= lmax.abs() {
        (lmin.abs(), lmax.abs())
    } else {
        (lmax.abs(), lmin.abs())
    };
    let sign = 1.0_f64.copysign(lmin);
    let oom_max = abs_max.log10().ceil();
    // The smallest power of ten at which abs_min and abs_max agree; the
    // search is bounded by the range of doubles.
    let first_oom = |differs: &dyn Fn(f64) -> bool| {
        (0..2200)
            .map(|k| oom_max - f64::from(k))
            .find(|&oom| differs(pow10(oom)))
            .map(|oom| oom + 1.0)
    };
    let Some(mut oom) = first_oom(&|p| floor_div(abs_min, p) != floor_div(abs_max, p)) else {
        return 0.0;
    };
    if (abs_max - abs_min) / pow10(oom) <= 1e-2 {
        // Straddling a multiple of a large power of ten: use the smallest
        // power at which they are no more than 1 apart.
        let Some(found) = first_oom(&|p| floor_div(abs_max, p) - floor_div(abs_min, p) > 1.0)
        else {
            return 0.0;
        };
        oom = found;
    }
    let scale = pow10(oom);
    let leading = floor_div(abs_max, scale);
    if leading >= pow10(f64::from(threshold - 1)) {
        sign * leading * scale
    } else {
        0.0
    }
}

/// matplotlib's `_set_format`: the number of decimals needed to tell the
/// scaled ticks apart.
pub(super) fn decimals(
    locs: &[f64],
    view: (f64, f64),
    offset: f64,
    oom: i32,
) -> Result<usize, TickerError> {
    let scale = pow10(f64::from(oom));
    let scaled = |l: f64| (l - offset) / scale;
    // With fewer than two ticks the view limits help estimate the range.
    let mut range_values: Vec<f64> = locs.iter().copied().map(scaled).collect();
    if locs.len() < 2 {
        range_values.extend([scaled(view.0), scaled(view.1)]);
    }
    let nan_fold = |init: f64, pick: fn(f64, f64) -> f64| {
        range_values.iter().fold(init, |acc, &v| {
            if v.is_nan() || acc.is_nan() {
                f64::NAN
            } else {
                pick(acc, v)
            }
        })
    };
    let mut loc_range = nan_fold(f64::NEG_INFINITY, f64::max) - nan_fold(f64::INFINITY, f64::min);
    if loc_range == 0.0 {
        // Curvilinear coordinates can yield two identical points.
        loc_range = range_values.iter().fold(0.0_f64, |acc, v| {
            if v.is_nan() || acc.is_nan() {
                f64::NAN
            } else {
                acc.max(v.abs())
            }
        });
    }
    if loc_range == 0.0 {
        loc_range = 1.0;
    }
    let range_oom = floor_int(loc_range.log10(), "tick range")?;
    let thresh = 1e-3 * pow10(range_oom as f64);
    let mut sigfigs = (3 - range_oom).max(0);
    let values = &range_values[..locs.len()];
    while sigfigs >= 0 {
        let worst = values.iter().fold(0.0_f64, |acc, &l| {
            let err = (l - np_round(l, sigfigs as i32)).abs();
            if err.is_nan() || acc.is_nan() {
                f64::NAN
            } else {
                acc.max(err)
            }
        });
        if worst < thresh {
            sigfigs -= 1;
        } else {
            break;
        }
    }
    Ok((sigfigs + 1) as usize)
}

/// Formats ticks as plain numbers, factoring out an offset (`+1e5`) and/or
/// a power of ten (`1e6`) shown at the axis end (matplotlib's
/// `ScalarFormatter`).
///
/// ```
/// use pltrs::ticker::{Formatter, ScalarFormatter, TickContext, AxisInfo, TickerSettings};
/// let axis = AxisInfo::with_view(1e5, 1e5 + 5.0);
/// let settings = TickerSettings::default();
/// let ctx = TickContext::new(&axis, &settings);
/// let locs: Vec<f64> = (0..6).map(|i| 1e5 + f64::from(i)).collect();
/// let labels = ScalarFormatter::new().format_ticks(&locs, &ctx).unwrap();
/// assert_eq!(labels.labels, ["0", "1", "2", "3", "4", "5"]);
/// assert_eq!(labels.offset, "+1e5");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarFormatter {
    use_offset: OffsetMode,
    use_mathtext: bool,
    usetex: bool,
    scientific: bool,
    powerlimits: (i32, i32),
    offset_threshold: i32,
}

impl ScalarFormatter {
    /// A formatter configured from matplotlib's default rcParams.
    pub fn new() -> Self {
        Self::from_settings(&TickerSettings::DEFAULT)
    }

    /// A formatter configured from `settings` (`axes.formatter.*`,
    /// `text.usetex`).
    pub fn from_settings(settings: &TickerSettings) -> Self {
        Self {
            use_offset: if settings.axes_formatter_useoffset {
                OffsetMode::Auto
            } else {
                OffsetMode::Off
            },
            use_mathtext: settings.axes_formatter_use_mathtext,
            usetex: settings.text_usetex,
            scientific: true,
            powerlimits: settings.axes_formatter_limits,
            offset_threshold: settings.axes_formatter_offset_threshold,
        }
    }

    /// Offset notation mode.
    #[must_use]
    pub fn with_use_offset(mut self, use_offset: OffsetMode) -> Self {
        self.use_offset = use_offset;
        self
    }

    /// Typesets scientific notation as mathtext (`1.2 \times 10^{3}`).
    #[must_use]
    pub fn with_use_mathtext(mut self, use_mathtext: bool) -> Self {
        self.use_mathtext = use_mathtext;
        self
    }

    /// Wraps labels in TeX math mode.
    #[must_use]
    pub fn with_usetex(mut self, usetex: bool) -> Self {
        self.usetex = usetex;
        self
    }

    /// Enables or disables scientific notation (`set_scientific`).
    #[must_use]
    pub fn with_scientific(mut self, scientific: bool) -> Self {
        self.scientific = scientific;
        self
    }

    /// Exponents `(min, max)` at or beyond which scientific notation is
    /// used; equal non-zero limits force that exponent (`set_powerlimits`).
    #[must_use]
    pub fn with_powerlimits(mut self, powerlimits: (i32, i32)) -> Self {
        self.powerlimits = powerlimits;
        self
    }

    /// The offset mode.
    pub fn use_offset(&self) -> OffsetMode {
        self.use_offset
    }

    /// Whether mathtext is used.
    pub fn use_mathtext(&self) -> bool {
        self.use_mathtext
    }

    /// Whether TeX math mode is used.
    pub fn usetex(&self) -> bool {
        self.usetex
    }

    /// Whether scientific notation is enabled.
    pub fn scientific(&self) -> bool {
        self.scientific
    }

    /// The scientific-notation exponent limits.
    pub fn powerlimits(&self) -> (i32, i32) {
        self.powerlimits
    }

    /// Digits an automatic offset must save.
    pub fn offset_threshold(&self) -> i32 {
        self.offset_threshold
    }

    fn math(&self) -> bool {
        self.use_mathtext || self.usetex
    }

    /// matplotlib's `_set_order_of_magnitude`.
    fn order_of_magnitude(
        &self,
        locs: &[f64],
        view: (f64, f64),
        offset: f64,
    ) -> Result<i32, TickerError> {
        if !self.scientific {
            return Ok(0);
        }
        let (lo, hi) = self.powerlimits;
        if lo == hi && lo != 0 {
            return Ok(lo);
        }
        let largest = visible(locs, view).map(f64::abs).reduce(f64::max);
        let Some(largest) = largest else {
            return Ok(0);
        };
        let oom = if offset != 0.0 {
            let span = (view.1 - view.0).abs();
            if span == 0.0 {
                return Err(TickerError::Domain("log10 of an empty view"));
            }
            floor_int(span.log10(), "view span")?
        } else if largest == 0.0 {
            0
        } else {
            floor_int(largest.log10(), "largest tick")?
        };
        let oom = i32::try_from(oom).map_err(|_| TickerError::NonFinite("order of magnitude"))?;
        Ok(if oom <= lo || oom >= hi { oom } else { 0 })
    }

    fn layout(&self, locs: &[f64], view: (f64, f64)) -> Result<ScalarLayout, TickerError> {
        let offset = match self.use_offset {
            OffsetMode::Auto => compute_offset(locs, view, self.offset_threshold),
            mode => mode.fixed_offset(),
        };
        let oom = self.order_of_magnitude(locs, view, offset)?;
        Ok(ScalarLayout {
            offset,
            oom,
            decimals: decimals(locs, view, offset, oom)?,
        })
    }

    /// matplotlib's `get_offset`.
    fn offset_text(
        &self,
        layout: &ScalarLayout,
        settings: &TickerSettings,
    ) -> Result<String, TickerError> {
        if layout.oom == 0 && layout.offset == 0.0 {
            return Ok(String::new());
        }
        let mut offset = String::new();
        if layout.offset != 0.0 {
            offset = self.format_value(layout.offset, settings)?;
            if layout.offset > 0.0 {
                offset.insert(0, '+');
            }
        }
        let mut sci = String::new();
        if layout.oom != 0 {
            sci = if self.math() {
                self.format_value(pow10_int(layout.oom), settings)?
            } else {
                format!("1e{}", layout.oom)
            };
        }
        let text = if self.math() {
            if !sci.is_empty() {
                sci = format!("\\times\\mathdefault{{{sci}}}");
            }
            format!("${sci}\\mathdefault{{{offset}}}$")
        } else {
            sci + &offset
        };
        Ok(settings.fix_minus(&text))
    }

    /// matplotlib's `ScalarFormatter.format_data`: `1.5e−7` or, with
    /// mathtext, `1.5 \times 10^{−7}`.
    fn format_value(&self, value: f64, settings: &TickerSettings) -> Result<String, TickerError> {
        if value == 0.0 {
            return Err(TickerError::Domain("log10(0)"));
        }
        let e = floor_int(value.abs().log10(), "value")?;
        let e = i32::try_from(e).map_err(|_| TickerError::NonFinite("exponent"))?;
        let s = py_round(value / pow10_int(e), 10);
        let significand = settings.fix_minus(&if remainder(s, 1.0) == 0.0 {
            integer(s)
        } else {
            general(s, 10)
        });
        if e == 0 {
            return Ok(significand);
        }
        let exponent = settings.fix_minus(&e.to_string());
        Ok(if self.math() {
            let power = format!("10^{{{exponent}}}");
            if s == 1.0 {
                power
            } else {
                format!("{significand} \\times {power}")
            }
        } else {
            format!("{significand}e{exponent}")
        })
    }
}

/// matplotlib's `cbook._g_sig_digits`: significant digits needed to show
/// `value` known to within `delta`.
pub(crate) fn g_sig_digits(value: f64, delta: f64) -> Result<usize, TickerError> {
    if !value.is_finite() {
        return Ok(0);
    }
    let delta = if delta == 0.0 {
        if value == 0.0 {
            return Ok(3);
        }
        spacing(value)
    } else {
        delta
    };
    if delta < 0.0 {
        return Err(TickerError::Domain("log10 of a negative resolution"));
    }
    let before = if value == 0.0 {
        1
    } else {
        floor_int(value.abs().log10(), "value")? + 1
    };
    let after = floor_int(delta.log10(), "resolution")?;
    Ok(usize::try_from(before - after).unwrap_or(0))
}

impl Default for ScalarFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for ScalarFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        if locs.is_empty() {
            return Ok(FormattedTicks::default());
        }
        let settings = ctx.settings();
        let layout = self.layout(locs, ctx.view_interval())?;
        Ok(FormattedTicks {
            labels: locs
                .iter()
                .map(|&x| layout.label(x, self.math(), settings))
                .collect(),
            offset: self.offset_text(&layout, settings)?,
        })
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        self.format_value(value, ctx.settings())
    }

    /// The value with as many significant digits as a ten-thousandth of
    /// the view resolves (`%-#.<n>g`).
    fn format_data_short(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        let (a, b) = ctx.view_interval();
        Self::format_data_resolved(value, (b - a) / 1e4, ctx.settings())
    }
}

impl ScalarFormatter {
    /// The offset subtracted from the labels of the ticks at `locs` and
    /// their order of magnitude, the power of ten they are divided by
    /// (matplotlib's `offset` and `orderOfMagnitude`, which `set_locs`
    /// computes).
    ///
    /// ```
    /// use pltrs::ticker::{AxisInfo, ScalarFormatter, TickContext, TickerSettings};
    /// let axis = AxisInfo::with_view(100_000.0, 100_010.0);
    /// let settings = TickerSettings::DEFAULT;
    /// let ctx = TickContext::new(&axis, &settings);
    /// let locs = [100_000.0, 100_005.0, 100_010.0];
    /// let formatter = ScalarFormatter::from_settings(&settings);
    /// assert_eq!(formatter.offset_and_order(&locs, &ctx)?, (100_000.0, 0));
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the order of magnitude cannot be computed (an empty view with
    /// an offset).
    pub fn offset_and_order(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<(f64, i32), TickerError> {
        if locs.is_empty() {
            return Ok((self.use_offset.fixed_offset(), 0));
        }
        let layout = self.layout(locs, ctx.view_interval())?;
        Ok((layout.offset, layout.oom))
    }

    /// The value with as many significant digits as needed to tell it
    /// from values `resolution` away (`%-#.<n>g`; matplotlib's
    /// `format_data_short` with an axis attached, where the resolution is
    /// the data size of a screen pixel).
    ///
    /// # Errors
    ///
    /// When the number of digits cannot be determined (a negative or
    /// non-finite resolution).
    pub fn format_data_resolved(
        value: f64,
        resolution: f64,
        settings: &TickerSettings,
    ) -> Result<String, TickerError> {
        let digits = g_sig_digits(value, resolution)?;
        Ok(settings.fix_minus(&general_alternate(value, digits)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::AxisInfo;

    fn format(formatter: &ScalarFormatter, locs: &[f64], view: (f64, f64)) -> FormattedTicks {
        let axis = AxisInfo::with_view(view.0, view.1);
        let settings = TickerSettings::default();
        formatter
            .format_ticks(locs, &TickContext::new(&axis, &settings))
            .unwrap()
    }

    #[test]
    fn plain_and_offset_labels() {
        let f = ScalarFormatter::new();
        let out = format(
            &f,
            &[0.0, 0.2, 0.4, 0.600_000_000_000_000_1, 0.8, 1.0],
            (0.0, 1.0),
        );
        assert_eq!(out.labels, ["0.0", "0.2", "0.4", "0.6", "0.8", "1.0"]);
        assert_eq!(out.offset, "");
        let out = format(&f, &[-1e5 - 5.0, -1e5], (-1e5 - 5.0, -1e5));
        assert_eq!(out.offset, "\u{2212}1e5");
        let out = format(&f, &[1e-7, 2e-7, 3e-7], (1e-7, 3e-7));
        assert_eq!(out.labels, ["1", "2", "3"]);
        assert_eq!(out.offset, "1e\u{2212}7");
    }

    #[test]
    fn nan_ticks_fail_like_matplotlib() {
        let f = ScalarFormatter::new();
        let axis = AxisInfo::with_view(0.0, 3.0);
        let settings = TickerSettings::default();
        let ctx = TickContext::new(&axis, &settings);
        assert!(f.format_ticks(&[f64::NAN, 1.0, 2.0], &ctx).is_err());
        assert_eq!(
            f.format_ticks(&[], &ctx).unwrap(),
            FormattedTicks::default()
        );
    }

    #[test]
    fn format_data_variants() {
        let ctx = TickContext::default();
        let f = ScalarFormatter::new();
        assert_eq!(f.format_data(123_456.0, &ctx).unwrap(), "1.23456e5");
        assert_eq!(f.format_data(-0.001, &ctx).unwrap(), "\u{2212}1e\u{2212}3");
        assert!(f.format_data(0.0, &ctx).is_err());
        let math = ScalarFormatter::new().with_use_mathtext(true);
        assert_eq!(math.format_data(1e6, &ctx).unwrap(), "10^{6}");
        assert_eq!(
            math.format_data(0.5, &ctx).unwrap(),
            "5 \\times 10^{\u{2212}1}"
        );
    }

    #[test]
    fn format_data_short_uses_view_resolution() {
        let f = ScalarFormatter::new();
        let axis = AxisInfo::with_view(0.0, 10.0);
        let settings = TickerSettings::default();
        let ctx = TickContext::new(&axis, &settings);
        assert_eq!(f.format_data_short(1.234_567_89, &ctx).unwrap(), "1.235");
        let axis = AxisInfo::with_view(0.0, 1e10);
        let ctx = TickContext::new(&axis, &settings);
        assert_eq!(f.format_data_short(2.0, &ctx).unwrap(), "2.");
    }
}
