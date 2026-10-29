//! 8-bit colours and the straight-alpha compositing arithmetic of
//! matplotlib's Agg backend.

use pltrs_core::Rgba;

/// An 8-bit-per-channel colour with straight (non-premultiplied) alpha,
/// Agg's `rgba8`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Opacity: 0 is fully transparent, 255 fully opaque.
    pub a: u8,
}

impl Rgba8 {
    /// Fully transparent white, the initial content of matplotlib's canvas.
    pub const TRANSPARENT_WHITE: Self = Self::new(255, 255, 255, 0);

    /// Creates a colour from its channels.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// The channels as `[r, g, b, a]`.
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Creates a colour from `[r, g, b, a]`.
    pub const fn from_array([r, g, b, a]: [u8; 4]) -> Self {
        Self::new(r, g, b, a)
    }

    /// The same colour with a different alpha.
    #[must_use]
    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

impl From<Rgba> for Rgba8 {
    /// Quantises with round-half-up, like Agg's `rgba8(rgba)` conversion.
    fn from(c: Rgba) -> Self {
        Self::from_array(c.to_rgba8())
    }
}

impl From<[u8; 4]> for Rgba8 {
    fn from(c: [u8; 4]) -> Self {
        Self::from_array(c)
    }
}

/// Agg's exact 8-bit fixed-point product `a·b/255` with rounding
/// (`rgba8::multiply`, also used as `mult_cover`).
pub(crate) fn multiply(a: u8, b: u8) -> u8 {
    let t = u32::from(a) * u32::from(b) + 128;
    (((t >> 8) + t) >> 8) as u8
}

/// Scales an anti-aliasing cover by an alpha-mask value, as Agg's
/// `alpha_mask_u8::combine_hspan` does: `(255 + cover·mask) >> 8`.
pub(crate) fn combine_mask(cover: u8, mask: u8) -> u8 {
    ((255 + u32::from(cover) * u32::from(mask)) >> 8) as u8
}

/// Composites `color` with coverage `cover` onto one RGBA pixel.
///
/// This is Agg's `copy_or_blend_pix` with matplotlib's
/// `fixed_blender_rgba_plain`: an opaque colour at full cover is copied;
/// otherwise the effective alpha `multiply(color.a, cover)` is blended in
/// straight-alpha space with integer arithmetic (see [`blend_translucent`]).
#[inline]
pub(crate) fn blend_pixel(dst: &mut [u8; 4], color: Rgba8, cover: u8) {
    match multiply(color.a, cover) {
        0 => {}
        255 => *dst = color.to_array(),
        alpha => blend_translucent(dst, color, alpha),
    }
}

/// Blends `color` at the effective alpha `alpha` (neither 0 nor 255) onto
/// one RGBA pixel.
///
/// Each channel is `(((src << 8) - premul) · alpha + (premul << 8)) / a`
/// with `premul` the destination channel times its alpha and `a` the new
/// alpha in 8.8 fixed point. Over an opaque pixel, the usual case, the
/// quotient comes from [`divide_over_opaque`] instead of a division.
#[inline]
fn blend_translucent(dst: &mut [u8; 4], color: Rgba8, alpha: u8) {
    let dst_alpha = u32::from(dst[3]);
    let alpha_u32 = u32::from(alpha);
    let new_alpha = ((alpha_u32 + dst_alpha) << 8) - alpha_u32 * dst_alpha;
    // The numerator rearranged to `src·256·alpha + premul·(256 - alpha)`:
    // the same value, with no negative intermediate.
    let numerator = |src: u8, dst: u8| {
        (u32::from(src) << 8) * alpha_u32 + u32::from(dst) * dst_alpha * (256 - alpha_u32)
    };
    let numerators = [(color.r, dst[0]), (color.g, dst[1]), (color.b, dst[2])]
        .map(|(src, dst)| numerator(src, dst));
    let [r, g, b] = if dst_alpha == 255 {
        numerators.map(|n| divide_over_opaque(n, alpha))
    } else {
        numerators.map(|n| n / new_alpha)
    };
    // Quotients never exceed 255: the numerator is at most 255 times the
    // new alpha.
    *dst = [r as u8, g as u8, b as u8, (new_alpha >> 8) as u8];
}

/// The shift of the fixed-point reciprocals in [`OPAQUE_RECIPROCALS`].
const RECIPROCAL_SHIFT: u32 = 40;

/// `ceil(2^40 / (65280 + alpha))`, the reciprocal of the new 8.8 alpha
/// when blending with `alpha` over an opaque pixel.
const OPAQUE_RECIPROCALS: [u64; 256] = {
    let mut table = [0; 256];
    let mut alpha = 0;
    while alpha < 256 {
        table[alpha] = (1_u64 << RECIPROCAL_SHIFT).div_ceil(65_280 + alpha as u64);
        alpha += 1;
    }
    table
};

/// `numerator / (65280 + alpha)`, rounded down, for the numerators of
/// [`blend_pixel`] (below `2^24`), as a multiplication by a rounded-up
/// reciprocal. With a divisor `d` of 16 bits and the reciprocal
/// `m = ⌈2^40 / d⌉ = (2^40 + e) / d` (`0 ≤ e < d`), `n·m / 2^40` exceeds
/// `n / d` by less than `n / 2^40 < 2^-16 < 1/d`, which never reaches the
/// next integer.
fn divide_over_opaque(numerator: u32, alpha: u8) -> u32 {
    ((u64::from(numerator) * OPAQUE_RECIPROCALS[usize::from(alpha)]) >> RECIPROCAL_SHIFT) as u32
}

/// Blends one colour over a run of pixels with a constant cover, using a
/// straight copy when the result is fully opaque.
pub(crate) fn blend_run(dst: &mut [[u8; 4]], color: Rgba8, cover: u8) {
    match multiply(color.a, cover) {
        0 => {}
        255 => dst.fill(color.to_array()),
        alpha => {
            for px in dst {
                blend_translucent(px, color, alpha);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply_is_exact_rounded_product() {
        for a in 0..=255u8 {
            for b in [0u8, 1, 127, 128, 254, 255] {
                let exact = (f64::from(a) * f64::from(b) / 255.0).round();
                assert_eq!(f64::from(multiply(a, b)), exact, "{a}*{b}");
            }
        }
        assert_eq!(multiply(255, 255), 255);
    }

    #[test]
    fn combine_mask_identity_at_full_cover() {
        for m in 0..=255u8 {
            assert_eq!(combine_mask(255, m), m);
        }
        assert_eq!(combine_mask(0, 255), 0);
    }

    #[test]
    fn opaque_full_cover_copies() {
        let mut px = [10, 20, 30, 40];
        blend_pixel(&mut px, Rgba8::new(1, 2, 3, 255), 255);
        assert_eq!(px, [1, 2, 3, 255]);
    }

    #[test]
    fn blend_onto_transparent_keeps_source_colour() {
        let mut px = Rgba8::TRANSPARENT_WHITE.to_array();
        blend_pixel(&mut px, Rgba8::new(255, 0, 0, 128), 255);
        assert_eq!(px, [255, 0, 0, 128]);
    }

    #[test]
    fn blend_matches_matplotlib_reference_values() {
        // Half-transparent blue over opaque white (checked against
        // matplotlib's RendererAgg).
        let mut px = [255, 255, 255, 255];
        blend_pixel(&mut px, Rgba8::new(0, 0, 255, 128), 255);
        assert_eq!(px, [127, 127, 255, 255]);
        // Zero effective alpha leaves the pixel untouched.
        let mut px = [1, 2, 3, 4];
        blend_pixel(&mut px, Rgba8::new(9, 9, 9, 1), 100);
        assert_eq!(px, [1, 2, 3, 4]);
    }

    /// `fixed_blender_rgba_plain` as written in matplotlib, with its
    /// signed intermediates and divisions.
    fn agg_blend(dst: [u8; 4], color: Rgba8, alpha: u8) -> [u8; 4] {
        let alpha = i32::from(alpha);
        let a = i32::from(dst[3]);
        let premul = |channel: u8| i32::from(channel) * a;
        let a = ((alpha + a) << 8) - alpha * a;
        let channel = |src: u8, premul: i32| {
            ((((i32::from(src) << 8) - premul) * alpha + (premul << 8)) / a) as u8
        };
        [
            channel(color.r, premul(dst[0])),
            channel(color.g, premul(dst[1])),
            channel(color.b, premul(dst[2])),
            (a >> 8) as u8,
        ]
    }

    #[test]
    fn blending_over_opaque_pixels_matches_agg_exactly() {
        for alpha in 1..=254_u8 {
            for src in 0..=255_u8 {
                for dst in 0..=255_u8 {
                    let color = Rgba8::new(src, dst, 255 - src, alpha);
                    let pixel = [dst, src, dst / 2, 255];
                    let mut blended = pixel;
                    blend_pixel(&mut blended, color, 255);
                    assert_eq!(
                        blended,
                        agg_blend(pixel, color, alpha),
                        "{color:?} {pixel:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn blending_over_translucent_pixels_matches_agg_exactly() {
        for alpha in (1..=254_u8).step_by(3) {
            for dst_alpha in 0..=254_u8 {
                for value in (0..=255_u8).step_by(5) {
                    let color = Rgba8::new(value, 255 - value, value / 3, alpha);
                    let pixel = [255 - value, value, 128, dst_alpha];
                    let mut blended = pixel;
                    blend_pixel(&mut blended, color, 255);
                    assert_eq!(
                        blended,
                        agg_blend(pixel, color, alpha),
                        "{color:?} {pixel:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn opaque_reciprocals_divide_exactly_around_every_multiple() {
        // The quotient changes only at multiples of the divisor: check
        // both sides of each one in the numerators' range.
        for alpha in 1..=254_u8 {
            let divisor = 65_280 + u32::from(alpha);
            for quotient in 0..=256 {
                for numerator in [quotient * divisor, (quotient * divisor).saturating_sub(1)] {
                    assert_eq!(divide_over_opaque(numerator, alpha), numerator / divisor);
                }
            }
            let largest = (1 << 24) - 1;
            assert_eq!(divide_over_opaque(largest, alpha), largest / divisor);
        }
    }

    #[test]
    fn conversion_rounds_half_up() {
        assert_eq!(
            Rgba8::from(Rgba::new(0.5, 1.0, 0.0, 0.25)),
            Rgba8::new(128, 255, 0, 64)
        );
    }
}
