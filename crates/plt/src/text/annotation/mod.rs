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

use std::ops::Deref;

use plt_core::{Bbox, Point};

pub(crate) use arrow::{AnnotationArrow, ArrowTarget};
pub use arrow::{ArrowProps, FancyArrowProps, LegacyArrowProps};

use super::{Text, TextArtist, TextDrawError};
use crate::artist::{Artist, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::coords::{CoordError, CoordFrame, Coords, annotation_shown};
use crate::events::Dragger;
use crate::patches::{ClipRegion, DisplayPath};

/// The options of [`Axes::annotate`](crate::axes::Axes::annotate)
/// (matplotlib's keyword arguments besides the text properties).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnnotateOptions {
    /// Where the text goes; `None` puts it at the annotated point.
    pub xytext: Option<Point>,
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
}

/// What makes a text an annotation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnnotationState {
    pub(crate) xy: Point,
    pub(crate) xycoords: Coords,
    pub(crate) textcoords: Coords,
    pub(crate) arrow: Option<AnnotationArrow>,
    pub(crate) annotation_clip: Option<bool>,
    /// The callbacks dragging the annotation, while it is draggable.
    pub(crate) dragger: Option<Dragger>,
}

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
            owner: text.base().parent,
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

    /// Whether the annotation is drawn (`_check_xy`).
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
            Some((bbox, placement)) => Some(
                bbox.clip_region(&text.rc, &placement)
                    .map_err(TextDrawError::Box)?,
            ),
            None if text.text.is_empty() => None,
            None => {
                let pad = layout.points_to_pixels(4.0);
                let padded = text_bbox.padded(pad / 2.0, pad / 2.0);
                let radius = if text.rc.patch_force_edgecolor {
                    text.rc.patch_linewidth
                } else {
                    0.0
                };
                Some(ClipRegion {
                    path: plt_core::Path::unit_rectangle()
                        .transformed(&crate::transforms::unit_to_bbox(&padded)),
                    radius,
                })
            }
        };
        Ok(ArrowTarget {
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
            .display_paths(layout, &text.rc, target)
            .map_err(TextDrawError::Box)?;
        let displayed: Vec<plt_core::Path> = paths.iter().map(DisplayPath::to_display).collect();
        Ok(text_bbox.union(&plt_core::Path::compound(&displayed).extents()))
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
        self.update(|a| a.xy = xy)
    }

    /// The coordinates of the text position.
    pub fn textcoords(&self) -> Coords {
        self.state(|a| a.textcoords.clone())
    }

    /// Sets when the annotation hides with its point outside the axes.
    pub fn set_annotation_clip(&self, clip: Option<bool>) -> &Self {
        self.update(|a| a.annotation_clip = clip)
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

#[cfg(test)]
mod tests;
