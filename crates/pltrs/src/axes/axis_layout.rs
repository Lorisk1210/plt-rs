//! Where the texts of an axis go — tick labels, the axis label and the
//! offset text, laid out in display coordinates — and how far its spines
//! reach with their ticks (matplotlib's `_get_ticklabel_bboxes`,
//! `_update_label_position`, `_update_offset_text_position` and
//! `Spine.get_window_extent`). Drawing and tight bounding boxes share
//! these placements.

use std::borrow::Cow;

use pltrs_core::{Bbox, Point, Rgba};
use pltrs_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis::AxisState;
use super::state::AxesState;
use super::tick_set::Tick;
use super::ticks::TickStyle;
use crate::artist::{ArtistId, LayoutContext};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::polar;
use crate::rc::RcParams;
use crate::spines::{SpineArtist, SpineSide};
use crate::text::{LaidOutText, lay_out};
use crate::ticker::AxisName;
use crate::transforms::Transform;

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
    /// The hyperlink of the text in vector output.
    pub(crate) url: Option<String>,
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
    /// fractions across it (matplotlib's `get_xaxis_transform('grid')`),
    /// which projections such as the geographic ones redefine.
    pub(super) fn grid_transform(&self) -> Transform {
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
        } else {
            let (anchor, align) = self.label_anchor(layout, tick, style, first_side)?;
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

    /// Where a tick label of axes other than polar ones goes, with its
    /// default alignment: where their projection puts it (geographic axes
    /// and custom projections), else beyond the tick on its side.
    fn label_anchor(
        &self,
        layout: LayoutContext<'_>,
        tick: &Tick,
        style: &TickStyle,
        first_side: bool,
    ) -> Option<(Point, (HAlign, VAlign))> {
        if self.axes.projection.custom().is_some() {
            let side = SpineSide::of(self.name, !first_side);
            let display = layout.axes_extent(self.axes_id)?;
            if let Some(placed) = self.axes.custom_tick_label(side, tick.loc, &display) {
                return Some(placed);
            }
        }
        self.cartesian_label_anchor(layout, tick, style, first_side)
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
        let (x_align, y_align) = (self.axes.rc.xtick_alignment, self.axes.rc.ytick_alignment);
        Some(match (self.name, first_side) {
            (AxisName::X, true) => (base - Point::new(0.0, offset), (x_align, VAlign::Top)),
            (AxisName::X, false) => (base + Point::new(0.0, offset), (x_align, VAlign::Bottom)),
            (AxisName::Y, true) => (base - Point::new(offset, 0.0), (HAlign::Right, y_align)),
            (AxisName::Y, false) => (base + Point::new(offset, 0.0), (HAlign::Left, y_align)),
        })
    }

    /// The spine on `side`, if the axes has one (the spines of curved
    /// projections do not frame the axes sides).
    fn spine<'c>(&self, layout: LayoutContext<'c>, side: SpineSide) -> Option<&'c SpineArtist> {
        if !self.axes.projection.is_rectangular() {
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
    /// drawn on it (matplotlib's `Spine.get_window_extent`): the major and
    /// the minor ticks each pad the bare spine, so the longer ones count.
    pub(crate) fn spine_extent(&self, layout: LayoutContext<'_>, side: SpineSide) -> Option<Bbox> {
        let spine = self.spine(layout, side)?.line_extent(layout)?;
        let mut bbox = spine;
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
            let (x0, y0, x1, y1) = (spine.x0(), spine.y0(), spine.x1(), spine.y1());
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
    let placed = place_text(
        layout,
        &tick.label,
        &font,
        &style.label_color,
        &block,
        placement.anchor,
        rc,
    )?;
    Ok(PlacedText {
        url: style.label_url.clone(),
        ..placed
    })
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
        url: None,
    })
}
