//! Patches: filled 2-D shapes with an edge (matplotlib.patches).
//!
//! A patch is a [`PatchShape`] (rectangle, circle, polygon, wedge, path,
//! fancy box, arrow, ...) painted with a [`PatchStyle`]. Adding a shape to
//! an axes returns a typed [`Patch`] handle:
//!
//! ```
//! use plt::prelude::*;
//! use plt::patches::Rectangle;
//! let ax = Figure::new().subplot();
//! let bar = ax.add_patch(Rectangle::new((0.0, 0.0), 2.0, 3.0));
//! bar.set_facecolor("tab:green").set_hatch("//");
//! bar.update_shape(|rect| rect.height = 5.0);
//! assert_eq!(bar.shape().height, 5.0);
//! ```
//!
//! Most shapes are fixed in data coordinates. A [`FancyBboxPatch`] draws a
//! [`BoxStyle`] around a rectangle; a [`FancyArrowPatch`] draws an
//! [`ArrowStyle`] along a [`ConnectionStyle`] path between two points,
//! sized in display space, and a [`ConnectionPatch`] does so between points
//! in different coordinate systems.

mod arcs;
mod arrow_style;
mod arrows;
mod artist;
mod box_style;
mod connection_patch;
mod connection_style;
mod error;
mod fancy_arrow;
mod fancy_box;
mod props;
mod shadow;
mod shape;
mod shapes;
mod step;
mod style;
mod style_spec;

pub use arcs::{Annulus, Arc};
pub use arrow_style::{ArrowStyle, Bracket, CurveArrow, CurveEnd};
pub use arrows::{Arrow, ArrowShape, FancyArrow};
pub use artist::Patch;
pub(crate) use artist::{PatchArtist, PatchParts};
pub use box_style::{ArrowHead, BoxShape, BoxStyle, DEFAULT_PAD};
pub use connection_patch::{ConnectionEnd, ConnectionPatch};
pub use connection_style::{ClipRegion, ConnectionStyle};
pub use error::{PatchError, ShapeError, StyleError};
pub(crate) use fancy_arrow::round_ends;
pub use fancy_arrow::{ArrowRoute, ArrowSpec, FancyArrowPatch};
pub use fancy_box::FancyBboxPatch;
pub use props::PatchProps;
pub use shadow::{Shadow, ShadowError};
pub use shape::{DisplayPath, PatchShape, ShapeFrame};
pub use shapes::{
    Circle, CirclePolygon, Ellipse, PathShape, Polygon, Rectangle, RegularPolygon, RotationPoint,
    Wedge,
};
pub use step::{StepBaseline, StepPatch, StepPatchError};
pub use style::PatchStyle;
pub(crate) use style::{draw_patch_paths, paint_paths};
