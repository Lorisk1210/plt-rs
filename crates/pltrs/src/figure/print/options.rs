//! The options of a save (matplotlib's `savefig` keywords): resolution,
//! the part of the figure saved, padding, transparency, colours, format
//! and document settings.

use pltrs_core::Bbox;
use pltrs_render::{Metadata, PageOrientation, Paper};

use crate::colors::Color;
use crate::figure::{PgfTextMetrics, SaveFormat};

/// Which part of the figure is saved (matplotlib's `bbox_inches`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BboxInches {
    /// The area everything is drawn in, padded (`"tight"`).
    Tight,
    /// This box, in inches from the figure's bottom-left corner.
    Inches(Bbox),
}

/// The padding around a tight saved area (matplotlib's `pad_inches`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PadInches {
    /// This many inches on every side.
    Inches(f64),
    /// The constrained layout's own padding, `w_pad` left and right and
    /// `h_pad` above and below, when the figure is laid out by constrained
    /// layout; `savefig.pad_inches` otherwise (`"layout"`).
    Layout,
}

impl From<f64> for PadInches {
    fn from(inches: f64) -> Self {
        Self::Inches(inches)
    }
}

/// The options of a save (matplotlib's `savefig` keywords).
///
/// [`Figure::savefig_with`](crate::figure::Figure::savefig_with),
/// [`Figure::to_format`](crate::figure::Figure::to_format) and each
/// `to_*_with` method of [`Figure`](crate::figure::Figure) take these
/// options. A field that is `None` uses its `savefig.*` runtime setting.
///
/// | Fields | What they set | Formats |
/// |--------|---------------|---------|
/// | [`dpi`](Self::dpi) | The resolution. Vector formats draw their images at it. | All |
/// | [`bbox_inches`](Self::bbox_inches), [`pad_inches`](Self::pad_inches) | The part of the figure to save, such as a tight crop, and its padding. | All |
/// | [`transparent`](Self::transparent) | Removes the figure and axes backgrounds. | All |
/// | [`facecolor`](Self::facecolor), [`edgecolor`](Self::edgecolor) | The background color and the frame color of the figure. | All |
/// | [`format`](Self::format) | The file format, instead of the extension of the file name. | All |
/// | [`metadata`](Self::metadata) | The title, author and other document entries. | PNG and vector formats |
/// | [`jpeg_quality`](Self::jpeg_quality) | The JPEG quality, from 1 to 100. | JPEG |
/// | [`avif_quality`](Self::avif_quality) | The AVIF quality, from 0 to 100. 100 is lossless. | AVIF |
/// | [`orientation`](Self::orientation), [`papertype`](Self::papertype) | The orientation and the paper of the page. | PostScript |
/// | [`pgf_text_metrics`](Self::pgf_text_metrics) | How pltrs measures the text: with its own text engine or with LaTeX. | PGF |
///
/// ```
/// use pltrs::figure::{BboxInches, Figure, SaveOptions};
///
/// let fig = Figure::with_size(4.0, 3.0, 100.0);
/// fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
/// let options = SaveOptions {
///     dpi: Some(300.0),
///     bbox_inches: Some(BboxInches::Tight),
///     transparent: Some(true),
///     ..SaveOptions::default()
/// };
/// assert!(fig.to_png_with(options)?.starts_with(b"\x89PNG"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveOptions {
    /// Resolution (`savefig.dpi`, by default the resolution the figure
    /// was made at, see [`Dpi::Figure`](crate::rc::Dpi::Figure)). Vector
    /// formats are drawn in points and rasterize images at this resolution.
    pub dpi: Option<f64>,
    /// The part of the figure saved (`savefig.bbox`, the whole figure by
    /// default).
    pub bbox_inches: Option<BboxInches>,
    /// Padding around a tight box (`savefig.pad_inches` inches by default).
    pub pad_inches: Option<PadInches>,
    /// Whether the figure and axes backgrounds are left out
    /// (`savefig.transparent`).
    pub transparent: Option<bool>,
    /// Figure background (`savefig.facecolor`, the figure's own by
    /// default); a colour that does not resolve fails the save.
    pub facecolor: Option<Color>,
    /// Figure border colour (`savefig.edgecolor`, the figure's own by
    /// default).
    pub edgecolor: Option<Color>,
    /// The file format, overriding the file name's extension
    /// (matplotlib's `format`).
    pub format: Option<SaveFormat>,
    /// Document metadata of the vector formats (pltrs as creator and
    /// the current or `SOURCE_DATE_EPOCH` date by default) and the text
    /// entries of PNG files (matplotlib's `Software` entry by default).
    pub metadata: Option<Metadata>,
    /// JPEG quality from 1 to 100 (Pillow's `quality` in matplotlib's
    /// `pil_kwargs`; 75 by default).
    pub jpeg_quality: Option<u8>,
    /// AVIF quality from 0 to 100, 100 being lossless (Pillow's `quality`
    /// in matplotlib's `pil_kwargs`; 75 by default).
    pub avif_quality: Option<u8>,
    /// How PostScript places the figure on the page
    /// (`savefig.orientation`).
    pub orientation: Option<PageOrientation>,
    /// The page of a PostScript document (`ps.papersize`; Encapsulated
    /// PostScript is sized to the figure).
    pub papertype: Option<Paper>,
    /// How the text of a PGF picture is measured: by pltrs' text engine
    /// (the default), or by LaTeX as matplotlib's PGF backend measures it.
    pub pgf_text_metrics: PgfTextMetrics,
}
