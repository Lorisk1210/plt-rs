//! The axis artist: the line, ticks, tick labels, offset text and label
//! of one axis line, drawn instead of the axes' spines and axes
//! (matplotlib's `axisartist.axis_artist.AxisArtist`).

use std::rc::Rc;

use plt_core::{Bbox, Path, Point};
use plt_render::GraphicsContext;

use super::handles::LabelDirection;
use super::helper::{AxisHelper, GONE, HelperContext};
use super::labels::{
    LabelKind, LabelState, PlacedTickLabels, TextStyle, axis_label, offset_text, tick_labels,
};
use super::line::LineState;
use super::ticks::{TicksState, tick_info};
use crate::artist::{Artist, ArtistBase, ArtistId, ClipPathSpec, DrawContext, LayoutContext};
use crate::axes::axis_draw::draw_text;
use crate::axes::{AxesState, PlacedText, TickDirection};
use crate::patches::{DisplayPath, paint_paths};
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform};

/// The z-order of axis artists (matplotlib's `AxisArtist.zorder`).
const ZORDER: f64 = 2.5;

/// The default gap between tick labels and the axis label, in points
/// (matplotlib's `AxisArtist.LABELPAD`).
const LABEL_PAD: f64 = 5.0;

/// The name of the renderer group of an axis artist (matplotlib's module
/// name).
const GROUP: &str = "mpl_toolkits.axisartist.axis_artist";

/// The two tick levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Major,
    Minor,
}

/// An axis artist (matplotlib's `AxisArtist`).
#[derive(Debug)]
pub(crate) struct AxisArtistState {
    base: ArtistBase,
    /// Its key in the axes' axis dictionary.
    pub(crate) name: String,
    axes: ArtistId,
    pub(crate) helper: AxisHelper,
    /// The axis whose tick colours, label text and offset text the artist
    /// follows: y for artists created facing left or right, else x.
    style_axis: AxisName,
    /// The direction the artist was created facing, which places the
    /// offset text.
    offset_side: SpineSide,
    /// A displacement of everything, in points (matplotlib's
    /// `offset_transform`).
    offset: (f64, f64),
    /// The side of the line the tick labels go (matplotlib's
    /// `_ticklabel_add_angle`).
    pub(crate) ticklabel_direction: LabelDirection,
    /// The side of the line the label goes (matplotlib's
    /// `_axislabel_add_angle`).
    pub(crate) axislabel_direction: LabelDirection,
    pub(crate) line: LineState,
    pub(crate) major_ticks: TicksState,
    pub(crate) minor_ticks: TicksState,
    pub(crate) major_ticklabels: LabelState,
    pub(crate) minor_ticklabels: LabelState,
    pub(crate) label: LabelState,
    /// The label text; `None` shows the style axis' label.
    pub(crate) label_text: Option<String>,
    rc: Rc<RcParams>,
}

/// The marks (locations and angles, unless hidden) and labels of one
/// tick level, laid out.
struct LevelDrawing {
    marks: Option<Vec<(Point, f64)>>,
    texts: Vec<PlacedText>,
}

/// Everything an axis artist draws, laid out: the major then the minor
/// tick level, the offset text, the line and the label.
struct Drawing {
    levels: Vec<LevelDrawing>,
    offset: Option<PlacedText>,
    line: Vec<DisplayPath>,
    label: Option<PlacedText>,
}

impl AxisArtistState {
    /// An axis artist of `axes` along `helper`, created facing
    /// `direction`, styled from `rc` (matplotlib's `AxisArtist.__init__`).
    pub(crate) fn new(
        name: &str,
        axes: ArtistId,
        helper: AxisHelper,
        (direction, offset): (SpineSide, (f64, f64)),
        rc: Rc<RcParams>,
    ) -> Self {
        let style_axis = direction.axis();
        let (major, minor, major_pad, minor_pad, tick_direction, labelsize) = match style_axis {
            AxisName::X => (
                rc.xtick_major_size,
                rc.xtick_minor_size,
                rc.xtick_major_pad,
                rc.xtick_minor_pad,
                &rc.xtick_direction,
                rc.xtick_labelsize,
            ),
            AxisName::Y => (
                rc.ytick_major_size,
                rc.ytick_minor_size,
                rc.ytick_major_pad,
                rc.ytick_minor_pad,
                &rc.ytick_direction,
                rc.ytick_labelsize,
            ),
        };
        let tick_direction: TickDirection = tick_direction.parse().unwrap_or_default();
        let tick_font = (labelsize.to_points(rc.font_size), rc.font_weight);
        let label_font = (
            rc.axes_labelsize.to_points(rc.font_size),
            rc.axes_labelweight,
        );
        let tick_labels = |pad| LabelState::new(LabelKind::TickLabels, direction, pad, tick_font);
        let mut artist = Self {
            base: ArtistBase::with_zorder(ZORDER),
            name: name.to_owned(),
            axes,
            helper,
            style_axis,
            offset_side: direction,
            offset,
            ticklabel_direction: LabelDirection::Plus,
            axislabel_direction: LabelDirection::Plus,
            line: LineState::default(),
            major_ticks: TicksState::new(major, tick_direction),
            minor_ticks: TicksState::new(minor, tick_direction),
            major_ticklabels: tick_labels(major_pad),
            minor_ticklabels: tick_labels(minor_pad),
            label: LabelState::new(LabelKind::AxisLabel, direction, LABEL_PAD, label_font),
            label_text: None,
            rc,
        };
        artist.set_axis_direction(direction);
        artist
    }

    /// Faces the major tick labels and the label `direction`, with their
    /// directions relative to the line following it (matplotlib's
    /// `set_axis_direction`; the minor tick labels keep theirs).
    pub(crate) fn set_axis_direction(&mut self, direction: SpineSide) {
        self.major_ticklabels.set_axis_direction(direction);
        self.label.set_axis_direction(direction);
        let side = match direction {
            SpineSide::Left | SpineSide::Top => LabelDirection::Minus,
            SpineSide::Right | SpineSide::Bottom => LabelDirection::Plus,
        };
        self.ticklabel_direction = side;
        self.axislabel_direction = side;
    }

    /// The tick marks of `level`.
    pub(crate) fn ticks_mut(&mut self, level: Level) -> &mut TicksState {
        match level {
            Level::Major => &mut self.major_ticks,
            Level::Minor => &mut self.minor_ticks,
        }
    }

    /// The tick labels of `level`.
    pub(crate) fn ticklabels_mut(&mut self, level: Level) -> &mut LabelState {
        match level {
            Level::Major => &mut self.major_ticklabels,
            Level::Minor => &mut self.minor_ticklabels,
        }
    }

    /// `transform` followed by the artist's offset.
    fn offset_transform(&self, transform: Transform) -> Transform {
        match self.offset {
            (0.0, 0.0) => transform,
            (dx, dy) => transform.offset_points(dx, dy),
        }
    }

    /// Lays out everything the artist draws (matplotlib's `_draw_ticks`,
    /// `_draw_line` and `_draw_label`).
    fn drawing(&self, layout: LayoutContext<'_>) -> Result<Drawing, String> {
        let axes = layout.store().get_as::<AxesState>(self.axes).ok_or(GONE)?;
        let ctx = HelperContext {
            layout,
            axes,
            id: self.axes,
        };
        let (major, minor) = self.helper.ticks(ctx)?;
        let transform = self.resolve(layout, self.helper.tick_transform(self.axes))?;
        // Both levels of tick labels clear the major ticks.
        let external = layout.points_to_pixels(self.major_ticks.outward());
        let tick_color = &self.rc.text_color;
        let mut levels = Vec::with_capacity(2);
        let mut axislabel_pad = f64::NEG_INFINITY;
        for (ticks, marks_state, labels_state) in [
            (&major, &self.major_ticks, &self.major_ticklabels),
            (&minor, &self.minor_ticks, &self.minor_ticklabels),
        ] {
            let (marks, labels) = tick_info(ticks, self.ticklabel_direction.add_angle());
            let (texts, pad) = if labels_state.visible {
                let style = labels_state.text_style(&self.rc, tick_color);
                let PlacedTickLabels {
                    texts,
                    axislabel_pad,
                } = tick_labels(
                    layout,
                    &labels,
                    &transform,
                    labels_state,
                    external,
                    &style,
                    &self.rc,
                )?;
                (texts, axislabel_pad)
            } else {
                (Vec::new(), external)
            };
            axislabel_pad = axislabel_pad.max(pad);
            levels.push(LevelDrawing {
                marks: marks_state.visible.then_some(marks),
                texts,
            });
        }
        let offset = if self.major_ticklabels.visible || self.minor_ticklabels.visible {
            self.offset_text(layout, axes)?
        } else {
            None
        };
        if self.ticklabel_direction != self.axislabel_direction {
            // The label faces away from the tick labels and clears only
            // the inward ticks, measured in points as if pixels (like
            // matplotlib).
            axislabel_pad = self.major_ticks.inward().max(self.minor_ticks.inward());
        }
        Ok(Drawing {
            levels,
            offset,
            line: self.line_paths(layout, ctx)?,
            label: self.axis_label(layout, ctx, axislabel_pad)?,
        })
    }

    /// Resolves `transform` followed by the artist's offset.
    fn resolve(
        &self,
        layout: LayoutContext<'_>,
        transform: Transform,
    ) -> Result<ResolvedTransform, String> {
        layout
            .resolve(&self.offset_transform(transform))
            .ok_or_else(|| GONE.to_owned())
    }

    /// The offset text of the style axis, unless it is empty (matplotlib's
    /// `_update_offsetText`).
    fn offset_text(
        &self,
        layout: LayoutContext<'_>,
        axes: &AxesState,
    ) -> Result<Option<PlacedText>, String> {
        let style_axis = axes.axis(self.style_axis);
        let text = style_axis
            .ticks
            .as_ref()
            .map(|ticks| ticks.offset.as_str())
            .unwrap_or_default();
        if text.is_empty() {
            return Ok(None);
        }
        let extent = layout.axes_extent(self.axes).ok_or(GONE)?;
        let labels = &self.major_ticklabels;
        let style = TextStyle {
            color: self.rc.xtick_color.clone(),
            ..labels.text_style(&self.rc, &self.rc.text_color)
        };
        let rise = labels.pad + labels.size + 2.0;
        offset_text(
            layout,
            text,
            extent,
            self.offset_side,
            rise,
            &style,
            &self.rc,
        )
        .map(Some)
    }

    /// The line in display pixels, plain or arrowed (matplotlib's
    /// `_draw_line`).
    fn line_paths(
        &self,
        layout: LayoutContext<'_>,
        ctx: HelperContext<'_>,
    ) -> Result<Vec<DisplayPath>, String> {
        if !self.line.visible {
            return Ok(Vec::new());
        }
        let (path, transform) = self.helper.line(ctx)?;
        let transform = self.resolve(layout, transform)?;
        self.line.display_paths(
            &path,
            &transform,
            self.major_ticklabels.size,
            &self.rc,
            layout.points_to_pixels(1.0),
        )
    }

    /// The axis label, `axislabel_pad` pixels plus its pad away from the
    /// line (matplotlib's `_update_label` and `AxisLabel.draw`).
    fn axis_label(
        &self,
        layout: LayoutContext<'_>,
        ctx: HelperContext<'_>,
        axislabel_pad: f64,
    ) -> Result<Option<PlacedText>, String> {
        let style_axis = ctx.axes.axis(self.style_axis);
        let text = self.label_text.as_deref().unwrap_or(&style_axis.label.text);
        if !self.label.visible || text.is_empty() {
            return Ok(None);
        }
        // Without a position the label keeps matplotlib's initial one:
        // the origin of its coordinates, facing right.
        let (reference, ref_angle) = match self.helper.label_position(ctx)? {
            Some((xy, tangent)) => (xy, tangent - 90.0 + self.axislabel_direction.add_angle()),
            None => (self.helper.label_origin(ctx)?, 0.0),
        };
        let offset = self.resolve(layout, Transform::Display)?;
        let radius = axislabel_pad + layout.points_to_pixels(self.label.pad);
        let style = self.label.text_style(&self.rc, &style_axis.label.color);
        axis_label(
            layout,
            text,
            (offset.transform_point(reference), ref_angle),
            radius,
            &self.label,
            &style,
            &self.rc,
        )
        .map(Some)
    }

    /// The pen of the tick marks of `ticks`: their colour and width, or
    /// those of the style axis' major ticks (matplotlib's `Ticks` copy
    /// them from `tick1line`).
    fn mark_gc(&self, axes: &AxesState, ticks: &TicksState) -> Result<GraphicsContext, String> {
        let style = &axes.axis(self.style_axis).major_style;
        let color = ticks.color.as_ref().unwrap_or(&style.color);
        Ok(GraphicsContext {
            foreground: color.resolve().map_err(|e| e.to_string())?,
            linewidth: ticks.width.unwrap_or(style.width),
            alpha: ticks.alpha,
            ..GraphicsContext::default()
        })
    }

    /// The artist properties of the line: the artist's opacity, clipped
    /// to the axes box if asked (by a clip path when the axes clips to an
    /// outline of its own).
    fn line_base(&self, layout: LayoutContext<'_>) -> ArtistBase {
        let mut base = ArtistBase::default();
        base.alpha = self.base.alpha;
        if self.line.clipped {
            base.clip_box = Some(self.axes);
            if layout.clip_outline(self.axes).is_some() {
                base.clip_path = Some(ClipPathSpec::Path {
                    path: Path::unit_rectangle(),
                    transform: Transform::Axes(self.axes),
                });
            }
        }
        base
    }
}

impl Artist for AxisArtistState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let id = self.base.id();
        let layout = *ctx.layout();
        let drawing = match self.drawing(layout) {
            Ok(drawing) => drawing,
            Err(problem) => return ctx.report(id, problem),
        };
        let (Some(axes), Ok(transform)) = (
            layout.store().get_as::<AxesState>(self.axes),
            self.resolve(layout, self.helper.tick_transform(self.axes)),
        ) else {
            return;
        };
        ctx.renderer().open_group(GROUP, self.base.gid.as_deref());
        for (level, ticks) in drawing
            .levels
            .iter()
            .zip([&self.major_ticks, &self.minor_ticks])
        {
            if let Some(marks) = &level.marks {
                match self.mark_gc(axes, ticks) {
                    Ok(gc) => ticks.draw(ctx, marks, &transform, &gc),
                    Err(problem) => ctx.report(id, problem),
                }
            }
            for text in &level.texts {
                draw_text(ctx, text, &self.rc);
            }
        }
        if let Some(offset) = &drawing.offset {
            draw_text(ctx, offset, &self.rc);
        }
        let (style, edge_default) = self.line.patch_style(&self.rc);
        if let Err(err) = paint_paths(
            ctx,
            &self.line_base(layout),
            &style,
            &self.rc,
            edge_default,
            &drawing.line,
        ) {
            ctx.report(id, err);
        }
        if let Some(label) = &drawing.label {
            draw_text(ctx, label, &self.rc);
        }
        ctx.renderer().close_group(GROUP);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let drawing = self.drawing(*ctx).ok()?;
        drawing
            .levels
            .iter()
            .flat_map(|level| &level.texts)
            .chain(&drawing.label)
            .chain(&drawing.offset)
            .map(PlacedText::bbox)
            .chain(drawing.line.iter().map(|p| p.to_display().extents()))
            .filter(|b| b.width() != 0.0 || b.height() != 0.0)
            .reduce(|a, b| a.union(&b))
    }
}
