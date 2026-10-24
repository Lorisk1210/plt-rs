//! Text artists (matplotlib.text): measuring, laying out and drawing
//! strings, with mathtext for the lines that contain `$...$` math.
//!
//! [`Text`] is the artist handle. Code drawing text without an artist
//! (tick labels, legends) uses the crate's `lay_out` and `draw_laid_out`, which
//! apply the same rules: lines are split at `\n`, each line with an even
//! number of unescaped `$` is typeset by mathtext with the `mathtext.*`
//! settings, and the block is stacked, aligned and rotated like
//! matplotlib's `Text._get_layout`.
//!
//! A text may draw a box behind itself ([`TextBoxProps`]) and, as an
//! [`Annotation`], an arrow to the point it annotates.
//!
//! A text drawn on a single line is shaped with its OpenType [`Feature`]s
//! and its [`Language`] (`text.language` unless set), in every backend
//! that shapes text; its measured extent ignores both, as matplotlib's
//! does.

mod annotation;
mod error;
mod handle;
mod layout;
mod math;
mod metrics;
mod text_box;
mod wrap;

use std::rc::Rc;

use plt_core::{Bbox, Point};
use plt_text::{BlockStyle, RotationMode, Shaping, TextAnchor, TextRun};

use crate::artist::{Artist, ArtistBase, DrawContext, LayoutContext};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::patches::ClipRegion;
use crate::patheffects::with_path_effects;
use crate::rc::RcParams;
use crate::transforms::Transform;

pub use annotation::{
    AnnotateOptions, Annotation, ArrowProps, ArrowStart, FancyArrowProps, LegacyArrowProps,
};
pub(crate) use annotation::{AnnotationArrow, AnnotationState, ArrowTarget};
pub use error::TextDrawError;
pub use handle::{Text, TextProps};
use layout::rc_language;
pub use layout::{LaidOutText, measure_text, text_path};
pub(crate) use layout::{lay_out, lay_out_with_math};
pub use math::math_style;
pub(crate) use metrics::{hinting, with_afm_metrics, with_text_path_metrics, without_hinting};
pub use plt_text::{
    BlockLayout, Feature, HAlign, Language, LanguageRange, LanguageTag, LineMetrics, LinePlacement,
    LineSpacing, ParseFeatureError, RotationMode as TextRotationMode, TextError, VAlign,
};
pub(crate) use text_box::BoxPlacement;
pub use text_box::{TextBox, TextBoxProps};

/// How a text's rotation is reduced before it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AngleReduction {
    /// Into `[0, 360)`, as `set_rotation` stores it.
    Modulo360,
    /// Not at all: matplotlib's `Text3D.draw` sets `_rotation` directly,
    /// bypassing `set_rotation`.
    AsIs,
}

/// State of a text artist (matplotlib's `Text`, and `Annotation` when
/// `annotation` is set).
#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors matplotlib's independent boolean text properties"
)]
pub(crate) struct TextArtist {
    base: ArtistBase,
    pub(crate) text: String,
    pub(crate) position: Point,
    pub(crate) color: Color,
    pub(crate) font: FontProperties,
    pub(crate) rotation: f64,
    /// How `rotation` is reduced before drawing.
    pub(crate) rotation_reduction: AngleReduction,
    /// Whether `rotation` is a direction in the text's coordinates, which
    /// the transform turns (matplotlib's `transform_rotates_text`).
    pub(crate) transform_rotates_text: bool,
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
    /// OpenType feature settings for drawing the text on one line.
    pub(crate) fontfeatures: Vec<Feature>,
    /// The language of the text (`text.language` when the text was made).
    pub(crate) language: Option<Language>,
    /// The box drawn behind the text.
    pub(crate) bbox: Option<TextBox>,
    /// What makes the text an annotation.
    pub(crate) annotation: Option<Box<AnnotationState>>,
    pub(crate) rc: Rc<RcParams>,
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
            rotation_reduction: AngleReduction::Modulo360,
            transform_rotates_text: false,
            halign: HAlign::Left,
            valign: VAlign::Baseline,
            multialign: None,
            rotation_mode: RotationMode::Default,
            linespacing: LineSpacing::Normal,
            antialiased: rc.text_antialiased,
            wrap: false,
            parse_math: rc.text_parse_math,
            fontfeatures: Vec::new(),
            language: rc_language(&rc),
            bbox: None,
            annotation: None,
            rc,
        }
    }

    /// The rotation drawn in degrees (matplotlib's `get_rotation`): turned
    /// by the transform at the text's position, then reduced to `[0, 360)`,
    /// when the transform rotates the text.
    fn drawn_rotation(&self, ctx: LayoutContext<'_>) -> f64 {
        let rotation = match self.rotation_reduction {
            AngleReduction::Modulo360 => self.rotation.rem_euclid(360.0),
            AngleReduction::AsIs => self.rotation,
        };
        if !self.transform_rotates_text {
            return rotation;
        }
        ctx.resolve(&self.transform())
            .map_or(rotation, |transform| {
                transform
                    .transform_angle(rotation, self.position)
                    .rem_euclid(360.0)
            })
    }

    /// `set_language(None)` means `text.language` (matplotlib's
    /// `_val_or_rc`).
    fn resolve_language(&mut self) {
        if self.language.is_none() {
            self.language = rc_language(&self.rc);
        }
    }

    /// The features and language the text is drawn with.
    fn shaping(&self) -> Shaping {
        Shaping {
            features: self.fontfeatures.clone(),
            language: self.language.clone(),
        }
    }

    /// Alignment and rotation of the block. Wrapped text always aligns
    /// before rotating (matplotlib switches it to the `anchor` rotation
    /// mode when wrapping).
    pub(crate) fn block_style(&self, ctx: LayoutContext<'_>) -> BlockStyle {
        BlockStyle {
            rotation: self.drawn_rotation(ctx),
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

    /// The anchor of a plain text in display coordinates, if it is finite.
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

    /// The text with line breaks inserted so that it fits in its figure
    /// when anchored at `anchor`, if wrapping is on (matplotlib's
    /// `_get_wrapped_text`).
    fn wrapped_at(&self, ctx: LayoutContext<'_>, anchor: Point) -> Result<String, TextError> {
        let Some(figure) = self.figure_box(ctx).filter(|_| self.wrap) else {
            return Ok(self.text.clone());
        };
        let style = self.block_style(ctx);
        let width = wrap::wrap_width(anchor, &figure, style.rotation, style.halign);
        wrap::wrap_text(&self.text, width, |line| {
            let math = plt_text::mathtext::is_math_text(line);
            measure_text(line, &self.font, math, ctx.dpi(), &self.rc).map(|m| m.width.ceil())
        })
    }

    /// The text with the line breaks wrapping adds.
    pub(crate) fn wrapped_text(&self, ctx: LayoutContext<'_>) -> Result<String, TextError> {
        match self.anchor(ctx) {
            Some(anchor) => self.wrapped_at(ctx, anchor),
            None => Ok(self.text.clone()),
        }
    }

    fn layout_at(&self, ctx: LayoutContext<'_>, anchor: Point) -> Result<LaidOutText, TextError> {
        lay_out_with_math(
            &self.wrapped_at(ctx, anchor)?,
            &self.font,
            &self.block_style(ctx),
            ctx.dpi(),
            &self.rc,
            self.parse_math,
        )
    }

    /// Lays the (wrapped) text out for the canvas resolution.
    pub(crate) fn layout(&self, ctx: LayoutContext<'_>) -> Result<LaidOutText, TextError> {
        match self.anchor(ctx) {
            Some(anchor) => self.layout_at(ctx, anchor),
            None => lay_out_with_math(
                &self.text,
                &self.font,
                &self.block_style(ctx),
                ctx.dpi(),
                &self.rc,
                self.parse_math,
            ),
        }
    }

    /// The text's extent when anchored at `anchor` (`Text.get_window_extent`).
    pub(crate) fn extent_at(
        &self,
        ctx: LayoutContext<'_>,
        anchor: Point,
    ) -> Result<Bbox, TextDrawError> {
        if self.text.is_empty() {
            return Ok(Bbox::from_bounds(anchor.x, anchor.y, 0.0, 0.0));
        }
        let laid_out = self.layout_at(ctx, anchor)?;
        Ok(laid_out.block.bbox.translated(anchor.x, anchor.y))
    }

    /// Whether display point `at` falls in the text's extent or on its box
    /// when the text is anchored at `anchor` (`Text.contains`; hidden
    /// texts are never hit).
    pub(crate) fn contains_at(&self, ctx: LayoutContext<'_>, anchor: Point, at: Point) -> bool {
        if !self.base.visible {
            return false;
        }
        let in_text = self
            .extent_at(ctx, anchor)
            .is_ok_and(|extent| extent.contains(at));
        in_text
            || self
                .box_placement(ctx, anchor)
                .ok()
                .flatten()
                .and_then(|(bbox, placement)| bbox.clip_region(&self.rc, &placement).ok())
                .is_some_and(|region| region.contains(at))
    }

    /// The box and where it goes for the text anchored at `anchor`
    /// (`update_bbox_position_size`).
    pub(crate) fn box_placement(
        &self,
        ctx: LayoutContext<'_>,
        anchor: Point,
    ) -> Result<Option<(&TextBox, BoxPlacement)>, TextDrawError> {
        let Some(bbox) = &self.bbox else {
            return Ok(None);
        };
        let laid_out = self.layout_at(ctx, anchor)?;
        let (corner, size) = laid_out.block.frame;
        Ok(Some((
            bbox,
            BoxPlacement {
                anchor,
                corner,
                size,
                rotation: self.block_style(ctx).rotation,
                fontsize_px: ctx.points_to_pixels(self.font.size),
            },
        )))
    }

    /// Draws the box and the text anchored at `anchor` (the body of
    /// `Text.draw`); path effects apply to the glyphs only.
    pub(crate) fn draw_at(
        &self,
        ctx: &mut DrawContext<'_>,
        anchor: Point,
    ) -> Result<(), TextDrawError> {
        if self.text.is_empty() {
            return Ok(());
        }
        ctx.renderer().open_group("text", self.base.gid.as_deref());
        let drawn = self.draw_block_at(ctx, anchor);
        ctx.renderer().close_group("text");
        drawn
    }

    /// Draws the text's box and lines at `anchor`.
    fn draw_block_at(&self, ctx: &mut DrawContext<'_>, anchor: Point) -> Result<(), TextDrawError> {
        let layout = *ctx.layout();
        if let Some((bbox, placement)) = self.box_placement(layout, anchor)? {
            bbox.draw(ctx, &self.base, &self.rc, &placement)
                .map_err(TextDrawError::Box)?;
        }
        let mut laid_out = self.layout_at(layout, anchor)?.shaped(&self.shaping())?;
        if !self.base.path_effects.is_empty() {
            laid_out =
                laid_out.as_text_paths(&self.font, self.parse_math, layout.dpi(), &self.rc)?;
        }
        let color = self.color.resolve_alpha(self.base.alpha)?;
        let mut gc = ctx.base_gc(&self.base);
        gc.foreground = color;
        gc.antialiased = self.antialiased;
        // matplotlib never sketches glyphs, even as paths.
        gc.sketch = None;
        let rotation = self.block_style(layout).rotation;
        with_path_effects(ctx, &self.base.path_effects, |ctx| {
            draw_laid_out(ctx, &gc, &laid_out, anchor, rotation);
        })?;
        Ok(())
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
        let drawn = match &self.annotation {
            Some(annotation) => annotation.draw(self, ctx),
            None => match self.anchor(*ctx.layout()) {
                Some(anchor) => self.draw_at(ctx, anchor),
                None => Ok(()),
            },
        };
        if let Err(err) = drawn {
            ctx.report(self.base.id(), err);
        }
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        match &self.annotation {
            Some(annotation) => annotation.extent(self, *ctx).ok(),
            None => self.extent_at(*ctx, self.anchor(*ctx)?).ok(),
        }
    }

    /// The text's box, where it is drawn (what matplotlib's
    /// `get_bbox_patch()` clips connections to); `None` without a box.
    fn connection_region(&self, ctx: &LayoutContext<'_>) -> Option<ClipRegion> {
        let anchor = match &self.annotation {
            Some(annotation) => annotation.text_display(self, *ctx).ok()?,
            None => self.anchor(*ctx)?,
        };
        let (bbox, placement) = self.box_placement(*ctx, anchor).ok()??;
        bbox.clip_region(&self.rc, &placement).ok()
    }

    /// matplotlib's `Text.contains` (and `Annotation.contains`, which adds
    /// the arrow).
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        match &self.annotation {
            Some(annotation) => annotation.contains(self, *ctx, point),
            None => self
                .anchor(*ctx)
                .is_some_and(|anchor| self.contains_at(*ctx, anchor, point)),
        }
    }
}

/// Draws every line of a laid-out text block anchored at `anchor`
/// (display coordinates), rotated by `angle` degrees. A single line tells
/// the renderer where and how the text is anchored, as matplotlib passes
/// the text artist along with single-line texts.
pub(crate) fn draw_laid_out(
    ctx: &mut DrawContext<'_>,
    gc: &plt_render::GraphicsContext,
    laid_out: &LaidOutText,
    anchor: Point,
    angle: f64,
) {
    if let ([line], [run]) = (laid_out.block.lines.as_slice(), laid_out.runs.as_slice()) {
        let anchored = TextRun {
            anchor: Some(TextAnchor {
                point: anchor,
                halign: laid_out.style.halign,
                valign: laid_out.style.valign,
                rotation_mode: laid_out.style.rotation_mode,
            }),
            ..run.clone()
        };
        ctx.renderer()
            .draw_text(gc, &anchored, anchor + line.origin, angle);
        return;
    }
    for (line, run) in laid_out.block.lines.iter().zip(&laid_out.runs) {
        ctx.renderer()
            .draw_text(gc, run, anchor + line.origin, angle);
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;

    #[test]
    fn rotations_can_follow_the_transform() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.125, 0.11, 0.775, 0.77])
            .expect("a valid rectangle");
        // 49.6 pixels per unit along x, 369.6 along y.
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        let text = ax.text(5.0, 0.5, "slope");
        text.set_rotation(45.0).set_transform_rotates_text(true);
        let rotation = text.layout().expect("laid out").style.rotation;
        let expected = (369.6_f64 / 49.6).atan().to_degrees();
        assert!(
            (rotation - expected).abs() < 1e-6,
            "{rotation} vs {expected}"
        );
        text.set_transform_rotates_text(false);
        assert_eq!(text.layout().expect("laid out").style.rotation, 45.0);
    }

    #[test]
    fn transformed_rotations_stay_in_one_turn() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.125, 0.11, 0.775, 0.77])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(1.0, 0.0))
            .expect("finite limits");
        let text = ax.text(5.0, 0.5, "down");
        text.set_rotation(45.0).set_transform_rotates_text(true);
        let rotation = text.layout().expect("laid out").style.rotation;
        let expected = 360.0 - (369.6_f64 / 49.6).atan().to_degrees();
        assert!(
            (rotation - expected).abs() < 1e-6,
            "{rotation} vs {expected}"
        );
    }
}
