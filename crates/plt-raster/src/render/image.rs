//! Compositing images and coverage bitmaps (matplotlib's `draw_image` and
//! `draw_text_image`).

use plt_core::{Affine2D, Point, Rgba};

use super::Surface;
use crate::clip::Clip;
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::raster::{Covers, Span};
use crate::style::Antialias;

/// Composites `image` (straight alpha) onto `target` with its top-left
/// corner at `(x, y)`, scaled by the global opacity `alpha ∈ [0, 1]`.
///
/// This reproduces both branches of matplotlib's `RendererAgg::draw_image`:
///
/// * without a mask, `alpha` becomes a cover `⌊255·alpha⌋` for every pixel
///   (`renderer_base::blend_from`); matplotlib clips with its clip box
///   *inclusively* here (see [`ClipBox::inclusive`](crate::ClipBox::inclusive));
/// * with a mask, each source alpha is scaled to `⌊a·alpha⌋` and the mask
///   value is the cover (`span_conv_alpha` through `pixfmt_amask_adaptor`).
///
/// matplotlib's Python wrapper always passes `alpha = 1` (image opacity is
/// baked into the pixels beforehand), so a faithful backend does the same.
pub fn draw_image(
    target: &mut Pixmap,
    image: &Pixmap,
    x: i32,
    y: i32,
    alpha: f64,
    clip: &Clip<'_>,
) -> Result<(), RasterError> {
    let alpha = if alpha.is_nan() {
        0.0
    } else {
        alpha.clamp(0.0, 1.0)
    };
    let mut surface = Surface::new(target, clip.rect, clip.mask)?;
    let width = image.width() as usize;
    let (convert_alpha, cover) = if clip.mask.is_some() {
        (true, 255)
    } else {
        (false, (alpha * 255.0) as u8)
    };
    let mut colors = Vec::with_capacity(width);
    for (row_index, row) in image.data().chunks_exact(width * 4).enumerate() {
        let Some(dy) = i32::try_from(row_index).ok().and_then(|r| y.checked_add(r)) else {
            break;
        };
        colors.clear();
        colors.extend(row.as_chunks::<4>().0.iter().map(|&[r, g, b, a]| {
            let a = if convert_alpha {
                (f64::from(a) * alpha) as u8
            } else {
                a
            };
            Rgba8::new(r, g, b, a)
        }));
        let span = Span {
            x,
            covers: Covers::Solid {
                len: image.width(),
                cover,
            },
        };
        surface.blend_colors(dy, span, &colors);
    }
    Ok(())
}

/// Blends a coverage bitmap (such as a rendered glyph) in `color` with its
/// top-left corner at `(x, y)` — matplotlib's `draw_text_image` at angle 0
/// (`blend_solid_hspan` with the bitmap as covers). matplotlib clips text
/// to the clip rectangle only; a mask in `clip` is honoured nevertheless.
pub fn draw_coverage(
    target: &mut Pixmap,
    coverage: &Mask,
    x: i32,
    y: i32,
    color: Rgba,
    clip: &Clip<'_>,
) -> Result<(), RasterError> {
    let mut surface = Surface::new(target, clip.rect, clip.mask)?;
    let color = Rgba8::from(color);
    for row_index in 0..coverage.height() {
        let Some(row) = coverage.row(row_index) else {
            break;
        };
        let Some(dy) = i32::try_from(row_index).ok().and_then(|r| y.checked_add(r)) else {
            break;
        };
        let span = Span {
            x,
            covers: Covers::Varying(row),
        };
        surface.blend_solid(dy, span, color, Antialias::On);
    }
    Ok(())
}

/// Agg's `iround`.
fn iround(v: f64) -> i32 {
    (if v < 0.0 { v - 0.5 } else { v + 0.5 }) as i32
}

/// Agg's forward-adjusted `dda2_line_interpolator`.
#[derive(Clone, Copy, Debug)]
struct Dda2 {
    cnt: i32,
    lft: i32,
    rem: i32,
    modulo: i32,
    y: i32,
}

impl Dda2 {
    fn new(y1: i32, y2: i32, count: i32) -> Self {
        let cnt = count.max(1);
        let diff = y2.wrapping_sub(y1);
        let mut d = Self {
            cnt,
            lft: diff / cnt,
            rem: diff % cnt,
            modulo: diff % cnt,
            y: y1,
        };
        if d.modulo <= 0 {
            d.modulo += cnt;
            d.rem += cnt;
            d.lft -= 1;
        }
        d.modulo -= cnt;
        d
    }

    fn step(&mut self) {
        self.modulo += self.rem;
        self.y = self.y.wrapping_add(self.lft);
        if self.modulo > 0 {
            self.modulo -= self.cnt;
            self.y = self.y.wrapping_add(1);
        }
    }
}

/// Samples a coverage bitmap through an inverse affine transform with
/// Agg's integer bilinear filter (`span_interpolator_linear` +
/// `span_image_filter_gray_bilinear` over `image_accessor_clip`) and turns
/// it into colour like matplotlib's `font_to_rgba`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CoverageSampler<'a> {
    src: &'a Mask,
    /// Inverse transform in Agg's order `[sx, shy, shx, sy, tx, ty]`.
    inv: [f64; 6],
    color: Rgba8,
}

impl<'a> CoverageSampler<'a> {
    /// `None` when `transform` is not invertible.
    pub(crate) fn new(src: &'a Mask, transform: &Affine2D, color: Rgba8) -> Option<Self> {
        let [sx, shy, shx, sy, tx, ty] = transform.values();
        let d = 1.0 / (sx * sy - shy * shx);
        if !d.is_finite() {
            return None;
        }
        // Agg's trans_affine::invert, operation for operation.
        let t0 = sy * d;
        let sy = sx * d;
        let shy = -shy * d;
        let shx = -shx * d;
        let t4 = -tx * t0 - ty * shx;
        let ty = -tx * shy - ty * sy;
        Some(Self {
            src,
            inv: [t0, shy, shx, sy, t4, ty],
            color,
        })
    }

    fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let [sx, shy, shx, sy, tx, ty] = self.inv;
        (x * sx + y * shx + tx, x * shy + y * sy + ty)
    }

    fn sample(&self, x: i32, y: i32) -> u32 {
        match (u32::try_from(x), u32::try_from(y)) {
            (Ok(x), Ok(y)) => self.src.get(x, y).map_or(0, u32::from),
            _ => 0,
        }
    }

    /// Fills `out` with the colours of pixels `x..x + out.len()` on row `y`.
    pub(crate) fn generate(&self, x: i32, y: i32, out: &mut [Rgba8]) {
        let len = i32::try_from(out.len()).unwrap_or(i32::MAX);
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let (sx1, sy1) = self.inverse(fx, fy);
        let (sx2, sy2) = self.inverse(fx + f64::from(len), fy);
        let mut li_x = Dda2::new(iround(sx1 * 256.0), iround(sx2 * 256.0), len);
        let mut li_y = Dda2::new(iround(sy1 * 256.0), iround(sy2 * 256.0), len);
        for px in out.iter_mut() {
            let x_hr = li_x.y.wrapping_sub(128);
            let y_hr = li_y.y.wrapping_sub(128);
            let (x_lr, y_lr) = (x_hr >> 8, y_hr >> 8);
            let (fx, fy) = ((x_hr & 255) as u32, (y_hr & 255) as u32);
            let fg = self.sample(x_lr, y_lr) * (256 - fx) * (256 - fy)
                + self.sample(x_lr + 1, y_lr) * fx * (256 - fy)
                + self.sample(x_lr, y_lr + 1) * (256 - fx) * fy
                + self.sample(x_lr + 1, y_lr + 1) * fx * fy;
            let v = fg >> 16;
            *px = self
                .color
                .with_alpha(((u32::from(self.color.a) * v) >> 8) as u8);
            li_x.step();
            li_y.step();
        }
    }

    /// The bitmap's outline in device space.
    pub(crate) fn outline(transform: &Affine2D, width: u32, height: u32) -> [Point; 4] {
        let (w, h) = (f64::from(width), f64::from(height));
        [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
            .map(|(x, y)| transform.transform_point(Point::new(x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::ClipBox;

    #[test]
    fn unmasked_image_uses_alpha_as_cover() {
        let mut target = Pixmap::filled(3, 3, Rgba8::new(255, 255, 255, 255)).unwrap();
        let img = Pixmap::filled(2, 2, Rgba8::new(0, 0, 0, 255)).unwrap();
        draw_image(&mut target, &img, 1, 1, 1.0, &Clip::NONE).unwrap();
        assert_eq!(target.pixel(2, 2), Some(Rgba8::new(0, 0, 0, 255)));
        assert_eq!(target.pixel(0, 0), Some(Rgba8::new(255, 255, 255, 255)));
        draw_image(&mut target, &img, -1, -1, 0.5, &Clip::NONE).unwrap();
        // cover = 127 -> alpha 127 over white.
        assert_eq!(target.pixel(0, 0), Some(Rgba8::new(128, 128, 128, 255)));
    }

    #[test]
    fn masked_image_scales_source_alpha() {
        let mut target = Pixmap::new(2, 1).unwrap();
        let img = Pixmap::filled(2, 1, Rgba8::new(10, 20, 30, 200)).unwrap();
        let mask = Mask::from_vec(2, 1, vec![255, 0]).unwrap();
        draw_image(&mut target, &img, 0, 0, 0.5, &Clip::NONE.with_mask(&mask)).unwrap();
        assert_eq!(target.pixel(0, 0), Some(Rgba8::new(10, 20, 30, 100)));
        assert_eq!(target.pixel(1, 0), Some(Rgba8::default()));
    }

    #[test]
    fn coverage_is_clipped() {
        let mut target = Pixmap::new(3, 1).unwrap();
        let glyph = Mask::from_vec(3, 1, vec![255, 255, 255]).unwrap();
        let clip = Clip::rect(ClipBox::new(1, 0, 2, 1));
        draw_coverage(&mut target, &glyph, 0, 0, Rgba::BLACK, &clip).unwrap();
        let alphas: Vec<u8> = target.data().chunks(4).map(|c| c[3]).collect();
        assert_eq!(alphas, vec![0, 255, 0]);
    }

    #[test]
    fn identity_sampling_reproduces_the_bitmap() {
        let src = Mask::from_vec(2, 1, vec![255, 100]).unwrap();
        let s =
            CoverageSampler::new(&src, &Affine2D::identity(), Rgba8::new(0, 0, 0, 255)).unwrap();
        let mut out = [Rgba8::default(); 3];
        s.generate(0, 0, &mut out);
        // font_to_rgba: (255 * v) >> 8.
        assert_eq!(out.map(|c| c.a), [254, 99, 0]);
        assert!(
            CoverageSampler::new(&src, &Affine2D::scaling(0.0, 1.0), Rgba8::default()).is_none()
        );
    }

    #[test]
    fn dda2_distributes_evenly() {
        let mut d = Dda2::new(0, 10, 4);
        let mut ys = vec![d.y];
        for _ in 0..4 {
            d.step();
            ys.push(d.y);
        }
        assert_eq!(ys.first(), Some(&0));
        assert_eq!(ys.last(), Some(&10));
        assert!(ys.windows(2).all(|w| w[1] - w[0] >= 2 && w[1] - w[0] <= 3));
    }
}
