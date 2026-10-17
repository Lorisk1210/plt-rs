//! Drawing an axis: gridlines, tick marks, tick labels, the axis label and
//! the offset text (matplotlib's `Axis.draw` / `Tick.draw`). Polar axes
//! draw their circular gridlines and rotated marks through
//! [`crate::polar`]; geographic axes draw theirs here, in the coordinates
//! [`crate::geo`] gives them.

use plt_core::{Affine2D, Path};
use plt_render::{GraphicsContext, SnapMode};

use super::axis_layout::{AxisPainter, PlacedText};
use super::ticks::{GridStyle, TickDirection, TickStyle};
use crate::artist::{ArtistId, DrawContext};
use crate::colors::ColorError;
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol};
use crate::polar;
use crate::text::draw_laid_out;
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
        let mut labels = texts.tick_labels.iter().peekable();
        for (index, (tick, style)) in self.styled_ticks().into_iter().enumerate() {
            match &polar {
                Some(polar) => {
                    polar::draw_tick_lines(ctx, self.axes_id, polar, self.name, tick, style, rc);
                }
                None => self.draw_tick_lines(ctx, tick.loc, style, rc),
            }
            while let Some(label) = labels.next_if(|label| label.tick == index) {
                draw_text(ctx, &label.text, rc);
            }
        }
        for text in [&texts.label, &texts.offset].into_iter().flatten() {
            draw_text(ctx, text, rc);
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
        if let Some(grid) = &style.grid {
            self.draw_gridline(ctx, loc, grid, rc);
        }
        let (mark1, mark2) = tick_markers(self.name, style.direction);
        if style.ticks.first {
            self.draw_tick_mark(ctx, loc, false, mark1, style);
        }
        if style.ticks.second {
            self.draw_tick_mark(ctx, loc, true, mark2, style);
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
        let color = match self.style.color.resolve() {
            Ok(color) => color,
            Err(err) => return ctx.report(owner, err),
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
        let gc = GraphicsContext {
            foreground: color,
            linewidth: self.style.width,
            join: self.marker.joinstyle(),
            cap: self.marker.capstyle(),
            snap,
            ..GraphicsContext::default()
        };
        let marker_transform = self.transform.scale(size, size);
        ctx.renderer().draw_markers(
            &gc,
            self.marker.path(),
            &marker_transform,
            points,
            affine,
            Some(color),
        );
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

/// Draws a laid-out axis text.
pub(crate) fn draw_text(ctx: &mut DrawContext<'_>, text: &PlacedText, rc: &crate::rc::RcParams) {
    let gc = GraphicsContext {
        foreground: text.color,
        antialiased: rc.text_antialiased,
        ..GraphicsContext::default()
    };
    draw_laid_out(ctx, &gc, &text.laid_out, text.anchor, text.rotation);
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
