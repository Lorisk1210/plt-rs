//! Annotations: texts pointing at a point, optionally with an arrow
//! (matplotlib's `Annotation`).
//!
//! An annotation is a text whose position is given in its own coordinate
//! system ([`AnnotateOptions::textcoords`]), possibly relative to the
//! annotated point, and which may draw an arrow from its box to that
//! point. Its handle [`Annotation`] dereferences to [`Text`] for the text
//! properties.

mod arrow;
mod drag;
pub(crate) use drag::reconnect_drag;

use std::ops::Deref;

use pltrs_core::{Bbox, Point};

pub(crate) use arrow::{AnnotationArrow, ArrowTarget, StartRegion};
pub use arrow::{ArrowProps, ArrowStart, FancyArrowProps, LegacyArrowProps};

use super::{BoxPlacement, Text, TextArtist, TextDrawError};
use crate::artist::{Artist, ArtistHandle, DrawContext, LayoutContext, RawHandle, TypedHandle};
use crate::coords::{CoordError, CoordFrame, Coords, annotation_shown};
use crate::events::Dragger;
use crate::patches::{ClipRegion, DisplayPath, FancyArrowPatch, Patch};
use crate::units::{DataPoint, UnitSources};

/// The options of [`Axes::annotate`](crate::axes::Axes::annotate)
/// (matplotlib's keyword arguments besides the text properties).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnnotateOptions {
    /// Where the text goes, in `textcoords`: numbers, or plotting values
    /// converted on their axis where they are data coordinates; `None`
    /// puts it at the annotated point.
    pub xytext: Option<DataPoint>,
    /// The coordinates of the annotated point (not an offset).
    pub xycoords: Coords,
    /// The coordinates of the text; `None` uses `xycoords`.
    pub textcoords: Option<Coords>,
    /// The arrow from the text to the point, if any.
    pub arrowprops: Option<ArrowProps>,
    /// Whether to hide the annotation when the point leaves the axes:
    /// always (`Some(true)`), never (`Some(false)`), or for points in data
    /// coordinates (`None`).
    pub annotation_clip: Option<bool>,
    /// Clips the text to the axes (matplotlib's `clip_on=True` passed to
    /// `annotate`, which clips to the axes patch; the arrow is not
    /// clipped).
    pub clip_on: bool,
}

/// What makes a text an annotation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnnotationState {
    pub(crate) xy: Point,
    /// The user values the annotated point came from, which it converts
    /// again when an axis changes unit (matplotlib's `Annotation.xy`
    /// converted when drawn); moving the point forgets them.
    pub(crate) unit_sources: UnitSources,
    pub(crate) xycoords: Coords,
    pub(crate) textcoords: Coords,
    pub(crate) arrow: Option<AnnotationArrow>,
    pub(crate) annotation_clip: Option<bool>,
    /// The callbacks dragging the annotation, while it is draggable.
    pub(crate) dragger: Option<Dragger>,
}

pltrs_core::archive_struct!(AnnotationState {
    xy,
    unit_sources,
    xycoords,
    textcoords,
    arrow,
    annotation_clip,
    dragger
});

impl AnnotationState {
    /// The annotation state `options` describe; fails when the annotated
    /// point is given as an offset.
    pub(crate) fn new(
        xy: Point,
        options: AnnotateOptions,
        rc: &crate::rc::RcParams,
    ) -> Result<Self, CoordError> {
        if options.xycoords.is_offset() {
            return Err(CoordError::NoOffset);
        }
        Ok(Self {
            xy,
            unit_sources: UnitSources::default(),
            textcoords: options
                .textcoords
                .unwrap_or_else(|| options.xycoords.clone()),
            xycoords: options.xycoords,
            arrow: options
                .arrowprops
                .map(|props| AnnotationArrow::new(props, rc)),
            annotation_clip: options.annotation_clip,
            dragger: None,
        })
    }

    /// The axes the annotation belongs to, if its parent is one.
    fn axes(text: &TextArtist, layout: LayoutContext<'_>) -> Option<crate::artist::ArtistId> {
        text.base()
            .parent
            .filter(|&parent| layout.axes_extent(parent).is_some())
    }

    fn frame<'a>(
        text: &TextArtist,
        layout: LayoutContext<'a>,
        offset_origin: Option<Point>,
    ) -> CoordFrame<'a> {
        CoordFrame {
            layout,
            axes: Self::axes(text, layout),
            // An annotation in no container, only given its figure, is
            // placed in that figure (matplotlib's `get_figure(root=False)`).
            owner: Some(text.base().parent.unwrap_or_else(|| layout.root())),
            offset_origin,
            fontsize: text.font.size,
        }
    }

    /// The annotated point in display pixels (`_get_position_xy`).
    fn point(&self, text: &TextArtist, layout: LayoutContext<'_>) -> Result<Point, CoordError> {
        Ok(self
            .xycoords
            .resolve(&Self::frame(text, layout, None))?
            .transform_point(self.xy))
    }

    /// The text anchor in display pixels.
    fn anchor(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
        point: Point,
    ) -> Result<Point, CoordError> {
        Ok(self
            .textcoords
            .resolve(&Self::frame(text, layout, Some(point)))?
            .transform_point(text.position))
    }

    /// The display position of the text (matplotlib's
    /// `get_transform().transform(xyann)`).
    pub(crate) fn text_display(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
    ) -> Result<Point, CoordError> {
        let point = self.point(text, layout)?;
        self.anchor(text, layout, point)
    }

    /// The text position, in the text coordinates, that puts the text at
    /// display position `at`; `None` when the coordinates cannot be
    /// inverted.
    pub(crate) fn text_position_at(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
        at: Point,
    ) -> Option<Point> {
        let point = self.point(text, layout).ok()?;
        let transform = self
            .textcoords
            .resolve(&Self::frame(text, layout, Some(point)))
            .ok()?;
        Some(transform.inverted()?.transform_point(at))
    }

    /// Whether the annotation is drawn (`_check_xy`); not when its point
    /// cannot be placed.
    pub(crate) fn is_shown(&self, text: &TextArtist, layout: LayoutContext<'_>) -> bool {
        self.point(text, layout)
            .is_ok_and(|point| self.shown(text, layout, point))
    }

    /// Whether the annotation is drawn with its point at `point`
    /// (`_check_xy`).
    fn shown(&self, text: &TextArtist, layout: LayoutContext<'_>, point: Point) -> bool {
        annotation_shown(
            layout,
            self.annotation_clip,
            &self.xycoords,
            Self::axes(text, layout),
            point,
        )
    }

    /// Where the arrow is aimed, given the text's extent.
    fn target(
        text: &TextArtist,
        layout: LayoutContext<'_>,
        point: Point,
        anchor: Point,
    ) -> Result<ArrowTarget, TextDrawError> {
        let text_bbox = text.extent_at(layout, anchor)?;
        let start_region = match text.box_placement(layout, anchor)? {
            Some((bbox, placement)) => {
                let region = |placement| {
                    bbox.clip_region(&text.rc, placement)
                        .map_err(TextDrawError::Box)
                };
                StartRegion::TextBox {
                    now: region(&placement)?,
                    initial: region(&BoxPlacement::before_drawn())?,
                }
            }
            None if text.text.is_empty() => StartRegion::Fresh(None),
            None => {
                let pad = layout.points_to_pixels(4.0);
                let padded = text_bbox.padded(pad / 2.0, pad / 2.0);
                let radius = if text.rc.patch_force_edgecolor {
                    text.rc.patch_linewidth
                } else {
                    0.0
                };
                StartRegion::Fresh(Some(ClipRegion {
                    path: pltrs_core::Path::unit_rectangle()
                        .transformed(&crate::transforms::unit_to_bbox(&padded)),
                    radius,
                }))
            }
        };
        Ok(ArrowTarget {
            owner: text.base().id(),
            text_bbox,
            point,
            fontsize: text.font.size,
            mutation_unit: 1.0,
            start_region,
        })
    }

    /// Draws the arrow, then the text (`Annotation.draw`).
    pub(crate) fn draw(
        &self,
        text: &TextArtist,
        ctx: &mut DrawContext<'_>,
    ) -> Result<(), TextDrawError> {
        let layout = *ctx.layout();
        let point = self.point(text, layout)?;
        if !self.shown(text, layout, point) {
            return Ok(());
        }
        let anchor = self.anchor(text, layout, point)?;
        if let Some(arrow) = &self.arrow {
            let target = Self::target(text, layout, point, anchor)?;
            arrow
                .draw(ctx, &text.rc, target)
                .map_err(TextDrawError::Box)?;
        }
        text.draw_at(ctx, anchor)
    }

    /// Whether display point `at` hits the text or the arrow of a shown
    /// annotation (`Annotation.contains`).
    pub(crate) fn contains(&self, text: &TextArtist, layout: LayoutContext<'_>, at: Point) -> bool {
        let Ok(point) = self.point(text, layout) else {
            return false;
        };
        if !self.shown(text, layout, point) {
            return false;
        }
        let Ok(anchor) = self.anchor(text, layout, point) else {
            return false;
        };
        text.contains_at(layout, anchor, at)
            || self.arrow.as_ref().is_some_and(|arrow| {
                Self::target(text, layout, point, anchor)
                    .is_ok_and(|target| arrow.contains(layout, &text.rc, target, at))
            })
    }

    /// The extent of the text alone, where the annotation places it
    /// (`Text.get_window_extent` of an annotation).
    fn text_extent(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
    ) -> Result<Bbox, TextDrawError> {
        let point = self.point(text, layout)?;
        let anchor = self.anchor(text, layout, point)?;
        text.extent_at(layout, anchor)
    }

    /// The outline of the arrow as drawn, in display pixels, `None`
    /// without an arrow or while the annotation is hidden.
    fn arrow_display_path(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
    ) -> Result<Option<pltrs_core::Path>, TextDrawError> {
        let Some(arrow) = &self.arrow else {
            return Ok(None);
        };
        let point = self.point(text, layout)?;
        if !self.shown(text, layout, point) {
            return Ok(None);
        }
        let anchor = self.anchor(text, layout, point)?;
        let target = Self::target(text, layout, point, anchor)?;
        let paths = arrow
            .measured_paths(layout, &text.rc, target)
            .map_err(TextDrawError::Box)?;
        let displayed: Vec<pltrs_core::Path> = paths.iter().map(DisplayPath::to_display).collect();
        Ok(Some(pltrs_core::Path::compound(&displayed)))
    }

    /// The extent of the arrow as drawn, `None` without an arrow or while
    /// the annotation is hidden.
    fn arrow_extent(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
    ) -> Result<Option<Bbox>, TextDrawError> {
        Ok(self
            .arrow_display_path(text, layout)?
            .map(|path| path.extents()))
    }

    /// The extent of the text and arrow (`Annotation.get_window_extent`;
    /// the unit box when the annotation is hidden).
    pub(crate) fn extent(
        &self,
        text: &TextArtist,
        layout: LayoutContext<'_>,
    ) -> Result<Bbox, TextDrawError> {
        let point = self.point(text, layout)?;
        if !self.shown(text, layout, point) {
            return Ok(Bbox::UNIT);
        }
        let anchor = self.anchor(text, layout, point)?;
        let text_bbox = text.extent_at(layout, anchor)?;
        let Some(arrow) = &self.arrow else {
            return Ok(text_bbox);
        };
        let target = Self::target(text, layout, point, anchor)?;
        let paths = arrow
            .measured_paths(layout, &text.rc, target)
            .map_err(TextDrawError::Box)?;
        let displayed: Vec<pltrs_core::Path> = paths.iter().map(DisplayPath::to_display).collect();
        Ok(text_bbox.union(&pltrs_core::Path::compound(&displayed).extents()))
    }
}

/// A handle to an annotation (matplotlib's `Annotation`); it dereferences
/// to its [`Text`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    text: Text,
}

impl Annotation {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self {
            text: Text::from_raw(raw),
        }
    }

    fn state<R>(&self, f: impl FnOnce(&AnnotationState) -> R) -> R {
        self.text.raw().with::<TextArtist, _>(|t| {
            f(t.annotation
                .as_deref()
                .expect("annotation handles refer to annotations"))
        })
    }

    fn update(&self, f: impl FnOnce(&mut AnnotationState)) -> &Self {
        self.text.raw().with_mut::<TextArtist, _>(|t| {
            f(t.annotation
                .as_deref_mut()
                .expect("annotation handles refer to annotations"));
        });
        self
    }

    /// The annotated point.
    pub fn xy(&self) -> Point {
        self.state(|a| a.xy)
    }

    /// Moves the annotated point.
    pub fn set_xy(&self, xy: impl Into<Point>) -> &Self {
        let xy = xy.into();
        self.update(|a| {
            a.xy = xy;
            a.unit_sources = UnitSources::default();
        })
    }

    /// The coordinates of the text position (matplotlib's
    /// `get_anncoords`).
    pub fn textcoords(&self) -> Coords {
        self.state(|a| a.textcoords.clone())
    }

    /// Gives the text position in other coordinates, keeping its numbers
    /// (matplotlib's `set_anncoords`).
    pub fn set_textcoords(&self, coords: Coords) -> &Self {
        self.update(|a| a.textcoords = coords)
    }

    /// The coordinates of the annotated point (matplotlib's `xycoords`).
    pub fn xycoords(&self) -> Coords {
        self.state(|a| a.xycoords.clone())
    }

    /// Gives the annotated point in other coordinates, keeping its numbers
    /// (matplotlib's `xycoords` setter).
    ///
    /// # Errors
    ///
    /// When `coords` is an offset, which the annotated point cannot be
    /// given in.
    pub fn set_xycoords(&self, coords: Coords) -> Result<&Self, CoordError> {
        if coords.is_offset() {
            return Err(CoordError::NoOffset);
        }
        Ok(self.update(|a| a.xycoords = coords))
    }

    /// Sets when the annotation hides with its point outside the axes.
    pub fn set_annotation_clip(&self, clip: Option<bool>) -> &Self {
        self.update(|a| a.annotation_clip = clip)
    }

    /// When the annotation hides with its point outside the axes: always,
    /// never, or for points in data coordinates (`None`; matplotlib's
    /// `get_annotation_clip`).
    pub fn annotation_clip(&self) -> Option<bool> {
        self.state(|a| a.annotation_clip)
    }

    /// Whether the annotation draws an arrow to its point, which it does
    /// when made with `arrowprops`, default ones included (matplotlib's
    /// `arrow_patch is not None`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::text::{AnnotateOptions, ArrowProps, LegacyArrowProps};
    /// let ax = Figure::new().subplot();
    /// let plain = ax.annotate("foo", (0.0, 1.0), AnnotateOptions::default())?;
    /// assert!(!plain.has_arrow());
    /// let arrowprops = Some(ArrowProps::Legacy(LegacyArrowProps::default()));
    /// let arrowed = ax.annotate("foo", (0.0, 1.0), AnnotateOptions { arrowprops, ..Default::default() })?;
    /// assert!(arrowed.has_arrow());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn has_arrow(&self) -> bool {
        self.state(|a| a.arrow.is_some())
    }

    /// The arrow as a loose fancy arrow patch of its look, `None` without
    /// an arrow: what a program reads and changes of the arrow as the
    /// patch matplotlib keeps as `arrow_patch`, the annotation placing it
    /// when drawn. Changes come back with [`Annotation::restyle_arrow`];
    /// [`Annotation::arrow_extent`] is where the arrow is drawn.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::text::{AnnotateOptions, ArrowProps, LegacyArrowProps};
    /// let ax = Figure::new().subplot();
    /// let arrowprops = Some(ArrowProps::Legacy(LegacyArrowProps::default()));
    /// let options = AnnotateOptions { arrowprops, ..Default::default() };
    /// let annotation = ax.annotate("foo", (0.0, 1.0), options)?;
    /// let arrow = annotation.arrow_patch().expect("an arrow");
    /// arrow.set_alpha(Some(0.5));
    /// annotation.restyle_arrow(&arrow);
    /// assert_eq!(annotation.arrow_patch().map(|a| a.alpha()), Some(Some(0.5)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn arrow_patch(&self) -> Option<Patch<FancyArrowPatch>> {
        self.state(|a| a.arrow.as_ref().map(AnnotationArrow::to_patch))
    }

    /// Gives the arrow the look of `patch` (see
    /// [`Annotation::arrow_patch`]); without an arrow, nothing changes.
    pub fn restyle_arrow(&self, patch: &Patch<FancyArrowPatch>) -> &Self {
        self.update(|a| {
            if let Some(arrow) = &mut a.arrow {
                arrow.restyle_from(patch);
            }
        })
    }

    /// The display box of the arrow in the figure's current layout (laid
    /// out first), `None` without an arrow or while the annotation is
    /// hidden (matplotlib's `arrow_patch.get_window_extent()`).
    pub fn arrow_extent(&self) -> Option<Bbox> {
        self.laid_out(|annotation, text, layout| {
            annotation.arrow_extent(text, layout).ok().flatten()
        })
    }

    /// The outline of the arrow in display pixels in the figure's current
    /// layout (laid out first), `None` without an arrow, while the
    /// annotation is hidden or when the arrow cannot be placed
    /// (matplotlib's `arrow_patch.get_path()` once the annotation placed
    /// its arrow).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::text::{AnnotateOptions, ArrowProps, LegacyArrowProps};
    /// let ax = Figure::new().subplot();
    /// let options = AnnotateOptions {
    ///     xytext: Some((0.2, 0.2).into()),
    ///     arrowprops: Some(ArrowProps::Legacy(LegacyArrowProps::default())),
    ///     ..AnnotateOptions::default()
    /// };
    /// let annotation = ax.annotate("x", (0.5, 0.5), options)?;
    /// assert!(annotation.arrow_display_path().is_some_and(|path| !path.is_empty()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn arrow_display_path(&self) -> Option<pltrs_core::Path> {
        self.laid_out(|annotation, text, layout| {
            annotation.arrow_display_path(text, layout).ok().flatten()
        })
    }

    /// The display box of the text alone, without the arrow, in the
    /// figure's current layout (laid out first; matplotlib's
    /// `Text.get_window_extent(annotation)`).
    pub fn text_extent(&self) -> Option<Bbox> {
        self.laid_out(|annotation, text, layout| annotation.text_extent(text, layout).ok())
    }

    /// `f` of the annotation's state and text in the figure's current
    /// layout, laid out first; `None` once the annotation is removed.
    fn laid_out<R>(
        &self,
        f: impl FnOnce(&AnnotationState, &TextArtist, LayoutContext<'_>) -> Option<R>,
    ) -> Option<R> {
        let raw = self.text.raw();
        let state = raw.figure_state().prepared();
        let layout = LayoutContext::new(&state);
        let text = state.store.get_as::<TextArtist>(raw.id())?;
        f(text.annotation.as_deref()?, text, layout)
    }
}

impl Deref for Annotation {
    type Target = Text;

    fn deref(&self) -> &Text {
        &self.text
    }
}

impl ArtistHandle for Annotation {
    fn raw(&self) -> &RawHandle {
        self.text.raw()
    }
}

/// A text handle is an annotation's when the text was made as one
/// (`Axes::annotate`).
impl TypedHandle for Annotation {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        (raw.is::<TextArtist>() && raw.with::<TextArtist, _>(|t| t.annotation.is_some()))
            .then(|| Self::from_raw(raw.clone()))
    }
}

#[cfg(test)]
mod tests;
