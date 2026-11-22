//! Renderer abstraction with raster (PNG), SVG, PDF, PostScript and PGF
//! backends.
//!
//! Artists in the crate `pltrs` draw through the [`Renderer`] trait. A
//! [`GraphicsContext`] holds the stroke and clipping state. The backends
//! change these calls into pixels or vector documents:
//!
//! * [`RasterRenderer`] gives the output of the matplotlib Agg backend,
//!   pixel for pixel, and writes PNG.
//! * [`SvgRenderer`] writes the SVG documents of the matplotlib
//!   `RendererSVG`. Text is glyph paths, or text elements if `svg.fonttype`
//!   is none.
//! * [`PdfRenderer`] writes the PDF documents of the matplotlib
//!   `RendererPdf`. Text uses Type 3 or Type 42 fonts or the 14 core
//!   fonts. Usetex text uses the Type 1 fonts of its DVI file.
//! * [`PsRenderer`] writes the PostScript and EPS documents of the
//!   matplotlib `RendererPS`. Text uses Type 3 or Type 42 fonts, or the
//!   core fonts of `ps.useafm`.
//! * [`PgfRenderer`] writes the LaTeX PGF pictures of the matplotlib
//!   `RendererPgf`. LaTeX typesets the text.
//!
//! TeX text (`text.usetex`) goes to a backend through
//! [`Renderer::draw_tex`]. Each backend implements it as the matplotlib
//! `draw_tex` does. It uses the glyphs of the DVI file or the image of
//! `dvipng`. It writes them as embedded Type 1 fonts, glyph paths or LaTeX
//! source. With
//! `text.usetex`, the PostScript backend tags the text for the LaTeX
//! package `psfrag`. The caller then runs LaTeX, `dvips` and Ghostscript
//! ([`PsOutput::Psfrag`]).
//!
//! The vector backends draw in points and rasterize images at their own
//! resolution ([`Renderer::image_magnification`]). They share [`Metadata`]
//! and [`VectorError`]. A [`MixedModeRenderer`] around a vector backend
//! draws rasterized artists into an image in the document.
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
    PdfTemplatePaint, pdf_line, pdf_text_string,
};
pub use pgf::{PgfDocument, PgfOptions, PgfRenderer, pgf_text, pgf_unknown_family};
pub use ps::{
    PageOrientation, PageSetupError, Paper, PsDocument, PsOptions, PsOutput, PsRenderer,
    PsfragFigure, pstoeps,
};
pub use quad_mesh::{QuadMesh, QuadMeshData};
pub use raster::{
    GlyphBitmap, GlyphShape, RasterBackendError, RasterRenderer, glyph_axis_aligned, render_glyph,
    set_texts_as_paths,
};
pub use renderer::{Renderer, TextLine, TextLines, draw_text_as_path};
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
