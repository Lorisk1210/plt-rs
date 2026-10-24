//! The options of a save (matplotlib's `savefig` keywords): resolution,
//! the part of the figure saved, padding, transparency, colours, format
//! and document settings.

use plt_core::Bbox;
use plt_render::{Metadata, PageOrientation, Paper};

use crate::colors::Color;
use crate::figure::SaveFormat;

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

/// Options of [`Figure::savefig_with`](crate::figure::Figure::savefig_with);
/// unset ones follow the `savefig.*` rcParams.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveOptions {
    /// Resolution (`savefig.dpi`, the figure's own by default). Vector
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
    /// Document metadata of the vector formats (plt-rs as creator and
    /// the current or `SOURCE_DATE_EPOCH` date by default) and the text
    /// entries of PNG files (matplotlib's `Software` entry by default).
    pub metadata: Option<Metadata>,
    /// JPEG quality from 1 to 100 (Pillow's `quality` in matplotlib's
    /// `pil_kwargs`; 75 by default).
    pub jpeg_quality: Option<u8>,
    /// How PostScript places the figure on the page
    /// (`savefig.orientation`).
    pub orientation: Option<PageOrientation>,
    /// The page of a PostScript document (`ps.papersize`; Encapsulated
    /// PostScript is sized to the figure).
    pub papertype: Option<Paper>,
}
