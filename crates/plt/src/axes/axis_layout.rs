//! Where the texts of an axis go — tick labels, the axis label and the
//! offset text, laid out in display coordinates — and how far its spines
//! reach with their ticks (matplotlib's `_get_ticklabel_bboxes`,
//! `_update_label_position`, `_update_offset_text_position` and
//! `Spine.get_window_extent`). Drawing and tight bounding boxes share
//! these placements.

use std::borrow::Cow;

use plt_core::{Bbox, Point, Rgba};
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis::{AxisState, Tick};
use super::state::AxesState;
use super::ticks::TickStyle;
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::spines::{SpineArtist, SpineSide};
use crate::text::{LaidOutText, lay_out};
use crate::ticker::AxisName;
use crate::transforms::Transform;
use crate::{geo, polar};

/// Distance between tick labels and the offset text, in points
/// (matplotlib's `Axis.OFFSETTEXTPAD`).
const OFFSET_TEXT_PAD: f64 = 3.0;

/// Everything needed to lay out and draw one axis of one axes.
pub(crate) struct AxisPainter<'a> {
    pub(super) axes_id: ArtistId,
    pub(super) axes: &'a AxesState,
    pub(super) axis: &'a AxisState,
    pub(super) name: AxisName,
}

/// A text of an axis, laid out at its anchor in display coordinates.
#[derive(Debug)]
pub(crate) struct PlacedText {
    /// The laid-out lines.
    pub(crate) laid_out: LaidOutText,
    /// The anchor in display coordinates.
    pub(crate) anchor: Point,
    /// The rotation in degrees.
    pub(crate) rotation: f64,
    /// The text colour.
    pub(crate) color: Rgba,
}

impl PlacedText {
    /// The extent of the text in display coordinates.
    pub(crate) fn bbox(&self) -> Bbox {
        self.laid_out
            .block
            .bbox
            .translated(self.anchor.x, self.anchor.y)
    }
}

/// A tick label and the tick it belongs to.
#[derive(Debug)]
pub(crate) struct TickLabel {
    /// The tick's index among the major ticks followed by the minor ones.
    pub(crate) tick: usize,
    /// Whether the label is on the first (bottom or left) side.
    pub(crate) first_side: bool,
    /// Whether the label counts in layouts and in placing the axis label
    /// and the offset text.
    pub(crate) in_layout: bool,
    /// The label.
    pub(crate) text: PlacedText,
}

/// The texts of an axis, laid out.
#[derive(Debug, Default)]
pub(crate) struct AxisTexts {
    /// The tick labels in tick order.
    pub(crate) tick_labels: Vec<TickLabel>,
    /// The axis label, if it has text.
    pub(crate) label: Option<PlacedText>,
    /// The offset text, if shown.
    pub(crate) offset: Option<PlacedText>,
    /// The anchors of the shown tick labels without text, each with
    /// whether it is on the first side: matplotlib counts them as empty
    /// boxes there when it places the axis label and the offset text.
    pub(crate) empty_label_anchors: Vec<(bool, Point)>,
    /// Texts that could not be laid out.
    pub(crate) problems: Vec<String>,
}

/// Where a tick label goes: its anchor in display coordinates, default
/// alignment, rotation and rotation mode.
struct LabelPlacement {
    anchor: Point,
    align: (HAlign, VAlign),
    rotation: f64,
    rotation_mode: RotationMode,
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

    /// The axis' ticks with their styles: the major ticks, then the minor
    /// ones (empty when the ticks could not be computed).
    pub(super) fn styled_ticks(&self) -> Vec<(&'a Tick, Cow<'a, TickStyle>)> {
        let Ok(ticks) = &self.axis.ticks else {
            return Vec::new();
        };
        let major = (ticks.major.iter())
            .zip(&ticks.major_index)
            .map(|(tick, &index)| (tick, self.axis.major_tick_style(index)));
        let minor = (ticks.minor.iter()).map(|tick| (tick, Cow::Borrowed(&self.axis.minor_style)));
        major.chain(minor).collect()
    }

    /// Lays out the tick labels, then the axis label beyond those of the
    /// axes it aligns with and the offset text beyond its own.
    pub(crate) fn texts(&self, layout: LayoutContext<'_>, rc: &RcParams) -> AxisTexts {
        let mut texts = AxisTexts::default();
        self.lay_out_tick_labels(layout, rc, &mut texts);
        let boxes = label_boxes(&texts);
        let aligned = self.aligned_label_boxes(layout, &boxes);
        for (slot, placed) in [
            (&mut texts.label, self.axis_label(layout, rc, &aligned)),
            (&mut texts.offset, self.offset_text(layout, rc, &boxes)),
        ] {
            match placed {
                Some(Ok(text)) => *slot = Some(text),
                Some(Err(problem)) => texts.problems.push(problem),
                None => {}
            }
        }
        texts
    }

    /// Lays out the tick labels into `texts` (matplotlib's
    /// `_get_ticklabel_bboxes`).
    fn lay_out_tick_labels(&self, layout: LayoutContext<'_>, rc: &RcParams, texts: &mut AxisTexts) {
        for (index, (tick, style)) in self.styled_ticks().into_iter().enumerate() {
            for (enabled, first_side) in [(style.labels.first, true), (style.labels.second, false)]
            {
                if !enabled {
                    continue;
                }
                let Some(placement) = self.label_placement(layout, tick, &style, first_side) else {
                    continue;
                };
                if tick.label.is_empty() {
                    if style.label_in_layout {
                        texts
                            .empty_label_anchors
                            .push((first_side, placement.anchor));
                    }
                    continue;
                }
                match lay_out_tick_label(layout, tick, &style, &placement, rc) {
                    Ok(text) => texts.tick_labels.push(TickLabel {
                        tick: index,
                        first_side,
                        in_layout: style.label_in_layout,
                        text,
                    }),
                    Err(problem) => texts.problems.push(problem),
                }
            }
        }
    }

    /// The tick label boxes of this axis (`own`) and of the same axis of
    /// the axes its label aligns with (matplotlib's
    /// `_get_tick_boxes_siblings`).
    fn aligned_label_boxes(
        &self,
        layout: LayoutContext<'_>,
        own: &[(bool, Bbox)],
    ) -> Vec<(bool, Bbox)> {
        let mut boxes = Vec::new();
        for sibling in layout.label_siblings(self.axes_id, self.name) {
            if sibling == self.axes_id {
                boxes.extend_from_slice(own);
            } else if let Some(axes) = layout.store().get_as::<AxesState>(sibling) {
                let mut texts = AxisTexts::default();
                AxisPainter::new(sibling, axes, self.name)
                    .lay_out_tick_labels(layout, &axes.rc, &mut texts);
                boxes.extend(label_boxes(&texts));
            }
        }
        boxes
    }

    /// The coordinate system of gridlines: data along the axis, axes
    /// fractions (or, on geographic axes, fractions of the gridline)
    /// across it (matplotlib's `get_xaxis_transform('grid')`).
    pub(super) fn grid_transform(&self) -> Transform {
        if let Some(geo) = self.axes.geo() {
            return geo::tick_transform(self.axes_id, geo, self.name);
        }
        let data = Transform::Data(self.axes_id);
        let across = Transform::Axes(self.axes_id);
        match self.name {
            AxisName::X => Transform::blended(data, across),
            AxisName::Y => Transform::blended(across, data),
        }
    }

    /// A point at `loc` along the axis and `across` (0 or 1) across it.
    pub(super) fn tick_point(&self, loc: f64, across: f64) -> Point {
        match self.name {
            AxisName::X => Point::new(loc, across),
            AxisName::Y => Point::new(across, loc),
        }
    }

    /// Where the label of `tick` on its first or second side goes,
    /// unless its anchor is not finite.
    fn label_placement(
        &self,
        layout: LayoutContext<'_>,
        tick: &Tick,
        style: &TickStyle,
        first_side: bool,
    ) -> Option<LabelPlacement> {
        let (anchor, align, rotation, rotation_mode) = if let Some(polar) = self.axes.polar_view() {
            let label = polar::tick_label_anchor(
                layout,
                self.axes_id,
                &polar,
                self.name,
                tick,
                style,
                first_side,
            )?;
            (
                label.anchor,
                label.align,
                label.rotation,
                RotationMode::Anchor,
            )
        } else if let Some(geo) = self.axes.geo() {
            let display = layout.axes_extent(self.axes_id)?;
            let (anchor, align) =
                geo::tick_label_anchor(geo, &display, self.name, tick.loc, first_side);
            (
                anchor,
                align,
                style.label_rotation,
                style.label_rotation_mode,
            )
        } else {
            let (anchor, align) = self.cartesian_label_anchor(layout, tick, style, first_side)?;
            (
                anchor,
                align,
                style.label_rotation,
                style.label_rotation_mode,
            )
        };
        anchor.is_finite().then_some(LabelPlacement {
            anchor,
            align,
            rotation,
            rotation_mode,
        })
    }

    /// Where a tick label of rectilinear axes goes: beyond the tick on
    /// its side, with the side's default alignment.
    fn cartesian_label_anchor(
        &self,
        layout: LayoutContext<'_>,
        tick: &Tick,
        style: &TickStyle,
        first_side: bool,
    ) -> Option<(Point, (HAlign, VAlign))> {
        let resolved = layout.resolve(&self.side_transform(layout, !first_side))?;
        let across = if first_side { 0.0 } else { 1.0 };
        let base = resolved.transform_point(self.tick_point(tick.loc, across));
        let offset = layout.points_to_pixels(style.label_offset());
        // `xtick.alignment` and `ytick.alignment` (matplotlib's
        // `get_xaxis_text1_transform` and friends).
        let rc = &self.axes.rc;
        let x_align = HAlign::parse(&rc.xtick_alignment).unwrap_or(HAlign::Center);
        let y_align = VAlign::parse(&rc.ytick_alignment).unwrap_or(VAlign::CenterBaseline);
        Some(match (self.name, first_side) {
            (AxisName::X, true) => (base - Point::new(0.0, offset), (x_align, VAlign::Top)),
            (AxisName::X, false) => (base + Point::new(0.0, offset), (x_align, VAlign::Bottom)),
            (AxisName::Y, true) => (base - Point::new(offset, 0.0), (HAlign::Right, y_align)),
            (AxisName::Y, false) => (base + Point::new(offset, 0.0), (HAlign::Left, y_align)),
        })
    }

    /// The axis label beyond the spine, its ticks and the tick labels on
    /// its side.
    fn axis_label(
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
        let along = label.loc.halign();
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
                    rotation: label.rotation.unwrap_or(0.0),
                    halign: along,
                    valign: label.valign.unwrap_or(if high {
                        VAlign::Baseline
                    } else {
                        VAlign::Top
                    }),
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
                    rotation: label.rotation.unwrap_or(90.0),
                    halign: along,
                    valign: label
                        .valign
                        .unwrap_or(if high { VAlign::Top } else { VAlign::Bottom }),
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
    fn offset_text(
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
                (
                    Point::new(x, extent.ymax() + pad),
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

    /// The spine on `side`, if the axes has one (the spines of curved
    /// projections do not frame the axes sides).
    fn spine<'c>(&self, layout: LayoutContext<'c>, side: SpineSide) -> Option<&'c SpineArtist> {
        if !self.axes.projection.is_rectilinear() {
            return None;
        }
        let index = SpineSide::ALL.iter().position(|s| *s == side)?;
        let spine_id = *self.axes.spines.get(index)?;
        layout.store().get_as::<SpineArtist>(spine_id)
    }

    /// The coordinate system of the ticks and labels on the low (bottom,
    /// left) or high side: that side's spine transform, so ticks follow a
    /// moved spine (matplotlib's `get_xaxis_transform('tick1')`).
    pub(super) fn side_transform(&self, layout: LayoutContext<'_>, high: bool) -> Transform {
        self.spine(layout, SpineSide::of(self.name, high))
            .map_or_else(
                || self.grid_transform(),
                |spine| spine.spine_transform(self.axes_id),
            )
    }

    /// The spine's extent widened by the outward/inward part of the ticks
    /// drawn on it (matplotlib's `Spine.get_window_extent`).
    pub(crate) fn spine_extent(&self, layout: LayoutContext<'_>, side: SpineSide) -> Option<Bbox> {
        let mut bbox = self.spine(layout, side)?.window_extent(&layout)?;
        let ticks = self.axis.ticks.as_ref().ok()?;
        for (present, style) in [
            (!ticks.major.is_empty(), &self.axis.major_style),
            (!ticks.minor.is_empty(), &self.axis.minor_style),
        ] {
            let marked = if side.is_high() {
                style.ticks.second
            } else {
                style.ticks.first
            };
            if !(present && marked) {
                continue;
            }
            let length = layout.points_to_pixels(style.size);
            let (out, inward) = (
                style.direction.outward_fraction() * length,
                (1.0 - style.direction.outward_fraction()) * length,
            );
            let (x0, y0, x1, y1) = (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1());
            let padded = match side {
                SpineSide::Bottom => Bbox::from_extents(x0, y0 - out, x1, y1 + inward),
                SpineSide::Left => Bbox::from_extents(x0 - out, y0, x1 + inward, y1),
                SpineSide::Right => Bbox::from_extents(x0 - inward, y0, x1 + out, y1),
                // matplotlib pads the inner edge of the top spine by the
                // outward length too.
                SpineSide::Top => Bbox::from_extents(x0, y0 - out, x1, y1 + out),
            };
            bbox = bbox.union(&padded);
        }
        Some(bbox)
    }
}

/// The label of `tick` in `style`, laid out at `placement`.
fn lay_out_tick_label(
    layout: LayoutContext<'_>,
    tick: &Tick,
    style: &TickStyle,
    placement: &LabelPlacement,
    rc: &RcParams,
) -> Result<PlacedText, String> {
    let font = style.label_font(rc);
    let (halign, valign) = placement.align;
    let halign = style.label_halign.unwrap_or(halign);
    let block = BlockStyle {
        halign,
        valign: style.label_valign.unwrap_or(valign),
        multialign: style.label_multialign.unwrap_or(halign),
        // Tick labels are texts, whose rotation matplotlib reduces modulo
        // 360 (which the `xtick` and `ytick` modes rely on).
        rotation: placement.rotation.rem_euclid(360.0),
        rotation_mode: placement.rotation_mode,
        ..BlockStyle::default()
    };
    place_text(
        layout,
        &tick.label,
        &font,
        &style.label_color,
        &block,
        placement.anchor,
        rc,
    )
}

/// The extents of the tick labels of `texts` that count in layouts, each
/// with whether it is on the first side; labels without text are empty
/// boxes at their anchors.
fn label_boxes(texts: &AxisTexts) -> Vec<(bool, Bbox)> {
    let empty = texts
        .empty_label_anchors
        .iter()
        .map(|&(first, at)| (first, Bbox::from_bounds(at.x, at.y, 0.0, 0.0)));
    texts
        .tick_labels
        .iter()
        .filter(|label| label.in_layout)
        .map(|label| (label.first_side, label.text.bbox()))
        .chain(empty)
        .collect()
}

/// `text` laid out at `anchor` with `font`, `block` and `color`.
pub(crate) fn place_text(
    layout: LayoutContext<'_>,
    text: &str,
    font: &FontProperties,
    color: &Color,
    block: &BlockStyle,
    anchor: Point,
    rc: &RcParams,
) -> Result<PlacedText, String> {
    let mut laid_out = lay_out(text, font, block, layout.dpi(), rc).map_err(|e| e.to_string())?;
    // The axis texts are drawn through the rc path effects.
    if !rc.path_effects.is_empty() {
        laid_out = laid_out
            .as_text_paths(font, rc.text_parse_math, layout.dpi(), rc)
            .map_err(|e| e.to_string())?;
    }
    let color = color.resolve().map_err(|e| e.to_string())?;
    Ok(PlacedText {
        laid_out,
        anchor,
        rotation: block.rotation,
        color,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::{Axes, TickParams};
    use crate::figure::Figure;

    /// The box of the label of axis `name` of `ax`.
    fn label_box(fig: &Figure, ax: &Axes, name: AxisName) -> Bbox {
        fig.layout();
        let state = fig.raw().figure_state().borrow();
        let axes = state.store.get_as::<AxesState>(ax.id()).expect("an axes");
        let texts =
            AxisPainter::new(ax.id(), axes, name).texts(LayoutContext::new(&state), &axes.rc);
        texts.label.expect("a label").bbox()
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
        ax.tick_params(&TickParams {
            labelbottom: Some(false),
            ..TickParams::default()
        });
        let hidden = xlabel_top(&fig, &ax);
        let pad = 3.5 * fig.dpi() / 72.0;
        assert!((hidden - empty - pad).abs() < 1e-9, "{hidden} - {empty}");
    }
}
