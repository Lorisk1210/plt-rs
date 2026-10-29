//! Logarithmic scales: `LogScale` and `FuncScaleLog`.

use std::rc::Rc;

use super::{
    AxisTickers, FuncLogTransform, FuncTransform, LogTransform, NonPositive, Scale, ScaleError,
    ScaleName,
};
use crate::ticker::{
    AxisName, LogBase, LogFormatter, LogLabelStyle, LogLocator, LogSubs, ScaleHint, TickerSettings,
};
use crate::transforms::AxisMap;

/// Restricts a range to positive values (matplotlib's
/// `LogScale.limit_range_for_scale`).
fn positive_range(vmin: f64, vmax: f64, minpos: f64) -> (f64, f64) {
    let minpos = if minpos.is_finite() { minpos } else { 1e-300 };
    (
        if vmin <= 0.0 { minpos } else { vmin },
        if vmax <= 0.0 { minpos } else { vmax },
    )
}

/// The tick helpers of a log axis with the given base and minor `subs`.
fn log_tickers(base: LogBase, subs: Option<&[f64]>, settings: &TickerSettings) -> AxisTickers {
    let locator = LogLocator::from_settings(base, settings);
    let minor_subs = subs.map_or(LogSubs::Auto, |subs| LogSubs::Explicit(subs.to_vec()));
    let formatter = LogFormatter::from_settings(LogLabelStyle::SciNotation, base.get(), settings);
    AxisTickers {
        minor_locator: Box::new(locator.clone().with_subs(minor_subs)),
        major_locator: Box::new(locator),
        minor_formatter: Box::new(formatter.clone().with_label_only_base(subs.is_some())),
        major_formatter: Box::new(formatter),
    }
}

/// A logarithmic scale; only positive values are shown (matplotlib's
/// `LogScale`).
#[derive(Clone, Debug, PartialEq)]
pub struct LogScale {
    transform: LogTransform,
    subs: Option<Vec<f64>>,
}

impl LogScale {
    /// A log scale with `base` (positive, not 1) that clips non-positive
    /// values.
    pub fn new(base: f64) -> Result<Self, ScaleError> {
        Ok(Self::with_transform(LogTransform::new(
            base,
            NonPositive::Clip,
        )?))
    }

    /// A log scale in `base` that clips non-positive values.
    pub fn from_base(base: LogBase) -> Self {
        Self::with_transform(LogTransform::from_base(base, NonPositive::Clip))
    }

    fn with_transform(transform: LogTransform) -> Self {
        Self {
            transform,
            subs: None,
        }
    }

    /// How non-positive values are treated.
    #[must_use]
    pub fn with_nonpositive(mut self, nonpositive: NonPositive) -> Self {
        self.transform = LogTransform::from_base(self.transform.log_base(), nonpositive);
        self
    }

    /// Multiples of each power where minor ticks go, in an array or a
    /// `Vec`, in place of the automatic ones.
    #[must_use]
    pub fn with_subs(mut self, subs: impl Into<Vec<f64>>) -> Self {
        self.subs = Some(subs.into());
        self
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.transform.base()
    }

    /// The minor tick multiples.
    pub fn subs(&self) -> Option<&[f64]> {
        self.subs.as_deref()
    }

    /// The underlying transform.
    pub fn transform(&self) -> LogTransform {
        self.transform
    }
}

impl Scale for LogScale {
    fn name(&self) -> ScaleName {
        ScaleName::Log
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(self.transform))
    }

    fn limit_range_for_scale(&self, vmin: f64, vmax: f64, minpos: f64) -> (f64, f64) {
        positive_range(vmin, vmax, minpos)
    }

    fn val_in_range(&self, value: f64) -> bool {
        value.is_finite() && value > 0.0
    }

    fn hint(&self) -> ScaleHint {
        ScaleHint::Log
    }

    fn default_tickers(&self, _: AxisName, settings: &TickerSettings) -> AxisTickers {
        log_tickers(self.transform.log_base(), self.subs(), settings)
    }
}

/// A user function followed by a logarithm, ticked like a log axis
/// (matplotlib's `FuncScaleLog`).
#[derive(Clone, Debug)]
pub struct FuncScaleLog {
    transform: FuncLogTransform,
}

impl FuncScaleLog {
    /// `log_base(func(x))`; `base` must be positive and not 1.
    pub fn new(func: FuncTransform, base: f64) -> Result<Self, ScaleError> {
        Ok(Self {
            transform: FuncLogTransform::new(func, LogTransform::new(base, NonPositive::Clip)?),
        })
    }

    /// `log_base(func(x))` in `base`.
    pub fn from_base(func: FuncTransform, base: LogBase) -> Self {
        Self {
            transform: FuncLogTransform::new(
                func,
                LogTransform::from_base(base, NonPositive::Clip),
            ),
        }
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.transform.log().base()
    }
}

impl Scale for FuncScaleLog {
    fn name(&self) -> ScaleName {
        ScaleName::FunctionLog
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(self.transform.clone()))
    }

    fn limit_range_for_scale(&self, vmin: f64, vmax: f64, minpos: f64) -> (f64, f64) {
        positive_range(vmin, vmax, minpos)
    }

    fn val_in_range(&self, value: f64) -> bool {
        value.is_finite() && value > 0.0
    }

    fn default_tickers(&self, _: AxisName, settings: &TickerSettings) -> AxisTickers {
        log_tickers(self.transform.log().log_base(), None, settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_scale_domain() {
        let log = LogScale::new(10.0).unwrap();
        assert_eq!(
            log.limit_range_for_scale(0.0, 10.0, f64::INFINITY),
            (1e-300, 10.0)
        );
        assert_eq!(log.limit_range_for_scale(-5.0, -1.0, 0.5), (0.5, 0.5));
        assert!(!log.val_in_range(0.0));
        assert!(log.val_in_range(1e-300));
        let masked = LogScale::new(2.0)
            .unwrap()
            .with_nonpositive(NonPositive::Mask);
        assert!(masked.forward(&[-1.0])[0].is_nan());
        assert_eq!(masked.forward(&[8.0]), [3.0]);
        assert!(LogScale::new(-10.0).is_err());
    }

    #[test]
    fn func_log() {
        let scale = FuncScaleLog::new(FuncTransform::new(|x| x * x, f64::sqrt), 10.0).unwrap();
        assert_eq!(scale.forward(&[10.0]), [2.0]);
        assert_eq!(scale.limit_range_for_scale(-1.0, 4.0, 0.5), (0.5, 4.0));
    }
}
