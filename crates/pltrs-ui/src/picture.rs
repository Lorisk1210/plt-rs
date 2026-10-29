//! Full-colour icons drawn from bitmaps at several pixel densities.

use pltrs_core::Point;
use pltrs_raster::{Pixmap, RasterError};

use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::Rect;

/// A full-colour square icon drawn from bitmaps, as toolkits draw
/// matplotlib's toolbar images: the same picture at several pixel
/// densities, e.g. 24 pixels wide for 1× screens and 48 for 2×. The
/// bitmap as wide as the icon on screen is drawn pixel for pixel, so it
/// stays crisp; at other scale factors the narrowest wider bitmap (or the
/// widest there is) is scaled to fit.
///
/// ```
/// use pltrs_raster::{Pixmap, Rgba8};
/// use pltrs_ui::Picture;
/// let black = Rgba8::new(0, 0, 0, 255);
/// let picture = Picture::new(
///     24.0,
///     vec![Pixmap::filled(24, 24, black)?, Pixmap::filled(48, 48, black)?],
/// );
/// assert_eq!(picture.side(), 24.0);
/// # Ok::<(), pltrs_raster::RasterError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    side: f64,
    /// Square bitmaps, narrowest first.
    bitmaps: Vec<Pixmap>,
}

impl Picture {
    /// A picture `side` logical pixels wide drawn from square `bitmaps`
    /// (straight RGBA), in any order. A picture without bitmaps draws
    /// nothing.
    pub fn new(side: f64, mut bitmaps: Vec<Pixmap>) -> Self {
        bitmaps.sort_by_key(Pixmap::width);
        Self { side, bitmaps }
    }

    /// The side of the picture in logical pixels.
    pub fn side(&self) -> f64 {
        self.side
    }

    /// Paints the picture centred in `rect`, its side times the scale
    /// factor wide and on whole pixels, at `opacity` (1 for opaque).
    pub fn paint(&self, canvas: &mut Canvas<'_>, rect: Rect, opacity: f64) -> Result<(), UiError> {
        // Sides are a few dozen pixels; the cast saturates on absurd ones.
        let wanted = (self.side * canvas.scale_factor()).round().max(0.0) as u32;
        let Some(bitmap) = self
            .bitmaps
            .iter()
            .find(|bitmap| bitmap.width() >= wanted)
            .or(self.bitmaps.last())
        else {
            return Ok(());
        };
        let side = f64::from(wanted);
        let center = rect.center();
        let origin = Point::new(
            (center.x - side / 2.0).round(),
            (center.y - side / 2.0).round(),
        );
        if bitmap.width() == wanted && bitmap.height() == wanted {
            canvas.draw_pixmap_with_opacity(bitmap, origin, opacity)
        } else {
            let scaled = resized(bitmap, wanted)?;
            canvas.draw_pixmap_with_opacity(&scaled, origin, opacity)
        }
    }
}

/// `bitmap` scaled to a `side`-pixel square with a box filter: each new
/// pixel averages, by area and with premultiplied alpha, the pixels it
/// covers.
fn resized(bitmap: &Pixmap, side: u32) -> Result<Pixmap, RasterError> {
    let columns = box_weights(bitmap.width(), side);
    let rows = box_weights(bitmap.height(), side);
    let width = bitmap.width() as usize;
    let data = bitmap.data();
    let mut out = Vec::with_capacity(side as usize * side as usize * 4);
    for row in &rows {
        for column in &columns {
            let mut sum = [0.0; 4];
            for &(y, wy) in row {
                for &(x, wx) in column {
                    let pixel = &data[(y * width + x) * 4..][..4];
                    let weight = wx * wy * f64::from(pixel[3]) / 255.0;
                    for (total, &channel) in sum.iter_mut().zip(&pixel[..3]) {
                        *total += weight * f64::from(channel);
                    }
                    sum[3] += weight;
                }
            }
            let alpha = sum[3];
            let straight = |total: f64| if alpha > 0.0 { total / alpha } else { 0.0 };
            // Averages of bytes stay within 0..=255.
            out.extend(sum[..3].iter().map(|&total| straight(total).round() as u8));
            out.push((alpha * 255.0).round() as u8);
        }
    }
    Pixmap::from_vec(side, side, out)
}

/// For each of `to` pixels spanning the same length as `from` pixels, the
/// pixels it covers with the share of it each covers.
fn box_weights(from: u32, to: u32) -> Vec<Vec<(usize, f64)>> {
    let ratio = f64::from(from) / f64::from(to);
    (0..to)
        .map(|index| {
            let (start, end) = (f64::from(index) * ratio, f64::from(index + 1) * ratio);
            // Pixel indices are within the bitmap.
            let first = start.floor() as usize;
            let last = (end.ceil() as usize).min(from as usize);
            (first..last)
                .map(|pixel| {
                    let covered = end.min(pixel as f64 + 1.0) - start.max(pixel as f64);
                    (pixel, covered / ratio)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pltrs_raster::{Painter, Rgba8};

    use super::*;

    /// A `side`-pixel bitmap, opaque black on its left half and
    /// transparent on its right half.
    fn half_black(side: u32) -> Pixmap {
        let mut bitmap = Pixmap::new(side, side).unwrap();
        for y in 0..side {
            for x in 0..side / 2 {
                bitmap.set_pixel(x, y, Rgba8::new(0, 0, 0, 255));
            }
        }
        bitmap
    }

    /// `picture` painted at `scale` into the middle of a white 40-pixel
    /// square (times the scale), at `opacity`.
    fn painted(picture: &Picture, scale: f64, opacity: f64) -> Pixmap {
        let side = (40.0 * scale) as u32;
        let mut target = Pixmap::filled(side, side, Rgba8::new(255, 255, 255, 255)).unwrap();
        let mut painter = Painter::new();
        let whole = Rect::new(0.0, 0.0, f64::from(side), f64::from(side));
        let mut canvas = Canvas::new(&mut target, &mut painter, whole, scale);
        picture.paint(&mut canvas, whole, opacity).unwrap();
        target
    }

    fn red(target: &Pixmap, x: u32, y: u32) -> u8 {
        target.pixel(x, y).unwrap().r
    }

    #[test]
    fn the_bitmap_of_the_scale_factor_is_drawn_pixel_for_pixel() {
        let picture = Picture::new(24.0, vec![half_black(48), half_black(24)]);
        // At 1×, the 24-pixel bitmap lands on pixels 8 to 31.
        let target = painted(&picture, 1.0, 1.0);
        assert_eq!(red(&target, 8, 20), 0);
        assert_eq!(red(&target, 19, 20), 0, "a crisp edge");
        assert_eq!(red(&target, 20, 20), 255);
        assert_eq!(red(&target, 7, 20), 255, "nothing outside");
        // At 2×, the 48-pixel one on pixels 16 to 63.
        let target = painted(&picture, 2.0, 1.0);
        assert_eq!(red(&target, 16, 40), 0);
        assert_eq!(red(&target, 39, 40), 0);
        assert_eq!(red(&target, 40, 40), 255);
        assert_eq!(red(&target, 15, 40), 255);
    }

    #[test]
    fn other_scale_factors_scale_the_next_wider_bitmap() {
        let picture = Picture::new(24.0, vec![half_black(24), half_black(48)]);
        // At 1.5×, the icon is 36 pixels wide, from pixel 12 to 47.
        let target = painted(&picture, 1.5, 1.0);
        assert_eq!(red(&target, 12, 30), 0);
        assert_eq!(red(&target, 29, 30), 0);
        assert_eq!(red(&target, 30, 30), 255);
        assert_eq!(red(&target, 47, 30), 255);
        assert_eq!(red(&target, 11, 30), 255);
    }

    #[test]
    fn opacity_fades_the_picture() {
        let picture = Picture::new(24.0, vec![half_black(24)]);
        let shade = red(&painted(&picture, 1.0, 0.4), 10, 20);
        assert!((150..=155).contains(&shade), "60% of white: {shade}");
        let empty = Picture::new(24.0, Vec::new());
        assert_eq!(red(&painted(&empty, 1.0, 1.0), 10, 20), 255);
    }

    #[test]
    fn box_filters_average_what_each_pixel_covers() {
        let weights = box_weights(4, 3);
        let expected = [
            [(0, 0.75), (1, 0.25)],
            [(1, 0.5), (2, 0.5)],
            [(2, 0.25), (3, 0.75)],
        ];
        for (got, want) in weights.iter().zip(expected) {
            assert_eq!(got.len(), 2);
            for (&(pixel, weight), (want_pixel, want_weight)) in got.iter().zip(want) {
                assert_eq!(pixel, want_pixel);
                assert!((weight - want_weight).abs() < 1e-12, "{got:?}");
            }
        }
        let shrunk = resized(&half_black(4), 2).unwrap();
        assert_eq!(shrunk.pixel(0, 0), Some(Rgba8::new(0, 0, 0, 255)));
        assert_eq!(shrunk.pixel(1, 1).map(|pixel| pixel.a), Some(0));
    }
}
