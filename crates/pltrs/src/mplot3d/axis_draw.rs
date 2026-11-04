//! Drawing the decorations of a 3-D axes: panes, gridlines, axis lines,
//! tick marks, tick labels, offset texts and axis labels (matplotlib's
//! `axis3d.Axis.draw_pane`, `draw_grid` and `draw`).

use std::rc::Rc;

use pltrs_core::{Bbox, Path, Point};
use pltrs_render::GraphicsContext;
use pltrs_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::axis3d::Axis3DState;
use super::camera::{Projection, norm_text_angle};
use super::coords::Axis3;
use super::geometry::{AxisPosition, BoxGeometry};
use super::math::{Vec3, add, scaled};
use super::state::Axes3DState;
use crate::artist::{Artist, ArtistBase, DrawContext, LayoutContext};
use crate::axes::axis_draw::grid_gc;
use crate::axes::{AxisGroup, GridStyle};
use crate::collections::{CollectionArtist, CollectionKind, EdgeColors};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::patches::{PatchStyle, draw_patch_paths};
use crate::patheffects::with_path_effects;
use crate::text::{draw_laid_out, lay_out};
use crate::transforms::{ResolvedTransform, Transform};

mod lines;
mod ticks;

pub use lines::{GridLine3D, TickLine3D};
pub(crate) use lines::{grid_lines, tick_lines};

/// Fraction of the tick offset the tick marks extend outwards and inwards.
const TICK_OUTWARD: f64 = 0.1;
const TICK_INWARD: f64 = 0.2;

/// Rough distance between tick marks and their labels, in points.
const TICK_LABEL_OFFSET: f64 = 8.0;

/// Rough distance between an axis line and its label, in points.
const AXIS_LABEL_OFFSET: f64 = 21.0;

/// Draws the panes, then the gridlines, then the axes (matplotlib's
/// order in `Axes3D.draw`), each in an SVG group named after its axis'
/// gid (`pane3d`, `grid3d`, `axis3d`).
pub(crate) fn draw_axes(axes: &Axes3DState, ctx: &mut DrawContext<'_>, extent: &Bbox) {
    // matplotlib ticks each axis as it draws it (`_update_ticks`), which
    // raises what its locator or formatter raises, after its warnings: the
    // first axis failing stops matplotlib's draw.
    let mut warned = false;
    for dir in Axis3::ALL {
        let axis = axes.axis(dir);
        if let Err(err) = &axis.ticks {
            if !std::mem::replace(&mut warned, true) {
                axis.warn_failed_ticks();
            }
            ctx.report_kind(axes.base_id(), err.draw_kind(), err);
        }
    }
    let Some(painter) = Painter::new(axes, *ctx.layout(), extent) else {
        return;
    };
    let gid = |dir: Axis3| axes.axis(dir).gids.get(AxisGroup::Axis);
    for dir in Axis3::ALL {
        grouped(ctx, "pane3d", gid(dir), |ctx| painter.draw_pane(ctx, dir));
    }
    if axes.grid_on {
        for dir in Axis3::ALL {
            grouped(ctx, "grid3d", gid(dir), |ctx| painter.draw_grid(ctx, dir));
        }
    }
    for dir in Axis3::ALL {
        grouped(ctx, "axis3d", gid(dir), |ctx| painter.draw_axis(ctx, dir));
    }
}

/// Draws `draw` into `sink` inside the SVG group `name` with id `gid`.
fn grouped<S: Sink>(sink: &mut S, name: &str, gid: Option<&str>, draw: impl FnOnce(&mut S)) {
    sink.open_group(name, gid);
    draw(sink);
    sink.close_group(name);
}

/// The display extents of the axes `dirs` of a 3-D axes: their lines,
/// offset texts, tick labels and, unless `for_layout_only`, labels, empty
/// tick labels counting as boxes of no size at their anchors (matplotlib's
/// `Axis3D.get_tightbbox`).
pub(crate) fn axis_extents(
    axes: &Axes3DState,
    layout: LayoutContext<'_>,
    extent: &Bbox,
    for_layout_only: bool,
    dirs: &[Axis3],
) -> Vec<Bbox> {
    let Some(painter) = Painter::new(axes, layout, extent) else {
        return Vec::new();
    };
    let mut extents = Extents {
        layout,
        labels: !for_layout_only,
        boxes: Vec::new(),
    };
    for &dir in dirs {
        painter.draw_axis(&mut extents, dir);
    }
    extents.boxes
}

/// The texts of the axis `dir` of a 3-D axes as the axes draw them: each
/// tick label, the offset text and the label, once per edge carrying
/// them, empty ones included.
pub(crate) fn axis_texts(
    axes: &Axes3DState,
    layout: LayoutContext<'_>,
    extent: &Bbox,
    dir: Axis3,
) -> Vec<AxisText3D> {
    let Some(painter) = Painter::new(axes, layout, extent) else {
        return Vec::new();
    };
    let mut texts = Texts {
        layout,
        found: Vec::new(),
    };
    painter.draw_axis(&mut texts, dir);
    texts.found
}

/// Which text of a 3-D axis an [`AxisText3D`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisTextPart {
    /// The label of the tick at `index` among the major (or `minor`)
    /// ticks the locator places, inside the view or not.
    TickLabel {
        /// Whether the tick is a minor one.
        minor: bool,
        /// Its position among the ticks of its level.
        index: usize,
    },
    /// The offset (or scale) text.
    Offset,
    /// The axis label.
    Label,
}

/// A text of a 3-D axis where the axes draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisText3D {
    /// Which text it is.
    pub part: AxisTextPart,
    /// Its anchor in projected coordinates (the data coordinates of the
    /// projection plane, where matplotlib moves the text).
    pub position: Point,
    /// Its extent in display pixels, a box of no size at its anchor when
    /// it is empty.
    pub extent: Bbox,
}

/// What the parts of an axis go to: a renderer drawing them, or
/// [`Extents`] measuring them.
trait Sink {
    /// The canvas resolution.
    fn dpi(&self) -> f64;
    /// The axis line, in projected coordinates.
    fn axis_line(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color);
    /// The gridline of a tick of `dir` in `grid` style; nothing by
    /// default, since it has no extent.
    fn tick_gridline(&mut self, _painter: &Painter<'_>, _dir: Axis3, _grid: &GridStyle) {}
    /// A tick mark, in projected coordinates.
    fn tick_mark(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color);
    /// A tick label or offset text.
    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>);
    /// Whether the axis labels go to the sink too.
    fn takes_labels(&self) -> bool {
        true
    }
    /// Opens the SVG group `name` with id `gid`; nothing by default.
    fn open_group(&mut self, _name: &str, _gid: Option<&str>) {}
    /// Closes the SVG group `name`; nothing by default.
    fn close_group(&mut self, _name: &str) {}
}

impl Sink for DrawContext<'_> {
    fn dpi(&self) -> f64 {
        self.layout().dpi()
    }

    fn axis_line(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color) {
        painter.stroke(self, path, width, color);
    }

    fn tick_gridline(&mut self, painter: &Painter<'_>, dir: Axis3, grid: &GridStyle) {
        painter.draw_tick_gridline(self, dir, grid);
    }

    fn tick_mark(&mut self, painter: &Painter<'_>, path: &Path, width: f64, color: &Color) {
        painter.stroke(self, path, width, color);
    }

    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>) {
        painter.draw_text(self, text);
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        self.renderer().open_group(name, gid);
    }

    fn close_group(&mut self, name: &str) {
        self.renderer().close_group(name);
    }
}

/// The extents of the lines and texts of axes, without their tick marks.
struct Extents<'a> {
    layout: LayoutContext<'a>,
    /// Whether the axis labels are measured.
    labels: bool,
    boxes: Vec<Bbox>,
}

impl Sink for Extents<'_> {
    fn dpi(&self) -> f64 {
        self.layout.dpi()
    }

    fn takes_labels(&self) -> bool {
        self.labels
    }

    /// The extent of the line's vertices (matplotlib's
    /// `Line2D.get_window_extent`).
    fn axis_line(&mut self, painter: &Painter<'_>, path: &Path, _width: f64, _color: &Color) {
        self.boxes
            .push(painter.plane.transform_path(path).extents());
    }

    fn tick_mark(&mut self, _painter: &Painter<'_>, _path: &Path, _width: f64, _color: &Color) {}

    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>) {
        self.boxes.extend(painter.text_extent(text, self.dpi()));
    }
}

/// The texts of an axis, with where they are drawn.
struct Texts<'a> {
    layout: LayoutContext<'a>,
    found: Vec<AxisText3D>,
}

impl Sink for Texts<'_> {
    fn dpi(&self) -> f64 {
        self.layout.dpi()
    }

    fn axis_line(&mut self, _painter: &Painter<'_>, _path: &Path, _width: f64, _color: &Color) {}

    fn tick_mark(&mut self, _painter: &Painter<'_>, _path: &Path, _width: f64, _color: &Color) {}

    fn text(&mut self, painter: &Painter<'_>, text: &PlacedText<'_>) {
        if let Some(extent) = painter.text_extent(text, self.dpi()) {
            self.found.push(AxisText3D {
                part: text.part,
                position: text.position,
                extent,
            });
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

/// A box edge along an axis.
struct AxisEdge {
    /// Its end points in transformed coordinates.
    points: [Vec3; 2],
    /// The same, projected.
    ends: [Vec3; 2],
    /// From the first projected end to the second, in axes pixels.
    delta: (f64, f64),
}

impl AxisEdge {
    /// The angle of the edge on screen in degrees, turned to read left to
    /// right (what texts along it are rotated by).
    fn angle(&self) -> f64 {
        let (dx, dy) = self.delta;
        norm_text_angle(dy.atan2(dx).to_degrees())
    }
}

/// A text placed in projected coordinates.
struct PlacedText<'t> {
    part: AxisTextPart,
    /// The id of its SVG group.
    gid: Option<&'t str>,
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

    /// The display extent of `text`, a box of no size at its anchor when
    /// it is empty; `None` when it lands at infinity or cannot be laid
    /// out. A text projected nowhere (limits outside the domain of a
    /// scale) has a box of NaN, as matplotlib's text translated by its
    /// position, which spreads through the unions of tight boxes.
    fn text_extent(&self, text: &PlacedText<'_>, dpi: f64) -> Option<Bbox> {
        let anchor = self.plane.transform_point(text.position);
        if anchor.x.is_nan() || anchor.y.is_nan() {
            return Some(Bbox::from_extents(f64::NAN, f64::NAN, f64::NAN, f64::NAN));
        }
        if !anchor.is_finite() {
            return None;
        }
        if text.text.is_empty() {
            return Some(Bbox::from_extents(anchor.x, anchor.y, anchor.x, anchor.y));
        }
        let laid_out = lay_out(text.text, text.font, &text.block, dpi, self.axes.rc()).ok()?;
        Some(laid_out.block.bbox.translated(anchor.x, anchor.y))
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
        let outline = Path::polyline(open_polygon(
            self.geometry
                .pane(dir)
                .iter()
                .map(|q| Point::new(q[0], q[1]))
                .collect(),
        ));
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
        let locs: Vec<f64> = axis.drawn_ticks().map(|drawn| drawn.tick.loc).collect();
        let paths = (self.gridlines(axis, &locs).into_iter())
            .map(|line| Path::polyline(line.to_vec()))
            .collect();
        let style = &axis.style;
        let mut grid =
            CollectionArtist::new(CollectionKind::Line, paths, Rc::clone(self.axes.rc()));
        grid.class_name = Some("Line3DCollection");
        grid.edgecolors = EdgeColors::Colors(vec![style.grid_color.clone()]);
        grid.linewidths = Some(vec![style.grid_width]);
        grid.linestyles = vec![style.grid_style.clone()];
        grid.base_mut().transform = Some(Transform::Data(self.axes.plane));
        if let Err(err) = grid.render(ctx) {
            self.report(ctx, err);
        }
    }

    /// Draws the line, tick marks, tick labels and offset text of `dir`
    /// along each edge its tick position names, then its label along each
    /// edge its label position names, each in its SVG group (matplotlib's
    /// `Axis.draw`).
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
        for &position in axis.tick_position.edges() {
            let edge = self.edge(dir, position);
            if axis.line_visible {
                let line =
                    Path::polyline(edge.ends.iter().map(|q| Point::new(q[0], q[1])).collect());
                grouped(sink, "line2d", None, |sink| {
                    sink.axis_line(self, &line, axis.style.line_width, &axis.style.line_color);
                });
            }
            self.draw_ticks(sink, axis, edge.points[0], deltas_per_point, position);
            self.draw_offset_text(sink, axis, &edge, label_deltas);
        }
        if axis.label.text.is_empty() || !sink.takes_labels() {
            return;
        }
        for &position in axis.label_position.edges() {
            let edge = self.edge(dir, position);
            let middle = scaled(add(edge.points[0], edge.points[1]), 0.5);
            let at = self.geometry.move_from_center(middle, label_deltas, dir);
            let rotation = if axis.label.rotated() {
                edge.angle()
            } else {
                0.0
            };
            self.text_in_group(
                sink,
                &PlacedText {
                    part: AxisTextPart::Label,
                    gid: axis.gids.get(AxisGroup::Label),
                    text: &axis.label.text,
                    font: &axis.label.font,
                    color: &axis.label.color,
                    block: anchored(rotation, HAlign::Center),
                    position: self.project(at),
                },
            );
        }
    }

    /// The projected gridlines of `axis` at `locs`: from the near edge of
    /// one pane through the far edge to the near edge of the other
    /// (matplotlib's `draw_grid`, which projects them all at once).
    fn gridlines(&self, axis: &Axis3DState, locs: &[f64]) -> Vec<[Point; 3]> {
        let i = axis.dir.index();
        let (a, b) = ((i + 1) % 3, (i + 2) % 3);
        let (far, near) = (self.geometry.far_corner(), self.geometry.near_corner());
        let points: Vec<Vec3> = locs
            .iter()
            .flat_map(|&loc| {
                let mut corner = far;
                corner[i] = axis.scale.masked_forward(loc);
                let (mut start, mut end) = (corner, corner);
                start[a] = near[a];
                end[b] = near[b];
                [start, corner, end]
            })
            .collect();
        let projected = self.projection.project_transformed_all(&points);
        (projected.as_chunks::<3>().0.iter())
            .map(|line| line.map(|q| Point::new(q[0], q[1])))
            .collect()
    }

    /// The box edge along `dir` at `position`.
    fn edge(&self, dir: Axis3, position: AxisPosition) -> AxisEdge {
        let points = self
            .geometry
            .axis_line(dir, self.axes.camera.vertical, position);
        let projected = self.projection.project_transformed_all(&points);
        let ends = [projected[0], projected[1]];
        // Texts along the edge follow it in axes coordinates.
        let delta = (
            (ends[1][0] - ends[0][0]) * self.extent.width(),
            (ends[1][1] - ends[0][1]) * self.extent.height(),
        );
        AxisEdge {
            points,
            ends,
            delta,
        }
    }

    /// Draws the gridline of a tick of `dir`, which a 3-D axis collapses to
    /// the origin of the projection plane (matplotlib's
    /// `_tick_update_position` sets `tick.gridline` to the single point
    /// `(0, 0)`, the 3-D gridlines being drawn by `draw_grid`): a stroke
    /// with nothing to paint, which only vector documents record. The x
    /// and y gridlines are clipped to the axes, whose patch `Axes.clear`
    /// makes the clip path of those two axes only.
    fn draw_tick_gridline(&self, ctx: &mut DrawContext<'_>, dir: Axis3, grid: &GridStyle) {
        if !grid.linestyle.is_visible() {
            return;
        }
        let mut gc = match grid_gc(grid, self.axes.rc()) {
            Ok(gc) => gc,
            Err(err) => return self.report(ctx, err),
        };
        if dir != Axis3::Z {
            gc.clip_rect = ctx.layout().clip_extent(self.axes.base_id());
        }
        let origin = Path::polyline(vec![Point::new(0.0, 0.0)]);
        let (path, affine) = self.plane.split_path(&origin);
        let drawn = with_path_effects(ctx, &grid.path_effects, |ctx| {
            ctx.renderer().draw_path(&gc, &path, &affine, None);
        });
        if let Err(err) = drawn {
            self.report(ctx, err);
        }
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

    /// Gives `text` to `sink`, in its SVG group when it is not empty
    /// (matplotlib's `Text.draw` opens none for an empty text).
    fn text_in_group(&self, sink: &mut impl Sink, text: &PlacedText<'_>) {
        if text.text.is_empty() {
            sink.text(self, text);
        } else {
            grouped(sink, "text", text.gid, |sink| sink.text(self, text));
        }
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
        if let Err(err) = draw_laid_out(ctx, &gc, &laid_out, anchor, text.block.rotation) {
            self.report(ctx, err);
        }
    }
}

/// The vertices of an open polygon: without the last one when it repeats
/// the first, as for a pane seen edge-on (matplotlib's `Polygon.set_xy`
/// with `closed=False`).
fn open_polygon(mut vertices: Vec<Point>) -> Vec<Point> {
    if vertices.len() > 2 && vertices.first() == vertices.last() {
        vertices.pop();
    }
    vertices
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The strokes of a lone move-to in the SVG of a default 3-D axes
    /// drawn with `axes.grid` set to `grid`: `(clipped, unclipped)`.
    fn lone_point_strokes(grid: &str) -> (usize, usize) {
        crate::rc::context(|| {
            crate::rc::set("axes.grid", grid).unwrap();
            let fig = crate::figure::Figure::new();
            fig.subplot_3d();
            let svg = fig.to_svg().unwrap();
            let lone: Vec<&str> = svg
                .split("<path ")
                .filter(|element| {
                    let d = element
                        .split_once("d=\"")
                        .and_then(|(_, rest)| rest.split_once('"'))
                        .map_or("", |(d, _)| d);
                    d.starts_with('M') && d.split_whitespace().count() == 3
                })
                .collect();
            let clipped = lone.iter().filter(|e| e.contains("clip-path")).count();
            (clipped, lone.len() - clipped)
        })
    }

    #[test]
    fn tick_gridlines_collapse_to_the_plane_origin() {
        assert_eq!(lone_point_strokes("False"), (0, 0));
        let (clipped, unclipped) = lone_point_strokes("True");
        // Six ticks per axis; only the x and y axes clip to the axes.
        assert_eq!((clipped, unclipped), (12, 6));
    }

    #[test]
    fn edge_on_panes_drop_their_closing_vertex() {
        let (a, b) = (Point::new(0.0, 0.0), Point::new(1.0, 0.0));
        assert_eq!(open_polygon(vec![a, b, b, a]), [a, b, b]);
        let square = vec![a, b, Point::new(1.0, 1.0), Point::new(0.0, 1.0)];
        assert_eq!(open_polygon(square.clone()), square);
        assert_eq!(open_polygon(vec![a, a]), [a, a]);
    }
}
