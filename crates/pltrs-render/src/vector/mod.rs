//! Infrastructure shared by the SVG, PDF and PostScript backends: Python's
//! number formatting, embedded-font data, glyph tracking, run analysis,
//! images, metadata and the error type.

mod font;
mod format;
mod image;
mod metadata;
mod text;
mod tracker;

use std::fmt;

use pltrs_codec::CompressionLevel;
use pltrs_codec::checksum::crc32;
use pltrs_codec::png::EncodeError;
use pltrs_core::Path;
use pltrs_core::path::convert::VertexSource;
use pltrs_core::{Affine2D, Bbox, Point};
use pltrs_text::TextError;
use pltrs_text::tex::TexError;

pub(crate) use font::{FontCache, FontInfo};
pub(crate) use format::{fixed, general, hex, repr, repr_bytes, repr_str, short, trimmed};
pub(crate) use image::{flipped_rows, png_idat};
pub use metadata::{Metadata, PNG_SOFTWARE, PngMetadata, SvgMetadata, Timestamp, Trapped};
pub(crate) use text::{POINTS_DPI, glyph_chars, in_points, prev_kerns};
pub(crate) use tracker::{CharacterTracker, Subset};

/// How PDF and PostScript documents embed TrueType fonts (matplotlib's
/// `pdf.fonttype` and `ps.fonttype`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontType {
    /// Type 3 fonts drawing the glyph outlines, 256 glyphs per font.
    #[default]
    Type3,
    /// Type 42 fonts: subsets of the TrueType font program itself.
    Type42,
}

impl FontType {
    /// The font type numbered `number` (3 or 42).
    pub const fn from_number(number: u32) -> Option<Self> {
        match number {
            3 => Some(Self::Type3),
            42 => Some(Self::Type42),
            _ => None,
        }
    }

    /// How many character codes one embedded font may use (matplotlib's
    /// `_FONT_MAX_GLYPH`).
    pub(crate) const fn subset_size(self) -> u32 {
        match self {
            Self::Type3 => 256,
            Self::Type42 => 65536,
        }
    }
}

/// Why a vector document could not be produced.
#[derive(Debug)]
pub enum VectorError {
    /// The page size or resolution is not positive and finite.
    InvalidCanvas {
        /// Requested width.
        width: f64,
        /// Requested height.
        height: f64,
        /// Requested image resolution.
        dpi: f64,
    },
    /// A font could not be read.
    Font(TextError),
    /// An image could not be encoded.
    Image(EncodeError),
    /// A number the format cannot represent (PDF has no NaN or infinity).
    NonFinite(&'static str),
    /// Something the format cannot hold.
    Unsupported(&'static str),
    /// TeX text could not be typeset or its fonts embedded.
    Tex(TexError),
    /// A PDF object was reserved but never written before the
    /// cross-reference table.
    UnwrittenObject(usize),
    /// A PDF stream was begun while another is open (matplotlib's
    /// assertion in `beginStream`).
    StreamOpen,
}

impl fmt::Display for VectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCanvas { width, height, dpi } => write!(
                f,
                "invalid page {width} x {height} at {dpi} dpi: all must be positive"
            ),
            Self::Font(err) => write!(f, "cannot embed font: {err}"),
            Self::Image(err) => write!(f, "cannot encode image: {err}"),
            Self::NonFinite(what) => write!(f, "cannot write a non-finite {what}"),
            Self::Unsupported(what) => f.write_str(what),
            Self::Tex(err) => write!(f, "cannot draw TeX text: {err}"),
            Self::UnwrittenObject(id) => {
                write!(f, "PDF object {id} was reserved but never written")
            }
            Self::StreamOpen => f.write_str("a PDF stream is open already"),
        }
    }
}

impl std::error::Error for VectorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Font(err) => Some(err),
            Self::Image(err) => Some(err),
            Self::Tex(err) => Some(err),
            Self::InvalidCanvas { .. }
            | Self::NonFinite(_)
            | Self::Unsupported(_)
            | Self::UnwrittenObject(_)
            | Self::StreamOpen => None,
        }
    }
}

impl From<TextError> for VectorError {
    fn from(err: TextError) -> Self {
        Self::Font(err)
    }
}

impl From<TexError> for VectorError {
    fn from(err: TexError) -> Self {
        Self::Tex(err)
    }
}

impl From<EncodeError> for VectorError {
    fn from(err: EncodeError) -> Self {
        Self::Image(err)
    }
}

/// Validates a page size and image resolution.
pub(crate) fn check_canvas(width: f64, height: f64, dpi: f64) -> Result<(), VectorError> {
    let valid = |v: f64| v.is_finite() && v > 0.0;
    if valid(width) && valid(height) && valid(dpi) {
        Ok(())
    } else {
        Err(VectorError::InvalidCanvas { width, height, dpi })
    }
}

/// Problems met while drawing, reported when the document is finished.
#[derive(Debug, Default)]
pub(crate) struct Problems(Option<VectorError>);

impl Problems {
    /// Records `result`'s error unless an earlier one is kept.
    pub(crate) fn record<T>(&mut self, result: Result<T, VectorError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(err) => {
                self.0.get_or_insert(err);
                None
            }
        }
    }

    /// `Ok(value)` unless a problem was recorded.
    pub(crate) fn into_result<T>(self, value: T) -> Result<T, VectorError> {
        self.0.map_or(Ok(value), Err)
    }
}

/// The marker positions of `path` (matplotlib's `iter_segments(trans,
/// clip=clip, simplify=False)`): the last vertex of every segment after
/// transforming, dropping non-finite points and clipping to `clip`.
pub(crate) fn segment_ends(path: &Path, transform: Affine2D, clip: Option<Bbox>) -> Vec<Point> {
    segment_points(path, transform, clip, false)
}

/// The first vertex of every segment of `path`, conditioned as for
/// [`segment_ends`] (where the PGF backend places markers).
pub(crate) fn segment_starts(path: &Path, transform: Affine2D, clip: Option<Bbox>) -> Vec<Point> {
    segment_points(path, transform, clip, true)
}

/// The first or last vertex of every segment of the conditioned path.
fn segment_points(path: &Path, transform: Affine2D, clip: Option<Bbox>, first: bool) -> Vec<Point> {
    let mut stream = path
        .iter_vertices()
        .transform(transform)
        .remove_nans(true, path.codes().is_some())
        .clip_to(clip);
    let mut points = Vec::new();
    while let Some(vertex) = stream.next() {
        let mut point = vertex.point;
        for _ in 1..vertex.code.vertex_count() {
            if let Some(next) = stream.next()
                && !first
            {
                point = next.point;
            }
        }
        points.push(point);
    }
    points
}

/// A gzip member (RFC 1952) holding `data`, as Python's `gzip.GzipFile`
/// writes it at its default level 9 (used for `.svgz`).
pub fn gzip(data: &[u8]) -> Vec<u8> {
    // Magic, deflate, no flags, no modification time, maximum compression,
    // unknown operating system.
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 2, 255];
    out.extend(pltrs_codec::deflate::compress(data, CompressionLevel::BEST));
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_members_round_trip() {
        let data = b"<svg/>\n".repeat(20);
        let member = gzip(&data);
        assert_eq!(&member[..3], &[0x1f, 0x8b, 8]);
        let body = &member[10..member.len() - 8];
        assert_eq!(pltrs_codec::deflate::decompress(body).unwrap(), data);
        assert_eq!(
            member[member.len() - 4..],
            (data.len() as u32).to_le_bytes()
        );
    }

    #[test]
    fn segment_ends_skip_nans_and_clip() {
        let path = Path::polyline(vec![
            Point::new(1.0, 1.0),
            Point::new(f64::NAN, 2.0),
            Point::new(3.0, 3.0),
            Point::new(50.0, 3.0),
        ]);
        let clip = Bbox::from_extents(0.0, 0.0, 10.0, 10.0);
        let ends = segment_ends(&path, Affine2D::scaling(2.0, 1.0), Some(clip));
        assert_eq!(ends[0], Point::new(2.0, 1.0));
        assert_eq!(ends[1], Point::new(6.0, 3.0));
        assert_eq!(ends.last().map(|p| p.x), Some(11.0), "clipped one unit out");
    }

    #[test]
    fn canvas_validation_and_problems() {
        assert!(check_canvas(10.0, 10.0, 72.0).is_ok());
        assert!(matches!(
            check_canvas(0.0, 10.0, 72.0),
            Err(VectorError::InvalidCanvas { .. })
        ));
        let mut problems = Problems::default();
        assert_eq!(problems.record(Ok::<_, VectorError>(3)), Some(3));
        assert_eq!(
            problems.record(Err::<(), _>(VectorError::NonFinite("x"))),
            None
        );
        assert!(problems.into_result(()).is_err());
    }
}
