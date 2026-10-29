//! Drawing an axis: gridlines, tick marks, tick labels, the axis label and
//! the offset text (matplotlib's `Axis.draw` / `Tick.draw`). Polar axes
//! draw their circular gridlines and rotated marks through
//! [`crate::polar`]; geographic axes draw theirs here, in the coordinates
//! [`crate::geo`] gives them.

use pltrs_core::{Affine2D, Path, Rgba};
use pltrs_render::{GraphicsContext, SnapMode};

use super::AxesState;
use super::axis_layout::{AxisPainter, PlacedText};
use super::ticks::{GridStyle, TickDirection, TickStyle};
use crate::artist::{ArtistBase, ArtistId, DrawContext, rc_sketch};
use crate::colors::{Color, ColorError};
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol, marker_sketch};
use crate::patheffects::with_path_effects;
use crate::polar;
use crate::rc::{ColorOrAuto, RcParams};
use crate::text::{BoxPlacement, draw_laid_out};
use crate::ticker::AxisName;

impl AxisPainter<'_> {
    /// Draws the axis tick by tick (gridline, marks, labels), then the
    /// axis label and the offset text; problems are reported on the axes.
    pub(crate) fn draw(&self, ctx: &mut DrawContext<'_>, rc: &crate::rc::RcParams) {
        if !self.axis.visible {
            return;
        }
        if let Err(message) = &self.axis.ticks {
            return ctx.report(self.axes_id, message);
        }
        let texts = self.texts(*ctx.layout(), rc);
        for problem in &texts.problems {
            ctx.report(self.axes_id, problem);
        }
        let polar = self.axes.polar_view();
        // Tick marks and texts are artists made with the rc path effects
        // in matplotlib; gridlines take those of their style.
        let mut labels = texts.tick_labels.iter().peekable();
        for (index, (tick, mut style)) in self.styled_ticks().into_iter().enumerate() {
            // A custom projection may hide a side of the tick while drawing.
            let sides = self.axes.drawn_tick_sides(self.name, tick.loc);
            if !(sides.first && sides.second) {
                let marks = &mut style.to_mut().ticks;
                marks.first &= sides.first;
                marks.second &= sides.second;
            }
            match &polar {
                Some(polar) => {
                    polar::draw_tick_lines(ctx, self.axes_id, polar, self.name, tick, &style, rc);
                }
                None => self.draw_tick_lines(ctx, tick.loc, &style, rc),
            }
            let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| {
                while let Some(label) = labels.next_if(|label| label.tick == index) {
                    let shown = if label.first_side {
                        sides.first
                    } else {
                        sides.second
                    };
                    if shown {
                        draw_text(ctx, self.axes_id, &label.text, rc);
                    }
                }
            });
            if let Err(err) = drawn {
                ctx.report(self.axes_id, err);
            }
        }
        if let Some(label) = &texts.label {
            self.draw_label_box(ctx, label, rc);
        }
        let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| {
            for text in [&texts.label, &texts.offset].into_iter().flatten() {
                draw_text(ctx, self.axes_id, text, rc);
            }
        });
        if let Err(err) = drawn {
            ctx.report(self.axes_id, err);
        }
    }

    /// The box behind the axis label `label`, if it has one (drawn first
    /// by matplotlib's `Text.draw`, with the box's own path effects).
    fn draw_label_box(&self, ctx: &mut DrawContext<'_>, label: &PlacedText, rc: &RcParams) {
        let Some(bbox) = &self.axis.label.bbox else {
            return;
        };
        let (corner, size) = label.laid_out.block.frame;
        let placement = BoxPlacement {
            anchor: label.anchor,
            corner,
            size,
            rotation: label.rotation,
            fontsize_px: ctx.layout().points_to_pixels(self.axis.label.font.size),
        };
        // Axis labels are not clipped.
        let mut owner = ArtistBase::default();
        owner.clip_on = false;
        owner.parent = Some(self.axes_id);
        if let Err(err) = bbox.draw(ctx, &owner, rc, &placement) {
            ctx.report(self.axes_id, err);
        }
    }

    /// The gridline and the tick marks of the tick at `loc`.
    fn draw_tick_lines(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        style: &TickStyle,
        rc: &crate::rc::RcParams,
    ) {
        if style.grid_on {
            let drawn = with_path_effects(ctx, &style.grid.path_effects, |ctx| {
                self.draw_gridline(ctx, loc, &style.grid, rc);
            });
            if let Err(err) = drawn {
                ctx.report(self.axes_id, err);
            }
        }
        let (mark1, mark2) = tick_markers(self.name, style.direction);
        let drawn = with_path_effects(ctx, &rc.path_effects, |ctx| {
            if style.ticks.first {
                self.draw_tick_mark(ctx, loc, false, mark1, style);
            }
            if style.ticks.second {
                self.draw_tick_mark(ctx, loc, true, mark2, style);
            }
        });
        if let Err(err) = drawn {
            ctx.report(self.axes_id, err);
        }
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
        // matplotlib clips gridlines to the axes patch: a rectangle, which
        // Artist.set_clip_path turns into a clip box, or the outline of a
        // geographic map.
        gc.clip_rect = ctx.layout().clip_extent(self.axes_id);
        gc.clip_path = ctx.layout().clip_outline(self.axes_id);
        let line = Path::polyline(vec![self.tick_point(loc, 0.0), self.tick_point(loc, 1.0)]);
        let (path, affine) = resolved.split_path(&line);
        if grid.linestyle.is_visible() {
            ctx.renderer().draw_path(&gc, &path, &affine, None);
        }
    }

    fn draw_tick_mark(
        &self,
        ctx: &mut DrawContext<'_>,
        loc: f64,
        high: bool,
        symbol: MarkerSymbol,
        style: &TickStyle,
    ) {
        let Some(resolved) = ctx.resolve(&self.side_transform(*ctx.layout(), high)) else {
            return;
        };
        let marker = MarkerStyle::new(symbol, FillStyle::Full).expect("tick markers are valid");
        let points = Path::polyline(vec![self.tick_point(loc, f64::from(u8::from(high)))]);
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
pub(crate) fn grid_gc(
    grid: &GridStyle,
    rc: &crate::rc::RcParams,
) -> Result<GraphicsContext, ColorError> {
    let mut gc = GraphicsContext {
        foreground: grid.color.resolve_alpha(Some(grid.alpha))?,
        linewidth: grid.linewidth,
        antialiased: rc.lines_antialiased,
        sketch: rc_sketch(rc),
        // Gridlines are lines without a snap setting of their own.
        snap: if rc.path_snap {
            SnapMode::Auto
        } else {
            SnapMode::Never
        },
        ..GraphicsContext::default()
    };
    (gc.cap, gc.join) = if grid.linestyle.is_dashed() {
        (rc.lines_dash_capstyle, rc.lines_dash_joinstyle)
    } else {
        (rc.lines_solid_capstyle, rc.lines_solid_joinstyle)
    };
    gc.dashes = grid.linestyle.scaled_dashes(grid.linewidth, rc);
    Ok(gc)
}

/// Draws a laid-out axis text, reporting a TeX failure on `owner`.
pub(crate) fn draw_text(
    ctx: &mut DrawContext<'_>,
    owner: ArtistId,
    text: &PlacedText,
    rc: &crate::rc::RcParams,
) {
    let gc = GraphicsContext {
        foreground: text.color,
        antialiased: rc.text_antialiased,
        url: text.url.clone(),
        ..GraphicsContext::default()
    };
    if let Err(err) = draw_laid_out(ctx, &gc, &text.laid_out, text.anchor, text.rotation) {
        ctx.report(owner, err);
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
