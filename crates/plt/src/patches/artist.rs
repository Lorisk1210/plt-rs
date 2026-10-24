//! The generic patch artist and its typed handle.

use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_render::{CapStyle, ClipPath, JoinStyle};

use super::shape::{DisplayPath, PatchShape, ShapeFrame};
use super::style::{PatchStyle, paint_paths};
use super::{ClipRegion, DataRectangle, PatchError, PatchProps, Rectangle};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, DrawnDpiCor, LayoutContext,
    LimitPoints, Picker, RawHandle, TypedHandle,
};
use crate::colors::{Color, ColorError};
use crate::lines::LineStyle;
use crate::rc::{ColorOrEdge, RcParams};
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
    base: ArtistBase,
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
    rc: Rc<RcParams>,
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

/// The colours a patch is painted with ([`Patch::colors`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchColors {
    /// The face (transparent when the patch is not filled).
    pub face: Rgba,
    /// The edge.
    pub edge: Rgba,
    /// The hatch lines.
    pub hatch: Rgba,
}

/// A handle to a patch whose geometry is an `S` (e.g. `Patch<Rectangle>`).
#[derive(Debug, PartialEq, Eq)]
pub struct Patch<S> {
    raw: RawHandle,
    shape: PhantomData<fn() -> S>,
}

impl<S> Clone for Patch<S> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            shape: PhantomData,
        }
    }
}

impl<S: PatchShape> ArtistHandle for Patch<S> {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl<S: PatchShape> TypedHandle for Patch<S> {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<PatchArtist<S>>()
            .then(|| Self::from_raw(raw.clone()))
    }
}

impl<S: PatchShape> Patch<S> {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self {
            raw,
            shape: PhantomData,
        }
    }

    fn update(&self, f: impl FnOnce(&mut PatchArtist<S>)) -> &Self {
        self.raw.with_mut::<PatchArtist<S>, _>(f);
        self
    }

    fn read<R>(&self, f: impl FnOnce(&PatchArtist<S>) -> R) -> R {
        self.raw.with::<PatchArtist<S>, _>(f)
    }

    /// A copy of the geometry.
    pub fn shape(&self) -> S {
        self.read(|p| p.shape.clone())
    }

    /// Replaces the geometry; a rectangle made from a
    /// [`DataRectangle`] no longer follows changes of axis unit.
    pub fn set_shape(&self, shape: S) -> &Self {
        self.update(|p| {
            p.shape = shape;
            p.unit_source = None;
        })
    }

    /// Modifies the geometry in place; a rectangle made from a
    /// [`DataRectangle`] no longer follows changes of axis unit.
    pub fn update_shape(&self, f: impl FnOnce(&mut S)) -> &Self {
        self.update(|p| {
            f(&mut p.shape);
            p.unit_source = None;
        })
    }

    /// How many straight pieces each segment of the outline is split into
    /// before a non-affine transform such as the polar one bends it
    /// (matplotlib's `get_path()._interpolation_steps`; 1 draws the
    /// segments straight, bars use 100).
    pub fn interpolation_steps(&self) -> usize {
        self.read(|p| p.interpolation_steps)
    }

    /// Sets [`interpolation_steps`](Self::interpolation_steps). matplotlib's
    /// rectangles share one unit path, which polar bars give 100 steps:
    /// after a polar bar chart, a `Rectangle` in polar axes follows the
    /// circle too.
    pub fn set_interpolation_steps(&self, steps: usize) -> &Self {
        self.update(|p| p.interpolation_steps = steps)
    }

    /// A copy of the edge/face/hatch properties.
    pub fn style(&self) -> PatchStyle {
        self.read(|p| p.style.clone())
    }

    /// The edge width in points: the one set, else `patch.linewidth`
    /// (matplotlib's `get_linewidth`).
    pub fn linewidth(&self) -> f64 {
        self.read(|p| p.style.linewidth.unwrap_or(p.rc.patch_linewidth))
    }

    /// The colours the patch is painted with, defaults and alpha applied
    /// (matplotlib's `get_facecolor`, `get_edgecolor` and
    /// `get_hatchcolor`).
    ///
    /// # Errors
    ///
    /// When a colour cannot be resolved.
    pub fn colors(&self) -> Result<PatchColors, ColorError> {
        self.read(|p| {
            let resolved = p.style.resolve(&p.rc, p.base.alpha, S::EDGE_DEFAULT)?;
            Ok(PatchColors {
                face: resolved.face,
                edge: resolved.edge,
                hatch: resolved.hatch_color,
            })
        })
    }

    /// Sets every property `props` sets.
    pub fn update_props(&self, props: PatchProps) -> &Self {
        self.update(|p| props.apply(p))
    }

    /// Whether the patch encloses display point `point` in the figure's
    /// current layout (matplotlib's `contains` with a zero radius).
    pub fn contains_display_point(&self, point: Point) -> bool {
        let figure = self.raw.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        layout
            .store()
            .get(self.raw.id())
            .is_some_and(|patch| patch.contains(&layout, point))
    }

    /// Sets the face colour.
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|p| p.style.facecolor = Some(color))
    }

    /// Sets the edge colour.
    pub fn set_edgecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|p| p.style.edgecolor = Some(color))
    }

    /// Sets both face and edge colour (matplotlib's `set_color`).
    pub fn set_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|p| {
            p.style.facecolor = Some(color.clone());
            p.style.edgecolor = Some(color);
        })
    }

    /// Fills the face or leaves it empty.
    pub fn set_fill(&self, fill: bool) -> &Self {
        self.update(|p| p.style.fill = fill)
    }

    /// Sets the edge width in points.
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.update(|p| p.style.linewidth = Some(width))
    }

    /// Sets the edge line style.
    pub fn set_linestyle(&self, style: LineStyle) -> &Self {
        self.update(|p| p.style.linestyle = style)
    }

    /// Sets the colour drawn in the gaps of a dashed edge (matplotlib's
    /// `set_edgegapcolor`; `"none"` leaves them empty).
    pub fn set_edgegapcolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|p| p.style.edgegapcolor = color)
    }

    /// Sets the hatch pattern (e.g. `"//"`, `"x"`, `"o."`).
    pub fn set_hatch(&self, pattern: impl Into<String>) -> &Self {
        let pattern = pattern.into();
        self.update(|p| p.style.hatch = Some(pattern))
    }

    /// Sets the hatch colour.
    pub fn set_hatchcolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|p| p.style.hatchcolor = ColorOrEdge::Color(color))
    }

    /// Sets the hatch line width in points (matplotlib's
    /// `set_hatch_linewidth`).
    pub fn set_hatch_linewidth(&self, width: f64) -> &Self {
        self.update(|p| p.style.hatch_linewidth = width)
    }

    /// Sets the edge cap style.
    pub fn set_capstyle(&self, cap: CapStyle) -> &Self {
        self.update(|p| p.style.capstyle = cap)
    }

    /// Sets the edge join style.
    pub fn set_joinstyle(&self, join: JoinStyle) -> &Self {
        self.update(|p| p.style.joinstyle = join)
    }

    /// Enables or disables anti-aliasing.
    pub fn set_antialiased(&self, antialiased: bool) -> &Self {
        self.update(|p| p.style.antialiased = Some(antialiased))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::patches::Rectangle;

    #[test]
    fn linewidths_fall_back_on_the_rc_default_and_handles_downcast() {
        let fig = Figure::new();
        let rect = fig.add_patch(Rectangle::new((0.0, 0.0), 0.5, 0.5));
        assert_eq!(rect.linewidth(), crate::rc::params().patch_linewidth);
        rect.set_linewidth(3.0);
        assert_eq!(rect.linewidth(), 3.0);
        assert_eq!(
            rect.raw().downcast::<Patch<Rectangle>>(),
            Some(rect.clone())
        );
        assert_eq!(rect.raw().downcast::<Patch<crate::patches::Circle>>(), None);
    }

    #[test]
    fn colors_apply_the_default_rules() {
        let fig = Figure::new();
        let rect = fig.add_patch(Rectangle::new((0.0, 0.0), 0.5, 0.5));
        rect.set_hatch("//").set_facecolor("none");
        let colors = rect.colors().unwrap();
        assert_eq!(colors.face, Rgba::TRANSPARENT);
        assert_eq!(colors.edge.a, 0.0, "no edge without force_edgecolor");
        assert_eq!(colors.hatch, Rgba::BLACK, "patch.edgecolor stands in");
        rect.set_edgecolor("red");
        assert_eq!(rect.colors().unwrap().hatch, Rgba::new(1.0, 0.0, 0.0, 1.0));
        rect.set_hatchcolor("blue");
        assert_eq!(rect.colors().unwrap().hatch, Rgba::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn interpolation_steps_bend_rectangles_on_polar_axes() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.polar_subplot();
        ax.set_rmax(1.0);
        let disk = ax.add_patch(Rectangle::new((0.0, 0.0), 2.0 * std::f64::consts::PI, 1.0));
        let data = fig.resolve_transform(&ax.trans_data()).unwrap();
        let inside = data.transform_point(Point::new(std::f64::consts::PI, 0.5));
        assert_eq!(disk.interpolation_steps(), 1);
        assert!(
            !disk.contains_display_point(inside),
            "straight segments between the corners enclose nothing"
        );
        disk.set_interpolation_steps(100);
        assert!(
            disk.contains_display_point(inside),
            "the edges follow the circle"
        );
    }

    #[test]
    fn hit_tests_take_the_radius_the_picker_implies() {
        let fig = Figure::with_size(2.0, 2.0, 100.0);
        let rect = fig.add_patch(Rectangle::new((0.25, 0.25), 0.5, 0.5));
        // The rectangle spans pixels 50 to 150; a radius r widens it by
        // r / 2, as a stroke r wide would.
        let near = Point::new(150.4, 100.0);
        assert_eq!(rect.contains(near), None, "no edge, no tolerance");
        rect.set_picker(Some(Picker::Default));
        assert_eq!(rect.contains(near), Some(Vec::new()), "picker=True is 1 px");
        rect.set_picker(None).set_edgecolor("black");
        rect.set_linewidth(2.0);
        assert!(rect.contains(Point::new(150.9, 100.0)).is_some());
        assert_eq!(rect.contains(Point::new(151.1, 100.0)), None);
    }
}
