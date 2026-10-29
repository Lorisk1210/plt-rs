//! The leaves of an offset-box tree and the padded box around one child
//! (matplotlib's `DrawingArea`, `TextArea`, `AuxTransformBox`,
//! `OffsetImage` and `PaddedBox`).
//!
//! Every box measures its extent relative to its own origin, in pixels;
//! sizes given in points are scaled by the canvas resolution.

use pltrs_core::{Affine2D, Bbox, Path, Point};
use pltrs_text::HAlign;

use super::{ArtistRef, OffsetBox, OffsetBoxError, Placement};
use crate::artist::LayoutContext;
use crate::image::BboxImageArtist;
use crate::text::TextArtist;
use crate::transforms::Transform;

/// An area of fixed size whose artists are positioned relative to its
/// origin, in points (matplotlib's `DrawingArea`).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingArea<A> {
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
    /// How far the area extends left of its origin, in points.
    pub xdescent: f64,
    /// How far the area extends below its origin, in points.
    pub ydescent: f64,
    /// Whether the artists are clipped to the area (matplotlib's `clip`).
    pub clip: bool,
    /// The artists drawn in the area, in drawing order.
    pub artists: Vec<A>,
    /// Whether the area takes part in its parent's layout.
    pub visible: bool,
}

impl<A> DrawingArea<A> {
    /// A visible, unclipped area of the given size in points, with no
    /// artists yet.
    pub fn new(width: f64, height: f64, xdescent: f64, ydescent: f64) -> Self {
        Self {
            width,
            height,
            xdescent,
            ydescent,
            clip: false,
            artists: Vec::new(),
            visible: true,
        }
    }

    /// The same area with `artist` drawn last (matplotlib's `add_artist`).
    #[must_use]
    pub fn with_artist(mut self, artist: A) -> Self {
        self.artists.push(artist);
        self
    }

    /// matplotlib's `DrawingArea.get_bbox`.
    pub(super) fn bbox(&self, ctx: LayoutContext<'_>) -> Bbox {
        let scale = ctx.points_to_pixels(1.0);
        Bbox::from_bounds(
            -self.xdescent * scale,
            -self.ydescent * scale,
            self.width * scale,
            self.height * scale,
        )
    }

    /// The artists placed with the area's origin at `offset`: points map
    /// to pixels from there, and a clipped area clips them to its
    /// rectangle (matplotlib's `DrawingArea.draw`).
    pub(super) fn placement(&self, ctx: LayoutContext<'_>, offset: Point) -> Placement<A>
    where
        A: Clone,
    {
        let scale = ctx.points_to_pixels(1.0);
        let clip = self.clip.then(|| {
            Path::polyline(vec![
                Point::new(0.0, 0.0),
                Point::new(0.0, self.height),
                Point::new(self.width, self.height),
                Point::new(self.width, 0.0),
            ])
        });
        Placement::Artists {
            artists: self.artists.clone(),
            transform: Transform::Affine(
                Affine2D::scaling(scale, scale).translate(offset.x, offset.y),
            ),
            clip,
        }
    }
}

/// A text whose origin is the start of its (last) baseline (matplotlib's
/// `TextArea`). The text artist must be left, centre or right aligned
/// horizontally and baseline aligned vertically.
#[derive(Clone, Debug, PartialEq)]
pub struct TextArea<A> {
    /// The text artist.
    pub text: A,
    /// Shift multi-line text so that it is roughly centred on the
    /// baseline of single-line text (matplotlib's `multilinebaseline`).
    pub multilinebaseline: bool,
    /// Whether the area takes part in its parent's layout.
    pub visible: bool,
}

/// The extent of a text area and how far its text is shifted up from the
/// area's origin.
pub(super) struct TextExtent {
    pub(super) bbox: Bbox,
    pub(super) baseline_shift: f64,
}

impl<A> TextArea<A> {
    /// A visible area for the text artist `text`.
    pub fn new(text: A, multilinebaseline: bool) -> Self {
        Self {
            text,
            multilinebaseline,
            visible: true,
        }
    }
}

impl<A: ArtistRef> TextArea<A> {
    /// matplotlib's `TextArea.get_bbox`: the text's extent, at least as tall
    /// as `"lp"` above the baseline, with multi-line text optionally moved
    /// to centre it on the baseline.
    pub(super) fn extent(&self, ctx: LayoutContext<'_>) -> Result<TextExtent, OffsetBoxError> {
        let id = self.text.artist_id();
        let text = ctx
            .store()
            .get_as::<TextArtist>(id)
            .ok_or(OffsetBoxError::MissingText(id))?;
        let reference = text.lp_metrics(ctx.dpi())?;
        let (h_ref, d_ref) = (reference.height, reference.descent);
        let laid_out = text.layout(ctx)?;
        let (width, mut height) = (laid_out.block.bbox.width(), laid_out.block.bbox.height());
        let mut descent = laid_out.block.lines.last().map_or(0.0, |line| line.descent);
        let mut baseline_shift = 0.0;
        if laid_out.block.lines.len() > 1 && self.multilinebaseline {
            let centred = 0.5 * height - 0.5 * (h_ref - d_ref);
            baseline_shift = descent - centred;
            descent = centred;
        } else {
            height = (h_ref - d_ref).max(height - descent) + descent;
        }
        let x0 = match text.halign {
            HAlign::Left => 0.0,
            HAlign::Center => -width / 2.0,
            HAlign::Right => -width,
        };
        Ok(TextExtent {
            bbox: Bbox::from_bounds(x0, -descent, width, height),
            baseline_shift,
        })
    }
}

/// Artists in any coordinate system, shifted so that the lower-left corner
/// of their extent is the box's origin (matplotlib's `AuxTransformBox`):
/// unlike a [`DrawingArea`], the box is as large as its artists.
#[derive(Clone, Debug, PartialEq)]
pub struct AuxTransformBox<A> {
    /// The coordinates of the artists (matplotlib's `aux_transform`).
    pub aux_transform: Transform,
    /// The artists, in drawing order.
    pub artists: Vec<A>,
    /// Whether the box takes part in its parent's layout.
    pub visible: bool,
    /// The extent of the artists under `aux_transform` alone, in display
    /// pixels, as last measured by the figure's layout pass.
    pub(super) extent: Option<Bbox>,
}

impl<A> AuxTransformBox<A> {
    /// A visible, empty box whose artists are in `aux_transform`
    /// coordinates.
    pub fn new(aux_transform: Transform) -> Self {
        Self {
            aux_transform,
            artists: Vec::new(),
            visible: true,
            extent: None,
        }
    }

    /// The same box with `artist` drawn last (matplotlib's `add_artist`).
    #[must_use]
    pub fn with_artist(mut self, artist: A) -> Self {
        self.artists.push(artist);
        self
    }

    /// Records the extent of the artists under the aux transform alone.
    pub(crate) fn set_extent(&mut self, extent: Bbox) {
        self.extent = Some(extent);
    }

    /// matplotlib's `AuxTransformBox.get_bbox`: the size of the artists'
    /// extent, from the origin (nothing before the first measurement).
    pub(super) fn bbox(&self) -> Bbox {
        let extent = self.extent.unwrap_or(Bbox::from_bounds(0.0, 0.0, 0.0, 0.0));
        Bbox::from_bounds(0.0, 0.0, extent.width(), extent.height())
    }

    /// The artists placed with the box's origin at `offset`: the aux
    /// transform, then the shift of the extent's corner to the origin
    /// (`ref_offset_transform`), then the offset.
    pub(super) fn placement(&self, offset: Point) -> Placement<A>
    where
        A: Clone,
    {
        let corner = self.extent.map_or(Point::ORIGIN, |extent| extent.p0);
        Placement::Artists {
            artists: self.artists.clone(),
            transform: self
                .aux_transform
                .clone()
                .then(Transform::Affine(Affine2D::translation(
                    -corner.x, -corner.y,
                )))
                .then(Transform::Affine(Affine2D::translation(offset.x, offset.y))),
            clip: None,
        }
    }
}

/// An image drawn at its own size, scaled by `zoom` (matplotlib's
/// `OffsetImage`).
#[derive(Clone, Debug, PartialEq)]
pub struct OffsetImage<A> {
    /// The image artist.
    pub image: A,
    /// Size of an image pixel in points (in pixels without `dpi_cor`).
    pub zoom: f64,
    /// Whether the size follows the canvas resolution (matplotlib's
    /// `dpi_cor`).
    pub dpi_cor: bool,
    /// Whether the image takes part in its parent's layout.
    pub visible: bool,
}

impl<A: ArtistRef> OffsetImage<A> {
    /// matplotlib's `OffsetImage.get_bbox`: the array's columns and rows,
    /// scaled by the zoom (and the resolution), from the origin.
    pub(super) fn bbox(&self, ctx: LayoutContext<'_>) -> Result<Bbox, OffsetBoxError> {
        let id = self.image.artist_id();
        let (rows, cols) = ctx
            .store()
            .get_as::<BboxImageArtist>(id)
            .ok_or(OffsetBoxError::MissingImage(id))?
            .shape();
        let dpi_cor = if self.dpi_cor {
            ctx.points_to_pixels(1.0)
        } else {
            1.0
        };
        Ok(Bbox::from_bounds(
            0.0,
            0.0,
            dpi_cor * cols as f64 * self.zoom,
            dpi_cor * rows as f64 * self.zoom,
        ))
    }
}

/// A child box with padding around it and an optional frame (matplotlib's
/// `PaddedBox`).
#[derive(Clone, Debug, PartialEq)]
pub struct PaddedBox<A> {
    /// Padding around the child, in points.
    pub pad: f64,
    /// The padded box.
    pub child: Box<OffsetBox<A>>,
    /// The frame (a `FancyBboxPatch`), drawn below the child when visible.
    pub frame: A,
    /// Whether the box takes part in its parent's layout.
    pub visible: bool,
}
