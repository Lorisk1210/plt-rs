//! Renderer abstraction with raster (PNG), SVG, PDF, PostScript and PGF
//! backends.
//!
//! Artists in the `pltrs` crate describe what to draw through the
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

pub use collection::{CollectionItem, ItemCycle, PathCollection, PathCollectionData};
pub use filter::{FilterFn, FilterImage, Filtered, SharedFilter};
pub use gc::{CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, Sketch, SnapMode};
pub use image::{ImageSizeError, RgbaImage};
pub use mixed::MixedModeRenderer;
pub use pdf::{
    PdfFont, PdfHatch, PdfMarkerPaint, PdfObject, PdfOptions, PdfRenderer, PdfShadingColors,
    PdfTemplatePaint, pdf_text_string,
};
pub use pgf::{PgfDocument, PgfOptions, PgfRenderer, pgf_text};
pub use ps::{
    PageOrientation, PageSetupError, Paper, PsDocument, PsOptions, PsOutput, PsRenderer,
    PsfragFigure, pstoeps,
};
pub use quad_mesh::{QuadMesh, QuadMeshData};
pub use raster::{
    GlyphBitmap, GlyphShape, RasterBackendError, RasterRenderer, glyph_axis_aligned, render_glyph,
    set_texts_as_paths,
};
pub use renderer::{Renderer, draw_text_as_path};
pub use svg::{
    Attribute, ImageFile, PROLOG as SVG_PROLOG, SvgFontType, SvgOptions, SvgRenderer,
    UnknownSvgFontType, XmlWriter,
};
pub use vector::{
    FontType, Metadata, PNG_SOFTWARE, PngMetadata, SvgMetadata, Timestamp, Trapped, VectorError,
    gzip,
};

use pltrs_core::{Affine2D, Bbox};

/// The affine map taking the unit square onto `bbox`.
fn unit_box(bbox: &Bbox) -> Affine2D {
    Affine2D::scaling(bbox.width(), bbox.height()).translate(bbox.x0(), bbox.y0())
}
