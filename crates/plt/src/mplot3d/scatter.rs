//! Scatter plots in 3-D (matplotlib's `Axes3D.scatter`).

use super::art3d::{Art3D, Points3D};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::plot::spread;
use crate::artist::ArtistHandle;
use crate::axes::ScatterOptions;
use crate::collections::{Collection, CollectionArtist};
use crate::units::AxisValues;

/// The smallest z margin of an axes holding a scatter plot (and, in
/// classic mode, the smallest x and y margins).
const SCATTER_MARGIN: f64 = 0.05;

/// The marker area of 3-D scatter plots in points² when none is given
/// (the `s=20` default of matplotlib's `Axes3D.scatter`, whatever
/// `lines.markersize` says).
const DEFAULT_SIZE: f64 = 20.0;

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
    /// The 2-D scatter options: sizes, colours, marker, edges, alpha;
    /// the marker areas default to 20 points² (matplotlib's
    /// `Axes3D.scatter(s=20)`), whatever the style.
    pub scatter: ScatterOptions,
}

impl Axes3D {
    /// A scatter plot of the points `(x, y, z)` with default styling;
    /// `z` holds one value per point or one for all. The coordinates are
    /// numbers, dates, category names or user values, converted on their
    /// axes as by [`Axes3D::plot`].
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    pub fn scatter(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
    ) -> Result<Collection, Axes3DError> {
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
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
        options: Scatter3DOptions,
    ) -> Result<Collection, Axes3DError> {
        let values = [x.into(), y.into(), z.into()];
        check_len("y", values[0].len(), values[1].len())?;
        if values[2].len() != 1 {
            check_len("z", values[0].len(), values[2].len())?;
        }
        let had_data = self.has_data();
        let [x, y, z] = self.convert_juggled(values, options.zdir);
        let z = spread(z, x.len());
        let rc = crate::rc::params();
        let scatter = ScatterOptions {
            sizes: Some(options.scatter.sizes.unwrap_or_else(|| vec![DEFAULT_SIZE])),
            ..options.scatter
        };
        let collection = self.plane().scatter_with(x.clone(), y.clone(), scatter)?;
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
                        depthshade: options.depthshade.unwrap_or(rc.axes3d_depthshade),
                        min_alpha: options
                            .depthshade_minalpha
                            .unwrap_or(rc.axes3d_depthshade_minalpha),
                        depth_sorted: true,
                        sort_zpos: None,
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
        self.auto_scale_xyz(&x, &y, Some(&z), had_data)?;
        Ok(collection)
    }

    /// Turns the depth shading of a scatter plot of this axes on or off
    /// (matplotlib's `Path3DCollection.set_depthshade`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let points = ax.scatter(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_depthshade(&points, false)?;
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `scatter` is not a scatter plot of this axes.
    pub fn set_depthshade(
        &self,
        scatter: &Collection,
        depthshade: bool,
    ) -> Result<&Self, Axes3DError> {
        let id = self.own_id(scatter);
        let found = self.with_mut(|axes| {
            let entry = axes.artists.iter_mut().find(|a| Some(a.id) == id)?;
            let Art3D::Points(points) = &mut entry.art else {
                return None;
            };
            points.depthshade = depthshade;
            Some(())
        });
        found.map(|()| self).ok_or(Axes3DError::Not3DArtist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::MappedColors;
    use crate::collections::CollectionProps;
    use crate::colors::{Colorize, colormaps};
    use crate::figure::Figure;

    #[test]
    fn a_new_colormap_recolours_the_markers_after_a_draw() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let jet = colormaps::get("jet").unwrap();
        let options = Scatter3DOptions {
            depthshade: Some(false),
            scatter: ScatterOptions {
                colors: MappedColors::new(vec![0.0, 1.0]).cmap(jet).into(),
                ..ScatterOptions::default()
            },
            ..Scatter3DOptions::default()
        };
        let points = ax
            .scatter_with([0.0, 1.0], [0.0, 1.0], [0.0, 1.0], options)
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
    fn later_sizes_widths_and_shading_are_drawn_in_depth_order() {
        // matplotlib's test_scatter3d_modification and
        // test_scatter3d_linewidth_modification.
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        // From the nearest marker to the deepest in the default view.
        let x = [2.0, 1.0, 0.0];
        let points = ax.scatter(x, x, x).unwrap();
        assert_eq!(points.sizes(), Some(vec![DEFAULT_SIZE]));
        points
            .set_sizes(vec![10.0, 20.0, 30.0])
            .set_linewidths(vec![1.0, 2.0, 3.0]);
        ax.set_depthshade(&points, false).unwrap();
        fig.layout();
        let (sizes, widths, alphas) = points.raw().with::<CollectionArtist, _>(|c| {
            let faces = c.resolved_colors().unwrap().0;
            (
                c.drawn_sizes().map(<[f64]>::to_vec).unwrap(),
                c.drawn_linewidths(),
                faces.iter().map(|face| face.a).collect::<Vec<_>>(),
            )
        });
        assert_eq!(sizes, [30.0, 20.0, 10.0], "drawn back to front");
        assert_eq!(widths, [3.0, 2.0, 1.0]);
        assert_eq!(alphas, [1.0], "one colour, not faded with depth");
        assert_eq!(points.sizes(), Some(vec![10.0, 20.0, 30.0]));
        let segments = vec![vec![[0.0; 3], [1.0; 3]]];
        let lines = ax
            .add_line_collection3d(segments, CollectionProps::default(), false)
            .unwrap();
        assert_eq!(
            ax.set_depthshade(&lines, true).err(),
            Some(Axes3DError::Not3DArtist)
        );
    }

    #[test]
    fn classic_scatter_plots_leave_room_around_x_and_y() {
        let margins = |classic: &str| {
            crate::rc::context(|| {
                crate::rc::set("_internal.classic_mode", classic).unwrap();
                crate::rc::set("axes.xmargin", "0").unwrap();
                let ax = Figure::new().subplot_3d();
                ax.scatter([0.0, 4.0], [0.0, 4.0], [0.0, 4.0]).unwrap();
                Axis3::ALL.map(|axis| ax.margin(axis))
            })
        };
        assert_eq!(margins("True")[..2], [0.05, 0.05]);
        assert_eq!(margins("False")[0], 0.0, "only classic mode adds them");
        assert_eq!(margins("False")[2], 0.05, "z always gets room");
    }
}
