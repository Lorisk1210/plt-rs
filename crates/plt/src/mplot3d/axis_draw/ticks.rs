//! Drawing the tick marks, tick labels and offset text of a 3-D axis
//! (matplotlib's `_get_updated_ticks`, `Tick.draw` and
//! `_update_offset_text`).

use plt_core::Path;
use plt_text::{BlockStyle, HAlign, VAlign};

use super::{
    AxisEdge, Painter, PlacedText, Sink, TICK_INWARD, TICK_LABEL_OFFSET, TICK_OUTWARD, anchored,
};
use crate::fonts::FontProperties;
use crate::mplot3d::axis3d::{Axis3DState, DrawnTick};
use crate::mplot3d::geometry::{AxisPosition, juggled, static_tick_direction, tick_direction};
use crate::mplot3d::math::{Vec3, scaled};

impl Painter<'_> {
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
        let tickdir = tick_direction(dir, &self.axes.camera, position);
        let delta = self.geometry.deltas[tickdir];
        let tickdelta = if self.geometry.highs[tickdir] {
            delta
        } else {
            -delta
        };
        let base = origin[tickdir];
        let (outward, inward) = (
            base + TICK_OUTWARD * tickdelta,
            base - TICK_INWARD * tickdelta,
        );
        let points = scaled(self.geometry.deltas, deltas_per_point);
        for DrawnTick {
            tick,
            style,
            width,
            pad,
        } in axis.drawn_ticks()
        {
            let mut p = origin;
            p[dir.index()] = axis.scale.masked_forward(tick.loc);
            p[tickdir] = outward;
            let out_end = self.project(p);
            p[tickdir] = inward;
            let in_end = self.project(p);
            p[tickdir] = base;
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
                text: &ticks.offset,
                font: &font,
                color: &axis.offset_color,
                block: anchored(edge.angle(), halign),
                position: self.project(at),
            },
        );
    }
}
