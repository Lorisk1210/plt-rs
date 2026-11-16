//! The generic patch artist: the state of a shape painted as a patch,
//! and the parts other artists draw patches from.

use std::any::Any;
use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox, Path, Point};
use pltrs_render::ClipPath;

use super::shape::{DisplayPath, PatchShape, ShapeFrame, outline_through};
use super::style::{PatchStyle, paint_paths};
use super::{ClipRegion, DataRectangle, PatchError, Rectangle};
use crate::artist::{
    Artist, ArtistBase, ArtistId, DrawContext, DrawnDpiCor, LayoutContext, LimitPoints, Picker,
    ProgramShaping,
};
use crate::colors::ColorError;
use crate::rc::RcParams;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform, TransformContext};
use crate::units::CustomValues;

/// The pieces a patch is drawn from, borrowed from whichever artist owns
/// them (a patch artist, or a text drawing its box or arrow).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PatchParts<'a, S> {
    pub(crate) base: &'a ArtistBase,
    pub(crate) style: &'a PatchStyle,
    pub(crate) rc: &'a RcParams,
    pub(crate) shape: &'a S,
    /// Pieces each outline segment is cut into for projections that bend
    /// lines (matplotlib's `get_path()._interpolation_steps`).
    pub(crate) interpolation_steps: usize,
    /// The resolution the patch last drew at, for patches whose arrow
    /// sizes follow it (`FancyArrowPatch._dpi_cor`).
    pub(crate) drawn: Option<&'a DrawnDpiCor>,
    /// The outline a class of the program's makes and its patch
    /// transform, in place of the shape's (see
    /// [`PatchArtist::program_path`]).
    pub(crate) program_path: Option<&'a (Path, Affine2D)>,
}

impl<S: PatchShape> PatchParts<'_, S> {
    /// The outlines in display space, with arrow sizes at `dpi_cor`
    /// pixels per point.
    pub(crate) fn display_paths(
        &self,
        layout: LayoutContext<'_>,
        dpi_cor: f64,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        let resolved = self.resolved(layout)?;
        self.display_paths_through(layout, resolved, dpi_cor)
    }

    /// The artist's transform resolved in `layout`.
    fn resolved(&self, layout: LayoutContext<'_>) -> Result<ResolvedTransform, PatchError> {
        let transform = self.base.transform.clone().unwrap_or(Transform::Display);
        layout.resolve(&transform).ok_or(PatchError::Detached)
    }

    /// [`display_paths`](Self::display_paths) through the artist's
    /// transform, `resolved`.
    fn display_paths_through(
        &self,
        layout: LayoutContext<'_>,
        resolved: ResolvedTransform,
        dpi_cor: f64,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        if let Some((path, placement)) = self.program_path {
            // Through the composition of its patch transform and the
            // artist's, as matplotlib's draw maps the path.
            let placed = ResolvedTransform::affine(*placement).then(resolved);
            let outline = path
                .clone()
                .with_interpolation_steps(self.interpolation_steps);
            let (path, affine) = placed.split_outline(&outline);
            return Ok(vec![DisplayPath::filled(path.into_owned(), affine)]);
        }
        self.shape
            .display_paths(&self.frame(layout, resolved, dpi_cor))
    }

    /// Where the shape is drawn: in `layout`, through the artist's
    /// transform, `resolved`, with arrow sizes at `dpi_cor` pixels per
    /// point.
    fn frame<'a>(
        &self,
        layout: LayoutContext<'a>,
        resolved: ResolvedTransform,
        dpi_cor: f64,
    ) -> ShapeFrame<'a> {
        ShapeFrame {
            layout,
            transform: resolved,
            dpi_cor,
            drawn_dpi_cor: self.drawn.and_then(DrawnDpiCor::last),
            linewidth: self.style.linewidth.unwrap_or(self.rc.patch_linewidth),
            parent: self.base.parent,
            interpolation_steps: self.interpolation_steps,
        }
    }

    /// The calls of the program's styles the patch's draw makes in display
    /// space (see [`PatchShape::program_drawing`]).
    fn program_drawing(&self, layout: LayoutContext<'_>) -> Option<ProgramShaping> {
        let resolved = self.resolved(layout).ok()?;
        let frame = self.frame(layout, resolved, layout.points_to_pixels(1.0));
        self.shape.program_drawing(&frame)
    }

    /// Draws the patch (matplotlib's `Patch.draw`). A transform or a box
    /// style of the program's makes or maps the outline with the figure
    /// free, as matplotlib's may read and change the figure meanwhile
    /// (unless the shape needs more of the figure than its transform).
    pub(crate) fn paint(&self, ctx: &mut DrawContext<'_>) -> Result<(), PatchError> {
        let layout = ctx.layout();
        let dpi_cor = layout.points_to_pixels(1.0);
        if let Some(drawn) = self.drawn {
            drawn.record(dpi_cor);
        }
        let resolved = self.resolved(layout)?;
        let paths =
            if S::MAPS_OUTLINE_ALONE && (resolved.calls_program() || self.shape.runs_program()) {
                self.program_outline(ctx, &resolved)
            } else {
                self.display_paths_through(ctx.layout(), resolved, dpi_cor)?
            };
        paint_paths(ctx, self.base, self.style, self.rc, S::EDGE_DEFAULT, &paths)?;
        Ok(())
    }

    /// The outline made or mapped by code of the program's, the figure let
    /// go first (kept out of the draws of every other patch).
    #[inline(never)]
    fn program_outline(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
    ) -> Vec<DisplayPath> {
        ctx.free();
        outline_through(self.shape, resolved, self.interpolation_steps)
    }

    /// The pixels per point the patch is measured at (see
    /// [`DrawnDpiCor`]).
    fn measured_dpi_cor(&self, layout: LayoutContext<'_>) -> f64 {
        self.drawn.map_or_else(
            || layout.measured_dpi_cor(),
            |drawn| drawn.measured(layout, self.base.id()),
        )
    }

    /// The outlines in display space, as measured (arrow sizes at the
    /// pixels per point of the last draw, see [`DrawnDpiCor`]).
    pub(crate) fn measured_display_paths(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        self.display_paths(layout, self.measured_dpi_cor(layout))
    }

    /// The extent of the outlines in display space, as measured.
    pub(crate) fn extent(&self, layout: LayoutContext<'_>) -> Option<Bbox> {
        self.measured_display_paths(layout)
            .ok()?
            .iter()
            .map(|path| path.to_display().extents())
            .reduce(|a, b| a.union(&b))
    }

    /// The outline as a clip region.
    pub(crate) fn outline(&self, layout: LayoutContext<'_>) -> Option<ClipPath> {
        self.outline_at(layout, layout.points_to_pixels(1.0))
    }

    /// The outline as measured, which hit tests use (matplotlib's
    /// `Patch.contains` reads `get_path`, whose arrows keep the size of
    /// their last draw).
    pub(crate) fn measured_outline(&self, layout: LayoutContext<'_>) -> Option<ClipPath> {
        self.outline_at(layout, self.measured_dpi_cor(layout))
    }

    /// The outline as a clip region, arrow sizes at `dpi_cor` pixels per
    /// point.
    fn outline_at(&self, layout: LayoutContext<'_>, dpi_cor: f64) -> Option<ClipPath> {
        let mut paths = self.display_paths(layout, dpi_cor).ok()?;
        if paths.len() == 1 {
            let only = paths.remove(0);
            return Some(ClipPath {
                path: only.path,
                transform: only.affine,
            });
        }
        let displayed: Vec<Path> = paths.iter().map(DisplayPath::to_display).collect();
        Some(ClipPath {
            path: Path::compound(&displayed),
            transform: Affine2D::IDENTITY,
        })
    }
}

/// The parts every patch has, whatever its shape (matplotlib's `Patch`
/// base class), as [`Artist::as_patch`] shows them.
#[derive(Clone, Copy, Debug)]
pub struct PatchView<'a> {
    /// How the outline, face and hatch are painted.
    pub style: &'a PatchStyle,
    /// Whether the shape draws an edge by default (matplotlib's
    /// `_edge_default`).
    pub edge_default: bool,
    /// The geometry, for downcasting to its concrete shape.
    pub shape: &'a dyn Any,
}

/// A shape painted with a [`PatchStyle`] (matplotlib's `Patch` subclasses).
#[derive(Clone, Debug)]
pub(crate) struct PatchArtist<S: PatchShape> {
    pub(super) base: ArtistBase,
    pub(crate) style: PatchStyle,
    pub(crate) shape: S,
    /// Pieces each outline segment is cut into for projections that bend
    /// lines (matplotlib's `get_path()._interpolation_steps`).
    pub(crate) interpolation_steps: usize,
    /// The rectangle in plotting values the shape was converted from,
    /// which the patch converts again when an axis changes unit
    /// (matplotlib's `Rectangle._convert_units` when drawn); always a
    /// [`Rectangle`]'s, and forgotten when the shape is replaced.
    pub(crate) unit_source: Option<DataRectangle>,
    pub(super) rc: Rc<RcParams>,
    /// The resolution of the patch's last draw.
    dpi_cor: DrawnDpiCor,
    /// The outline a class of the program's makes and the map into the
    /// patch's coordinates it is placed by (its `get_path` and
    /// `get_patch_transform`, which matplotlib's draw and measures read):
    /// drawn, measured, hit and autoscaled in place of the shape's.
    pub(crate) program_path: Option<(Path, Affine2D)>,
}

pltrs_core::archive_struct!(PatchArtist<S: PatchShape> { base, style, shape, interpolation_steps, unit_source, rc, dpi_cor, program_path });

impl<S: PatchShape> PatchArtist<S> {
    /// A patch with matplotlib's defaults under `rc` (z-order 1), adjusted
    /// by the shape.
    pub(crate) fn new(shape: S, rc: Rc<RcParams>) -> Self {
        let mut base = ArtistBase::with_zorder(1.0);
        let mut style = PatchStyle::from_rc(&rc);
        shape.init_style(&mut style, &mut base);
        Self {
            base,
            style,
            shape,
            interpolation_steps: 1,
            unit_source: None,
            rc,
            dpi_cor: DrawnDpiCor::default(),
            program_path: None,
        }
    }

    pub(super) fn parts(&self) -> PatchParts<'_, S> {
        PatchParts {
            base: &self.base,
            style: &self.style,
            rc: &self.rc,
            shape: &self.shape,
            interpolation_steps: self.drawn_interpolation_steps(),
            drawn: Some(&self.dpi_cor),
            program_path: self.program_path.as_ref(),
        }
    }

    /// The interpolation steps the patch draws with: those all rectangles
    /// share when they are set, else its own.
    fn drawn_interpolation_steps(&self) -> usize {
        S::SHARES_UNIT_PATH
            .then(super::shared_rectangle_steps)
            .flatten()
            .unwrap_or(self.interpolation_steps)
    }

    /// The outlines in display space, with arrow sizes at `dpi_cor` pixels
    /// per point, for an artist drawing the patch as part of itself.
    pub(crate) fn display_paths_at(
        &self,
        layout: LayoutContext<'_>,
        dpi_cor: f64,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        self.parts().display_paths(layout, dpi_cor)
    }

    /// Paints `paths`, in display space, in the patch's style, as a patch
    /// updated from this one paints them (matplotlib's
    /// `PathPatch(path).update_from(patch)`).
    pub(crate) fn paint_display_paths(
        &self,
        ctx: &mut DrawContext<'_>,
        paths: &[DisplayPath],
    ) -> Result<(), ColorError> {
        paint_paths(
            ctx,
            &self.base,
            &self.style,
            &self.rc,
            S::EDGE_DEFAULT,
            paths,
        )
    }

    /// Whether the patch's edge looks like `other`'s: the same alpha, edge
    /// colour, line style and width (what matplotlib compares with
    /// `get_alpha`, `get_edgecolor`, `get_linestyle` and `get_linewidth`).
    pub(crate) fn shares_edge_style<T: PatchShape>(&self, other: &PatchArtist<T>) -> bool {
        let edge = |style: &PatchStyle, rc: &RcParams, alpha: Option<f64>, default: bool| {
            style
                .resolve(rc, alpha, default)
                .ok()
                .map(|resolved| (resolved.edge, resolved.linewidth))
        };
        self.base.alpha == other.base.alpha
            && self.style.linestyle == other.style.linestyle
            && edge(&self.style, &self.rc, self.base.alpha, S::EDGE_DEFAULT)
                == edge(&other.style, &other.rc, other.base.alpha, T::EDGE_DEFAULT)
    }

    /// Points bounding the outline (ends and axis extrema of every curve),
    /// in the coordinates of the artist transform.
    pub(crate) fn data_points(&self) -> Vec<Point> {
        if let Some((path, placement)) = &self.program_path {
            return path
                .bezier_bounding_points()
                .map(|p| placement.transform_point(p))
                .collect();
        }
        if self.shape.skips_autoscale() {
            return Vec::new();
        }
        let placement = self.shape.patch_transform();
        let path = self.shape.path();
        // Two points per segment of a polygon, the usual outline.
        let mut points = Vec::with_capacity(2 * path.vertices().len());
        points.extend(
            path.bezier_bounding_points()
                .map(|p| placement.transform_point(p)),
        );
        points
    }

    /// Whether display point `point` hits a patch without a picker: within
    /// the edge width of its outline when the edge shows (matplotlib's
    /// `Patch.contains` with its default radius).
    pub(crate) fn contains_unpicked(&self, ctx: LayoutContext<'_>, point: Point) -> bool {
        self.hit_region(ctx)
            .is_some_and(|region| region.contains(point))
    }

    /// The outline in display space with the edge width as tolerance when
    /// the edge shows (matplotlib's `Patch.contains` with its default
    /// radius).
    fn hit_region(&self, ctx: LayoutContext<'_>) -> Option<ClipRegion> {
        self.region(ctx, self.edge_radius())
    }

    /// The edge width when the edge shows, else 0.
    fn edge_radius(&self) -> f64 {
        self.style
            .resolve(&self.rc, self.base.alpha, S::EDGE_DEFAULT)
            .map_or(0.0, |resolved| resolved.hit_radius())
    }

    /// The tolerance of a hit test given no radius (matplotlib's
    /// `_process_radius(None)`): a numeric picker's radius, 1 pixel for
    /// `picker=True` (the number 1 to Python), else the edge width when
    /// the edge shows.
    fn default_radius(&self) -> f64 {
        match &self.base.picker {
            Some(Picker::Radius(radius)) => *radius,
            Some(Picker::Default) => 1.0,
            Some(Picker::Custom(_) | Picker::Press) | None => self.edge_radius(),
        }
    }

    /// The outline in display space, as measured, with tolerance
    /// `radius`.
    fn region(&self, ctx: LayoutContext<'_>, radius: f64) -> Option<ClipRegion> {
        self.parts()
            .measured_outline(ctx)
            .map(|clip| ClipRegion::new(clip.path.transformed(&clip.transform), radius))
    }

    /// Whether display point `point` lies within `radius` of a sub-path of
    /// the outline (matplotlib's `Patch.contains`).
    fn encloses(&self, ctx: LayoutContext<'_>, point: Point, radius: f64) -> bool {
        self.region(ctx, radius)
            .is_some_and(|region| region.contains(point))
    }
}

impl<S: PatchShape> Artist for PatchArtist<S> {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        // Once per draw of matplotlib's (twice for a print it draws once
        // before).
        for _ in 0..crate::text::font_notes::draws_of_print() {
            self.shape.warn_drawing();
        }
        if let Err(err) = self.parts().paint(ctx) {
            ctx.report_kind(self.base.id(), err.draw_kind(), err);
        }
    }

    fn unit_sources(&self, name: AxisName) -> Vec<CustomValues> {
        self.unit_source
            .as_ref()
            .map(|source| source.unit_sources(name))
            .unwrap_or_default()
    }

    fn reconvert_units(&mut self, name: AxisName, numbers: Vec<Vec<f64>>) {
        let rectangle = (&mut self.shape as &mut dyn Any).downcast_mut::<Rectangle>();
        if let (Some(source), Some(rectangle)) = (&self.unit_source, rectangle) {
            source.reconvert(name, rectangle, numbers);
        }
    }

    fn has_wild_transform(&self) -> bool {
        if self.shape.has_tame_numbers() {
            return false;
        }
        let matrix = self.shape.patch_transform().values();
        !matrix.into_iter().all(crate::transforms::is_tame)
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.parts().extent(*ctx)
    }

    fn outline(&self, ctx: &LayoutContext<'_>) -> Option<ClipPath> {
        self.parts().outline(*ctx)
    }

    fn limits_run_program(&self) -> bool {
        self.shape.runs_program()
    }

    /// The program's style shaping the patch, as its draw calls it
    /// (matplotlib's `get_path`), or those its draw calls in display space
    /// (an arrow's connection and arrow styles).
    fn program_shaping(&self, layout: &LayoutContext<'_>) -> Option<ProgramShaping> {
        if !self.shape.runs_program() {
            // Placed only when a style of the program's draws it.
            return (self.shape.draws_program())
                .then(|| self.parts().program_drawing(*layout))
                .flatten();
        }
        let shape = self.shape.clone();
        Some(Box::new(move || drop(shape.path())))
    }

    /// The outline's ends and axis extrema, in the directions where the
    /// patch's coordinates are data values (matplotlib's
    /// `_update_patch_limits`).
    fn limit_points(&self, axes: ArtistId, ctx: &dyn TransformContext) -> Option<LimitPoints> {
        let transform = self.base.transform.as_ref()?;
        if self.shape.runs_program() {
            // matplotlib's `_update_patch_limits` makes the path (calling a
            // style of the program's) before it looks at the transform.
            let points = self.data_points();
            return LimitPoints::through(transform, axes, || points);
        }
        let linewidth = self.style.linewidth.unwrap_or(self.rc.patch_linewidth);
        match self.shape.limit_outline(transform, linewidth, ctx, axes) {
            Some(points) => Some(LimitPoints {
                points,
                directions: transform.data_directions(axes),
            }),
            None => LimitPoints::through(transform, axes, || self.data_points()),
        }
    }

    /// Whether the outline encloses `point` (matplotlib's
    /// `Patch.contains` with a zero radius).
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        self.encloses(*ctx, point, 0.0)
    }

    fn connection_region(&self, ctx: &LayoutContext<'_>) -> Option<ClipRegion> {
        self.hit_region(*ctx)
    }

    /// matplotlib's `Patch.contains`: within `radius` of a sub-path of the
    /// outline, or within the radius the picker implies (1 pixel for
    /// [`Picker::Default`], as matplotlib's `picker=True` counts as the
    /// number 1; the edge width of a visible edge without a numeric
    /// picker).
    fn pick_hit(
        &self,
        ctx: &LayoutContext<'_>,
        at: Point,
        radius: Option<f64>,
    ) -> Option<Vec<usize>> {
        let radius = radius.unwrap_or_else(|| self.default_radius());
        self.encloses(*ctx, at, radius).then(Vec::new)
    }

    fn as_patch(&self) -> Option<PatchView<'_>> {
        Some(PatchView {
            style: &self.style,
            edge_default: S::EDGE_DEFAULT,
            shape: &self.shape,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounding_points_are_moves_segment_ends_and_curve_extrema() {
        let p = Point::new;
        let polyline = Path::polyline(vec![p(0.0, 0.0), p(1.0, 2.0), p(3.0, 1.0)]);
        assert_eq!(
            polyline.bezier_bounding_points().collect::<Vec<_>>(),
            [
                p(0.0, 0.0),
                p(0.0, 0.0),
                p(1.0, 2.0),
                p(1.0, 2.0),
                p(3.0, 1.0)
            ]
        );
        // A symmetric arch peaks at t = 1/2, between its ends, where x
        // turns (matplotlib evaluates those extrema too).
        let arch = Path::builder()
            .move_to(p(0.0, 0.0))
            .cubic_to(p(0.0, 4.0), p(2.0, 4.0), p(2.0, 0.0))
            .build();
        assert_eq!(
            arch.bezier_bounding_points().collect::<Vec<_>>(),
            [
                p(0.0, 0.0),
                p(0.0, 0.0),
                p(0.0, 0.0),
                p(2.0, 0.0),
                p(1.0, 3.0),
                p(2.0, 0.0)
            ]
        );
    }
}
