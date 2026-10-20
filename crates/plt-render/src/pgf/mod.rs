//! The PGF backend: matplotlib's `RendererPgf`, which writes a figure as
//! the drawing commands of a LaTeX `pgfpicture` environment.
//!
//! Paths become `\pgfpath…` commands in inches, markers are defined once
//! per draw call as a PGF object and placed at every vertex, hatches are
//! tiled one-inch pattern objects, images are written as PNG files beside
//! the document and included with `\includegraphics`, and text is left to
//! LaTeX: each line becomes a `\pgftext` with font commands for its
//! family, size, style and weight, anchored where and as matplotlib
//! anchors it. Mathtext is passed through for LaTeX to typeset.
//!
//! matplotlib measures text with LaTeX while laying the figure out; plt-rs
//! measures it with its own (Agg) text engine, so text positions follow
//! the bundled fonts' metrics.

mod path;
mod text;

use std::fmt::Write as _;

use plt_core::path::convert::SimplifySettings;
use plt_core::{Affine2D, Path, Point, Rgba};
use plt_text::TextRun;

use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::renderer::Renderer;
use crate::svg::ImageFile;
use crate::vector::{Problems, VectorError, check_canvas, fixed};

/// LaTeX points per inch (TeX's `pt`), as opposed to PostScript points.
const LATEX_POINTS_PER_INCH: f64 = 72.27;
/// matplotlib's points per inch.
const POINTS_PER_INCH: f64 = 72.0;

/// Settings of a PGF document (matplotlib's `pgf.*` rcParams and the
/// file it is written to).
#[derive(Clone, Debug, PartialEq)]
pub struct PgfOptions {
    /// `path.simplify` and `path.simplify_threshold`.
    pub simplify: SimplifySettings,
    /// `image.composite_image`.
    pub composite_images: bool,
    /// `pgf.rcfonts`: select fonts by name through `fontspec`; otherwise
    /// families other than the generic ones use LaTeX's default.
    pub rcfonts: bool,
    /// The family names of the installed fonts, which `pgf.rcfonts`
    /// selects by name (matplotlib's `fontManager.ttflist`).
    pub font_names: Vec<String>,
    /// The LaTeX preamble the figure assumes, recorded in the header
    /// (matplotlib's `_get_preamble()`).
    pub preamble: String,
    /// The stem of the document's file name: images go beside it as
    /// `<stem>-img<N>.png`. Without one, images cannot be written.
    pub image_stem: Option<String>,
}

impl Default for PgfOptions {
    fn default() -> Self {
        Self {
            simplify: SimplifySettings::default(),
            composite_images: true,
            rcfonts: true,
            font_names: Vec::new(),
            preamble: String::new(),
            image_stem: None,
        }
    }
}

/// A finished PGF document and the images to write beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PgfDocument {
    /// The `pgfpicture` and its header.
    pub text: String,
    /// The PNG files the picture includes, in drawing order.
    pub images: Vec<ImageFile>,
}

/// Renders into a PGF picture exactly like matplotlib's `RendererPgf`.
///
/// Coordinates are display pixels at the figure's resolution, written in
/// inches.
#[derive(Debug)]
pub struct PgfRenderer {
    width: f64,
    height: f64,
    dpi: f64,
    options: PgfOptions,
    body: String,
    images: Vec<ImageFile>,
    problems: Problems,
}

impl PgfRenderer {
    /// A picture of `width × height` inches drawn at `dpi`.
    ///
    /// # Errors
    ///
    /// When the size or resolution is not positive and finite.
    pub fn new(
        width: f64,
        height: f64,
        dpi: f64,
        options: PgfOptions,
    ) -> Result<Self, VectorError> {
        check_canvas(width, height, dpi)?;
        Ok(Self {
            width: width * dpi,
            height: height * dpi,
            dpi,
            options,
            body: String::new(),
            images: Vec::new(),
            problems: Problems::default(),
        })
    }

    /// The document (matplotlib's `_print_pgf_to_fh`).
    ///
    /// # Errors
    ///
    /// When an image could not be encoded or written.
    pub fn finish(self) -> Result<PgfDocument, VectorError> {
        let mut text = String::from(HEADER);
        text.push_str("%% plt-rs used the following preamble\n");
        for line in self.options.preamble.lines() {
            let _ = writeln!(text, "%%   {line}");
        }
        text.push_str("%%\n");
        writeln_pgf(&mut text, r"\begingroup");
        writeln_pgf(&mut text, r"\makeatletter");
        writeln_pgf(&mut text, r"\begin{pgfpicture}");
        writeln_pgf(
            &mut text,
            &format!(
                r"\pgfpathrectangle{{\pgfpointorigin}}{{\pgfqpoint{{{}in}}{{{}in}}}}",
                num(self.width / self.dpi),
                num(self.height / self.dpi)
            ),
        );
        writeln_pgf(&mut text, r"\pgfusepath{use as bounding box, clip}");
        text.push_str(&self.body);
        writeln_pgf(&mut text, r"\end{pgfpicture}");
        writeln_pgf(&mut text, r"\makeatother");
        writeln_pgf(&mut text, r"\endgroup");
        self.problems.into_result(PgfDocument {
            text,
            images: self.images,
        })
    }

    /// Writes one line of the picture.
    fn line(&mut self, line: &str) {
        writeln_pgf(&mut self.body, line);
    }

    /// Display pixels to inches.
    fn inches(&self, pixels: f64) -> f64 {
        pixels / self.dpi
    }
}

/// The header comment of every document.
const HEADER: &str = r"%% Creator: plt-rs, PGF backend
%%
%% To include the figure in your LaTeX document, write
%%   \input{<filename>.pgf}
%%
%% Make sure the required packages are loaded in your preamble
%%   \usepackage{pgf}
%%
%% Also ensure that all the required font packages are loaded; for instance,
%% the lmodern package is sometimes necessary when using math font.
%%   \usepackage{lmodern}
%%
%% Figures using additional raster images can only be included by \input if
%% they are in the same directory as the main LaTeX file. For loading figures
%% from other directories you can use the `import` package
%%   \usepackage{import}
%%
%% and then include the figures with
%%   \import{<path to file>}{<filename>.pgf}
%%
";

/// Writes `line` ending in `%`, which keeps TeX from inserting spurious
/// spaces (`_writeln`).
fn writeln_pgf(out: &mut String, line: &str) {
    out.push_str(line);
    out.push_str("%\n");
}

/// A number as `%f` writes it: six decimals (TeX knows no scientific
/// notation).
fn num(value: f64) -> String {
    fixed(value, 6)
}

/// A length in matplotlib points as TeX points (`pt`), as matplotlib
/// converts line widths.
fn latex_points(points: f64) -> f64 {
    points / POINTS_PER_INCH * LATEX_POINTS_PER_INCH
}

impl Renderer for PgfRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    fn dpi(&self) -> f64 {
        self.dpi
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.pgf_path(gc, path, transform, fill);
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
        self.pgf_markers(gc, marker, marker_transform, path, transform, fill);
    }

    fn draw_gouraud_triangles(
        &mut self,
        _gc: &GraphicsContext,
        _triangles: &[[Point; 3]],
        _colors: &[[Rgba; 3]],
        _transform: &Affine2D,
    ) {
        // matplotlib's `RendererBase.draw_gouraud_triangles` raises.
        self.problems.record::<()>(Err(VectorError::Unsupported(
            "PGF cannot draw Gouraud-shaded triangles",
        )));
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.pgf_image(gc, origin, image, transform);
    }

    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.pgf_text(gc, run, origin, angle);
    }

    fn option_image_nocomposite(&self) -> bool {
        !self.options.composite_images
    }

    fn option_scale_image(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_hold_one_clipped_picture() {
        let renderer = PgfRenderer::new(2.0, 1.0, 100.0, PgfOptions::default()).unwrap();
        let document = renderer.finish().unwrap();
        assert!(
            document
                .text
                .starts_with("%% Creator: plt-rs, PGF backend\n")
        );
        assert!(document.text.contains(
            "\\begin{pgfpicture}%\n\\pgfpathrectangle{\\pgfpointorigin}{\\pgfqpoint{2.000000in}{1.000000in}}%\n\\pgfusepath{use as bounding box, clip}%\n\\end{pgfpicture}%\n"
        ));
        assert!(document.text.ends_with("\\makeatother%\n\\endgroup%\n"));
        assert_eq!(document.images, []);
    }

    #[test]
    fn numbers_and_widths() {
        assert_eq!(num(1.0 / 3.0), "0.333333");
        assert_eq!(num(-0.0), "-0.000000");
        assert_eq!(num(1e20), "100000000000000000000.000000");
        assert!((latex_points(72.0) - 72.27).abs() < 1e-12);
    }

    #[test]
    fn gouraud_shading_is_an_error() {
        let mut renderer = PgfRenderer::new(1.0, 1.0, 72.0, PgfOptions::default()).unwrap();
        renderer.draw_gouraud_triangles(
            &GraphicsContext::default(),
            &[[Point::new(0.0, 0.0); 3]],
            &[[Rgba::BLACK; 3]],
            &Affine2D::IDENTITY,
        );
        assert!(matches!(
            renderer.finish(),
            Err(VectorError::Unsupported(_))
        ));
    }
}
