//! Lifting a 2-D collection into a 3-D axes (matplotlib's
//! `add_collection3d` with `poly_collection_2d_to_3d` and
//! `line_collection_2d_to_3d`).

use super::art3d::{Art3D, ZSort};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::{broadcast, columns};
use crate::collections::{Collection, CollectionKind, CollectionSpec};

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
        if autolim && !points.is_empty() {
            let [xs, ys, zs] = columns(&points);
            self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        }
        Ok(Collection::from_raw(raw))
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
}
