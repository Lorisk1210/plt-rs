//! Norms by scale name and several norms side by side, one per variate of
//! multivariate data (matplotlib's `_auto_norm_from_scale` and
//! `MultiNorm`).

use std::fmt;
use std::str::FromStr;

use super::scaled::{LinearTransform, ScaleNorm};
use super::{AsinhNorm, LogNorm, Norm, NormError, NormKind, SymLogNorm};
use crate::colors::Normalized;
use crate::scale::{LogitTransform, NonPositive, ScaleName};

/// A norm made from the name of a scale (matplotlib's `norm="log"`,
/// built by `_auto_norm_from_scale` with masking of values outside the
/// scale's domain).
///
/// ```
/// use plt::colors::{NormName, Normalized::{Fraction, Masked}};
/// let mut norm = "log".parse::<NormName>()?.norm();
/// norm.set_vmin(Some(1.0));
/// norm.set_vmax(Some(100.0));
/// assert_eq!(norm.normalize(&[10.0, -1.0])?, [Fraction(0.5), Masked]);
/// # Ok::<(), plt::colors::NormError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NormName {
    /// `"linear"`: linear, with NaN and infinite values masked.
    Linear,
    /// `"log"`: a base-10 [`LogNorm`].
    Log,
    /// `"symlog"`: a [`SymLogNorm`] with the scale's defaults (base 10,
    /// `linthresh` 2, `linscale` 1).
    Symlog,
    /// `"asinh"`: an [`AsinhNorm`] with a linear width of 1.
    Asinh,
    /// `"logit"`: `log10(p / (1 - p))`, masking values outside `(0, 1)`.
    Logit,
}

impl NormName {
    const ALL: [Self; 5] = [
        Self::Linear,
        Self::Log,
        Self::Symlog,
        Self::Asinh,
        Self::Logit,
    ];

    /// The scale name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Log => "log",
            Self::Symlog => "symlog",
            Self::Asinh => "asinh",
            Self::Logit => "logit",
        }
    }

    /// A new, autoscaling norm of this kind.
    pub fn norm(self) -> Box<dyn Norm> {
        match self {
            Self::Linear => Box::new(ScaleNorm::from_transform(LinearTransform)),
            Self::Log => Box::new(LogNorm::default()),
            Self::Symlog => Box::new(
                SymLogNorm::new(2.0, 1.0, 10.0).expect("the symlog scale defaults are valid"),
            ),
            Self::Asinh => Box::new(AsinhNorm::new(1.0).expect("the asinh scale default is valid")),
            Self::Logit => Box::new(ScaleNorm::from_transform(LogitTransform::new(
                NonPositive::Mask,
            ))),
        }
    }
}

impl fmt::Display for NormName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for NormName {
    type Err = NormError;

    /// The norm of a scale name; the function scales need functions and
    /// cannot make a norm from their name alone.
    fn from_str(name: &str) -> Result<Self, NormError> {
        if let Some(found) = Self::ALL.into_iter().find(|n| n.name() == name) {
            return Ok(found);
        }
        Err(match name {
            "function" | "functionlog" => NormError::NeedsFunctions(name.to_owned()),
            other => NormError::UnknownName(other.to_owned()),
        })
    }
}

/// A norm given as a scale name or as a norm (matplotlib's `norm`
/// argument, a `str` or a `Normalize`): the norm of a colour mapping
/// ([`ColorizerOptions`](crate::colors::ColorizerOptions)), or one norm
/// of a [`MultiNorm`].
///
/// ```
/// use plt::colors::{LogNorm, NormName, NormSpec};
/// let named: NormSpec = NormName::Log.into();
/// let given: NormSpec = LogNorm::new(1.0, 10.0).into();
/// assert!(named.resolve().vmin().is_none(), "a new norm, not yet scaled");
/// assert_eq!(given.resolve().vmin(), Some(1.0));
/// ```
#[derive(Clone, Debug)]
pub enum NormSpec {
    /// A new norm of the named kind.
    Name(NormName),
    /// This norm.
    Norm(Box<dyn Norm>),
}

impl NormSpec {
    /// The norm `norm`.
    pub fn norm(norm: impl Norm + 'static) -> Self {
        Self::Norm(Box::new(norm))
    }

    /// Whether the norm is logarithmic: the `"log"` scale or a norm made
    /// from it (matplotlib's `isinstance(norm, LogNorm)`).
    pub fn is_log(&self) -> bool {
        match self {
            Self::Name(name) => *name == NormName::Log,
            Self::Norm(norm) => {
                matches!(norm.kind(), NormKind::Scaled(scale) if scale.name() == ScaleName::Log)
            }
        }
    }

    /// The norm described.
    pub fn resolve(self) -> Box<dyn Norm> {
        match self {
            Self::Name(name) => name.norm(),
            Self::Norm(norm) => norm,
        }
    }
}

impl From<NormName> for NormSpec {
    fn from(name: NormName) -> Self {
        Self::Name(name)
    }
}

impl From<Box<dyn Norm>> for NormSpec {
    fn from(norm: Box<dyn Norm>) -> Self {
        Self::Norm(norm)
    }
}

impl<N: Norm + 'static> From<N> for NormSpec {
    fn from(norm: N) -> Self {
        Self::norm(norm)
    }
}

/// One norm per variate of multivariate data (matplotlib's `MultiNorm`):
/// each variate is normalised, inverted and autoscaled by its own norm.
///
/// Inputs hold one sequence per variate (`None` = masked); a wrong number
/// of sequences, limits or clip flags is an error.
///
/// ```
/// use plt::colors::{MultiNorm, NormName, Normalized::Fraction};
/// let mut norm = MultiNorm::new([NormName::Linear.into(), NormName::Log.into()])?;
/// norm.set_vmin(&[Some(0.0), Some(1.0)])?;
/// norm.set_vmax(&[Some(2.0), Some(100.0)])?;
/// let normed = norm.normalize(&[vec![Some(1.0)], vec![Some(10.0)]], None)?;
/// assert_eq!(normed, [vec![Fraction(0.5)], vec![Fraction(0.5)]]);
/// # Ok::<(), plt::colors::NormError>(())
/// ```
#[derive(Clone, Debug)]
pub struct MultiNorm {
    norms: Vec<Box<dyn Norm>>,
}

impl MultiNorm {
    /// The norms described, at least one.
    ///
    /// # Errors
    ///
    /// [`NormError::NoNorms`] when none is given.
    pub fn new(norms: impl IntoIterator<Item = NormSpec>) -> Result<Self, NormError> {
        let norms: Vec<_> = norms.into_iter().map(NormSpec::resolve).collect();
        if norms.is_empty() {
            return Err(NormError::NoNorms);
        }
        Ok(Self { norms })
    }

    /// `n_components` autoscaling linear norms (matplotlib's default
    /// `MultiNorm(["linear"] * n)`).
    ///
    /// # Errors
    ///
    /// [`NormError::NoNorms`] for zero components.
    pub fn linear(n_components: usize) -> Result<Self, NormError> {
        Self::new(std::iter::repeat_n(
            NormSpec::Name(NormName::Linear),
            n_components,
        ))
    }

    /// The number of norms.
    pub fn n_components(&self) -> usize {
        self.norms.len()
    }

    /// The norms, one per variate.
    pub fn norms(&self) -> &[Box<dyn Norm>] {
        &self.norms
    }

    /// The norms, mutably.
    pub fn norms_mut(&mut self) -> &mut [Box<dyn Norm>] {
        &mut self.norms
    }

    /// The lower limit of each norm.
    pub fn vmin(&self) -> Vec<Option<f64>> {
        self.norms.iter().map(|norm| norm.vmin()).collect()
    }

    /// The upper limit of each norm.
    pub fn vmax(&self) -> Vec<Option<f64>> {
        self.norms.iter().map(|norm| norm.vmax()).collect()
    }

    /// The clipping flag of each norm.
    pub fn clip(&self) -> Vec<bool> {
        self.norms.iter().map(|norm| norm.clip()).collect()
    }

    /// Sets (or, with `None` entries, clears) the lower limit of each norm.
    ///
    /// # Errors
    ///
    /// When not given one limit per norm.
    pub fn set_vmin(&mut self, vmin: &[Option<f64>]) -> Result<(), NormError> {
        self.check("vmin", vmin.len())?;
        for (norm, &v) in self.norms.iter_mut().zip(vmin) {
            norm.set_vmin(v);
        }
        Ok(())
    }

    /// Sets (or, with `None` entries, clears) the upper limit of each norm.
    ///
    /// # Errors
    ///
    /// When not given one limit per norm.
    pub fn set_vmax(&mut self, vmax: &[Option<f64>]) -> Result<(), NormError> {
        self.check("vmax", vmax.len())?;
        for (norm, &v) in self.norms.iter_mut().zip(vmax) {
            norm.set_vmax(v);
        }
        Ok(())
    }

    /// Sets the clipping flag of each norm.
    ///
    /// # Errors
    ///
    /// When not given one flag per norm.
    pub fn set_clip(&mut self, clip: &[bool]) -> Result<(), NormError> {
        self.check("clip", clip.len())?;
        for (norm, &c) in self.norms.iter_mut().zip(clip) {
            norm.set_clip(c);
        }
        Ok(())
    }

    /// Whether every norm has both limits.
    pub fn scaled(&self) -> bool {
        self.norms.iter().all(|norm| norm.scaled())
    }

    /// Sets the limits of each norm from its variate of `data`.
    ///
    /// # Errors
    ///
    /// When `data` does not hold one sequence per norm.
    pub fn autoscale(&mut self, data: &[impl AsRef<[Option<f64>]>]) -> Result<(), NormError> {
        self.check("data", data.len())?;
        for (norm, values) in self.norms.iter_mut().zip(data) {
            norm.autoscale(values.as_ref());
        }
        Ok(())
    }

    /// Fills the unset limits of each norm from its variate of `data`.
    ///
    /// # Errors
    ///
    /// When `data` does not hold one sequence per norm.
    pub fn autoscale_none(&mut self, data: &[impl AsRef<[Option<f64>]>]) -> Result<(), NormError> {
        self.check("data", data.len())?;
        for (norm, values) in self.norms.iter_mut().zip(data) {
            norm.autoscale_none(values.as_ref());
        }
        Ok(())
    }

    /// Normalises each variate of `data` with its norm; `clip` overrides
    /// the norms' own flags.
    ///
    /// # Errors
    ///
    /// When `data` or `clip` does not hold one entry per norm, or a norm
    /// cannot map its variate.
    pub fn normalize(
        &mut self,
        data: &[impl AsRef<[Option<f64>]>],
        clip: Option<&[bool]>,
    ) -> Result<Vec<Vec<Normalized>>, NormError> {
        let clip = match clip {
            Some(clip) => clip.to_vec(),
            None => self.clip(),
        };
        self.check("clip", clip.len())?;
        self.check("data", data.len())?;
        self.norms
            .iter_mut()
            .zip(data)
            .zip(clip)
            .map(|((norm, values), clip)| norm.normalize_masked(values.as_ref(), Some(clip)))
            .collect()
    }

    /// Maps each variate of normalised `values` back to data values with
    /// its norm.
    ///
    /// # Errors
    ///
    /// When `values` does not hold one sequence per norm, or a norm cannot
    /// be inverted.
    pub fn inverse(
        &self,
        values: &[impl AsRef<[f64]>],
    ) -> Result<Vec<Vec<Option<f64>>>, NormError> {
        self.check("data", values.len())?;
        self.norms
            .iter()
            .zip(values)
            .map(|(norm, values)| norm.inverse(values.as_ref()))
            .collect()
    }

    /// Fails unless `given` is the number of norms.
    fn check(&self, what: &'static str, given: usize) -> Result<(), NormError> {
        if given == self.norms.len() {
            Ok(())
        } else {
            Err(NormError::ComponentCount {
                what,
                expected: self.norms.len(),
                given,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalize;
    use crate::colors::Normalized::{Fraction, Masked};

    #[test]
    fn names_make_norms() {
        assert_eq!("symlog".parse(), Ok(NormName::Symlog));
        assert_eq!(NormName::Logit.to_string(), "logit");
        assert_eq!(
            "function".parse::<NormName>(),
            Err(NormError::NeedsFunctions("function".into()))
        );
        assert_eq!(
            "nope".parse::<NormName>(),
            Err(NormError::UnknownName("nope".into()))
        );
        let mut linear = NormName::Linear.norm();
        assert_eq!(
            linear.normalize(&[0.0, f64::NAN, 2.0]),
            Ok(vec![Fraction(0.0), Masked, Fraction(1.0)]),
            "autoscaled to the finite values, NaN masked"
        );
        let mut logit = NormName::Logit.norm();
        logit.set_vmin(Some(0.1));
        logit.set_vmax(Some(0.9));
        let normed = logit.normalize(&[0.5, 1.5]).unwrap();
        assert!((normed[0].fraction().unwrap() - 0.5).abs() < 1e-12);
        assert_eq!(normed[1], Masked, "outside (0, 1)");
    }

    #[test]
    fn components_are_checked() {
        assert_eq!(MultiNorm::new([]).err(), Some(NormError::NoNorms));
        let mut norm = MultiNorm::linear(2).unwrap();
        assert_eq!(
            norm.set_vmin(&[Some(1.0)]),
            Err(NormError::ComponentCount {
                what: "vmin",
                expected: 2,
                given: 1
            })
        );
        assert!(norm.normalize(&[vec![Some(1.0)]], None).is_err());
        assert!(norm.set_clip(&[true, false, true]).is_err());
        assert!(norm.inverse(&[[0.5]]).is_err());
    }

    #[test]
    fn each_variate_has_its_norm() {
        let mut norm = MultiNorm::new([
            NormSpec::norm(Normalize::new(0.0, 10.0)),
            NormName::Log.into(),
        ])
        .unwrap();
        assert!(!norm.scaled());
        norm.autoscale_none(&[vec![Some(3.0)], vec![Some(1.0), None, Some(100.0)]])
            .unwrap();
        assert_eq!(norm.vmin(), [Some(0.0), Some(1.0)]);
        assert_eq!(norm.vmax(), [Some(10.0), Some(100.0)]);
        norm.set_clip(&[true, false]).unwrap();
        let normed = norm
            .normalize(&[vec![Some(20.0)], vec![Some(1000.0)]], None)
            .unwrap();
        assert_eq!(normed, [vec![Fraction(1.0)], vec![Fraction(1.5)]]);
        let unclipped = norm
            .normalize(&[[Some(20.0)], [Some(10.0)]], Some(&[false, false]))
            .unwrap();
        assert_eq!(unclipped, [vec![Fraction(2.0)], vec![Fraction(0.5)]]);
        let inverse = norm.inverse(&[[0.5], [0.5]]).unwrap();
        assert_eq!(inverse[0], [Some(5.0)]);
        assert!((inverse[1][0].unwrap() - 10.0).abs() < 1e-12);
        norm.autoscale(&[[Some(-1.0), Some(1.0)], [Some(2.0), Some(4.0)]])
            .unwrap();
        assert_eq!(norm.vmin(), [Some(-1.0), Some(2.0)]);
    }
}
