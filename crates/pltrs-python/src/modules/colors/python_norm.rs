//! Norms pltrs (Rust) has no copy of (a `Norm` subclass of the user's, or
//! a matplotlib class pltrs (Rust) lacks) map data through the Python norm
//! itself: pltrs (Rust) calls it as matplotlib's mappables do.
//!
//! The package registers `native_norm(norm)` (in `colors.py`), which gives
//! the callables pltrs (Rust) calls, working on plain arrays: `normalize`,
//! `autoscale_none`, `inverse` and `set_limits`, and `state`, read once. Their exceptions
//! raise from the native call that drew, as those of every callback do.

use pltrs::colors::{Norm, NormError, Normalized};

use crate::binding::lookup;
use crate::binding::prelude::*;

/// A Python norm, with the limits and clipping it had when it was read.
#[derive(Clone, Debug)]
pub(crate) struct PythonNorm {
    /// The Python norm (what an archive holds in place of this copy).
    object: Owned,
    vmin: Option<f64>,
    vmax: Option<f64>,
    clip: bool,
    hooks: Hooks,
}

/// The callables `native_norm` gives for a norm.
#[derive(Clone, Debug)]
struct Hooks {
    /// `normalize(values, mask, clip) -> (values, mask)`.
    normalize: Callback,
    /// `autoscale_none(values, mask) -> (vmin, vmax)`.
    autoscale_none: Callback,
    /// `inverse(values) -> (values, mask)`.
    inverse: Callback,
    /// `set_limits(vmin, vmax)`.
    set_limits: Callback,
}

impl PythonNorm {
    /// The norm `norm`, through the callables the package's
    /// `native_norm` gives for it.
    pub(crate) fn of(norm: &Bound<'_>) -> PyResult<Self> {
        let py = norm.py();
        let (normalize, autoscale_none, inverse, set_limits, state): (
            Bound<'_>,
            Bound<'_>,
            Bound<'_>,
            Bound<'_>,
            Bound<'_>,
        ) = lookup(py, "native_norm")?
            .call(std::slice::from_ref(norm), &[])?
            .extract()?;
        let (vmin, vmax, clip): (Option<f64>, Option<f64>, bool) =
            state.call(&[], &[])?.extract()?;
        Ok(Self {
            object: norm.clone().unbind(),
            vmin,
            vmax,
            clip,
            hooks: Hooks {
                normalize: normalize.extract()?,
                autoscale_none: autoscale_none.extract()?,
                inverse: inverse.extract()?,
                set_limits: set_limits.extract()?,
            },
        })
    }

    /// The Python norm.
    pub(crate) fn object(&self) -> &Owned {
        &self.object
    }

    /// Hands the limits to the Python norm, as matplotlib's mappables set
    /// them on the norm they share.
    fn push_limits(&self) {
        let _: Option<Owned> = self.hooks.set_limits.call((self.vmin, self.vmax));
    }
}

/// Values and their mask as arrays: masked entries hold NaN.
fn split(data: &[Option<f64>]) -> (NdArray, NdArray<bool>) {
    let values = data.iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    let mask = data.iter().map(Option::is_none).collect();
    (NdArray::from_vec(values), NdArray::from_vec(mask))
}

impl Norm for PythonNorm {
    fn vmin(&self) -> Option<f64> {
        self.vmin
    }

    fn vmax(&self) -> Option<f64> {
        self.vmax
    }

    fn set_vmin(&mut self, vmin: Option<f64>) {
        self.vmin = vmin;
        self.push_limits();
    }

    fn set_vmax(&mut self, vmax: Option<f64>) {
        self.vmax = vmax;
        self.push_limits();
    }

    fn clip(&self) -> bool {
        self.clip
    }

    fn set_clip(&mut self, clip: bool) {
        self.clip = clip;
    }

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        let (values, mask) = split(data);
        if let Some((vmin, vmax)) = self.hooks.autoscale_none.call((values, mask)) {
            self.vmin = vmin;
            self.vmax = vmax;
        }
    }

    /// The Python norm's values; masked or none at all when it raised (the
    /// exception raises from the native call drawing).
    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        let (values, mask) = split(data);
        let Some((values, mask)) = self
            .hooks
            .normalize
            .call::<(NdArray, NdArray<bool>)>((values, mask, clip))
        else {
            return Ok(vec![Normalized::Masked; data.len()]);
        };
        Ok(values
            .data
            .iter()
            .zip(mask.data.iter().chain(std::iter::repeat(&false)))
            .map(|(&value, &masked)| {
                if masked {
                    Normalized::Masked
                } else {
                    Normalized::Fraction(value)
                }
            })
            .collect())
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        let Some((values, mask)) = self
            .hooks
            .inverse
            .call::<(NdArray, NdArray<bool>)>((NdArray::from_vec(values.to_vec()),))
        else {
            return Err(NormError::NotInvertible);
        };
        Ok(values
            .data
            .iter()
            .zip(mask.data.iter().chain(std::iter::repeat(&false)))
            .map(|(&value, &masked)| (!masked).then_some(value))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, str, with_python};

    /// A norm of no class of matplotlib's, and hooks standing for the
    /// package's `native_norm` (those working on arrays are not called).
    const LIMITS: &str = "
class Half:
    n_components = 1
    vmin, vmax, clip = 0.0, 1.0, False
def hooks(norm):
    def set_limits(vmin, vmax):
        norm.vmin, norm.vmax = vmin, vmax
    def state():
        return norm.vmin, norm.vmax, norm.clip
    return print, print, print, set_limits, state
";

    #[test]
    fn norms_of_other_classes_keep_their_limits_in_python() {
        with_python(|py| {
            let hooks = eval(py, LIMITS, "hooks");
            extension(py)
                .getattr("register")
                .and_then(|register| register.call(&[str(py, "native_norm"), hooks], &[]))
                .unwrap();
            let half = eval(py, LIMITS, "Half()");
            let mut norm: Box<dyn Norm> = half.extract().unwrap();
            assert_eq!(
                (norm.vmin(), norm.vmax(), norm.clip()),
                (Some(0.0), Some(1.0), false)
            );
            norm.set_vmax(Some(4.0));
            assert_eq!(norm.clone().vmax(), Some(4.0));
            let vmax: f64 = half.getattr("vmax").unwrap().extract().unwrap();
            assert_eq!(vmax, 4.0, "the Python norm follows");
            let multi = eval(py, "class Two:\n    n_components = 2", "Two()");
            assert!(multi.extract::<Box<dyn Norm>>().is_err());
        });
    }
}
