//! Drawing the tick marks, tick labels and offset text of a 3-D axis
//! (matplotlib's `_get_updated_ticks`, `Tick.draw` and
//! `_update_offset_text`).

use pltrs_core::{Path, Point};
use pltrs_text::{BlockStyle, HAlign, VAlign};

use super::{
    AxisEdge, AxisTextPart, Painter, PlacedText, Sink, TICK_INWARD, TICK_LABEL_OFFSET,
    TICK_OUTWARD, anchored, grouped,
};
use crate::axes::{AxisGroup, TickPart};
use crate::fonts::FontProperties;
use crate::mplot3d::axis3d::{Axis3DState, DrawnTick};
use crate::mplot3d::coords::Axis3;
use crate::mplot3d::geometry::{AxisPosition, static_tick_direction, tick_direction};
use crate::mplot3d::math::{Vec3, scaled};

/// Where the tick marks of an axis go along one of its edges: across the
/// edge, from outside the box inwards (matplotlib's `_get_updated_ticks`).
pub(super) struct MarkSpan {
    /// The start of the edge, in transformed coordinates.
    origin: Vec3,
    /// The direction the marks run along.
    tickdir: usize,
    /// Where the marks start and end along it.
    ends: [f64; 2],
}

impl MarkSpan {
    /// The point of the edge at `coordinate` along axis `along`, moved to
    /// `at` along the direction of the marks.
    fn point(&self, along: usize, coordinate: f64, at: f64) -> Vec3 {
        let mut p = self.origin;
        p[along] = coordinate;
        p[self.tickdir] = at;
        p
    }
}

impl Painter<'_> {
    /// The span of the tick marks of `dir` along the edge at `position`
    /// starting at `origin`.
    pub(super) fn mark_span(&self, dir: Axis3, position: AxisPosition, origin: Vec3) -> MarkSpan {
        let tickdir = tick_direction(dir, &self.axes.camera, position);
        let delta = self.geometry.deltas[tickdir];
        let tickdelta = if self.geometry.highs[tickdir] {
            delta
        } else {
            -delta
        };
        let base = origin[tickdir];
        MarkSpan {
            origin,
            tickdir,
            ends: [
                base + TICK_OUTWARD * tickdelta,
                base - TICK_INWARD * tickdelta,
            ],
        }
    }

    /// The projected tick mark of `axis` at `loc` in `span`, placed where
    /// the axis' scale maps `loc` (matplotlib's `axis.get_transform()`,
    /// whose log scales clip what is not positive).
    pub(super) fn mark(&self, span: &MarkSpan, axis: &Axis3DState, loc: f64) -> [Point; 2] {
        let coordinate = axis.scale.forward(loc);
        span.ends
            .map(|at| self.project(span.point(axis.dir.index(), coordinate, at)))
    }

    /// Draws each tick mark and its label, each tick in its SVG group
    /// (matplotlib's `_get_updated_ticks` followed by `Tick.draw`).
    pub(super) fn draw_ticks(
        &self,
        sink: &mut impl Sink,
        axis: &Axis3DState,
        origin: Vec3,
        deltas_per_point: f64,
        position: AxisPosition,
    ) {
        let dir = axis.dir;
        let span = self.mark_span(dir, position, origin);
        let points = scaled(self.geometry.deltas, deltas_per_point);
        for drawn in axis.drawn_ticks() {
            let mark = self.mark(&span, axis, drawn.tick.loc);
            let p = span.point(
                dir.index(),
                axis.scale.forward(drawn.tick.loc),
                origin[span.tickdir],
            );
            let label_deltas = scaled(points, drawn.pad + TICK_LABEL_OFFSET);
            let label_at = self.geometry.move_from_center(p, label_deltas, dir);
            let group = AxisGroup::Tick {
                minor: drawn.minor,
                index: drawn.slot,
            };
            grouped(sink, "xtick", axis.gids.get(group), |sink| {
                self.draw_tick(sink, axis, &drawn, mark, self.project(label_at));
            });
        }
    }

    /// Draws the parts of one tick, each in its SVG group: its gridline,
    /// its mark from `mark[0]` to `mark[1]` and its label at `label_at`, in
    /// projected coordinates (matplotlib's `Tick.draw`).
    fn draw_tick(
        &self,
        sink: &mut impl Sink,
        axis: &Axis3DState,
        drawn: &DrawnTick<'_>,
        mark: [Point; 2],
        label_at: Point,
    ) {
        let DrawnTick {
            minor, slot, width, ..
        } = *drawn;
        let style = axis.tick_style_at(minor, slot, true);
        let gid = |part| {
            (axis.gids).get(AxisGroup::TickPart {
                minor,
                index: slot,
                part,
            })
        };
        if style.grid_on {
            grouped(sink, "line2d", gid(TickPart::Gridline), |sink| {
                sink.tick_gridline(self, axis.dir, &style.grid);
            });
        }
        // The marks are always drawn, as wide as the axis says
        // (matplotlib's `_tick_update_position`).
        grouped(sink, "line2d", gid(TickPart::Tick1Line), |sink| {
            sink.tick_mark(self, &Path::polyline(mark.to_vec()), width, &style.color);
        });
        // matplotlib's `_tick_update_position` puts both labels where the
        // first one goes; each is drawn (and measured) when shown, with its
        // own style and alignment (the second one's text over the point).
        if style.labels.first {
            self.draw_tick_label(sink, axis, drawn, label_at, false);
        }
        if style.labels.second {
            self.draw_tick_label(sink, axis, drawn, label_at, true);
        }
    }

    /// Draws the first (or `second`) label of a tick at `label_at`, in
    /// projected coordinates, in its SVG group (matplotlib's
    /// `Tick.label1` and `Tick.label2`, both `XTick` labels in 3-D: the
    /// first hangs below the point, the second stands on it).
    fn draw_tick_label(
        &self,
        sink: &mut impl Sink,
        axis: &Axis3DState,
        drawn: &DrawnTick<'_>,
        label_at: Point,
        second: bool,
    ) {
        let DrawnTick {
            tick, minor, slot, ..
        } = *drawn;
        let style = axis.tick_style_at(minor, slot, !second);
        let font = style.label_font(self.axes.rc());
        let halign = style.label_halign.unwrap_or(HAlign::Center);
        let default_valign = if second { VAlign::Bottom } else { VAlign::Top };
        let part = if second {
            TickPart::Label2
        } else {
            TickPart::Label1
        };
        self.text_in_group(
            sink,
            &PlacedText {
                part: AxisTextPart::TickLabel {
                    minor,
                    index: slot,
                    second,
                },
                gid: (axis.gids).get(AxisGroup::TickPart {
                    minor,
                    index: slot,
                    part,
                }),
                text: &tick.label,
                font: &font,
                color: &style.label_color,
                block: BlockStyle {
                    halign,
                    valign: style.label_valign.unwrap_or(default_valign),
                    multialign: halign,
                    rotation: style.label_rotation.rem_euclid(360.0),
                    rotation_mode: style.label_rotation_mode,
                    ..BlockStyle::default()
                },
                position: label_at,
            },
        );
    }

    /// Draws the offset text at the end of the axis, aligned by
    /// matplotlib's trial-and-error rules (`_update_offset_text`).
    pub(super) fn draw_offset_text(
        &self,
        sink: &mut impl Sink,
        axis: &Axis3DState,
        edge: &AxisEdge,
        label_deltas: Vec3,
    ) {
        let Ok(ticks) = &axis.ticks else {
            return;
        };
        if ticks.offset.is_empty() || !axis.offset_visible() {
            return;
        }
        let dir = axis.dir;
        let index = dir.index();
        let tickdir = static_tick_direction(dir);
        let outer = outer_end(axis);
        let at = self
            .geometry
            .move_from_center(edge.points[outer], label_deltas, dir);
        let center = self.projection.project_transformed(self.geometry.centers);
        let end = edge.ends[outer];
        let even_highs = self.geometry.highs.iter().filter(|&&h| h).count() % 2 == 0;
        let halign = if center[tickdir] > end[tickdir] {
            if center[index] <= end[index] && even_highs {
                if self.geometry.highs == [false, true, true] && index != 0 {
                    HAlign::Left
                } else {
                    HAlign::Right
                }
            } else {
                HAlign::Left
            }
        } else if center[index] > end[index] && even_highs {
            if index == 2 {
                HAlign::Right
            } else {
                HAlign::Left
            }
        } else {
            HAlign::Right
        };
        let rc = self.axes.rc();
        let font = FontProperties::from_rc(rc).with_size(rc.xtick_labelsize, rc.font_size);
        self.text_in_group(
            sink,
            &PlacedText {
                part: AxisTextPart::Offset,
                gid: axis.gids.get(AxisGroup::OffsetText),
                text: &ticks.offset,
                font: &font,
                color: &axis.offset_color,
                block: anchored(edge.angle(), halign),
                position: self.project(at),
            },
        );
    }
}

/// Which end of its edge the offset text of `axis` is moved away from
/// (matplotlib's `outeredgep`: the first end unless the axis' juggled
/// triple ends in 2).
pub(super) fn outer_end(axis: &Axis3DState) -> usize {
    usize::from(axis.juggled[2] != 2)
}
