//! 8-bit colours and the straight-alpha compositing arithmetic of
//! matplotlib's Agg backend.

use plt_core::Rgba;

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
/// straight-alpha space with integer arithmetic.
pub(crate) fn blend_pixel(dst: &mut [u8], color: Rgba8, cover: u8) {
    let alpha = multiply(color.a, cover);
    if alpha == 0 {
        return;
    }
    if alpha == 255 {
        dst[..4].copy_from_slice(&color.to_array());
        return;
    }
    let alpha = i32::from(alpha);
    let a = i32::from(dst[3]);
    let r = i32::from(dst[0]) * a;
    let g = i32::from(dst[1]) * a;
    let b = i32::from(dst[2]) * a;
    let a = ((alpha + a) << 8) - alpha * a;
    let channel = |src: u8, premul: i32| {
        ((((i32::from(src) << 8) - premul) * alpha + (premul << 8)) / a) as u8
    };
    dst[0] = channel(color.r, r);
    dst[1] = channel(color.g, g);
    dst[2] = channel(color.b, b);
    dst[3] = (a >> 8) as u8;
}

/// Blends one colour over a run of pixels with a constant cover, using a
/// straight copy when the result is fully opaque.
pub(crate) fn blend_run(dst: &mut [u8], color: Rgba8, cover: u8) {
    if multiply(color.a, cover) == 255 {
        dst.as_chunks_mut::<4>().0.fill(color.to_array());
    } else {
        for px in dst.as_chunks_mut::<4>().0 {
            blend_pixel(px, color, cover);
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

    #[test]
    fn conversion_rounds_half_up() {
        assert_eq!(
            Rgba8::from(Rgba::new(0.5, 1.0, 0.0, 0.25)),
            Rgba8::new(128, 255, 0, 64)
        );
    }
}
