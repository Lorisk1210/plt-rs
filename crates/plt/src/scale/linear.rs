//! Scales with linear-style ticks: `LinearScale` and `FuncScale`.

use std::rc::Rc;

use super::{AxisTickers, FuncTransform, Scale, ScaleName, linear_tickers};
use crate::ticker::{AxisName, ScaleHint, TickerSettings};
use crate::transforms::AxisMap;

/// The identity scale (matplotlib's `LinearScale`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinearScale;

impl Scale for LinearScale {
    fn name(&self) -> ScaleName {
        ScaleName::Linear
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        None
    }

    fn hint(&self) -> ScaleHint {
        ScaleHint::Linear
    }

    fn default_tickers(&self, axis: AxisName, settings: &TickerSettings) -> AxisTickers {
        linear_tickers(axis, settings)
    }
}

/// A scale given by a user forward/inverse function pair, ticked like a
/// linear axis (matplotlib's `FuncScale`).
///
/// ```
/// use plt::scale::{FuncScale, Scale};
/// let square = FuncScale::new(|x| x * x, f64::sqrt);
/// assert_eq!(square.forward(&[3.0]), [9.0]);
/// assert_eq!(square.inverse(&[9.0]), [3.0]);
/// ```
#[derive(Clone, Debug)]
pub struct FuncScale {
    transform: FuncTransform,
}

impl FuncScale {
    /// `forward` must be monotonic and `inverse` its inverse.
    pub fn new(
        forward: impl Fn(f64) -> f64 + 'static,
        inverse: impl Fn(f64) -> f64 + 'static,
    ) -> Self {
        Self::from_transform(FuncTransform::new(forward, inverse))
    }

    /// A scale using an existing function pair.
    pub fn from_transform(transform: FuncTransform) -> Self {
        Self { transform }
    }
}

impl Scale for FuncScale {
    fn name(&self) -> ScaleName {
        ScaleName::Function
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(self.transform.clone()))
    }

    fn default_tickers(&self, axis: AxisName, settings: &TickerSettings) -> AxisTickers {
        linear_tickers(axis, settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_is_identity_on_finite_values() {
        let values = [-1e300, 0.0, f64::INFINITY];
        assert_eq!(LinearScale.forward(&values), values);
        assert_eq!(
            LinearScale.limit_range_for_scale(-5.0, 5.0, 1.0),
            (-5.0, 5.0)
        );
        assert!(LinearScale.val_in_range(1.0));
        assert!(!LinearScale.val_in_range(f64::NAN));
        assert!(!LinearScale.val_in_range(f64::INFINITY));
    }

    #[test]
    fn minor_ticks_follow_visibility_settings() {
        let settings = TickerSettings {
            ytick_minor_visible: true,
            ..TickerSettings::DEFAULT
        };
        let x = LinearScale.default_tickers(AxisName::X, &settings);
        let y = LinearScale.default_tickers(AxisName::Y, &settings);
        assert!(format!("{:?}", x.minor_locator).contains("NullLocator"));
        assert!(format!("{:?}", y.minor_locator).contains("AutoMinorLocator"));
    }
}
