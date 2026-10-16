//! Drawing an axis: gridlines, tick marks, tick labels, the axis label and
//! the offset text (matplotlib's `Axis.draw` / `Tick.draw`).

use plt_core::{Bbox, Path, Point};
use plt_render::{GraphicsContext, SnapMode};
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis::{AxisState, Tick};
use super::state::AxesState;
use super::ticks::{TickDirection, TickStyle};
use crate::artist::{ArtistId, DrawContext};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol};
use crate::spines::SpineSide;
use crate::text::{draw_laid_out, lay_out};
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// Distance between tick labels and the offset text, in points
/// (matplotlib's `Axis.OFFSETTEXTPAD`).
const OFFSET_TEXT_PAD: f64 = 3.0;

/// Everything needed to draw one axis of one axes.
pub(crate) struct AxisPainter<'a> {
    axes_id: ArtistId,
    axes: &'a AxesState,
    axis: &'a AxisState,
    name: AxisName,
}

impl<'a> AxisPainter<'a> {
    pub(crate) fn new(axes_id: ArtistId, axes: &'a AxesState, name: AxisName) -> Self {
        Self {
            axes_id,
            axes,
            axis: axes.axis(name),
            name,
        }
    }

    /// Draws the axis; problems are reported on the axes.
    pub(crate) fn draw(&self, ctx: &mut DrawContext<'_>, rc: &crate::rc::RcParams) {
        let ticks = match &self.axis.ticks {
            Ok(ticks) => ticks,
            Err(message) => return ctx.report(self.axes_id, message),
        };
        let mut label_boxes = Vec::new();
        for (set, style) in [
            (&ticks.major, &self.axis.major_style),
            (&ticks.minor, &self.axis.minor_style),
        ] {
            for tick in set {
                self.draw_tick(ctx, tick, style, rc, &mut label_boxes);
            }
        }
        self.draw_axis_label(ctx, rc, &label_boxes);
        self.draw_offset_text(ctx, rc, &ticks.offset, &label_boxes);
    }

    /// The coordinate system of tick positions: data along the axis, axes
    /// fractions across it.
    fn tick_transform(&self) -> Transform {
        let data = Transform::Data(self.axes_id);
        let across = Transform::Axes(self.axes_id);
        match self.name {
            AxisName::X => Transform::blended(data, across),
            AxisName::Y => Transform::blended(across, data),
        }
    }

    /// A point at `loc` along the axis and `across` (0 or 1) across it.
    fn tick_point(&self, loc: f64, across: f64) -> Point {
        match self.name {
            AxisName::X => Point::new(loc, across),
            AxisName::Y => Point::new(across, loc),
        }
    }

    fn draw_tick(
        &self,
        ctx: &mut DrawContext<'_>,
        tick: &Tick,
        style: &TickStyle,
        rc: &crate::rc::RcParams,
        label_boxes: &mut Vec<(bool, Bbox)>,
    ) {
        if let Some(grid) = &style.grid {
            self.draw_gridline(ctx, tick.loc, grid, rc);
        }
        let (mark1, mark2) = tick_markers(self.name, style.direction);
        if style.ticks.first {
            self.draw_tick_mark(ctx, tick.loc, 0.0, mark1, style);
        }
        if style.ticks.second {
            self.draw_tick_mark(ctx, tick.loc, 1.0, mark2, style);
        }
        for (enabled, side_one) in [(style.labels.first, true), (style.labels.second, false)] {
            if enabled
                && !tick.label.is_empty()
                && let Some(bbox) = self.draw_tick_label(ctx, tick, style, side_one, rc)
            {
                label_boxes.push((side_one, bbox));
            }
        }
    }

    fn draw_gridline(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        grid: &super::ticks::GridStyle,
        rc: &crate::rc::RcParams,
    ) {
        let Some(resolved) = ctx.resolve(&self.tick_transform()) else {
            return;
        };
        let color = match grid.color.resolve_alpha(Some(grid.alpha)) {
            Ok(color) => color,
            Err(err) => return ctx.report(self.axes_id, err),
        };
        let line = Path::polyline(vec![self.tick_point(loc, 0.0), self.tick_point(loc, 1.0)]);
        let (path, affine) = resolved.split_path(&line);
        let mut gc = GraphicsContext {
            foreground: color,
            linewidth: grid.linewidth,
            antialiased: rc.lines_antialiased,
            ..GraphicsContext::default()
        };
        (gc.cap, gc.join) = if grid.linestyle.is_dashed() {
            (rc.lines_dash_capstyle, rc.lines_dash_joinstyle)
        } else {
            (rc.lines_solid_capstyle, rc.lines_solid_joinstyle)
        };
        gc.dashes = grid.linestyle.scaled_dashes(grid.linewidth, rc);
        if grid.linestyle.is_visible() {
            ctx.renderer().draw_path(&gc, &path, &affine, None);
        }
    }

    fn draw_tick_mark(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        across: f64,
        symbol: MarkerSymbol,
        style: &TickStyle,
    ) {
        let Some(resolved) = ctx.resolve(&self.tick_transform()) else {
            return;
        };
        let color = match style.color.resolve() {
            Ok(color) => color,
            Err(err) => return ctx.report(self.axes_id, err),
        };
        let marker = MarkerStyle::new(symbol, FillStyle::Full).expect("tick markers are valid");
        let size = ctx.points_to_pixels(style.size);
        let snap = marker.snap_threshold().map_or(SnapMode::Auto, |threshold| {
            if size >= threshold {
                SnapMode::Always
            } else {
                SnapMode::Never
            }
        });
        let gc = GraphicsContext {
            foreground: color,
            linewidth: style.width,
            join: marker.joinstyle(),
            cap: marker.capstyle(),
            snap,
            ..GraphicsContext::default()
        };
        let points = Path::polyline(vec![self.tick_point(loc, across)]);
        let (points, affine) = resolved.split_path(&points);
        let marker_transform = marker.transform().scale(size, size);
        ctx.renderer().draw_markers(
            &gc,
            marker.path(),
            &marker_transform,
            &points,
            &affine,
            Some(color),
        );
    }

    /// Draws one tick label and returns its display extent.
    fn draw_tick_label(
        &self,
        ctx: &mut DrawContext<'_>,
        tick: &Tick,
        style: &TickStyle,
        side_one: bool,
        rc: &crate::rc::RcParams,
    ) -> Option<Bbox> {
        let resolved = ctx.resolve(&self.tick_transform())?;
        let across = if side_one { 0.0 } else { 1.0 };
        let base = resolved.transform_point(self.tick_point(tick.loc, across));
        let offset = ctx.points_to_pixels(style.label_offset());
        let (anchor, halign, valign) = match (self.name, side_one) {
            (AxisName::X, true) => (base - Point::new(0.0, offset), HAlign::Center, VAlign::Top),
            (AxisName::X, false) => (
                base + Point::new(0.0, offset),
                HAlign::Center,
                VAlign::Bottom,
            ),
            (AxisName::Y, true) => (
                base - Point::new(offset, 0.0),
                HAlign::Right,
                VAlign::CenterBaseline,
            ),
            (AxisName::Y, false) => (
                base + Point::new(offset, 0.0),
                HAlign::Left,
                VAlign::CenterBaseline,
            ),
        };
        if !anchor.is_finite() {
            return None;
        }
        let font = FontProperties {
            size: style.label_size,
            ..FontProperties::from_rc(rc)
        };
        let block = BlockStyle {
            halign,
            valign,
            multialign: halign,
            ..BlockStyle::default()
        };
        self.draw_text(
            ctx,
            &tick.label,
            &font,
            &style.label_color,
            &block,
            anchor,
            rc,
        )
    }

    fn draw_axis_label(
        &self,
        ctx: &mut DrawContext<'_>,
        rc: &crate::rc::RcParams,
        label_boxes: &[(bool, Bbox)],
    ) {
        let label = &self.axis.label;
        if label.text.is_empty() {
            return;
        }
        let Some(extent) = ctx.layout().axes_extent(self.axes_id) else {
            return;
        };
        let pad = ctx.points_to_pixels(label.pad);
        let side = match self.name {
            AxisName::X => SpineSide::Bottom,
            AxisName::Y => SpineSide::Left,
        };
        let edge = self.spine_extent(ctx, side).unwrap_or(extent);
        let boxes = label_boxes.iter().filter(|(one, _)| *one).map(|(_, b)| *b);
        let bounds = boxes.fold(edge, |acc, b| acc.union(&b));
        let (anchor, block) = match self.name {
            AxisName::X => (
                Point::new(extent.x0() + 0.5 * extent.width(), bounds.y0() - pad),
                BlockStyle {
                    halign: HAlign::Center,
                    valign: VAlign::Top,
                    multialign: HAlign::Center,
                    ..BlockStyle::default()
                },
            ),
            AxisName::Y => (
                Point::new(bounds.x0() - pad, extent.y0() + 0.5 * extent.height()),
                BlockStyle {
                    rotation: 90.0,
                    halign: HAlign::Center,
                    valign: VAlign::Bottom,
                    multialign: HAlign::Center,
                    rotation_mode: RotationMode::Anchor,
                    ..BlockStyle::default()
                },
            ),
        };
        self.draw_text(
            ctx,
            &label.text,
            &label.font,
            &label.color,
            &block,
            anchor,
            rc,
        );
    }

    fn draw_offset_text(
        &self,
        ctx: &mut DrawContext<'_>,
        rc: &crate::rc::RcParams,
        text: &str,
        label_boxes: &[(bool, Bbox)],
    ) {
        if text.is_empty() {
            return;
        }
        let Some(extent) = ctx.layout().axes_extent(self.axes_id) else {
            return;
        };
        let pad = ctx.points_to_pixels(OFFSET_TEXT_PAD);
        let style = &self.axis.major_style;
        let font = FontProperties {
            size: style.label_size,
            ..FontProperties::from_rc(rc)
        };
        let (anchor, block) = match self.name {
            AxisName::X => {
                let below = label_boxes
                    .iter()
                    .filter(|(one, _)| *one)
                    .map(|(_, b)| b.y0())
                    .fold(f64::INFINITY, f64::min);
                let bottom = if below.is_finite() {
                    below
                } else {
                    extent.ymin()
                };
                (
                    Point::new(extent.x1(), bottom - pad),
                    BlockStyle {
                        halign: HAlign::Right,
                        valign: VAlign::Top,
                        multialign: HAlign::Right,
                        ..BlockStyle::default()
                    },
                )
            }
            AxisName::Y => (
                Point::new(extent.x0(), extent.ymax() + pad),
                BlockStyle {
                    halign: HAlign::Left,
                    valign: VAlign::Baseline,
                    ..BlockStyle::default()
                },
            ),
        };
        self.draw_text(ctx, text, &font, &style.label_color, &block, anchor, rc);
    }

    /// The spine's extent widened by the outward/inward part of the ticks
    /// drawn on it (matplotlib's `Spine.get_window_extent`).
    fn spine_extent(&self, ctx: &DrawContext<'_>, side: SpineSide) -> Option<Bbox> {
        let index = SpineSide::ALL.iter().position(|s| *s == side)?;
        let spine_id = *self.axes.spines.get(index)?;
        let spine = ctx.layout().store().get(spine_id)?;
        let mut bbox = spine.window_extent(ctx.layout())?;
        let ticks = self.axis.ticks.as_ref().ok()?;
        for (present, style) in [
            (!ticks.major.is_empty(), &self.axis.major_style),
            (!ticks.minor.is_empty(), &self.axis.minor_style),
        ] {
            if !(present && style.ticks.first) {
                continue;
            }
            let length = ctx.points_to_pixels(style.size);
            let (out, inward) = (
                style.direction.outward_fraction() * length,
                (1.0 - style.direction.outward_fraction()) * length,
            );
            let padded = match side {
                SpineSide::Bottom => {
                    Bbox::from_extents(bbox.x0(), bbox.y0() - out, bbox.x1(), bbox.y1() + inward)
                }
                _ => Bbox::from_extents(bbox.x0() - out, bbox.y0(), bbox.x1() + inward, bbox.y1()),
            };
            bbox = bbox.union(&padded);
        }
        Some(bbox)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "a text draw needs its content, style and placement"
    )]
    fn draw_text(
        &self,
        ctx: &mut DrawContext<'_>,
        text: &str,
        font: &FontProperties,
        color: &Color,
        block: &BlockStyle,
        anchor: Point,
        rc: &crate::rc::RcParams,
    ) -> Option<Bbox> {
        let laid_out = match lay_out(text, font, block, ctx.layout().dpi(), rc) {
            Ok(laid_out) => laid_out,
            Err(err) => {
                ctx.report(self.axes_id, err);
                return None;
            }
        };
        let color = match color.resolve() {
            Ok(color) => color,
            Err(err) => {
                ctx.report(self.axes_id, err);
                return None;
            }
        };
        let gc = GraphicsContext {
            foreground: color,
            antialiased: rc.text_antialiased,
            ..GraphicsContext::default()
        };
        draw_laid_out(ctx, &gc, &laid_out, anchor, block.rotation);
        Some(laid_out.block.bbox.translated(anchor.x, anchor.y))
    }
}

/// The tick markers on side 1 (bottom/left) and side 2 (top/right).
fn tick_markers(name: AxisName, direction: TickDirection) -> (MarkerSymbol, MarkerSymbol) {
    use MarkerSymbol as M;
    match (name, direction) {
        (AxisName::X, TickDirection::Out) => (M::TickDown, M::TickUp),
        (AxisName::X, TickDirection::In) => (M::TickUp, M::TickDown),
        (AxisName::X, TickDirection::InOut) => (M::VLine, M::VLine),
        (AxisName::Y, TickDirection::Out) => (M::TickLeft, M::TickRight),
        (AxisName::Y, TickDirection::In) => (M::TickRight, M::TickLeft),
        (AxisName::Y, TickDirection::InOut) => (M::HLine, M::HLine),
    }
}
