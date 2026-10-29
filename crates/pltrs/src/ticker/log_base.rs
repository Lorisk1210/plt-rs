//! `LogBase`: the base of a logarithm, checked once.

/// The base of a logarithmic axis: finite, positive and not 1.
///
/// Log locators, transforms and scales take their base as a plain number
/// and check it; holding a `LogBase` instead, they need not check it again.
///
/// ```
/// use pltrs::ticker::LogBase;
/// assert_eq!(LogBase::new(2.0).map(LogBase::get), Some(2.0));
/// assert_eq!(LogBase::new(1.0), None);
/// assert_eq!(LogBase::TEN.get(), 10.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogBase(f64);

impl LogBase {
    /// Base 10, matplotlib's default.
    pub const TEN: Self = Self(10.0);

    /// `base` as a logarithm base, if it is finite, positive and not 1.
    pub fn new(base: f64) -> Option<Self> {
        (base.is_finite() && base > 0.0 && base != 1.0).then_some(Self(base))
    }

    /// The base as a number.
    pub fn get(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_positive_finite_bases_other_than_one() {
        for base in [0.0, -2.0, 1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(LogBase::new(base), None, "{base}");
        }
        assert_eq!(LogBase::new(0.5).map(LogBase::get), Some(0.5));
    }
}
