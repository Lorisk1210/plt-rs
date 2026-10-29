//! `SymmetricalLogScale`: logarithmic on both sides of a linear region.

use std::rc::Rc;

use super::{AxisTickers, Scale, ScaleError, ScaleName, SymmetricalLogTransform};
use crate::ticker::{
    AxisName, LogFormatter, LogLabelStyle, NullFormatter, ScaleHint, SymmetricalLogLocator,
    TickerSettings,
};
use crate::transforms::AxisMap;

/// Linear within `(-linthresh, linthresh)` and logarithmic beyond, in both
/// directions (matplotlib's `SymmetricalLogScale`).
#[derive(Clone, Debug, PartialEq)]
pub struct SymmetricalLogScale {
    transform: SymmetricalLogTransform,
    subs: Option<Vec<f64>>,
}

impl SymmetricalLogScale {
    /// A symlog scale; `base > 1`, `linthresh > 0`, and `linscale > 0`
    /// decades for each half of the linear region.
    pub fn new(base: f64, linthresh: f64, linscale: f64) -> Result<Self, ScaleError> {
        Ok(Self {
            transform: SymmetricalLogTransform::new(base, linthresh, linscale)?,
            subs: None,
        })
    }

    /// Multiples of each power where minor ticks go, in an array or a
    /// `Vec` (by default the powers themselves).
    #[must_use]
    pub fn with_subs(mut self, subs: impl Into<Vec<f64>>) -> Self {
        self.subs = Some(subs.into());
        self
    }

    /// The underlying transform.
    pub fn transform(&self) -> SymmetricalLogTransform {
        self.transform
    }
}

impl Default for SymmetricalLogScale {
    /// Base 10, `linthresh = 2`, `linscale = 1`.
    fn default() -> Self {
        Self::new(10.0, 2.0, 1.0).expect("default parameters are valid")
    }
}

impl Scale for SymmetricalLogScale {
    fn name(&self) -> ScaleName {
        ScaleName::Symlog
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(self.transform))
    }

    fn hint(&self) -> ScaleHint {
        ScaleHint::SymLog {
            linthresh: self.transform.linthresh(),
        }
    }

    fn default_tickers(&self, _: AxisName, settings: &TickerSettings) -> AxisTickers {
        let t = self.transform;
        let major = SymmetricalLogLocator::new(t.base(), t.linthresh());
        let minor = match &self.subs {
            Some(subs) => major.clone().with_subs(subs.clone()),
            None => major.clone(),
        };
        AxisTickers {
            major_locator: Box::new(major),
            minor_locator: Box::new(minor),
            major_formatter: Box::new(LogFormatter::from_settings(
                LogLabelStyle::SciNotation,
                t.base(),
                settings,
            )),
            minor_formatter: Box::new(NullFormatter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symlog_defaults() {
        let scale = SymmetricalLogScale::default();
        assert_eq!(scale.forward(&[0.0]), [0.0]);
        assert_eq!(scale.hint(), ScaleHint::SymLog { linthresh: 2.0 });
        assert!(SymmetricalLogScale::new(0.5, 1.0, 1.0).is_err());
        assert!(scale.val_in_range(-1e300));
    }
}
