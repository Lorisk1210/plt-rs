//! Scatter plots in 3-D (matplotlib's `Axes3D.scatter`).

use super::art3d::{Art3D, Points3D};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::plot::broadcast;
use crate::artist::ArtistHandle;
use crate::axes::ScatterOptions;
use crate::collections::{Collection, CollectionArtist};

/// The smallest z margin of an axes holding a scatter plot (and, in
/// classic mode, the smallest x and y margins).
const SCATTER_MARGIN: f64 = 0.05;

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
    /// Whether the markers outside the view limits are hidden; the depth
    /// shading then spans the visible markers only (matplotlib's
    /// `axlim_clip`).
    pub axlim_clip: bool,
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
        let points =
            collection
                .raw()
                .with::<CollectionArtist, _>(|c| -> Result<Points3D, Axes3DError> {
                    // The colours are resolved, shaded and sorted at every
                    // projection; reject invalid ones now, as matplotlib does.
                    c.base_colors().map_err(Axes3DError::Style)?;
                    Ok(Points3D {
                        positions: c
                            .offsets
                            .iter()
                            .zip(&z)
                            .map(|(o, &z)| options.zdir.juggle([o.x, o.y, z]))
                            .collect(),
                        sizes: c.sizes.clone(),
                        linewidths: c.linewidths.clone(),
                        depthshade: options.depthshade.unwrap_or(rc.axes3d_depthshade),
                        min_alpha: options
                            .depthshade_minalpha
                            .unwrap_or(rc.axes3d_depthshade_minalpha),
                    })
                })?;
        self.register(collection.id(), Art3D::Points(points), options.axlim_clip);
        self.with_mut(|axes| {
            let margins = &mut axes.limits.margins;
            if !x.is_empty() {
                // z always gets room for the markers; in classic mode x and
                // y get it too, from the 2-D scatter the 3-D one calls.
                let widened: &[Axis3] = if rc.internal_classic_mode {
                    &Axis3::ALL
                } else {
                    &[Axis3::Z]
                };
                for axis in widened {
                    let margin = &mut margins[axis.index()];
                    *margin = margin.max(SCATTER_MARGIN);
                }
            }
        });
        self.auto_scale_xyz(x, y, Some(&z), had_data)?;
        Ok(collection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::MappedColors;
    use crate::colors::colormaps;
    use crate::figure::Figure;

    #[test]
    fn a_new_colormap_recolours_the_markers_after_a_draw() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let jet = colormaps::get("jet").unwrap();
        let options = Scatter3DOptions {
            depthshade: Some(false),
            scatter: ScatterOptions {
                colors: MappedColors::new(vec![0.0, 1.0]).with_cmap(jet).into(),
                ..ScatterOptions::default()
            },
            ..Scatter3DOptions::default()
        };
        let points = ax
            .scatter_with(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0], options)
            .unwrap();
        fig.layout();
        let viridis = colormaps::get("viridis").unwrap();
        points.set_cmap(viridis.clone());
        fig.layout();
        let mut faces = points.colors().unwrap().faces;
        faces.sort_by(|a, b| a.r.total_cmp(&b.r));
        let ends = [viridis.lut()[0], viridis.lut()[viridis.n() - 1]];
        assert_eq!(faces, ends, "the markers take the new colormap");
    }

    #[test]
    fn classic_scatter_plots_leave_room_around_x_and_y() {
        let margins = |classic: &str| {
            crate::rc::context(|| {
                crate::rc::set("_internal.classic_mode", classic).unwrap();
                crate::rc::set("axes.xmargin", "0").unwrap();
                let ax = Figure::new().subplot_3d();
                ax.scatter(&[0.0, 4.0], &[0.0, 4.0], &[0.0, 4.0]).unwrap();
                Axis3::ALL.map(|axis| ax.margin(axis))
            })
        };
        assert_eq!(margins("True")[..2], [0.05, 0.05]);
        assert_eq!(margins("False")[0], 0.0, "only classic mode adds them");
        assert_eq!(margins("False")[2], 0.05, "z always gets room");
    }
}
