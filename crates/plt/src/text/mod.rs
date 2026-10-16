//! Text artists (matplotlib.text).

mod layout;

use std::rc::Rc;

use plt_core::{Bbox, Point};
use plt_text::{BlockStyle, HAlign, LineSpacing, RotationMode, VAlign};

use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::transforms::Transform;

pub(crate) use layout::{LaidOutText, lay_out};
pub use plt_text::{
    HAlign as HorizontalAlignment, RotationMode as TextRotationMode, VAlign as VerticalAlignment,
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
            rc,
        }
    }

    /// Alignment and rotation of the block.
    pub(crate) fn block_style(&self) -> BlockStyle {
        BlockStyle {
            rotation: self.rotation.rem_euclid(360.0),
            halign: self.halign,
            valign: self.valign,
            multialign: self.multialign.unwrap_or(self.halign),
            rotation_mode: self.rotation_mode,
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

    /// Lays the text out for the canvas resolution.
    pub(crate) fn layout(&self, dpi: f64) -> Result<LaidOutText, plt_text::TextError> {
        lay_out(&self.text, &self.font, &self.block_style(), dpi, &self.rc)
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
        let laid_out = match self.layout(ctx.layout().dpi()) {
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
        let laid_out = self.layout(ctx.dpi()).ok()?;
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

    /// Sets the font size in points.
    pub fn set_fontsize(&self, points: f64) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| t.font.size = points);
        self
    }

    /// Sets the font size from a keyword relative to `font.size`.
    pub fn set_fontsize_relative(&self, size: crate::fonts::RelativeSize) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.size = size.scale() * t.rc.font_size);
        self
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
    }
}
