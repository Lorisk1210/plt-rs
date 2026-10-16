//! Pixel formats and Agg's per-format arithmetic.
//!
//! matplotlib instantiates its resampler for Agg colour types `gray8`,
//! `gray32`, `gray64` (single-channel data) and `rgba8`, `rgba32`, `rgba64`.
//! Each type has its own accumulator width, clamping and blending rules;
//! [`ops::Channel`] captures the scalar arithmetic and [`ops::PixelOps`]
//! the gray/RGBA layouts built on it.

use std::ops::{Add, Mul, Sub};

use super::numeric::FILTER_SHIFT;

/// A pixel format accepted by [`resample`](super::resample).
///
/// Implemented for single-channel data — `f32`, `f64` (and `u8`), as used
/// by matplotlib's `interpolation_stage='data'` — and for RGBA pixels
/// `[u8; 4]`, `[f32; 4]` and `[f64; 4]` (`interpolation_stage='rgba'`).
/// RGBA channels are in R, G, B, A order; float channels are nominally in
/// `[0, 1]` and `u8` channels in `[0, 255]`. The trait is sealed.
pub trait Pixel: Copy + Default + Send + Sync + 'static + ops::PixelOps {}

impl<T: ops::Channel + Send + Sync + 'static> Pixel for T {}
impl<T: ops::Channel + Send + Sync + 'static> Pixel for [T; 4] {}

/// A single-channel span colour: Agg's `grayN` value plus coverage alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Gray<T> {
    v: T,
    a: T,
}

pub(super) mod ops {
    use super::Gray;

    /// The scalar arithmetic of one Agg colour channel type.
    pub trait Channel: Copy + Default + PartialOrd {
        /// Agg's `long_type`: the filter accumulator.
        type Acc: Copy + PartialOrd;
        /// `empty_value()`.
        const ZERO: Self;
        /// `full_value()`.
        const FULL: Self;
        /// Zero accumulator.
        const ACC_ZERO: Self::Acc;
        /// `full_value()` as an accumulator.
        const ACC_FULL: Self::Acc;
        /// `value * weight` with Agg's operand types.
        fn weighted(self, weight: i32) -> Self::Acc;
        /// Accumulator addition.
        fn add(acc: Self::Acc, term: Self::Acc) -> Self::Acc;
        /// `downshift(acc, image_filter_shift)`.
        fn downshift(acc: Self::Acc) -> Self::Acc;
        /// `acc /= total_weight`.
        fn divide(acc: Self::Acc, total_weight: i32) -> Self::Acc;
        /// `(value_type)acc`.
        fn from_acc(acc: Self::Acc) -> Self;
        /// `is_transparent()` applied to an alpha value.
        fn is_transparent(self) -> bool;
        /// `is_opaque()` applied to an alpha value.
        fn is_opaque(self) -> bool;
        /// `span_conv_alpha`: `(value_type)((calc_type)a * alpha)`.
        fn scale(self, alpha: f64) -> Self;
        /// `mult_cover(value, cover)`.
        fn mult_cover(self, cover: u8) -> Self;
        /// `lerp(p, q, a)`.
        fn lerp(p: Self, q: Self, a: Self) -> Self;
        /// The RGBA blender matplotlib pairs with this channel type.
        fn blend_rgba(dst: &mut [Self; 4], src: [Self; 4], cover: u8);
    }

    /// The span-generation and blending steps of one pixel layout.
    pub trait PixelOps: Copy {
        /// Agg's `color_type` as produced by the span generators.
        type Color: Copy + Default;
        /// One accumulator per channel.
        type Accum: Copy;
        /// All-zero accumulator.
        const ACCUM_ZERO: Self::Accum;
        /// The clip accessor's background pixel (`no_color()`).
        const BACKGROUND: Self;
        /// Adds `weight × self` to `acc`.
        fn accumulate(self, acc: &mut Self::Accum, weight: i32);
        /// The span colour of a nearest-neighbour sample.
        fn nearest(self) -> Self::Color;
        /// Final colour of `span_image_filter_*` (fixed-point downshift).
        fn filtered(acc: Self::Accum) -> Self::Color;
        /// Final colour of `span_image_resample_*_affine` (weight division).
        fn resampled(acc: Self::Accum, total_weight: i32) -> Self::Color;
        /// `span_conv_alpha`: scales the colour's alpha.
        fn scale_alpha(color: &mut Self::Color, alpha: f64);
        /// `pixfmt::copy_or_blend_pix` with a coverage value.
        fn blend(&mut self, color: Self::Color, cover: u8);
    }

    impl<T: Channel> PixelOps for T {
        type Color = Gray<T>;
        type Accum = T::Acc;
        const ACCUM_ZERO: T::Acc = T::ACC_ZERO;
        const BACKGROUND: Self = T::ZERO;

        fn accumulate(self, acc: &mut T::Acc, weight: i32) {
            *acc = T::add(*acc, self.weighted(weight));
        }

        fn nearest(self) -> Gray<T> {
            Gray { v: self, a: T::FULL }
        }

        // matplotlib defines MPL_DISABLE_AGG_GRAY_CLIPPING: no clamping here.
        fn filtered(acc: T::Acc) -> Gray<T> {
            T::from_acc(T::downshift(acc)).nearest()
        }

        fn resampled(acc: T::Acc, total_weight: i32) -> Gray<T> {
            T::from_acc(T::divide(acc, total_weight)).nearest()
        }

        fn scale_alpha(color: &mut Gray<T>, alpha: f64) {
            color.a = color.a.scale(alpha);
        }

        fn blend(&mut self, color: Gray<T>, cover: u8) {
            if color.a.is_transparent() {
                return;
            }
            *self = if color.a.is_opaque() && cover == u8::MAX {
                color.v
            } else {
                T::lerp(*self, color.v, color.a.mult_cover(cover))
            };
        }
    }

    impl<T: Channel> PixelOps for [T; 4] {
        type Color = [T; 4];
        type Accum = [T::Acc; 4];
        const ACCUM_ZERO: [T::Acc; 4] = [T::ACC_ZERO; 4];
        const BACKGROUND: Self = [T::ZERO; 4];

        fn accumulate(self, acc: &mut [T::Acc; 4], weight: i32) {
            for (sum, channel) in acc.iter_mut().zip(self) {
                *sum = T::add(*sum, channel.weighted(weight));
            }
        }

        fn nearest(self) -> [T; 4] {
            self
        }

        fn filtered(acc: [T::Acc; 4]) -> [T; 4] {
            clamp_rgba::<T>(acc.map(T::downshift))
        }

        fn resampled(acc: [T::Acc; 4], total_weight: i32) -> [T; 4] {
            clamp_rgba::<T>(acc.map(|sum| T::divide(sum, total_weight)))
        }

        fn scale_alpha(color: &mut [T; 4], alpha: f64) {
            color[3] = color[3].scale(alpha);
        }

        fn blend(&mut self, color: [T; 4], cover: u8) {
            if color[3].is_transparent() {
                return;
            }
            if color[3].is_opaque() && cover == u8::MAX {
                *self = color;
            } else {
                T::blend_rgba(self, color, cover);
            }
        }
    }

    /// The RGBA span generators' clamping: channels to `[0, ∞)`, alpha to
    /// `full_value()`, colour channels to alpha (premultiplied range).
    fn clamp_rgba<T: Channel>(mut acc: [T::Acc; 4]) -> [T; 4] {
        for sum in &mut acc {
            if *sum < T::ACC_ZERO {
                *sum = T::ACC_ZERO;
            }
        }
        if acc[3] > T::ACC_FULL {
            acc[3] = T::ACC_FULL;
        }
        let alpha = acc[3];
        for sum in &mut acc[..3] {
            if *sum > alpha {
                *sum = alpha;
            }
        }
        acc.map(T::from_acc)
    }
}

/// `rgba8T::multiply`: exact fixed-point `a·b/255`.
fn multiply_u8(a: u8, b: u8) -> u8 {
    let t = u32::from(a) * u32::from(b) + 128;
    (((t >> 8) + t) >> 8) as u8
}

impl ops::Channel for u8 {
    type Acc = i32;
    const ZERO: Self = 0;
    const FULL: Self = u8::MAX;
    const ACC_ZERO: i32 = 0;
    const ACC_FULL: i32 = 255;

    fn weighted(self, weight: i32) -> i32 {
        i32::from(self).wrapping_mul(weight)
    }

    fn add(acc: i32, term: i32) -> i32 {
        acc.wrapping_add(term)
    }

    fn downshift(acc: i32) -> i32 {
        acc >> FILTER_SHIFT
    }

    fn divide(acc: i32, total_weight: i32) -> i32 {
        // Agg would fault on a zero total; it cannot occur for real kernels.
        acc.checked_div(total_weight).unwrap_or(0)
    }

    fn from_acc(acc: i32) -> Self {
        acc as u8
    }

    fn is_transparent(self) -> bool {
        self == 0
    }

    fn is_opaque(self) -> bool {
        self == u8::MAX
    }

    fn scale(self, alpha: f64) -> Self {
        (f64::from(self) * alpha) as u8
    }

    fn mult_cover(self, cover: u8) -> Self {
        multiply_u8(self, cover)
    }

    fn lerp(p: Self, q: Self, a: Self) -> Self {
        let (p, q) = (i32::from(p), i32::from(q));
        let t = (q - p) * i32::from(a) + 128 - i32::from(p > q);
        (p + (((t >> 8) + t) >> 8)) as u8
    }

    /// matplotlib's `fixed_blender_rgba_plain` (non-premultiplied).
    fn blend_rgba(dst: &mut [Self; 4], src: [Self; 4], cover: u8) {
        let alpha = u32::from(multiply_u8(src[3], cover));
        if alpha == 0 {
            return;
        }
        let dst_alpha = u32::from(dst[3]);
        let total = ((alpha + dst_alpha) << 8) - alpha * dst_alpha;
        dst[3] = (total >> 8) as u8;
        for (d, s) in dst[..3].iter_mut().zip(src) {
            let premul = u32::from(*d) * dst_alpha;
            let blended = (u32::from(s) << 8)
                .wrapping_sub(premul)
                .wrapping_mul(alpha)
                .wrapping_add(premul << 8);
            *d = (blended / total) as u8;
        }
    }
}

impl ops::Channel for f32 {
    type Acc = f64;
    const ZERO: Self = 0.0;
    const FULL: Self = 1.0;
    const ACC_ZERO: f64 = 0.0;
    const ACC_FULL: f64 = 1.0;

    /// `float * int` is evaluated in single precision, then widened.
    fn weighted(self, weight: i32) -> f64 {
        f64::from(self * weight as f32)
    }

    fn add(acc: f64, term: f64) -> f64 {
        acc + term
    }

    fn downshift(acc: f64) -> f64 {
        acc / f64::from(1_u32 << FILTER_SHIFT)
    }

    fn divide(acc: f64, total_weight: i32) -> f64 {
        acc / f64::from(total_weight)
    }

    fn from_acc(acc: f64) -> Self {
        acc as f32
    }

    fn is_transparent(self) -> bool {
        self <= 0.0
    }

    fn is_opaque(self) -> bool {
        self >= 1.0
    }

    fn scale(self, alpha: f64) -> Self {
        (f64::from(self) * alpha) as f32
    }

    fn mult_cover(self, cover: u8) -> Self {
        self * f32::from(cover) / 255.0
    }

    fn lerp(p: Self, q: Self, a: Self) -> Self {
        lerp_float(p, q, a)
    }

    fn blend_rgba(dst: &mut [Self; 4], src: [Self; 4], cover: u8) {
        blend_premultiplied(dst, src, cover);
    }
}

impl ops::Channel for f64 {
    type Acc = f64;
    const ZERO: Self = 0.0;
    const FULL: Self = 1.0;
    const ACC_ZERO: f64 = 0.0;
    const ACC_FULL: f64 = 1.0;

    fn weighted(self, weight: i32) -> f64 {
        self * f64::from(weight)
    }

    fn add(acc: f64, term: f64) -> f64 {
        acc + term
    }

    fn downshift(acc: f64) -> f64 {
        acc / f64::from(1_u32 << FILTER_SHIFT)
    }

    fn divide(acc: f64, total_weight: i32) -> f64 {
        acc / f64::from(total_weight)
    }

    fn from_acc(acc: f64) -> Self {
        acc
    }

    fn is_transparent(self) -> bool {
        self <= 0.0
    }

    fn is_opaque(self) -> bool {
        self >= 1.0
    }

    fn scale(self, alpha: f64) -> Self {
        self * alpha
    }

    fn mult_cover(self, cover: u8) -> Self {
        self * f64::from(cover) / 255.0
    }

    fn lerp(p: Self, q: Self, a: Self) -> Self {
        lerp_float(p, q, a)
    }

    fn blend_rgba(dst: &mut [Self; 4], src: [Self; 4], cover: u8) {
        blend_premultiplied(dst, src, cover);
    }
}

/// Floating-point `lerp`: `(1 - a)·p + a·q`.
fn lerp_float<T>(p: T, q: T, a: T) -> T
where
    T: ops::Channel + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    (T::FULL - a) * p + a * q
}

/// Agg's `blender_rgba_pre`, which matplotlib uses for float RGBA.
fn blend_premultiplied<T>(dst: &mut [T; 4], src: [T; 4], cover: u8)
where
    T: ops::Channel + Add<Output = T> + Sub<Output = T> + Mul<Output = T>,
{
    let src = src.map(|c| c.mult_cover(cover));
    let keep = T::FULL - src[3];
    for (d, s) in dst.iter_mut().zip(src) {
        *d = keep * *d + s;
    }
}

#[cfg(test)]
mod tests {
    use super::ops::{Channel, PixelOps};

    #[test]
    fn u8_fixed_point_matches_agg() {
        assert_eq!(u8::mult_cover(255, 255), 255);
        assert_eq!(u8::mult_cover(255, 128), 128);
        assert_eq!(u8::mult_cover(0, 200), 0);
        assert_eq!(u8::lerp(0, 255, 128), 128);
        assert_eq!(u8::lerp(200, 100, 255), 100);
        assert_eq!(u8::scale(255, 0.5), 127);
    }

    #[test]
    fn rgba_clamps_to_premultiplied_range() {
        let acc = [300 << 14, -5 << 14, 100 << 14, 400 << 14];
        assert_eq!(<[u8; 4]>::filtered(acc), [255, 0, 100, 255]);
        let acc = [0.9, 0.2, -0.1, 0.5].map(|v: f64| v * 16384.0);
        assert_eq!(<[f64; 4]>::filtered(acc), [0.5, 0.2, 0.0, 0.5]);
        assert_eq!(<[u8; 4]>::resampled([10, 20, 30, 40], 0), [0; 4]);
    }

    #[test]
    fn gray_is_not_clamped_and_keeps_nan() {
        let c = f64::filtered(-3.0 * 16384.0);
        let mut px = 7.0;
        px.blend(c, 255);
        assert_eq!(px, -3.0);
        px.blend(f64::NAN.nearest(), 100);
        assert!(px.is_nan());
    }

    #[test]
    fn gray_blending_respects_alpha_and_cover() {
        let mut c = 1.0_f32.nearest();
        f32::scale_alpha(&mut c, 0.0);
        let mut px = 0.25_f32;
        px.blend(c, 255);
        assert_eq!(px, 0.25);
        let mut px = 0.0_f32;
        px.blend(1.0_f32.nearest(), 51);
        assert!((px - 0.2).abs() < 1e-6);
    }

    #[test]
    fn rgba_blending() {
        let mut px = [10_u8, 20, 30, 40];
        px.blend([0, 0, 0, 0], 255);
        assert_eq!(px, [10, 20, 30, 40]);
        px.blend([1, 2, 3, 255], 255);
        assert_eq!(px, [1, 2, 3, 255]);
        let mut px = [0_u8; 4];
        px.blend([200, 100, 50, 255], 128);
        assert_eq!(px, [200, 100, 50, 128]);

        let mut px = [0.5_f64, 0.5, 0.5, 1.0];
        px.blend([0.25, 0.0, 0.5, 0.5], 255);
        assert_eq!(px, [0.5, 0.25, 0.75, 1.0]);
    }
}
