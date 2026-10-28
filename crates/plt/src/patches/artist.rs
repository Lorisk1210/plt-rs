//! The generic patch artist: the state of a shape painted as a patch,
//! and the parts other artists draw patches from.

use std::any::Any;
use std::rc::Rc;

use plt_core::{Affine2D, Bbox, Path, Point};
use plt_render::ClipPath;

use super::shape::{DisplayPath, PatchShape, ShapeFrame};
use super::style::{PatchStyle, paint_paths};
use super::{ClipRegion, DataRectangle, PatchError, Rectangle};
use crate::artist::{
    Artist, ArtistBase, ArtistId, DrawContext, DrawnDpiCor, LayoutContext, LimitPoints, Picker,
};
use crate::rc::RcParams;
use crate::ticker::AxisName;
use crate::transforms::Transform;
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
}

impl<S: PatchShape> PatchParts<'_, S> {
    /// The outlines in display space, with arrow sizes at `dpi_cor`
    /// pixels per point.
    pub(crate) fn display_paths(
        &self,
        layout: LayoutContext<'_>,
        dpi_cor: f64,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        let transform = self.base.transform.clone().unwrap_or(Transform::Display);
        let frame = ShapeFrame {
            layout,
            transform: layout.resolve(&transform).ok_or(PatchError::Detached)?,
            dpi_cor,
            linewidth: self.style.linewidth.unwrap_or(self.rc.patch_linewidth),
            parent: self.base.parent,
            interpolation_steps: self.interpolation_steps,
        };
        self.shape.display_paths(&frame)
    }

    /// Draws the patch (matplotlib's `Patch.draw`).
    pub(crate) fn paint(&self, ctx: &mut DrawContext<'_>) -> Result<(), PatchError> {
        let layout = *ctx.layout();
        let dpi_cor = layout.points_to_pixels(1.0);
        if let Some(drawn) = self.drawn {
            drawn.record(dpi_cor);
        }
        let paths = self.display_paths(layout, dpi_cor)?;
        paint_paths(ctx, self.base, self.style, self.rc, S::EDGE_DEFAULT, &paths)?;
        Ok(())
    }

    /// The extent of the outlines in display space, as measured (see
    /// [`DrawnDpiCor`]).
    pub(crate) fn extent(&self, layout: LayoutContext<'_>) -> Option<Bbox> {
        let dpi_cor = self.drawn.map_or_else(
            || layout.measured_dpi_cor(),
            |drawn| drawn.measured(layout, self.base.id()),
        );
        self.display_paths(layout, dpi_cor)
            .ok()?
            .iter()
            .map(|path| path.to_display().extents())
            .reduce(|a, b| a.union(&b))
    }

    /// The outline as a clip region.
    pub(crate) fn outline(&self, layout: LayoutContext<'_>) -> Option<ClipPath> {
        let mut paths = self
            .display_paths(layout, layout.points_to_pixels(1.0))
            .ok()?;
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
#[derive(Debug)]
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
}

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
        }
    }

    fn parts(&self) -> PatchParts<'_, S> {
        PatchParts {
            base: &self.base,
            style: &self.style,
            rc: &self.rc,
            shape: &self.shape,
            interpolation_steps: self.interpolation_steps,
            drawn: Some(&self.dpi_cor),
        }
    }

    /// Points bounding the outline (ends and axis extrema of every curve),
    /// in the coordinates of the artist transform.
    pub(crate) fn data_points(&self) -> Vec<Point> {
        if self.shape.skips_autoscale() {
            return Vec::new();
        }
        let placement = self.shape.patch_transform();
        self.shape
            .path()
            .beziers()
            .flat_map(|curve| {
                let mut ts = vec![0.0];
                ts.extend(curve.axis_aligned_extrema());
                ts.push(1.0);
                ts.into_iter().map(move |t| curve.eval(t))
            })
            .map(|p| placement.transform_point(p))
            .collect()
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
            Some(Picker::Custom(_)) | None => self.edge_radius(),
        }
    }

    /// The outline in display space with tolerance `radius`.
    fn region(&self, ctx: LayoutContext<'_>, radius: f64) -> Option<ClipRegion> {
        self.outline(&ctx).map(|clip| ClipRegion {
            path: clip.path.transformed(&clip.transform),
            radius,
        })
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
        if let Err(err) = self.parts().paint(ctx) {
            ctx.report(self.base.id(), err);
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

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.parts().extent(*ctx)
    }

    fn outline(&self, ctx: &LayoutContext<'_>) -> Option<ClipPath> {
        self.parts().outline(*ctx)
    }

    /// The outline's ends and axis extrema, in the directions where the
    /// patch's coordinates are data values (matplotlib's
    /// `_update_patch_limits`).
    fn limit_points(&self, axes: ArtistId) -> Option<LimitPoints> {
        LimitPoints::through(self.base.transform.as_ref()?, axes, || self.data_points())
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
