//! Norms pltrs (Rust) has no copy of (a `Norm` subclass of the user's, or
//! a matplotlib class pltrs (Rust) lacks) map data through the Python norm
//! itself: pltrs (Rust) calls it as matplotlib's mappables do.
//!
//! The package registers `native_norm(norm)` (in `colors.py`), which gives
//! the callables pltrs (Rust) calls, working on plain arrays: `normalize`,
//! `normalize_scalar`, `autoscale_none`, `inverse` and `set_limits`, and
//! `state`, `scale` and `boundary`, read once. Their exceptions raise from
//! the native call that drew, as those of every callback do.

use pltrs::colors::{ArrayForm, BoundaryNorm, Norm, NormError, NormKind, Normalized};
use pltrs::scale::{ScaleOptions, scale_factory};

use super::super::scale::scale_options;
use super::mapping::extend;
use crate::binding::prelude::*;
use crate::binding::{IntoPy, dict_value, lookup};

/// A Python norm, with the limits and clipping it had when it was read.
#[derive(Clone, Debug)]
pub(crate) struct PythonNorm {
    /// The Python norm, as `native_norm` gives it: held weakly (a
    /// `Held`, which an archive holds as the norm) for a norm the
    /// program's colorizer holds, strongly for one read back from an
    /// archive.
    object: Owned,
    vmin: Option<f64>,
    vmax: Option<f64>,
    clip: bool,
    /// The scale the norm is made from (matplotlib's `norm._scale`), by
    /// name and options, when pltrs (Rust) has one of its kind.
    scale: Option<(String, ScaleOptions)>,
    /// The bins of a norm deriving from matplotlib's `BoundaryNorm`, which
    /// colorbars show as matplotlib's `isinstance` checks do (its
    /// boundaries, never its inverse); `None` for any other norm.
    boundary: Option<BoundaryNorm>,
    /// Whether the norm derives from matplotlib's `NoNorm`, which
    /// colorbars show by its colour indices.
    identity: bool,
    hooks: Hooks,
}

/// The callables `native_norm` gives for a norm.
#[derive(Clone, Debug)]
struct Hooks {
    /// `normalize(values, mask, clip, shape, single) -> (values, mask,
    /// integers)`, the values handed to the norm as an array of `shape`
    /// (`None`: as they are), of `float32` if `single`.
    normalize: Callback,
    /// `normalize_scalar(value) -> (values, mask, integers)`.
    normalize_scalar: Callback,
    /// `autoscale_none(values, mask, shape) -> (vmin, vmax)`, the values
    /// handed to the norm as a masked array of `shape` (`None`: flat), a
    /// plain one for no `mask`.
    autoscale_none: Callback,
    /// `inverse(values) -> (values, mask)`.
    inverse: Callback,
    /// `set_limits(vmin, vmax)`.
    set_limits: Callback,
}

impl PythonNorm {
    /// The norm `norm`, through the callables the package's
    /// `native_norm` gives for it, which hold it `weak`ly unless it is
    /// read back from an archive (nothing else holds it then).
    pub(crate) fn of(norm: &Bound<'_>, weak: bool) -> PyResult<Self> {
        let py = norm.py();
        let hooks = lookup(py, "native_norm")?.call(&[norm.clone(), weak.into_py(py)?], &[])?;
        let hook = |name: &str| dict_value::<Bound<'_>>(&hooks, name);
        let (vmin, vmax, clip): (Option<f64>, Option<f64>, bool) =
            hook("state")?.call(&[], &[])?.extract()?;
        let scale = match hook("scale")?.extract::<Option<(String, Bound<'_>)>>()? {
            Some((name, kwargs)) => Some((name, scale_options(&kwargs)?)),
            None => None,
        };
        // Bins pltrs (Rust) refuses (more than colours) show as any norm's.
        let boundary = match hook("boundary")?.extract::<Option<(Vec<f64>, usize, String)>>()? {
            Some((boundaries, ncolors, name)) => {
                BoundaryNorm::new(boundaries, ncolors, extend(&name)?).ok()
            }
            None => None,
        };
        Ok(Self {
            object: hook("object")?.unbind(),
            vmin,
            vmax,
            clip,
            scale,
            boundary,
            identity: hook("identity")?.extract()?,
            hooks: Hooks {
                normalize: hook("normalize")?.extract()?,
                normalize_scalar: hook("normalize_scalar")?.extract()?,
                autoscale_none: hook("autoscale_none")?.extract()?,
                inverse: hook("inverse")?.extract()?,
                set_limits: hook("set_limits")?.extract()?,
            },
        })
    }

    /// The Python norm.
    pub(crate) fn object(&self) -> &Owned {
        &self.object
    }

    /// Autoscales the Python norm to `data`, handed over as a masked array
    /// of `shape` (flat for `None`), and takes the limits it found.
    fn autoscale_none_as(&mut self, data: &[Option<f64>], shape: Option<Vec<usize>>) {
        let (values, mask) = split(data);
        self.autoscale_with((values, Some(mask), shape));
    }

    /// Autoscales the Python norm with the arguments of its
    /// `autoscale_none` hook, and takes the limits it found.
    fn autoscale_with(&mut self, arguments: (NdArray, Option<NdArray<bool>>, Option<Vec<usize>>)) {
        if let Some((vmin, vmax)) = self.hooks.autoscale_none.call(arguments) {
            self.vmin = vmin;
            self.vmax = vmax;
        }
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

/// [`split`], the mask left out (`None`: a plain array) unless `masked`.
fn split_as(data: &[Option<f64>], masked: bool) -> (NdArray, Option<NdArray<bool>>) {
    let (values, mask) = split(data);
    (values, masked.then_some(mask))
}

/// What a `normalize` hook gave: the normalised values, masked or none
/// at all when it raised (the exception raises from the native call
/// drawing), `len` of them; integers (a `BoundaryNorm`'s) index the
/// colours, as matplotlib's colormaps take them.
fn normalized(given: Option<(NdArray, NdArray<bool>, bool)>, len: usize) -> Vec<Normalized> {
    let Some((values, mask, integers)) = given else {
        return vec![Normalized::Masked; len];
    };
    values
        .data
        .iter()
        .zip(mask.data.iter().chain(std::iter::repeat(&false)))
        .map(|(&value, &masked)| {
            if masked {
                Normalized::Masked
            } else if integers {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the norm gave integers, carried as floats"
                )]
                let index = value as i64;
                Normalized::Index(index)
            } else {
                Normalized::Fraction(value)
            }
        })
        .collect()
}

impl Norm for PythonNorm {
    fn calls_program(&self) -> bool {
        true
    }

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
        self.autoscale_none_as(data, None);
    }

    fn autoscale_none_shaped(&mut self, data: &[Option<f64>], shape: &[usize]) {
        self.autoscale_none_as(data, Some(shape.to_vec()));
    }

    fn autoscale_none_plain(&mut self, values: &[f64]) {
        self.autoscale_with((NdArray::from_vec(values.to_vec()), None, None));
    }

    /// A norm deriving from `BoundaryNorm` is shown by its bins, one
    /// deriving from `NoNorm` by its colour indices; one made
    /// from a scale of pltrs (Rust)'s kind on that scale (matplotlib's
    /// colorbar takes `norm._scale`).
    fn kind(&self) -> NormKind<'_> {
        if let Some(boundary) = &self.boundary {
            return NormKind::Boundary(boundary);
        }
        if self.identity {
            return NormKind::Identity;
        }
        let scale = self.scale.as_ref();
        scale
            .and_then(|(name, options)| scale_factory(name.as_str(), options.clone()).ok())
            .map_or(NormKind::Other, NormKind::Scaled)
    }

    /// The Python norm's values; masked or none at all when it raised (the
    /// exception raises from the native call drawing).
    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        let (values, mask) = split(data);
        let given =
            self.hooks
                .normalize
                .call((values, Some(mask), clip, None::<Vec<usize>>, false));
        Ok(normalized(given, data.len()))
    }

    /// The Python norm's values for the array `form` describes.
    fn normalize_shaped(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
        form: ArrayForm<'_>,
    ) -> Result<Vec<Normalized>, NormError> {
        let (values, mask) = split_as(data, form.masked);
        let given =
            self.hooks
                .normalize
                .call((values, mask, clip, form.shape.to_vec(), form.single));
        Ok(normalized(given, data.len()))
    }

    /// The Python norm's value for `value` handed over alone.
    fn normalize_scalar(&mut self, value: f64) -> Result<Normalized, NormError> {
        let given = self.hooks.normalize_scalar.call((value,));
        let normalized = normalized(given, 1);
        Ok(normalized.into_iter().next().unwrap_or(Normalized::Masked))
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
def hooks(norm, weak):
    def set_limits(vmin, vmax):
        norm.vmin, norm.vmax = vmin, vmax
    def state():
        return norm.vmin, norm.vmax, norm.clip
    return {'normalize': print, 'normalize_scalar': print, 'autoscale_none': print,
            'inverse': print, 'set_limits': set_limits, 'state': state, 'scale': None,
            'boundary': getattr(norm, 'bins', None), 'identity': False, 'object': norm}
class Bins(Half):
    bins = ([0.0, 0.5, 2.0], 2, 'neither')
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
            assert!(matches!(norm.kind(), NormKind::Other));
            let bins: Box<dyn Norm> = eval(py, LIMITS, "Bins()").extract().unwrap();
            let NormKind::Boundary(boundary) = bins.kind() else {
                panic!("a norm of bins shows as one");
            };
            assert_eq!(
                (boundary.boundaries(), boundary.ncolors()),
                (&[0.0, 0.5, 2.0][..], 2)
            );
        });
    }
}
