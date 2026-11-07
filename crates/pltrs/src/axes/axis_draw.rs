//! Drawing an axis: gridlines, tick marks, tick labels, the axis label and
//! the offset text (matplotlib's `Axis.draw` / `Tick.draw`). Polar axes
//! draw their circular gridlines and rotated marks through
//! [`crate::polar`]; geographic axes draw theirs here, in the coordinates
//! [`crate::geo`] gives them.

use std::borrow::Cow;

use pltrs_core::{Affine2D, Bbox, Path, Rgba};
use pltrs_render::{ClipPath, GraphicsContext, SnapMode};

use super::AxesState;
use super::axis_artists::{AxisGroup, TickGroups, TickPart, tick_group_name};
pub(crate) use super::axis_layout::AxisTextDraw;
use super::axis_layout::{AxisPainter, PlacedText, TickLabel};
use super::grid_props::{GridClip, GridLineProps};
use super::tick_set::Tick;
use super::ticks::{GridStyle, TickDirection, TickStyle};
use crate::artist::{AggFilter, ArtistBase, ArtistId, DrawContext, rc_sketch};
use crate::colors::{Color, ColorError};
use crate::lines::{inverse_dashes, line_vertices};
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol, marker_sketch};
use crate::patheffects::with_path_effects;
use crate::polar;
use crate::rc::{ColorOrAuto, RcParams};
use crate::text::{BoxPlacement, TextBoxPatch, TextDrawError, draw_laid_out, has_line_measurer};
use crate::ticker::AxisName;
use crate::transforms::program_calls;

impl AxisPainter<'_> {
    /// Draws the axis tick by tick (gridline, marks, labels), then the
    /// axis label and the offset text, each in its SVG group inside the
    /// axis' (matplotlib's `Axis.draw`); problems are reported on the axes.
    pub(crate) fn draw(&self, ctx: &mut DrawContext<'_>, rc: &RcParams) {
        if !self.axis.visible {
            return;
        }
        self.axis.ticks_placed.set(true);
        // The program's scales answer from the calls matplotlib's draw
        // makes (see `transform_sites`).
        program_calls::quietly(|| {
            (self.axis.gids).grouped(ctx, "matplotlib.axis", AxisGroup::Axis, |ctx| {
                self.draw_contents(ctx, rc);
            });
        });
    }

    /// The body of [`AxisPainter::draw`], inside the axis' group.
    fn draw_contents(&self, ctx: &mut DrawContext<'_>, rc: &RcParams) {
        let ticks = match &self.axis.ticks {
            Ok(ticks) => ticks,
            Err(err) => return ctx.report_kind(self.axes_id, err.draw_kind(), err),
        };
        let texts = self.texts(ctx, rc);
        for problem in &texts.problems {
            ctx.report(self.axes_id, problem);
        }
        let gids = &self.axis.gids;
        let mut labels = texts.tick_labels.iter().peekable();
        for (index, (tick, style)) in self.styled_ticks().into_iter().enumerate() {
            let key = ticks
                .drawn_key(index)
                .expect("the styled ticks are the ticks in view");
            let name = tick_group_name(self.name);
            gids.tick(ctx, name, key, |ctx, groups| {
                let tick_labels = std::iter::from_fn(|| labels.next_if(|l| l.tick == index));
                self.draw_tick(ctx, groups, tick, style, tick_labels, rc);
            });
        }
        if let Some(label) = &texts.label {
            gids.part_grouped(ctx, "text", AxisGroup::Label, |ctx| {
                self.draw_label_box(ctx, label, rc);
                self.draw_text_with_effects(ctx, label, rc);
            });
        }
        if let Some(offset) = &texts.offset {
            gids.part_grouped(ctx, "text", AxisGroup::OffsetText, |ctx| {
                self.draw_text_with_effects(ctx, offset, rc);
            });
        }
    }

    /// Draws one tick: its gridline, its marks and its `labels`, each in
    /// its group of `groups` (matplotlib's `Tick.draw`). Tick marks and
    /// texts are artists made with the rc path effects in matplotlib;
    /// gridlines take those of their style.
    fn draw_tick<'t>(
        &self,
        ctx: &mut DrawContext<'_>,
        groups: TickGroups<'_>,
        tick: &Tick,
        mut style: Cow<'_, TickStyle>,
        labels: impl Iterator<Item = &'t TickLabel>,
        rc: &RcParams,
    ) {
        // A custom projection may hide a side of the tick while drawing.
        let sides = self.axes.drawn_tick_sides(self.name, tick.loc);
        if !(sides.first && sides.second) {
            let marks = &mut style.to_mut().ticks;
            marks.first &= sides.first;
            marks.second &= sides.second;
        }
        match &self.axes.polar_view() {
            Some(polar) => polar::draw_tick_lines(
                ctx,
                self.axes_id,
                polar,
                self.name,
                tick,
                &style,
                rc,
                groups,
            ),
            None => self.draw_tick_lines(ctx, tick.loc, &style, rc, groups),
        }
        let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| {
            for label in labels {
                let shown = if label.first_side {
                    sides.first
                } else {
                    sides.second
                };
                if shown {
                    groups.part(ctx, TickPart::label(!label.first_side), |ctx| {
                        if let Some(bbox) = &style.label_bbox {
                            self.draw_text_box(ctx, bbox, &label.text, style.label_size, rc);
                        }
                        draw_text(
                            ctx,
                            self.axes_id,
                            &label.text,
                            rc,
                            AxisTextDraw::AfterMeasuring,
                        );
                    });
                }
            }
        });
        if let Err(err) = drawn {
            ctx.report(self.axes_id, err);
        }
    }

    /// Draws the axis label or offset text `text` with the rc path
    /// effects.
    fn draw_text_with_effects(&self, ctx: &mut DrawContext<'_>, text: &PlacedText, rc: &RcParams) {
        let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| {
            draw_text(ctx, self.axes_id, text, rc, AxisTextDraw::AsPlaced);
        });
        if let Err(err) = drawn {
            ctx.report(self.axes_id, err);
        }
    }

    /// The box behind the axis label `label`, if it has one (drawn first
    /// by matplotlib's `Text.draw`, with the box's own path effects).
    fn draw_label_box(&self, ctx: &mut DrawContext<'_>, label: &PlacedText, rc: &RcParams) {
        if let Some(bbox) = &self.axis.label.bbox {
            self.draw_text_box(ctx, bbox, label, self.axis.label.font.size, rc);
        }
    }

    /// The box `bbox` behind the axis text `text` of `fontsize` points
    /// (an axis label or a tick label, whose `Text.draw` draws it first,
    /// with the box's own path effects).
    fn draw_text_box(
        &self,
        ctx: &mut DrawContext<'_>,
        bbox: &TextBoxPatch,
        text: &PlacedText,
        fontsize: f64,
        rc: &RcParams,
    ) {
        let (corner, size) = text.laid_out.block.frame;
        let placement = BoxPlacement {
            anchor: text.anchor,
            corner,
            size,
            rotation: text.rotation,
            fontsize_px: ctx.layout().points_to_pixels(fontsize),
        };
        // The texts of an axis are not clipped.
        let mut owner = ArtistBase::default();
        owner.clip_on = false;
        owner.parent = Some(self.axes_id);
        if let Err(err) = bbox.draw(ctx, &owner, rc, &placement) {
            ctx.report(self.axes_id, err);
        }
    }

    /// The gridline and the tick marks of the tick at `loc`, each in its
    /// group of `groups`.
    fn draw_tick_lines(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        style: &TickStyle,
        rc: &RcParams,
        groups: TickGroups<'_>,
    ) {
        if style.grid_on {
            let line = &style.grid.line;
            // As matplotlib's `Line2D.draw` under `allow_rasterization`:
            // rasterized and filtered as the gridline says, in its group.
            let drawn = ctx.draw_artist(line.rasterized, |ctx| {
                with_agg_filter(ctx, line.agg_filter.as_ref(), |ctx| {
                    groups.part_or(ctx, TickPart::Gridline, line.gid.as_deref(), |ctx| {
                        with_path_effects(ctx, &style.grid.path_effects, |ctx| {
                            self.draw_gridline(ctx, loc, &style.grid, rc);
                        })
                    })
                })
            });
            if let Err(err) = drawn {
                ctx.report(self.axes_id, err);
            }
        }
        let (mark1, mark2) = tick_markers(self.name, style.direction);
        let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| {
            for (on, second, mark) in [
                (style.ticks.first, false, mark1),
                (style.ticks.second, true, mark2),
            ] {
                // A mark is a line of its own, which ends the rasterizing
                // a rasterized gridline started.
                ctx.draw_artist(false, |ctx| {
                    if on {
                        groups.part(ctx, TickPart::mark(second), |ctx| {
                            let style = style.mark_style(second);
                            self.draw_tick_mark(ctx, loc, second, mark, &style);
                        });
                    }
                });
            }
        });
        if let Err(err) = drawn {
            ctx.report(self.axes_id, err);
        }
    }

    /// The path the gridline of the tick at `loc` is drawn as, in its
    /// coordinates (data along the axis, axes fractions across it).
    pub(crate) fn gridline_path(&self, loc: f64, grid: &GridStyle) -> Path {
        let (start, end) = (self.tick_point(loc, 0.0), self.tick_point(loc, 1.0));
        let points = line_vertices(&[start.x, end.x], &[start.y, end.y], grid.line.drawstyle)
            .expect("the two ends of a gridline make a line of any draw style");
        Path::polyline(points)
    }

    /// The path of the mark of the tick at `loc` on the first or second
    /// (`high`) side: the one point it is drawn at.
    pub(crate) fn tick_mark_path(&self, loc: f64, high: bool) -> Path {
        Path::polyline(vec![self.tick_point(loc, f64::from(u8::from(high)))])
    }

    fn draw_gridline(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        grid: &GridStyle,
        rc: &crate::rc::RcParams,
    ) {
        let Some(resolved) = ctx.resolve(&self.grid_transform()) else {
            return;
        };
        let mut gc = match grid_gc(grid, rc) {
            Ok(gc) => gc,
            Err(err) => return ctx.report(self.axes_id, err),
        };
        // matplotlib clips gridlines to the axes patch: a rectangle,
        // which Artist.set_clip_path turns into a clip box, or the outline
        // of a geographic map.
        let outline = ctx.layout().clip_outline(self.axes_id);
        (gc.clip_rect, gc.clip_path) = grid_clip(ctx, self.axes_id, &grid.line, outline);
        let line = self.gridline_path(loc, grid);
        ctx.free_for(&resolved);
        let (path, affine) = resolved.split_path(&line);
        if !grid.linestyle.is_visible() {
            return;
        }
        let gapcolor = grid.line.gapcolor.as_ref();
        if let Some(gapcolor) = gapcolor.filter(|_| grid.linestyle.is_dashed()) {
            // The gaps first, in their colour (matplotlib's `Line2D.draw`).
            let mut gaps = gc.clone();
            gaps.foreground = match gapcolor.resolve_alpha(Some(grid.alpha)) {
                Ok(color) => color,
                Err(err) => return ctx.report(self.axes_id, err),
            };
            gaps.dashes = inverse_dashes(&gc.dashes);
            ctx.renderer().draw_path(&gaps, &path, &affine, None);
        }
        ctx.renderer().draw_path(&gc, &path, &affine, None);
    }

    fn draw_tick_mark(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        high: bool,
        symbol: MarkerSymbol,
        style: &TickStyle,
    ) {
        let Some(resolved) = ctx.resolve(&self.side_transform(ctx.layout(), high)) else {
            return;
        };
        let marker = MarkerStyle::new(symbol, FillStyle::Full).expect("tick markers are valid");
        let points = self.tick_mark_path(loc, high);
        ctx.free_for(&resolved);
        let (points, affine) = resolved.split_points(&points);
        TickMark {
            marker: &marker,
            transform: marker.transform(),
            style,
        }
        .draw(ctx, self.axes_id, &points, &affine);
    }
}

/// A tick mark: the marker, its transform in marker units (scaled to the
/// tick size when drawn) and the tick style.
pub(crate) struct TickMark<'a> {
    /// The marker shape.
    pub(crate) marker: &'a MarkerStyle,
    /// The marker transform before scaling to the tick size.
    pub(crate) transform: Affine2D,
    /// Size, width and colour.
    pub(crate) style: &'a TickStyle,
}

impl TickMark<'_> {
    /// Draws the mark at every vertex of `points` (non-affine part
    /// applied, `affine` still to apply); problems are reported on artist
    /// `owner`.
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        owner: ArtistId,
        points: &Path,
        affine: &Affine2D,
    ) {
        // Tick marks are line markers, which matplotlib skips at size 0.
        if self.style.size <= 0.0 {
            return;
        }
        let color = match self.style.color.resolve() {
            Ok(color) => color,
            Err(err) => return ctx.report(owner, err),
        };
        let edge = match self.style.edgecolor.as_ref().map(Color::resolve) {
            None => color,
            Some(Ok(edge)) => edge,
            Some(Err(err)) => return ctx.report(owner, err),
        };
        let size = ctx.points_to_pixels(self.style.size);
        let snap = self
            .marker
            .snap_threshold()
            .map_or(SnapMode::Auto, |threshold| {
                if size >= threshold {
                    SnapMode::Always
                } else {
                    SnapMode::Never
                }
            });
        // Tick marks are line markers, sketched and filled like markers of
        // lines made under the owning axes' rc parameters.
        let (sketch, face) = match ctx.layout().store().get_as::<AxesState>(owner) {
            Some(axes) => (rc_sketch(&axes.rc), tick_face(&axes.rc, color)),
            None => (None, Ok(color)),
        };
        let face = match face {
            Ok(face) => face,
            Err(err) => return ctx.report(owner, err),
        };
        let gc = GraphicsContext {
            foreground: edge,
            linewidth: self.style.width,
            join: self.marker.joinstyle(),
            cap: self.marker.capstyle(),
            snap,
            sketch: marker_sketch(sketch),
            ..GraphicsContext::default()
        };
        let marker_transform = self.transform.scale(size, size);
        ctx.renderer().draw_markers(
            &gc,
            self.marker.path(),
            &marker_transform,
            points,
            affine,
            Some(face),
        );
    }
}

/// The face of tick marks made under `rc`: matplotlib's tick lines are
/// markers that take `lines.markerfacecolor` (`auto` follows the tick
/// `color`) and `markers.fillstyle` (`none` makes the face transparent).
/// The marks are open paths, so only vector output shows the face.
fn tick_face(rc: &RcParams, color: Rgba) -> Result<Rgba, ColorError> {
    if rc.markers_fillstyle == FillStyle::None {
        return Ok(Rgba::TRANSPARENT);
    }
    match &rc.lines_markerfacecolor {
        ColorOrAuto::Auto => Ok(color),
        ColorOrAuto::Color(face) => face.resolve(),
    }
}

/// The graphics context of gridlines in `grid` style (unclipped).
/// The clip box and path of the gridlines `line` of axes `axes` (none
/// when they are not clipped): the box of the axes and their `outline`
/// unless the tick parameters gave others (matplotlib's `grid_clip_box`
/// and `grid_clip_path`).
pub(crate) fn grid_clip(
    ctx: &DrawContext<'_>,
    axes: ArtistId,
    line: &GridLineProps,
    outline: Option<ClipPath>,
) -> (Option<Bbox>, Option<ClipPath>) {
    if !line.clip_on {
        return (None, None);
    }
    let rect = match &line.clip_box {
        GridClip::Axes => ctx.layout().clip_extent(axes),
        GridClip::Off => None,
        GridClip::Given(spec) => ctx.layout().bounding_rect(spec),
    };
    let path = match &line.clip_path {
        GridClip::Axes => outline,
        GridClip::Off => None,
        GridClip::Given(spec) => ctx.clip_path(spec),
    };
    (rect, path)
}

pub(crate) fn grid_gc(
    grid: &GridStyle,
    rc: &crate::rc::RcParams,
) -> Result<GraphicsContext, ColorError> {
    let mut gc = GraphicsContext {
        foreground: grid.color.resolve_alpha(Some(grid.alpha))?,
        linewidth: grid.linewidth,
        antialiased: grid.stroke.antialiased,
        sketch: grid.line.sketch.unwrap_or_else(|| rc_sketch(rc)),
        // As a line's: never snapped when `path.snap` is off.
        snap: match (rc.path_snap, grid.line.snap) {
            (false, _) | (true, Some(false)) => SnapMode::Never,
            (true, None) => SnapMode::Auto,
            (true, Some(true)) => SnapMode::Always,
        },
        url: grid.line.url.clone(),
        ..GraphicsContext::default()
    };
    (gc.cap, gc.join) = grid.stroke.cap_join(grid.linestyle.is_dashed());
    gc.dashes = grid.linestyle.scaled_dashes(grid.linewidth, rc);
    Ok(gc)
}

/// Runs `draw` through the image filter `filter`, if there is one (the
/// `agg_filter` of matplotlib's `allow_rasterization`).
fn with_agg_filter<R>(
    ctx: &mut DrawContext<'_>,
    filter: Option<&AggFilter>,
    draw: impl FnOnce(&mut DrawContext<'_>) -> R,
) -> R {
    let Some(filter) = filter else {
        return draw(ctx);
    };
    ctx.renderer().start_filter();
    let drawn = draw(ctx);
    ctx.renderer().stop_shared_filter(filter.shared());
    drawn
}

/// Draws an axis text as matplotlib's `Text.draw` draws it (see
/// [`PlacedText::as_drawn`]), reporting a failure on `owner`.
pub(crate) fn draw_text(
    ctx: &mut DrawContext<'_>,
    owner: ArtistId,
    text: &PlacedText,
    rc: &crate::rc::RcParams,
    drawn_as: AxisTextDraw,
) {
    let gc = GraphicsContext {
        foreground: text.color,
        antialiased: rc.text_antialiased,
        url: text.url.clone(),
        ..GraphicsContext::default()
    };
    let dpi = ctx.layout().dpi();
    // The program measures the text again with the figure free.
    if has_line_measurer() {
        ctx.free();
    }
    let drawn = text
        .as_drawn(dpi, rc, drawn_as)
        .and_then(|laid_out| draw_laid_out(ctx, &gc, &laid_out, text.anchor, text.rotation));
    if let Err(err) = drawn {
        let err = TextDrawError::from(err);
        ctx.report_kind(owner, err.kind(), err);
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

#[cfg(test)]
mod tests {
    use pltrs_core::Bbox;

    use super::*;
    use crate::figure::Figure;
    use crate::patches::PatchProps;
    use crate::testing::{Call, RecordingRenderer};
    use crate::text::TextBoxProps;

    #[test]
    fn a_label_box_is_drawn_around_the_label() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_ylabel("depth");
        ax.yaxis().set_label_bbox(Some(TextBoxProps {
            patch: PatchProps::default().facecolor("yellow"),
            ..TextBoxProps::default()
        }));
        let label = ax.yaxis().label_extent().expect("a label");
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        let yellow = Some(Rgba::new(1.0, 1.0, 0.0, 1.0));
        let boxes: Vec<Bbox> = (renderer.calls.into_iter())
            .filter_map(|call| match call {
                Call::Path { points, fill, .. } if fill == yellow => {
                    Some(Bbox::from_points(points))
                }
                _ => None,
            })
            .collect();
        assert_eq!(boxes.len(), 1);
        let around = boxes[0];
        assert!(around.x0() < label.x0() && around.x1() > label.x1());
        assert!(around.y0() < label.y0() && around.y1() > label.y1());
    }

    /// matplotlib's `Text.draw` lays a tick label out again where it draws
    /// it, and `_update_label_position` measures it again to place the axis
    /// label, so the renderer's text metrics cache sees its lines twice more
    /// (`test_text.py::test_metrics_cache2` counts on it): pltrs looks them
    /// up again, and draws what it draws without a cache.
    #[test]
    fn drawn_tick_labels_are_looked_up_again_in_the_metrics_cache() {
        use std::rc::Rc;

        use crate::text::{MetricsCache, set_metrics_cache};

        let fig = Figure::with_size(2.0, 2.0, 100.0);
        let ax = fig.subplot();
        ax.set_xticks([0.0, 1.0]);
        ax.set_xticklabels(["a", "b\nc"])
            .expect("one label per tick");
        ax.set_yticks(Vec::<f64>::new());
        let uncached = fig.to_png().expect("drawn");
        let cache = Rc::new(MetricsCache::new());
        let before = set_metrics_cache(Some(Rc::clone(&cache)));
        let cached = fig.to_png();
        set_metrics_cache(before);
        assert!(cached.expect("drawn") == uncached, "the same pixels");
        // Each line is measured where the labels are placed, then found
        // where the axis label is placed and where they are drawn
        // (matplotlib: 3 misses, 6 hits).
        let info = cache.info();
        assert_eq!((info.misses, info.hits, info.size), (3, 6, 3), "{info:?}");
    }

    #[test]
    fn tick_faces_follow_the_marker_rc_parameters() {
        let black = Rgba::new(0.0, 0.0, 0.0, 1.0);
        let mut rc = RcParams::matplotlib();
        assert_eq!(tick_face(&rc, black).unwrap(), black);
        rc.set("lines.markerfacecolor", "white").unwrap();
        assert_eq!(tick_face(&rc, black).unwrap(), Rgba::WHITE);
        rc.set("markers.fillstyle", "none").unwrap();
        assert_eq!(tick_face(&rc, black).unwrap(), Rgba::TRANSPARENT);
    }
}
