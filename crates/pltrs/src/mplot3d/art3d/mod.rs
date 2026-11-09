//! The 3-D geometry of the projection plane's artists and its projection
//! onto them (matplotlib's `art3d`: `Line3D`, `Text3D`, `Collection3D`,
//! `Line3DCollection`, `Path3DCollection`, `Poly3DCollection`, `Patch3D`
//! and `PathPatch3D`).
//!
//! matplotlib turns 2-D artists into 3-D ones by changing their class;
//! here a 2-D artist stays what it is and its 3-D geometry is kept next to
//! it. Projecting writes the projected geometry (and, for depth-sorted
//! collections, the reordered colours and sizes) into the 2-D artist.
//!
//! An artist clipped to the view limits (matplotlib 3.10's `axlim_clip`)
//! is projected by a projection hiding the points outside them: they
//! become NaN, which breaks lines, drops markers and texts, and leaves
//! polygons with gaps, the way matplotlib's masked points do.

mod points;
mod polys;

pub(crate) use points::Points3D;
pub(crate) use polys::Polys3D;
pub use polys::ZSort;

use pltrs_core::{Path, Point};

use super::camera::{Projection, norm_text_angle};
use super::coords::Axis3;
use super::math::{Vec3, add};
use super::units::Sources3D;
use crate::artist::{ArtistId, ArtistStore};
use crate::collections::{CollectionArtist, CollectionKind};
use crate::lines::LineArtist;
use crate::patches::{PatchArtist, PatchShape};
use crate::text::{AngleReduction, TextArtist};

/// The outline of a 3-D patch as projected by the last layout pass
/// (matplotlib's `Patch3D`, which unlike a `PathPatch` has no edge by
/// default).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectedOutline(pub Path);

pltrs_core::archive_struct!(ProjectedOutline(v0));

impl PatchShape for ProjectedOutline {
    fn path(&self) -> Path {
        self.0.clone()
    }
}

/// The path of a 3-D path patch as projected by the last layout pass,
/// its codes kept (matplotlib's `PathPatch3D`, which, like a `PathPatch`,
/// draws an edge by default).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectedPath(pub Path);

pltrs_core::archive_struct!(ProjectedPath(v0));

impl PatchShape for ProjectedPath {
    const EDGE_DEFAULT: bool = true;

    fn path(&self) -> Path {
        self.0.clone()
    }
}

/// How computed z-order counts an artist (matplotlib numbers collections
/// and patches separately).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SortKind {
    /// A collection.
    Collection,
    /// A patch.
    Patch,
}

pltrs_core::archive_enum!(SortKind { Collection, Patch });

/// A path whose vertices live in 3-D space; its codes are kept by a 2-D
/// template path.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path3D {
    /// The 2-D path the projected vertices are written into.
    template: Path,
    /// One 3-D point per template vertex.
    pub(super) vertices: Vec<Vec3>,
}

pltrs_core::archive_struct!(Path3D { template, vertices });

impl Path3D {
    /// `template` with its vertices replaced by `vertices`.
    pub(crate) fn new(template: Path, vertices: Vec<Vec3>) -> Self {
        debug_assert_eq!(template.len(), vertices.len());
        Self { template, vertices }
    }
}

/// A 2-D artist of the projection plane with its 3-D geometry.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Artist3D {
    /// The 2-D artist the geometry is projected onto.
    pub(crate) id: ArtistId,
    /// The geometry.
    pub(crate) art: Art3D,
    /// Whether the points outside the view limits are hidden
    /// (matplotlib's `axlim_clip`).
    pub(crate) axlim_clip: bool,
    /// The user values the geometry came from, converted again when an
    /// axis changes unit.
    pub(crate) sources: Sources3D,
    /// Whether a draw of the axes projected it since it was added: the
    /// 2-D data of a matplotlib `Line3D` are the projection of its last
    /// draw from its first draw on.
    pub(crate) drawn: bool,
}

pltrs_core::archive_struct!(Artist3D {
    id,
    art,
    axlim_clip,
    sources,
    drawn
});

/// The polylines of a line collection (matplotlib's `Line3DCollection`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Segments3D {
    /// The polylines, possibly of different lengths.
    pub(crate) lines: Vec<Vec<Vec3>>,
    /// The z position given with `set_sort_zpos`, which matplotlib's line
    /// collections keep without ordering by it (`_sort_zpos`).
    pub(crate) sort_zpos: Option<f64>,
}

pltrs_core::archive_struct!(Segments3D { lines, sort_zpos });

impl Segments3D {
    /// `lines` without a sort position.
    pub(crate) fn new(lines: Vec<Vec<Vec3>>) -> Self {
        Self {
            lines,
            sort_zpos: None,
        }
    }
}

/// The 3-D geometry of one artist of the projection plane.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Art3D {
    /// The points of a line (`Line3D`).
    Line(Vec<Vec3>),
    /// A text anchored at `position` and aligned with `direction`
    /// (`Text3D`; a zero direction keeps the text horizontal).
    Text {
        /// The anchor.
        position: Vec3,
        /// The direction the baseline follows.
        direction: Vec3,
    },
    /// The paths of a converted collection (`Collection3D`).
    Paths(Vec<Path3D>),
    /// The polylines of a line collection (`Line3DCollection`).
    Segments(Segments3D),
    /// Scatter markers (`Path3DCollection`).
    Points(Points3D),
    /// Polygons (`Poly3DCollection`).
    Polys(Polys3D),
    /// The outline of a flat patch (`Patch3D`).
    Patch(Vec<Vec3>),
    /// The path of a patch, codes kept (`PathPatch3D`).
    PathPatch(Path3D),
    /// A 2-D patch or collection of a class of the program's that projects
    /// itself (matplotlib's `do_3d_projection` of a subclass), at the
    /// depth it last returned.
    Program {
        /// How computed z-order counts it.
        kind: SortKind,
        /// The depth its projection last returned (NaN before the first).
        depth: f64,
    },
}

pltrs_core::archive_enum!(Art3D {
    Line(v0),
    Text { position, direction },
    Paths(v0),
    Segments(v0),
    Points(v0),
    Polys(v0),
    Patch(v0),
    PathPatch(v0),
    Program { kind, depth },
});

impl Art3D {
    /// matplotlib's class of a collection of `kind` with this geometry,
    /// which names its SVG group; `None` for the geometries of other
    /// artists, and for converted collections, whose class matplotlib
    /// derives from the 2-D one (`Collection3D`).
    pub(crate) fn collection_class(&self, kind: CollectionKind) -> Option<&'static str> {
        match self {
            Self::Segments(_) => Some("Line3DCollection"),
            Self::Polys(_) => Some("Poly3DCollection"),
            Self::Points(_) if kind == CollectionKind::Patch => Some("Patch3DCollection"),
            Self::Points(_) => Some("Path3DCollection"),
            Self::Line(_)
            | Self::Text { .. }
            | Self::Paths(_)
            | Self::Patch(_)
            | Self::PathPatch(_)
            | Self::Program { .. } => None,
        }
    }

    /// Whether the artist counts as data (matplotlib's `has_data`).
    pub(crate) fn is_data(&self) -> bool {
        !matches!(self, Self::Text { .. })
    }

    /// Whether matplotlib projects the artist as it draws it (a line,
    /// `Line3D.draw`), not before drawing the axes' artists
    /// (`do_3d_projection`): an artist the draw leaves out is not
    /// projected.
    pub(crate) fn projected_when_drawn(&self) -> bool {
        matches!(self, Self::Line(_))
    }

    /// How computed z-order counts the artist; lines and texts keep
    /// their own z-order.
    pub(crate) fn sort_kind(&self) -> Option<SortKind> {
        match self {
            Self::Line(_) | Self::Text { .. } => None,
            Self::Paths(_) | Self::Segments(_) | Self::Points(_) | Self::Polys(_) => {
                Some(SortKind::Collection)
            }
            Self::Patch(_) | Self::PathPatch(_) => Some(SortKind::Patch),
            Self::Program { kind, .. } => Some(*kind),
        }
    }

    /// Every 3-D point of the geometry of a data artist, which the data
    /// limits include (none for texts).
    pub(crate) fn points(&self) -> Vec<Vec3> {
        match self {
            Self::Line(points) | Self::Patch(points) => points.clone(),
            Self::Text { .. } | Self::Program { .. } => Vec::new(),
            Self::Paths(paths) => (paths.iter())
                .flat_map(|path| path.vertices.iter().copied())
                .collect(),
            Self::Segments(segments) => segments.lines.concat(),
            Self::Points(points) => points.positions.clone(),
            Self::Polys(polys) => polys.faces.concat(),
            Self::PathPatch(path) => path.vertices.clone(),
        }
    }

    /// Takes `numbers`, the user values along `dir` the geometry came
    /// from converted again: one per point of a line (or one for all),
    /// one for the anchor of a text.
    pub(crate) fn reconvert(&mut self, dir: Axis3, numbers: &[f64]) {
        let i = dir.index();
        match self {
            Self::Line(points) => {
                let single = numbers.len() == 1;
                for (j, point) in points.iter_mut().enumerate() {
                    if let Some(&number) = numbers.get(if single { 0 } else { j }) {
                        point[i] = number;
                    }
                }
            }
            Self::Text { position, .. } => {
                if let Some(&number) = numbers.first() {
                    position[i] = number;
                }
            }
            Self::Paths(_)
            | Self::Segments(_)
            | Self::Points(_)
            | Self::Polys(_)
            | Self::Patch(_)
            | Self::PathPatch(_)
            | Self::Program { .. } => {}
        }
    }

    /// The sort position of a collection that takes one (matplotlib's
    /// `_sort_zpos` of line, path, patch and polygon collections).
    pub(crate) fn sort_zpos(&self) -> Option<&Option<f64>> {
        match self {
            Self::Segments(segments) => Some(&segments.sort_zpos),
            Self::Points(points) => Some(&points.sort_zpos),
            Self::Polys(polys) => Some(&polys.sort_zpos),
            Self::Line(_)
            | Self::Text { .. }
            | Self::Paths(_)
            | Self::Patch(_)
            | Self::PathPatch(_)
            | Self::Program { .. } => None,
        }
    }

    /// The same, to be set (matplotlib's `set_sort_zpos`).
    pub(crate) fn sort_zpos_mut(&mut self) -> Option<&mut Option<f64>> {
        match self {
            Self::Segments(segments) => Some(&mut segments.sort_zpos),
            Self::Points(points) => Some(&mut points.sort_zpos),
            Self::Polys(polys) => Some(&mut polys.sort_zpos),
            Self::Line(_)
            | Self::Text { .. }
            | Self::Paths(_)
            | Self::Patch(_)
            | Self::PathPatch(_)
            | Self::Program { .. } => None,
        }
    }

    /// Projects the geometry onto the 2-D artist `id` and returns the
    /// depth used for computed z-order (matplotlib's `do_3d_projection`);
    /// `None` when the artist is gone or of another type. An artist of
    /// the program's projects itself: its last depth.
    pub(crate) fn project(
        &self,
        id: ArtistId,
        store: &mut ArtistStore,
        projection: &Projection,
    ) -> Option<f64> {
        match self {
            Self::Line(points) => {
                let line = store.get_as_mut::<LineArtist>(id)?;
                (line.xdata, line.ydata) = (projection.project_clipped_all(points).into_iter())
                    .map(|q| q.map_or((f64::NAN, f64::NAN), |q| (q[0], q[1])))
                    .unzip();
                line.recache();
                Some(f64::NAN)
            }
            Self::Text {
                position,
                direction,
            } => {
                let text = store.get_as_mut::<TextArtist>(id)?;
                let [anchor, end] = [*position, add(*position, *direction)];
                let projected = projection.project_all(&[anchor, end]);
                let (anchor, end) = (projected[0], projected[1]);
                let angle = (end[1] - anchor[1]).atan2(end[0] - anchor[0]).to_degrees();
                text.position = Point::new(anchor[0], anchor[1]);
                text.rotation = norm_text_angle(angle);
                text.rotation_reduction = AngleReduction::AsIs;
                Some(f64::NAN)
            }
            Self::Paths(paths) => {
                let collection = store.get_as_mut::<CollectionArtist>(id)?;
                let mut depth = Depth::default();
                collection.paths = paths
                    .iter()
                    .map(|path| {
                        let mut projected = path.template.clone();
                        let points = projection.project_hidden_too_all(&path.vertices);
                        for ((vertex, &p), q) in projected
                            .vertices_mut()
                            .iter_mut()
                            .zip(&path.vertices)
                            .zip(points)
                        {
                            // matplotlib gathers the depths with
                            // `np.concatenate`, which drops the mask:
                            // hidden vertices still count.
                            let q = depth.include(q);
                            *vertex = if projection.hides(p) {
                                Point::new(f64::NAN, f64::NAN)
                            } else {
                                q
                            };
                        }
                        projected
                    })
                    .collect();
                Some(depth.min_or(1e9))
            }
            Self::Segments(Segments3D { lines, .. }) => {
                let collection = store.get_as_mut::<CollectionArtist>(id)?;
                let mut depth = Depth::default();
                // matplotlib projects the points of every segment at once.
                let mut points = projection.project_all(&lines.concat()).into_iter();
                collection.paths = (lines.iter())
                    .map(|segment| {
                        let projected = points.by_ref().take(segment.len());
                        Path::polyline(projected.map(|q| depth.include(q)).collect())
                    })
                    .collect();
                Some(depth.min_or(f64::NAN).min(1e9))
            }
            Self::Points(points) => {
                let collection = store.get_as_mut::<CollectionArtist>(id)?;
                Some(points.project(collection, projection))
            }
            Self::Polys(polys) => {
                let collection = store.get_as_mut::<CollectionArtist>(id)?;
                Some(polys.project(collection, projection))
            }
            Self::Patch(outline) => {
                let patch = store.get_as_mut::<PatchArtist<ProjectedOutline>>(id)?;
                let mut depth = Depth::default();
                let points = (projection.project_clipped_all(outline).into_iter())
                    .map(|q| q.map_or(Point::new(f64::NAN, f64::NAN), |q| depth.include(q)))
                    .collect();
                patch.shape = ProjectedOutline(Path::polyline(points));
                Some(depth.min_or(f64::NAN))
            }
            Self::PathPatch(path) => {
                let patch = store.get_as_mut::<PatchArtist<ProjectedPath>>(id)?;
                let mut depth = Depth::default();
                let mut projected = path.template.clone();
                let points = projection.project_clipped_all(&path.vertices);
                for (vertex, q) in projected.vertices_mut().iter_mut().zip(points) {
                    *vertex = q.map_or(Point::new(f64::NAN, f64::NAN), |q| depth.include(q));
                }
                patch.shape = ProjectedPath(projected);
                Some(depth.min_or(f64::NAN))
            }
            Self::Program { depth, .. } => store.contains(id).then_some(*depth),
        }
    }
}

/// The smallest projected depth seen while projecting.
#[derive(Clone, Copy, Debug)]
struct Depth {
    min: f64,
    any: bool,
}

impl Default for Depth {
    fn default() -> Self {
        Self {
            min: f64::INFINITY,
            any: false,
        }
    }
}

impl Depth {
    /// Records the depth of a projected point and returns its 2-D part.
    fn include(&mut self, q: Vec3) -> Point {
        self.min = self.min.min(q[2]);
        self.any = true;
        Point::new(q[0], q[1])
    }

    /// The smallest depth, or `empty` when nothing was projected.
    fn min_or(self, empty: f64) -> f64 {
        if self.any { self.min } else { empty }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::CollectionKind;
    use crate::mplot3d::camera::{Camera, ProjType};

    /// An orthographic projection of the unit cube.
    fn unit_projection() -> Projection {
        Camera::new(ProjType::Ortho)
            .expect("valid camera")
            .projection([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
    }

    /// The paths `art` projects into a collection when the points outside
    /// the unit cube are hidden, and the depth it reports.
    fn project_hidden(art: &Art3D) -> (Vec<Path>, f64) {
        let projection = unit_projection().hiding_outside([(0.0, 1.0); 3]);
        let mut store = ArtistStore::default();
        let collection =
            CollectionArtist::new(CollectionKind::Line, Vec::new(), crate::rc::params());
        let id = store.insert(Box::new(collection));
        let depth = art
            .project(id, &mut store, &projection)
            .expect("a collection");
        let paths = store
            .get_as::<CollectionArtist>(id)
            .expect("a collection")
            .paths
            .clone();
        (paths, depth)
    }

    #[test]
    fn hidden_vertices_leave_gaps_but_still_deepen_contours() {
        let depth_of = |p: Vec3| unit_projection().project_all(&[p])[0][2];
        let inside = [0.5, 0.5, 0.5];
        // The hidden vertex is the one in front, so that it sets the depth.
        let outside = [[0.5, 0.5, 9.0], [0.5, 0.5, -8.0]]
            .into_iter()
            .min_by(|a, b| depth_of(*a).total_cmp(&depth_of(*b)))
            .expect("two candidates");
        // Projected together, as the artists project their two vertices
        // (a single point is multiplied otherwise, which may round
        // differently).
        let projected = unit_projection().project_all(&[inside, outside]);
        let (near, far) = (projected[0][2], projected[1][2]);
        assert!(far < near);
        let template = Path::polyline(vec![Point::ORIGIN; 2]);
        let contour = Art3D::Paths(vec![Path3D::new(template, vec![inside, outside])]);
        let (paths, depth) = project_hidden(&contour);
        assert!(paths[0].vertices()[0].is_finite());
        assert!(!paths[0].vertices()[1].is_finite());
        assert_eq!(depth, far, "matplotlib drops the mask of contours");
        let lines = Art3D::Segments(Segments3D::new(vec![vec![inside, outside]]));
        let (paths, depth) = project_hidden(&lines);
        assert!(!paths[0].vertices()[1].is_finite());
        assert_eq!(depth, near, "line collections keep it");
    }
}
