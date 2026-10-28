//! Where the axis label and the offset text of an axis go: beyond the
//! spine, its ticks and the tick labels on their side (matplotlib's
//! `_update_label_position` and `_update_offset_text_position`).

use plt_core::{Bbox, Point};
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis_layout::{AxisPainter, PlacedText, place_text};
use crate::artist::LayoutContext;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::ticker::AxisName;

/// Distance between tick labels and the offset text, in points
/// (matplotlib's `Axis.OFFSETTEXTPAD`).
const OFFSET_TEXT_PAD: f64 = 3.0;

impl AxisPainter<'_> {
    /// The axis label beyond the spine, its ticks and the tick labels on
    /// its side.
    pub(super) fn axis_label(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        label_boxes: &[(bool, Bbox)],
    ) -> Option<Result<PlacedText, String>> {
        let label = &self.axis.label;
        if label.text.is_empty() || !label.visible {
            return None;
        }
        let extent = layout.axes_extent(self.axes_id)?;
        let pad = layout.points_to_pixels(label.pad);
        let high = label.side.is_high();
        let edge = self.spine_extent(layout, label.side).unwrap_or(extent);
        let boxes = label_boxes
            .iter()
            .filter(|(first, _)| *first != high)
            .map(|(_, b)| *b);
        let bounds = boxes.fold(edge, |acc, b| acc.union(&b));
        let along = label.halign();
        let multialign = label.multialign.unwrap_or(along);
        let (automatic, block) = match self.name {
            AxisName::X => (
                Point::new(
                    extent.x0() + label.loc.fraction() * extent.width(),
                    if high {
                        bounds.y1() + pad
                    } else {
                        bounds.y0() - pad
                    },
                ),
                BlockStyle {
                    rotation: label.rotation_on(self.name),
                    halign: along,
                    valign: label.valign_on(self.name),
                    multialign,
                    ..BlockStyle::default()
                },
            ),
            AxisName::Y => (
                Point::new(
                    if high {
                        bounds.x1() + pad
                    } else {
                        bounds.x0() - pad
                    },
                    extent.y0() + label.loc.fraction() * extent.height(),
                ),
                BlockStyle {
                    rotation: label.rotation_on(self.name),
                    halign: along,
                    valign: label.valign_on(self.name),
                    multialign,
                    rotation_mode: RotationMode::Anchor,
                    ..BlockStyle::default()
                },
            ),
        };
        let anchor = label.coords.map_or(automatic, |at| {
            Point::new(
                extent.x0() + at.x * extent.width(),
                extent.y0() + at.y * extent.height(),
            )
        });
        Some(place_text(
            layout,
            &label.text,
            &label.font,
            &label.color,
            &block,
            anchor,
            rc,
        ))
    }

    /// The offset text at the end of the axis, beyond the tick labels.
    pub(super) fn offset_text(
        &self,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        label_boxes: &[(bool, Bbox)],
    ) -> Option<Result<PlacedText, String>> {
        let text = &self.axis.ticks.as_ref().ok()?.offset;
        if text.is_empty() || !self.axis.offset_visible {
            return None;
        }
        let extent = layout.axes_extent(self.axes_id)?;
        let pad = layout.points_to_pixels(OFFSET_TEXT_PAD);
        let style = &self.axis.major_style;
        let font = FontProperties {
            size: style.label_size,
            ..FontProperties::from_rc(rc)
        };
        let high = self.axis.offset_side.is_high();
        let (anchor, block) = match self.name {
            AxisName::X => {
                let side_boxes = label_boxes.iter().filter(|(first, _)| *first != high);
                let (y, valign) = if high {
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
        };
        Some(place_text(
            layout,
            text,
            &font,
            &style.label_color,
            &block,
            anchor,
            rc,
        ))
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
            AxisPainter::new(ax.id(), axes, name).texts(LayoutContext::new(&state), &axes.rc);
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
