//! The `Line2D` artist: a polyline with optional markers.

use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_render::{CapStyle, GraphicsContext, JoinStyle};

use super::axline::InfiniteLine;
use super::hit_region::hit_region;
use super::line_markers::MarkerError;
use super::subslice::visible_range;
use super::{DrawStyle, LineStyle, MarkEvery, StepError, inverse_dashes, line_vertices};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, LimitPoints, RawHandle,
    TypedHandle, segment_hits,
};
use crate::colors::Color;
use crate::markers::{FillStyle, Marker};
use crate::patches::ClipRegion;
use crate::rc::{ColorOrAuto, RcParams};
use crate::transforms::{ResolvedTransform, Transform};

/// A marker rotation following the θ orientation of polar axes: by angle
/// θ shifted the way the axes shift θ, plus a fixed turn (matplotlib's
/// `_ScaledRotation(theta, transShift)`, evaluated when drawing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PolarMarkerRotation {
    /// The data angle in radians.
    pub(crate) theta: f64,
    /// A further turn in radians.
    pub(crate) extra: f64,
}

impl PolarMarkerRotation {
    /// The rotation in radians inside axes `parent`: θ times the θ
    /// direction plus the θ offset when they are polar, plus the extra
    /// turn.
    pub(super) fn angle(
        self,
        ctx: LayoutContext<'_>,
        parent: Option<crate::artist::ArtistId>,
    ) -> f64 {
        let shifted = parent
            .and_then(|axes| ctx.store().get_as::<crate::axes::AxesState>(axes))
            .and_then(crate::axes::AxesState::polar)
            .map_or(self.theta, |polar| {
                self.theta * polar.direction.sign() + polar.theta_offset
            });
        shifted + self.extra
    }
}

/// State of a [`Line2D`] (matplotlib's `Line2D`).
#[derive(Debug)]
pub(crate) struct LineArtist {
    pub(super) base: ArtistBase,
    pub(crate) xdata: Vec<f64>,
    pub(crate) ydata: Vec<f64>,
    /// Vertices actually drawn (draw style applied), in data coordinates,
    /// or why the data cannot be drawn.
    path: Result<Path, StepError>,
    pub(crate) color: Color,
    pub(crate) linewidth: f64,
    pub(crate) linestyle: LineStyle,
    pub(crate) drawstyle: DrawStyle,
    pub(crate) gapcolor: Color,
    pub(crate) marker: Marker,
    pub(crate) fillstyle: FillStyle,
    pub(crate) marker_transform: Option<Affine2D>,
    pub(crate) marker_capstyle: Option<CapStyle>,
    pub(crate) marker_joinstyle: Option<JoinStyle>,
    pub(crate) markersize: f64,
    pub(crate) markeredgewidth: f64,
    pub(crate) markeredgecolor: ColorOrAuto,
    pub(crate) markerfacecolor: ColorOrAuto,
    pub(crate) markerfacecoloralt: Color,
    pub(crate) markevery: Option<MarkEvery>,
    /// Straight pieces each segment is split into before a non-affine
    /// transform bends it (1: drawn straight).
    pub(crate) interpolation_steps: usize,
    pub(crate) antialiased: bool,
    /// How far from the line, in points, a pick still hits it.
    pub(crate) pickradius: f64,
    pub(crate) solid_capstyle: CapStyle,
    pub(crate) solid_joinstyle: JoinStyle,
    pub(crate) dash_capstyle: CapStyle,
    pub(crate) dash_joinstyle: JoinStyle,
    /// Set for lines extending across the whole view (`axline`).
    pub(crate) infinite: Option<InfiniteLine>,
    /// Turns the markers through an angle of the polar axes the line is
    /// in (the caps of polar error bars).
    pub(crate) marker_rotation: Option<PolarMarkerRotation>,
    /// rc snapshot used to resolve named dash patterns.
    rc: std::rc::Rc<RcParams>,
}

impl LineArtist {
    /// A line through `(x, y)` with defaults from `rc`.
    pub(crate) fn new(xdata: Vec<f64>, ydata: Vec<f64>, rc: std::rc::Rc<RcParams>) -> Self {
        let mut line = Self {
            base: ArtistBase::with_zorder(2.0),
            xdata,
            ydata,
            path: Ok(Path::default()),
            color: rc.lines_color.clone(),
            linewidth: rc.lines_linewidth,
            linestyle: rc.lines_linestyle.clone(),
            drawstyle: DrawStyle::Default,
            gapcolor: Color::none(),
            marker: rc.lines_marker.clone(),
            fillstyle: rc
                .markers_fillstyle
                .parse()
                .expect("rc validates markers.fillstyle"),
            marker_transform: None,
            marker_capstyle: None,
            marker_joinstyle: None,
            markersize: rc.lines_markersize,
            markeredgewidth: rc.lines_markeredgewidth,
            markeredgecolor: rc.lines_markeredgecolor.clone(),
            markerfacecolor: rc.lines_markerfacecolor.clone(),
            markerfacecoloralt: Color::none(),
            markevery: None,
            interpolation_steps: 1,
            antialiased: rc.lines_antialiased,
            pickradius: 5.0,
            solid_capstyle: rc.lines_solid_capstyle,
            solid_joinstyle: rc.lines_solid_joinstyle,
            dash_capstyle: rc.lines_dash_capstyle,
            dash_joinstyle: rc.lines_dash_joinstyle,
            infinite: None,
            marker_rotation: None,
            rc,
        };
        line.recache();
        line
    }

    /// Rebuilds the drawn path after data or draw-style changes.
    pub(crate) fn recache(&mut self) {
        let steps = self.interpolation_steps;
        self.path = line_vertices(&self.xdata, &self.ydata, self.drawstyle)
            .map(|vertices| Path::polyline(vertices).with_interpolation_steps(steps));
    }

    /// The vertices actually drawn, in the line's coordinates.
    pub(crate) fn drawn_points(&self) -> Vec<Point> {
        self.path
            .as_ref()
            .map(|p| p.vertices().to_vec())
            .unwrap_or_default()
    }

    /// The data points (ignoring the draw style), for markers.
    pub(super) fn points(&self) -> Result<Path, StepError> {
        line_vertices(&self.xdata, &self.ydata, DrawStyle::Default).map(Path::polyline)
    }

    pub(super) fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// Draws the line through `drawn` and markers at `points`, both mapped
    /// to display space by `resolved`.
    pub(crate) fn draw_paths(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
        drawn: &Path,
        points: &Path,
    ) {
        ctx.renderer()
            .open_group("line2d", self.base.gid.as_deref());
        let drawn = crate::patheffects::with_path_effects(ctx, &self.base.path_effects, |ctx| {
            self.draw_line(ctx, resolved, drawn)
                .and_then(|()| self.draw_markers(ctx, resolved, points))
        })
        .unwrap_or_else(|err| Err(err.to_string()));
        ctx.renderer().close_group("line2d");
        if let Err(problem) = drawn {
            ctx.report(self.base.id(), problem);
        }
    }

    fn draw_line(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
        drawn: &Path,
    ) -> Result<(), String> {
        if !self.linestyle.is_visible() || drawn.is_empty() {
            return Ok(());
        }
        let (path, affine) = resolved.split_path(drawn);
        let dashed = self.linestyle.is_dashed();
        let mut gc = self.line_gc(ctx);
        gc.linewidth = self.linewidth;
        (gc.cap, gc.join) = if dashed {
            (self.dash_capstyle, self.dash_joinstyle)
        } else {
            (self.solid_capstyle, self.solid_joinstyle)
        };
        let dashes = self.linestyle.scaled_dashes(self.linewidth, &self.rc);
        if dashed && !self.gapcolor.is_none() {
            gc.foreground = self
                .gapcolor
                .resolve_alpha(self.base.alpha)
                .map_err(|e| e.to_string())?;
            gc.dashes = inverse_dashes(&dashes);
            ctx.renderer().draw_path(&gc, &path, &affine, None);
        }
        gc.foreground = self
            .color
            .resolve_alpha(self.base.alpha)
            .map_err(|e| e.to_string())?;
        gc.dashes = dashes;
        ctx.renderer().draw_path(&gc, &path, &affine, None);
        Ok(())
    }

    /// The drawn vertices in display space: the visible segment of an
    /// infinite line.
    fn display_points(&self, layout: LayoutContext<'_>) -> Option<Vec<Point>> {
        let resolved = layout.resolve(&self.transform())?;
        let drawn = match &self.infinite {
            Some(infinite) => infinite
                .display_segment(layout, &resolved, self.base.parent)
                .ok()?,
            None => resolved.transform_path(self.path.as_ref().ok()?),
        };
        Some(drawn.vertices().to_vec())
    }

    pub(super) fn line_gc(&self, ctx: &DrawContext<'_>) -> GraphicsContext {
        let mut gc = ctx.base_gc(&self.base);
        // Lines bake their alpha into the colours (matplotlib does not set
        // a forced alpha on the graphics context).
        gc.alpha = None;
        gc.antialiased = self.antialiased;
        gc
    }
}

impl Artist for LineArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some(resolved) = ctx.resolve(&self.transform()) else {
            ctx.report(self.base.id(), "its coordinate system no longer exists");
            return;
        };
        if let Some(infinite) = &self.infinite {
            match infinite.display_segment(*ctx.layout(), &resolved, self.base.parent) {
                Ok(segment) => {
                    self.draw_paths(ctx, &ResolvedTransform::identity(), &segment, &segment);
                }
                Err(problem) => ctx.report(self.base.id(), problem),
            }
            return;
        }
        let visible = (self.xdata.len() == self.ydata.len())
            .then(|| {
                visible_range(
                    *ctx.layout(),
                    &self.base,
                    &self.xdata,
                    self.markevery.is_some(),
                )
            })
            .flatten();
        let paths = match visible {
            Some(range) => {
                let (x, y) = (&self.xdata[range.clone()], &self.ydata[range]);
                line_vertices(x, y, self.drawstyle).and_then(|drawn| {
                    let points = line_vertices(x, y, DrawStyle::Default)?;
                    let drawn =
                        Path::polyline(drawn).with_interpolation_steps(self.interpolation_steps);
                    Ok((drawn, Path::polyline(points)))
                })
            }
            None => self
                .path
                .clone()
                .and_then(|drawn| Ok((drawn, self.points()?))),
        };
        match paths {
            Ok((drawn, points)) => self.draw_paths(ctx, &resolved, &drawn, &points),
            Err(problem) => ctx.report(self.base.id(), problem.to_string()),
        }
    }

    /// matplotlib's `Line2D.get_window_extent`: the display extent of the
    /// finite data points (of the part inside the axes for an infinite
    /// line), padded by half the marker size when there is a marker;
    /// `None` without a finite point.
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let resolved = ctx.resolve(&self.transform())?;
        let points: Vec<Point> = match &self.infinite {
            Some(infinite) => infinite
                .display_segment(*ctx, &resolved, self.base.parent)
                .ok()?
                .vertices()
                .to_vec(),
            None => self
                .points()
                .ok()?
                .vertices()
                .iter()
                .map(|&p| resolved.transform_point(p))
                .collect(),
        };
        let extent = Bbox::from_points(points.into_iter().filter(|p| p.is_finite()));
        if extent.is_null() {
            return None;
        }
        let pad = if self.marker.is_nothing() {
            0.0
        } else {
            ctx.points_to_pixels(self.markersize) * 0.5
        };
        Some(extent.padded(pad, pad))
    }

    /// The drawn vertices in data coordinates, in the directions where the
    /// line's coordinates are data values; infinite lines do not count
    /// (matplotlib's `_update_line_limits`).
    fn limit_points(&self, axes: ArtistId) -> Option<LimitPoints> {
        if self.infinite.is_some() {
            return None;
        }
        LimitPoints::through(&self.transform(), axes, || self.drawn_points())
    }

    /// matplotlib's `Line2D.contains` as a region: the surroundings of the
    /// vertices (of a line without a line style) or of the segments within
    /// the pick radius, which clip the arrows of annotations that are
    /// given the line as `patchA` or `patchB`.
    fn connection_region(&self, ctx: &LayoutContext<'_>) -> Option<ClipRegion> {
        let points = self.display_points(*ctx)?;
        let pixels = ctx.points_to_pixels(self.pickradius);
        Some(hit_region(&points, pixels, self.linestyle.is_visible()))
    }

    /// matplotlib's `Line2D.contains`: the vertices within the pick radius
    /// (in points) of a line without a line style, else the segments
    /// passing within it; step lines report data indices.
    fn pick_hit(
        &self,
        ctx: &LayoutContext<'_>,
        at: Point,
        radius: Option<f64>,
    ) -> Option<Vec<usize>> {
        let points = self.display_points(*ctx)?;
        let pixels = ctx.points_to_pixels(radius.unwrap_or(self.pickradius));
        let hits: Vec<usize> = if self.linestyle.is_visible() {
            let hits = segment_hits(at, &points, pixels);
            if self.drawstyle == DrawStyle::Default {
                hits
            } else {
                hits.into_iter().map(|i| i / 2).collect()
            }
        } else {
            let near = |p: &Point| (p.x - at.x).powi(2) + (p.y - at.y).powi(2) <= pixels * pixels;
            (0..points.len()).filter(|&i| near(&points[i])).collect()
        };
        (!hits.is_empty()).then_some(hits)
    }
}

/// A handle to a line in a figure (matplotlib's `Line2D`).
///
/// ```
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
/// let line = ax.plot([1.0, 2.0, 3.0], [4.0, 1.0, 2.0])?;
/// line.set_color("tab:orange").set_linestyle(LineStyle::Dashed).set_linewidth(2.5);
/// assert_eq!(line.linewidth(), 2.5);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line2D {
    raw: RawHandle,
}

impl ArtistHandle for Line2D {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for Line2D {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<LineArtist>().then(|| Self { raw: raw.clone() })
    }
}

impl Line2D {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The line's data (matplotlib's `get_data`).
    pub fn data(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw
            .with::<LineArtist, _>(|line| (line.xdata.clone(), line.ydata.clone()))
    }

    /// The line's data as points, as many as the shorter of x and y
    /// (matplotlib's `get_xydata`).
    pub fn xydata(&self) -> Vec<Point> {
        self.raw.with::<LineArtist, _>(|line| {
            (line.xdata.iter().zip(&line.ydata))
                .map(|(&x, &y)| Point::new(x, y))
                .collect()
        })
    }

    /// Replaces the line's data.
    pub fn set_data(&self, x: &[f64], y: &[f64]) -> &Self {
        self.raw.with_mut::<LineArtist, _>(|line| {
            line.xdata = x.to_vec();
            line.ydata = y.to_vec();
            line.recache();
        });
        self
    }

    /// The vertices actually drawn (with the draw style applied), in data
    /// coordinates (matplotlib's `get_path()`).
    pub fn path(&self) -> Result<Path, StepError> {
        self.raw.with::<LineArtist, _>(|line| line.path.clone())
    }

    /// The indices of the data points that get markers when the line is
    /// drawn with the figure's current layout, in drawing order
    /// (`markevery` applied; every point without it). Lay the figure out
    /// first ([`crate::figure::Figure::layout`]) so the view is final.
    pub fn marker_indices(&self) -> Result<Vec<usize>, MarkerError> {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<LineArtist, _>(|line| line.marker_indices(LayoutContext::new(&state)))
    }

    /// The colour of the marker edges (matplotlib's `get_markeredgecolor`):
    /// when automatic, the line colour, or black for filled markers in
    /// classic mode.
    pub fn effective_markeredgecolor(&self) -> Color {
        self.raw.with::<LineArtist, _>(LineArtist::marker_edge)
    }

    /// The colour of the marker faces (matplotlib's `get_markerfacecolor`):
    /// none when the fill style is [`FillStyle::None`](crate::markers::FillStyle::None),
    /// else the face colour, the line colour when automatic.
    pub fn effective_markerfacecolor(&self) -> Color {
        self.raw
            .with::<LineArtist, _>(LineArtist::marker_face_color)
    }

    /// The resolved line colour.
    pub fn rgba(&self) -> Result<Rgba, crate::colors::ColorError> {
        self.raw
            .with::<LineArtist, _>(|line| line.color.resolve_alpha(line.base.alpha))
    }
}

artist_properties! {
    Line2D => LineArtist {
        /// The x data.
        xdata, set_xdata: Vec<f64> => recache;
        /// The y data.
        ydata, set_ydata: Vec<f64> => recache;
    }
}

artist_properties! {
    Line2D => LineArtist, LineProps {
        /// Line colour.
        color, set_color: Color;
        /// Line width in points.
        linewidth, set_linewidth: f64;
        /// Line style (solid, dashed, custom dashes, none).
        linestyle, set_linestyle: LineStyle;
        /// How points are connected (straight or steps).
        drawstyle, set_drawstyle: DrawStyle => recache;
        /// Colour drawn in the gaps of a dashed line (`"none"` leaves them empty).
        gapcolor, set_gapcolor: Color;
        /// Marker drawn at each data point.
        marker, set_marker: Marker;
        /// Which part of the marker is filled with the face colour.
        fillstyle, set_fillstyle: FillStyle;
        /// A transform applied to the marker before it is scaled to
        /// `markersize` (matplotlib's `MarkerStyle(transform=...)`).
        marker_transform, set_marker_transform: Option<Affine2D>;
        /// Cap style of the marker edges, replacing the marker's own
        /// (matplotlib's `MarkerStyle(capstyle=...)`).
        marker_capstyle, set_marker_capstyle: Option<CapStyle>;
        /// Join style of the marker edges, replacing the marker's own where
        /// the marker chooses one (matplotlib's `MarkerStyle(joinstyle=...)`).
        marker_joinstyle, set_marker_joinstyle: Option<JoinStyle>;
        /// Marker size in points.
        markersize, set_markersize: f64;
        /// Marker edge width in points.
        markeredgewidth, set_markeredgewidth: f64;
        /// Marker edge colour (`"auto"` follows the line colour).
        markeredgecolor, set_markeredgecolor: ColorOrAuto;
        /// Marker face colour (`"auto"` follows the line colour).
        markerfacecolor, set_markerfacecolor: ColorOrAuto;
        /// Face colour of the alternate half of half-filled markers.
        markerfacecoloralt, set_markerfacecoloralt: Color;
        /// Which points get markers (`None` = all).
        markevery, set_markevery: Option<MarkEvery>;
        /// How many straight pieces each segment is split into before a
        /// non-affine transform such as the polar one bends it
        /// (matplotlib's `get_path()._interpolation_steps`; 1 draws the
        /// segments straight).
        interpolation_steps, set_interpolation_steps: usize => recache;
        /// Anti-aliasing.
        antialiased, set_antialiased: bool;
        /// How far from the line, in points, a pick still hits it (see
        /// [`crate::artist::Picker`]).
        pickradius, set_pickradius: f64;
        /// Cap style of solid lines.
        solid_capstyle, set_solid_capstyle: CapStyle;
        /// Join style of solid lines.
        solid_joinstyle, set_solid_joinstyle: JoinStyle;
        /// Cap style of dashed lines.
        dash_capstyle, set_dash_capstyle: CapStyle;
        /// Join style of dashed lines.
        dash_joinstyle, set_dash_joinstyle: JoinStyle;
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::lines::{AxLineThrough, LineProps};
    use plt_core::{Bbox, Point};

    #[test]
    fn window_extents_cover_the_finite_points_and_markers() {
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        let props = LineProps::fmt("o-").unwrap().markersize(10.0);
        let line = ax.plot_with([0.25, f64::NAN, 0.75], [0.5, 0.2, 1.0], props);
        let extent = line.unwrap().window_extent().unwrap();
        assert_eq!(extent, Bbox::from_extents(31.0, 67.0, 113.0, 149.0));
        let empty = ax.plot([f64::NAN], [1.0]).unwrap();
        assert_eq!(empty.window_extent(), None);
        let flat = ax
            .axline(
                Point::new(0.0, 0.5),
                AxLineThrough::Slope(0.0),
                LineProps::default(),
            )
            .unwrap();
        assert_eq!(
            flat.window_extent(),
            Some(Bbox::from_extents(0.0, 72.0, 144.0, 72.0))
        );
    }

    #[test]
    fn interpolation_steps_reach_the_drawn_path() {
        let line = Figure::new()
            .subplot()
            .plot([0.0, 1.0], [1.0, 2.0])
            .unwrap();
        assert_eq!(line.path().unwrap().interpolation_steps(), 1);
        line.set_interpolation_steps(100_usize);
        assert_eq!(line.interpolation_steps(), 100);
        assert_eq!(line.path().unwrap().interpolation_steps(), 100);
    }

    #[test]
    fn slopes_are_in_data_units_whatever_the_point_coordinates() {
        // matplotlib's test_axline_transaxes_panzoom.
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        let line = ax
            .axline(
                Point::new(0.5, 0.5),
                AxLineThrough::Slope(2.0),
                LineProps::default(),
            )
            .unwrap();
        line.set_transform(ax.trans_axes());
        ax.set_xlim(0.0, 5.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        // Through the data point (2.5, 5) with slope 2: y = 2x, from the
        // bottom-left corner to the top-right one.
        assert_eq!(
            line.window_extent(),
            Some(Bbox::from_extents(0.0, 0.0, 144.0, 144.0))
        );
    }
}
