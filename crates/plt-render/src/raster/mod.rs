//! The raster backend: matplotlib's `RendererAgg` on top of `plt-raster`.
//!
//! Every drawing call conditions its geometry with plt-core's port of
//! matplotlib's path converters, chained exactly as `_backend_agg.h` chains
//! them, and rasterizes it with plt-raster's port of Agg, so the pixels
//! match matplotlib's Agg output.

mod condition;
mod draw;
mod text;

use std::fmt;

use plt_codec::png::{self, EncodeError, ImageRef};
use plt_core::path::convert::SimplifySettings;
use plt_raster::{Mask, Painter, Pixmap, RasterError, Rgba8};

use crate::gc::ClipPath;
use crate::image::RgbaImage;

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
    clip_cache: Option<CachedClip>,
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
            clip_cache: None,
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

    /// The pixels drawn so far.
    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
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

    /// Encodes the pixels as an RGBA PNG carrying the resolution.
    pub fn to_png(&self) -> Result<Vec<u8>, RasterBackendError> {
        let image = ImageRef::rgba8(
            self.pixmap.width(),
            self.pixmap.height(),
            self.pixmap.data(),
        )
        .map_err(RasterBackendError::Encode)?;
        png::Encoder::new()
            .dpi(self.dpi)
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
