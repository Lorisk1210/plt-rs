//! Resampling parameters: interpolation kernel, pixel transform and flags.

use std::fmt;
use std::str::FromStr;

use plt_core::Affine2D;

use super::error::ResampleError;
use super::grid::Grid;

/// The interpolation kernels of `matplotlib._image` (in matplotlib's order).
///
/// The names match matplotlib's `interpolation=` strings; matplotlib's
/// `'antialiased'`, `'auto'` and `'none'` are resolved by the caller before
/// resampling and are therefore not variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interpolation {
    /// Nearest-neighbour sampling (no filtering).
    Nearest,
    /// Tent filter, radius 1.
    Bilinear,
    /// Cubic B-spline-like filter, radius 2.
    Bicubic,
    /// 16-tap spline, radius 2.
    Spline16,
    /// 36-tap spline, radius 3.
    Spline36,
    /// Hanning window, radius 1.
    Hanning,
    /// Hamming window, radius 1.
    Hamming,
    /// Hermite cubic, radius 1.
    Hermite,
    /// Kaiser window (`b = 6.33`), radius 1.
    Kaiser,
    /// Quadratic B-spline, radius 1.5.
    Quadric,
    /// Catmull–Rom spline, radius 2.
    Catrom,
    /// Gaussian, radius 2.
    Gaussian,
    /// Jinc (Bessel) filter, radius 3.2383.
    Bessel,
    /// Mitchell–Netravali (`B = C = 1/3`), radius 2.
    Mitchell,
    /// Unwindowed sinc; radius from [`ResampleParams::radius`] (at least 2).
    Sinc,
    /// Lanczos-windowed sinc; radius from [`ResampleParams::radius`] (at least 2).
    Lanczos,
    /// Blackman-windowed sinc; radius from [`ResampleParams::radius`] (at least 2).
    Blackman,
}

impl Interpolation {
    /// Every kernel, in the order of matplotlib's `_image` constants.
    pub const ALL: [Self; 17] = [
        Self::Nearest,
        Self::Bilinear,
        Self::Bicubic,
        Self::Spline16,
        Self::Spline36,
        Self::Hanning,
        Self::Hamming,
        Self::Hermite,
        Self::Kaiser,
        Self::Quadric,
        Self::Catrom,
        Self::Gaussian,
        Self::Bessel,
        Self::Mitchell,
        Self::Sinc,
        Self::Lanczos,
        Self::Blackman,
    ];

    /// matplotlib's name for the kernel (e.g. `"spline16"`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nearest => "nearest",
            Self::Bilinear => "bilinear",
            Self::Bicubic => "bicubic",
            Self::Spline16 => "spline16",
            Self::Spline36 => "spline36",
            Self::Hanning => "hanning",
            Self::Hamming => "hamming",
            Self::Hermite => "hermite",
            Self::Kaiser => "kaiser",
            Self::Quadric => "quadric",
            Self::Catrom => "catrom",
            Self::Gaussian => "gaussian",
            Self::Bessel => "bessel",
            Self::Mitchell => "mitchell",
            Self::Sinc => "sinc",
            Self::Lanczos => "lanczos",
            Self::Blackman => "blackman",
        }
    }

    /// Whether [`ResampleParams::radius`] affects this kernel.
    pub const fn uses_radius(self) -> bool {
        matches!(self, Self::Sinc | Self::Lanczos | Self::Blackman)
    }
}

impl fmt::Display for Interpolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Interpolation {
    type Err = ResampleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|i| i.name() == s)
            .ok_or_else(|| ResampleError::UnknownInterpolation(s.to_owned()))
    }
}

/// How source pixel coordinates map to destination pixel coordinates.
///
/// Coordinates are in array-index space: pixel `(i, j)` (column `i`, row
/// `j`) covers `[i, i + 1) × [j, j + 1)`, and row 0 is the first row in
/// memory. Any vertical flip is part of the transform, exactly as with the
/// transform matplotlib passes to `_image.resample`.
#[derive(Clone, Debug, PartialEq)]
pub enum PixelTransform {
    /// An affine map from source to destination pixel coordinates.
    Affine(Affine2D),
    /// A non-affine map given as a lookup mesh with the destination's
    /// dimensions: entry `(i, j)` holds the source coordinates of the centre
    /// `(i + 0.5, j + 0.5)` of destination pixel `(i, j)` — what matplotlib's
    /// `_get_transform_mesh` computes from the inverse transform.
    Mesh(Grid<[f64; 2]>),
}

impl From<Affine2D> for PixelTransform {
    fn from(affine: Affine2D) -> Self {
        Self::Affine(affine)
    }
}

/// Parameters of [`resample`](super::resample), mirroring the arguments of
/// `matplotlib._image.resample`.
#[derive(Clone, Debug, PartialEq)]
pub struct ResampleParams {
    /// Source-to-destination pixel transform.
    pub transform: PixelTransform,
    /// Interpolation kernel.
    pub interpolation: Interpolation,
    /// For affine transforms, `true` selects Agg's resampling generator,
    /// whose kernel footprint grows with the downsampling factor
    /// (anti-aliasing); `false` applies the kernel at its natural size.
    /// Non-affine transforms always use the latter.
    pub resample: bool,
    /// Opacity multiplier in `[0, 1]` applied to every generated pixel.
    pub alpha: f64,
    /// Whether the kernel weights are normalised to sum to one per phase
    /// (matplotlib's `filternorm`).
    pub norm: bool,
    /// Kernel radius for [`Interpolation::Sinc`], [`Interpolation::Lanczos`]
    /// and [`Interpolation::Blackman`] (matplotlib's `filterrad`).
    pub radius: f64,
}

impl ResampleParams {
    /// Parameters with `_image.resample`'s defaults: nearest-neighbour,
    /// `resample = false`, `alpha = 1`, `norm = false`, `radius = 1`.
    pub fn new(transform: impl Into<PixelTransform>) -> Self {
        Self {
            transform: transform.into(),
            interpolation: Interpolation::Nearest,
            resample: false,
            alpha: 1.0,
            norm: false,
            radius: 1.0,
        }
    }

    /// Sets [`interpolation`](Self::interpolation).
    #[must_use]
    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Sets [`resample`](Self::resample).
    #[must_use]
    pub fn with_resample(mut self, resample: bool) -> Self {
        self.resample = resample;
        self
    }

    /// Sets [`alpha`](Self::alpha).
    #[must_use]
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// Sets [`norm`](Self::norm).
    #[must_use]
    pub fn with_norm(mut self, norm: bool) -> Self {
        self.norm = norm;
        self
    }

    /// Sets [`radius`](Self::radius).
    #[must_use]
    pub fn with_radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for interpolation in Interpolation::ALL {
            assert_eq!(
                interpolation.name().parse::<Interpolation>(),
                Ok(interpolation)
            );
            assert_eq!(interpolation.to_string(), interpolation.name());
        }
        assert_eq!(
            "antialiased".parse::<Interpolation>(),
            Err(ResampleError::UnknownInterpolation("antialiased".into()))
        );
    }

    #[test]
    fn builder_sets_fields() {
        let p = ResampleParams::new(Affine2D::identity())
            .with_interpolation(Interpolation::Lanczos)
            .with_resample(true)
            .with_alpha(0.5)
            .with_norm(true)
            .with_radius(3.0);
        assert_eq!(p.interpolation, Interpolation::Lanczos);
        assert!(p.resample && p.norm);
        assert_eq!((p.alpha, p.radius), (0.5, 3.0));
        assert!(Interpolation::Lanczos.uses_radius() && !Interpolation::Hanning.uses_radius());
    }
}
