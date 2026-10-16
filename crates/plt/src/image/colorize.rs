//! The colormap, norm and limits of colour-mapped artists (matplotlib's
//! `cmap`, `norm`, `vmin` and `vmax` keywords).

use super::ImageError;
use crate::colors::{Colormap, Norm, ScalarMappable, colormaps};
use crate::rc::RcParams;

/// How scalar data is mapped to colours; unset parts default to
/// `image.cmap` and a linear norm autoscaled to the data.
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

    /// The colormap and norm (matplotlib's `_get_colorizer`) with the
    /// explicit limits applied (`set_clim`), not yet autoscaled.
    ///
    /// # Errors
    ///
    /// When `image.cmap` names an unknown colormap, or both a norm and
    /// limits are given.
    pub(crate) fn into_mappable(self, rc: &RcParams) -> Result<ScalarMappable, ImageError> {
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
