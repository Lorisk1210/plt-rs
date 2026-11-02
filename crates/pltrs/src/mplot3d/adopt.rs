//! Adding to a 3-D axes collections made outside any figure (matplotlib's
//! `Poly3DCollection` and `Line3DCollection` made with their properties
//! before `add_collection3d` adds them).

use std::rc::Rc;

use super::art3d::{Art3D, Path3D, Points3D, Segments3D, ZSort};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::lift::patch_positions;
use super::math::Vec3;
use crate::artist::{ArtistHandle, ArtistLookup, RawHandle};
use crate::axes::AddAs;
use crate::collections::{Collection, CollectionArtist};

/// The 3-D geometry a collection made outside any figure is added with.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry3D {
    /// Polygons through 3-D vertices (matplotlib's `Poly3DCollection`).
    Polygons {
        /// The vertices of each polygon.
        faces: Vec<Vec<Vec3>>,
        /// How the depth of a polygon is computed from its vertices.
        zsort: ZSort,
        /// Whether the outlines are closed; open polygons are not stroked
        /// along their last edge.
        closed: bool,
        /// Whether the polygons are those of a 2-D collection lifted into
        /// 3-D (matplotlib's `poly_collection_2d_to_3d`), whose alpha has
        /// faded its edges, rather than made as 3-D polygons.
        from_2d: bool,
    },
    /// Polylines through 3-D points (matplotlib's `Line3DCollection`).
    Lines(Vec<Vec<Vec3>>),
    /// The collection's own paths, each vertex placed at a 3-D point, one
    /// list of points per path (matplotlib's `Collection3D`, which
    /// `collection_2d_to_3d` makes).
    Paths(Vec<Vec<Vec3>>),
    /// Patches drawn at their offsets placed in 3-D, which leave the
    /// data limits alone (matplotlib's `Patch3DCollection`).
    Patches {
        /// The positions of the offsets along `zdir`: one for all or one
        /// per offset.
        zs: Vec<f64>,
        /// The axis the positions lie along.
        zdir: Axis3,
        /// Whether the patches fade with depth.
        depthshade: bool,
        /// The alpha factor of the deepest patches.
        min_alpha: f64,
    },
}

impl Geometry3D {
    /// Every vertex.
    fn points(&self) -> Vec<Vec3> {
        match self {
            Self::Polygons { faces, .. } => faces.iter().flatten().copied().collect(),
            Self::Lines(lines) | Self::Paths(lines) => lines.iter().flatten().copied().collect(),
            Self::Patches { .. } => Vec::new(),
        }
    }
}

impl Axes3D {
    /// Adds the collection `collection`, made outside any figure (a loose
    /// polygon or line collection, its colours and widths set), with the
    /// 3-D geometry `geometry`, projected at every draw and ordered by
    /// depth among the other collections; with `autolim` the data limits
    /// extend to every vertex, and `axlim_clip` hides the vertices outside
    /// the view limits. Returns the collection's handle in the figure.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::collections::{Collection, CollectionSpec};
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Geometry3D;
    /// let ax = Figure::new().subplot_3d();
    /// let loose = Collection::loose(CollectionSpec::lines(Vec::new()));
    /// let lines = Geometry3D::Lines(vec![vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]]);
    /// ax.adopt_collection3d(loose.raw(), lines, true, false)?;
    /// assert!(ax.zlim().1 > 3.0);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the collection cannot join the axes (it belongs to another
    /// figure, or is in one already), a colour cannot be resolved or
    /// autoscaling fails.
    pub fn adopt_collection3d(
        &self,
        collection: &RawHandle,
        geometry: Geometry3D,
        autolim: bool,
        axlim_clip: bool,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let points = geometry.points();
        // matplotlib leaves the limits alone for patch collections.
        let scales = !matches!(geometry, Geometry3D::Patches { .. });
        let raw = self
            .plane()
            .adopt_artist(collection, AddAs::Collection { autolim: false })?;
        match geometry {
            Geometry3D::Polygons {
                faces,
                zsort,
                closed,
                from_2d,
            } => {
                self.register_polys(&raw, faces, zsort, None, from_2d, axlim_clip)?;
                if !closed {
                    self.open_polygons(&raw);
                }
            }
            Geometry3D::Lines(lines) => {
                self.register(
                    raw.id(),
                    Art3D::Segments(Segments3D::new(lines)),
                    axlim_clip,
                );
            }
            Geometry3D::Paths(vertices) => {
                let paths = paths_3d(&raw, vertices)?;
                self.register(raw.id(), Art3D::Paths(paths), axlim_clip);
            }
            Geometry3D::Patches {
                zs,
                zdir,
                depthshade,
                min_alpha,
            } => {
                let positions =
                    raw.with_mut::<CollectionArtist, _>(|c| patch_positions(c, &zs, zdir))?;
                let points = Points3D {
                    positions,
                    depthshade,
                    min_alpha,
                    depth_sorted: false,
                    sort_zpos: None,
                };
                self.register(raw.id(), Art3D::Points(points), axlim_clip);
            }
        }
        if autolim && scales {
            self.scale_to(&points, had_data)?;
        }
        Ok(Collection::from_raw(raw))
    }
}

impl Axes3D {
    /// Places the paths of `collection`, a collection of the projection
    /// plane of these axes, in 3-D: every vertex at the point of
    /// `vertices` for it, one list of points per path (matplotlib's
    /// `collection_2d_to_3d` of a collection in an axes), projected at
    /// every draw; `axlim_clip` hides the points outside the view limits.
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::figure::Figure;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot_3d();
    /// let line = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
    /// let lines = ax.projection_plane().add_collection(CollectionSpec::lines(vec![line]));
    /// ax.lift_paths(&lines, vec![vec![[0.0, 0.0, 2.0], [1.0, 1.0, 2.0]]], false)?;
    /// assert!(ax.do_3d_projection(&lines)?.is_finite());
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `collection` is not on the projection plane of these axes, or
    /// `vertices` does not have a point per vertex of its paths.
    pub fn lift_paths(
        &self,
        collection: &Collection,
        vertices: Vec<Vec<Vec3>>,
        axlim_clip: bool,
    ) -> Result<(), Axes3DError> {
        let raw = collection.raw();
        let plane = self.plane().id();
        let on_plane = Rc::ptr_eq(self.raw().figure_state(), raw.figure_state())
            && (raw.figure_state().borrow().artist(raw.id()))
                .is_some_and(|artist| artist.base().parent == Some(plane));
        if !on_plane {
            return Err(Axes3DError::ForeignArtist);
        }
        let paths = paths_3d(raw, vertices)?;
        self.register(raw.id(), Art3D::Paths(paths), axlim_clip);
        Ok(())
    }
}

/// The paths of the collection `raw` with their vertices at the points of
/// `vertices`, one list per path.
fn paths_3d(raw: &RawHandle, vertices: Vec<Vec<Vec3>>) -> Result<Vec<Path3D>, Axes3DError> {
    if !raw.is::<CollectionArtist>() {
        return Err(Axes3DError::UnsupportedCollection);
    }
    let paths = raw.with::<CollectionArtist, _>(|c| c.paths.clone());
    check_len("vertices", paths.len(), vertices.len())?;
    paths
        .into_iter()
        .zip(vertices)
        .map(|(path, points)| {
            check_len("path vertices", path.len(), points.len())?;
            Ok(Path3D::new(path, points))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::collections::CollectionSpec;
    use crate::figure::Figure;

    #[test]
    fn adopted_polygons_keep_their_properties_and_extend_the_limits() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let loose = Collection::loose(CollectionSpec::polygons(Vec::new()));
        loose.set_alpha(Some(0.5));
        let faces = vec![vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 4.0, 2.0]]];
        let geometry = Geometry3D::Polygons {
            faces,
            zsort: ZSort::Average,
            closed: false,
            from_2d: false,
        };
        let adopted = ax
            .adopt_collection3d(loose.raw(), geometry, true, false)
            .unwrap();
        assert_eq!(adopted.alpha(), Some(0.5));
        assert!(ax.xlim().1 > 4.0);
        let patches = Collection::loose(CollectionSpec::patches(&[], false).unwrap());
        let before = ax.xlim();
        let offsets = Geometry3D::Patches {
            zs: vec![5.0],
            zdir: Axis3::Z,
            depthshade: false,
            min_alpha: 0.3,
        };
        let lifted = ax
            .adopt_collection3d(patches.raw(), offsets, true, false)
            .unwrap();
        assert_eq!(ax.xlim(), before, "patches leave the limits alone");
        assert_eq!(ax.scatter_offsets(&lifted).unwrap()[2], [5.0]);
        assert!(!ax.depthshade(&lifted).unwrap());
        let again = Geometry3D::Lines(Vec::new());
        assert!(
            ax.adopt_collection3d(adopted.raw(), again, false, false)
                .is_err()
        );
    }
}
