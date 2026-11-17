//! The leaves of an offset-box tree and the padded box around one child
//! (matplotlib's `DrawingArea`, `TextArea`, `AuxTransformBox`,
//! `OffsetImage` and `PaddedBox`).
//!
//! Every box measures its extent relative to its own origin, in pixels;
//! sizes given in points are scaled by the canvas resolution.

use pltrs_core::{Affine2D, Bbox, Path, Point};
use pltrs_text::HAlign;

use super::{ArtistRef, OffsetBox, OffsetBoxError, Placement};
use crate::artist::{ArtistHandle, ArtistLookup, ClipPathSpec, LayoutContext, RawHandle};
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

pltrs_core::archive_struct!(DrawingArea<A> { width, height, xdescent, ydescent, clip, artists, visible });

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
        let scale = ctx.measuring_points_to_pixels(1.0);
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
        let scale = ctx.measuring_points_to_pixels(1.0);
        let clip = self.clip.then(|| area_outline(self.width, self.height));
        Placement::Artists {
            artists: self.artists.clone(),
            transform: Transform::Affine(
                Affine2D::scaling(scale, scale).translate(offset.x, offset.y),
            ),
            clip,
        }
    }
}

/// The outline a clipped area of `width` by `height` points gives its
/// artists, in points from its origin (matplotlib's `DrawingArea.draw`
/// makes it unclosed, of four vertices).
fn area_outline(width: f64, height: f64) -> Path {
    Path::polyline(vec![
        Point::new(0.0, 0.0),
        Point::new(0.0, height),
        Point::new(width, height),
        Point::new(width, 0.0),
    ])
}

/// Whether `path` is the outline of a clipped area (see
/// [`DrawingArea::placement`]), which its artists keep from the area's
/// first draw on, rather than a clip path of the program's.
pub(crate) fn is_area_outline(path: &Path) -> bool {
    match path.vertices() {
        [_, _, corner, _] => *path == area_outline(corner.x, corner.y),
        _ => false,
    }
}

/// The outline of the clipped [`DrawingArea`] that clips `artist`, in
/// points from the area's origin: the clip path matplotlib's
/// `DrawingArea.draw` gives an artist with no clip of its own, of the
/// area's size at its first draw; `None` when no area clips it.
pub fn area_clip_outline(artist: &RawHandle) -> Option<Path> {
    match artist.clip_path()? {
        ClipPathSpec::Path { path, .. } if is_area_outline(&path) => Some(path),
        _ => None,
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

pltrs_core::archive_struct!(TextArea<A> { text, multilinebaseline, visible });

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
    /// to centre it on the baseline. The text may be removed from the
    /// figure with the box holding it, which matplotlib measures all the
    /// same.
    pub(super) fn extent(&self, ctx: LayoutContext<'_>) -> Result<TextExtent, OffsetBoxError> {
        let id = self.text.artist_id();
        let text = ctx
            .figure_state()
            .artist_as::<TextArtist>(id)
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

/// The extent of a text area holding `text`, a text of a figure or a
/// loose one, relative to the area's origin, in display pixels at the
/// resolution `dpi`, else its figure's (matplotlib's `TextArea.get_bbox`,
/// which measures a loose text at the dpi of the figure its box was given).
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::text_area_bbox;
/// use pltrs::prelude::*;
/// let text = Figure::new().text(0.0, 0.0, "one\ntwo");
/// let single = text_area_bbox(text.raw(), false, None)?;
/// let centred = text_area_bbox(text.raw(), true, None)?;
/// assert_eq!(single.height(), centred.height());
/// assert!(centred.y0() < single.y0(), "centred on the baseline");
/// let doubled = text_area_bbox(text.raw(), false, Some(200.0))?;
/// assert!((doubled.width() - 2.0 * single.width()).abs() < 2.0, "at twice the dpi");
/// # Ok::<(), pltrs::offsetbox::OffsetBoxError>(())
/// ```
///
/// # Errors
///
/// [`OffsetBoxError::MissingText`] when `text` is no text, or when it
/// cannot be measured (malformed math).
pub fn text_area_bbox(
    text: &crate::artist::RawHandle,
    multilinebaseline: bool,
    dpi: Option<f64>,
) -> Result<Bbox, OffsetBoxError> {
    Ok(text_area_extent(text, multilinebaseline, dpi)?.bbox)
}

/// How far a text area holding `text` moves it up from the area's origin,
/// in display pixels at the resolution `dpi`, else its figure's
/// (as [`text_area_bbox`]): multi-line text centred
/// on the baseline with `multilinebaseline`, else none (matplotlib's
/// `TextArea._baseline_transform`, which its `get_bbox` sets).
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::text_area_baseline_shift;
/// use pltrs::prelude::*;
/// let text = Figure::new().text(0.0, 0.0, "one\ntwo");
/// assert_eq!(text_area_baseline_shift(text.raw(), false, None)?, 0.0);
/// assert!(text_area_baseline_shift(text.raw(), true, None)? < 0.0, "lowered");
/// # Ok::<(), pltrs::offsetbox::OffsetBoxError>(())
/// ```
///
/// # Errors
///
/// As [`text_area_bbox`].
pub fn text_area_baseline_shift(
    text: &crate::artist::RawHandle,
    multilinebaseline: bool,
    dpi: Option<f64>,
) -> Result<f64, OffsetBoxError> {
    Ok(text_area_extent(text, multilinebaseline, dpi)?.baseline_shift)
}

/// The extent of a text area holding `text` and the shift of its text, at
/// the resolution `dpi` (its figure's for `None`, which is restored after).
fn text_area_extent(
    text: &crate::artist::RawHandle,
    multilinebaseline: bool,
    dpi: Option<f64>,
) -> Result<TextExtent, OffsetBoxError> {
    let state = text.figure_state().borrow();
    let own = state.dpi();
    if let Some(dpi) = dpi {
        state.set_dpi(dpi);
    }
    let extent = TextArea::new(text.id(), multilinebaseline).extent(LayoutContext::new(&state));
    state.set_dpi(own);
    extent
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
    /// Whether the last measurement met a visible text without a figure
    /// among the artists, which matplotlib fails to measure.
    pub(super) figureless_text: bool,
}

pltrs_core::archive_struct!(AuxTransformBox<A> {
    aux_transform,
    artists,
    visible,
    extent,
    figureless_text
});

impl<A> AuxTransformBox<A> {
    /// A visible, empty box whose artists are in `aux_transform`
    /// coordinates.
    pub fn new(aux_transform: Transform) -> Self {
        Self {
            aux_transform,
            artists: Vec::new(),
            visible: true,
            extent: None,
            figureless_text: false,
        }
    }

    /// The same box with `artist` drawn last (matplotlib's `add_artist`).
    #[must_use]
    pub fn with_artist(mut self, artist: A) -> Self {
        self.artists.push(artist);
        self
    }

    /// Records the extent of the artists under the aux transform alone,
    /// and whether a visible text without a figure was among them.
    pub(crate) fn set_extent(&mut self, extent: Bbox, figureless_text: bool) {
        self.extent = Some(extent);
        self.figureless_text = figureless_text;
    }

    /// matplotlib's `AuxTransformBox.get_bbox`: the size of the artists'
    /// extent, from the origin (nothing before the first measurement).
    ///
    /// # Errors
    ///
    /// [`OffsetBoxError::EmptyAuxBox`] for a box without artists, whose
    /// extent matplotlib cannot unite, and [`OffsetBoxError::NoFigure`]
    /// for one measured with a visible text without a figure.
    pub(super) fn bbox(&self) -> Result<Bbox, OffsetBoxError> {
        if self.artists.is_empty() {
            return Err(OffsetBoxError::EmptyAuxBox);
        }
        if self.figureless_text {
            return Err(OffsetBoxError::NoFigure);
        }
        let extent = self.extent.unwrap_or(Bbox::from_bounds(0.0, 0.0, 0.0, 0.0));
        Ok(Bbox::from_bounds(0.0, 0.0, extent.width(), extent.height()))
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

pltrs_core::archive_struct!(OffsetImage<A> { image, zoom, dpi_cor, visible });

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
            ctx.measuring_points_to_pixels(1.0)
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

pltrs_core::archive_struct!(PaddedBox<A> { pad, child, frame, visible });
