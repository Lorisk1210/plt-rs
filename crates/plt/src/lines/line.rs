//! The `Line2D` artist: a polyline with optional markers.

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{CapStyle, GraphicsContext, JoinStyle, SnapMode};

use super::{DrawStyle, LineStyle, MarkEvery, StepError, inverse_dashes, line_vertices};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, RawHandle};
use crate::colors::Color;
use crate::markers::{FillStyle, Marker, MarkerStyle};
use crate::rc::{ColorOrAuto, RcParams};
use crate::transforms::{ResolvedTransform, Transform};

/// State of a [`Line2D`] (matplotlib's `Line2D`).
#[derive(Debug)]
pub(crate) struct LineArtist {
    base: ArtistBase,
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
    pub(crate) markersize: f64,
    pub(crate) markeredgewidth: f64,
    pub(crate) markeredgecolor: ColorOrAuto,
    pub(crate) markerfacecolor: ColorOrAuto,
    pub(crate) markerfacecoloralt: Color,
    pub(crate) markevery: Option<MarkEvery>,
    pub(crate) antialiased: bool,
    pub(crate) solid_capstyle: CapStyle,
    pub(crate) solid_joinstyle: JoinStyle,
    pub(crate) dash_capstyle: CapStyle,
    pub(crate) dash_joinstyle: JoinStyle,
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
            markersize: rc.lines_markersize,
            markeredgewidth: rc.lines_markeredgewidth,
            markeredgecolor: rc.lines_markeredgecolor.clone(),
            markerfacecolor: rc.lines_markerfacecolor.clone(),
            markerfacecoloralt: Color::none(),
            markevery: None,
            antialiased: rc.lines_antialiased,
            solid_capstyle: rc.lines_solid_capstyle,
            solid_joinstyle: rc.lines_solid_joinstyle,
            dash_capstyle: rc.lines_dash_capstyle,
            dash_joinstyle: rc.lines_dash_joinstyle,
            rc,
        };
        line.recache();
        line
    }

    /// Rebuilds the drawn path after data or draw-style changes.
    pub(crate) fn recache(&mut self) {
        self.path = line_vertices(&self.xdata, &self.ydata, self.drawstyle).map(Path::polyline);
    }

    /// The vertices actually drawn, in the line's coordinates.
    pub(crate) fn drawn_points(&self) -> Vec<Point> {
        self.path
            .as_ref()
            .map(|p| p.vertices().to_vec())
            .unwrap_or_default()
    }

    /// The data points (ignoring the draw style), for markers.
    fn points(&self) -> Result<Path, StepError> {
        line_vertices(&self.xdata, &self.ydata, DrawStyle::Default).map(Path::polyline)
    }

    fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    fn draw_line(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
    ) -> Result<(), String> {
        let drawn = self.path.as_ref().map_err(ToString::to_string)?;
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

    fn draw_markers(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
    ) -> Result<(), String> {
        if self.marker.is_nothing() || self.markersize <= 0.0 || self.xdata.is_empty() {
            return Ok(());
        }
        let style =
            MarkerStyle::new(self.marker.clone(), self.fillstyle).map_err(|e| e.to_string())?;
        let points = self.points().map_err(|e| e.to_string())?;
        let (points, affine) = resolved.split_path(&points);
        let points = self.subsample(ctx, &points, &affine)?;
        if points.is_empty() {
            return Ok(());
        }
        let alpha = self.base.alpha;
        let face_spec = self.marker_face(&style, false);
        let face = face_spec.resolve_alpha(alpha).map_err(|e| e.to_string())?;
        let face_alt = self
            .marker_face(&style, true)
            .resolve_alpha(alpha)
            .map_err(|e| e.to_string())?;
        let mut edge = self
            .markeredgecolor
            .or(&self.color)
            .resolve_alpha(alpha)
            .map_err(|e| e.to_string())?;
        // An automatic edge takes the line colour but the face's alpha.
        if self.markeredgecolor == ColorOrAuto::Auto && !face_spec.is_none() {
            edge = edge.with_alpha(face.a);
        }

        let mut gc = self.line_gc(ctx);
        gc.linewidth = self.markeredgewidth;
        gc.foreground = edge;
        gc.join = style.joinstyle();
        gc.cap = style.capstyle();
        let size = ctx.points_to_pixels(self.markersize);
        gc.snap = match style.snap_threshold() {
            Some(threshold) if size >= threshold => SnapMode::Always,
            Some(_) => SnapMode::Never,
            None => gc.snap,
        };
        let is_pixel = matches!(
            self.marker,
            Marker::Symbol(crate::markers::MarkerSymbol::Pixel)
        );
        let scale = |t: Affine2D| if is_pixel { t } else { t.scale(size, size) };
        if is_pixel {
            gc.linewidth = 0.0;
        }
        ctx.renderer().draw_markers(
            &gc,
            style.path(),
            &scale(style.transform()),
            &points,
            &affine,
            Some(face),
        );
        if let (Some(alt_path), Some(alt_transform)) = (style.alt_path(), style.alt_transform()) {
            ctx.renderer().draw_markers(
                &gc,
                alt_path,
                &scale(alt_transform),
                &points,
                &affine,
                Some(face_alt),
            );
        }
        Ok(())
    }

    /// The face colour of the marker (or its alternate half).
    fn marker_face(&self, style: &MarkerStyle, alt: bool) -> Color {
        if style.fillstyle() == FillStyle::None {
            return Color::none();
        }
        if alt {
            return self.markerfacecoloralt.clone();
        }
        self.markerfacecolor.or(&self.color).clone()
    }

    fn subsample(
        &self,
        ctx: &DrawContext<'_>,
        points: &Path,
        affine: &Affine2D,
    ) -> Result<Path, String> {
        let Some(markevery) = &self.markevery else {
            return Ok(points.clone());
        };
        let display: Vec<Point> = points
            .vertices()
            .iter()
            .map(|p| affine.transform_point(*p))
            .collect();
        let diagonal = self
            .base
            .parent
            .and_then(|axes| ctx.layout().axes_extent(axes))
            .map_or(0.0, |bbox| bbox.p0.distance(bbox.p1));
        let indices = markevery
            .select(&display, diagonal)
            .map_err(|e| e.to_string())?;
        Ok(Path::polyline(
            indices.into_iter().map(|i| points.vertices()[i]).collect(),
        ))
    }

    fn line_gc(&self, ctx: &DrawContext<'_>) -> GraphicsContext {
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
        ctx.renderer()
            .open_group("line2d", self.base.gid.as_deref());
        let drawn = self
            .draw_line(ctx, &resolved)
            .and_then(|()| self.draw_markers(ctx, &resolved));
        ctx.renderer().close_group("line2d");
        if let Err(problem) = drawn {
            ctx.report(self.base.id(), problem);
        }
    }
}

/// A handle to a line in a figure (matplotlib's `Line2D`).
///
/// ```
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
/// let line = ax.plot([1.0, 2.0, 3.0], [4.0, 1.0, 2.0])?;
/// line.set_color("tab:orange").set_linestyle(LineStyle::Dashed).set_linewidth(2.5);
/// assert_eq!(line.linewidth(), 2.5);
/// # Ok::<(), plt::axes::PlotError>(())
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

impl Line2D {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
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

    /// The colour of the marker edges (the line colour when automatic).
    pub fn effective_markeredgecolor(&self) -> Color {
        self.raw
            .with::<LineArtist, _>(|line| line.markeredgecolor.or(&line.color).clone())
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
        /// Anti-aliasing.
        antialiased, set_antialiased: bool;
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
