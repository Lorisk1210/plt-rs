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
mod handle;
#[macro_use]
mod properties;
mod store;

use std::any::Any;
use std::fmt::Debug;

use plt_core::{Bbox, Point};
use plt_render::ClipPath;

pub use base::{ArtistBase, ClipPathSpec};
pub use context::{DrawContext, DrawError, LayoutContext};
pub use handle::{ArtistHandle, RawHandle};
pub use properties::BaseProps;
pub use store::{ArtistId, ArtistStore};

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
}
