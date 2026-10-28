//! Multivariate data → colour mapping through a [`MultiNorm`] and a
//! [`MultiColormap`] (matplotlib's `Colorizer` with a bivariate or
//! multivariate colormap).

use plt_core::Rgba;

use super::{MultiColormap, MultivarError};
use crate::colors::{MultiNorm, Normalized};

/// One norm per variate and a colormap taking every variate: the
/// colouring machinery of multivariate data.
///
/// ```
/// use plt::colors::multivar::{MultiColormap, MultiMappable};
/// let mut mappable = MultiMappable::new(MultiColormap::get("2VarAddA")?);
/// let data = [vec![Some(0.0), Some(4.0)], vec![Some(1.0), None]];
/// let colors = mappable.to_rgba(&data, None)?;
/// assert_eq!(colors[0].to_array(), [0.0, 0.0, 0.0, 1.0], "both variates at their minimum");
/// assert_eq!(colors[1].a, 0.0, "a masked variate is bad");
/// # Ok::<(), plt::colors::multivar::MultivarError>(())
/// ```
#[derive(Clone, Debug)]
pub struct MultiMappable {
    norm: MultiNorm,
    cmap: MultiColormap,
}

impl MultiMappable {
    /// A mappable with autoscaling linear norms (matplotlib's default
    /// `MultiNorm(["linear"] * n_variates)`).
    pub fn new(cmap: MultiColormap) -> Self {
        let norm = MultiNorm::linear(cmap.n_variates())
            .expect("colormaps of several variates have at least two");
        Self { norm, cmap }
    }

    /// A mappable with the given norms.
    ///
    /// # Errors
    ///
    /// [`MultivarError::NormCount`] unless there is one norm per variate.
    pub fn with_norm(cmap: MultiColormap, norm: MultiNorm) -> Result<Self, MultivarError> {
        check_norms(&cmap, &norm)?;
        Ok(Self { norm, cmap })
    }

    /// The norms.
    pub fn norm(&self) -> &MultiNorm {
        &self.norm
    }

    /// The norms, mutably (e.g. to set limits).
    pub fn norm_mut(&mut self) -> &mut MultiNorm {
        &mut self.norm
    }

    /// The colormap.
    pub fn cmap(&self) -> &MultiColormap {
        &self.cmap
    }

    /// Replaces the colormap.
    ///
    /// # Errors
    ///
    /// [`MultivarError::NormCount`] when the colormap takes another number
    /// of variates than there are norms.
    pub fn set_cmap(&mut self, cmap: MultiColormap) -> Result<&mut Self, MultivarError> {
        check_norms(&cmap, &self.norm)?;
        self.cmap = cmap;
        Ok(self)
    }

    /// Sets the limits of every norm (matplotlib's `set_clim`); `None`
    /// leaves that side unchanged, while `None` entries clear a limit.
    ///
    /// # Errors
    ///
    /// When a list does not hold one limit per variate.
    pub fn set_clim(
        &mut self,
        vmin: Option<&[Option<f64>]>,
        vmax: Option<&[Option<f64>]>,
    ) -> Result<&mut Self, MultivarError> {
        if let Some(vmin) = vmin {
            self.norm.set_vmin(vmin)?;
        }
        if let Some(vmax) = vmax {
            self.norm.set_vmax(vmax)?;
        }
        Ok(self)
    }

    /// The limits of every norm (matplotlib's `get_clim`).
    pub fn clim(&self) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
        (self.norm.vmin(), self.norm.vmax())
    }

    /// Fills the unset limits of each norm from its variate of `data`.
    ///
    /// # Errors
    ///
    /// When `data` does not hold one sequence per variate.
    pub fn autoscale_none(
        &mut self,
        data: &[impl AsRef<[Option<f64>]>],
    ) -> Result<(), MultivarError> {
        Ok(self.norm.autoscale_none(data)?)
    }

    /// The colours of `data`, one sequence per variate holding one value
    /// per point (`None` = masked), with an optional alpha (matplotlib's
    /// `to_rgba`); unset limits autoscale first.
    ///
    /// # Errors
    ///
    /// When `data` does not hold one sequence per variate, the sequences
    /// differ in length, or a norm or the colormap rejects the values.
    pub fn to_rgba(
        &mut self,
        data: &[impl AsRef<[Option<f64>]>],
        alpha: Option<f64>,
    ) -> Result<Vec<Rgba>, MultivarError> {
        let normalized = self.norm.normalize(data, None)?;
        let points = normalized.first().map_or(0, Vec::len);
        if let Some(other) = normalized.iter().find(|variate| variate.len() != points) {
            return Err(MultivarError::Lengths {
                first: points,
                other: other.len(),
            });
        }
        (0..points)
            .map(|i| {
                let point: Vec<Normalized> = normalized.iter().map(|variate| variate[i]).collect();
                self.cmap.lookup(&point, alpha)
            })
            .collect()
    }
}

/// Fails unless `norm` has one norm per variate of `cmap`.
fn check_norms(cmap: &MultiColormap, norm: &MultiNorm) -> Result<(), MultivarError> {
    if cmap.n_variates() == norm.n_components() {
        Ok(())
    } else {
        Err(MultivarError::NormCount {
            variates: cmap.n_variates(),
            norms: norm.n_components(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{NormName, NormSpec};

    fn bivar() -> MultiColormap {
        MultiColormap::get("BiOrangeBlue").unwrap()
    }

    #[test]
    fn norms_must_match_the_variates() {
        let three = MultiNorm::linear(3).unwrap();
        assert_eq!(
            MultiMappable::with_norm(bivar(), three).err(),
            Some(MultivarError::NormCount {
                variates: 2,
                norms: 3
            })
        );
        let mut mappable = MultiMappable::new(bivar());
        let three = MultiColormap::get("3VarAddA").unwrap();
        assert!(mappable.set_cmap(three).is_err());
        assert!(mappable.set_clim(Some(&[Some(1.0)]), None).is_err());
    }

    #[test]
    fn limits_autoscale_and_map() {
        let norm =
            MultiNorm::new([NormName::Linear.into(), NormSpec::Name(NormName::Log)]).unwrap();
        let mut mappable = MultiMappable::with_norm(bivar(), norm).unwrap();
        mappable
            .set_clim(Some(&[Some(0.0), None]), Some(&[Some(2.0), None]))
            .unwrap();
        let data = [vec![Some(2.0), Some(0.0)], vec![Some(100.0), Some(1.0)]];
        mappable.autoscale_none(&data).unwrap();
        assert_eq!(
            mappable.clim(),
            (vec![Some(0.0), Some(1.0)], vec![Some(2.0), Some(100.0)])
        );
        let colors = mappable.to_rgba(&data, Some(0.5)).unwrap();
        assert_eq!(colors[0].to_array(), [1.0, 1.0, 1.0, 0.5]);
        let uneven = [vec![Some(1.0)], vec![Some(1.0), Some(2.0)]];
        assert_eq!(
            mappable.to_rgba(&uneven, None),
            Err(MultivarError::Lengths { first: 1, other: 2 })
        );
    }
}
