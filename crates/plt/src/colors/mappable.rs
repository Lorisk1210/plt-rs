//! Data → colour mapping through a norm and a colormap (matplotlib's
//! `ScalarMappable` / `Colorizer`).

use plt_core::Rgba;

use super::{Colormap, Norm, NormError, Normalize, Normalized};

/// A norm and a colormap applied together: the colouring machinery of
/// images, collections and colorbars.
///
/// ```
/// use plt::colors::{ScalarMappable, colormaps};
/// let mut mappable = ScalarMappable::new(colormaps::get("viridis")?);
/// let colors = mappable.to_rgba(&[Some(0.0), Some(10.0), None], None)?;
/// assert_eq!(colors[0], mappable.cmap().lut()[0], "autoscaled to the data");
/// assert_eq!(colors[2], mappable.cmap().bad());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct ScalarMappable {
    norm: Box<dyn Norm>,
    cmap: Colormap,
}

impl ScalarMappable {
    /// A mappable with an autoscaling linear [`Normalize`].
    pub fn new(cmap: Colormap) -> Self {
        Self::with_norm(cmap, Box::new(Normalize::default()))
    }

    /// A mappable with the given norm.
    pub fn with_norm(cmap: Colormap, norm: Box<dyn Norm>) -> Self {
        Self { norm, cmap }
    }

    /// The norm.
    pub fn norm(&self) -> &dyn Norm {
        self.norm.as_ref()
    }

    /// The norm, mutably (e.g. to set limits).
    pub fn norm_mut(&mut self) -> &mut dyn Norm {
        self.norm.as_mut()
    }

    /// Replaces the norm.
    pub fn set_norm(&mut self, norm: Box<dyn Norm>) {
        self.norm = norm;
    }

    /// The colormap.
    pub fn cmap(&self) -> &Colormap {
        &self.cmap
    }

    /// Replaces the colormap.
    pub fn set_cmap(&mut self, cmap: Colormap) {
        self.cmap = cmap;
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged.
    pub fn set_clim(&mut self, vmin: Option<f64>, vmax: Option<f64>) {
        if vmin.is_some() {
            self.norm.set_vmin(vmin);
        }
        if vmax.is_some() {
            self.norm.set_vmax(vmax);
        }
    }

    /// The data values mapped to the ends of the colormap (matplotlib's
    /// `get_clim`).
    pub fn clim(&self) -> (Option<f64>, Option<f64>) {
        (self.norm.vmin(), self.norm.vmax())
    }

    /// Normalises the data (autoscaling unset limits from it).
    pub fn normalize(&mut self, data: &[Option<f64>]) -> Result<Vec<Normalized>, NormError> {
        self.norm.normalize_masked(data, None)
    }

    /// The colours of `data` (`None` = masked), with an optional alpha
    /// override.
    pub fn to_rgba(
        &mut self,
        data: &[Option<f64>],
        alpha: Option<f64>,
    ) -> Result<Vec<Rgba>, NormError> {
        let values = self.normalize(data)?;
        Ok(values
            .into_iter()
            .map(|v| self.cmap.lookup(v, alpha))
            .collect())
    }

    /// The colours of `data` as 8-bit RGBA.
    pub fn to_rgba_bytes(
        &mut self,
        data: &[Option<f64>],
        alpha: Option<f64>,
    ) -> Result<Vec<[u8; 4]>, NormError> {
        let values = self.normalize(data)?;
        Ok(values
            .into_iter()
            .map(|v| self.cmap.lookup_bytes(v, alpha))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{BoundaryNorm, Extend, colormaps};

    #[test]
    fn discrete_norms_index_the_colormap() {
        let norm = BoundaryNorm::new([0.0, 2.0, 5.0, 10.0], 3, Extend::Neither).unwrap();
        let mut mappable =
            ScalarMappable::with_norm(colormaps::get("tab10").unwrap(), Box::new(norm));
        let colors = mappable.to_rgba(&[Some(1.0), Some(11.0)], None).unwrap();
        let cmap = mappable.cmap();
        assert_eq!(
            colors,
            [cmap.lut()[0], cmap.lut()[3]],
            "index ncolors, inside tab10"
        );
    }

    #[test]
    fn bytes_and_alpha() {
        let mut mappable = ScalarMappable::with_norm(
            colormaps::get("gray").unwrap(),
            Box::new(Normalize::new(0.0, 1.0)),
        );
        assert_eq!(
            mappable
                .to_rgba_bytes(&[Some(1.0), None], Some(0.5))
                .unwrap(),
            [[255, 255, 255, 127], [0, 0, 0, 0]]
        );
        assert_eq!(mappable.norm().vmax(), Some(1.0));
    }
}
