//! Triangulated surfaces (matplotlib's `plot_trisurf`).

use super::art3d::ZSort;
use super::collections3d::{FaceColors, NewPolys, PolyFaces};
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::shade::LightSource;
use crate::collections::{Collection, CollectionProps};
use crate::tri::Triangulation;

/// Options of [`Axes3D::plot_trisurf`].
#[derive(Clone, Debug, Default)]
pub struct TrisurfOptions {
    /// How the triangles are coloured.
    pub colors: FaceColors,
    /// Whether colours are shaded by the light; by default unless
    /// colour-mapped.
    pub shade: Option<bool>,
    /// The light shading the triangles.
    pub lightsource: Option<LightSource>,
    /// Further collection properties (alpha, edge colour, width, ...).
    pub props: CollectionProps,
    /// Whether the points outside the view limits are hidden, breaking the
    /// triangles reaching them (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
}

impl Axes3D {
    /// A surface of the unmasked triangles of `tri` lifted to one `z` per
    /// point (matplotlib's `plot_trisurf`).
    ///
    /// # Errors
    ///
    /// When `z` does not have one value per point or a colour is invalid.
    pub fn plot_trisurf(
        &self,
        tri: &Triangulation,
        z: &[f64],
        options: TrisurfOptions,
    ) -> Result<Collection, Axes3DError> {
        check_len("z", tri.npoints(), z.len())?;
        let had_data = self.has_data();
        // matplotlib takes the cycle colour even when colour-mapping.
        let cycle_color = self.plane().next_line_color().resolve()?;
        let (x, y) = (tri.x(), tri.y());
        let faces: Vec<Vec<Vec3>> = tri
            .drawn_triangles()
            .iter()
            .map(|t| t.iter().map(|&i| [x[i], y[i], z[i]]).collect())
            .collect();
        let shade = options
            .shade
            .unwrap_or_else(|| options.colors.shaded_by_default());
        let colors = match &options.colors {
            FaceColors::Cycle => PolyFaces::Colors(vec![cycle_color]),
            other => self.face_colors(other, &faces)?,
        };
        let mapped = matches!(colors, PolyFaces::Mapped(_));
        let light =
            (shade && !mapped).then(|| options.lightsource.unwrap_or_else(LightSource::mplot3d));
        let collection = self.add_polys(NewPolys {
            faces,
            colors,
            shaded_edges: None,
            light,
            zsort: ZSort::Average,
            props: options.props,
            axlim_clip: options.axlim_clip,
        })?;
        self.auto_scale_xyz(x, y, Some(z), had_data)?;
        Ok(collection)
    }
}
