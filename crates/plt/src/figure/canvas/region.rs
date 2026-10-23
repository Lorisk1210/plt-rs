//! Saved rectangles of a canvas (matplotlib's `BufferRegion`) and the
//! pixel boxes they are measured in.

use std::fmt;

use plt_core::{Bbox, Point};
use plt_raster::Pixmap;

use super::CanvasError;
use super::state::{Generation, KeptCanvas};

/// A rectangle of whole canvas pixels, measured from the top-left corner
/// of the canvas with y growing downwards, as the pixels are stored
/// (matplotlib's `BufferRegion.get_extents()`). The left and top edges
/// are included, the right and bottom ones are not. Boxes may reach past
/// the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelBox {
    /// The left edge.
    pub x0: i64,
    /// The top edge.
    pub y0: i64,
    /// The right edge.
    pub x1: i64,
    /// The bottom edge.
    pub y1: i64,
}

impl PixelBox {
    /// The box from `(x0, y0)` to `(x1, y1)`.
    pub const fn new(x0: i64, y0: i64, x1: i64, y1: i64) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// The whole of a `width × height` canvas.
    pub(crate) fn canvas(width: u32, height: u32) -> Self {
        Self::new(0, 0, i64::from(width), i64::from(height))
    }

    /// The width in pixels (0 when the right edge is not past the left).
    pub fn width(self) -> u32 {
        span(self.x0, self.x1)
    }

    /// The height in pixels (0 when the bottom edge is not below the top).
    pub fn height(self) -> u32 {
        span(self.y0, self.y1)
    }

    /// Whether the box holds no pixel.
    pub fn is_empty(self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    /// The pixels in both boxes.
    #[must_use]
    pub fn intersection(self, other: Self) -> Self {
        Self::new(
            self.x0.max(other.x0),
            self.y0.max(other.y0),
            self.x1.min(other.x1),
            self.y1.min(other.y1),
        )
    }

    /// The box moved by `(dx, dy)`, saturating at the `i64` range.
    #[must_use]
    pub fn translated(self, dx: i64, dy: i64) -> Self {
        Self::new(
            self.x0.saturating_add(dx),
            self.y0.saturating_add(dy),
            self.x1.saturating_add(dx),
            self.y1.saturating_add(dy),
        )
    }

    /// The pixels matplotlib's `copy_from_bbox` saves for the display box
    /// `bbox` (pixels from the bottom-left corner) on a canvas `height`
    /// pixels tall: every coordinate truncated towards zero, as C's
    /// `(int)` does, then flipped.
    ///
    /// # Errors
    ///
    /// When a coordinate is not finite or out of C's `int` range, or the
    /// box is upside down.
    pub(crate) fn saved_from(bbox: &Bbox, height: u32) -> Result<Self, CanvasError> {
        let truncated = |value: f64| whole_pixel(value).ok_or(CanvasError::InvalidBox(*bbox));
        let (x0, x1) = (truncated(bbox.x0())?, truncated(bbox.x1())?);
        let (top, bottom) = (truncated(bbox.y1())?, truncated(bbox.y0())?);
        let height = i64::from(height);
        let saved = Self::new(x0, height - top, x1, height - bottom);
        if saved.x1 < saved.x0 || saved.y1 < saved.y0 {
            return Err(CanvasError::InvalidBox(*bbox));
        }
        Ok(saved)
    }

    /// The whole pixels covering the display box `bbox` on a canvas
    /// `width × height` pixels large, within the canvas: what a window
    /// updates for a blit of `bbox` (matplotlib's Tk backend rounds the
    /// box outwards this way).
    ///
    /// # Errors
    ///
    /// When a coordinate is not finite.
    pub(crate) fn covering(bbox: &Bbox, width: u32, height: u32) -> Result<Self, CanvasError> {
        let coordinates = [bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()];
        if !coordinates.iter().all(|c| c.is_finite()) {
            return Err(CanvasError::InvalidBox(*bbox));
        }
        let (left, right) = (bbox.xmin().floor(), bbox.xmax().ceil());
        let (bottom, top) = (bbox.ymin().floor(), bbox.ymax().ceil());
        let flipped = |y: f64| f64::from(height) - y;
        // Float-to-int `as` casts saturate; the box is clipped to the
        // canvas right after.
        let covering = Self::new(
            left as i64,
            flipped(top) as i64,
            right as i64,
            flipped(bottom) as i64,
        );
        Ok(covering.intersection(Self::canvas(width, height)))
    }
}

/// The pixels from `start` to `end`, saturating at the `u32` range.
fn span(start: i64, end: i64) -> u32 {
    u32::try_from(end.saturating_sub(start).max(0)).unwrap_or(u32::MAX)
}

/// `value` truncated towards zero, as C's `(int)` and Python's `int()`
/// do, if it is finite and in the range of C's `int`.
fn whole_pixel(value: f64) -> Option<i64> {
    let whole = value.trunc();
    let in_range = whole >= f64::from(i32::MIN) && whole <= f64::from(i32::MAX);
    // In range and whole, the cast is exact.
    in_range.then_some(whole as i64)
}

/// A rectangle of a figure's canvas saved by
/// [`FigureCanvas::copy_from_bbox`](super::FigureCanvas::copy_from_bbox)
/// to be put back with
/// [`FigureCanvas::restore_region`](super::FigureCanvas::restore_region)
/// (matplotlib's `BufferRegion`), usually the background that animated
/// artists are drawn over.
///
/// A region belongs to the canvas as it was drawn: once the figure is
/// drawn again, resized or given another resolution, restoring it fails
/// with [`CanvasError::StaleRegion`] and a new background should be saved
/// (matplotlib programs do so on `draw_event`).
#[derive(Clone, PartialEq)]
pub struct BufferRegion {
    /// The box asked for.
    extents: PixelBox,
    /// The part of `extents` inside the canvas, whose pixels are saved.
    saved: PixelBox,
    /// The pixels of `saved`.
    pixels: Pixmap,
    /// The canvas the pixels come from.
    generation: Generation,
}

impl fmt::Debug for BufferRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferRegion")
            .field("extents", &self.extents)
            .finish_non_exhaustive()
    }
}

impl BufferRegion {
    /// The rectangle of the canvas saved, in canvas pixels from its
    /// top-left corner (matplotlib's `get_extents()`); parts outside the
    /// canvas hold no pixels and are never restored.
    pub fn extents(&self) -> PixelBox {
        self.extents
    }

    /// The width of the rectangle in pixels.
    pub fn width(&self) -> u32 {
        self.extents.width()
    }

    /// The height of the rectangle in pixels.
    pub fn height(&self) -> u32 {
        self.extents.height()
    }

    /// Saves the pixels of `canvas` in the display box `bbox`
    /// (matplotlib's `RendererAgg.copy_from_bbox`).
    pub(super) fn save(canvas: &KeptCanvas, bbox: &Bbox) -> Result<Self, CanvasError> {
        let pixmap = canvas.renderer.pixmap();
        let extents = PixelBox::saved_from(bbox, pixmap.height())?;
        let saved = extents.intersection(PixelBox::canvas(pixmap.width(), pixmap.height()));
        let size = (saved.width(), saved.height());
        let mut pixels = Pixmap::new(size.0, size.1).map_err(CanvasError::Pixels)?;
        copy_pixels(pixmap, (saved.x0, saved.y0), &mut pixels, (0, 0), size);
        Ok(Self {
            extents,
            saved,
            pixels,
            generation: canvas.generation,
        })
    }

    /// Writes the region back into `canvas` (matplotlib's
    /// `RendererAgg.restore_region`): all of it where it came from, or
    /// with `bbox` the part it names, moved so that the region's top-left
    /// corner lands on `xy` (by default the corner of `bbox`). Like
    /// matplotlib, `bbox` and `xy` are taken in canvas pixels from the
    /// top-left corner, truncated to whole pixels, and `bbox` includes its
    /// right and bottom edges.
    pub(super) fn restore(
        &self,
        canvas: &mut KeptCanvas,
        bbox: Option<Bbox>,
        xy: Option<Point>,
    ) -> Result<(), CanvasError> {
        if self.generation != canvas.generation {
            return Err(CanvasError::StaleRegion);
        }
        let (source, offset) = match (bbox, xy) {
            (None, None) => {
                let whole = PixelBox::new(0, 0, i64::MAX, i64::MAX);
                (whole, (self.extents.x0, self.extents.y0))
            }
            _ => self.part(bbox, xy)?,
        };
        let pixmap = canvas.renderer.pixmap_mut();
        let into_canvas = PixelBox::canvas(pixmap.width(), pixmap.height());
        // What is copied, in canvas pixels before the move: the part asked
        // for of the saved pixels, as far as it lands on the canvas.
        let local = self.saved.translated(-self.extents.x0, -self.extents.y0);
        let copied = source
            .intersection(local)
            .intersection(into_canvas.translated(-offset.0, -offset.1))
            .translated(self.extents.x0, self.extents.y0);
        if copied.is_empty() {
            return Ok(());
        }
        let from = (copied.x0 - self.saved.x0, copied.y0 - self.saved.y0);
        let to = (
            copied.x0 - self.extents.x0 + offset.0,
            copied.y0 - self.extents.y0 + offset.1,
        );
        copy_pixels(
            &self.pixels,
            from,
            pixmap,
            to,
            (copied.width(), copied.height()),
        );
        Ok(())
    }

    /// The part of the region `bbox` names, in region pixels, and where
    /// the region's top-left corner goes, as matplotlib's `restore_region`
    /// computes them with `bbox` or `xy`.
    fn part(
        &self,
        bbox: Option<Bbox>,
        xy: Option<Point>,
    ) -> Result<(PixelBox, (i64, i64)), CanvasError> {
        let extents = self.extents;
        let (x1, y1, x2, y2) = match bbox {
            Some(bbox) => {
                let whole = |value: f64| whole_pixel(value).ok_or(CanvasError::InvalidBox(bbox));
                let corners = (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1());
                (
                    whole(corners.0)?,
                    whole(corners.1)?,
                    whole(corners.2)?,
                    whole(corners.3)?,
                )
            }
            None => (extents.x0, extents.y0, extents.x1, extents.y1),
        };
        let (x, y) = match xy {
            Some(xy) => {
                let invalid =
                    || CanvasError::InvalidBox(Bbox::from_extents(xy.x, xy.y, xy.x, xy.y));
                let whole = |value: f64| whole_pixel(value).ok_or_else(invalid);
                (whole(xy.x)?, whole(xy.y)?)
            }
            None => (x1, y1),
        };
        // Agg's `copy_from` includes the right and bottom edges.
        let source = PixelBox::new(x1, y1, x2 + 1, y2 + 1).translated(-extents.x0, -extents.y0);
        Ok((source, (x, y)))
    }
}

/// Copies the `width × height` pixels at `from` in `source` to `to` in
/// `target`; both rectangles lie inside their pixmaps.
fn copy_pixels(
    source: &Pixmap,
    from: (i64, i64),
    target: &mut Pixmap,
    to: (i64, i64),
    (width, height): (u32, u32),
) {
    let column = |x: i64| usize::try_from(x).unwrap_or(0) * 4;
    let line = |y: i64, row: u32| u32::try_from(y + i64::from(row)).unwrap_or(u32::MAX);
    let bytes = width as usize * 4;
    let (start, end) = (column(from.0), column(to.0));
    for row in 0..height {
        if let (Some(source_row), Some(target_row)) = (
            source.row(line(from.1, row)),
            target.row_mut(line(to.1, row)),
        ) {
            target_row[end..end + bytes].copy_from_slice(&source_row[start..start + bytes]);
        }
    }
}
