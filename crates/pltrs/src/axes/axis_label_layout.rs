//! Where the axis label and the offset text of an axis go: beyond the
//! spine, its ticks and the tick labels on their side (matplotlib's
//! `_update_label_position` and `_update_offset_text_position`).

use pltrs_core::{Bbox, Point};
use pltrs_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis_layout::{
    AxisPainter, PlacedText, measured_at_nan, place_text_measured, rc_with_effects,
};
use crate::artist::{FigureReader, LayoutContext};
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::text::{TextProblem, Typesetting};
use crate::ticker::AxisName;

/// Distance between tick labels and the offset text, in points
/// (matplotlib's `Axis.OFFSETTEXTPAD`).
const OFFSET_TEXT_PAD: f64 = 3.0;

impl AxisPainter<'_> {
    /// The axis label at `automatic` (its
    /// [`axis_label_anchor`](Self::axis_label_anchor))
    /// or at its coordinates.
    pub(super) fn axis_label(
        &self,
        cx: &mut impl FigureReader,
        rc: &RcParams,
        automatic: Option<Point>,
    ) -> Option<Result<PlacedText, TextProblem>> {
        let label = &self.axis.label;
        if label.text.is_empty() || !label.visible {
            return None;
        }
        let extent = cx.read().axes_extent(self.axes_id)?;
        let automatic = automatic?;
        let along = label.halign();
        let multialign = label.multialign.unwrap_or(along);
        let block = match self.name {
            AxisName::X => BlockStyle {
                rotation: label.rotation_on(self.name),
                halign: along,
                valign: label.valign_on(self.name),
                multialign,
                ..BlockStyle::default()
            },
            AxisName::Y => BlockStyle {
                rotation: label.rotation_on(self.name),
                halign: along,
                valign: label.valign_on(self.name),
                multialign,
                rotation_mode: RotationMode::Anchor,
                ..BlockStyle::default()
            },
        };
        let anchor = label.coords.map_or(automatic, |at| {
            Point::new(
                extent.x0() + at.x * extent.width(),
                extent.y0() + at.y * extent.height(),
            )
        });
        let rc = &*rc_with_effects(rc, label.path_effects.as_deref());
        let placed = place_text_measured(
            cx,
            &label.text,
            &label.font,
            &label.color,
            &block,
            anchor,
            rc,
            Typesetting::of(rc.text_parse_math, label.usetex),
        );
        Some(placed.map(|placed| PlacedText {
            path_effects: label.path_effects.clone(),
            ..placed
        }))
    }

    /// Where the axis label goes unless placed by coordinates, with or
    /// without text: in display coordinates, along the axis at the label's
    /// `loc` and beyond the spine, its ticks and the tick labels
    /// `label_boxes` on its side by the label's pad (matplotlib's
    /// `_update_label_position`); `None` for axes not laid out.
    pub(super) fn axis_label_anchor(
        &self,
        cx: &mut impl FigureReader,
        label_boxes: &[(bool, Bbox)],
    ) -> Option<Point> {
        let label = &self.axis.label;
        let extent = cx.read().axes_extent(self.axes_id)?;
        let pad = cx.read().points_to_pixels(label.pad);
        let high = label.side.is_high();
        let edge = self.spine_extent(cx, label.side).unwrap_or(extent);
        let boxes = label_boxes
            .iter()
            .filter(|(first, _)| *first != high)
            .map(|(_, b)| *b);
        let bounds = boxes.fold(edge, |acc, b| acc.union(&b));
        let beyond = match (self.name, high) {
            _ if measured_at_nan(label_boxes, high) => f64::NAN,
            (AxisName::X, true) => bounds.y1() + pad,
            (AxisName::X, false) => bounds.y0() - pad,
            (AxisName::Y, true) => bounds.x1() + pad,
            (AxisName::Y, false) => bounds.x0() - pad,
        };
        Some(match self.name {
            AxisName::X => Point::new(extent.x0() + label.along() * extent.width(), beyond),
            AxisName::Y => Point::new(beyond, extent.y0() + label.along() * extent.height()),
        })
    }

    /// The offset text at the end of the axis, beyond the tick labels.
    pub(super) fn offset_text(
        &self,
        cx: &mut impl FigureReader,
        rc: &RcParams,
        label_boxes: &[(bool, Bbox)],
    ) -> Option<Result<PlacedText, TextProblem>> {
        let text = &self.axis.ticks.as_ref().ok()?.offset;
        if text.is_empty() || !self.axis.offset_visible {
            return None;
        }
        let (anchor, block) = self.offset_placement(cx.read(), label_boxes)?;
        let font = FontProperties {
            size: self.axis.offset_size,
            ..FontProperties::from_rc(rc)
        };
        Some(place_text_measured(
            cx,
            text,
            &font,
            &self.axis.major_style.label_color,
            &block,
            anchor,
            rc,
            Typesetting::from_rc(rc),
        ))
    }

    /// Where the offset text goes, shown or not: its anchor in display
    /// coordinates, beyond the tick labels `label_boxes`, and its
    /// alignment (matplotlib's `_update_offset_text_position`).
    pub(super) fn offset_placement(
        &self,
        layout: LayoutContext<'_>,
        label_boxes: &[(bool, Bbox)],
    ) -> Option<(Point, BlockStyle)> {
        let extent = layout.axes_extent(self.axes_id)?;
        let pad = layout.points_to_pixels(OFFSET_TEXT_PAD);
        let high = self.axis.offset_side.is_high();
        Some(match self.name {
            AxisName::X => {
                let side_boxes = label_boxes.iter().filter(|(first, _)| *first != high);
                let (y, valign) = if measured_at_nan(label_boxes, high) {
                    let valign = if high { VAlign::Bottom } else { VAlign::Top };
                    (f64::NAN, valign)
                } else if high {
                    let top = side_boxes
                        .map(|(_, b)| b.y1())
                        .fold(f64::NEG_INFINITY, f64::max);
                    let top = if top.is_finite() { top } else { extent.ymax() };
                    (top + pad, VAlign::Bottom)
                } else {
                    let bottom = side_boxes
                        .map(|(_, b)| b.y0())
                        .fold(f64::INFINITY, f64::min);
                    let bottom = if bottom.is_finite() {
                        bottom
                    } else {
                        extent.ymin()
                    };
                    (bottom - pad, VAlign::Top)
                };
                (
                    Point::new(extent.x1(), y),
                    BlockStyle {
                        halign: HAlign::Right,
                        valign,
                        multialign: HAlign::Right,
                        ..BlockStyle::default()
                    },
                )
            }
            AxisName::Y => {
                let (x, halign) = if high {
                    (extent.x1(), HAlign::Right)
                } else {
                    (extent.x0(), HAlign::Left)
                };
                // Above the extensions of a colorbar.
                let top = crate::colorbar::outline_extent(&layout, self.axes_id)
                    .map_or(extent.ymax(), |outline| outline.ymax());
                (
                    Point::new(x, top + pad),
                    BlockStyle {
                        halign,
                        valign: VAlign::Baseline,
                        ..BlockStyle::default()
                    },
                )
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::{Axes, AxesState, TickParams};
    use crate::figure::Figure;
    use crate::spines::SpineSide;

    /// The box of the label of axis `name` of `ax`.
    fn label_box(fig: &Figure, ax: &Axes, name: AxisName) -> Bbox {
        fig.layout();
        let state = fig.raw().figure_state().borrow();
        let axes = state.store.get_as::<AxesState>(ax.id()).expect("an axes");
        let texts =
            AxisPainter::new(ax.id(), axes, name).texts(&mut LayoutContext::new(&state), &axes.rc);
        texts.label.expect("a label").bbox()
    }

    #[test]
    fn colorbar_offset_texts_clear_the_extensions() {
        // matplotlib's test_offset_text_loc: above the triangle at the top.
        use crate::colorbar::ColorbarOptions;
        use crate::colors::{Extend, Normalize, ScalarMappable, colormaps};
        let fig = Figure::new();
        let ax = fig.subplot();
        let mappable = ScalarMappable::with_norm(
            colormaps::get("viridis").expect("a colormap"),
            Box::new(Normalize::new(0.0, 2e6)),
        );
        let options = ColorbarOptions {
            extend: Some(Extend::Max),
            ..ColorbarOptions::beside(&ax)
        };
        let colorbar = fig.colorbar(&mappable, options).expect("a colorbar");
        let offset = colorbar
            .long_axis()
            .offset_text_extent()
            .expect("an offset text");
        let outline = colorbar.outline().window_extent().expect("laid out");
        assert!(offset.y0() > outline.y1(), "{offset:?} over {outline:?}");
    }

    /// The top of the x axis label of `ax`.
    fn xlabel_top(fig: &Figure, ax: &Axes) -> f64 {
        label_box(fig, ax, AxisName::X).y1()
    }

    #[test]
    fn rotated_and_aligned_labels_turn_about_their_anchor() {
        // A colorbar label as `cbar.ax.set_ylabel(s, rotation=-90,
        // va="bottom")` sets it: on the right, reading downwards, on the
        // same side of its anchor as the default upward label.
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_ylabel("label");
        let yaxis = ax.yaxis();
        yaxis
            .set_label_position(SpineSide::Right)
            .expect("right is a side of y");
        let upward = label_box(&fig, &ax, AxisName::Y);
        yaxis
            .set_label_rotation(-90.0)
            .set_label_valign(VAlign::Bottom);
        let downward = label_box(&fig, &ax, AxisName::Y);
        assert!((downward.x0() - upward.x0()).abs() < 1e-9);
        assert!((downward.width() - upward.width()).abs() < 1e-9);
        assert!((downward.height() - upward.height()).abs() < 1e-9);
        yaxis.set_label_valign(VAlign::Top);
        let hanging = label_box(&fig, &ax, AxisName::Y);
        assert!(
            (hanging.x1() - upward.x0()).abs() < 1e-9,
            "left of the anchor"
        );
        yaxis
            .set_label_valign(VAlign::Bottom)
            .set_label_position(SpineSide::Right)
            .expect("right is a side of y");
        assert_eq!(
            label_box(&fig, &ax, AxisName::Y),
            hanging,
            "moving the label brings back the alignment of its side"
        );
    }

    #[test]
    fn empty_tick_labels_keep_the_axis_label_beyond_their_anchors() {
        // matplotlib measures an empty tick label as a box of no size at
        // its anchor, the tick pad below the ticks.
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlabel("x");
        ax.set_xticklabels(Vec::<String>::new())
            .expect("no labels fit any ticks");
        let empty = xlabel_top(&fig, &ax);
        ax.tick_params(TickParams {
            labelbottom: Some(false),
            ..TickParams::default()
        });
        let hidden = xlabel_top(&fig, &ax);
        let pad = 3.5 * fig.dpi() / 72.0;
        assert!((hidden - empty - pad).abs() < 1e-9, "{hidden} - {empty}");
    }
}
