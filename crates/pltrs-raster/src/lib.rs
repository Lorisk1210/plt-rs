//! Anti-aliased scanline rasterizer, stroker and compositor.
//!
//! `pltrs-raster` reproduces the parts of Anti-Grain Geometry (Agg) that
//! matplotlib's Agg backend uses, with the aim of pixel-identical output.
//! It works purely in device pixel space (y down, origin top-left); the
//! caller transforms, NaN-filters, snaps and simplifies paths beforehand.
//!
//! * [`Pixmap`] — RGBA8 with straight alpha (Agg's `pixfmt_rgba32_plain`),
//!   and [`Mask`], a gray8 coverage map for clip masks and glyphs.
//! * [`Rasterizer`] — Agg's `rasterizer_scanline_aa`: 24.8 fixed-point cell
//!   accumulation, non-zero/even-odd rules, [`Gamma`], clip box.
//! * The vertex pipeline (curve flattening like `conv_curve`, dashing like
//!   `vcgen_dash`, stroking like `vcgen_stroke`), configured by
//!   [`StrokeStyle`].
//! * Compositing with matplotlib's `fixed_blender_rgba_plain`, clip boxes
//!   ([`ClipBox`]) and clip masks ([`Clip`]).
//! * [`Painter`] — fills, strokes, hatch patterns, Gouraud triangles,
//!   [`MarkerStamp`]s and transformed coverage, plus [`draw_image`] and
//!   [`draw_coverage`] for blitting.
//! * [`resample`] — matplotlib's image resampler (`_image.resample`):
//!   Agg's interpolation kernels and span generators.
#![forbid(unsafe_code)]

mod clip;
mod color;
mod dda;
mod error;
mod geometry;
mod mask;
mod painter;
mod pixmap;
mod raster;
mod render;
pub mod resample;
mod style;

pub use clip::{Clip, ClipBox};
pub use color::Rgba8;
pub use error::RasterError;
pub use geometry::PathSink;
pub use mask::Mask;
pub use painter::Painter;
pub use pixmap::{MAX_DIMENSION, MAX_PIXELS, Pixmap};
pub use raster::{CoverageRounding, Covers, DEFAULT_CELL_LIMIT, Gamma, Rasterizer, Scanline, Span};
pub use render::image::{draw_coverage, draw_image};
pub use render::marker::MarkerStamp;
pub use style::{
    Antialias, DashPattern, FillRule, InnerJoin, LineCap, LineJoin, MAX_DASH_PAIRS, StrokeStyle,
};
