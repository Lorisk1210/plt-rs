//! The colormap, norm and limits of colour-mapped artists (matplotlib's
//! `cmap`, `norm`, `vmin` and `vmax` keywords), for scalar data and for
//! data of several variates.

use plt_core::Rgba;

use super::{ImageError, MultiField};
use crate::colors::multivar::{MultiColormap, MultiMappable, MultivarError};
use crate::colors::{Colormap, MultiNorm, Norm, NormSpec, ScalarMappable, colormaps};
use crate::rc::RcParams;

/// How data is mapped to colours. Scalar data uses the colormap, norm and
/// limits, whose unset parts default to `image.cmap` and a linear norm
/// autoscaled to the data; data of several variates uses `multivariate`
/// instead (matplotlib takes a bivariate or multivariate colormap as
/// `cmap`).
///
/// ```
/// use plt::colors::{LogNorm, colormaps};
/// use plt::image::ColorizerOptions;
/// let options = ColorizerOptions::default()
///     .cmap(colormaps::get("magma")?)
///     .norm(LogNorm::default());
/// # Ok::<(), plt::colors::ColormapError>(())
/// ```
#[derive(Debug, Default)]
pub struct ColorizerOptions {
    /// The colormap (`None` = `image.cmap`).
    pub cmap: Option<Colormap>,
    /// The norm (`None` = a linear norm); excludes `vmin` and `vmax`.
    pub norm: Option<Box<dyn Norm>>,
    /// The value mapped to the bottom of the colormap.
    pub vmin: Option<f64>,
    /// The value mapped to the top of the colormap.
    pub vmax: Option<f64>,
    /// The colormap, norms and limits of data of several variates; scalar
    /// data rejects them.
    pub multivariate: Option<MultivarColorizerOptions>,
}

impl ColorizerOptions {
    /// Uses `cmap`.
    #[must_use]
    pub fn cmap(mut self, cmap: Colormap) -> Self {
        self.cmap = Some(cmap);
        self
    }

    /// Uses `norm`.
    #[must_use]
    pub fn norm(mut self, norm: impl Norm + 'static) -> Self {
        self.norm = Some(Box::new(norm));
        self
    }

    /// Fixes the colour limits (`None` = autoscaled from the data).
    #[must_use]
    pub fn limits(mut self, vmin: Option<f64>, vmax: Option<f64>) -> Self {
        self.vmin = vmin;
        self.vmax = vmax;
        self
    }

    /// Maps data of several variates as `options` say.
    #[must_use]
    pub fn multivariate(mut self, options: MultivarColorizerOptions) -> Self {
        self.multivariate = Some(options);
        self
    }

    /// The colormap and norm (matplotlib's `_get_colorizer`) with the
    /// explicit limits applied (`set_clim`), not yet autoscaled.
    ///
    /// # Errors
    ///
    /// When `image.cmap` names an unknown colormap, both a norm and limits
    /// are given, or the options are for data of several variates.
    pub(crate) fn into_mappable(self, rc: &RcParams) -> Result<ScalarMappable, ImageError> {
        if self.multivariate.is_some() {
            return Err(ImageError::MultivariateColormap);
        }
        let cmap = match self.cmap {
            Some(cmap) => cmap,
            None => colormaps::get(&rc.image_cmap)?,
        };
        let limits = self.vmin.is_some() || self.vmax.is_some();
        let mut mappable = match self.norm {
            Some(_) if limits => return Err(ImageError::NormWithLimits),
            Some(norm) => ScalarMappable::with_norm(cmap, norm),
            None => ScalarMappable::new(cmap),
        };
        let norm = mappable.norm_mut();
        if self.vmin.is_some() {
            norm.set_vmin(self.vmin);
        }
        if self.vmax.is_some() {
            norm.set_vmax(self.vmax);
        }
        Ok(mappable)
    }

    /// The colours of `field`, whose norms are autoscaled to its variates
    /// (matplotlib's colorizer, `_scale_norm` and `to_rgba(A, alpha)`):
    /// `alpha` replaces (bivariate) or multiplies (multivariate) the
    /// colormap's opacity.
    ///
    /// # Errors
    ///
    /// When no multivariate options are given, scalar options are, or the
    /// norms and colormap do not fit the variates.
    pub(crate) fn multivariate_colors(
        self,
        field: &MultiField,
        alpha: Option<f64>,
    ) -> Result<Vec<Rgba>, ImageError> {
        let scalar = self.cmap.is_some()
            || self.norm.is_some()
            || self.vmin.is_some()
            || self.vmax.is_some();
        let Some(options) = self.multivariate.filter(|_| !scalar) else {
            return Err(ImageError::ScalarColorizer);
        };
        let mut mappable = options.into_mappable()?;
        let variates = mappable.cmap().n_variates();
        if field.n_variates() != variates {
            return Err(MultivarError::VariateCount {
                expected: variates,
                given: field.n_variates(),
            }
            .into());
        }
        let values = field.masked_values();
        mappable.autoscale_none(&values)?;
        Ok(mappable.to_rgba(&values, alpha)?)
    }
}

/// How data of several variates is mapped to colours: a bivariate or
/// multivariate colormap, one norm per variate given by scale name or as a
/// norm (linear when unset) and per-variate limits (matplotlib's `cmap`,
/// `norm`, `vmin` and `vmax` for multivariate data). Limits may be combined
/// with norms given as norms, as matplotlib allows for lists of norms.
///
/// ```
/// use plt::colors::{LogNorm, NormName, NormSpec};
/// use plt::colors::multivar::MultiColormap;
/// use plt::image::MultivarColorizerOptions;
/// let options = MultivarColorizerOptions::new(MultiColormap::get("BiPeak")?)
///     .norms([NormName::Linear.into(), NormSpec::norm(LogNorm::default())])
///     .limits(Some(vec![Some(0.0), None]), None);
/// # Ok::<(), plt::colors::multivar::MultivarError>(())
/// ```
#[derive(Debug)]
pub struct MultivarColorizerOptions {
    /// The colormap, taking one value per variate.
    pub cmap: MultiColormap,
    /// One norm per variate (`None` = linear norms).
    pub norms: Option<Vec<NormSpec>>,
    /// The value of each variate mapped to the bottom of its range; a
    /// `None` entry autoscales.
    pub vmin: Option<Vec<Option<f64>>>,
    /// The value of each variate mapped to the top of its range; a `None`
    /// entry autoscales.
    pub vmax: Option<Vec<Option<f64>>>,
}

impl MultivarColorizerOptions {
    /// Linear norms autoscaled to the data, through `cmap`.
    pub fn new(cmap: impl Into<MultiColormap>) -> Self {
        Self {
            cmap: cmap.into(),
            norms: None,
            vmin: None,
            vmax: None,
        }
    }

    /// Uses one of `norms` per variate.
    #[must_use]
    pub fn norms(mut self, norms: impl IntoIterator<Item = NormSpec>) -> Self {
        self.norms = Some(norms.into_iter().collect());
        self
    }

    /// Fixes the limits of the variates (`None` = autoscaled).
    #[must_use]
    pub fn limits(
        mut self,
        vmin: Option<Vec<Option<f64>>>,
        vmax: Option<Vec<Option<f64>>>,
    ) -> Self {
        self.vmin = vmin;
        self.vmax = vmax;
        self
    }

    /// The norms and colormap with the explicit limits applied, not yet
    /// autoscaled (matplotlib's `Colorizer` and `set_clim`).
    fn into_mappable(self) -> Result<MultiMappable, ImageError> {
        let norm = match self.norms {
            Some(norms) => MultiNorm::new(norms)?,
            None => MultiNorm::linear(self.cmap.n_variates())?,
        };
        let mut mappable = MultiMappable::with_norm(self.cmap, norm)?;
        mappable.set_clim(self.vmin.as_deref(), self.vmax.as_deref())?;
        Ok(mappable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalize;

    #[test]
    fn defaults_and_limits() {
        let rc = RcParams::matplotlib();
        let mappable = ColorizerOptions::default()
            .limits(Some(-1.0), None)
            .into_mappable(&rc)
            .unwrap();
        assert_eq!(mappable.cmap().name(), "viridis");
        assert_eq!(mappable.norm().vmin(), Some(-1.0));
        assert_eq!(mappable.norm().vmax(), None);
    }

    fn bipeak() -> MultivarColorizerOptions {
        MultivarColorizerOptions::new(MultiColormap::get("BiPeak").unwrap())
    }

    fn field() -> MultiField {
        let a = crate::numeric::Grid::from_vec(1, 2, vec![0.0, 1.0]).unwrap();
        MultiField::new([a.clone(), a]).unwrap()
    }

    #[test]
    fn scalar_and_multivariate_options_do_not_mix() {
        let rc = RcParams::matplotlib();
        let multi = ColorizerOptions::default().multivariate(bipeak());
        assert_eq!(
            multi.into_mappable(&rc).err(),
            Some(ImageError::MultivariateColormap)
        );
        let scalar = ColorizerOptions::default().limits(Some(0.0), None);
        assert_eq!(
            scalar.multivariate_colors(&field(), None).err(),
            Some(ImageError::ScalarColorizer)
        );
        let both = ColorizerOptions::default()
            .multivariate(bipeak())
            .limits(None, Some(1.0));
        assert_eq!(
            both.multivariate_colors(&field(), None).err(),
            Some(ImageError::ScalarColorizer)
        );
    }

    #[test]
    fn multivariate_options_fit_the_variates() {
        let three = MultivarColorizerOptions::new(MultiColormap::get("3VarAddA").unwrap());
        let err = ColorizerOptions::default()
            .multivariate(three)
            .multivariate_colors(&field(), None);
        assert!(matches!(
            err,
            Err(ImageError::Multivar(MultivarError::VariateCount { .. }))
        ));
        let norms = bipeak().norms([crate::colors::NormName::Log.into()]);
        let err = ColorizerOptions::default()
            .multivariate(norms)
            .multivariate_colors(&field(), None);
        assert!(matches!(
            err,
            Err(ImageError::Multivar(MultivarError::NormCount { .. }))
        ));
        let short = bipeak().limits(Some(vec![Some(0.0)]), None);
        assert!(
            ColorizerOptions::default()
                .multivariate(short)
                .multivariate_colors(&field(), None)
                .is_err()
        );
        let colors = ColorizerOptions::default()
            .multivariate(bipeak())
            .multivariate_colors(&field(), Some(0.5))
            .unwrap();
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0].a, 0.5);
    }

    #[test]
    fn a_norm_excludes_limits() {
        let rc = RcParams::matplotlib();
        let options = ColorizerOptions::default()
            .norm(Normalize::default())
            .limits(None, Some(2.0));
        assert_eq!(
            options.into_mappable(&rc).err(),
            Some(ImageError::NormWithLimits)
        );
    }
}
