//! Renderer abstraction with raster (PNG), SVG, PDF and PostScript backends.
//!
//! Artists in the `plt` crate describe what to draw through the
//! [`Renderer`] trait, using a [`GraphicsContext`] for stroke and clipping
//! state. Backends translate those calls into pixels or vector documents:
//!
//! * [`RasterRenderer`] reproduces matplotlib's Agg backend pixel for
//!   pixel and writes PNG.
//! * [`SvgRenderer`] writes the SVG documents of matplotlib's `RendererSVG`
//!   (text as glyph paths).
//! * [`PdfRenderer`] writes the PDF documents of matplotlib's
//!   `RendererPdf` (text in Type 3 fonts).
//! * [`PsRenderer`] writes the PostScript and EPS documents of
//!   matplotlib's `RendererPS` (text in Type 3 fonts).
//!
//! The vector backends draw in points, rasterize images at their own
//! resolution ([`Renderer::image_magnification`]) and share [`Metadata`]
//! and [`VectorError`]. A [`MixedModeRenderer`] around one of them draws
//! rasterized artists into an image placed in the document.
#![forbid(unsafe_code)]

mod collection;
mod filter;
mod gc;
mod image;
mod mixed;
mod pdf;
mod ps;
mod quad_mesh;
mod raster;
mod renderer;
mod svg;
mod vector;

pub use collection::{CollectionItem, PathCollection};
pub use filter::{FilterFn, FilterImage, Filtered};
pub use gc::{CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, Sketch, SnapMode};
pub use image::{ImageSizeError, RgbaImage};
pub use mixed::MixedModeRenderer;
pub use pdf::{PdfOptions, PdfRenderer};
pub use ps::{Orientation, Paper, PsOptions, PsRenderer};
pub use quad_mesh::QuadMesh;
pub use raster::{RasterBackendError, RasterRenderer};
pub use renderer::Renderer;
pub use svg::{SvgFontType, SvgOptions, SvgRenderer};
pub use vector::{FontType, Metadata, Timestamp, Trapped, VectorError, gzip};

use plt_core::{Affine2D, Bbox};

/// The affine map taking the unit square onto `bbox`.
fn unit_box(bbox: &Bbox) -> Affine2D {
    Affine2D::scaling(bbox.width(), bbox.height()).translate(bbox.x0(), bbox.y0())
}
