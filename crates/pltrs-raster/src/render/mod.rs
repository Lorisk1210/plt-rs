//! Compositing coverage onto a pixmap, and the span generators that
//! colour it (Gouraud shading, images, coverage bitmaps, markers).

pub(crate) mod gouraud;
pub(crate) mod image;
pub(crate) mod marker;

use crate::clip::ClipBox;
use crate::color::{Rgba8, blend_covered, blend_pixel, blend_run, combine_mask};
use crate::error::RasterError;
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::raster::{Covers, Span};
use crate::style::Antialias;

/// Agg's `iround` (half away from zero) with Rust's saturating conversion
/// for values beyond `i32`; the resampler's copy reproduces x86-64's
/// conversion instead (`resample::numeric::iround`).
fn iround(v: f64) -> i32 {
    (if v < 0.0 { v - 0.5 } else { v + 0.5 }) as i32
}

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

    /// The visible pixels `[start, end)` of the span at column `x` of row
    /// `y`, with the clip mask's values for them (if masked).
    fn run(&mut self, y: i32, x: i32, start: usize, end: usize) -> (&mut [[u8; 4]], Option<&[u8]>) {
        let first = y as usize * self.width + (x + start as i32) as usize;
        let pixels = &mut self.pixels.as_chunks_mut::<4>().0[first..first + end - start];
        (pixels, self.mask.map(|m| &m[first..first + end - start]))
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
        let (pixels, mask) = self.run(y, span.x, start, end);
        match (aa, span.covers) {
            (Antialias::Off, _) => blend_uniform(pixels, mask, color, 255),
            (Antialias::On, Covers::Solid { cover, .. }) => {
                blend_uniform(pixels, mask, color, cover);
            }
            (Antialias::On, Covers::Varying(covers)) => match mask {
                None => {
                    for (px, &cover) in pixels.iter_mut().zip(&covers[start..end]) {
                        blend_pixel(px, color, cover);
                    }
                }
                Some(mask) => {
                    for ((px, &cover), &m) in pixels.iter_mut().zip(&covers[start..end]).zip(mask) {
                        blend_pixel(px, color, combine_mask(cover, m));
                    }
                }
            },
        }
    }

    /// Whether every pixel of the inclusive rectangle `[x0, x1] × [y0,
    /// y1]` is visible and unmasked, so that spans within it can be
    /// blended by [`Surface::blend_solid_inside`].
    pub(crate) fn shows_all_of(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        self.mask.is_none()
            && self
                .bounds
                .is_some_and(|b| x0 >= b.x0() && y0 >= b.y0() && x1 < b.x1() && y1 < b.y1())
    }

    /// [`Surface::blend_solid`] with anti-aliasing for a span the caller
    /// knows lies in a rectangle [`Surface::shows_all_of`] accepted: the
    /// same blends, without clipping each span.
    pub(crate) fn blend_solid_inside(&mut self, y: i32, span: Span<'_>, color: Rgba8) {
        if color.a == 0 {
            return;
        }
        let first = y as usize * self.width + span.x as usize;
        let pixels = &mut self.pixels.as_chunks_mut::<4>().0[first..first + span.len()];
        match span.covers {
            Covers::Solid { cover, .. } => blend_run(pixels, color, cover),
            Covers::Varying(covers) => blend_covered(pixels, covers, color),
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
        let (pixels, mask) = self.run(y, span.x, start, end);
        let colors = colors.get(start..).unwrap_or_default();
        for (offset, (px, &color)) in pixels.iter_mut().zip(colors).enumerate() {
            let cover = span.cover_at(start + offset);
            let cover = mask.map_or(cover, |m| combine_mask(cover, m[offset]));
            blend_pixel(px, color, cover);
        }
    }
}

/// Blends `color` with one cover onto a run of pixels, scaled by the clip
/// mask's values when masked.
fn blend_uniform(pixels: &mut [[u8; 4]], mask: Option<&[u8]>, color: Rgba8, cover: u8) {
    match mask {
        None => blend_run(pixels, color, cover),
        Some(mask) => {
            for (px, &m) in pixels.iter_mut().zip(mask) {
                blend_pixel(px, color, combine_mask(cover, m));
            }
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
    fn spans_inside_blend_as_clipped_ones_do() {
        let covers = [0, 37, 128, 255, 200];
        let solid = Span {
            x: 1,
            covers: Covers::Solid { len: 3, cover: 90 },
        };
        let color = Rgba8::new(10, 120, 240, 200);
        let mut drawn = Vec::new();
        for inside in [false, true] {
            let mut p = Pixmap::new(6, 2).unwrap();
            p.fill(Rgba8::new(50, 60, 70, 255));
            let mut s = Surface::new(&mut p, Some(ClipBox::new(0, 0, 6, 2)), None).unwrap();
            assert!(s.shows_all_of(0, 0, 5, 1));
            for _ in 0..2 {
                if inside {
                    s.blend_solid_inside(0, span(1, &covers), color);
                    s.blend_solid_inside(1, solid, color);
                } else {
                    s.blend_solid(0, span(1, &covers), color, Antialias::On);
                    s.blend_solid(1, solid, color, Antialias::On);
                }
            }
            drawn.push(p.data().to_vec());
        }
        assert_eq!(drawn[0], drawn[1]);
        let mut p = Pixmap::new(6, 2).unwrap();
        let mask = Mask::from_vec(6, 2, vec![255; 12]).unwrap();
        let clipped = Surface::new(&mut p, Some(ClipBox::new(1, 0, 6, 2)), None).unwrap();
        assert!(!clipped.shows_all_of(0, 0, 5, 1));
        assert!(!clipped.shows_all_of(1, 0, 6, 1));
        assert!(clipped.shows_all_of(1, 0, 5, 1));
        let masked = Surface::new(&mut p, None, Some(&mask)).unwrap();
        assert!(!masked.shows_all_of(1, 0, 5, 1));
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
