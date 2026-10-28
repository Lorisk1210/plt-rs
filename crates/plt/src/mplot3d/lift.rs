//! Adding collections to a 3-D axes (matplotlib's `add_collection3d`):
//! 2-D collections lifted into planes (`poly_collection_2d_to_3d`,
//! `line_collection_2d_to_3d` and `patch_collection_2d_to_3d`), and
//! polylines and polygons through 3-D points (`Line3DCollection` and
//! `Poly3DCollection`).

use plt_core::Point;

use super::art3d::{Art3D, Points3D, Segments3D, ZSort};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::{broadcast, columns};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, CollectionSpec,
};
use crate::transforms::Transform;

/// Options of [`Axes3D::add_poly_collection3d_with`] (matplotlib's
/// `Poly3DCollection` keywords, and `add_collection3d`'s `autolim` and
/// `axlim_clip`).
#[derive(Clone, Debug)]
pub struct Poly3DOptions {
    /// Colours, widths and the other collection properties.
    pub props: CollectionProps,
    /// How the depth of a polygon is computed from its vertices.
    pub zsort: ZSort,
    /// Whether the outlines are closed; open polygons are filled but not
    /// stroked along their last edge (matplotlib's `closed`).
    pub closed: bool,
    /// Whether the data limits extend to every vertex.
    pub autolim: bool,
    /// Whether the vertices outside the view limits are hidden
    /// (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
}

impl Default for Poly3DOptions {
    fn default() -> Self {
        Self {
            props: CollectionProps::default(),
            zsort: ZSort::Average,
            closed: true,
            autolim: true,
            axlim_clip: false,
        }
    }
}

impl Axes3D {
    /// Places the paths of a 2-D polygon or line collection in the planes
    /// normal to `zdir` at `zs` (one position, or one per path) and adds
    /// it, extending the data limits to it with `autolim` (matplotlib's
    /// `add_collection3d`). Lifted polygons keep their path codes and are
    /// ordered among the other collections by the depth of the lowest
    /// position; lines lose their non-finite points. A patch collection
    /// keeps its patches in projected coordinates, as matplotlib leaves
    /// them: its offsets (the origin of the display when it has none, as
    /// matplotlib's identity offset transform) go to `zs` along `zdir`
    /// and are projected at every draw (matplotlib's `Patch3DCollection`,
    /// whose offsets [`Axes3D::set_scatter_offsets`] replaces), and the
    /// data limits stay as they are, `autolim` or not.
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
    /// When the collection is not a polygon, line or patch collection
    /// (matplotlib leaves the others in projected coordinates), or `zs`
    /// does not match the paths (the offsets of a patch collection).
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
        let sort_zpos = zs.iter().copied().reduce(f64::min).unwrap_or(0.0);
        match kind {
            CollectionKind::Patch => return self.lift_patches(artist, zs, zdir, sort_zpos),
            CollectionKind::Poly | CollectionKind::Line => {}
            CollectionKind::Generic | CollectionKind::Path | CollectionKind::QuadMesh => {
                return Err(Axes3DError::UnsupportedCollection);
            }
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
            self.register_polys(
                &raw,
                lifted,
                ZSort::Average,
                Some((paths, sort_zpos)),
                false,
            )?;
        } else {
            let segments = Segments3D {
                lines: lifted,
                sort_zpos: Some(sort_zpos),
            };
            self.register(raw.id(), Art3D::Segments(segments), false);
        }
        if autolim {
            self.scale_to(&points, had_data)?;
        }
        Ok(Collection::from_raw(raw))
    }

    /// Adds a patch collection whose offsets go to `zs` along `zdir`
    /// (matplotlib's `patch_collection_2d_to_3d` with `set_sort_zpos`);
    /// the offsets and `zs` broadcast as numpy arrays do.
    fn lift_patches(
        &self,
        mut artist: CollectionArtist,
        zs: &[f64],
        zdir: Axis3,
        sort_zpos: f64,
    ) -> Result<Collection, Axes3DError> {
        if artist.offset_transform.is_none() {
            artist.offsets = vec![Point::ORIGIN];
            artist.offset_transform = Some(Transform::Display);
        }
        let offsets = &artist.offsets;
        let n = match (offsets.len(), zs.len()) {
            (1, count) | (count, 1) => count,
            (count, z_count) => {
                check_len("zs", count, z_count)?;
                count
            }
        };
        let pick = |len: usize, i: usize| if len == 1 { 0 } else { i };
        let positions = (0..n)
            .map(|i| {
                let offset = offsets[pick(offsets.len(), i)];
                zdir.juggle([offset.x, offset.y, zs[pick(zs.len(), i)]])
            })
            .collect();
        let rc = crate::rc::params();
        let raw = self.adopt_projected(artist);
        let points = Points3D {
            positions,
            depthshade: rc.axes3d_depthshade,
            min_alpha: rc.axes3d_depthshade_minalpha,
            depth_sorted: false,
            sort_zpos: Some(sort_zpos),
        };
        self.register(raw.id(), Art3D::Points(points), false);
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
        let options = Poly3DOptions {
            props,
            autolim,
            ..Poly3DOptions::default()
        };
        self.add_poly_collection3d_with(polygons, options)
    }

    /// Adds polygons through 3-D vertices as `options` say (matplotlib's
    /// `add_collection3d` of a `Poly3DCollection` made with keywords),
    /// ordered among the other collections by the depth `options.zsort`
    /// picks.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Poly3DOptions;
    /// let ax = Figure::new().subplot_3d();
    /// let triangle = vec![vec![[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 0.0]]];
    /// let open = Poly3DOptions { closed: false, autolim: false, ..Poly3DOptions::default() };
    /// ax.add_poly_collection3d_with(triangle, open)?;
    /// assert_eq!(ax.zlim(), (-1.0 / 48.0, 49.0 / 48.0), "the limits stay");
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a colour cannot be resolved or autoscaling fails.
    pub fn add_poly_collection3d_with(
        &self,
        polygons: Vec<Vec<Vec3>>,
        options: Poly3DOptions,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let points: Vec<Vec3> = polygons.iter().flatten().copied().collect();
        let mut artist =
            CollectionArtist::new(CollectionKind::Poly, Vec::new(), crate::rc::params());
        options.props.apply(&mut artist);
        let raw = self.adopt_projected(artist);
        self.register_polys(&raw, polygons, options.zsort, None, options.axlim_clip)?;
        if !options.closed {
            self.with_mut(|axes| {
                let entry = axes.artists.iter_mut().find(|a| a.id == raw.id());
                if let Some(Art3D::Polys(polys)) = entry.map(|a| &mut a.art) {
                    polys.closed = false;
                }
            });
        }
        if options.autolim {
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
        let Art3D::Segments(Segments3D { lines, .. }) = art else {
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
        let patches = || {
            let square = crate::collections::CollectionPatch::new(
                &crate::patches::Rectangle::new((0.0, 0.0), 0.01, 0.01),
                crate::patches::PatchProps::default(),
            );
            CollectionSpec::patches(&[square], false).expect("valid colours")
        };
        assert!(
            ax.add_collection3d(patches(), &[1.0, 2.0], Axis3::Z, true)
                .is_ok(),
            "one offset broadcasts to the positions"
        );
        let placed = patches().with_offsets(vec![Point::new(0.0, 0.0); 3], Transform::Display);
        assert!(matches!(
            ax.add_collection3d(placed, &[1.0, 2.0], Axis3::Z, true),
            Err(Axes3DError::Shape { argument: "zs", .. })
        ));
        let before = ax.zlim();
        ax.add_collection3d(poly(), &[7.0], Axis3::Z, false)
            .unwrap();
        assert_eq!(ax.zlim(), before, "without autolim the limits stay");
    }

    #[test]
    fn patch_collections_place_their_offsets_and_keep_the_limits() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let before = ax.zlim();
        let circle = crate::collections::CollectionPatch::new(
            &crate::patches::Circle::new((0.0, 0.0), 0.01),
            crate::patches::PatchProps::default(),
        );
        let spec = CollectionSpec::patches(&[circle], false)
            .expect("valid colours")
            .with_offsets(
                vec![Point::new(0.2, 0.3), Point::new(0.6, 0.7)],
                Transform::Display,
            );
        let patches = ax
            .add_collection3d(spec, &[0.1, 0.9], Axis3::Y, true)
            .unwrap();
        assert_eq!(ax.zlim(), before, "matplotlib does not autoscale them");
        assert_eq!(
            ax.scatter_offsets(&patches).unwrap(),
            [vec![0.2, 0.6], vec![0.1, 0.9], vec![0.3, 0.7]]
        );
        assert_eq!(ax.depthshade(&patches), Ok(true), "axes3d.depthshade");
        assert_eq!(ax.sort_zpos(&patches), Ok(Some(0.1)));
        fig.layout();
        let offsets = patches.offsets();
        assert_eq!(offsets.len(), 2, "kept in order and projected");
        assert!(offsets.iter().all(|p| p.x.abs() < 0.2 && p.y.abs() < 0.2));
    }

    #[test]
    fn alpha_fades_the_edges_of_lifted_polygons_only() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let props = || {
            CollectionProps::default()
                .facecolor("b")
                .edgecolor("k")
                .alpha(0.5)
        };
        let triangle = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let made = ax
            .add_poly_collection3d(vec![triangle], props(), true)
            .unwrap();
        let flat = vec![vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        ]];
        let spec = CollectionSpec::polygons(flat).with_props(props());
        let lifted = ax.add_collection3d(spec, &[0.0], Axis3::Z, true).unwrap();
        fig.layout();
        let alphas = |c: &Collection| {
            let colors = c.colors().unwrap();
            (colors.faces[0].a, colors.edges[0].a)
        };
        // matplotlib's `Poly3DCollection.set_alpha` leaves `_edgecolor3d`.
        assert_eq!(alphas(&made), (0.5, 1.0));
        assert_eq!(alphas(&lifted), (0.5, 0.5));
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
