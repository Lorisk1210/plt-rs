//! The `Line2D` artist: a polyline with optional markers. This module
//! holds its state and drawing; the handle is in `handle`.

mod handle;

pub use handle::{Line2D, LineProps};

use std::borrow::Cow;
use std::ops::Range;

use pltrs_core::{Affine2D, Bbox, Path, Point};
use pltrs_render::{CapStyle, GraphicsContext, JoinStyle};

use super::axline::{InfiniteLine, SegmentError};
use super::hit_region::hit_region;
use super::subslice::visible_range;
use super::{DrawStyle, LineStyle, MarkEvery, StepError, inverse_dashes, line_vertices};
use crate::artist::{
    Artist, ArtistBase, ArtistId, DrawContext, DrawErrorKind, LayoutContext, LimitPoints,
    segment_hits,
};
use crate::colors::Color;
use crate::markers::{FillStyle, Marker};
use crate::patches::ClipRegion;
use crate::rc::{ColorOrAuto, RcParams};
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform};
use crate::units::{CustomValues, UnitSources};

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

pltrs_core::archive_struct!(PolarMarkerRotation { theta, extra });

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
#[derive(Clone, Debug)]
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
    /// The data that came from user values, kept so that the line
    /// converts it again when its axis changes unit; replacing the data
    /// drops it.
    pub(crate) unit_sources: UnitSources,
    /// rc snapshot used to resolve named dash patterns.
    rc: std::rc::Rc<RcParams>,
}

pltrs_core::archive_struct!(LineArtist {
    base,
    xdata,
    ydata,
    path,
    color,
    linewidth,
    linestyle,
    drawstyle,
    gapcolor,
    marker,
    fillstyle,
    marker_transform,
    marker_capstyle,
    marker_joinstyle,
    markersize,
    markeredgewidth,
    markeredgecolor,
    markerfacecolor,
    markerfacecoloralt,
    markevery,
    interpolation_steps,
    antialiased,
    pickradius,
    solid_capstyle,
    solid_joinstyle,
    dash_capstyle,
    dash_joinstyle,
    infinite,
    marker_rotation,
    unit_sources,
    rc
});

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
            fillstyle: rc.markers_fillstyle,
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
            unit_sources: UnitSources::default(),
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

    /// Replaces the data along axis `name` with `numbers`, converted from
    /// `source` when they come from user values.
    pub(super) fn replace_data(
        &mut self,
        name: AxisName,
        numbers: Vec<f64>,
        source: Option<CustomValues>,
    ) {
        match name {
            AxisName::X => self.xdata = numbers,
            AxisName::Y => self.ydata = numbers,
        }
        self.unit_sources.replace(name, source);
        self.recache();
    }

    /// Takes `numbers`, the user values the data along axis `name` came
    /// from converted again; one number stands for all the points, as
    /// the value of `axhline` or `axvline` does (matplotlib's
    /// `recache_always`).
    fn reconvert(&mut self, name: AxisName, numbers: Vec<f64>) {
        let data = match name {
            AxisName::X => &mut self.xdata,
            AxisName::Y => &mut self.ydata,
        };
        *data = match numbers.as_slice() {
            &[number] => vec![number; data.len()],
            _ => numbers,
        };
        self.recache();
    }

    /// The vertices actually drawn, in the line's coordinates.
    pub(crate) fn drawn_points(&self) -> Vec<Point> {
        self.path
            .as_ref()
            .map(|p| p.vertices().to_vec())
            .unwrap_or_default()
    }

    /// The data points (ignoring the draw style), for markers: the drawn
    /// path itself for a line without steps.
    pub(super) fn points(&self) -> Result<Cow<'_, Path>, StepError> {
        if self.drawstyle == DrawStyle::Default {
            return self.path.as_ref().map(Cow::Borrowed).map_err(Clone::clone);
        }
        line_vertices(&self.xdata, &self.ydata, DrawStyle::Default)
            .map(|vertices| Cow::Owned(Path::polyline(vertices)))
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

    /// The drawn vertices in display space.
    pub(super) fn display_points(&self, layout: LayoutContext<'_>) -> Option<Vec<Point>> {
        let resolved = self.drawn_transform(layout).ok()??;
        let drawn = resolved.transform_path(self.path.as_ref().ok()?);
        Some(drawn.vertices().to_vec())
    }

    /// The transform the line's data are drawn through: its own, or for an
    /// infinite line the map onto the part of it inside the axes (see
    /// [`InfiniteLine::display_map`]), which fails when the line cannot
    /// be placed; `None` when the line's coordinate system no longer
    /// exists.
    fn drawn_transform(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Option<ResolvedTransform>, SegmentError> {
        let Some(resolved) = layout.resolve(&self.transform()) else {
            return Ok(None);
        };
        match &self.infinite {
            Some(infinite) => infinite
                .display_map(layout, &resolved, self.base.parent)
                .map(Some),
            None => Ok(Some(resolved)),
        }
    }

    /// The points a long line draws when only part of it is in view (see
    /// [`visible_range`]); `None` when it draws them all. An infinite
    /// line's data are not in data coordinates, which this needs.
    fn drawn_range(&self, layout: LayoutContext<'_>) -> Option<Range<usize>> {
        let n = self.xdata.len();
        if self.infinite.is_some() || n != self.ydata.len() {
            return None;
        }
        visible_range(layout, &self.base, &self.xdata, self.markevery.is_some())
            .filter(|range| *range != (0..n))
    }

    /// The path the line draws through its points `range` (all of them
    /// for `None`), its draw style applied: the cached path when it draws
    /// them all, so that a long line is not copied to be drawn.
    fn drawn_path(&self, range: Option<Range<usize>>) -> Result<Cow<'_, Path>, StepError> {
        if range.is_none() {
            return self.path.as_ref().map(Cow::Borrowed).map_err(Clone::clone);
        }
        let (x, y) = self.data_in(range);
        let vertices = line_vertices(x, y, self.drawstyle)?;
        Ok(Cow::Owned(
            Path::polyline(vertices).with_interpolation_steps(self.interpolation_steps),
        ))
    }

    /// Where the line's markers go among its points `range` (all of them
    /// for `None`), with `drawn` the path it draws through them: the
    /// same vertices for a line without steps, borrowed then (the
    /// renderers place markers at the vertices whatever the path's
    /// interpolation steps), and nothing for a line without markers.
    fn marker_points<'a>(
        &'a self,
        drawn: &'a Path,
        range: Option<Range<usize>>,
    ) -> Result<Cow<'a, Path>, StepError> {
        if self.marker.is_nothing() || self.markersize <= 0.0 {
            return Ok(Cow::Owned(Path::default()));
        }
        if self.drawstyle == DrawStyle::Default {
            return Ok(Cow::Borrowed(drawn));
        }
        let (x, y) = self.data_in(range);
        line_vertices(x, y, DrawStyle::Default).map(|vertices| Cow::Owned(Path::polyline(vertices)))
    }

    /// The data of the points `range`, all of them for `None`.
    fn data_in(&self, range: Option<Range<usize>>) -> (&[f64], &[f64]) {
        match range {
            Some(range) => (&self.xdata[range.clone()], &self.ydata[range]),
            None => (&self.xdata, &self.ydata),
        }
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

    fn unit_sources(&self, name: AxisName) -> Vec<CustomValues> {
        self.unit_sources.along(name).cloned().into_iter().collect()
    }

    fn reconvert_units(&mut self, name: AxisName, numbers: Vec<Vec<f64>>) {
        if let Some(numbers) = numbers.into_iter().next() {
            self.reconvert(name, numbers);
        }
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let resolved = match self.drawn_transform(*ctx.layout()) {
            Ok(Some(resolved)) => resolved,
            Ok(None) => {
                ctx.report(self.base.id(), "its coordinate system no longer exists");
                return;
            }
            Err(problem) => {
                ctx.report_kind(self.base.id(), problem.kind(), problem);
                return;
            }
        };
        let range = self.drawn_range(*ctx.layout());
        let drawn = self.drawn_path(range.clone());
        let paths = drawn
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|drawn| Ok((drawn, self.marker_points(drawn, range)?)));
        match paths {
            Ok((drawn, points)) => self.draw_paths(ctx, &resolved, drawn, &points),
            // Data that cannot be drawn: matplotlib's `recache` raises a
            // `ValueError` when it draws the line.
            Err(problem) => ctx.report_kind(self.base.id(), DrawErrorKind::InvalidValue, problem),
        }
    }

    /// matplotlib's `Line2D.get_window_extent`: the display extent of the
    /// finite data points (mapped onto the part inside the axes for an
    /// infinite line), padded by half the marker size when there is a
    /// marker; `None` without a finite point.
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let resolved = self.drawn_transform(*ctx).ok()??;
        let points = self.points().ok()?;
        let extent = Bbox::from_points(
            points
                .vertices()
                .iter()
                .map(|&p| resolved.transform_point(p))
                .filter(|p| p.is_finite()),
        );
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
