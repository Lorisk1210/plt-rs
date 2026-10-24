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
pub(crate) mod pdf;
mod pgf;
mod postscript;
mod ps;
mod svg;
mod truetype;
mod wide;
pub(crate) mod xml;

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use plt::figure::{PdfPages, SaveFormat, SaveOptions};
use plt_render::Metadata;

pub use diff::{Item, Token, VectorReport, compare_items};
pub use wide::{WideOutcome, WideResult, wide_comparison, wide_outcome, wide_summary, write_items};

use super::FigureCase;
use crate::scratch::ScratchDir;

/// The cases with vector goldens (`VECTOR_CASES` in the spec's
/// `figures/test_vectors.py`).
pub const VECTOR_CASES: &[&str] = &[
    "afm/core_fonts_layout",
    "afm/core_fonts_text",
    "afm/ps_fonts_layout",
    "afm/ps_fonts_text",
    "axes/grid_major_minor_styles",
    "axes/mathtext_labels",
    "bars/hatches",
    "basic/line",
    "contour/tripcolor_gouraud",
    "hinting/axes_100dpi",
    "images/clip_path_circle",
    "images/imshow_default",
    "images/pcolormesh_gouraud",
    "lines/alpha_overlap",
    "lines/joinstyles",
    "lines/linestyles_dash_tuples",
    "lines/markers_filled",
    "mpl_backends/multi_font",
    "mpl_backends/pdf_eps_savefig_when_color_is_none",
    "mpl_backends/pdf_grayscale_alpha",
    "mpl_backends/pdf_hatching_legend",
    "mpl_backends/pdf_hexbin_negative_offsets",
    "mpl_backends/pdf_kerning",
    "mpl_backends/pdf_scatter_large_markers_partial_clip",
    "mpl_backends/pdf_scatter_logscale",
    "mpl_backends/pdf_scatter_mixed_onoff_axis",
    "mpl_backends/pdf_scatter_offaxis_colored_visual",
    "mpl_backends/pdf_scatter_polar",
    "mpl_backends/pdf_use14corefonts",
    "mpl_backends/ps_colorbar_shift",
    "mpl_backends/ps_colored_hatch_zero_linewidth",
    "mpl_backends/ps_path_collection",
    "mpl_backends/ps_text_clip",
    "mpl_backends/ps_transparency",
    "mpl_backends/ps_type42_font_without_prep",
    "mpl_backends/ps_useafm",
    "mpl_backends/svg_bold_font_output",
    "mpl_backends/svg_bold_font_output_with_none_fonttype",
    "mpl_backends/svg_fill_black_with_alpha",
    "mpl_backends/svg_noscale",
    "mpl_backends/svg_prevent_rasterization",
    "mpl_backends/svg_rasterized",
    "mpl_backends/svg_rasterized_ordering",
    "patches/clip_path_artists",
    "patches/hatches_basic",
    "patches/linecollection_styles",
    "patches/patchcollection_cmap",
    "polar/fill_and_fill_between",
    "polar/plot_default",
    "scatter/default",
    "scatter/edgecolor_face_and_alpha",
    "shaping/features",
    "shaping/languages",
    "shaping/single_lines_only",
    "stats/fill_between_two_curves_alpha",
    "text/color_alpha_background",
    "text/mathtext_mixed_sizes",
    "text/rotation_default_mode",
    "text/unicode",
    "vectors/colorbar_many_colors",
    "vectors/rasterized_artists",
    "vectors/rasterization_zorder",
    "vectors/agg_filter",
    "vectors/text_fonts",
];

/// A case saved again with other settings (`VECTOR_VARIANTS` in the
/// spec's `figures/test_vectors.py`); its goldens are named
/// `<case>.<name>.<extension>`.
#[derive(Clone, Copy, Debug)]
pub struct VectorVariant {
    /// The figure case.
    pub case: &'static str,
    /// The variant's name.
    pub name: &'static str,
    /// The rcParams changed.
    pub rc: &'static [(&'static str, &'static str)],
    /// The formats saved.
    pub formats: &'static [VectorFormat],
}

/// The cases saved with other font settings.
pub const VECTOR_VARIANTS: &[VectorVariant] = &[
    VectorVariant {
        case: "vectors/text_fonts",
        name: "type42",
        rc: &[("pdf.fonttype", "42"), ("ps.fonttype", "42")],
        formats: &[VectorFormat::Pdf, VectorFormat::Ps],
    },
    VectorVariant {
        case: "vectors/text_fonts",
        name: "nofont",
        rc: &[("svg.fonttype", "none")],
        formats: &[VectorFormat::Svg],
    },
    VectorVariant {
        case: "shaping/features",
        name: "type42",
        rc: &[("pdf.fonttype", "42"), ("ps.fonttype", "42")],
        formats: &[VectorFormat::Pdf, VectorFormat::Ps],
    },
    VectorVariant {
        case: "shaping/features",
        name: "afm",
        rc: &[("pdf.use14corefonts", "True"), ("ps.useafm", "True")],
        formats: &[VectorFormat::Pdf, VectorFormat::Ps],
    },
    VectorVariant {
        case: "mpl_backends/multi_font",
        name: "type42",
        rc: &[("pdf.fonttype", "42"), ("ps.fonttype", "42")],
        formats: &[VectorFormat::Pdf, VectorFormat::Ps],
    },
    VectorVariant {
        case: "mpl_backends/multi_font",
        name: "astext",
        rc: &[("svg.fonttype", "none")],
        formats: &[VectorFormat::Svg],
    },
];

/// The cases collected, one per page, into the multi-page golden
/// `vectors/pages.pdf` (`PDF_PAGES` in the spec), with a note attached to
/// the second page.
pub const PDF_PAGES: &[&str] = &["vectors/text_fonts", "vectors/rasterized_artists"];

/// A vector format with goldens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VectorFormat {
    /// SVG with glyphs as paths.
    Svg,
    /// PDF with Type 3 fonts.
    Pdf,
    /// PostScript with Type 3 fonts.
    Ps,
    /// LaTeX PGF pictures (no goldens: compared in the wide comparison
    /// only, where matplotlib measures text like Agg instead of LaTeX).
    Pgf,
}

impl VectorFormat {
    /// Every format with goldens.
    pub const ALL: [Self; 3] = [Self::Svg, Self::Pdf, Self::Ps];

    /// Every format of the wide comparison.
    pub const WIDE: [Self; 4] = [Self::Svg, Self::Pdf, Self::Ps, Self::Pgf];

    /// The file extension.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Ps => "ps",
            Self::Pgf => "pgf",
        }
    }

    const fn save_format(self) -> SaveFormat {
        match self {
            Self::Svg => SaveFormat::Svg,
            Self::Pdf => SaveFormat::Pdf,
            Self::Ps => SaveFormat::Ps,
            Self::Pgf => SaveFormat::Pgf,
        }
    }

    /// Reduces a document to drawing items.
    pub fn items(self, document: &[u8]) -> Result<Vec<Item>, String> {
        match self {
            Self::Svg => svg::items(document),
            Self::Pdf => pdf::items(document),
            Self::Ps => ps::items(document),
            Self::Pgf => pgf::items(document),
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

    /// The golden of this case saved in `format` as `variant`.
    pub fn variant_path(&self, variant: &str, format: VectorFormat) -> PathBuf {
        crate::spec_root().join("goldens/figures").join(format!(
            "{}.{variant}.{}",
            self.name,
            format.extension()
        ))
    }
}

/// The save options of the vector goldens: the case's resolution and the
/// default metadata without creator, producer and date (the spec's
/// `VECTOR_METADATA`).
fn golden_options(case: &FigureCase) -> SaveOptions {
    SaveOptions {
        dpi: Some(case.dpi),
        metadata: Some(Metadata {
            creator: None,
            producer: None,
            date: None,
            ..Metadata::default()
        }),
        ..SaveOptions::default()
    }
}

/// Renders a case as `format` the way the spec saves its goldens: at the
/// case's resolution, without metadata, with the fixed SVG id salt and
/// the rcParams `rc` on top of the case's. PGF pictures are saved to a
/// file, beside which their images go.
pub fn render_vector_with(
    case: &FigureCase,
    format: VectorFormat,
    rc: &[(&str, &str)],
) -> Result<Vec<u8>, Box<dyn Error>> {
    case.with_figure(|fig| {
        plt::rc::set("svg.hashsalt", "plt-rs")?;
        for (key, value) in rc {
            plt::rc::set(key, value)?;
        }
        if format == VectorFormat::Pgf {
            return saved_pgf(fig, case);
        }
        Ok(fig.to_format(format.save_format(), golden_options(case))?)
    })
}

/// The PGF picture of `fig` saved to a scratch file named after `case`.
fn saved_pgf(fig: &plt::figure::Figure, case: &FigureCase) -> Result<Vec<u8>, Box<dyn Error>> {
    let directory = ScratchDir::new("pgf")?;
    let path = directory.path(&format!("{}.pgf", case.name.replace('/', "__")));
    fig.savefig_with(&path, golden_options(case))?;
    Ok(std::fs::read(&path)?)
}

/// Renders a case as `format` the way the spec saves its goldens.
pub fn render_vector(case: &FigureCase, format: VectorFormat) -> Result<Vec<u8>, Box<dyn Error>> {
    render_vector_with(case, format, &[])
}

/// Collects `cases` into one PDF document, one page each, the way the
/// spec writes `vectors/pages.pdf`: titled "Pages", without other
/// metadata, a note attached to the second page.
pub fn render_pdf_pages(cases: &[FigureCase]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pages = PdfPages::with_metadata(Metadata {
        title: Some("Pages".to_owned()),
        ..Metadata::none()
    })?;
    for (page, case) in cases.iter().enumerate() {
        if page == 1 {
            pages.attach_note("the second page");
        }
        case.with_figure(|fig| Ok(pages.savefig_with(fig, golden_options(case))?))?;
    }
    Ok(pages.finish()?)
}

/// Compares a rendering with the golden at `golden` in `format`.
pub fn compare_documents(
    format: VectorFormat,
    golden: &Path,
    rendered: Result<Vec<u8>, Box<dyn Error>>,
) -> Result<VectorReport, String> {
    let golden = std::fs::read(golden).map_err(|e| format!("cannot read golden: {e}"))?;
    let rendered = rendered.map_err(|e| format!("render failed: {e}"))?;
    let expected = format
        .items(&golden)
        .map_err(|e| format!("cannot read golden: {e}"))?;
    let actual = format
        .items(&rendered)
        .map_err(|e| format!("cannot read rendering: {e}"))?;
    Ok(compare_items(&expected, &actual))
}

/// Renders a case and compares it with its golden.
pub fn vector_report(case: &FigureCase, format: VectorFormat) -> Result<VectorReport, String> {
    compare_documents(
        format,
        &case.vector_path(format),
        render_vector(case, format),
    )
}
