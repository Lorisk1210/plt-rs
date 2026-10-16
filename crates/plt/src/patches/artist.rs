//! The generic patch artist and its typed handle.

use std::marker::PhantomData;
use std::rc::Rc;

use plt_core::{Bbox, Point};
use plt_render::{CapStyle, ClipPath, JoinStyle};

use super::shapes::PatchShape;
use super::style::{PatchStyle, draw_patch_paths};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::rc::{ColorOrEdge, RcParams};
use crate::transforms::{ResolvedTransform, Transform};

/// A shape painted with a [`PatchStyle`] (matplotlib's `Patch` subclasses).
#[derive(Debug)]
pub(crate) struct PatchArtist<S: PatchShape> {
    base: ArtistBase,
    pub(crate) style: PatchStyle,
    pub(crate) shape: S,
    rc: Rc<RcParams>,
}

impl<S: PatchShape> PatchArtist<S> {
    /// A patch with matplotlib's defaults under `rc` (z-order 1).
    pub(crate) fn new(shape: S, rc: Rc<RcParams>) -> Self {
        Self {
            base: ArtistBase::with_zorder(1.0),
            style: PatchStyle::from_rc(&rc),
            shape,
            rc,
        }
    }

    fn transform(&self) -> Transform {
        let shape = Transform::Affine(self.shape.patch_transform());
        shape.then(self.base.transform.clone().unwrap_or(Transform::Display))
    }

    fn resolved(&self, ctx: &LayoutContext<'_>) -> Option<ResolvedTransform> {
        ctx.resolve(&self.transform())
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
        let Some(resolved) = self.resolved(ctx.layout()) else {
            ctx.report(self.base.id(), "its coordinate system no longer exists");
            return;
        };
        let outline = self.shape.path();
        let (path, affine) = resolved.split_path(&outline);
        let paths = [(path.into_owned(), affine)];
        if let Err(err) = draw_patch_paths(
            ctx,
            &self.base,
            &self.style,
            &self.rc,
            S::EDGE_DEFAULT,
            &paths,
        ) {
            ctx.report(self.base.id(), err);
        }
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        Some(
            self.resolved(ctx)?
                .transform_path(&self.shape.path())
                .extents(),
        )
    }

    fn outline(&self, ctx: &LayoutContext<'_>) -> Option<ClipPath> {
        let outline = self.shape.path();
        let (path, transform) = self.resolved(ctx)?.split_path(&outline);
        Some(ClipPath {
            path: path.into_owned(),
            transform,
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
