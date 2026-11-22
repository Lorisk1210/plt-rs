//! Anti-aliased scanline rasterizer, stroker and compositor.
//!
//! `pltrs-raster` copies the parts of Anti-Grain Geometry (Agg) that the
//! matplotlib Agg backend uses. The aim is output that is identical pixel
//! for pixel. It works in device pixels (y down, origin at the top left).
//! The caller transforms, NaN-filters, snaps and simplifies paths before.
//!
//! * [`Pixmap`]: RGBA8 with straight alpha (Agg `pixfmt_rgba32_plain`).
//! * [`Mask`]: a gray8 coverage map for clip masks and glyphs.
//!   [`Mask::blurred`] blurs it as the SVG `feGaussianBlur` does, for drop
//!   shadows.
//! * [`Rasterizer`]: the Agg `rasterizer_scanline_aa`, with 24.8
//!   fixed-point cell accumulation, non-zero and even-odd rules, [`Gamma`]
//!   and a clip box.
//! * The vertex pipeline: curve flattening like `conv_curve`, dashing like
//!   `vcgen_dash` and stroking like `vcgen_stroke`. [`StrokeStyle`]
//!   configures it.
//! * Compositing with the matplotlib `fixed_blender_rgba_plain`, clip
//!   boxes ([`ClipBox`]) and clip masks ([`Clip`]).
//! * [`Painter`]: fills, strokes, hatch patterns, Gouraud triangles,
//!   [`MarkerStamp`]s and transformed coverage. [`draw_image`] and
//!   [`draw_coverage`] blit.
//! * [`resample`]: the matplotlib image resampler (`_image.resample`), with
//!   the Agg interpolation kernels and span generators.
#![forbid(unsafe_code)]

mod blur;
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
pub use raster::{
    CoverageRounding, Covers, DEFAULT_CELL_LIMIT, Gamma, GrayBitmap, Rasterizer, Scanline, Span,
    render_glyph,
};
pub use render::image::{draw_coverage, draw_image};
pub use render::marker::MarkerStamp;
pub use style::{
    Antialias, DashPattern, FillRule, InnerJoin, LineCap, LineJoin, MAX_DASH_PAIRS, StrokeStyle,
};
