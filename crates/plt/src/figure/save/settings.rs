//! Renderer settings taken from the rcParams when saving: PDF documents
//! (`pdf.*`), font embedding, path simplification and the raster canvas
//! (`agg.path.chunksize`).

use plt_core::path::convert::SimplifySettings;
use plt_render::{Metadata, PdfOptions, RasterBackendError, RasterRenderer};

use crate::rc::FontType;

/// The settings of a PDF document from the `pdf.*` rcParams, with
/// `metadata`.
pub(in crate::figure) fn pdf_options(metadata: Metadata) -> PdfOptions {
    let rc = crate::rc::params();
    PdfOptions {
        compression: u8::try_from(rc.pdf_compression.clamp(0, 9)).unwrap_or(6),
        inherit_color: rc.pdf_inheritcolor,
        metadata,
        simplify: simplify_settings(),
        composite_images: rc.image_composite_image,
        fonttype: embedding(rc.pdf_fonttype),
        core_fonts: rc.pdf_use14corefonts,
    }
}

/// How a `pdf.fonttype` or `ps.fonttype` setting embeds fonts.
pub(in crate::figure) const fn embedding(fonttype: FontType) -> plt_render::FontType {
    match fonttype {
        FontType::Type3 => plt_render::FontType::Type3,
        FontType::TrueType => plt_render::FontType::Type42,
    }
}

/// matplotlib's `path.simplify` settings from the rcParams.
pub(in crate::figure) fn simplify_settings() -> SimplifySettings {
    let rc = crate::rc::params();
    SimplifySettings {
        enabled: rc.path_simplify,
        threshold: rc.path_simplify_threshold,
    }
}

/// The whole pixels a canvas `size` pixels long holds: truncated, but
/// rounded up within 1e-8 of the next pixel to undo floating-point error
/// (matplotlib's `FigureCanvasBase.get_width_height`).
pub(in crate::figure) fn whole_pixels(size: f64) -> u32 {
    // The conversion saturates: negative and NaN sizes give 0.
    (size + 1e-8) as u32
}

/// A raster canvas of `width × height` whole pixels at `dpi`, with the
/// current `path.simplify` and `agg.path.chunksize` settings.
pub(in crate::figure) fn raster_renderer(
    width: u32,
    height: u32,
    dpi: f64,
) -> Result<RasterRenderer, RasterBackendError> {
    // Negative sizes never split, as matplotlib's `npts > nmax > 100`.
    let chunksize = usize::try_from(crate::rc::params().agg_path_chunksize).unwrap_or(0);
    RasterRenderer::new(f64::from(width), f64::from(height), dpi).map(|canvas| {
        canvas
            .with_simplify(simplify_settings())
            .with_path_chunksize(chunksize)
    })
}
