//! The 3-D geometry of the projection plane's artists and its projection
//! onto them (matplotlib's `art3d`: `Line3D`, `Text3D`, `Collection3D`,
//! `Line3DCollection`, `Path3DCollection`, `Poly3DCollection` and
//! `Patch3D`).
//!
//! matplotlib turns 2-D artists into 3-D ones by changing their class;
//! here a 2-D artist stays what it is and its 3-D geometry is kept next to
//! it. Projecting writes the projected geometry (and, for depth-sorted
//! collections, the reordered colours and sizes) into the 2-D artist.

mod points;
mod polys;

pub(crate) use points::Points3D;
pub(crate) use polys::Polys3D;
pub use polys::ZSort;

use plt_core::{Path, Point};

use super::camera::{Projection, norm_text_angle};
use super::math::{Vec3, add};
use crate::artist::{ArtistId, ArtistStore};
use crate::collections::CollectionArtist;
use crate::lines::LineArtist;
use crate::patches::{PatchArtist, PatchShape};
use crate::text::TextArtist;

/// The outline of a 3-D patch as projected by the last layout pass
/// (matplotlib's `Patch3D`, which unlike a `PathPatch` has no edge by
/// default).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectedOutline(pub Path);

impl PatchShape for ProjectedOutline {
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

/// A path whose vertices live in 3-D space; its codes are kept by a 2-D
/// template path.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path3D {
    /// The 2-D path the projected vertices are written into.
    template: Path,
    /// One 3-D point per template vertex.
    vertices: Vec<Vec3>,
}

impl Path3D {
    /// `template` with its vertices replaced by `vertices`.
    pub(crate) fn new(template: Path, vertices: Vec<Vec3>) -> Self {
        debug_assert_eq!(template.len(), vertices.len());
        Self { template, vertices }
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
    Segments(Vec<Vec<Vec3>>),
    /// Scatter markers (`Path3DCollection`).
    Points(Points3D),
    /// Polygons (`Poly3DCollection`).
    Polys(Polys3D),
    /// The outline of a flat patch (`Patch3D`).
    Patch(Vec<Vec3>),
}

impl Art3D {
    /// Whether the artist counts as data (matplotlib's `has_data`).
    pub(crate) fn is_data(&self) -> bool {
        !matches!(self, Self::Text { .. })
    }

    /// How computed z-order counts the artist; lines and texts keep
    /// their own z-order.
    pub(crate) fn sort_kind(&self) -> Option<SortKind> {
        match self {
            Self::Line(_) | Self::Text { .. } => None,
            Self::Paths(_) | Self::Segments(_) | Self::Points(_) | Self::Polys(_) => {
                Some(SortKind::Collection)
            }
            Self::Patch(_) => Some(SortKind::Patch),
        }
    }

    /// Projects the geometry onto the 2-D artist `id` and returns the
    /// depth used for computed z-order (matplotlib's `do_3d_projection`);
    /// `None` when the artist is gone or of another type.
    pub(crate) fn project(
        &self,
        id: ArtistId,
        store: &mut ArtistStore,
        projection: &Projection,
    ) -> Option<f64> {
        match self {
            Self::Line(points) => {
                let line = store.get_as_mut::<LineArtist>(id)?;
                (line.xdata, line.ydata) = points
                    .iter()
                    .map(|&p| {
                        projection
                            .project_clipped(p)
                            .map_or((f64::NAN, f64::NAN), |q| (q[0], q[1]))
                    })
                    .unzip();
                line.recache();
                Some(f64::NAN)
            }
            Self::Text {
                position,
                direction,
            } => {
                let text = store.get_as_mut::<TextArtist>(id)?;
                let anchor = projection.project(*position);
                let end = projection.project(add(*position, *direction));
                let angle = (end[1] - anchor[1]).atan2(end[0] - anchor[0]).to_degrees();
                text.position = Point::new(anchor[0], anchor[1]);
                text.rotation = norm_text_angle(angle);
                Some(f64::NAN)
            }
            Self::Paths(paths) => {
                let collection = store.get_as_mut::<CollectionArtist>(id)?;
                let mut depth = Depth::default();
                collection.paths = paths
                    .iter()
                    .map(|path| {
                        let mut projected = path.template.clone();
                        for (vertex, &p) in projected.vertices_mut().iter_mut().zip(&path.vertices)
                        {
                            *vertex = depth.project(projection, p);
                        }
                        projected
                    })
                    .collect();
                Some(depth.min_or(1e9))
            }
            Self::Segments(segments) => {
                let collection = store.get_as_mut::<CollectionArtist>(id)?;
                let mut depth = Depth::default();
                collection.paths = segments
                    .iter()
                    .map(|segment| {
                        Path::polyline(
                            segment
                                .iter()
                                .map(|&p| depth.project(projection, p))
                                .collect(),
                        )
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
                let points = outline
                    .iter()
                    .map(|&p| match projection.project_clipped(p) {
                        Some(q) => depth.include(q),
                        None => Point::new(f64::NAN, f64::NAN),
                    })
                    .collect();
                patch.shape = ProjectedOutline(Path::polyline(points));
                Some(depth.min_or(f64::NAN))
            }
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
    /// Projects `p` without clipping, recording its depth.
    fn project(&mut self, projection: &Projection, p: Vec3) -> Point {
        self.include(projection.project(p))
    }

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
