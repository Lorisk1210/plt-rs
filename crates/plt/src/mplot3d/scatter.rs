//! Scatter plots in 3-D (matplotlib's `Axes3D.scatter`).

use super::art3d::{Art3D, Points3D};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::plot::broadcast;
use crate::artist::{Artist, ArtistHandle};
use crate::axes::ScatterOptions;
use crate::collections::{Collection, CollectionArtist};

/// The smallest z margin of an axes holding a scatter plot.
const SCATTER_ZMARGIN: f64 = 0.05;

/// Options of [`Axes3D::scatter_with`].
#[derive(Debug, Default)]
pub struct Scatter3DOptions {
    /// The axis normal to the plane of the 2-D points, along which `z`
    /// is measured.
    pub zdir: Axis3,
    /// Whether markers fade with depth; default `axes3d.depthshade`.
    pub depthshade: Option<bool>,
    /// The alpha factor of the deepest markers; default
    /// `axes3d.depthshade_minalpha`.
    pub depthshade_minalpha: Option<f64>,
    /// The 2-D scatter options: sizes, colours, marker, edges, alpha.
    pub scatter: ScatterOptions,
}

impl Axes3D {
    /// A scatter plot of the points `(x, y, z)` with default styling;
    /// `z` holds one value per point or one for all.
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    pub fn scatter(&self, x: &[f64], y: &[f64], z: &[f64]) -> Result<Collection, Axes3DError> {
        self.scatter_with(x, y, z, Scatter3DOptions::default())
    }

    /// A scatter plot with explicit options; markers are drawn back to
    /// front and, with depth shading, fade with depth (matplotlib's
    /// `Path3DCollection`).
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length, or as
    /// [`crate::axes::Axes::scatter_with`].
    pub fn scatter_with(
        &self,
        x: &[f64],
        y: &[f64],
        z: &[f64],
        options: Scatter3DOptions,
    ) -> Result<Collection, Axes3DError> {
        check_len("y", x.len(), y.len())?;
        let z = broadcast("z", z, x.len())?;
        let had_data = self.has_data();
        let rc = crate::rc::params();
        let collection = self
            .plane()
            .scatter_with(x.to_vec(), y.to_vec(), options.scatter)?;
        let points = collection.raw().with_mut::<CollectionArtist, _>(
            |c| -> Result<Points3D, Axes3DError> {
                let (faces, edges) = c.resolved_colors().map_err(Axes3DError::Style)?;
                // The depth-sorted colours are written at every projection,
                // with the alpha applied by the 3-D collection itself.
                c.mapping = None;
                let alpha = c.base_mut().alpha.take();
                Ok(Points3D {
                    positions: c
                        .offsets
                        .iter()
                        .zip(&z)
                        .map(|(o, &z)| options.zdir.juggle([o.x, o.y, z]))
                        .collect(),
                    faces,
                    edges,
                    sizes: c.sizes.clone(),
                    linewidths: c.linewidths.clone(),
                    alpha,
                    depthshade: options.depthshade.unwrap_or(rc.axes3d_depthshade),
                    min_alpha: options
                        .depthshade_minalpha
                        .unwrap_or(rc.axes3d_depthshade_minalpha),
                })
            },
        )?;
        self.register(collection.id(), Art3D::Points(points));
        self.with_mut(|axes| {
            let zmargin = &mut axes.limits.margins[Axis3::Z.index()];
            if *zmargin < SCATTER_ZMARGIN && !x.is_empty() {
                *zmargin = SCATTER_ZMARGIN;
            }
        });
        self.auto_scale_xyz(x, y, Some(&z), had_data)?;
        Ok(collection)
    }
}
