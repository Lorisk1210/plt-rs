//! The PostScript backend: matplotlib's `RendererPS`, writing PostScript
//! (centred on a page of `ps.papersize`) or Encapsulated PostScript (sized
//! to the figure).
//!
//! Every drawing is wrapped in `gsave`/`grestore`; line properties and
//! the stroke colour are only written when they change. Markers and large
//! path collections become procedures, clip paths procedures reused while
//! the same clip comes back, hatches pattern dictionaries, images hex
//! `colorimage` data and Gouraud triangles type 4 shadings. Fonts are
//! embedded in the prolog as Type 3 programs or, with `ps.fonttype = 42`,
//! as Type 42 fonts over TrueType subsets. With `ps.useafm`, text that is
//! not mathtext is shown in standard PostScript fonts placed by their AFM
//! metrics instead, and no font is embedded at all (see `afm`).
//! PostScript has no transparency: partially transparent colours are drawn
//! opaque.
//!
//! Not supported: distillers (`ps.usedistiller`) and `usetex`.

mod afm;
mod draw;
mod fonts;
mod page;
mod text;
mod type42;

use std::fmt::Write as _;

use plt_core::path::convert::SimplifySettings;
use plt_core::{Affine2D, Path, Point, Rgba};
use plt_text::{AfmDirectory, TextRun};

use crate::collection::PathCollection;
use crate::gc::{CapStyle, ClipPath, GraphicsContext, JoinStyle};
use crate::image::RgbaImage;
use crate::renderer::Renderer;
pub use page::{PageOrientation, PageSetupError, Paper};

use crate::vector::{
    CharacterTracker, FontCache, FontType, Metadata, Problems, VectorError, check_canvas,
};

/// Settings of a PostScript document.
#[derive(Clone, Debug, PartialEq)]
pub struct PsOptions {
    /// Encapsulated PostScript: no page, sized to the figure.
    pub eps: bool,
    /// `ps.papersize` (ignored for EPS).
    pub paper: Paper,
    /// `savefig.orientation`.
    pub orientation: PageOrientation,
    /// Title, creator and creation date (DSC comments).
    pub metadata: Metadata,
    /// `path.simplify` and `path.simplify_threshold`.
    pub simplify: SimplifySettings,
    /// `image.composite_image`.
    pub composite_images: bool,
    /// `ps.fonttype`: how TrueType fonts are embedded.
    pub fonttype: FontType,
    /// `ps.useafm`: text that is not mathtext is drawn in standard
    /// PostScript fonts, measured with their AFM metrics, and no font is
    /// embedded.
    pub afm: bool,
}

impl Default for PsOptions {
    fn default() -> Self {
        Self {
            eps: false,
            paper: Paper::from_name("letter").unwrap_or(Paper::Figure),
            orientation: PageOrientation::Portrait,
            metadata: Metadata::default(),
            simplify: SimplifySettings::default(),
            composite_images: true,
            fonttype: FontType::Type3,
            afm: false,
        }
    }
}

impl PsOptions {
    /// The AFM fonts text that is not mathtext is drawn in, and must be
    /// measured with, if any (`RendererPS._afm_font_dir` when `ps.useafm`
    /// is set).
    ///
    /// matplotlib reads these metrics from `mpl-data/fonts/afm` (the 35
    /// standard PostScript fonts); the notices of those files do not
    /// permit redistribution, so plt-rs uses the metrics of the 14 PDF
    /// core fonts, which describe the same Courier, Helvetica, Times,
    /// Symbol and `ZapfDingbats` fonts.
    pub fn afm_directory(&self) -> Option<AfmDirectory> {
        self.afm.then_some(AfmDirectory::PdfCore)
    }
}

/// The line properties last written (`None` until first set).
#[derive(Clone, Debug, Default, PartialEq)]
struct LineState {
    color: Option<[f64; 3]>,
    width: Option<f64>,
    join: Option<JoinStyle>,
    cap: Option<CapStyle>,
    dash: Option<(f64, Option<Vec<f64>>)>,
}

/// Renders into a PostScript document exactly like matplotlib's
/// `RendererPS`. Coordinates are points with the origin at the bottom
/// left.
#[derive(Debug)]
pub struct PsRenderer {
    /// Figure size in inches.
    width: f64,
    height: f64,
    image_dpi: f64,
    body: String,
    line: LineState,
    hatches: Vec<String>,
    clip_paths: Vec<ClipPath>,
    path_collection_id: usize,
    tracker: CharacterTracker,
    fonts: FontCache,
    options: PsOptions,
    problems: Problems,
}

impl PsRenderer {
    /// A document for a figure of `width × height` inches whose images are
    /// rasterized at `image_dpi`.
    pub fn new(
        width: f64,
        height: f64,
        image_dpi: f64,
        options: PsOptions,
    ) -> Result<Self, VectorError> {
        check_canvas(width, height, image_dpi)?;
        Ok(Self {
            width,
            height,
            image_dpi,
            body: String::new(),
            line: LineState::default(),
            hatches: Vec::new(),
            clip_paths: Vec::new(),
            path_collection_id: 0,
            tracker: CharacterTracker::new(options.fonttype.subset_size()),
            fonts: FontCache::default(),
            options,
            problems: Problems::default(),
        })
    }

    /// The document: header comments, the prolog with the procedures and
    /// fonts, then the drawing placed on the page (matplotlib's
    /// `_print_figure`).
    ///
    /// # Errors
    ///
    /// When a font met while drawing could not be embedded.
    pub fn finish(mut self) -> Result<String, VectorError> {
        let (width, height) = (self.width, self.height);
        let (paper_width, paper_height) = match (self.options.eps, self.options.paper) {
            (false, Paper::Named { width, height, .. }) => match self.options.orientation {
                PageOrientation::Portrait => (width, height),
                PageOrientation::Landscape => (height, width),
            },
            _ => (width, height),
        };
        let (mut xo, mut yo) = (
            72.0 * 0.5 * (paper_width - width),
            72.0 * 0.5 * (paper_height - height),
        );
        let (mut llx, mut lly) = (xo, yo);
        let (mut urx, mut ury) = (llx + width * 72.0, lly + height * 72.0);
        let landscape = self.options.orientation == PageOrientation::Landscape;
        if landscape {
            (llx, lly, urx, ury) = (lly, llx, ury, urx);
            (xo, yo) = (72.0 * paper_height - yo, xo);
        }
        let mut out = String::new();
        if self.options.eps {
            out.push_str("%!PS-Adobe-3.0 EPSF-3.0\n");
        } else {
            out.push_str("%!PS-Adobe-3.0\n");
            if let Paper::Named { name, .. } = self.options.paper {
                let _ = writeln!(out, "%%DocumentPaperSizes: {name}");
            }
            out.push_str("%%Pages: 1\n");
        }
        out.push_str("%%LanguageLevel: 3\n");
        let metadata = &self.options.metadata;
        let ascii = |text: &str| -> String {
            text.chars()
                .map(|c| if c.is_ascii() { c } else { '?' })
                .collect()
        };
        if let Some(title) = &metadata.title {
            let _ = writeln!(out, "%%Title: {}", ascii(title));
        }
        if let Some(creator) = &metadata.creator {
            let _ = writeln!(out, "%%Creator: {creator}");
        }
        if let Some(date) = metadata.date {
            let _ = writeln!(out, "%%CreationDate: {}", date.ctime());
        }
        let _ = writeln!(
            out,
            "%%Orientation: {}\n\
             %%BoundingBox: {} {} {} {}\n\
             %%HiResBoundingBox: {llx:.6} {lly:.6} {urx:.6} {ury:.6}\n\
             %%EndComments",
            self.options.orientation.name(),
            llx.trunc() as i64,
            lly.trunc() as i64,
            urx.ceil() as i64,
            ury.ceil() as i64
        );
        // With `ps.useafm` matplotlib embeds no font, not even the fonts
        // mathtext was drawn with.
        let fonts: Vec<_> = if self.options.afm {
            Vec::new()
        } else {
            self.tracker
                .fonts()
                .map(|(id, subsets)| (id, subsets.to_vec()))
                .collect()
        };
        let dictionary_size = PS_DEFS.len() + fonts.iter().map(|(_, s)| s.len()).sum::<usize>();
        let _ = writeln!(
            out,
            "%%BeginProlog\n/mpldict {dictionary_size} dict def\nmpldict begin\n{}",
            PS_DEFS.join("\n")
        );
        for (id, subsets) in fonts {
            let font = self.fonts.get(id)?;
            for (index, subset) in subsets.iter().enumerate().filter(|(_, s)| !s.is_empty()) {
                match self.options.fonttype {
                    FontType::Type3 => out.push_str(&fonts::type3_font(font, index, subset)),
                    FontType::Type42 => out.push_str(&type42::type42_font(font, index, subset)?),
                }
            }
        }
        out.push_str("end\n%%EndProlog\n");
        if !self.options.eps {
            out.push_str("%%Page: 1 1\n");
        }
        let _ = writeln!(out, "mpldict begin\n{} translate", draw::nums(&[xo, yo]));
        if landscape {
            out.push_str("90 rotate\n");
        }
        let _ = writeln!(
            out,
            "0 0 {} rectclip\n{}\nend\nshowpage",
            draw::nums(&[width * 72.0, height * 72.0]),
            self.body
        );
        if !self.options.eps {
            out.push_str("%%EOF\n");
        }
        self.problems.into_result(out)
    }
}

/// The procedures of `mpldict` (`_psDefs`).
const PS_DEFS: [&str; 8] = [
    "/_d { bind def } bind def",
    "/m { moveto } _d",
    "/l { lineto } _d",
    "/r { rlineto } _d",
    "/c { curveto } _d",
    "/cl { closepath } _d",
    "/ce { closepath eofill } _d",
    "/sc { setcachedevice } _d",
];

impl Renderer for PsRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        (self.width * 72.0, self.height * 72.0)
    }

    /// The image resolution, as matplotlib's `MixedModeRenderer.dpi`.
    fn dpi(&self) -> f64 {
        self.image_dpi
    }

    /// Lengths are points already.
    fn points_to_pixels(&self, points: f64) -> f64 {
        points
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.ps_path(gc, path, *transform, fill);
    }

    fn draw_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.ps_markers(gc, marker, *marker_transform, path, *transform, fill);
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        self.ps_path_collection(gc, collection);
    }

    fn draw_gouraud_triangles(
        &mut self,
        _gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        self.ps_gouraud_triangles(triangles, colors, *transform);
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.ps_image(gc, origin, image, transform.copied());
    }

    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.ps_text(gc, run, origin, angle);
    }

    fn option_image_nocomposite(&self) -> bool {
        !self.options.composite_images
    }

    fn option_scale_image(&self) -> bool {
        true
    }

    fn image_magnification(&self) -> f64 {
        self.image_dpi / 72.0
    }
}

#[cfg(test)]
mod tests;
