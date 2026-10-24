//! The colormap, norms and limits of data of several variates
//! (matplotlib's `cmap`, `norm`, `vmin` and `vmax` keywords given for
//! multivariate data).

use plt_core::Rgba;

use super::{ImageError, MultiField};
use crate::colors::multivar::{MultiColormap, MultiMappable, MultivarError};
use crate::colors::{MultiNorm, NormSpec};

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

    /// The colours of `field`, whose norms are autoscaled to its variates
    /// (matplotlib's colorizer, `_scale_norm` and `to_rgba(A, alpha)`):
    /// `alpha` replaces (bivariate) or multiplies (multivariate) the
    /// colormap's opacity.
    ///
    /// # Errors
    ///
    /// When the norms and colormap do not fit the variates.
    pub(crate) fn colors(
        self,
        field: &MultiField,
        alpha: Option<f64>,
    ) -> Result<Vec<Rgba>, ImageError> {
        let mut mappable = self.into_mappable()?;
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

    fn bipeak() -> MultivarColorizerOptions {
        MultivarColorizerOptions::new(MultiColormap::get("BiPeak").unwrap())
    }

    fn field() -> MultiField {
        let a = crate::numeric::Grid::from_vec(1, 2, vec![0.0, 1.0]).unwrap();
        MultiField::new([a.clone(), a]).unwrap()
    }

    #[test]
    fn multivariate_options_fit_the_variates() {
        let three = MultivarColorizerOptions::new(MultiColormap::get("3VarAddA").unwrap());
        let err = three.colors(&field(), None);
        assert!(matches!(
            err,
            Err(ImageError::Multivar(MultivarError::VariateCount { .. }))
        ));
        let norms = bipeak().norms([crate::colors::NormName::Log.into()]);
        let err = norms.colors(&field(), None);
        assert!(matches!(
            err,
            Err(ImageError::Multivar(MultivarError::NormCount { .. }))
        ));
        let short = bipeak().limits(Some(vec![Some(0.0)]), None);
        assert!(short.colors(&field(), None).is_err());
        let colors = bipeak().colors(&field(), Some(0.5)).unwrap();
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0].a, 0.5);
    }
}
