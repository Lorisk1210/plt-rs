//! What a [`Collection`] reads back besides the style it was given: its
//! segments, data limits, line widths and dash patterns as drawn, the
//! vertex grid of a mesh and the values under a point (matplotlib's
//! `get_segments`, `get_datalim`, `get_linewidth`, `get_linestyle`,
//! `get_coordinates` and `get_cursor_data`).

use pltrs_core::path::DataLimits;
use pltrs_core::{Bbox, Point};
use pltrs_render::Dashes;

use super::style::{broadcast_widths_and_dashes, unfilled};
use super::{Collection, CollectionArtist};
use crate::artist::{ArtistHandle, LayoutContext};
use crate::numeric::Grid;

impl Collection {
    /// Whether the faces are coloured (matplotlib's `get_fill`): not when
    /// they are `'none'`, as the faces of line collections are unless set.
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::colors::Color;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let segment = vec![Point::new(0.0, 0.0), Point::new(1.0, 2.0)];
    /// let lines = ax.add_collection(CollectionSpec::lines(vec![segment.clone()]));
    /// assert!(!lines.fill());
    /// let polygons = ax.add_collection(CollectionSpec::polygons(vec![segment]));
    /// assert!(polygons.fill());
    /// polygons.set_facecolors(vec![Color::none()]);
    /// assert!(!polygons.fill());
    /// ```
    pub fn fill(&self) -> bool {
        self.raw()
            .with::<CollectionArtist, _>(|c| !unfilled(&c.face_spec(&c.colors_rc())))
    }

    /// The rotation of the items scaled to their sizes, in radians
    /// (matplotlib's `RegularPolyCollection.get_rotation`; 0 for the
    /// other collections).
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let polygons = ax.add_collection(CollectionSpec::regular_polygons(5, 0.25, vec![10.0]));
    /// assert_eq!(polygons.size_rotation(), 0.25);
    /// ```
    pub fn size_rotation(&self) -> f64 {
        self.raw().with::<CollectionArtist, _>(|c| c.size_rotation)
    }

    /// The points of every item (matplotlib's `LineCollection.get_segments`).
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let segment = vec![Point::new(0.0, 0.0), Point::new(1.0, 2.0)];
    /// let lines = ax.add_collection(CollectionSpec::lines(vec![segment.clone()]));
    /// assert_eq!(lines.segments(), [segment]);
    /// ```
    pub fn segments(&self) -> Vec<Vec<Point>> {
        self.raw().with::<CollectionArtist, _>(|c| {
            c.shapes()
                .iter()
                .map(|path| path.vertices().to_vec())
                .collect()
        })
    }

    /// The box the collection adds to its axes' data limits (matplotlib's
    /// `get_datalim(ax.transData)`): its shapes placed at their offsets
    /// when the shapes are in data coordinates, else its offsets when
    /// those are; the null box when it has no shapes, when its offsets
    /// are in other coordinates, or when it is in no axes.
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let lines = ax.add_collection(CollectionSpec::lines(vec![vec![Point::new(1.0, 2.0), Point::new(3.0, 0.0)]]));
    /// let limits = lines.datalim();
    /// assert_eq!((limits.x0(), limits.y0(), limits.x1(), limits.y1()), (1.0, 0.0, 3.0, 2.0));
    /// ```
    pub fn datalim(&self) -> Bbox {
        self.data_limits().bbox
    }

    /// [`Collection::datalim`] with the smallest positive coordinates of
    /// the collection, which logarithmic axes autoscale from (the
    /// `minpos` of matplotlib's `get_datalim` box).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![-1.0, 0.5, 2.0], vec![3.0, -2.0, 4.0])?;
    /// let limits = dots.data_limits();
    /// assert_eq!((limits.minpos.x, limits.minpos.y), (0.5, 3.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn data_limits(&self) -> DataLimits {
        let Some(axes) = self.raw().with_base(|base| base.parent) else {
            return DataLimits::NULL;
        };
        let state = self.raw().figure_state().borrow();
        state
            .store
            .get_as::<CollectionArtist>(self.raw().id())
            .map_or(DataLimits::NULL, |c| c.data_limits(axes, &*state).0)
    }

    /// The edge widths as drawn (matplotlib's `get_linewidth`): the widths
    /// set (or the kind's default) cycled with the line styles to the
    /// least common multiple of their lengths.
    pub fn broadcast_linewidths(&self) -> Vec<f64> {
        self.broadcast().0
    }

    /// The dash pattern of every edge as drawn (matplotlib's
    /// `get_linestyle`): the line styles cycled with the widths as in
    /// [`Collection::broadcast_linewidths`], each scaled by its width when
    /// `lines.scale_dashes` is on (solid lines have an empty pattern).
    pub fn dash_patterns(&self) -> Vec<Dashes> {
        self.broadcast().1
    }

    fn broadcast(&self) -> (Vec<f64>, Vec<Dashes>) {
        self.raw()
            .with::<CollectionArtist, _>(CollectionArtist::drawn_widths_and_dashes)
    }

    /// The vertex grid of a quad mesh, drawn as a mesh or as polygons, one
    /// row of points per row of vertices (matplotlib's
    /// `QuadMesh.get_coordinates` and `PolyQuadMesh.get_coordinates`);
    /// `None` for other collections.
    pub fn mesh_coordinates(&self) -> Option<Grid<Point>> {
        self.raw().with::<CollectionArtist, _>(|c| {
            c.mesh
                .as_ref()
                .map(|m| m.coordinates.clone())
                .or_else(|| c.polygon_grid.clone())
        })
    }

    /// The `(rows, columns)` of the vertex grid of
    /// [`mesh_coordinates`](Self::mesh_coordinates), without copying it;
    /// `None` for other collections.
    pub fn mesh_shape(&self) -> Option<(usize, usize)> {
        self.raw().with::<CollectionArtist, _>(|c| {
            c.mesh
                .as_ref()
                .map(|m| m.coordinates.shape())
                .or_else(|| c.polygon_grid.as_ref().map(Grid::shape))
        })
    }

    /// The values mapped to colours of the cells of a quad mesh under
    /// display point `at`, in the figure's current layout (matplotlib's
    /// `QuadMesh.get_cursor_data`): `None` when no cell is hit, the mesh
    /// maps no values or the collection is no mesh; masked values are
    /// `None`.
    pub fn cursor_data(&self, at: Point) -> Option<Vec<Option<f64>>> {
        let state = self.raw().figure_state().prepared();
        let layout = LayoutContext::new(&state);
        layout
            .store()
            .get_as::<CollectionArtist>(self.raw().id())?
            .cursor_values(&layout, at)
    }
}

impl CollectionArtist {
    /// The line widths and the dash patterns scaled by them, as drawn
    /// (matplotlib's `get_linewidth` and `get_linestyle`).
    pub(crate) fn drawn_widths_and_dashes(&self) -> (Vec<f64>, Vec<Dashes>) {
        broadcast_widths_and_dashes(&self.linewidths_or_default(), &self.linestyles, self.rc())
    }

    /// The mapped values of the mesh cells under `at`.
    pub(super) fn cursor_values(
        &self,
        ctx: &LayoutContext<'_>,
        at: Point,
    ) -> Option<Vec<Option<f64>>> {
        self.mesh.as_ref()?;
        let values = &self.mapping.as_ref()?.source.values;
        let cells = crate::artist::Artist::pick_hit(self, ctx, at, None)?;
        Some(
            cells
                .into_iter()
                .map(|i| values.get(i).copied().flatten())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use crate::axes::{Mesh, PcolorOptions};
    use crate::collections::CollectionSpec;
    use crate::figure::Figure;
    use crate::lines::LineStyle;
    use crate::numeric::Grid;

    #[test]
    fn widths_and_dashes_broadcast_together() {
        let ax = Figure::new().subplot();
        let paths = ax.add_collection(CollectionSpec::paths(Vec::new()));
        paths
            .set_linestyles(vec![LineStyle::Solid; 2])
            .set_linewidths(vec![1.0, 2.0, 3.0]);
        assert_eq!(paths.broadcast_linewidths(), [1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
        assert_eq!(paths.dash_patterns().len(), 6);
        assert!(
            paths
                .dash_patterns()
                .iter()
                .all(|d| d.pattern.as_deref().is_none_or(<[f64]>::is_empty))
        );
        paths.set_linestyles(vec![LineStyle::Solid; 3]);
        assert_eq!(paths.broadcast_linewidths(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn empty_collections_add_no_limits() {
        let ax = Figure::new().subplot();
        let empty = ax.add_collection(CollectionSpec::paths(Vec::new()));
        assert_eq!(empty.datalim(), pltrs_core::Bbox::NULL);
    }

    #[test]
    fn meshes_give_their_grid_and_the_values_under_a_point() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let values = Grid::from_fn(3, 3, |r, c| (r * c) as f64);
        let mesh = ax
            .pcolormesh(Mesh::new(values), PcolorOptions::default())
            .unwrap();
        let grid = mesh.mesh_coordinates().expect("a mesh");
        assert_eq!(grid.shape(), (4, 4));
        assert_eq!(mesh.mesh_shape(), Some((4, 4)));
        assert_eq!(grid[(1, 2)], Point::new(2.0, 1.0));
        let centre = fig
            .resolve_transform(&ax.trans_data())
            .unwrap()
            .transform_point(Point::new(1.5, 1.5));
        assert_eq!(mesh.cursor_data(centre), Some(vec![Some(1.0)]));
        mesh.clear_array();
        assert_eq!(mesh.cursor_data(centre), None);
        let lines = ax.add_collection(CollectionSpec::lines(Vec::new()));
        assert_eq!(
            (lines.mesh_coordinates(), lines.cursor_data(centre)),
            (None, None)
        );
        assert_eq!(lines.mesh_shape(), None);
    }
}
