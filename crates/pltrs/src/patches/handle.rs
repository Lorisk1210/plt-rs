//! The typed handle to a patch ([`Patch`]) and the colours it is painted
//! with.

use std::marker::PhantomData;

use pltrs_core::{Path, Point, Rgba};
use pltrs_render::{CapStyle, JoinStyle};

use super::shape::PatchShape;
use super::style::PatchStyle;
use super::{DisplayPath, PatchArtist, PatchError, PatchProps};
use crate::artist::{ArtistHandle, LayoutContext, RawHandle, TypedHandle};
use crate::colors::{Color, ColorError};
use crate::lines::LineStyle;
use crate::rc::ColorOrEdge;

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

pltrs_core::archive_struct!(Patch<S> { raw, shape });

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
    /// [`DataRectangle`](super::DataRectangle) no longer follows changes
    /// of axis unit.
    pub fn set_shape(&self, shape: S) -> &Self {
        self.update(|p| {
            p.shape = shape;
            p.unit_source = None;
        })
    }

    /// Changes the geometry with `f`, which works on a copy put back once
    /// it returns, so `f` may use any handle of the figure; a rectangle
    /// made from a [`DataRectangle`](super::DataRectangle) no longer
    /// follows changes of axis unit.
    pub fn update_shape(&self, f: impl FnOnce(&mut S)) -> &Self {
        let mut shape = self.shape();
        f(&mut shape);
        self.set_shape(shape)
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

    /// Changes the edge/face/hatch properties with `f`; a field set back
    /// to `None` takes its default again (matplotlib's setters given
    /// `None`), which the edge colour re-derives from the fill.
    ///
    /// ```
    /// use pltrs::patches::Rectangle;
    /// use pltrs::prelude::*;
    /// let patch = Figure::new().subplot().add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    /// patch.set_edgecolor("red");
    /// patch.update_style(|style| style.edgecolor = None);
    /// assert_eq!(patch.style().edgecolor, None);
    /// ```
    pub fn update_style(&self, f: impl FnOnce(&mut PatchStyle)) -> &Self {
        self.update(|p| f(&mut p.style))
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

    /// The outlines as the patch is drawn now, in display pixels, an
    /// arrow sized at the pixels per point of its last draw (one before
    /// its first): matplotlib's `get_path` of patches placed as they are
    /// drawn, such as `FancyArrowPatch._get_path_in_displaycoord`, which
    /// reads the `_dpi_cor` only drawing sets.
    ///
    /// ```
    /// use pltrs::patches::{ConnectionStyle, FancyArrowPatch, PatchError};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let arrow = ax.add_patch(FancyArrowPatch::between((0.2, 0.2), (0.5, 0.5)));
    /// assert!(!arrow.display_paths()?.is_empty());
    /// let parallel = ConnectionStyle::Angle3 { angle_a: 20.0, angle_b: 200.0 };
    /// let crooked = ax.add_patch(
    ///     FancyArrowPatch::between((0.2, 0.2), (0.5, 0.5)).with_connectionstyle(parallel),
    /// );
    /// assert_eq!(crooked.display_paths(), Err(PatchError::ParallelAngles));
    /// # Ok::<(), PatchError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the patch cannot be placed: it is in no figure, or its shape
    /// cannot be built there (a connection between parallel angles).
    pub fn display_paths(&self) -> Result<Vec<Path>, PatchError> {
        let mut state = self.raw.figure_state().borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        let patch = layout
            .store()
            .get_as::<PatchArtist<S>>(self.raw.id())
            .ok_or(PatchError::Detached)?;
        let paths = patch.parts().measured_display_paths(layout)?;
        Ok(paths.iter().map(DisplayPath::to_display).collect())
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
    use crate::artist::Picker;
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
