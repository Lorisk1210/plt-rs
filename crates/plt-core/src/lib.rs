//! Geometry, paths, affine transforms and colours shared by every plt-rs crate.
//!
//! This crate is the vocabulary of plt-rs. It deliberately mirrors the
//! semantics of matplotlib's `transforms.Affine2D`, `transforms.Bbox` and
//! `path.Path` so that higher layers can follow matplotlib's algorithms
//! faithfully:
//!
//! * [`Point`] — a 2-D point / vector in `f64`.
//! * [`Affine2D`] — an affine map with matplotlib's "append" composition
//!   order (`Affine2D::identity().scale(2.0, 2.0).rotate_deg(90.0)` scales
//!   first, then rotates).
//! * [`Bbox`] — a possibly-inverted axis-aligned box (`x0` may exceed `x1`).
//! * [`Path`] — vertices plus optional [`PathCode`]s, iterated as
//!   [`Segment`]s; see [`path`] for constructors of matplotlib's unit shapes.
//! * [`bezier`] — evaluation, subdivision and extrema of Bézier curves.
//! * [`Rgba`] — a straight-alpha colour with components in `[0, 1]`.
#![forbid(unsafe_code)]

mod affine;
mod bbox;
pub mod bezier;
mod color;
pub mod path;
mod point;

pub use affine::Affine2D;
pub use bbox::{Anchor, Bbox};
pub use color::Rgba;
pub use path::{Path, PathBuilder, PathCode, PathError, Segment};
pub use point::Point;
