//! Where the texts of an axis go — tick labels, the axis label and the
//! offset text, laid out in display coordinates — and how far its spines
//! reach with their ticks (matplotlib's `_get_ticklabel_bboxes`,
//! `_update_label_position`, `_update_offset_text_position` and
//! `Spine.get_window_extent`). Drawing and tight bounding boxes share
//! these placements.

use std::borrow::Cow;

use pltrs_core::{Bbox, Point, Rgba};
use pltrs_text::{BlockStyle, HAlign, RotationMode, TextError, VAlign};

use super::axis::AxisState;
use super::state::AxesState;
use super::tick_set::Tick;
use super::ticks::TickStyle;
use crate::artist::{ArtistId, FigureReader, LayoutContext};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::patheffects::PathEffect;
use crate::polar;
use crate::rc::RcParams;
use crate::spines::{SpineArtist, SpineSide};
use crate::text::{
    LaidOutText, TextProblem, Typesetting, has_line_measurer, lay_out_as, metrics_cache_installed,
};
use crate::ticker::AxisName;
use crate::transforms::{Transform, program_calls};

/// Everything needed to lay out and draw one axis of one axes.
pub(crate) struct AxisPainter<'a> {
    pub(super) axes_id: ArtistId,
    pub(super) axes: &'a AxesState,
    pub(super) axis: &'a AxisState,
    pub(super) name: AxisName,
}

/// How matplotlib draws a text of an axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AxisTextDraw {
    /// Laid out again where it is drawn, after `Axis.draw` measured it: a
    /// tick label.
    AfterMeasuring,
    /// Laid out by its draw alone: the axis label and the offset text, the
    /// layout placing them.
    AsPlaced,
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
    /// Whether the text is antialiased (`None`: as `text.antialiased`
    /// says).
    pub(crate) antialiased: Option<bool>,
    /// The path effects the text is drawn through in place of
    /// `path.effects`, when the program gave it some
    /// ([`rc_with_effects`]).
    pub(crate) path_effects: Option<Vec<PathEffect>>,
}

impl PlacedText {
    /// The text as it is drawn. matplotlib's `Text.draw` lays a tick label
    /// out again where it draws it (`drawn_as`
    /// [`AxisTextDraw::AfterMeasuring`]; the layout placing an axis label
    /// is the one drawing it), measuring its lines a second time through
    /// the renderer's text metrics cache. That layout is the one
    /// made when placing the text, which is drawn; while such a cache is
    /// installed, its lines are looked up again
    /// ([`LaidOutText::measure_again`]), so that the cache sees the same
    /// lookups. The text is laid out again only where a second layout
    /// does more than look its lines up: while the program measures them
    /// (a [`LineMeasurer`](crate::text::LineMeasurer), which matplotlib
    /// calls again too), and under the rc path effects, whose glyphs are
    /// placed from text paths measured through the cache as well.
    ///
    /// # Errors
    ///
    /// When the text cannot be measured or laid out again.
    pub(crate) fn as_drawn(
        &self,
        dpi: f64,
        rc: &RcParams,
        drawn_as: AxisTextDraw,
    ) -> Result<Cow<'_, LaidOutText>, TextError> {
        let laid_out = &self.laid_out;
        if drawn_as == AxisTextDraw::AsPlaced || !metrics_cache_installed() {
            return Ok(Cow::Borrowed(laid_out));
        }
        if rc.path_effects.is_empty() && !has_line_measurer() {
            laid_out.measure_again(dpi, rc)?;
            return Ok(Cow::Borrowed(laid_out));
        }
        lay_out_axis_text(
            &laid_out.lines.join("\n"),
            &laid_out.font,
            &laid_out.style,
            dpi,
            rc,
            laid_out.typesetting,
        )
        .map(Cow::Owned)
    }

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
    /// Where the axis label goes unless placed by coordinates, if it has
    /// text and is shown (see [`AxisPainter::axis_label_anchor`]).
    pub(crate) label_anchor: Option<Point>,
    /// The offset text, if shown.
    pub(crate) offset: Option<PlacedText>,
    /// Where the offset text is anchored in display coordinates, shown or
    /// not.
    pub(crate) offset_anchor: Option<Point>,
    /// The anchors of the shown tick labels without text, each with
    /// whether it is on the first side: matplotlib counts them as empty
    /// boxes there when it places the axis label and the offset text.
    pub(crate) empty_label_anchors: Vec<(bool, Point)>,
    /// Whether each shown tick label whose anchor is not finite is on the
    /// first side (the transform of its axes is not finite: a view its
    /// scale maps onto one point): matplotlib measures them at NaN, which
    /// is where it then puts the axis label and the offset text.
    pub(crate) unplaced_labels: Vec<bool>,
    /// Texts that could not be laid out.
    pub(crate) problems: Vec<TextProblem>,
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
    /// The painter of axis `name` of the axes `axes_id`, whose state is
    /// `axes`.
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
        self.ticks_with(|axis, minor, index| axis.tick_style_at(minor, index))
    }

    /// The axis' ticks drawn, the major then the minor ones, each with its
    /// level (minor or not) and its index among those the locator places.
    pub(super) fn located_ticks(&self) -> Vec<(&'a Tick, bool, usize)> {
        (self
            .ticks_with(|_, minor, index| (minor, index))
            .into_iter())
        .map(|(tick, (minor, index))| (tick, minor, index))
        .collect()
    }

    /// The axis' ticks, the major then the minor ones, each with `style`
    /// of the axis, the level (minor or not) and the tick's index among
    /// those the locator places.
    fn ticks_with<S>(&self, style: impl Fn(&'a AxisState, bool, usize) -> S) -> Vec<(&'a Tick, S)> {
        let Ok(ticks) = &self.axis.ticks else {
            return Vec::new();
        };
        let level = |ticks: &'a [Tick], indices: &'a [usize], minor: bool| {
            (ticks.iter())
                .zip(indices)
                .map(|(tick, &index)| (tick, style(self.axis, minor, index)))
                .collect::<Vec<_>>()
        };
        let mut styled = level(&ticks.major, &ticks.major_index, false);
        styled.extend(level(&ticks.minor, &ticks.minor_index, true));
        styled
    }

    /// Lays out the tick labels, then the axis label beyond those of the
    /// axes it aligns with and the offset text beyond its own; a draw lets
    /// the figure go around the program's code this runs (`cx`). The
    /// program's scales answer from the calls matplotlib makes to place
    /// these texts (see [`program_calls`]).
    pub(crate) fn texts(&self, cx: &mut impl FigureReader, rc: &RcParams) -> AxisTexts {
        program_calls::quietly(|| self.texts_quietly(cx, rc))
    }

    /// [`texts`](Self::texts), the program's scales answering quietly.
    fn texts_quietly(&self, cx: &mut impl FigureReader, rc: &RcParams) -> AxisTexts {
        let mut texts = AxisTexts::default();
        self.lay_out_tick_labels(cx, rc, &mut texts);
        self.place_label_and_offset(cx, rc, &mut texts);
        texts
    }

    /// Places the axis label and the offset text of `texts` from its tick
    /// labels. Out of [`texts_quietly`](Self::texts_quietly)'s frame,
    /// which the tick labels' layout runs below (on what is left of the
    /// stack in a nested draw).
    #[inline(never)]
    fn place_label_and_offset(
        &self,
        cx: &mut impl FigureReader,
        rc: &RcParams,
        texts: &mut AxisTexts,
    ) {
        let boxes = label_boxes(texts);
        if self.axis.label.coords.is_none() {
            // matplotlib places the label from the tick labels measured
            // again (`_update_label_position`'s `_get_tick_boxes_siblings`).
            let dpi = cx.read().dpi();
            cx.release_if(has_line_measurer);
            for label in &texts.tick_labels {
                if let Err(err) = label.text.laid_out.measure_again(dpi, rc) {
                    texts.problems.push(err.into());
                }
            }
        }
        texts.offset_anchor = self
            .offset_placement(cx.read(), &boxes)
            .map(|(anchor, _)| anchor);
        self.axis.offset_placed.set(texts.offset_anchor);
        let aligned = self.aligned_label_boxes(cx, &boxes);
        // matplotlib places the label whatever its text and visibility
        // (`_update_label_position`, which asks the spine for its extent),
        // and draws it only with text.
        texts.label_anchor = self.axis_label_anchor(cx, &aligned);
        self.axis.label_placed.set(texts.label_anchor);
        let label = self.axis_label(cx, rc, texts.label_anchor);
        let offset = self.offset_text(cx, rc, &boxes);
        for (slot, placed) in [(&mut texts.label, label), (&mut texts.offset, offset)] {
            match placed {
                Some(Ok(text)) => *slot = Some(text),
                Some(Err(problem)) => texts.problems.push(problem),
                None => {}
            }
        }
    }

    /// Lays out the tick labels into `texts` (matplotlib's
    /// `_get_ticklabel_bboxes`).
    fn lay_out_tick_labels(
        &self,
        cx: &mut impl FigureReader,
        rc: &RcParams,
        texts: &mut AxisTexts,
    ) {
        let labelled = self.ticks_with(|_, minor, index| (minor, index));
        for (index, (tick, (minor, at))) in labelled.into_iter().enumerate() {
            for first_side in [true, false] {
                let Some((style, placement)) = self.labelled(cx, tick, (minor, at), first_side)
                else {
                    continue;
                };
                if !placement.anchor.is_finite() {
                    if style.label_in_layout {
                        texts.unplaced_labels.push(first_side);
                    }
                    continue;
                }
                if tick.label.is_empty() {
                    if style.label_in_layout {
                        texts
                            .empty_label_anchors
                            .push((first_side, placement.anchor));
                    }
                    continue;
                }
                match lay_out_tick_label(cx, tick, &style, &placement, rc) {
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

    /// The style and placement of the label of `tick` on its first or
    /// second side, `(minor, index)` naming the tick among those the
    /// locator places, its anchor finite or not; `None` when that side
    /// shows no label or its projection places none.
    fn labelled(
        &self,
        cx: &mut impl FigureReader,
        tick: &Tick,
        (minor, index): (bool, usize),
        first_side: bool,
    ) -> Option<(Cow<'a, TickStyle>, LabelPlacement)> {
        let style = self.axis.tick_label_style_at(minor, index, first_side);
        let enabled = if first_side {
            style.labels.first
        } else {
            style.labels.second
        };
        // A label not shown is not placed: matplotlib's places none
        // (through the program's transforms).
        if !enabled {
            return None;
        }
        let placement = self.label_placement(cx, tick, &style, first_side)?;
        Some((style, placement))
    }

    /// The label of the major (or `minor`) tick at `index` among those the
    /// locator places, on its first or second side, laid out where
    /// matplotlib keeps the label's `Text`, drawn or not (a tick outside
    /// the view is placed without being drawn); `None` for a side without
    /// a label, a label without text or a tick not placed.
    pub(super) fn tick_label(
        &self,
        mut layout: LayoutContext<'_>,
        rc: &RcParams,
        (minor, index): (bool, usize),
        first_side: bool,
    ) -> Option<Result<PlacedText, TextProblem>> {
        let ticks = self.axis.ticks.as_ref().ok()?;
        let tick = ticks.unclipped(minor).get(index)?;
        program_calls::quietly(|| {
            let (style, placement) =
                self.labelled(&mut layout, tick, (minor, index), first_side)?;
            (!tick.label.is_empty() && placement.anchor.is_finite())
                .then(|| lay_out_tick_label(&mut layout, tick, &style, &placement, rc))
        })
    }

    /// The tick label boxes of this axis (`own`) and of the same axis of
    /// the axes its label aligns with (matplotlib's
    /// `_get_tick_boxes_siblings`).
    // Out of the frame laying the axis label out, which text layout runs below.
    #[inline(never)]
    fn aligned_label_boxes(
        &self,
        cx: &mut impl FigureReader,
        own: &[(bool, Bbox)],
    ) -> Vec<(bool, Bbox)> {
        let mut boxes = Vec::new();
        for sibling in cx.read().label_siblings(self.axes_id, self.name) {
            if sibling == self.axes_id {
                boxes.extend_from_slice(own);
                continue;
            }
            // Held apart from the figure, which their labels may let go.
            let axes = cx.read().store().shared_as::<AxesState>(sibling);
            if let Some(axes) = axes {
                let mut texts = AxisTexts::default();
                AxisPainter::new(sibling, &axes, self.name)
                    .lay_out_tick_labels(cx, &axes.rc, &mut texts);
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
        point_on_axis(self.name, loc, across)
    }

    /// Where the label of `tick` on its first or second side goes: its
    /// anchor is not finite where the transform of the axes is not.
    fn label_placement(
        &self,
        cx: &mut impl FigureReader,
        tick: &Tick,
        style: &TickStyle,
        first_side: bool,
    ) -> Option<LabelPlacement> {
        let (anchor, align, rotation, rotation_mode) = if let Some(polar) = self.axes.polar_view() {
            let label = polar::tick_label_anchor(
                cx,
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
            let (anchor, align) = self.label_anchor(cx, tick, style, first_side)?;
            (
                anchor,
                align,
                style.label_rotation,
                style.label_rotation_mode,
            )
        };
        Some(LabelPlacement {
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
        cx: &mut impl FigureReader,
        tick: &Tick,
        style: &TickStyle,
        first_side: bool,
    ) -> Option<(Point, (HAlign, VAlign))> {
        if self.axes.projection.custom().is_some() {
            let side = SpineSide::of(self.name, !first_side);
            let display = cx.read().axes_extent(self.axes_id)?;
            // The projection asks the program, with the figure free.
            cx.release_if(|| true);
            if let Some(placed) = self.axes.custom_tick_label(side, tick.loc, &display) {
                return Some(placed);
            }
        }
        self.cartesian_label_anchor(cx, tick, style, first_side)
    }

    /// Where a tick label of rectilinear axes goes: beyond the tick on
    /// its side (or where along and across the axis its program moved
    /// it), with the side's default alignment.
    fn cartesian_label_anchor(
        &self,
        cx: &mut impl FigureReader,
        tick: &Tick,
        style: &TickStyle,
        first_side: bool,
    ) -> Option<(Point, (HAlign, VAlign))> {
        let pad = style.label_offset();
        let align = tick_label_alignment(self.name, first_side, &self.axes.rc);
        let (dx, dy) = match (self.name, first_side) {
            (AxisName::X, true) => (0.0, -pad),
            (AxisName::X, false) => (0.0, pad),
            (AxisName::Y, true) => (-pad, 0.0),
            (AxisName::Y, false) => (pad, 0.0),
        };
        // The pad joins the side's transform as matplotlib's
        // `ScaledTranslation` joins it: in inches scaled by the dpi, in one
        // affine with the side's, so the label lands on matplotlib's bits.
        let transform = self
            .side_transform(cx.read(), !first_side)
            .offset_points(dx, dy);
        let across = style
            .label_across
            .unwrap_or(if first_side { 0.0 } else { 1.0 });
        let resolved = cx.read().resolve(&transform)?;
        cx.release_for(&resolved);
        let along = style.label_along.unwrap_or(tick.loc);
        let anchor = resolved.transform_point(self.tick_point(along, across));
        Some((anchor, align))
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
    /// moved spine (matplotlib's `get_xaxis_transform('tick1')`), or the one
    /// of the spine they were made on, which the program replaced since.
    pub(super) fn side_transform(&self, layout: LayoutContext<'_>, high: bool) -> Transform {
        if let Some(kept) = &self.axis.kept_tick_sides[usize::from(high)] {
            // The spine the ticks were made on, replaced since.
            return kept.clone();
        }
        self.spine(layout, SpineSide::of(self.name, high))
            .map_or_else(
                || self.grid_transform(),
                |spine| spine.spine_transform(self.axes_id),
            )
    }

    /// The spine's extent widened by the outward/inward part of the ticks
    /// drawn on it (matplotlib's `Spine.get_window_extent`): the major and
    /// the minor ticks each pad the bare spine, so the longer ones count.
    /// `None` on a side the axes has no spine of: what goes beyond the
    /// spine then goes beyond the axes (matplotlib's
    /// `spines.get("bottom", self.axes)`, where a projection class made a
    /// single spine around its outline).
    pub(crate) fn spine_extent(&self, cx: &mut impl FigureReader, side: SpineSide) -> Option<Bbox> {
        program_calls::quietly(|| self.spine_extent_quietly(cx, side))
    }

    /// [`spine_extent`](Self::spine_extent), the program's scales
    /// answering quietly.
    fn spine_extent_quietly(&self, cx: &mut impl FigureReader, side: SpineSide) -> Option<Bbox> {
        if !self.axes.projection.has_spine(side) {
            return None;
        }
        let outline = {
            let layout = cx.read();
            self.spine(layout, side)?.outline_through(layout)?
        };
        cx.release_for(outline.transform());
        let spine = outline.extent();
        let mut bbox = spine;
        let layout = cx.read();
        if self.axis.ticks.is_err() {
            return None;
        }
        // matplotlib pads by one drawn major tick and one drawn minor one,
        // each with what it has of its own (a mark hidden on it), taken
        // from a set of them: by any drawn tick showing its mark, which is
        // the same whenever the drawn ticks agree.
        for (_, style) in self.styled_ticks() {
            let marked = if side.is_high() {
                style.ticks.second
            } else {
                style.ticks.first
            };
            if !marked {
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

/// The label of `tick` in `style`, laid out at `placement`; a draw lets
/// the figure go while the program measures it (`cx`).
fn lay_out_tick_label(
    cx: &mut impl FigureReader,
    tick: &Tick,
    style: &TickStyle,
    placement: &LabelPlacement,
    rc: &RcParams,
) -> Result<PlacedText, TextProblem> {
    let rc = &*rc_with_effects(rc, style.label_path_effects.as_deref());
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
    let placed = place_text_measured(
        cx,
        &tick.label,
        &font,
        &style.label_color,
        &block,
        placement.anchor,
        rc,
        Typesetting::from_rc(rc),
    )?;
    Ok(PlacedText {
        url: style.label_url.clone(),
        antialiased: style.label_antialiased,
        path_effects: style.label_path_effects.clone(),
        ..placed
    })
}

/// The rcParams an axis text is laid out and drawn with: the axes' own,
/// or a copy on the heap for a text with path effects of its own (the
/// parameters are too large to copy onto the stack of every text's
/// layout and draw).
#[derive(Debug)]
pub(crate) enum TextRc<'a> {
    /// The parameters of the axes.
    Shared(&'a RcParams),
    /// Those with the text's own path effects.
    Own(Box<RcParams>),
}

impl std::ops::Deref for TextRc<'_> {
    type Target = RcParams;

    fn deref(&self) -> &RcParams {
        match self {
            Self::Shared(rc) => rc,
            Self::Own(rc) => rc,
        }
    }
}

/// The rcParams an axis text with path effects of its own is laid out
/// and drawn with: `rc` with `own` in place of `path.effects`
/// (matplotlib's `Text` keeps the effects it was given, which the
/// rcParam only gives a text made without any), `rc` itself without
/// `own`.
pub(crate) fn rc_with_effects<'a>(rc: &'a RcParams, own: Option<&[PathEffect]>) -> TextRc<'a> {
    match own {
        None => TextRc::Shared(rc),
        Some(effects) => {
            let mut rc = Box::new(rc.clone());
            rc.path_effects = effects.to_vec();
            TextRc::Own(rc)
        }
    }
}

/// The extents of the tick labels of `texts` that count in layouts, each
/// with whether it is on the first side; labels without text are empty
/// boxes at their anchors.
/// The alignment of the tick labels of axis `name` on its first or second
/// side before any of their own: `xtick.alignment` along an x axis, below
/// or above it, `ytick.alignment` across a y axis, right or left of it
/// (matplotlib's `get_xaxis_text1_transform` and friends).
pub(crate) fn tick_label_alignment(
    name: AxisName,
    first_side: bool,
    rc: &RcParams,
) -> (HAlign, VAlign) {
    match (name, first_side) {
        (AxisName::X, true) => (rc.xtick_alignment, VAlign::Top),
        (AxisName::X, false) => (rc.xtick_alignment, VAlign::Bottom),
        (AxisName::Y, true) => (HAlign::Right, rc.ytick_alignment),
        (AxisName::Y, false) => (HAlign::Left, rc.ytick_alignment),
    }
}

/// The boxes of the tick labels in the layout, each with whether it is on
/// the first side: those of the labels laid out, empty ones at the anchors
/// of the labels without text, and boxes at NaN for the labels whose
/// anchor is not finite (see [`measured_at_nan`]).
fn label_boxes(texts: &AxisTexts) -> Vec<(bool, Bbox)> {
    let empty = texts
        .empty_label_anchors
        .iter()
        .map(|&(first, at)| (first, Bbox::from_bounds(at.x, at.y, 0.0, 0.0)));
    let unplaced = (texts.unplaced_labels.iter())
        .map(|&first| (first, Bbox::from_bounds(f64::NAN, f64::NAN, 0.0, 0.0)));
    texts
        .tick_labels
        .iter()
        .filter(|label| label.in_layout)
        .map(|label| (label.first_side, label.text.bbox()))
        .chain(empty)
        .chain(unplaced)
        .collect()
}

/// Whether a tick label on the high (or low) side among `boxes` is
/// measured at NaN: the union matplotlib takes of the boxes is then at
/// NaN, and so is what it places beyond them.
pub(super) fn measured_at_nan(boxes: &[(bool, Bbox)], high: bool) -> bool {
    (boxes.iter()).any(|(first, bbox)| *first != high && bbox.x0().is_nan())
}

/// `text` laid out at `dpi` with `font` and `block`, typeset as
/// `typesetting` says, its glyphs placed as the rc path effects draw them
/// (the axis texts are drawn through them).
fn lay_out_axis_text(
    text: &str,
    font: &FontProperties,
    block: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
    typesetting: Typesetting,
) -> Result<LaidOutText, TextError> {
    let laid_out = lay_out_as(text, font, block, dpi, rc, typesetting)?;
    if rc.path_effects.is_empty() {
        return Ok(laid_out);
    }
    laid_out.as_text_paths(font, rc.text_parse_math, dpi, rc)
}

/// `text` laid out at `anchor` with `font`, `block` and `color`.
pub(crate) fn place_text(
    mut layout: LayoutContext<'_>,
    text: &str,
    font: &FontProperties,
    color: &Color,
    block: &BlockStyle,
    anchor: Point,
    rc: &RcParams,
) -> Result<PlacedText, String> {
    place_text_measured(
        &mut layout,
        text,
        font,
        color,
        block,
        anchor,
        rc,
        Typesetting::from_rc(rc),
    )
    .map_err(|problem| problem.message)
}

/// [`place_text`] typeset as `typesetting` says, letting the figure go
/// while the program measures the text (`cx`).
#[expect(
    clippy::too_many_arguments,
    reason = "the text, its look and placement, and how it is typeset"
)]
pub(crate) fn place_text_measured(
    cx: &mut impl FigureReader,
    text: &str,
    font: &FontProperties,
    color: &Color,
    block: &BlockStyle,
    anchor: Point,
    rc: &RcParams,
    typesetting: Typesetting,
) -> Result<PlacedText, TextProblem> {
    let dpi = cx.read().dpi();
    cx.release_if(has_line_measurer);
    let laid_out = lay_out_axis_text(text, font, block, dpi, rc, typesetting)?;
    let color = color.resolve()?;
    Ok(PlacedText {
        laid_out,
        anchor,
        rotation: block.rotation,
        color,
        url: None,
        antialiased: None,
        path_effects: None,
    })
}

/// A point at `loc` along axis `name` and `across` (0 or 1) across it.
pub(super) fn point_on_axis(name: AxisName, loc: f64, across: f64) -> Point {
    match name {
        AxisName::X => Point::new(loc, across),
        AxisName::Y => Point::new(across, loc),
    }
}
