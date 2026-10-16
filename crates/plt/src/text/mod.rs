//! Text artists (matplotlib.text): measuring, laying out and drawing
//! strings, with mathtext for the lines that contain `$...$` math.
//!
//! [`Text`] is the artist handle. Code drawing text without an artist
//! (tick labels, legends) uses [`lay_out`] and [`draw_laid_out`], which
//! apply the same rules: lines are split at `\n`, each line with an even
//! number of unescaped `$` is typeset by mathtext with the `mathtext.*`
//! settings, and the block is stacked, aligned and rotated like
//! matplotlib's `Text._get_layout`.

mod layout;
mod math;
mod wrap;

use std::rc::Rc;

use plt_core::{Bbox, Point};
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::rc::RcParams;
use crate::transforms::Transform;

pub use layout::{LaidOutText, measure_text};
pub(crate) use layout::{lay_out, lay_out_with_math};
pub use plt_text::{
    BlockLayout, HAlign as HorizontalAlignment, LineMetrics, LinePlacement, LineSpacing,
    RotationMode as TextRotationMode, TextError, VAlign as VerticalAlignment,
};

/// State of a text artist (matplotlib's `Text`).
#[derive(Debug)]
pub(crate) struct TextArtist {
    base: ArtistBase,
    pub(crate) text: String,
    pub(crate) position: Point,
    pub(crate) color: Color,
    pub(crate) font: FontProperties,
    pub(crate) rotation: f64,
    pub(crate) halign: HAlign,
    pub(crate) valign: VAlign,
    /// Alignment of lines within the block; `None` follows `halign`.
    pub(crate) multialign: Option<HAlign>,
    pub(crate) rotation_mode: RotationMode,
    pub(crate) linespacing: LineSpacing,
    pub(crate) antialiased: bool,
    /// Whether lines are wrapped at the figure edge.
    pub(crate) wrap: bool,
    /// Whether `$...$` is typeset as math.
    pub(crate) parse_math: bool,
    rc: Rc<RcParams>,
}

impl TextArtist {
    /// Text at `position` with matplotlib's defaults under `rc`
    /// (baseline/left aligned, z-order 3).
    pub(crate) fn new(position: Point, text: impl Into<String>, rc: Rc<RcParams>) -> Self {
        Self {
            base: ArtistBase::with_zorder(3.0),
            text: text.into(),
            position,
            color: rc.text_color.clone(),
            font: FontProperties::from_rc(&rc),
            rotation: 0.0,
            halign: HAlign::Left,
            valign: VAlign::Baseline,
            multialign: None,
            rotation_mode: RotationMode::Default,
            linespacing: LineSpacing::Normal,
            antialiased: rc.text_antialiased,
            wrap: false,
            parse_math: rc.text_parse_math,
            rc,
        }
    }

    /// Alignment and rotation of the block. Wrapped text always aligns
    /// before rotating (matplotlib switches it to the `anchor` rotation
    /// mode when wrapping).
    pub(crate) fn block_style(&self) -> BlockStyle {
        BlockStyle {
            rotation: self.rotation.rem_euclid(360.0),
            halign: self.halign,
            valign: self.valign,
            multialign: self.multialign.unwrap_or(self.halign),
            rotation_mode: if self.wrap {
                RotationMode::Anchor
            } else {
                self.rotation_mode
            },
            linespacing: self.linespacing,
        }
    }

    fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// The anchor in display coordinates, if it is finite.
    fn anchor(&self, ctx: LayoutContext<'_>) -> Option<Point> {
        let anchor = ctx
            .resolve(&self.transform())?
            .transform_point(self.position);
        anchor.is_finite().then_some(anchor)
    }

    /// The extent of the (sub)figure containing the text.
    fn figure_box(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        let mut id = self.base.parent?;
        loop {
            if let Some(extent) = ctx.frame_extent(id) {
                return Some(extent);
            }
            id = ctx.store().get(id)?.base().parent?;
        }
    }

    /// The text with line breaks inserted so that it fits in its figure,
    /// if wrapping is on (matplotlib's `_get_wrapped_text`).
    pub(crate) fn wrapped_text(&self, ctx: LayoutContext<'_>) -> Result<String, TextError> {
        if !self.wrap {
            return Ok(self.text.clone());
        }
        let (Some(anchor), Some(figure)) = (self.anchor(ctx), self.figure_box(ctx)) else {
            return Ok(self.text.clone());
        };
        let style = self.block_style();
        let width = wrap::wrap_width(anchor, &figure, style.rotation, style.halign);
        wrap::wrap_text(&self.text, width, |line| {
            let math = plt_text::mathtext::is_math_text(line);
            measure_text(line, &self.font, math, ctx.dpi(), &self.rc).map(|m| m.width.ceil())
        })
    }

    /// Lays the (wrapped) text out for the canvas resolution.
    pub(crate) fn layout(&self, ctx: LayoutContext<'_>) -> Result<LaidOutText, TextError> {
        lay_out_with_math(
            &self.wrapped_text(ctx)?,
            &self.font,
            &self.block_style(),
            ctx.dpi(),
            &self.rc,
            self.parse_math,
        )
    }
}

impl Artist for TextArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if self.text.is_empty() {
            return;
        }
        let Some(anchor) = self.anchor(*ctx.layout()) else {
            return;
        };
        let laid_out = match self.layout(*ctx.layout()) {
            Ok(laid_out) => laid_out,
            Err(err) => return ctx.report(self.base.id(), err),
        };
        let color = match self.color.resolve_alpha(self.base.alpha) {
            Ok(color) => color,
            Err(err) => return ctx.report(self.base.id(), err),
        };
        let mut gc = ctx.base_gc(&self.base);
        gc.foreground = color;
        gc.antialiased = self.antialiased;
        draw_laid_out(ctx, &gc, &laid_out, anchor, self.block_style().rotation);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let anchor = self.anchor(*ctx)?;
        if self.text.is_empty() {
            return Some(Bbox::from_bounds(anchor.x, anchor.y, 0.0, 0.0));
        }
        let laid_out = self.layout(*ctx).ok()?;
        Some(laid_out.block.bbox.translated(anchor.x, anchor.y))
    }
}

/// Draws every line of a laid-out text block anchored at `anchor`
/// (display coordinates), rotated by `angle` degrees.
pub(crate) fn draw_laid_out(
    ctx: &mut DrawContext<'_>,
    gc: &plt_render::GraphicsContext,
    laid_out: &LaidOutText,
    anchor: Point,
    angle: f64,
) {
    for (line, run) in laid_out.block.lines.iter().zip(&laid_out.runs) {
        ctx.renderer()
            .draw_text(gc, run, anchor + line.origin, angle);
    }
}

/// A handle to a text artist (matplotlib's `Text`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    raw: RawHandle,
}

impl ArtistHandle for Text {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl Text {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Sets the font size in points (sizes below 1 pt become 1 pt).
    pub fn set_fontsize(&self, points: f64) -> &Self {
        self.set_size(FontSize::Points(points))
    }

    /// Sets the font size from a keyword relative to `font.size`.
    pub fn set_fontsize_relative(&self, size: crate::fonts::RelativeSize) -> &Self {
        self.set_size(FontSize::Relative(size))
    }

    /// Sets the font size in points or relative to `font.size`
    /// (matplotlib's `set_fontsize`).
    pub fn set_size(&self, size: FontSize) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| {
            let base = t.rc.font_size;
            t.font.set_size(size, base);
        });
        self
    }

    /// Sets the fonts of math text (matplotlib's `set_math_fontfamily`).
    pub fn set_math_fontfamily(&self, fontset: crate::fonts::Fontset) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.math_fontfamily = fontset);
        self
    }

    /// The layout matplotlib's `Text._get_layout` computes: the (wrapped)
    /// lines, their placement relative to the anchor and the bounds of the
    /// rotated block. Fails for malformed math.
    pub fn layout(&self) -> Result<LaidOutText, TextError> {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<TextArtist, _>(|t| t.layout(LayoutContext::new(&state)))
    }

    /// The text with the line breaks wrapping adds (the text itself when
    /// wrapping is off; matplotlib's `_get_wrapped_text`).
    pub fn wrapped_text(&self) -> Result<String, TextError> {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<TextArtist, _>(|t| t.wrapped_text(LayoutContext::new(&state)))
    }

    /// Sets the font families (names or generic families).
    pub fn set_family<I: IntoIterator<Item = S>, S: Into<String>>(&self, families: I) -> &Self {
        let families: Vec<String> = families.into_iter().map(Into::into).collect();
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.families = families);
        self
    }

    /// Sets the font weight.
    pub fn set_weight(&self, weight: crate::fonts::FontWeight) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.weight = weight);
        self
    }
}

artist_properties! {
    Text => TextArtist {
        /// The string shown (lines separated by `\n`).
        text, set_text: String;
        /// Position in the text's coordinate system.
        position, set_position: Point;
        /// Text colour.
        color, set_color: Color;
        /// Counter-clockwise rotation in degrees.
        rotation, set_rotation: f64;
        /// Horizontal alignment relative to the position.
        halign, set_halign: HAlign;
        /// Vertical alignment relative to the position.
        valign, set_valign: VAlign;
        /// Alignment of lines within a multi-line block (`None` follows `halign`).
        multialign, set_multialign: Option<HAlign>;
        /// How rotation and alignment combine.
        rotation_mode, set_rotation_mode: RotationMode;
        /// Spacing between lines.
        linespacing, set_linespacing: LineSpacing;
        /// Font description.
        font, set_font: FontProperties;
        /// Anti-aliasing.
        antialiased, set_antialiased: bool;
        /// Whether lines are broken to fit in the figure.
        wrap, set_wrap: bool;
        /// Whether `$...$` is typeset as math (`text.parse_math`).
        parse_math, set_parse_math: bool;
    }
}
