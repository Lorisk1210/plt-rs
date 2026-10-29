//! Why a figure could not be rendered or saved.

use std::fmt;

use pltrs_render::{RasterBackendError, VectorError};
use pltrs_text::tex::TexError;

use super::SaveFormat;
use crate::figure::{DrawErrors, ProgramError};
use crate::image::BitmapError;

/// Why a figure could not be rendered or saved.
#[derive(Debug)]
pub enum SaveError {
    /// The canvas could not be created or encoded.
    Backend(RasterBackendError),
    /// A vector document could not be written.
    Vector(VectorError),
    /// The canvas could not be encoded as a bitmap file.
    Bitmap(BitmapError),
    /// Some artists could not be drawn.
    Draw(DrawErrors),
    /// The file format is not one pltrs can write.
    UnsupportedFormat(String),
    /// Metadata was given for a format that cannot hold it (the bitmap
    /// formats other than PNG).
    MetadataUnsupported(SaveFormat),
    /// A program the PostScript backend runs (a distiller, `dvips`) is
    /// missing or failed.
    Program(ProgramError),
    /// Writing the file failed.
    Io(std::io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "cannot render the figure: {err}"),
            Self::Vector(err) => write!(f, "cannot write the document: {err}"),
            Self::Bitmap(err) => write!(f, "cannot write the image: {err}"),
            Self::Draw(err) => err.fmt(f),
            Self::UnsupportedFormat(format) => write!(
                f,
                "format {format:?} is not supported (supported formats: eps, gif, jpeg, jpg, pdf, pgf, png, ps, raw, rgba, svg, svgz, tif, tiff, webp)"
            ),
            Self::MetadataUnsupported(format) => write!(
                f,
                "metadata not supported for format {:?}",
                format.extension()
            ),
            Self::Program(err) => err.fmt(f),
            Self::Io(err) => write!(f, "cannot write the figure: {err}"),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(err) => Some(err),
            Self::Vector(err) => Some(err),
            Self::Bitmap(err) => Some(err),
            Self::Draw(err) => Some(err),
            Self::Program(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::UnsupportedFormat(_) | Self::MetadataUnsupported(_) => None,
        }
    }
}

impl From<ProgramError> for SaveError {
    fn from(err: ProgramError) -> Self {
        Self::Program(err)
    }
}

impl From<TexError> for SaveError {
    /// TeX failing on a document is the document's failure.
    fn from(err: TexError) -> Self {
        Self::Vector(err.into())
    }
}
