//! Renderer abstraction with raster (PNG), SVG, PDF, PostScript and PGF
//! backends.
//!
//! Artists in the `plt` crate describe what to draw through the
//! [`Renderer`] trait, using a [`GraphicsContext`] for stroke and clipping
//! state. Backends translate those calls into pixels or vector documents:
//!
//! * [`RasterRenderer`] reproduces matplotlib's Agg backend pixel for
//!   pixel and writes PNG.
//! * [`SvgRenderer`] writes the SVG documents of matplotlib's `RendererSVG`
//!   (text as glyph paths, or as text elements with `svg.fonttype` none).
//! * [`PdfRenderer`] writes the PDF documents of matplotlib's
//!   `RendererPdf` (text in Type 3 or Type 42 fonts or the 14 core fonts,
//!   usetex text in the Type 1 fonts of its DVI file).
//! * [`PsRenderer`] writes the PostScript and EPS documents of
//!   matplotlib's `RendererPS` (text in Type 3 or Type 42 fonts or the
//!   core fonts of `ps.useafm`).
//! * [`PgfRenderer`] writes the LaTeX PGF pictures of matplotlib's
//!   `RendererPgf` (text left to LaTeX).
//!
//! TeX text (`text.usetex`) reaches a backend through
//! [`Renderer::draw_tex`], which each backend implements as matplotlib's
//! `draw_tex`: from the DVI file's glyphs (or `dvipng`'s image), as
//! embedded Type 1 fonts, as glyph paths, or as LaTeX source. With
//! `text.usetex`, the PostScript backend tags it for LaTeX's `psfrag`
//! package and leaves LaTeX, `dvips` and Ghostscript to the caller
//! ([`PsOutput::Psfrag`]).
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
mod pgf;
mod ps;
mod quad_mesh;
mod raster;
mod renderer;
mod svg;
#[cfg(test)]
mod testing;
mod vector;

pub use collection::{CollectionItem, PathCollection};
pub use filter::{FilterFn, FilterImage, Filtered};
pub use gc::{CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, Sketch, SnapMode};
pub use image::{ImageSizeError, RgbaImage};
pub use mixed::MixedModeRenderer;
pub use pdf::{PdfOptions, PdfRenderer};
pub use pgf::{PgfDocument, PgfOptions, PgfRenderer};
pub use ps::{
    PageOrientation, PageSetupError, Paper, PsDocument, PsOptions, PsOutput, PsRenderer,
    PsfragFigure, pstoeps,
};
pub use quad_mesh::QuadMesh;
pub use raster::{RasterBackendError, RasterRenderer};
pub use renderer::Renderer;
pub use svg::{ImageFile, SvgFontType, SvgOptions, SvgRenderer, UnknownSvgFontType};
pub use vector::{
    FontType, Metadata, PNG_SOFTWARE, PngMetadata, SvgMetadata, Timestamp, Trapped, VectorError,
    gzip,
};

use plt_core::{Affine2D, Bbox};

/// The affine map taking the unit square onto `bbox`.
fn unit_box(bbox: &Bbox) -> Affine2D {
    Affine2D::scaling(bbox.width(), bbox.height()).translate(bbox.x0(), bbox.y0())
}
