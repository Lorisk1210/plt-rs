//! Vector documents of a subset of the figure cases, compared structurally
//! with the SVG, PDF and PostScript goldens matplotlib wrote for them.
//!
//! Both documents are reduced to a sequence of drawing [`Item`]s — one per
//! drawn element or painting operator, with every reference (marker,
//! glyph, clip path, hatch pattern, font, image, graphics state) replaced
//! by what it refers to and every number kept for a tolerant comparison —
//! and the sequences are aligned and compared (`diff`). Ids, object
//! numbers, metadata, compression and image encodings are thereby ignored.

mod diff;
mod pdf;
mod ps;
mod svg;
mod xml;

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use plt::figure::{SaveFormat, SaveOptions};
use plt_render::Metadata;

pub use diff::{Item, Token, VectorReport, compare_items};

use super::FigureCase;

/// The cases with vector goldens (`VECTOR_CASES` in the spec's
/// `figures/test_vectors.py`).
pub const VECTOR_CASES: &[&str] = &[
    "axes/grid_major_minor_styles",
    "axes/mathtext_labels",
    "bars/hatches",
    "basic/line",
    "contour/tripcolor_gouraud",
    "images/clip_path_circle",
    "images/imshow_default",
    "images/pcolormesh_gouraud",
    "lines/alpha_overlap",
    "lines/joinstyles",
    "lines/linestyles_dash_tuples",
    "lines/markers_filled",
    "patches/clip_path_artists",
    "patches/hatches_basic",
    "patches/linecollection_styles",
    "patches/patchcollection_cmap",
    "polar/fill_and_fill_between",
    "polar/plot_default",
    "scatter/default",
    "scatter/edgecolor_face_and_alpha",
    "stats/fill_between_two_curves_alpha",
    "text/color_alpha_background",
    "text/mathtext_mixed_sizes",
    "text/rotation_default_mode",
    "text/unicode",
];

/// A vector format with goldens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VectorFormat {
    /// SVG with glyphs as paths.
    Svg,
    /// PDF with Type 3 fonts.
    Pdf,
    /// PostScript with Type 3 fonts.
    Ps,
}

impl VectorFormat {
    /// Every format.
    pub const ALL: [Self; 3] = [Self::Svg, Self::Pdf, Self::Ps];

    /// The file extension.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Ps => "ps",
        }
    }

    const fn save_format(self) -> SaveFormat {
        match self {
            Self::Svg => SaveFormat::Svg,
            Self::Pdf => SaveFormat::Pdf,
            Self::Ps => SaveFormat::Ps,
        }
    }

    /// Reduces a document to drawing items.
    pub fn items(self, document: &[u8]) -> Result<Vec<Item>, String> {
        match self {
            Self::Svg => svg::items(document),
            Self::Pdf => pdf::items(document),
            Self::Ps => ps::items(document),
        }
    }
}

impl fmt::Display for VectorFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

impl FigureCase {
    /// The vector golden of this case.
    pub fn vector_path(&self, format: VectorFormat) -> PathBuf {
        crate::spec_root().join("goldens/figures").join(format!(
            "{}.{}",
            self.name,
            format.extension()
        ))
    }
}

/// Renders a case as `format` the way the spec saves its goldens: at the
/// case's resolution, without metadata and with the fixed SVG id salt.
pub fn render_vector(case: &FigureCase, format: VectorFormat) -> Result<Vec<u8>, Box<dyn Error>> {
    case.with_figure(|fig| {
        plt::rc::set("svg.hashsalt", "plt-rs")?;
        let options = SaveOptions {
            dpi: Some(case.dpi),
            metadata: Some(Metadata::none()),
            ..SaveOptions::default()
        };
        Ok(fig.to_format(format.save_format(), &options)?)
    })
}

/// Renders a case and compares it with its golden.
pub fn vector_report(case: &FigureCase, format: VectorFormat) -> Result<VectorReport, String> {
    let golden =
        std::fs::read(case.vector_path(format)).map_err(|e| format!("cannot read golden: {e}"))?;
    let rendered = render_vector(case, format).map_err(|e| format!("render failed: {e}"))?;
    let expected = format
        .items(&golden)
        .map_err(|e| format!("cannot read golden: {e}"))?;
    let actual = format
        .items(&rendered)
        .map_err(|e| format!("cannot read rendering: {e}"))?;
    Ok(compare_items(&expected, &actual))
}
