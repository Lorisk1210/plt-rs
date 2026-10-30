//! `LogitScale`: probabilities in `(0, 1)`.

use std::rc::Rc;

use super::{AxisTickers, LogitTransform, NonPositive, Scale, ScaleName};
use crate::ticker::{AxisName, LogitFormatter, LogitLocator, TickerSettings};
use crate::transforms::AxisMap;

/// The logit scale `log10(p / (1 - p))`, close to a log scale near 0 and 1
/// and almost linear around one half (matplotlib's `LogitScale`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogitScale {
    transform: LogitTransform,
    one_half: String,
    use_overline: bool,
}

pltrs_core::archive_struct!(LogitScale {
    transform,
    one_half,
    use_overline
});

impl Default for LogitScale {
    /// Masks values outside `(0, 1)`.
    fn default() -> Self {
        Self::new(NonPositive::Mask)
    }
}

impl LogitScale {
    /// A logit scale treating values outside `(0, 1)` per `nonpositive`.
    pub fn new(nonpositive: NonPositive) -> Self {
        Self {
            transform: LogitTransform::new(nonpositive),
            one_half: r"\frac{1}{2}".to_owned(),
            use_overline: false,
        }
    }

    /// The label text for one half.
    #[must_use]
    pub fn with_one_half(mut self, one_half: impl Into<String>) -> Self {
        self.one_half = one_half.into();
        self
    }

    /// Label probabilities above one half as `\overline{x}` instead of
    /// `1-x`.
    #[must_use]
    pub fn with_use_overline(mut self, use_overline: bool) -> Self {
        self.use_overline = use_overline;
        self
    }

    fn formatter(&self, minor: bool) -> LogitFormatter {
        LogitFormatter::new()
            .with_one_half(self.one_half.clone())
            .with_use_overline(self.use_overline)
            .with_minor(minor)
    }
}

impl Scale for LogitScale {
    fn name(&self) -> ScaleName {
        ScaleName::Logit
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(self.transform))
    }

    fn limit_range_for_scale(&self, vmin: f64, vmax: f64, minpos: f64) -> (f64, f64) {
        let minpos = if minpos.is_finite() { minpos } else { 1e-7 };
        (
            if vmin <= 0.0 { minpos } else { vmin },
            if vmax >= 1.0 { 1.0 - minpos } else { vmax },
        )
    }

    fn val_in_range(&self, value: f64) -> bool {
        0.0 < value && value < 1.0
    }

    fn default_tickers(&self, _: AxisName, _: &TickerSettings) -> AxisTickers {
        AxisTickers {
            major_locator: Box::new(LogitLocator::new(false)),
            minor_locator: Box::new(LogitLocator::new(true)),
            major_formatter: Box::new(self.formatter(false)),
            minor_formatter: Box::new(self.formatter(true)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logit_domain() {
        let scale = LogitScale::default();
        assert_eq!(
            scale.limit_range_for_scale(0.0, 1.0, 1e-3),
            (1e-3, 1.0 - 1e-3)
        );
        assert_eq!(
            scale.limit_range_for_scale(0.0, 1.0, f64::INFINITY),
            (1e-7, 1.0 - 1e-7)
        );
        assert!(!scale.val_in_range(1.0));
        assert!(scale.val_in_range(0.5));
        assert!(scale.forward(&[1.1])[0].is_nan());
    }
}
