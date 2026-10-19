//! Adding collections to a 3-D axes (matplotlib's `add_collection3d`):
//! 2-D collections lifted into planes (`poly_collection_2d_to_3d` and
//! `line_collection_2d_to_3d`), and polylines and polygons through 3-D
//! points (`Line3DCollection` and `Poly3DCollection`).

use super::art3d::{Art3D, ZSort};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::{broadcast, columns};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, CollectionSpec,
};

impl Axes3D {
    /// Places the paths of a 2-D polygon or line collection in the planes
    /// normal to `zdir` at `zs` (one position, or one per path) and adds
    /// it, extending the data limits to it with `autolim` (matplotlib's
    /// `add_collection3d`). Lifted polygons keep their path codes and are
    /// ordered among the other collections by the depth of the lowest
    /// position; lines lose their non-finite points.
    /// [`Axes3D::set_axlim_clip`] on the result hides the points outside
    /// the view limits (matplotlib's `axlim_clip`; where a lifted polygon
    /// keeps only some of its vertices, matplotlib 3.11 fails to draw, here
    /// the hidden vertices leave gaps).
    ///
    /// ```
    /// use plt::collections::CollectionSpec;
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Axis3;
    /// use plt::transforms::Point;
    /// let ax = Figure::new().subplot_3d();
    /// let square = vec![Point::new(0.0, 0.0), Point::new(2.0, 0.0), Point::new(2.0, 2.0)];
    /// ax.add_collection3d(CollectionSpec::polygons(vec![square]), &[4.0], Axis3::Y, true)?;
    /// let (low, high) = ax.ylim();
    /// assert!(low < 4.0 && high > 4.0);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the collection is neither a polygon nor a line collection
    /// (matplotlib leaves those, and patch collections, in projected
    /// coordinates), or `zs` does not match the paths.
    pub fn add_collection3d(
        &self,
        spec: CollectionSpec,
        zs: &[f64],
        zdir: Axis3,
        autolim: bool,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let mut artist = spec.artist;
        let kind = artist.kind;
        if !matches!(kind, CollectionKind::Poly | CollectionKind::Line) {
            return Err(Axes3DError::UnsupportedCollection);
        }
        let paths = std::mem::take(&mut artist.paths);
        let positions = broadcast("zs", zs, paths.len())?;
        let lifted: Vec<Vec<Vec3>> = paths
            .iter()
            .zip(&positions)
            .map(|(path, &z)| {
                path.vertices()
                    .iter()
                    // Line segments come from `iter_segments`, which drops
                    // non-finite points.
                    .filter(|v| kind == CollectionKind::Poly || v.is_finite())
                    .map(|v| zdir.juggle([v.x, v.y, z]))
                    .collect()
            })
            .collect();
        let points: Vec<Vec3> = lifted.iter().flatten().copied().collect();
        let raw = self.adopt_projected(artist);
        if kind == CollectionKind::Poly {
            let sort_zpos = zs.iter().copied().reduce(f64::min).unwrap_or(0.0);
            self.register_polys(
                &raw,
                lifted,
                ZSort::Average,
                Some((paths, sort_zpos)),
                false,
            )?;
        } else {
            self.register(raw.id(), Art3D::Segments(lifted), false);
        }
        if autolim {
            self.scale_to(&points, had_data)?;
        }
        Ok(Collection::from_raw(raw))
    }

    /// Adds polylines through 3-D points (matplotlib's `add_collection3d`
    /// of a `Line3DCollection`), extending the data limits to every point
    /// with `autolim`; the polylines need not have equal lengths.
    ///
    /// # Errors
    ///
    /// When autoscaling fails.
    pub fn add_line_collection3d(
        &self,
        segments: Vec<Vec<Vec3>>,
        props: CollectionProps,
        autolim: bool,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let points: Vec<Vec3> = segments.iter().flatten().copied().collect();
        let collection = self.add_segments(segments, props, false);
        if autolim {
            self.scale_to(&points, had_data)?;
        }
        Ok(collection)
    }

    /// Adds polygons through 3-D vertices (matplotlib's `add_collection3d`
    /// of a `Poly3DCollection`), ordered among the other collections by
    /// the mean depth of their vertices, extending the data limits to
    /// every vertex with `autolim`.
    ///
    /// # Errors
    ///
    /// When a colour cannot be resolved or autoscaling fails.
    pub fn add_poly_collection3d(
        &self,
        polygons: Vec<Vec<Vec3>>,
        props: CollectionProps,
        autolim: bool,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let points: Vec<Vec3> = polygons.iter().flatten().copied().collect();
        let mut artist =
            CollectionArtist::new(CollectionKind::Poly, Vec::new(), crate::rc::params());
        props.apply(&mut artist);
        let raw = self.adopt_projected(artist);
        self.register_polys(&raw, polygons, ZSort::Average, None, false)?;
        if autolim {
            self.scale_to(&points, had_data)?;
        }
        Ok(Collection::from_raw(raw))
    }

    /// Extends the data limits to `points` and autoscales.
    fn scale_to(&self, points: &[Vec3], had_data: bool) -> Result<(), Axes3DError> {
        if points.is_empty() {
            return Ok(());
        }
        let [xs, ys, zs] = columns(points);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use plt_core::Point;

    #[test]
    fn lines_are_lifted_per_path() {
        let ax = Figure::new().subplot_3d();
        let segments = vec![
            vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)],
            vec![
                Point::new(2.0, 0.0),
                Point::new(f64::NAN, 1.0),
                Point::new(3.0, 1.0),
            ],
        ];
        ax.add_collection3d(
            CollectionSpec::lines(segments),
            &[-5.0, 5.0],
            Axis3::Z,
            true,
        )
        .unwrap();
        let (low, high) = ax.zlim();
        assert!(low < -5.0 && high > 5.0);
        let art = ax.with(|axes| axes.artists[0].art.clone());
        let Art3D::Segments(lines) = art else {
            panic!("a line collection");
        };
        assert_eq!(lines[1].len(), 2, "the NaN point is dropped");
    }

    #[test]
    fn mismatched_positions_and_other_collections_are_rejected() {
        let ax = Figure::new().subplot_3d();
        let poly = || CollectionSpec::polygons(vec![vec![Point::new(0.0, 0.0); 3]; 2]);
        assert!(
            ax.add_collection3d(poly(), &[1.0, 2.0, 3.0], Axis3::Z, true)
                .is_err()
        );
        assert_eq!(
            ax.add_collection3d(CollectionSpec::circles(vec![1.0]), &[0.0], Axis3::Z, true)
                .unwrap_err(),
            Axes3DError::UnsupportedCollection
        );
        let before = ax.zlim();
        ax.add_collection3d(poly(), &[7.0], Axis3::Z, false)
            .unwrap();
        assert_eq!(ax.zlim(), before, "without autolim the limits stay");
    }

    #[test]
    fn three_dimensional_collections_scale_to_every_point() {
        let ax = Figure::new().subplot_3d();
        let ragged = vec![
            vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]],
            vec![[1.0, 0.0, 1.0], [4.0, 3.0, 4.0]],
        ];
        ax.add_line_collection3d(ragged, CollectionProps::default(), true)
            .unwrap();
        // The x and y margins of 5 %, then 1/48 of the span on either side
        // (matplotlib gives -0.2916666666666667 and 4.291666666666667).
        let close = |(a, b): (f64, f64), (c, d): (f64, f64)| (a - c).abs() + (b - d).abs() < 1e-12;
        assert!(close(ax.xlim(), (-0.2 - 4.4 / 48.0, 4.2 + 4.4 / 48.0)));
        assert!(close(ax.ylim(), (-0.15 - 3.3 / 48.0, 3.15 + 3.3 / 48.0)));
        let ax = Figure::new().subplot_3d();
        let triangle = vec![vec![[0.0, 0.0, 0.0], [1.0, 1.0, 3.0], [1.0, 0.0, 4.0]]];
        let polys = ax
            .add_poly_collection3d(triangle, CollectionProps::default(), true)
            .unwrap();
        // No z margin: only the 1/48 of the span.
        assert!(close(ax.zlim(), (-4.0 / 48.0, 4.0 + 4.0 / 48.0)));
        assert_eq!(polys.colors().unwrap().faces.len(), 1);
    }
}
