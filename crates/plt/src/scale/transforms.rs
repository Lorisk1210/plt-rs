//! The one-dimensional maps behind the non-linear scales.
//!
//! Each map implements [`AxisMap`] so axes can plug it straight into their
//! data transform. The formulas follow matplotlib's transform classes
//! operation by operation, so results agree to the last bit wherever the
//! underlying `libm` functions do.

use std::fmt;
use std::rc::Rc;

use plt_core::numeric::np_sign;

use super::{NonPositive, ScaleError};
use crate::transforms::AxisMap;

/// The value non-positive inputs map to when clipping (`log10` of the
/// smallest double is about -323, so -1000 is safely beyond).
const CLIPPED: f64 = 1000.0;

/// `log_base(x)` (matplotlib's `LogTransform`), with non-positive inputs
/// clipped to -1000 or mapped to NaN/-inf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogTransform {
    base: f64,
    nonpositive: NonPositive,
}

impl LogTransform {
    /// A log map; `base` must be positive and not 1.
    pub fn new(base: f64, nonpositive: NonPositive) -> Result<Self, ScaleError> {
        if base > 0.0 && base != 1.0 && base.is_finite() {
            Ok(Self { base, nonpositive })
        } else {
            Err(ScaleError::invalid(
                "base",
                "the log base cannot be <= 0 or == 1",
            ))
        }
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The treatment of non-positive values.
    pub fn nonpositive(&self) -> NonPositive {
        self.nonpositive
    }
}

impl AxisMap for LogTransform {
    fn forward(&self, value: f64) -> f64 {
        let out = if self.base == std::f64::consts::E {
            value.ln()
        } else if self.base == 2.0 {
            value.log2()
        } else if self.base == 10.0 {
            value.log10()
        } else {
            value.ln() / self.base.ln()
        };
        if self.nonpositive == NonPositive::Clip && value <= 0.0 {
            -CLIPPED
        } else {
            out
        }
    }

    /// Non-positive values are masked whatever `nonpositive` says.
    fn forward_masked(&self, value: f64) -> f64 {
        if value > 0.0 {
            self.forward(value)
        } else {
            f64::NAN
        }
    }

    fn inverse(&self, value: f64) -> f64 {
        if self.base == std::f64::consts::E {
            value.exp()
        } else if self.base == 2.0 {
            value.exp2()
        } else {
            (value * self.base.ln()).exp()
        }
    }
}

/// Linear within `±linthresh`, logarithmic beyond (matplotlib's
/// `SymmetricalLogTransform`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymmetricalLogTransform {
    base: f64,
    linthresh: f64,
    linscale: f64,
}

impl SymmetricalLogTransform {
    /// A symlog map; `base > 1`, `linthresh > 0` and `linscale > 0`.
    pub fn new(base: f64, linthresh: f64, linscale: f64) -> Result<Self, ScaleError> {
        if base.is_nan() || base <= 1.0 {
            return Err(ScaleError::invalid("base", "must be larger than 1"));
        }
        if linthresh.is_nan() || linthresh <= 0.0 {
            return Err(ScaleError::invalid("linthresh", "must be positive"));
        }
        if linscale.is_nan() || linscale <= 0.0 {
            return Err(ScaleError::invalid("linscale", "must be positive"));
        }
        Ok(Self {
            base,
            linthresh,
            linscale,
        })
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The extent of the linear region around zero.
    pub fn linthresh(&self) -> f64 {
        self.linthresh
    }

    /// The number of decades each half of the linear region spans.
    pub fn linscale(&self) -> f64 {
        self.linscale
    }

    fn linscale_adj(&self) -> f64 {
        self.linscale / (1.0 - 1.0 / self.base)
    }
}

impl AxisMap for SymmetricalLogTransform {
    fn forward(&self, value: f64) -> f64 {
        let adj = self.linscale_adj();
        if value.abs() <= self.linthresh {
            return value * adj;
        }
        let log_base = self.base.ln();
        np_sign(value)
            * self.linthresh
            * (adj - self.linthresh.ln() / log_base + value.abs().ln() / log_base)
    }

    fn inverse(&self, value: f64) -> f64 {
        let adj = self.linscale_adj();
        if value.abs() <= self.forward(self.linthresh) {
            return value / adj;
        }
        np_sign(value)
            * self.linthresh
            * ((value.abs() / self.linthresh - adj) * self.base.ln()).exp()
    }
}

/// `log10(p / (1 - p))` (matplotlib's `LogitTransform`); clipping maps
/// `p <= 0` to -1000 and `p >= 1` to 1000.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogitTransform {
    nonpositive: NonPositive,
}

impl LogitTransform {
    /// A logit map.
    pub fn new(nonpositive: NonPositive) -> Self {
        Self { nonpositive }
    }

    /// The treatment of values outside `(0, 1)`.
    pub fn nonpositive(&self) -> NonPositive {
        self.nonpositive
    }
}

impl AxisMap for LogitTransform {
    fn forward(&self, value: f64) -> f64 {
        let out = (value / (1.0 - value)).log10();
        match self.nonpositive {
            NonPositive::Clip if value <= 0.0 => -CLIPPED,
            NonPositive::Clip if value >= 1.0 => CLIPPED,
            _ => out,
        }
    }

    /// Values outside `(0, 1)` are masked whatever `nonpositive` says.
    fn forward_masked(&self, value: f64) -> f64 {
        if 0.0 < value && value < 1.0 {
            self.forward(value)
        } else {
            f64::NAN
        }
    }

    fn inverse(&self, value: f64) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf(-value))
    }
}

/// `w * asinh(x / w)` (matplotlib's `AsinhTransform`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsinhTransform {
    linear_width: f64,
}

impl AsinhTransform {
    /// An asinh map; `linear_width` must be positive.
    pub fn new(linear_width: f64) -> Result<Self, ScaleError> {
        if linear_width > 0.0 {
            Ok(Self { linear_width })
        } else {
            Err(ScaleError::invalid(
                "linear_width",
                "must be strictly positive",
            ))
        }
    }

    /// The width of the quasi-linear region.
    pub fn linear_width(&self) -> f64 {
        self.linear_width
    }
}

impl AxisMap for AsinhTransform {
    fn forward(&self, value: f64) -> f64 {
        self.linear_width * (value / self.linear_width).asinh()
    }

    fn inverse(&self, value: f64) -> f64 {
        self.linear_width * (value / self.linear_width).sinh()
    }
}

/// A scalar function used by function scales.
pub type ScaleFn = Rc<dyn Fn(f64) -> f64>;

/// A user-supplied forward/inverse pair (matplotlib's `FuncTransform`).
#[derive(Clone)]
pub struct FuncTransform {
    forward: ScaleFn,
    inverse: ScaleFn,
}

impl FuncTransform {
    /// `forward` must be monotonic and `inverse` its inverse.
    pub fn new(
        forward: impl Fn(f64) -> f64 + 'static,
        inverse: impl Fn(f64) -> f64 + 'static,
    ) -> Self {
        Self {
            forward: Rc::new(forward),
            inverse: Rc::new(inverse),
        }
    }

    /// The identity.
    pub fn identity() -> Self {
        Self::new(|x| x, |x| x)
    }

    /// The same pair with forward and inverse swapped.
    #[must_use]
    pub fn reversed(&self) -> Self {
        Self {
            forward: Rc::clone(&self.inverse),
            inverse: Rc::clone(&self.forward),
        }
    }
}

impl fmt::Debug for FuncTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FuncTransform").finish_non_exhaustive()
    }
}

impl AxisMap for FuncTransform {
    fn forward(&self, value: f64) -> f64 {
        (self.forward)(value)
    }

    fn inverse(&self, value: f64) -> f64 {
        (self.inverse)(value)
    }
}

/// A function followed by a logarithm (`FuncTransform + LogTransform`).
#[derive(Clone, Debug)]
pub struct FuncLogTransform {
    func: FuncTransform,
    log: LogTransform,
}

impl FuncLogTransform {
    /// `log_base(func(x))`.
    pub fn new(func: FuncTransform, log: LogTransform) -> Self {
        Self { func, log }
    }

    /// The log stage.
    pub fn log(&self) -> &LogTransform {
        &self.log
    }
}

impl AxisMap for FuncLogTransform {
    fn forward(&self, value: f64) -> f64 {
        self.log.forward(self.func.forward(value))
    }

    fn forward_masked(&self, value: f64) -> f64 {
        self.log.forward_masked(self.func.forward(value))
    }

    fn inverse(&self, value: f64) -> f64 {
        self.func.inverse(self.log.inverse(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_clip_and_mask() {
        let clip = LogTransform::new(10.0, NonPositive::Clip).unwrap();
        assert_eq!(clip.forward(0.001), -3.0);
        assert_eq!(clip.forward(0.0), -1000.0);
        assert_eq!(clip.forward(-1.0), -1000.0);
        assert!(clip.forward(f64::NAN).is_nan());
        let mask = LogTransform::new(10.0, NonPositive::Mask).unwrap();
        assert_eq!(mask.forward(0.0), f64::NEG_INFINITY);
        assert!(mask.forward(-1.0).is_nan());
        assert!((clip.inverse(2.0) - 100.0).abs() < 1e-12);
        assert!(LogTransform::new(1.0, NonPositive::Clip).is_err());
    }

    #[test]
    fn masked_arrays_lose_the_values_outside_the_domain() {
        let log = LogTransform::new(10.0, NonPositive::Clip).unwrap();
        assert!(log.forward_masked(0.0).is_nan());
        assert!(log.forward_masked(-1.0).is_nan());
        assert_eq!(log.forward_masked(100.0), 2.0);
        let logit = LogitTransform::new(NonPositive::Clip);
        assert!(logit.forward_masked(0.0).is_nan());
        assert!(logit.forward_masked(1.0).is_nan());
        assert_eq!(logit.forward_masked(0.5), 0.0);
        let symlog = SymmetricalLogTransform::new(10.0, 1.0, 1.0).unwrap();
        assert_eq!(symlog.forward_masked(0.0), 0.0, "symlog masks nothing");
    }

    #[test]
    fn symlog_round_trip() {
        let t = SymmetricalLogTransform::new(10.0, 1.0, 1.0).unwrap();
        for v in [-1000.0, -2.0, -0.5, 0.0, 0.5, 2.0, 1000.0] {
            assert!((t.inverse(t.forward(v)) - v).abs() <= 1e-9 * v.abs().max(1.0));
        }
        assert_eq!(t.forward(0.5), 0.5 / 0.9);
        assert!(SymmetricalLogTransform::new(1.0, 1.0, 1.0).is_err());
        assert!(SymmetricalLogTransform::new(10.0, 0.0, 1.0).is_err());
    }

    #[test]
    fn logit_and_asinh() {
        let t = LogitTransform::new(NonPositive::Clip);
        assert_eq!(t.forward(0.5), 0.0);
        assert_eq!(t.forward(1.0), 1000.0);
        assert_eq!(t.inverse(0.0), 0.5);
        let a = AsinhTransform::new(1.0).unwrap();
        assert_eq!(a.forward(0.0), 0.0);
        assert!(AsinhTransform::new(0.0).is_err());
    }

    #[test]
    fn function_maps() {
        let f = FuncTransform::new(|x| 2.0 * x + 1.0, |y| (y - 1.0) / 2.0);
        assert_eq!(f.forward(1.0), 3.0);
        let log = FuncLogTransform::new(f, LogTransform::new(10.0, NonPositive::Clip).unwrap());
        assert_eq!(log.forward(4.5), 1.0);
        assert!((log.inverse(1.0) - 4.5).abs() < 1e-12);
    }
}
