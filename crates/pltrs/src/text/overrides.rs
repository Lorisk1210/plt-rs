//! Text measured and drawn by the program instead of by pltrs.
//!
//! matplotlib measures every line of text through one module function,
//! `matplotlib.text._get_text_metrics_with_cache`, and draws every text
//! (titles, tick and axis labels, legend and table texts, annotations)
//! through `Text.draw`, which hands TeX lines to the renderer's
//! `draw_tex`. Its tests replace them, so that texts become boxes whose
//! size depends only on the font size and the number of characters (the
//! `text_placeholders` fixture), or to see where TeX lines are drawn.
//! pltrs (Python) hands such replacements to pltrs (Rust) for the thread
//! (for the threads sharing a [state](pltrs_core::state)),
//! as a [`LineMeasurer`], a [`TextPainter`] and a [`TexPainter`]: while a
//! measurer is set, every line pltrs lays out is measured by it, while a
//! painter is set, every text pltrs draws is drawn by it instead, and
//! while a TeX painter is set, every TeX line. A measurer or painter
//! standing for a replacement the program may take back says whether it
//! is in force each time it would be used ([`LineMeasurer::in_force`]).
//! Without them, laying out a line or drawing a text costs one
//! thread-local read more.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::thread::LocalKey;

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::GraphicsContext;
use pltrs_text::{AfmDirectory, FontHeightMetrics, LineMetrics, TextError};

use super::layout::{LaidOutText, Typesetting};
use super::{Text, TextArtist};
use crate::artist::{Artist, ArtistHandle, DrawContext, RawHandle};
use crate::colors::Color;
use crate::figure::Figure;
use crate::fonts::FontProperties;
use crate::transforms::Transform;

/// How a line is typeset, as matplotlib's `get_text_width_height_descent`
/// receives it (its `ismath`: `False`, `True` or `"TeX"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// Plain text.
    Plain,
    /// Text with `$...$` math, typeset by mathtext.
    Math,
    /// Text typeset by TeX (`usetex`).
    Tex,
}

/// Measures lines of text in place of pltrs (matplotlib's
/// `_get_text_metrics_with_cache`, replaced).
pub trait LineMeasurer {
    /// The width, height and descent of `line` set in `font` as `kind`,
    /// in pixels at `dpi`; `None` leaves it to pltrs.
    fn measure(
        &self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: f64,
    ) -> Option<LineMetrics>;

    /// The minimum ascent, descent and line gap of the lines of `font` at
    /// `dpi`, read from the font's tables; `None` when the tables cannot
    /// be read, and `"lp"` measured by [`measure`](Self::measure) gives
    /// the ascent and descent, without a gap (matplotlib's fallback in
    /// `Text._get_layout`).
    fn font_height(&self, font: &FontProperties, dpi: f64) -> Option<FontHeightMetrics>;

    /// Whether the measurer measures now; while it does not, pltrs
    /// measures as without one (the program put matplotlib's function
    /// back).
    fn in_force(&self) -> bool {
        true
    }
}

/// Draws texts in place of pltrs (matplotlib's `Text.draw`, replaced).
pub trait TextPainter {
    /// Draws the text `stand_in` stands for, drawn at `dpi`: the artists
    /// to draw in its place, where the text is being drawn. The stand-in
    /// is a text of its own, positioned in display coordinates where the
    /// text is drawn, with its lines, font, colour, alignment and
    /// rotation, in a figure at `dpi`.
    fn paint(&self, stand_in: &Text, dpi: f64) -> Vec<RawHandle>;

    /// Whether the painter draws now; while it does not, pltrs draws as
    /// without one (the program put matplotlib's `Text.draw` back).
    fn in_force(&self) -> bool {
        true
    }
}

/// A path the program drew in place of a TeX line: stroked as `gc` says
/// and filled with `fill`, mapped into display pixels by `transform`
/// (matplotlib's `draw_path(gc, path, transform, rgbFace)`).
#[derive(Clone, Debug)]
pub struct DrawnPath {
    /// How the path is stroked.
    pub gc: GraphicsContext,
    /// The path.
    pub path: Path,
    /// Its transform into display pixels.
    pub transform: Affine2D,
    /// Its fill.
    pub fill: Option<Rgba>,
}

/// Draws TeX lines in place of the renderer (matplotlib's `draw_tex`,
/// replaced).
pub trait TexPainter {
    /// Draws `line`, typeset by TeX in `font`, with the left end of its
    /// baseline at `origin` (display pixels) and rotated by `angle`
    /// degrees counter-clockwise, as `gc` says: the paths to draw in its
    /// place.
    fn paint(
        &self,
        gc: &GraphicsContext,
        line: &str,
        font: &FontProperties,
        origin: Point,
        angle: f64,
    ) -> Vec<DrawnPath>;
}

pltrs_core::thread_state! {
    /// The measurer of the thread's lines, if the program set one.
    static MEASURER: RefCell<Option<Rc<dyn LineMeasurer>>> = const { RefCell::new(None) };
    /// The painter of the thread's texts, if the program set one.
    static PAINTER: RefCell<Option<Rc<dyn TextPainter>>> = const { RefCell::new(None) };
    /// The painter of the thread's TeX lines, if the program set one.
    static TEX_PAINTER: RefCell<Option<Rc<dyn TexPainter>>> = const { RefCell::new(None) };
}

thread_local! {
    /// Whether the thread measures text itself for now, whatever measurer
    /// is set ([`without_measurer`]).
    static MEASURING_ITSELF: Cell<bool> = const { Cell::new(false) };
    /// Whether the thread draws texts itself for now, whatever painter is
    /// set (while it draws what the painter gave, see [`paint`]).
    static PAINTING_ITSELF: Cell<bool> = const { Cell::new(false) };
}

/// Sets (or, with `None`, removes) what measures the lines of text laid
/// out on this thread.
pub fn set_line_measurer(measurer: Option<Rc<dyn LineMeasurer>>) {
    MEASURER.set(measurer);
}

/// Sets (or, with `None`, removes) what draws the texts drawn on this
/// thread.
pub fn set_text_painter(painter: Option<Rc<dyn TextPainter>>) {
    PAINTER.set(painter);
}

/// Sets (or, with `None`, removes) what draws the TeX lines drawn on
/// this thread.
pub fn set_tex_painter(painter: Option<Rc<dyn TexPainter>>) {
    TEX_PAINTER.set(painter);
}

/// The thread's TeX painter.
pub(super) fn tex_painter() -> Option<Rc<dyn TexPainter>> {
    TEX_PAINTER.with_borrow(Clone::clone)
}

/// The thread's measurer if it is in force, unless the thread measures
/// text itself for now.
fn measurer() -> Option<Rc<dyn LineMeasurer>> {
    if MEASURING_ITSELF.get() {
        return None;
    }
    MEASURER
        .with_borrow(Clone::clone)
        .filter(|measurer| measurer.in_force())
}

/// Whether the program measures the lines laid out on this thread (a
/// [`LineMeasurer`] in force is set, and the thread does not measure text
/// itself for now).
pub(crate) fn has_line_measurer() -> bool {
    measurer().is_some()
}

/// The thread's painter if it is in force, unless the thread draws texts
/// itself for now.
pub(super) fn painter() -> Option<Rc<dyn TextPainter>> {
    if PAINTING_ITSELF.get() {
        return None;
    }
    PAINTER
        .with_borrow(Clone::clone)
        .filter(|painter| painter.in_force())
}

/// Has the thread measure text itself while `f` runs, whatever measurer
/// is set (for the thread alone: other threads sharing its state still
/// measure with the measurer).
pub(super) fn without_measurer<R>(f: impl FnOnce() -> R) -> R {
    itself(&MEASURING_ITSELF, f)
}

/// Runs `f` with the thread's `flag` set, restored however `f` ends.
fn itself<R>(flag: &'static LocalKey<Cell<bool>>, f: impl FnOnce() -> R) -> R {
    struct Restore(&'static LocalKey<Cell<bool>>, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            self.0.set(self.1);
        }
    }
    let _restore = Restore(flag, flag.replace(true));
    f()
}

/// The metrics of `line` as the thread's measurer gives them, if one is
/// set and gives them.
pub(super) fn measure(
    line: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
) -> Option<LineMetrics> {
    measurer()?.measure(line, font, kind, dpi)
}

/// The font height metrics of lines typeset by `typesetting` in `font`
/// at `dpi` when a measurer is set: those of the font's tables (never
/// for TeX), else the ascent and descent of `"lp"` it measures. `None`
/// without a measurer, or when it cannot measure `"lp"`.
pub(super) fn font_height(
    font: &FontProperties,
    typesetting: Typesetting,
    dpi: f64,
) -> Option<FontHeightMetrics> {
    let measurer = measurer()?;
    let kind = match typesetting {
        Typesetting::Tex => LineKind::Tex,
        Typesetting::Mathtext { .. } => {
            if let Some(tables) = measurer.font_height(font, dpi) {
                return Some(tables);
            }
            LineKind::Plain
        }
    };
    let lp = measurer.measure("lp", font, kind, dpi)?;
    Some(FontHeightMetrics {
        ascent: lp.height - lp.descent,
        descent: lp.descent,
        line_gap: 0.0,
    })
}

/// Draws `laid_out`, anchored at `anchor` in `color`, through `painter`:
/// the artists it gives are drawn where `ctx` draws, as pltrs draws them
/// (a text among them, which the program's `Text.draw` hands the original
/// `Text.draw`, without the painter again).
pub(super) fn paint(
    painter: &dyn TextPainter,
    ctx: &mut DrawContext<'_>,
    laid_out: &LaidOutText,
    anchor: Point,
    color: Rgba,
) {
    let dpi = ctx.layout().dpi();
    // The program's `Text.draw` finds the figure free.
    ctx.free();
    let drawn = painter.paint(&stand_in(laid_out, anchor, color, dpi), dpi);
    itself(&PAINTING_ITSELF, || {
        for artist in drawn {
            ctx.draw_handle(&artist);
        }
    });
}

/// A text of its own standing for `laid_out` drawn at `anchor` in
/// `color`: in display coordinates, in a figure at `dpi`.
fn stand_in(laid_out: &LaidOutText, anchor: Point, color: Rgba, dpi: f64) -> Text {
    let rc = crate::rc::params();
    let [width, height] = rc.figure_figsize;
    let mut text = TextArtist::new(anchor, laid_out.lines.join("\n"), Rc::clone(&rc));
    text.font = laid_out.font.clone();
    text.color = Color::from(color);
    let style = laid_out.style;
    text.rotation = style.rotation;
    text.halign = style.halign;
    text.valign = style.valign;
    text.multialign = Some(style.multialign);
    text.rotation_mode = style.rotation_mode;
    text.linespacing = style.linespacing;
    (text.parse_math, text.usetex) = match laid_out.typesetting {
        Typesetting::Mathtext { parse_math } => (parse_math, false),
        Typesetting::Tex => (false, true),
    };
    text.base_mut().transform = Some(Transform::Display);
    let figure = Figure::with_size(width, height, dpi);
    Text::from_raw(figure.add_custom_artist(text).raw().clone())
}

/// matplotlib's `get_text_width_height_descent(line, prop, ismath)` as
/// pltrs measures it, whatever measurer is set: the width, height and
/// descent of `line` typeset in `font` as `kind`, in pixels at `dpi`.
///
/// # Errors
///
/// When the math is malformed, no font is found or TeX fails.
pub fn measure_line(
    line: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
) -> Result<LineMetrics, TextError> {
    let rc = crate::rc::params();
    without_measurer(|| match kind {
        LineKind::Tex => super::tex::tex_run(line, font, dpi).map(|run| LineMetrics {
            width: run.width,
            height: run.height,
            descent: run.descent,
        }),
        LineKind::Plain | LineKind::Math => {
            super::measure_text(line, font, kind == LineKind::Math, dpi, &rc)
        }
    })
}

/// [`measure_line`] as the PDF backend measures the text it draws, at
/// `dpi` (72 for a document): with the AFM metrics of the PDF core fonts
/// while `pdf.use14corefonts` is set (matplotlib's
/// `RendererPdf.get_text_width_height_descent`).
///
/// # Errors
///
/// As [`measure_line`].
pub fn measure_line_pdf(
    line: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
) -> Result<LineMetrics, TextError> {
    let core_fonts = crate::rc::params()
        .pdf_use14corefonts
        .then_some(AfmDirectory::PdfCore);
    super::metrics::with_afm_metrics(core_fonts, || measure_line(line, font, kind, dpi))
}

/// [`measure_line`] as the PostScript backend measures the text it draws,
/// at `dpi` (72 for a document): with the AFM metrics of the standard
/// PostScript fonts while `ps.useafm` is set (matplotlib's
/// `RendererPS.get_text_width_height_descent`).
///
/// # Errors
///
/// As [`measure_line`].
pub fn measure_line_ps(
    line: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
) -> Result<LineMetrics, TextError> {
    let afm = crate::rc::params()
        .ps_useafm
        .then_some(AfmDirectory::PostScript);
    super::metrics::with_afm_metrics(afm, || measure_line(line, font, kind, dpi))
}

/// [`measure_line`] as the SVG backend measures the text it draws, at
/// `dpi` (matplotlib's `RendererSVG.get_text_width_height_descent`, that
/// of `TextToPath`).
///
/// # Errors
///
/// As [`measure_line`].
pub fn measure_line_svg(
    line: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
) -> Result<LineMetrics, TextError> {
    super::metrics::with_text_path_metrics(|| measure_line(line, font, kind, dpi))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use pltrs_text::{BlockStyle, LineSpacing};

    use super::*;
    use crate::patches::{PatchProps, Rectangle};
    use crate::rc::RcParams;

    /// matplotlib's placeholder measurement (`text_placeholders`): as tall
    /// as the font size, a golden-ratio fraction of it wide per character,
    /// no descent; the font tables cannot be read.
    struct Placeholders {
        calls: Cell<usize>,
    }

    impl LineMeasurer for Placeholders {
        fn measure(
            &self,
            line: &str,
            font: &FontProperties,
            _: LineKind,
            _: f64,
        ) -> Option<LineMetrics> {
            self.calls.set(self.calls.get() + 1);
            Some(LineMetrics {
                width: line.chars().count() as f64 * font.size / 1.618,
                height: font.size,
                descent: 0.0,
            })
        }

        fn font_height(&self, _: &FontProperties, _: f64) -> Option<FontHeightMetrics> {
            None
        }
    }

    /// Runs `f` with `measurer` set, removing it afterwards.
    fn with_measurer<R>(measurer: Rc<dyn LineMeasurer>, f: impl FnOnce() -> R) -> R {
        set_line_measurer(Some(measurer));
        let result = f();
        set_line_measurer(None);
        result
    }

    /// matplotlib's defaults.
    fn reference() -> RcParams {
        RcParams::matplotlib()
    }

    #[test]
    fn a_measurer_measures_every_line_and_the_font_height() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        let measurer = Rc::new(Placeholders {
            calls: Cell::new(0),
        });
        let lay_out = |linespacing| {
            let style = BlockStyle {
                linespacing,
                ..BlockStyle::default()
            };
            with_measurer(measurer.clone(), || {
                super::super::lay_out("ab\ncdef", &font, &style, 100.0, &rc)
            })
            .expect("laid out")
        };
        let normal = lay_out(LineSpacing::Normal);
        // Two lines and "lp" for the missing font tables.
        assert_eq!(measurer.calls.get(), 3);
        let size = font.size;
        assert_eq!(normal.block.lines[1].width, 4.0 * size / 1.618);
        // Lines as tall as the font size ("lp"), stacked without a gap.
        let bbox = normal.block.bbox;
        assert!((bbox.height() - 2.0 * size).abs() < 1e-9, "{bbox:?}");
        assert!((bbox.width() - 4.0 * size / 1.618).abs() < 1e-9);
        let spaced = lay_out(LineSpacing::Factor(1.2)).block.bbox;
        assert!((spaced.height() - 2.0 * 1.2 * size).abs() < 1e-9);
    }

    #[test]
    fn the_pdf_backend_measures_in_its_core_fonts_when_asked() {
        crate::rc::context(|| {
            let font = FontProperties::from_rc(&crate::rc::params());
            let measure = || measure_line_pdf("Wg", &font, LineKind::Plain, 72.0).expect("bundled");
            let embedded = measure();
            assert_eq!(
                embedded,
                measure_line("Wg", &font, LineKind::Plain, 72.0).expect("bundled")
            );
            crate::rc::set("pdf.use14corefonts", "True").expect("a valid rcParam");
            assert_ne!(measure(), embedded, "Helvetica's metrics");
        });
    }

    #[test]
    fn without_a_measurer_pltrs_measures() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        let own = measure_line("lp", &font, LineKind::Plain, 100.0).expect("bundled font");
        let measurer = Rc::new(Placeholders {
            calls: Cell::new(0),
        });
        let original = with_measurer(measurer.clone(), || {
            measure_line("lp", &font, LineKind::Plain, 100.0)
        })
        .expect("bundled font");
        assert_eq!(own, original, "measure_line ignores the measurer");
        assert_eq!(measurer.calls.get(), 0);
        let replaced = with_measurer(measurer, || {
            super::super::measure_text("lp", &font, false, 100.0, &rc)
        })
        .expect("measured");
        assert_eq!(replaced.descent, 0.0);
        assert_eq!(replaced.height, font.size);
    }

    /// Draws a black rectangle over the stand-in's extent, as the
    /// placeholder fixture's `Text.draw` does.
    struct Boxes;

    impl TextPainter for Boxes {
        fn paint(&self, stand_in: &Text, _: f64) -> Vec<RawHandle> {
            let Some(extent) = stand_in.window_extent() else {
                return Vec::new();
            };
            let props = PatchProps {
                facecolor: Some(Color::from("black")),
                ..PatchProps::default()
            };
            let rect = crate::patches::Patch::loose(
                Rectangle::new((extent.x0(), extent.y0()), extent.width(), extent.height()),
                props,
            );
            rect.set_transform(Transform::Display);
            vec![rect.raw().clone()]
        }
    }

    #[test]
    fn a_painter_draws_in_place_of_texts() {
        let fig = Figure::with_size(2.0, 1.0, 100.0);
        let text = fig.text(0.5, 0.5, "abc");
        text.set_fontsize(20.0);
        let plain = fig.render_raster().expect("drawn").to_image();
        set_text_painter(Some(Rc::new(Boxes)));
        let painted = fig.render_raster();
        set_text_painter(None);
        let painted = painted.expect("drawn").to_image();
        assert_ne!(plain, painted);
        // The box covers the text's extent with solid black.
        let extent = text.window_extent().expect("drawn text");
        let (x, y) = (extent.center().x, 100.0 - extent.center().y);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a pixel inside the 200 x 100 canvas"
        )]
        let pixel = painted.pixel(x as usize, y as usize);
        assert_eq!(pixel, Some([0, 0, 0, 255]));
    }

    /// Hands the stand-in back to be drawn, as a `Text.draw` of the
    /// program's calling the original does.
    struct Original;

    impl TextPainter for Original {
        fn paint(&self, stand_in: &Text, _: f64) -> Vec<RawHandle> {
            vec![stand_in.raw().clone()]
        }
    }

    #[test]
    fn texts_a_painter_gives_back_are_drawn_without_it() {
        let fig = Figure::with_size(2.0, 1.0, 100.0);
        fig.text(0.5, 0.5, "abc").set_fontsize(20.0);
        let plain = fig.render_raster().expect("drawn").to_image();
        set_text_painter(Some(Rc::new(Original)));
        let painted = fig.render_raster();
        set_text_painter(None);
        assert_eq!(painted.expect("drawn once").to_image(), plain);
    }
}
