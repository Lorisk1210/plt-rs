//! Drawing the decorations of a 3-D axes: panes, gridlines, axis lines,
//! tick marks, tick labels, offset texts and axis labels (matplotlib's
//! `axis3d.Axis.draw_pane`, `draw_grid` and `draw`).

use std::rc::Rc;

use plt_core::{Bbox, Path, Point};
use plt_render::GraphicsContext;
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis3d::{Axis3DState, DrawnTick};
use super::camera::{Projection, norm_text_angle};
use super::coords::Axis3;
use super::geometry::{BoxGeometry, juggled, static_tick_direction, tick_direction};
use super::math::{Vec3, add, scaled};
use super::state::Axes3DState;
use crate::artist::{Artist, ArtistBase, DrawContext, LayoutContext};
use crate::collections::{CollectionArtist, CollectionKind, EdgeColors};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::patches::{PatchStyle, draw_patch_paths};
use crate::text::{draw_laid_out, lay_out};
use crate::transforms::{ResolvedTransform, Transform};

/// Fraction of the tick offset the tick marks extend outwards and inwards.
const TICK_OUTWARD: f64 = 0.1;
const TICK_INWARD: f64 = 0.2;

/// Rough distance between tick marks and their labels, in points.
const TICK_LABEL_OFFSET: f64 = 8.0;

/// Rough distance between an axis line and its label, in points.
const AXIS_LABEL_OFFSET: f64 = 21.0;

/// Draws the panes, then the gridlines, then the axes (matplotlib's
/// order in `Axes3D.draw`).
pub(crate) fn draw_axes(axes: &Axes3DState, ctx: &mut DrawContext<'_>, extent: &Bbox) {
    let Some(painter) = Painter::new(axes, *ctx.layout(), extent) else {
        return;
    };
    for dir in Axis3::ALL {
        painter.draw_pane(ctx, dir);
    }
    if axes.grid_on {
        for dir in Axis3::ALL {
            painter.draw_grid(ctx, dir);
        }
    }
    for dir in Axis3::ALL {
        painter.draw_axis(ctx, dir);
    }
}

/// The display extents of the axes of a 3-D axes: their lines, offset
/// texts, tick labels and labels, empty tick labels counting as boxes of
/// no size at their anchors (matplotlib's `Axis3D.get_tightbbox`).
pub(crate) fn axis_extents(
    axes: &Axes3DState,
    layout: LayoutContext<'_>,
    extent: &Bbox,
) -> Vec<Bbox> {
    let Some(painter) = Painter::new(axes, layout, extent) else {
        return Vec::new();
    };
    let mut extents = Extents {
        layout,
        boxes: Vec::new(),
    };
    for dir in Axis3::ALL {
        painter.draw_axis(&mut extents, dir);
    }
    extents.boxes
}

/// What the parts of an axis go to: a renderer drawing them, or
/// [`Extents`] measuring them.
trait Sink {
    /// The canvas resolution.
    fn dpi(&self) -> f64;
    /// The axis line, in projected coordinates.
    fn axis_line(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color);
    /// A tick mark, in projected coordinates.
    fn tick_mark(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color);
    /// A tick label, label or offset text.
    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>);
}

impl Sink for DrawContext<'_> {
    fn dpi(&self) -> f64 {
        self.layout().dpi()
    }

    fn axis_line(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color) {
        painter.stroke(self, path, width, color);
    }

    fn tick_mark(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color) {
        painter.stroke(self, path, width, color);
    }

    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>) {
        painter.draw_text(self, text);
    }
}

/// The extents of the lines and texts of axes, without their tick marks.
struct Extents<'a> {
    layout: LayoutContext<'a>,
    boxes: Vec<Bbox>,
}

impl Sink for Extents<'_> {
    fn dpi(&self) -> f64 {
        self.layout.dpi()
    }

    /// The extent of the line's vertices (matplotlib's
    /// `Line2D.get_window_extent`).
    fn axis_line(&mut self, painter: &Painter<'_>, path: &Path, _width: f64, _color: &Color) {
        self.boxes
            .push(painter.plane.transform_path(path).extents());
    }

    fn tick_mark(&mut self, _painter: &Painter<'_>, _path: &Path, _width: f64, _color: &Color) {}

    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>) {
        let anchor = painter.plane.transform_point(text.position);
        if !anchor.is_finite() {
            return;
        }
        if text.text.is_empty() {
            self.boxes
                .push(Bbox::from_extents(anchor.x, anchor.y, anchor.x, anchor.y));
        } else if let Ok(laid_out) = lay_out(
            text.text,
            text.font,
            &text.block,
            self.dpi(),
            painter.axes.rc(),
        ) {
            self.boxes
                .push(laid_out.block.bbox.translated(anchor.x, anchor.y));
        }
    }
}

/// Everything shared by the decorations of one draw, which are placed in
/// transformed coordinates.
struct Painter<'a> {
    axes: &'a Axes3DState,
    geometry: BoxGeometry,
    projection: Projection,
    /// Projected coordinates to display pixels.
    plane: ResolvedTransform,
    extent: Bbox,
}

/// A text placed in projected coordinates.
struct PlacedText<'t> {
    text: &'t str,
    font: &'t FontProperties,
    color: &'t Color,
    block: BlockStyle,
    position: Point,
}

impl<'a> Painter<'a> {
    /// The painter of `axes` spanning `extent`, unless its projection
    /// plane is gone.
    fn new(axes: &'a Axes3DState, layout: LayoutContext<'_>, extent: &Bbox) -> Option<Self> {
        let plane = layout.resolve(&Transform::Data(axes.plane))?;
        let projection = axes.projection();
        Some(Self {
            axes,
            geometry: BoxGeometry::new(axes.scaled_limits(), &projection),
            projection,
            plane,
            extent: *extent,
        })
    }

    fn report(&self, ctx: &mut DrawContext<'_>, problem: impl std::fmt::Display) {
        ctx.report(self.axes.base_id(), problem);
    }

    /// The projected `(x, y)` of a point in transformed coordinates.
    fn project(&self, p: Vec3) -> Point {
        let q = self.projection.project_transformed(p);
        Point::new(q[0], q[1])
    }

    /// Fills the far plane of `dir` with its pane colour (matplotlib's
    /// open `Polygon` pane, edged in the same colour).
    fn draw_pane(&self, ctx: &mut DrawContext<'_>, dir: Axis3) {
        let color = &self.axes.axis(dir).pane_color;
        let alpha = match color.resolve() {
            Ok(rgba) => rgba.a,
            Err(err) => return self.report(ctx, err),
        };
        let outline: Vec<Point> = self
            .geometry
            .pane(dir)
            .iter()
            .map(|q| Point::new(q[0], q[1]))
            .collect();
        let outline = Path::polyline(outline);
        let (path, affine) = self.plane.split_path(&outline);
        let style = PatchStyle {
            facecolor: Some(color.clone()),
            edgecolor: Some(color.clone()),
            ..PatchStyle::from_rc(self.axes.rc())
        };
        let mut base = ArtistBase::default();
        base.alpha = Some(alpha);
        let paths = [(path.into_owned(), affine)];
        if let Err(err) = draw_patch_paths(ctx, &base, &style, self.axes.rc(), false, &paths) {
            self.report(ctx, err);
        }
    }

    /// Draws the gridlines of `dir`: from each major and minor tick across
    /// the two panes meeting along the far edge (matplotlib's
    /// `draw_grid`).
    fn draw_grid(&self, ctx: &mut DrawContext<'_>, dir: Axis3) {
        let axis = self.axes.axis(dir);
        if axis.drawn_ticks().next().is_none() {
            return;
        }
        let i = dir.index();
        let (a, b) = ((i + 1) % 3, (i + 2) % 3);
        let (far, near) = (self.geometry.far_corner(), self.geometry.near_corner());
        let paths = axis
            .drawn_ticks()
            .map(|drawn| {
                let mut corner = far;
                corner[i] = axis.scale.masked_forward(drawn.tick.loc);
                let (mut start, mut end) = (corner, corner);
                start[a] = near[a];
                end[b] = near[b];
                Path::polyline(vec![
                    self.project(start),
                    self.project(corner),
                    self.project(end),
                ])
            })
            .collect();
        let style = &axis.style;
        let mut grid =
            CollectionArtist::new(CollectionKind::Line, paths, Rc::clone(self.axes.rc()));
        grid.edgecolors = EdgeColors::Colors(vec![style.grid_color.clone()]);
        grid.linewidths = Some(vec![style.grid_width]);
        grid.linestyles = vec![style.grid_style.clone()];
        grid.base_mut().transform = Some(Transform::Data(self.axes.plane));
        if let Err(err) = grid.render(ctx) {
            self.report(ctx, err);
        }
    }

    /// Draws the line, tick marks, tick labels, offset text and label of
    /// `dir` (matplotlib's `Axis.draw`).
    fn draw_axis(&self, sink: &mut impl Sink, dir: Axis3) {
        let axis = self.axes.axis(dir);
        let (width, height) = (self.extent.width(), self.extent.height());
        let dpi = sink.dpi();
        // A rough estimate; points are ambiguous since 3-D plots rotate.
        let to_inches = 1.0 / dpi;
        let points_estimate = 72.0 * (width * to_inches) + 72.0 * (height * to_inches);
        let deltas_per_point = 48.0 / points_estimate;
        let label_deltas = scaled(
            self.geometry.deltas,
            (axis.label.pad + AXIS_LABEL_OFFSET) * deltas_per_point,
        );
        let edges = self.geometry.axis_line(dir, self.axes.camera.vertical);
        let ends = edges.map(|p| self.projection.project_transformed(p));
        // The text angle follows the axis in axes coordinates.
        let (dx, dy) = (
            (ends[1][0] - ends[0][0]) * width,
            (ends[1][1] - ends[0][1]) * height,
        );
        let line = Path::polyline(ends.iter().map(|q| Point::new(q[0], q[1])).collect());
        sink.axis_line(self, &line, axis.style.line_width, &axis.style.line_color);
        self.draw_ticks(sink, axis, edges[0], deltas_per_point);
        self.draw_offset_text(sink, axis, edges, ends, label_deltas, (dx, dy));
        if axis.label.text.is_empty() {
            return;
        }
        let middle = scaled(add(edges[0], edges[1]), 0.5);
        let at = self.geometry.move_from_center(middle, label_deltas, dir);
        let rotation = if axis.label.rotated() {
            norm_text_angle(dy.atan2(dx).to_degrees())
        } else {
            0.0
        };
        sink.text(
            self,
            &PlacedText {
                text: &axis.label.text,
                font: &axis.label.font,
                color: &axis.label.color,
                block: anchored(rotation, HAlign::Center),
                position: self.project(at),
            },
        );
    }

    /// Draws each tick mark and its label (matplotlib's
    /// `_get_updated_ticks` followed by `Tick.draw`).
    fn draw_ticks(
        &self,
        sink: &mut impl Sink,
        axis: &Axis3DState,
        origin: Vec3,
        deltas_per_point: f64,
    ) {
        let dir = axis.dir;
        let tickdir = tick_direction(dir, &self.axes.camera);
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
        for DrawnTick { tick, style, width } in axis.drawn_ticks() {
            let mut p = origin;
            p[dir.index()] = axis.scale.masked_forward(tick.loc);
            p[tickdir] = outward;
            let out_end = self.project(p);
            p[tickdir] = inward;
            let in_end = self.project(p);
            p[tickdir] = base;
            let label_deltas = scaled(points, style.pad + TICK_LABEL_OFFSET);
            let label_at = self.geometry.move_from_center(p, label_deltas, dir);
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
    fn draw_offset_text(
        &self,
        sink: &mut impl Sink,
        axis: &Axis3DState,
        edges: [Vec3; 2],
        ends: [Vec3; 2],
        label_deltas: Vec3,
        (dx, dy): (f64, f64),
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
            .move_from_center(edges[outer], label_deltas, dir);
        let center = self.projection.project_transformed(self.geometry.centers);
        let edge = ends[outer];
        let even_highs = self.geometry.highs.iter().filter(|&&h| h).count() % 2 == 0;
        let halign = if center[tickdir] > edge[tickdir] {
            if center[index] <= edge[index] && even_highs {
                if self.geometry.highs == [false, true, true] && index != 0 {
                    HAlign::Left
                } else {
                    HAlign::Right
                }
            } else {
                HAlign::Left
            }
        } else if center[index] > edge[index] && even_highs {
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
                block: anchored(norm_text_angle(dy.atan2(dx).to_degrees()), halign),
                position: self.project(at),
            },
        );
    }

    /// Strokes a path in projected coordinates like a solid `Line2D`.
    fn stroke(&self, ctx: &mut DrawContext<'_>, path: &Path, width: f64, color: &Color) {
        let rc = self.axes.rc();
        let foreground = match color.resolve() {
            Ok(color) => color,
            Err(err) => return self.report(ctx, err),
        };
        let gc = GraphicsContext {
            foreground,
            linewidth: width,
            cap: rc.lines_solid_capstyle,
            join: rc.lines_solid_joinstyle,
            antialiased: rc.lines_antialiased,
            ..GraphicsContext::default()
        };
        let (path, affine) = self.plane.split_path(path);
        ctx.renderer().draw_path(&gc, &path, &affine, None);
    }

    /// Draws a text anchored at a projected position.
    fn draw_text(&self, ctx: &mut DrawContext<'_>, text: &PlacedText<'_>) {
        if text.text.is_empty() {
            return;
        }
        let rc = self.axes.rc();
        let anchor = self.plane.transform_point(text.position);
        if !anchor.is_finite() {
            return;
        }
        let laid_out = match lay_out(text.text, text.font, &text.block, ctx.layout().dpi(), rc) {
            Ok(laid_out) => laid_out,
            Err(err) => return self.report(ctx, err),
        };
        let foreground = match text.color.resolve() {
            Ok(color) => color,
            Err(err) => return self.report(ctx, err),
        };
        let gc = GraphicsContext {
            foreground,
            antialiased: rc.text_antialiased,
            ..GraphicsContext::default()
        };
        draw_laid_out(ctx, &gc, &laid_out, anchor, text.block.rotation);
    }
}

/// A block centred vertically, rotated about its alignment point
/// (matplotlib's 3-D axis label and offset text settings).
fn anchored(rotation: f64, halign: HAlign) -> BlockStyle {
    BlockStyle {
        rotation: rotation.rem_euclid(360.0),
        halign,
        valign: VAlign::Center,
        multialign: halign,
        rotation_mode: RotationMode::Anchor,
        ..BlockStyle::default()
    }
}
