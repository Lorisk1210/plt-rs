//! Markers: a shape rasterized once and stamped at many positions, like
//! matplotlib's `draw_markers` (which caches the fill and stroke coverage
//! in Agg `scanline_storage_aa` and replays it at integer offsets).

use pltrs_core::{Point, Rgba};

use super::Surface;
use crate::clip::Clip;
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::pixmap::Pixmap;
use crate::raster::{Scanline, Span};
use crate::style::Antialias;

/// Pre-rasterized marker coverage, relative to the marker's origin.
///
/// Create one with [`Painter::marker_stamp`](crate::Painter::marker_stamp).
#[derive(Clone, Debug, Default)]
pub struct MarkerStamp {
    fill: Vec<Scanline>,
    stroke: Vec<Scanline>,
}

impl MarkerStamp {
    pub(crate) fn new(fill: Vec<Scanline>, stroke: Vec<Scanline>) -> Self {
        Self { fill, stroke }
    }

    /// Returns `true` when the marker covers no pixel at all.
    pub fn is_empty(&self) -> bool {
        self.fill.is_empty() && self.stroke.is_empty()
    }

    /// Inclusive pixel bounds `(min_x, min_y, max_x, max_y)` of the
    /// coverage relative to the origin, or `None` when empty.
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        self.fill
            .iter()
            .chain(&self.stroke)
            .flat_map(|sl| {
                sl.spans()
                    .map(move |s| (sl.y(), s.x, s.x + s.len() as i32 - 1))
            })
            .fold(None, |acc, (y, x0, x1)| {
                let (a, b, c, d) = acc.unwrap_or((x0, y, x1, y));
                Some((a.min(x0), b.min(y), c.max(x1), d.max(y)))
            })
    }

    /// Stamps the marker at every position: the origin lands on pixel
    /// `(⌊x⌋, ⌊y⌋)`. Non-finite positions are skipped. At each position
    /// the face (if any) is blended first, then the edge (if any).
    ///
    /// matplotlib folds a half-pixel offset into the position transform
    /// (`translate(0.5, height + 0.5)`) before flooring, so positions are
    /// effectively rounded; it clips stamps with its clip box *inclusively*
    /// (see [`ClipBox::inclusive`](crate::ClipBox::inclusive)) and, when a
    /// clip path is active, ignores the clip box altogether. Under a mask,
    /// runs of uniform partial coverage are drawn at full strength, as
    /// matplotlib's packed scanlines are.
    pub fn draw(
        &self,
        target: &mut Pixmap,
        positions: &[Point],
        face: Option<Rgba>,
        edge: Option<Rgba>,
        clip: &Clip<'_>,
    ) -> Result<(), RasterError> {
        let (width, height) = (f64::from(target.width()), f64::from(target.height()));
        let mut surface = Surface::new(target, clip.rect, clip.mask)?.with_packed_spans();
        let Some((min_x, min_y, max_x, max_y)) = self.bounds() else {
            return Ok(());
        };
        if surface.is_empty() {
            return Ok(());
        }
        let face = face.map(Rgba8::from);
        let edge = edge.map(Rgba8::from);
        // The spans of both coverages, read out of their scanlines once.
        let layers = [(rows(&self.fill), face), (rows(&self.stroke), edge)];
        for p in positions.iter().filter(|p| p.is_finite()) {
            let (x, y) = (p.x.floor(), p.y.floor());
            // Cull stamps entirely outside the target (as matplotlib does),
            // which also keeps the offsets within `i32`.
            if x < -1.0 - f64::from(max_x)
                || x > 1.0 + width - f64::from(min_x)
                || y < -1.0 - f64::from(max_y)
                || y > 1.0 + height - f64::from(min_y)
            {
                continue;
            }
            let (dx, dy) = (x as i32, y as i32);
            // A stamp wholly visible and unmasked, the usual case, needs no
            // clipping span by span.
            let inside = surface.shows_all_of(min_x + dx, min_y + dy, max_x + dx, max_y + dy);
            for (spans, color) in &layers {
                let Some(color) = *color else { continue };
                for &(row, span) in spans {
                    let shifted = Span {
                        x: span.x + dx,
                        covers: span.covers,
                    };
                    if inside {
                        surface.blend_solid_inside(row + dy, shifted, color);
                    } else {
                        surface.blend_solid(row + dy, shifted, color, Antialias::On);
                    }
                }
            }
        }
        Ok(())
    }
}

/// The spans of `scanlines` with the row of each, in order.
fn rows(scanlines: &[Scanline]) -> Vec<(i32, Span<'_>)> {
    scanlines
        .iter()
        .flat_map(|sl| sl.spans().map(move |span| (sl.y(), span)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot() -> MarkerStamp {
        let mut sl = Scanline::new();
        sl.reset(0);
        sl.add_cell(0, 255);
        sl.add_cell(1, 128);
        MarkerStamp::new(vec![sl], Vec::new())
    }

    #[test]
    fn stamps_at_floored_positions() {
        let stamp = dot();
        assert_eq!(stamp.bounds(), Some((0, 0, 1, 0)));
        let mut p = Pixmap::new(4, 2).unwrap();
        let positions = [
            Point::new(1.9, 1.2),
            Point::new(f64::NAN, 0.0),
            Point::new(1e300, 0.0),
        ];
        stamp
            .draw(&mut p, &positions, Some(Rgba::BLACK), None, &Clip::NONE)
            .unwrap();
        let alphas: Vec<u8> = p.data().chunks(4).map(|c| c[3]).collect();
        assert_eq!(alphas, vec![0, 0, 0, 0, 0, 255, 128, 0]);
    }

    #[test]
    fn empty_stamp_and_missing_colours_draw_nothing() {
        let mut p = Pixmap::new(2, 2).unwrap();
        let before = p.clone();
        MarkerStamp::default()
            .draw(
                &mut p,
                &[Point::ORIGIN],
                Some(Rgba::BLACK),
                Some(Rgba::BLACK),
                &Clip::NONE,
            )
            .unwrap();
        dot()
            .draw(&mut p, &[Point::ORIGIN], None, None, &Clip::NONE)
            .unwrap();
        assert_eq!(p, before);
        assert!(MarkerStamp::default().is_empty());
    }
}
