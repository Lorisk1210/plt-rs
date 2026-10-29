//! `AsinhScale`: quasi-logarithmic, smooth through zero.

use std::rc::Rc;

use super::{AsinhTransform, AxisTickers, Scale, ScaleError, ScaleName};
use crate::ticker::{
    AsinhLocator, AxisName, Formatter, LogFormatter, LogLabelStyle, NullFormatter,
    StrMethodFormatter, TickerSettings,
};
use crate::transforms::AxisMap;

/// The minor tick multiples used for common bases when none are given.
fn auto_tick_multipliers(base: i64) -> Vec<f64> {
    let subs: &[f64] = match base {
        3..=5 => &[2.0],
        8 => &[2.0, 4.0],
        10 => &[2.0, 5.0],
        16 => &[2.0, 4.0, 8.0],
        64 => &[4.0, 16.0],
        1024 => &[256.0, 512.0],
        _ => &[],
    };
    subs.to_vec()
}

/// The scale `w * asinh(x / w)`: linear near zero, logarithmic for large
/// magnitudes, without symlog's kinks (matplotlib's `AsinhScale`).
#[derive(Clone, Debug, PartialEq)]
pub struct AsinhScale {
    transform: AsinhTransform,
    base: i64,
    /// The minor tick multiples; none when empty.
    subs: Vec<f64>,
}

impl AsinhScale {
    /// An asinh scale with quasi-linear width `linear_width > 0`, ticks
    /// rounded to powers of `base` (truncated to an integer; `<= 1` rounds
    /// to multiples of powers of ten) and the default minor multiples.
    pub fn new(linear_width: f64, base: f64) -> Result<Self, ScaleError> {
        let base = base.trunc() as i64;
        Ok(Self {
            transform: AsinhTransform::new(linear_width)?,
            base,
            subs: auto_tick_multipliers(base),
        })
    }

    /// Minor tick multiples in an array or a `Vec`, in place of the
    /// base's defaults; an empty list for none.
    #[must_use]
    pub fn with_subs(mut self, subs: impl Into<Vec<f64>>) -> Self {
        self.subs = subs.into();
        self
    }

    /// The quasi-linear width.
    pub fn linear_width(&self) -> f64 {
        self.transform.linear_width()
    }

    /// The rounding base.
    pub fn base(&self) -> i64 {
        self.base
    }

    fn locator(&self) -> AsinhLocator {
        AsinhLocator::new(self.linear_width()).with_base(self.base as f64)
    }
}

impl Default for AsinhScale {
    /// `linear_width = 1`, base 10.
    fn default() -> Self {
        Self::new(1.0, 10.0).expect("default parameters are valid")
    }
}

impl Scale for AsinhScale {
    fn name(&self) -> ScaleName {
        ScaleName::Asinh
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(self.transform))
    }

    fn default_tickers(&self, _: AxisName, settings: &TickerSettings) -> AxisTickers {
        let major_formatter: Box<dyn Formatter> = if self.base > 1 {
            Box::new(LogFormatter::from_settings(
                LogLabelStyle::SciNotation,
                self.base as f64,
                settings,
            ))
        } else {
            Box::new(StrMethodFormatter::new("{x:.3g}").expect("the template is valid"))
        };
        AxisTickers {
            major_locator: Box::new(self.locator()),
            minor_locator: Box::new(self.locator().with_subs(self.subs.clone())),
            major_formatter,
            minor_formatter: Box::new(NullFormatter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asinh_defaults() {
        let scale = AsinhScale::default();
        assert_eq!(scale.forward(&[0.0]), [0.0]);
        assert_eq!(scale.subs, [2.0, 5.0]);
        assert_eq!(AsinhScale::new(1.0, 7.0).unwrap().subs, Vec::<f64>::new());
        assert!(AsinhScale::new(0.0, 10.0).is_err());
    }
}
