//! The generic patch artist and its typed handle.

use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

use plt_core::{Affine2D, Bbox, Path, Point};
use plt_render::{CapStyle, ClipPath, JoinStyle};

use super::shape::{DisplayPath, PatchShape, ShapeFrame};
use super::style::{PatchStyle, paint_paths};
use super::{PatchError, PatchProps};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::rc::{ColorOrEdge, RcParams};
use crate::transforms::Transform;

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
}

impl<S: PatchShape> PatchParts<'_, S> {
    /// The outlines in display space.
    pub(crate) fn display_paths(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        let transform = self.base.transform.clone().unwrap_or(Transform::Display);
        let frame = ShapeFrame {
            layout,
            transform: layout.resolve(&transform).ok_or(PatchError::Detached)?,
            linewidth: self.style.linewidth.unwrap_or(self.rc.patch_linewidth),
            parent: self.base.parent,
            interpolation_steps: self.interpolation_steps,
        };
        self.shape.display_paths(&frame)
    }

    /// Draws the patch (matplotlib's `Patch.draw`).
    pub(crate) fn paint(&self, ctx: &mut DrawContext<'_>) -> Result<(), PatchError> {
        let paths = self.display_paths(*ctx.layout())?;
        paint_paths(ctx, self.base, self.style, self.rc, S::EDGE_DEFAULT, &paths)?;
        Ok(())
    }

    /// The extent of the outlines in display space.
    pub(crate) fn extent(&self, layout: LayoutContext<'_>) -> Option<Bbox> {
        self.display_paths(layout)
            .ok()?
            .iter()
            .map(|path| path.to_display().extents())
            .reduce(|a, b| a.union(&b))
    }

    /// The outline as a clip region.
    pub(crate) fn outline(&self, layout: LayoutContext<'_>) -> Option<ClipPath> {
        let mut paths = self.display_paths(layout).ok()?;
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
    rc: Rc<RcParams>,
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
            rc,
        }
    }

    fn parts(&self) -> PatchParts<'_, S> {
        PatchParts {
            base: &self.base,
            style: &self.style,
            rc: &self.rc,
            shape: &self.shape,
            interpolation_steps: self.interpolation_steps,
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

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.parts().extent(*ctx)
    }

    fn outline(&self, ctx: &LayoutContext<'_>) -> Option<ClipPath> {
        self.parts().outline(*ctx)
    }

    /// Whether the outline encloses `point` (matplotlib's
    /// `Patch.contains` with a zero radius).
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        self.outline(ctx)
            .is_some_and(|clip| clip.path.contains_point(point, Some(&clip.transform), 0.0))
    }

    fn as_patch(&self) -> Option<PatchView<'_>> {
        Some(PatchView {
            style: &self.style,
            edge_default: S::EDGE_DEFAULT,
            shape: &self.shape,
        })
    }
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

    /// Replaces the geometry.
    pub fn set_shape(&self, shape: S) -> &Self {
        self.update(|p| p.shape = shape)
    }

    /// Modifies the geometry in place.
    pub fn update_shape(&self, f: impl FnOnce(&mut S)) -> &Self {
        self.update(|p| f(&mut p.shape))
    }

    /// A copy of the edge/face/hatch properties.
    pub fn style(&self) -> PatchStyle {
        self.read(|p| p.style.clone())
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
