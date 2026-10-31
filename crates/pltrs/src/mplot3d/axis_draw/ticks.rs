//! Drawing the tick marks, tick labels and offset text of a 3-D axis
//! (matplotlib's `_get_updated_ticks`, `Tick.draw` and
//! `_update_offset_text`).

use pltrs_core::{Path, Point};
use pltrs_text::{BlockStyle, HAlign, VAlign};

use super::{
    AxisEdge, AxisTextPart, Painter, PlacedText, Sink, TICK_INWARD, TICK_LABEL_OFFSET,
    TICK_OUTWARD, anchored,
};
use crate::fonts::FontProperties;
use crate::mplot3d::axis3d::{Axis3DState, DrawnTick};
use crate::mplot3d::coords::Axis3;
use crate::mplot3d::geometry::{AxisPosition, juggled, static_tick_direction, tick_direction};
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

    /// The projected tick mark of `axis` at `loc` in `span`.
    pub(super) fn mark(&self, span: &MarkSpan, axis: &Axis3DState, loc: f64) -> [Point; 2] {
        let coordinate = axis.scale.masked_forward(loc);
        span.ends
            .map(|at| self.project(span.point(axis.dir.index(), coordinate, at)))
    }

    /// Draws each tick mark and its label (matplotlib's
    /// `_get_updated_ticks` followed by `Tick.draw`).
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
        for DrawnTick {
            tick,
            minor,
            slot,
            style,
            width,
            pad,
        } in axis.drawn_ticks()
        {
            let [out_end, in_end] = self.mark(&span, axis, tick.loc);
            let p = span.point(
                dir.index(),
                axis.scale.masked_forward(tick.loc),
                origin[span.tickdir],
            );
            let label_deltas = scaled(points, pad + TICK_LABEL_OFFSET);
            let label_at = self.geometry.move_from_center(p, label_deltas, dir);
            if style.grid_on {
                sink.tick_gridline(self, dir, &style.grid);
            }
            // The marks are always drawn, as wide as the axis says
            // (matplotlib's `_tick_update_position`).
            sink.tick_mark(
                self,
                &Path::polyline(vec![out_end, in_end]),
                width,
                &style.color,
            );
            if !style.labels.first {
                continue;
            }
            let font = FontProperties {
                size: style.label_size,
                ..FontProperties::from_rc(self.axes.rc())
            };
            let halign = style.label_halign.unwrap_or(HAlign::Center);
            sink.text(
                self,
                &PlacedText {
                    part: AxisTextPart::TickLabel { minor, index: slot },
                    text: &tick.label,
                    font: &font,
                    color: &style.label_color,
                    block: BlockStyle {
                        halign,
                        valign: style.label_valign.unwrap_or(VAlign::Top),
                        multialign: halign,
                        rotation: style.label_rotation.rem_euclid(360.0),
                        rotation_mode: style.label_rotation_mode,
                        ..BlockStyle::default()
                    },
                    position: self.project(label_at),
                },
            );
        }
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
        let outer = usize::from(juggled(dir)[2] != 2);
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
        sink.text(
            self,
            &PlacedText {
                part: AxisTextPart::Offset,
                text: &ticks.offset,
                font: &font,
                color: &axis.offset_color,
                block: anchored(edge.angle(), halign),
                position: self.project(at),
            },
        );
    }
}
