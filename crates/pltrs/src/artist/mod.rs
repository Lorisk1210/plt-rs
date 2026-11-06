//! The artist model: everything visible in a figure is an [`Artist`].
//!
//! A figure owns its artists in an [`ArtistStore`]. Users manipulate them
//! through cheap, clonable *handles* (`Figure`, `Axes`, `Line2D`, ...) that
//! pair the figure with an [`ArtistId`]; every handle implements
//! [`ArtistHandle`] for the properties all artists share. An artist
//! removed from its figure ([`ArtistHandle::remove`]) is kept apart from
//! the store, so that its handles stay usable for as long as the figure
//! lives.
//!
//! Rendering happens in two passes: a mutable *prepare* pass (autoscaling,
//! tick computation, layout) followed by a read-only *draw* pass in which
//! each artist renders itself through a [`DrawContext`]: an artist of
//! your own draws paths, markers, text and images with the context's
//! [`Renderer`], in the colour, width and dashes of a
//! [`GraphicsContext`] ([`DrawContext::base_gc`] starts one from the
//! artist's common properties). Any artist may instead be drawn by a
//! [`Painter`] of the program's (matplotlib's `draw` of a subclass), which
//! is prepared before the draw pass, with the figure free to change, and
//! may keep what it draws until then as a [`Drawing`].

mod apart;
mod archive;
mod attach;
mod base;
mod context;
mod copy;
mod cursor;
mod custom;
mod detached;
mod drawing;
mod filter;
mod graphics_state;
mod handle;
mod kept;
mod loose;
mod painter;
mod pick;
#[macro_use]
mod properties;
mod raw;
mod stopping;
mod store;
mod tree;

use std::any::Any;
use std::fmt::Debug;

use pltrs_core::path::DataLimits;
use pltrs_core::{Bbox, Point};
use pltrs_render::ClipPath;

use crate::patches::{ClipRegion, PatchView};
use crate::ticker::AxisName;
use crate::transforms::Transform;
use crate::units::CustomValues;

pub use archive::FigureRef;
pub(crate) use archive::reconnect_drags;
pub use attach::AttachError;
pub(crate) use base::rc_sketch;
pub use base::{ArtistBase, ClipPathSpec, is_hidden_label};
pub use context::{DrawContext, DrawError, DrawErrorKind, LayoutContext};
pub(crate) use context::{DrawnDpiCor, FigureReader};
pub(crate) use cursor::format_values;
pub use custom::Custom;
pub(crate) use detached::DetachedArtists;
pub use drawing::{DrawStep, Drawing};
pub(crate) use filter::FilteredGroupArtist;
pub use filter::{AggFilter, FilteredGroup};
pub use graphics_state::{GraphicsState, drawing_with_state};
pub use handle::{ArtistHandle, TypedHandle};
pub use kept::ArtistClone;
pub(crate) use kept::KeptArtist;
pub(crate) use loose::take_in;
pub(crate) use painter::tight_extent_of;
pub use painter::{ArtistPainter, DrawTarget, MeasuredExtent, Painter};
pub(crate) use pick::segment_hits;
pub use pick::{PickFn, Picker};
pub use pltrs_render::{FilterImage, Filtered, GraphicsContext, Renderer, Sketch};
pub use properties::BaseProps;
pub use raw::RawHandle;
pub(crate) use stopping::draws_stop_at_first_problem;
pub use stopping::set_draws_stop_at_first_problem;
pub use store::{ArtistId, ArtistStore};
pub(crate) use store::{ArtistLookup, downcast_mut, downcast_ref};

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
    /// `axes`: in the directions where they are data values, mapped by
    /// what the transform applies before the data transform, non-affine
    /// steps included (matplotlib's `trf_to_data = transform -
    /// ax.transData`); `None` when no direction is a data value.
    pub(crate) fn through(
        transform: &Transform,
        axes: ArtistId,
        points: impl FnOnce() -> Vec<Point>,
    ) -> Option<Self> {
        let directions = transform.data_directions(axes);
        if directions == (false, false) {
            return None;
        }
        let points = match transform.to_data(axes) {
            Some(to_data) if !to_data.is_identity() => points()
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
///
/// An artist's methods run while its figure is laid out, drawn or
/// hit-tested, and get what they need as arguments ([`DrawContext`],
/// [`LayoutContext`]); while they hold what those give them, the figure
/// is borrowed. An artist of your own that keeps handles to its figure
/// may read them then, but changing the figure meanwhile, or reading it
/// while the figure is laid out, conflicts with that borrow and panics.
/// A draw lets the figure go whenever it hands its renderer out or is
/// told to ([`DrawContext::free`]): [`draw`](Self::draw) may then change
/// the figure, the artist drawn included, and the rest of the draw sees
/// the change. The artist drawn finishes as it was: a change to it is made
/// to a copy, which the figure keeps ([`ArtistClone`], why artists are
/// `Clone`). Event callbacks, timers and animation functions, which run
/// between draws, may change the figure as they please.
pub trait Artist: Any + Debug + ArtistClone {
    /// Properties common to all artists.
    fn base(&self) -> &ArtistBase;

    /// Mutable access to the common properties.
    fn base_mut(&mut self) -> &mut ArtistBase;

    /// Renders the artist (only called when it is visible).
    fn draw(&self, ctx: &mut DrawContext<'_>);

    /// Whether drawing the visible artist makes it fresh (matplotlib's
    /// `draw` ending with `self.stale = False`): not for an annotation
    /// whose point is clipped, whose `draw` returns first, nor for an
    /// inset indicator, whose `draw` never does.
    fn freshened_by_draw(&self, _ctx: &LayoutContext<'_>) -> bool {
        true
    }

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

    /// The box the artist adds to a tight bounding box before clipping
    /// (matplotlib's `get_tightbbox`): its window extent, unless the artist
    /// leaves itself out (a hidden annotation).
    fn tight_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.window_extent(ctx)
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
    /// and, for artists the axes added the way matplotlib's `add_line`,
    /// `add_patch`, `add_image` and `add_collection` add them, when the
    /// axes recompute them ([`crate::axes::Axes::relim`]); `None` for
    /// artists that do not count.
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

    /// The plotting values of user types the artist keeps along axis
    /// `name`, which it drew converted: when that axis of its axes
    /// changes unit ([`Axis::set_units`](crate::axes::Axis::set_units)),
    /// the axes converts each again and hands the numbers to
    /// [`reconvert_units`](Self::reconvert_units) (matplotlib's artists
    /// converting their values with `convert_xunits` when drawn, and its
    /// lines on the axis' `units` callback). None by default.
    fn unit_sources(&self, _name: AxisName) -> Vec<CustomValues> {
        Vec::new()
    }

    /// Takes the values [`unit_sources`](Self::unit_sources) gave for
    /// axis `name` converted in its new unit, one list per value list, in
    /// their order.
    fn reconvert_units(&mut self, _name: AxisName, _numbers: Vec<Vec<f64>>) {}
}
