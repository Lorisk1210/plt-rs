//! What a patch geometry provides: its outline and placement, and how it
//! is mapped to display space for drawing.

use std::fmt::Debug;

use pltrs_core::{Affine2D, Path, Point};

use super::{PatchError, PatchStyle};
use crate::artist::{ArtistBase, ArtistId, LayoutContext};
use crate::transforms::{ResolvedTransform, Transform, TransformContext};

/// The geometry of a patch (matplotlib's `get_path` + `get_patch_transform`).
pub trait PatchShape: Debug + Clone + 'static {
    /// Whether the patch draws an edge by default (matplotlib's
    /// `_edge_default`).
    const EDGE_DEFAULT: bool = false;

    /// Whether patches of the shape draw with the interpolation steps all
    /// rectangles share once a program sets them (matplotlib's
    /// `Rectangle`, whose outline is the shared `Path.unit_rectangle()`;
    /// see [`set_shared_rectangle_steps`](super::set_shared_rectangle_steps)).
    const SHARES_UNIT_PATH: bool = false;

    /// The outline, in the shape's own coordinates.
    fn path(&self) -> Path;

    /// Maps the outline into the patch's coordinate system (usually data).
    fn patch_transform(&self) -> Affine2D {
        Affine2D::IDENTITY
    }

    /// Whether [`display_paths`](Self::display_paths) only maps the
    /// outline through the frame's transform, as the default does, needing
    /// nothing else of the figure: a draw then lets the figure go while a
    /// transform or a style of the program's makes or maps the outline.
    /// Shapes overriding `display_paths` say `false`.
    const MAPS_OUTLINE_ALONE: bool = true;

    /// Whether making the outline runs code of the program's (a box style
    /// of the program's), which matplotlib lets read and change the
    /// figure while it is drawn.
    fn runs_program(&self) -> bool {
        false
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
        Ok(outline_through(
            self,
            &frame.transform,
            frame.interpolation_steps,
        ))
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

    /// The points the shape counts in the data limits of `axes`, in its
    /// data coordinates, when it is placed by `transform` in the figure
    /// `ctx` as it is now and its edge is `linewidth` points wide: `None`
    /// for those bounding its outline, which most shapes make in their own
    /// coordinates; a shape made in display space (an arrow) gives its own.
    fn limit_outline(
        &self,
        _transform: &Transform,
        _linewidth: f64,
        _ctx: &dyn TransformContext,
        _axes: ArtistId,
    ) -> Option<Vec<Point>> {
        None
    }
}

/// The outline of `shape` cut into `interpolation_steps` pieces per
/// segment, through its patch transform, then `transform`, the artist's:
/// what [`PatchShape::display_paths`] paints by default.
pub(crate) fn outline_through<S: PatchShape>(
    shape: &S,
    transform: &ResolvedTransform,
    interpolation_steps: usize,
) -> Vec<DisplayPath> {
    let placed = ResolvedTransform::affine(shape.patch_transform()).then(transform.clone());
    let outline = shape.path().with_interpolation_steps(interpolation_steps);
    let (path, affine) = placed.split_outline(&outline);
    vec![DisplayPath::filled(path.into_owned(), affine)]
}

/// Where a patch is being drawn: what [`PatchShape::display_paths`] may
/// consult.
#[derive(Clone, Debug)]
pub struct ShapeFrame<'a> {
    /// The figure geometry.
    pub layout: LayoutContext<'a>,
    /// The artist transform (the patch's coordinates to display pixels).
    pub transform: ResolvedTransform,
    /// Pixels per point of the sizes of arrows (matplotlib's `_dpi_cor`):
    /// the canvas resolution's while drawing,
    /// [`LayoutContext::measured_dpi_cor`] while measuring.
    pub dpi_cor: f64,
    /// Pixels per point of the patch's last draw (of the draw running
    /// while it draws), `None` before its first: what the offset of a
    /// shadow follows, as matplotlib's `Shadow` sets it when it draws.
    pub drawn_dpi_cor: Option<f64>,
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
