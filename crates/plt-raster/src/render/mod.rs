//! Compositing coverage onto a pixmap, and the span generators that
//! colour it (Gouraud shading, images, coverage bitmaps, markers).

pub(crate) mod gouraud;
pub(crate) mod image;
pub(crate) mod marker;

use crate::clip::ClipBox;
use crate::color::{Rgba8, blend_pixel, blend_run, combine_mask};
use crate::error::RasterError;
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::raster::{Covers, Span};
use crate::style::Antialias;

/// A pixmap restricted to a pixel clip rectangle and an optional mask —
/// the equivalent of Agg's `renderer_base` (+ `pixfmt_amask_adaptor`).
#[derive(Debug)]
pub(crate) struct Surface<'a> {
    pixels: &'a mut [u8],
    mask: Option<&'a [u8]>,
    width: usize,
    /// Visible pixels; `None` when the clip leaves nothing visible.
    bounds: Option<ClipBox>,
    /// See [`Surface::with_packed_spans`].
    packed: bool,
}

impl<'a> Surface<'a> {
    /// Wraps `target`, clipping pixels to `rect` (or the whole target)
    /// and scaling covers by `mask`.
    pub(crate) fn new(
        target: &'a mut Pixmap,
        rect: Option<ClipBox>,
        mask: Option<&'a Mask>,
    ) -> Result<Self, RasterError> {
        let (width, height) = (target.width(), target.height());
        if let Some(mask) = mask {
            mask.check_covers(width, height)?;
        }
        Ok(Self {
            bounds: rect
                .unwrap_or_else(|| ClipBox::full(width, height))
                .within(width, height),
            mask: mask.map(Mask::data),
            width: width as usize,
            pixels: target.data_mut(),
            packed: false,
        })
    }

    /// Reproduces how matplotlib composites Agg's *packed* scanlines
    /// (`scanline_p8`) through a clip mask: `pixfmt_amask_adaptor` drops
    /// the cover of solid runs and uses the mask value alone, so masked
    /// runs of uniform partial coverage come out at full strength.
    /// matplotlib hits this for markers and hatch patterns; paths, images
    /// and Gouraud triangles use unpacked scanlines and are unaffected.
    pub(crate) const fn with_packed_spans(mut self) -> Self {
        self.packed = true;
        self
    }

    /// The span as the mask adaptor effectively sees it.
    fn effective<'s>(&self, span: Span<'s>) -> Span<'s> {
        match span.covers {
            Covers::Solid { len, .. } if self.packed && self.mask.is_some() => Span {
                x: span.x,
                covers: Covers::Solid { len, cover: 255 },
            },
            _ => span,
        }
    }

    /// Returns `true` when nothing can be drawn.
    pub(crate) const fn is_empty(&self) -> bool {
        self.bounds.is_none()
    }

    /// The visible part `[start, end)` (offsets into a span of `len` pixels
    /// starting at `x` on row `y`), if any.
    fn visible(&self, y: i32, x: i32, len: usize) -> Option<(usize, usize)> {
        let b = self.bounds?;
        if y < b.y0() || y >= b.y1() {
            return None;
        }
        let end = x.saturating_add(i32::try_from(len).unwrap_or(i32::MAX));
        let (lo, hi) = (x.max(b.x0()), end.min(b.x1()));
        (lo < hi).then(|| ((lo - x) as usize, (hi - x) as usize))
    }

    /// Byte offset of pixel `(x, y)` (both known to be visible).
    fn index(&self, x: i32, y: i32) -> usize {
        y as usize * self.width + x as usize
    }

    fn masked(&self, pixel: usize, cover: u8) -> u8 {
        match self.mask {
            Some(m) => combine_mask(cover, m[pixel]),
            None => cover,
        }
    }

    /// Blends a solid colour through the span's covers (Agg's
    /// `renderer_scanline_aa_solid`), or at full strength wherever the span
    /// has coverage when anti-aliasing is off (`renderer_scanline_bin_solid`).
    pub(crate) fn blend_solid(&mut self, y: i32, span: Span<'_>, color: Rgba8, aa: Antialias) {
        if color.a == 0 {
            return;
        }
        let span = self.effective(span);
        let Some((start, end)) = self.visible(y, span.x, span.len()) else {
            return;
        };
        let first = self.index(span.x + start as i32, y);
        let uniform = match (aa, span.covers) {
            (Antialias::Off, _) => Some(255),
            (Antialias::On, Covers::Solid { cover, .. }) => Some(cover),
            (Antialias::On, Covers::Varying(_)) => None,
        };
        if let (Some(cover), None) = (uniform, self.mask) {
            blend_run(
                &mut self.pixels[first * 4..(first + end - start) * 4],
                color,
                cover,
            );
            return;
        }
        for i in start..end {
            let pixel = first + i - start;
            let cover = self.masked(pixel, uniform.unwrap_or_else(|| span.cover_at(i)));
            blend_pixel(&mut self.pixels[pixel * 4..pixel * 4 + 4], color, cover);
        }
    }

    /// Blends per-pixel colours (`colors[i]` belongs to pixel `span.x + i`)
    /// through the span's covers (Agg's `renderer_scanline_aa` with
    /// `copy_or_blend_pix`).
    pub(crate) fn blend_colors(&mut self, y: i32, span: Span<'_>, colors: &[Rgba8]) {
        let span = self.effective(span);
        let Some((start, end)) = self.visible(y, span.x, span.len()) else {
            return;
        };
        let first = self.index(span.x + start as i32, y);
        for (i, &color) in colors.iter().enumerate().take(end).skip(start) {
            let pixel = first + i - start;
            let cover = self.masked(pixel, span.cover_at(i));
            blend_pixel(&mut self.pixels[pixel * 4..pixel * 4 + 4], color, cover);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(x: i32, covers: &[u8]) -> Span<'_> {
        Span {
            x,
            covers: Covers::Varying(covers),
        }
    }

    #[test]
    fn solid_spans_are_clipped_to_the_box() {
        let mut p = Pixmap::new(4, 2).unwrap();
        let mut s = Surface::new(&mut p, Some(ClipBox::new(1, 0, 3, 1)), None).unwrap();
        let solid = Span {
            x: -2,
            covers: Covers::Solid {
                len: 10,
                cover: 255,
            },
        };
        s.blend_solid(0, solid, Rgba8::new(1, 2, 3, 255), Antialias::On);
        s.blend_solid(1, solid, Rgba8::new(1, 2, 3, 255), Antialias::On);
        let alphas: Vec<u8> = p.data().chunks(4).map(|c| c[3]).collect();
        assert_eq!(alphas, vec![0, 255, 255, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn binary_mode_paints_any_coverage_fully() {
        let mut p = Pixmap::new(3, 1).unwrap();
        let mut s = Surface::new(&mut p, None, None).unwrap();
        s.blend_solid(
            0,
            span(0, &[1, 0, 200]),
            Rgba8::new(9, 9, 9, 255),
            Antialias::Off,
        );
        let alphas: Vec<u8> = p.data().chunks(4).map(|c| c[3]).collect();
        assert_eq!(alphas, vec![255, 255, 255]);
    }

    #[test]
    fn mask_scales_cover() {
        let mut p = Pixmap::new(2, 1).unwrap();
        let mask = Mask::from_vec(2, 1, vec![255, 128]).unwrap();
        let mut s = Surface::new(&mut p, None, Some(&mask)).unwrap();
        s.blend_solid(
            0,
            span(0, &[255, 255]),
            Rgba8::new(0, 0, 0, 255),
            Antialias::On,
        );
        assert_eq!(p.pixel(0, 0).unwrap().a, 255);
        assert_eq!(p.pixel(1, 0).unwrap().a, 128);
    }

    #[test]
    fn packed_solid_runs_ignore_their_cover_under_a_mask() {
        let mask = Mask::from_vec(2, 1, vec![255, 255]).unwrap();
        let half = Span {
            x: 0,
            covers: Covers::Solid { len: 2, cover: 128 },
        };
        let black = Rgba8::new(0, 0, 0, 255);
        let mut p = Pixmap::new(2, 1).unwrap();
        Surface::new(&mut p, None, Some(&mask))
            .unwrap()
            .blend_solid(0, half, black, Antialias::On);
        assert_eq!(p.pixel(0, 0).unwrap().a, 128);
        let mut p = Pixmap::new(2, 1).unwrap();
        Surface::new(&mut p, None, Some(&mask))
            .unwrap()
            .with_packed_spans()
            .blend_solid(0, half, black, Antialias::On);
        assert_eq!(p.pixel(0, 0).unwrap().a, 255);
    }

    #[test]
    fn mismatched_mask_is_an_error() {
        let mut p = Pixmap::new(2, 1).unwrap();
        let mask = Mask::new(3, 1).unwrap();
        assert!(Surface::new(&mut p, None, Some(&mask)).is_err());
    }

    #[test]
    fn per_pixel_colours() {
        let mut p = Pixmap::new(2, 1).unwrap();
        let mut s = Surface::new(&mut p, None, None).unwrap();
        let colors = [Rgba8::new(1, 1, 1, 255), Rgba8::new(2, 2, 2, 0)];
        s.blend_colors(0, span(0, &[255, 255]), &colors);
        assert_eq!(p.pixel(0, 0), Some(Rgba8::new(1, 1, 1, 255)));
        assert_eq!(p.pixel(1, 0), Some(Rgba8::default()));
        assert!(
            Surface::new(&mut p, Some(ClipBox::new(5, 5, 6, 6)), None)
                .unwrap()
                .visible(0, 0, 1)
                .is_none()
        );
    }
}
