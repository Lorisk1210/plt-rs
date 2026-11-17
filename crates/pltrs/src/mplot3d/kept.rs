//! The 3-D geometry of an artist as matplotlib's `art3d` classes keep it
//! in their attributes (`_segments3d`, `_faces`, `_3dverts_codes`,
//! `_dir_vec`, `_zsortfunc`, ...): what pltrs (Python) reads when a
//! program asks an artist these axes made for such an attribute.

use pltrs_core::Path;

use super::art3d::{Art3D, ZSort};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use crate::artist::ArtistHandle;

/// Which of matplotlib's `art3d` classes keeps a geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept3DKind {
    /// The points of a line (`Line3D`).
    Line,
    /// The anchor of a text, then its direction (`Text3D`).
    Text,
    /// The vertices of each path of a converted collection
    /// (`Collection3D`).
    Paths,
    /// The points of each polyline (`Line3DCollection`).
    Segments,
    /// The positions of the markers (`Path3DCollection`,
    /// `Patch3DCollection`).
    Points,
    /// The vertices of each polygon (`Poly3DCollection`).
    Polys,
    /// The outline of a patch (`Patch3D`).
    Patch,
    /// The vertices of the path of a patch (`PathPatch3D`).
    PathPatch,
}

impl Kept3DKind {
    /// The kind's name, in lower case.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Text => "text",
            Self::Paths => "paths",
            Self::Segments => "segments",
            Self::Points => "points",
            Self::Polys => "polys",
            Self::Patch => "patch",
            Self::PathPatch => "pathpatch",
        }
    }
}

/// The 3-D geometry these axes keep for one of their artists.
#[derive(Clone, Debug, PartialEq)]
pub struct Kept3D {
    /// Which kind of geometry it is.
    pub kind: Kept3DKind,
    /// The points, in the parts the kind has (one per polyline, polygon
    /// or path; one in all for a line, the markers and a patch; the
    /// anchor and the direction of a text).
    pub parts: Vec<Vec<Vec3>>,
    /// The 2-D paths whose codes the projected parts keep, one per part:
    /// those of a converted collection, of polygons lifted from a 2-D
    /// collection and of a path patch; none otherwise.
    pub templates: Vec<Path>,
    /// How the depth of a polygon is computed (polygons only).
    pub zsort: Option<ZSort>,
    /// Whether the polygons are closed (polygons only).
    pub closed: Option<bool>,
    /// The alpha factor of the deepest markers (markers only).
    pub min_alpha: Option<f64>,
}

impl Kept3D {
    /// `parts` of `kind`, with nothing else.
    fn of(kind: Kept3DKind, parts: Vec<Vec<Vec3>>) -> Self {
        Self {
            kind,
            parts,
            templates: Vec::new(),
            zsort: None,
            closed: None,
            min_alpha: None,
        }
    }
}

impl Axes3D {
    /// The 3-D geometry kept for `artist`, a 3-D artist of these axes
    /// pltrs (Rust) projects.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Kept3DKind;
    /// let ax = Figure::new().subplot_3d();
    /// let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 2.0])?;
    /// let kept = ax.kept_3d(&line)?;
    /// assert_eq!(kept.kind, Kept3DKind::Line);
    /// assert_eq!(kept.parts, [vec![[0.0, 0.0, 0.0], [1.0, 1.0, 2.0]]]);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `artist` is not a 3-D artist of these axes, or is one a class
    /// of the program's projects.
    pub fn kept_3d(&self, artist: &impl ArtistHandle) -> Result<Kept3D, Axes3DError> {
        self.read_art(artist, |art| {
            Some(match art {
                Art3D::Line(points) => Kept3D::of(Kept3DKind::Line, vec![points.clone()]),
                Art3D::Text {
                    position,
                    direction,
                } => Kept3D::of(Kept3DKind::Text, vec![vec![*position], vec![*direction]]),
                Art3D::Paths(paths) => Kept3D {
                    templates: paths.iter().map(|path| path.template().clone()).collect(),
                    ..Kept3D::of(
                        Kept3DKind::Paths,
                        paths.iter().map(|path| path.vertices.clone()).collect(),
                    )
                },
                Art3D::Segments(segments) => {
                    Kept3D::of(Kept3DKind::Segments, segments.lines.clone())
                }
                Art3D::Points(points) => Kept3D {
                    min_alpha: Some(points.min_alpha),
                    ..Kept3D::of(Kept3DKind::Points, vec![points.positions.clone()])
                },
                Art3D::Polys(polys) => Kept3D {
                    templates: polys.templates.clone().unwrap_or_default(),
                    zsort: Some(polys.zsort),
                    closed: Some(polys.closed),
                    ..Kept3D::of(Kept3DKind::Polys, polys.faces.clone())
                },
                Art3D::Patch(outline) => Kept3D::of(Kept3DKind::Patch, vec![outline.clone()]),
                Art3D::PathPatch(path) => Kept3D {
                    templates: vec![path.template().clone()],
                    ..Kept3D::of(Kept3DKind::PathPatch, vec![path.vertices.clone()])
                },
                Art3D::Program { .. } => return None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::mplot3d::Axes3DError;

    #[test]
    fn each_kind_of_artist_keeps_its_geometry() {
        let ax = Figure::new().subplot_3d();
        let points = ax
            .scatter([0.0, 1.0], [2.0, 3.0], [4.0, 5.0])
            .expect("a scatter");
        let kept = ax.kept_3d(&points).expect("kept");
        assert_eq!(kept.kind, Kept3DKind::Points);
        assert_eq!(kept.parts, [vec![[0.0, 2.0, 4.0], [1.0, 3.0, 5.0]]]);
        assert!(kept.min_alpha.is_some() && kept.zsort.is_none());
        let label = ax.text(1.0, 2.0, 3.0, "deep", None);
        let kept = ax.kept_3d(&label).expect("kept");
        assert_eq!(kept.kind.name(), "text");
        assert_eq!(kept.parts[0], [[1.0, 2.0, 3.0]]);
        assert_eq!(kept.parts.len(), 2, "the anchor and the direction");
        let other = Figure::new().subplot_3d();
        assert_eq!(other.kept_3d(&label), Err(Axes3DError::Not3DArtist));
    }
}
