//! The axis artist: the line, ticks, tick labels, offset text and label
//! of one axis line, drawn instead of the axes' spines and axes
//! (matplotlib's `axisartist.axis_artist.AxisArtist`).

use std::cell::Cell;
use std::rc::Rc;

use pltrs_core::{Bbox, Path, Point};
use pltrs_render::GraphicsContext;

use super::grid_helper_curvelinear::AxisGrid;
use super::handles::LabelDirection;
use super::helper::{AxisHelper, GONE, HelperContext};
use super::labels::{
    LabelKind, LabelState, PlacedTickLabels, TextLook, TextStyle, axis_label, offset_text,
    tick_labels,
};
use super::line::LineState;
use super::ticks::{TicksState, tick_info};
use crate::artist::{Artist, ArtistBase, ArtistId, ClipPathSpec, DrawContext, LayoutContext};
use crate::axes::axis_draw::{AxisTextDraw, draw_text};
use crate::axes::{AxesState, PlacedText};
use crate::figure::FigureState;
use crate::fonts::FontProperties;
use crate::patches::{DisplayPath, paint_paths};
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform};

/// The z-order of axis artists (matplotlib's `AxisArtist.zorder`).
mod extents;

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
#[derive(Clone, Debug)]
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
    /// The gap the tick labels left for the label when they were last
    /// laid out, in pixels (matplotlib's `TickLabels._axislabel_pad`):
    /// measuring the artist places the label by it before laying the tick
    /// labels out again, so a first measurement leaves no gap.
    tick_labels_gap: Cell<f64>,
    /// The gap the label was last placed with, in pixels (matplotlib's
    /// `AxisLabel._external_pad`).
    pub(crate) label_gap: Cell<f64>,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(AxisArtistState {
    base,
    name,
    axes,
    helper,
    style_axis,
    offset_side,
    offset,
    ticklabel_direction,
    axislabel_direction,
    line,
    major_ticks,
    minor_ticks,
    major_ticklabels,
    minor_ticklabels,
    label,
    label_text,
    tick_labels_gap,
    label_gap,
    rc
});

/// Which gap left by the tick labels places the label, and which gaps a
/// layout remembers, as matplotlib's callers of the parts' layout keep
/// them (`TickLabels._axislabel_pad`, `AxisLabel._external_pad`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelGap {
    /// The one the tick labels leave now, as drawing uses; both are
    /// remembered.
    Current,
    /// The one they left when last laid out, as matplotlib's
    /// `AxisArtist.get_tightbbox` uses (it updates the label first); both
    /// are remembered.
    Previous,
    /// The gap the tick labels leave now is remembered, the label's left
    /// as it was (matplotlib's `TickLabels.get_window_extents`).
    TickLabelsOnly,
    /// The one the label was last placed with, remembering nothing
    /// (matplotlib's `AxisLabel.get_window_extent`).
    Kept,
}

/// The marks (locations and angles, unless hidden) and labels of one
/// tick level, laid out.
struct LevelDrawing {
    marks: Option<Vec<(Point, f64)>>,
    /// What places the marks.
    marks_transform: ResolvedTransform,
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
    /// The axes the artist is drawn in.
    pub(crate) fn axes(&self) -> ArtistId {
        self.axes
    }

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
                rc.xtick_direction,
                rc.xtick_labelsize,
            ),
            AxisName::Y => (
                rc.ytick_major_size,
                rc.ytick_minor_size,
                rc.ytick_major_pad,
                rc.ytick_minor_pad,
                rc.ytick_direction,
                rc.ytick_labelsize,
            ),
        };
        let look = |weight| TextLook::new(FontProperties::from_rc(&rc).with_weight(weight));
        let tick_font = (labelsize.to_points(rc.font_size), look(rc.font_weight));
        let label_font = (
            rc.axes_labelsize.to_points(rc.font_size),
            look(rc.axes_labelweight),
        );
        let tick_labels =
            |pad| LabelState::new(LabelKind::TickLabels, direction, pad, tick_font.clone());
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
            tick_labels_gap: Cell::new(0.0),
            label_gap: Cell::new(0.0),
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

    /// The locations (in the coordinates of the helper's tick transform)
    /// and angles in degrees of the tick marks of `level`, as drawing
    /// them now places them (matplotlib's `Ticks.locs_angles` after
    /// `_update_ticks`).
    pub(crate) fn tick_locs_angles(
        &self,
        layout: LayoutContext<'_>,
        level: Level,
    ) -> Result<Vec<(Point, f64)>, String> {
        let axes = layout.store().get_as::<AxesState>(self.axes).ok_or(GONE)?;
        let ctx = HelperContext {
            layout,
            axes,
            id: self.axes,
        };
        let (major, minor) = self.helper.ticks(ctx)?;
        let ticks = match level {
            Level::Major => major,
            Level::Minor => minor,
        };
        Ok(tick_info(&ticks, self.ticklabel_direction.add_angle()).0)
    }

    /// Lays out everything the artist draws (matplotlib's `_draw_ticks`,
    /// `_draw_line` and `_draw_label`), an arrowed line at `dpi_cor`
    /// pixels per point and the label clear of the tick labels by the gap
    /// `label_gap` names; remembers the gap they leave now.
    fn drawing(
        &self,
        layout: LayoutContext<'_>,
        dpi_cor: f64,
        label_gap: LabelGap,
    ) -> Result<Drawing, String> {
        let axes = layout.store().get_as::<AxesState>(self.axes).ok_or(GONE)?;
        let ctx = HelperContext {
            layout,
            axes,
            id: self.axes,
        };
        let (major, minor) = self.helper.ticks(ctx)?;
        let transform = self.resolve(layout, self.helper.tick_transform(self.axes))?;
        // Both levels of tick labels clear the major ticks.
        let external = layout.measuring_points_to_pixels(self.major_ticks.outward());
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
                    &part_transform(layout, labels_state.transform.as_ref(), &transform)?,
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
                marks_transform: part_transform(
                    layout,
                    marks_state.transform.as_ref(),
                    &transform,
                )?,
                texts,
            });
        }
        let offset = if self.major_ticklabels.visible || self.minor_ticklabels.visible {
            self.offset_text(layout, axes)?
        } else {
            None
        };
        if label_gap == LabelGap::Kept {
            axislabel_pad = self.label_gap.get();
        } else {
            let previous_gap = self.tick_labels_gap.replace(axislabel_pad);
            if label_gap == LabelGap::Previous {
                axislabel_pad = previous_gap;
            }
            if self.ticklabel_direction != self.axislabel_direction {
                // The label faces away from the tick labels and clears
                // only the inward ticks, measured in points as if pixels
                // (like matplotlib).
                axislabel_pad = self.major_ticks.inward().max(self.minor_ticks.inward());
            }
            if label_gap != LabelGap::TickLabelsOnly {
                self.label_gap.set(axislabel_pad);
            }
        }
        let (line, line_grid) = self.line_paths(layout, ctx, dpi_cor)?;
        Ok(Drawing {
            levels,
            offset,
            line,
            label: self.axis_label(layout, ctx, axislabel_pad, line_grid.as_ref())?,
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

    /// The line in display pixels, plain or arrowed with arrow sizes at
    /// `dpi_cor` pixels per point (matplotlib's `_draw_line`; the arrow is
    /// a `FancyArrowPatch`).
    /// The grid a curvilinear line of constant coordinate was computed
    /// from comes along, which matplotlib's `_draw_line` computes for a
    /// hidden line too (`get_line`).
    fn line_paths(
        &self,
        layout: LayoutContext<'_>,
        ctx: HelperContext<'_>,
        dpi_cor: f64,
    ) -> Result<(Vec<DisplayPath>, Option<AxisGrid>), String> {
        let (path, transform, line_grid) = self.helper.line_with_grid(ctx)?;
        if !self.line.visible {
            return Ok((Vec::new(), line_grid));
        }
        let transform = self.resolve(layout, transform)?;
        let paths = self.line.display_paths(
            &path,
            &transform,
            self.major_ticklabels.size,
            &self.rc,
            dpi_cor,
        )?;
        Ok((paths, line_grid))
    }

    /// The axis label, `axislabel_pad` pixels plus its pad away from the
    /// line (matplotlib's `_update_label` and `AxisLabel.draw`), placed
    /// from `line_grid`, the grid of a curvilinear line, as computed for
    /// the line. A shown label is placed with or without text, as
    /// matplotlib's `_update_label` places it.
    fn axis_label(
        &self,
        layout: LayoutContext<'_>,
        ctx: HelperContext<'_>,
        axislabel_pad: f64,
        line_grid: Option<&AxisGrid>,
    ) -> Result<Option<PlacedText>, String> {
        let style_axis = ctx.axes.axis(self.style_axis);
        let text = self.label_text.as_deref().unwrap_or(&style_axis.label.text);
        if !self.label.visible {
            return Ok(None);
        }
        let position = self.helper.label_position(ctx, line_grid)?;
        if text.is_empty() {
            return Ok(None);
        }
        let (reference, radius) = self.label_placement(layout, ctx, axislabel_pad, position)?;
        let style = self.label.text_style(&self.rc, &style_axis.label.color);
        axis_label(
            layout,
            text,
            reference,
            radius,
            &self.label,
            &style,
            &self.rc,
        )
        .map(Some)
    }

    /// Where the label goes, `axislabel_pad` pixels plus its pad away from
    /// the line: its reference point in display pixels, the direction to
    /// it in degrees, and its distance from the point in pixels.
    pub(crate) fn label_placement(
        &self,
        layout: LayoutContext<'_>,
        ctx: HelperContext<'_>,
        axislabel_pad: f64,
        position: Option<(Point, f64)>,
    ) -> Result<((Point, f64), f64), String> {
        // Without a position the label keeps matplotlib's initial one:
        // the origin of its coordinates, facing right.
        let (reference, ref_angle) = match position {
            Some((xy, tangent)) => (xy, tangent - 90.0 + self.axislabel_direction.add_angle()),
            None => (self.helper.label_origin(ctx)?, 0.0),
        };
        let anchor = match &self.label.transform {
            None => self
                .resolve(layout, Transform::Display)?
                .transform_point(reference),
            Some(given) => {
                // The program's transform takes the reference point in the
                // coordinates of the helper's label transform.
                let helper = (layout.resolve(&self.helper.label_transform(self.axes)))
                    .and_then(|transform| transform.inverted())
                    .ok_or_else(|| GONE.to_owned())?;
                let given = layout.resolve(given).ok_or_else(|| GONE.to_owned())?;
                given.transform_point(helper.transform_point(reference))
            }
        };
        let radius = axislabel_pad + layout.measuring_points_to_pixels(self.label.pad);
        Ok(((anchor, ref_angle), radius))
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

    /// Draws `text`, one of the texts of `labels`, behind its box if the
    /// labels have one.
    fn draw_part_text(&self, ctx: &mut DrawContext<'_>, labels: &LabelState, text: &PlacedText) {
        let id = self.base.id();
        if let Err(err) = labels.draw_box(ctx, &self.base, &self.rc, text) {
            ctx.report(id, err);
        }
        draw_text(ctx, id, text, &self.rc, AxisTextDraw::AfterMeasuring);
    }

    /// The artist properties of the line: the artist's opacity, clipped
    /// to the axes box if asked, even when the axes clips its plotted
    /// artists to an outline of its own (matplotlib's
    /// `line.set_clip_box(axes.bbox)`).
    fn line_base(&self) -> ArtistBase {
        let mut base = ArtistBase::default();
        base.alpha = self.base.alpha;
        if self.line.clipped {
            base.clip_path = Some(ClipPathSpec::BoundingBox {
                path: Path::unit_rectangle(),
                transform: Transform::Axes(self.axes),
            });
        }
        base
    }
}

/// What places a part of the artist: the transform the program gave it
/// (`given`), else the helper's tick transform followed by the artist's
/// offset (`shared`).
fn part_transform(
    layout: LayoutContext<'_>,
    given: Option<&Transform>,
    shared: &ResolvedTransform,
) -> Result<ResolvedTransform, String> {
    match given {
        Some(transform) => layout.resolve(transform).ok_or_else(|| GONE.to_owned()),
        None => Ok(shared.clone()),
    }
}

impl AxisArtistState {
    /// Lays the parts out as a draw does, remembering the gaps they leave
    /// (matplotlib's `AxisArtist.draw` sets `_axislabel_pad` and
    /// `_external_pad`, which a later measurement places the label by).
    fn lay_out_as_drawn(&self, layout: LayoutContext<'_>) {
        // What cannot be laid out is reported when the artist is drawn.
        let _ = self.drawing(layout, layout.points_to_pixels(1.0), LabelGap::Current);
    }
}

/// Lays the visible axis artists of the figure out as a draw does (see
/// [`AxisArtistState::lay_out_as_drawn`]): the draw matplotlib makes
/// before it measures the figure for a tight print.
pub(crate) fn lay_out_as_drawn(state: &FigureState) {
    let layout = LayoutContext::new(state);
    for id in state.store.ids() {
        if let Some(artist) = state.store.get_as::<AxisArtistState>(id)
            && artist.base.visible
        {
            artist.lay_out_as_drawn(layout);
        }
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
        let layout = ctx.layout();
        let drawing = crate::text::font_notes::placed_before_drawn(|| {
            self.drawing(layout, layout.points_to_pixels(1.0), LabelGap::Current)
        });
        let axes = layout.store().shared_as::<AxesState>(self.axes);
        let drawing = match drawing {
            Ok(drawing) => drawing,
            Err(problem) => return ctx.report(id, problem),
        };
        let Some(axes) = axes else {
            return;
        };
        ctx.renderer().open_group(GROUP, self.base.gid.as_deref());
        for ((level, ticks), labels) in (drawing.levels.iter())
            .zip([&self.major_ticks, &self.minor_ticks])
            .zip([&self.major_ticklabels, &self.minor_ticklabels])
        {
            if let Some(marks) = &level.marks {
                match self.mark_gc(&axes, ticks) {
                    Ok(gc) => ticks.draw(ctx, marks, &level.marks_transform, &gc),
                    Err(problem) => ctx.report(id, problem),
                }
            }
            for text in &level.texts {
                self.draw_part_text(ctx, labels, text);
            }
        }
        if let Some(offset) = &drawing.offset {
            draw_text(ctx, id, offset, &self.rc, AxisTextDraw::AfterMeasuring);
        }
        let (style, edge_default) = self.line.patch_style(&self.rc);
        if let Err(err) = paint_paths(
            ctx,
            &self.line_base(),
            &style,
            &self.rc,
            edge_default,
            &drawing.line,
        ) {
            ctx.report(id, err);
        }
        if let Some(label) = &drawing.label {
            self.draw_part_text(ctx, &self.label, label);
        }
        ctx.renderer().close_group(GROUP);
    }

    /// The box around the tick labels, the label, the offset text and the
    /// line (matplotlib's `AxisArtist.get_tightbbox`); a line clipped to
    /// the axes box counts only inside it (`line.get_tightbbox`).
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let drawing = self
            .drawing(*ctx, ctx.measured_dpi_cor(), LabelGap::Previous)
            .ok()?;
        let clip = if self.line.clipped {
            Some(ctx.axes_extent(self.axes)?)
        } else {
            None
        };
        let line = drawing.line.iter().filter_map(|path| {
            let extents = path.to_display().extents();
            clip.map_or(Some(extents), |clip| extents.intersection(&clip))
        });
        drawing
            .levels
            .iter()
            .flat_map(|level| &level.texts)
            .chain(&drawing.label)
            .chain(&drawing.offset)
            .map(PlacedText::bbox)
            .chain(line)
            .filter(|b| b.width() != 0.0 || b.height() != 0.0)
            .reduce(|a, b| a.union(&b))
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::axisartist::grid_finder::GridFinderOptions;
    use crate::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
    use crate::axisartist::{AuxTransform, AxisArtistAxes, Toggle};
    use crate::figure::Figure;
    use crate::ticker::AxisName;

    #[test]
    fn clipped_lines_count_only_inside_the_axes() {
        let fig = Figure::new();
        let polar = GridHelperCurveLinear::new(AuxTransform::polar(), GridFinderOptions::default());
        let ax = AxisArtistAxes::with_grid_helper(fig.subplot(), polar);
        ax.ax()
            .set_xlim(-1.0, 1.0)
            .and_then(|ax| ax.set_ylim(-1.0, 1.0))
            .expect("finite limits");
        // The ray at angle 0 runs to r = √2, past the right edge.
        let ray = ax.new_floating_axis("ray", AxisName::X, 0.0);
        ray.toggle(Toggle {
            all: Some(false),
            ..Toggle::default()
        });
        let extent = ray.window_extent().expect("measured");
        let axes = ax.ax().window_extent().expect("laid out");
        assert!((extent.x1() - axes.x1()).abs() < 1e-9);
    }

    #[test]
    fn measuring_places_the_label_by_the_previous_tick_label_gap() {
        let fig = Figure::new();
        let ax = AxisArtistAxes::new(fig.subplot());
        ax.ax().set_ylabel("label");
        let left = ax.axis("left").expect("an edge");
        let first = left.window_extent().expect("measured");
        let second = left.window_extent().expect("measured");
        assert!(
            second.x0() < first.x0(),
            "a first measurement leaves no gap"
        );
        assert_eq!(left.window_extent(), Some(second));
    }
}
