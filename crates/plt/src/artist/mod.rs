//! The artist model: everything visible in a figure is an [`Artist`].
//!
//! A figure owns its artists in an [`ArtistStore`]. Users manipulate them
//! through cheap, clonable *handles* (`Figure`, `Axes`, `Line2D`, ...) that
//! pair the figure with an [`ArtistId`]; every handle implements
//! [`ArtistHandle`] for the properties all artists share.
//!
//! Rendering happens in two passes: a mutable *prepare* pass (autoscaling,
//! tick computation, layout) followed by a read-only *draw* pass in which
//! each artist renders itself through a [`DrawContext`].

mod base;
mod context;
mod cursor;
mod filter;
mod handle;
mod pick;
#[macro_use]
mod properties;
mod store;

use std::any::Any;
use std::fmt::Debug;

use plt_core::path::DataLimits;
use plt_core::{Affine2D, Bbox, Point};
use plt_render::ClipPath;

use crate::patches::{ClipRegion, PatchView};
use crate::transforms::Transform;

pub(crate) use base::rc_sketch;
pub use base::{ArtistBase, ClipPathSpec, is_hidden_label};
pub(crate) use context::DrawnDpiCor;
pub use context::{DrawContext, DrawError, LayoutContext};
pub(crate) use cursor::format_values;
pub use filter::AggFilter;
pub use handle::{ArtistHandle, RawHandle};
pub(crate) use pick::segment_hits;
pub use pick::{PickFn, Picker};
pub use plt_render::{FilterImage, Filtered};
pub use properties::BaseProps;
pub use store::{ArtistId, ArtistStore};

/// The points an artist adds to its axes' data limits, in data
/// coordinates, and whether their x and y are data values (matplotlib's
/// `_update_line_limits`, `_update_patch_limits`, `_update_image_limits`
/// and `_update_collection_limits`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LimitPoints {
    /// The points.
    pub points: Vec<Point>,
    /// Whether the x and the y of the points are data values.
    pub directions: (bool, bool),
}

impl LimitPoints {
    /// The points in the coordinates of `transform` as data limits of
    /// `axes`: in the directions where they are data values, mapped by the
    /// fixed affine map in front of the data transform (matplotlib's
    /// `trf_to_data = transform - ax.transData`); `None` when no direction
    /// is a data value.
    pub(crate) fn through(
        transform: &Transform,
        axes: ArtistId,
        points: impl FnOnce() -> Vec<Point>,
    ) -> Option<Self> {
        let directions = transform.data_directions(axes);
        if directions == (false, false) {
            return None;
        }
        let points = match transform.affine_before_data(axes) {
            Some(to_data) if to_data != Affine2D::IDENTITY => points()
                .into_iter()
                .map(|p| to_data.transform_point(p))
                .collect(),
            _ => points(),
        };
        Some(Self { points, directions })
    }

    /// The corners and smallest positive point of `limits`; `None` when
    /// they are empty.
    pub fn from_limits(limits: DataLimits, directions: (bool, bool)) -> Option<Self> {
        (!limits.bbox.is_null()).then(|| Self {
            points: vec![limits.bbox.p0, limits.bbox.p1, limits.minpos],
            directions,
        })
    }
}

/// Behaviour shared by every drawable object.
pub trait Artist: Any + Debug {
    /// Properties common to all artists.
    fn base(&self) -> &ArtistBase;

    /// Mutable access to the common properties.
    fn base_mut(&mut self) -> &mut ArtistBase;

    /// Renders the artist (only called when it is visible).
    fn draw(&self, ctx: &mut DrawContext<'_>);

    /// Artists owned by this one; they are removed together with it.
    fn children(&self) -> Vec<ArtistId> {
        Vec::new()
    }

    /// Forgets a child that is being removed from the figure.
    fn remove_child(&mut self, _child: ArtistId) {}

    /// Bounding box in display coordinates, if the artist occupies space.
    fn window_extent(&self, _ctx: &LayoutContext<'_>) -> Option<Bbox> {
        None
    }

    /// The artist's outline in display space, used when another artist is
    /// clipped to it (`set_clip_path(patch)`).
    fn outline(&self, _ctx: &LayoutContext<'_>) -> Option<ClipPath> {
        None
    }

    /// Whether a display-space point hits the artist (for picking).
    fn contains(&self, _ctx: &LayoutContext<'_>, _point: Point) -> bool {
        false
    }

    /// The region in display space that clips the end of a connection
    /// starting or ending at the artist (matplotlib's `patchA` and
    /// `patchB`, tested with `Patch.contains`): a patch's outline or a
    /// text's box; `None` for other artists.
    fn connection_region(&self, _ctx: &LayoutContext<'_>) -> Option<ClipRegion> {
        None
    }

    /// Whether display point `at` picks the artist, and which of its items
    /// (matplotlib's `contains` as `Artist.pick` asks it): `radius` is the
    /// tolerance a [`Picker::Radius`] gives. `None` when the pointer misses.
    fn pick_hit(
        &self,
        ctx: &LayoutContext<'_>,
        at: Point,
        _radius: Option<f64>,
    ) -> Option<Vec<usize>> {
        self.contains(ctx, at).then(Vec::new)
    }

    /// What the artist adds to the data limits of `axes`, when it is added
    /// and when the axes recompute them ([`crate::axes::Axes::relim`]);
    /// `None` for artists that do not count.
    fn limit_points(&self, _axes: ArtistId) -> Option<LimitPoints> {
        None
    }

    /// What a status bar shows of the artist's data under display point
    /// `at`, such as `[0.532]` (matplotlib's `format_cursor_data` of
    /// `get_cursor_data`); asked only of artists with
    /// [`ArtistBase::mouseover`] set that contain the point.
    fn cursor_text(&self, _ctx: &LayoutContext<'_>, _at: Point) -> Option<String> {
        None
    }

    /// The artist as a patch, whatever its shape (matplotlib's
    /// `isinstance(artist, Patch)`), for code that treats every patch
    /// alike such as legends.
    fn as_patch(&self) -> Option<PatchView<'_>> {
        None
    }
}
