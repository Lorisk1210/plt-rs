//! The raster backend: matplotlib's `RendererAgg` on top of `plt-raster`.
//!
//! Every drawing call conditions its geometry with plt-core's port of
//! matplotlib's path converters, chained exactly as `_backend_agg.h` chains
//! them, and rasterizes it with plt-raster's port of Agg, so the pixels
//! match matplotlib's Agg output.

mod condition;
mod draw;
mod filter;
mod glyph;
mod mono;
mod text;

use std::fmt;

use plt_codec::png::{self, EncodeError, ImageRef};
use plt_core::Path;
use plt_core::path::convert::SimplifySettings;
use plt_raster::{Mask, Painter, Pixmap, RasterError, Rgba8};

use crate::gc::ClipPath;
use crate::image::RgbaImage;
use crate::vector::PngMetadata;

/// Why a raster canvas could not be created or encoded.
#[derive(Debug)]
pub enum RasterBackendError {
    /// The canvas size is invalid (matplotlib caps each side below 2²³).
    Canvas(RasterError),
    /// The resolution is not a positive finite number.
    Dpi(f64),
    /// PNG encoding failed.
    Encode(EncodeError),
}

impl fmt::Display for RasterBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canvas(err) => write!(f, "invalid canvas: {err}"),
            Self::Dpi(dpi) => write!(f, "dpi must be positive, got {dpi}"),
            Self::Encode(err) => write!(f, "PNG encoding failed: {err}"),
        }
    }
}

impl std::error::Error for RasterBackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canvas(err) => Some(err),
            Self::Encode(err) => Some(err),
            Self::Dpi(_) => None,
        }
    }
}

/// A rasterized clip path, reused while the same clip path and transform
/// come back (matplotlib's `lastclippath` cache).
#[derive(Debug)]
struct CachedClip {
    key: ClipPath,
    mask: Mask,
}

/// Renders into an RGBA pixel buffer exactly like matplotlib's
/// `RendererAgg` (straight alpha, initially transparent white).
#[derive(Debug)]
pub struct RasterRenderer {
    pixmap: Pixmap,
    painter: Painter,
    /// Canvas size as requested; the buffer holds its truncated size, as
    /// matplotlib's `RendererAgg(int(width), int(height), dpi)` does.
    size: (f64, f64),
    dpi: f64,
    simplify: SimplifySettings,
    /// Long unfilled paths are drawn in pieces of at most this many
    /// vertices (matplotlib's `agg.path.chunksize`; 0 never splits).
    chunksize: usize,
    clip_cache: Option<CachedClip>,
    /// The last conditioned path, kept for its allocations.
    conditioned: Path,
    /// Canvases set aside while filtered artists draw.
    set_aside: Vec<Pixmap>,
    errors: Vec<RasterError>,
}

impl RasterRenderer {
    /// A canvas of `width × height` display pixels at `dpi`.
    pub fn new(width: f64, height: f64, dpi: f64) -> Result<Self, RasterBackendError> {
        if !(dpi.is_finite() && dpi > 0.0) {
            return Err(RasterBackendError::Dpi(dpi));
        }
        let pixmap = Pixmap::filled(width as u32, height as u32, Rgba8::TRANSPARENT_WHITE)
            .map_err(RasterBackendError::Canvas)?;
        Ok(Self {
            pixmap,
            painter: Painter::new(),
            size: (width, height),
            dpi,
            simplify: SimplifySettings::default(),
            chunksize: 0,
            clip_cache: None,
            conditioned: Path::default(),
            set_aside: Vec::new(),
            errors: Vec::new(),
        })
    }

    /// Uses matplotlib's `path.simplify` / `path.simplify_threshold`
    /// settings (the defaults otherwise).
    #[must_use]
    pub fn with_simplify(mut self, settings: SimplifySettings) -> Self {
        self.simplify = settings;
        self
    }

    /// Draws unfilled, unhatched paths that would be simplified in pieces
    /// of at most `chunksize` vertices when they have more, as matplotlib
    /// does with `agg.path.chunksize` (0, the default, never splits; nor do
    /// sizes up to 100).
    #[must_use]
    pub fn with_path_chunksize(mut self, chunksize: usize) -> Self {
        self.chunksize = chunksize;
        self
    }

    /// The pixels drawn so far.
    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
    }

    /// The pixels, for changes made outside drawing calls (matplotlib's
    /// `restore_region` writes saved pixels back into the canvas).
    pub fn pixmap_mut(&mut self) -> &mut Pixmap {
        &mut self.pixmap
    }

    /// Consumes the renderer, returning its pixels.
    pub fn into_pixmap(self) -> Pixmap {
        self.pixmap
    }

    /// The pixels as an [`RgbaImage`].
    pub fn to_image(&self) -> RgbaImage {
        RgbaImage::new(
            self.pixmap.width() as usize,
            self.pixmap.height() as usize,
            self.pixmap.data().to_vec(),
        )
        .expect("a pixmap holds width × height × 4 bytes")
    }

    /// Encodes the pixels as an RGBA PNG carrying the resolution, without
    /// text entries.
    ///
    /// # Errors
    ///
    /// When the image cannot be encoded.
    pub fn to_png(&self) -> Result<Vec<u8>, RasterBackendError> {
        self.to_png_with(&PngMetadata::none())
    }

    /// Encodes the canvas as PNG with the text entries of `metadata`, as
    /// matplotlib's `savefig` writes them.
    ///
    /// # Errors
    ///
    /// When the image cannot be encoded, or a keyword is not a valid PNG
    /// keyword (1 to 79 printable Latin-1 characters).
    pub fn to_png_with(&self, metadata: &PngMetadata) -> Result<Vec<u8>, RasterBackendError> {
        let image = ImageRef::rgba8(
            self.pixmap.width(),
            self.pixmap.height(),
            self.pixmap.data(),
        )
        .map_err(RasterBackendError::Encode)?;
        metadata
            .apply_to(png::Encoder::new().dpi(self.dpi))
            .encode(image)
            .map_err(RasterBackendError::Encode)
    }

    /// Resets every pixel to transparent white (matplotlib's `clear()`).
    pub fn clear(&mut self) {
        self.pixmap.fill(Rgba8::TRANSPARENT_WHITE);
    }

    /// Problems met while drawing (matplotlib would have raised, e.g.
    /// `OverflowError` for paths exceeding the cell limit), oldest first.
    pub fn take_errors(&mut self) -> Vec<RasterError> {
        std::mem::take(&mut self.errors)
    }

    /// Records the outcome of one raster operation.
    fn record(&mut self, result: Result<(), RasterError>) {
        if let Err(err) = result {
            self.errors.push(err);
        }
    }

    fn width(&self) -> u32 {
        self.pixmap.width()
    }

    fn height(&self) -> u32 {
        self.pixmap.height()
    }
}

#[cfg(test)]
mod tests;
