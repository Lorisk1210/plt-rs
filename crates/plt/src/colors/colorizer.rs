//! How plotting functions map values to colours: a colormap, and a norm
//! or the limits of a linear one (matplotlib's `cmap`, `norm`, `vmin` and
//! `vmax` keywords).

use std::fmt;

use super::{Colormap, ColormapError, Norm, ScalarMappable, colormaps};
use crate::rc::RcParams;

/// A colormap, and a norm or limits, mapping values to colours
/// (matplotlib's `cmap`, `norm`, `vmin` and `vmax` keywords). Unset parts
/// default to `image.cmap` and a linear norm autoscaled to the data.
///
/// Every function that maps values to colours takes one in the
/// `colorizer` field of its options, which implement [`Colorize`] for the
/// builders below. matplotlib refuses a norm together with limits in most
/// functions; where it lets one win over the other (`contour`,
/// `plot_surface`, `LightSource.shade`), the function says so.
///
/// ```
/// use plt::colors::{Colorize, ColorizerOptions, LogNorm, colormaps};
/// let colorizer = ColorizerOptions::default()
///     .cmap(colormaps::get("magma")?)
///     .norm(LogNorm::default());
/// let limited = ColorizerOptions::default().limits(Some(0.0), None);
/// assert_eq!((limited.vmin, limited.vmax), (Some(0.0), None));
/// # Ok::<(), plt::colors::ColormapError>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct ColorizerOptions {
    /// The colormap (`None` = `image.cmap`).
    pub cmap: Option<Colormap>,
    /// The norm (`None` = a linear norm between `vmin` and `vmax`).
    pub norm: Option<Box<dyn Norm>>,
    /// The value mapped to the bottom of the colormap (`None` = the
    /// smallest value).
    pub vmin: Option<f64>,
    /// The value mapped to the top of the colormap (`None` = the largest
    /// value).
    pub vmax: Option<f64>,
}

impl ColorizerOptions {
    /// The colormap, `image.cmap` when unset.
    pub(crate) fn colormap(&self, rc: &RcParams) -> Result<Colormap, ColormapError> {
        match &self.cmap {
            Some(cmap) => Ok(cmap.clone()),
            None => colormaps::get(&rc.image_cmap),
        }
    }

    /// Whether any part is set.
    pub(crate) fn is_set(&self) -> bool {
        self.cmap.is_some() || self.norm.is_some() || self.vmin.is_some() || self.vmax.is_some()
    }

    /// The colormap and norm with the limits applied, not yet autoscaled
    /// (matplotlib's `_get_colorizer` and `_scale_norm`, which refuses a
    /// norm together with limits).
    ///
    /// # Errors
    ///
    /// When `image.cmap` names an unknown colormap, or both a norm and
    /// limits are given.
    pub(crate) fn into_mappable(self, rc: &RcParams) -> Result<ScalarMappable, ColorizerError> {
        let cmap = self.colormap(rc)?;
        let limits = self.vmin.is_some() || self.vmax.is_some();
        let mut mappable = match self.norm {
            Some(_) if limits => return Err(ColorizerError::NormWithLimits),
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

/// The options of a function mapping values to colours, which hold a
/// [`ColorizerOptions`] in their `colorizer` field: the builders setting
/// its parts.
///
/// ```
/// use plt::axes::ImshowOptions;
/// use plt::colors::{Colorize, colormaps};
/// let options = ImshowOptions::default()
///     .cmap(colormaps::get("gray")?)
///     .limits(Some(-1.0), Some(1.0));
/// assert_eq!(options.colorizer.vmax, Some(1.0));
/// # Ok::<(), plt::colors::ColormapError>(())
/// ```
pub trait Colorize: Sized {
    /// The colormap, norm and limits.
    fn colorizer_mut(&mut self) -> &mut ColorizerOptions;

    /// Uses the colormap, norm and limits of `colorizer`.
    #[must_use]
    fn colorizer(mut self, colorizer: ColorizerOptions) -> Self {
        *self.colorizer_mut() = colorizer;
        self
    }

    /// Uses `cmap` instead of `image.cmap`.
    #[must_use]
    fn cmap(mut self, cmap: Colormap) -> Self {
        self.colorizer_mut().cmap = Some(cmap);
        self
    }

    /// Uses `norm` instead of a linear norm.
    #[must_use]
    fn norm(mut self, norm: impl Norm + 'static) -> Self {
        self.colorizer_mut().norm = Some(Box::new(norm));
        self
    }

    /// Fixes the values mapped to the ends of the colormap (`None` =
    /// autoscaled from the data).
    #[must_use]
    fn limits(mut self, vmin: Option<f64>, vmax: Option<f64>) -> Self {
        let colorizer = self.colorizer_mut();
        colorizer.vmin = vmin;
        colorizer.vmax = vmax;
        self
    }
}

impl Colorize for ColorizerOptions {
    fn colorizer_mut(&mut self) -> &mut ColorizerOptions {
        self
    }
}

/// Implements [`Colorize`] for option structs through their `colorizer`
/// field.
macro_rules! impl_colorize {
    ($($options:ty),+ $(,)?) => {$(
        impl $crate::colors::Colorize for $options {
            fn colorizer_mut(&mut self) -> &mut $crate::colors::ColorizerOptions {
                &mut self.colorizer
            }
        }
    )+};
}

pub(crate) use impl_colorize;

/// Why colorizer options cannot map values.
#[derive(Clone, Debug, PartialEq)]
pub enum ColorizerError {
    /// An unknown or invalid colormap.
    Colormap(ColormapError),
    /// Both a norm and explicit `vmin`/`vmax` were given.
    NormWithLimits,
}

impl fmt::Display for ColorizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Colormap(err) => err.fmt(f),
            Self::NormWithLimits => f.write_str(NORM_WITH_LIMITS),
        }
    }
}

impl std::error::Error for ColorizerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colormap(err) => Some(err),
            Self::NormWithLimits => None,
        }
    }
}

impl From<ColormapError> for ColorizerError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

/// matplotlib's message for a norm given together with limits.
pub(crate) const NORM_WITH_LIMITS: &str =
    "passing a Normalize instance simultaneously with vmin/vmax is not supported";

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
        assert!(!ColorizerOptions::default().is_set());
    }

    #[test]
    fn a_norm_excludes_limits() {
        let rc = RcParams::matplotlib();
        let options = ColorizerOptions::default()
            .norm(Normalize::default())
            .limits(None, Some(2.0));
        assert!(options.is_set());
        assert_eq!(
            options.into_mappable(&rc).err(),
            Some(ColorizerError::NormWithLimits)
        );
    }

    #[test]
    fn an_unknown_rc_colormap_is_an_error() {
        let mut rc = RcParams::matplotlib();
        rc.image_cmap = "not a colormap".to_owned();
        assert!(matches!(
            ColorizerOptions::default().into_mappable(&rc),
            Err(ColorizerError::Colormap(_))
        ));
    }
}
