//! The documents of vector renderers a program makes around a file of its
//! own (matplotlib's `RendererSVG(width, height, svgwriter)`,
//! `RendererPS(width, height, pswriter)` and `RendererPgf(figure, fh)`):
//! renderers configured by the rc parameters as a figure's documents are,
//! which the program draws into piece by piece, outside any save
//! ([`Drawing::draw_onto`](crate::artist::Drawing::draw_onto)).
//!
//! An SVG renderer gives its whole document when it is finished; the
//! PostScript and PGF renderers give what was drawn so far with
//! `take_body`, which matplotlib writes to the file as it draws.

use pltrs_render::{Metadata, PgfRenderer, PsRenderer, SvgRenderer, VectorError};

use super::pgf::pgf_options;
use super::postscript::ps_options;
use super::svg::svg_options;

/// The renderer of an SVG document of `width` × `height` points with
/// `metadata`, its images embedded and rasterized at `image_dpi`.
///
/// ```
/// use pltrs::artist::{DrawStep, Drawing};
/// use pltrs::path::Path;
/// use pltrs::transforms::{Affine2D, Point};
/// use pltrs_render::GraphicsContext;
/// let mut page = pltrs::figure::svg_page(100.0, 50.0, 72.0, Default::default())?;
/// let mut drawing = Drawing::default();
/// drawing.push(DrawStep::Path {
///     gc: GraphicsContext::default(),
///     path: Path::polyline(vec![Point::new(0.0, 0.0), Point::new(100.0, 50.0)]),
///     transform: Affine2D::IDENTITY,
///     fill: None,
/// });
/// assert!(drawing.draw_onto(&mut page).is_empty());
/// let document = page.finish()?;
/// assert!(document.contains(r#"width="100pt""#) && document.contains("<path"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the size or resolution is not positive and finite.
pub fn svg_page(
    width: f64,
    height: f64,
    image_dpi: f64,
    metadata: Metadata,
) -> Result<SvgRenderer, VectorError> {
    SvgRenderer::new(width, height, image_dpi, svg_options(metadata, None))
}

/// The renderer of a PostScript page of `width` × `height` inches drawn in
/// points, its images rasterized at `image_dpi`.
///
/// ```
/// let mut page = pltrs::figure::ps_page(2.0, 1.0, 72.0)?;
/// page.set_linewidth(2.0, true);
/// assert_eq!(page.take_body(), "2 setlinewidth\n");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the size or resolution is not positive and finite.
pub fn ps_page(width: f64, height: f64, image_dpi: f64) -> Result<PsRenderer, VectorError> {
    PsRenderer::new(width, height, image_dpi, ps_options())
}

/// The renderer of a PGF picture of `width` × `height` inches drawn at
/// `dpi`, its images written beside a file named after `image_stem`.
///
/// ```
/// let mut page = pltrs::figure::pgf_page(2.0, 1.0, 100.0, None)?;
/// let (text, images) = page.take_body();
/// assert!(text.is_empty() && images.is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the size or resolution is not positive and finite.
pub fn pgf_page(
    width: f64,
    height: f64,
    dpi: f64,
    image_stem: Option<String>,
) -> Result<PgfRenderer, VectorError> {
    PgfRenderer::new(width, height, dpi, pgf_options(image_stem))
}
