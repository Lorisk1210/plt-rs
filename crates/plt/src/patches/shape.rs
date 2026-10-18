//! What a patch geometry provides: its outline and placement, and how it
//! is mapped to display space for drawing.

use std::fmt::Debug;

use plt_core::{Affine2D, Path};

use super::{PatchError, PatchStyle};
use crate::artist::{ArtistBase, ArtistId, LayoutContext};
use crate::transforms::ResolvedTransform;

/// The geometry of a patch (matplotlib's `get_path` + `get_patch_transform`).
pub trait PatchShape: Debug + Clone + 'static {
    /// Whether the patch draws an edge by default (matplotlib's
    /// `_edge_default`).
    const EDGE_DEFAULT: bool = false;

    /// The outline, in the shape's own coordinates.
    fn path(&self) -> Path;

    /// Maps the outline into the patch's coordinate system (usually data).
    fn patch_transform(&self) -> Affine2D {
        Affine2D::IDENTITY
    }

    /// Whether the shape is degenerate and should not affect autoscaling.
    fn skips_autoscale(&self) -> bool {
        false
    }

    /// Adjusts the defaults of a new patch of this shape, as the
    /// constructors of some matplotlib patch classes do (an `Arc` is not
    /// filled, a `FancyArrowPatch` has round caps, ...).
    fn init_style(&self, _style: &mut PatchStyle, _base: &mut ArtistBase) {}

    /// The outlines to paint in display space, each with the affine map the
    /// renderer applies and whether its face is filled. By default the
    /// outline goes through the patch transform and then the artist's.
    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let placed =
            ResolvedTransform::affine(self.patch_transform()).then(frame.transform.clone());
        let outline = self
            .path()
            .with_interpolation_steps(frame.interpolation_steps);
        let (path, affine) = placed.split_path(&outline);
        Ok(vec![DisplayPath::filled(path.into_owned(), affine)])
    }

    /// The outline mapped by the patch transform: in the coordinates of the
    /// artist transform (matplotlib's patch path before `transData`).
    fn placed_path(&self) -> Path {
        self.path().transformed(&self.patch_transform())
    }

    /// Whether a clip to this shape clips to its bounding box instead of
    /// its outline (matplotlib's `set_clip_path` makes a `Rectangle` the
    /// clip box).
    fn clips_to_bounding_box(&self) -> bool {
        false
    }
}

/// Where a patch is being drawn: what [`PatchShape::display_paths`] may
/// consult.
#[derive(Clone, Debug)]
pub struct ShapeFrame<'a> {
    /// The figure geometry.
    pub layout: LayoutContext<'a>,
    /// The artist transform (the patch's coordinates to display pixels).
    pub transform: ResolvedTransform,
    /// The edge width in points.
    pub linewidth: f64,
    /// The axes or figure containing the patch.
    pub parent: Option<ArtistId>,
    /// Pieces each outline segment is cut into for projections that bend
    /// lines (matplotlib's `get_path()._interpolation_steps`).
    pub interpolation_steps: usize,
}

/// One outline ready for the renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayPath {
    /// The outline after the non-affine part of its transform.
    pub path: Path,
    /// The affine map to display pixels.
    pub affine: Affine2D,
    /// Whether the face is filled (open arrow heads are only stroked).
    pub filled: bool,
}

impl DisplayPath {
    /// An outline whose face is filled.
    pub fn filled(path: Path, affine: Affine2D) -> Self {
        Self {
            path,
            affine,
            filled: true,
        }
    }

    /// An outline in display pixels, filled or only stroked.
    pub fn display(path: Path, filled: bool) -> Self {
        Self {
            path,
            affine: Affine2D::IDENTITY,
            filled,
        }
    }

    /// The outline in display pixels.
    pub fn to_display(&self) -> Path {
        self.path.transformed(&self.affine)
    }
}
