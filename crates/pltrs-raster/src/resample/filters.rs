//! Agg's interpolation kernels (`agg_image_filters.h`).
//!
//! Each kernel is evaluated for `x >= 0` only; the weight table mirrors it.
//! Expressions keep Agg's operation order so that the tabulated weights
//! round identically.

use std::f64::consts::PI;

use super::params::Interpolation;

/// The Kaiser window's `b` parameter (`image_filter_kaiser` default).
const KAISER_B: f64 = 6.33;
/// Mitchell–Netravali `B` and `C` (`image_filter_mitchell` defaults).
const MITCHELL_B: f64 = 1.0 / 3.0;
const MITCHELL_C: f64 = 1.0 / 3.0;
/// Support radius of `image_filter_bessel`.
const BESSEL_RADIUS: f64 = 3.2383;

/// A filter kernel together with its parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum FilterShape {
    Bilinear,
    Hanning,
    Hamming,
    Hermite,
    Quadric,
    Bicubic,
    Kaiser { b: f64, i0_inv: f64 },
    Catrom,
    Mitchell(MitchellCoefficients),
    Spline16,
    Spline36,
    Gaussian,
    Bessel,
    Sinc { radius: f64 },
    Lanczos { radius: f64 },
    Blackman { radius: f64 },
}

/// The piecewise-cubic coefficients of the Mitchell filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MitchellCoefficients {
    p0: f64,
    p2: f64,
    p3: f64,
    q0: f64,
    q1: f64,
    q2: f64,
    q3: f64,
}

impl MitchellCoefficients {
    fn new(b: f64, c: f64) -> Self {
        Self {
            p0: (6.0 - 2.0 * b) / 6.0,
            p2: (-18.0 + 12.0 * b + 6.0 * c) / 6.0,
            p3: (12.0 - 9.0 * b - 6.0 * c) / 6.0,
            q0: (8.0 * b + 24.0 * c) / 6.0,
            q1: (-12.0 * b - 48.0 * c) / 6.0,
            q2: (6.0 * b + 30.0 * c) / 6.0,
            q3: (-b - 6.0 * c) / 6.0,
        }
    }
}

impl FilterShape {
    /// The kernel matplotlib uses for `interpolation`, or `None` for
    /// nearest-neighbour sampling. `radius` only affects the windowed sinc
    /// family, whose radius is clamped to at least 2 as in Agg.
    pub(super) fn for_interpolation(interpolation: Interpolation, radius: f64) -> Option<Self> {
        let windowed = if radius < 2.0 { 2.0 } else { radius };
        Some(match interpolation {
            Interpolation::Nearest => return None,
            Interpolation::Bilinear => Self::Bilinear,
            Interpolation::Bicubic => Self::Bicubic,
            Interpolation::Spline16 => Self::Spline16,
            Interpolation::Spline36 => Self::Spline36,
            Interpolation::Hanning => Self::Hanning,
            Interpolation::Hamming => Self::Hamming,
            Interpolation::Hermite => Self::Hermite,
            Interpolation::Kaiser => Self::Kaiser {
                b: KAISER_B,
                i0_inv: 1.0 / bessel_i0(KAISER_B),
            },
            Interpolation::Quadric => Self::Quadric,
            Interpolation::Catrom => Self::Catrom,
            Interpolation::Gaussian => Self::Gaussian,
            Interpolation::Bessel => Self::Bessel,
            Interpolation::Mitchell => {
                Self::Mitchell(MitchellCoefficients::new(MITCHELL_B, MITCHELL_C))
            }
            Interpolation::Sinc => Self::Sinc { radius: windowed },
            Interpolation::Lanczos => Self::Lanczos { radius: windowed },
            Interpolation::Blackman => Self::Blackman { radius: windowed },
        })
    }

    /// The support radius of the kernel.
    pub(super) fn radius(&self) -> f64 {
        match *self {
            Self::Bilinear
            | Self::Hanning
            | Self::Hamming
            | Self::Hermite
            | Self::Kaiser { .. } => 1.0,
            Self::Quadric => 1.5,
            Self::Bicubic | Self::Catrom | Self::Mitchell(_) | Self::Spline16 | Self::Gaussian => {
                2.0
            }
            Self::Spline36 => 3.0,
            Self::Bessel => BESSEL_RADIUS,
            Self::Sinc { radius } | Self::Lanczos { radius } | Self::Blackman { radius } => radius,
        }
    }

    /// The kernel's weight at distance `x >= 0` (`calc_weight`).
    pub(super) fn weight(&self, x: f64) -> f64 {
        match *self {
            Self::Bilinear => 1.0 - x,
            Self::Hanning => 0.5 + 0.5 * (PI * x).cos(),
            Self::Hamming => 0.54 + 0.46 * (PI * x).cos(),
            Self::Hermite => (2.0 * x - 3.0) * x * x + 1.0,
            Self::Quadric => quadric(x),
            Self::Bicubic => bicubic(x),
            Self::Kaiser { b, i0_inv } => bessel_i0(b * (1.0 - x * x).sqrt()) * i0_inv,
            Self::Catrom => catrom(x),
            Self::Mitchell(c) => mitchell(&c, x),
            Self::Spline16 => spline16(x),
            Self::Spline36 => spline36(x),
            Self::Gaussian => (-2.0 * x * x).exp() * (2.0 / PI).sqrt(),
            Self::Bessel => {
                if x == 0.0 {
                    PI / 4.0
                } else {
                    besj(PI * x, 1) / (2.0 * x)
                }
            }
            Self::Sinc { .. } => sinc(x),
            Self::Lanczos { radius } => windowed_sinc(x, radius, |xr| xr.sin() / xr),
            Self::Blackman { radius } => windowed_sinc(x, radius, |xr| {
                0.42 + 0.5 * xr.cos() + 0.08 * (2.0 * xr).cos()
            }),
        }
    }
}

fn quadric(x: f64) -> f64 {
    if x < 0.5 {
        0.75 - x * x
    } else if x < 1.5 {
        let t = x - 1.5;
        0.5 * t * t
    } else {
        0.0
    }
}

fn bicubic(x: f64) -> f64 {
    let pow3 = |v: f64| if v <= 0.0 { 0.0 } else { v * v * v };
    (1.0 / 6.0) * (pow3(x + 2.0) - 4.0 * pow3(x + 1.0) + 6.0 * pow3(x) - 4.0 * pow3(x - 1.0))
}

#[expect(
    clippy::manual_midpoint,
    reason = "Agg's polynomial, kept verbatim for identical rounding"
)]
fn catrom(x: f64) -> f64 {
    if x < 1.0 {
        0.5 * (2.0 + x * x * (-5.0 + x * 3.0))
    } else if x < 2.0 {
        0.5 * (4.0 + x * (-8.0 + x * (5.0 - x)))
    } else {
        0.0
    }
}

fn mitchell(c: &MitchellCoefficients, x: f64) -> f64 {
    if x < 1.0 {
        c.p0 + x * x * (c.p2 + x * c.p3)
    } else if x < 2.0 {
        c.q0 + x * (c.q1 + x * (c.q2 + x * c.q3))
    } else {
        0.0
    }
}

fn spline16(x: f64) -> f64 {
    if x < 1.0 {
        ((x - 9.0 / 5.0) * x - 1.0 / 5.0) * x + 1.0
    } else {
        let t = x - 1.0;
        ((-1.0 / 3.0 * t + 4.0 / 5.0) * t - 7.0 / 15.0) * t
    }
}

fn spline36(x: f64) -> f64 {
    if x < 1.0 {
        ((13.0 / 11.0 * x - 453.0 / 209.0) * x - 3.0 / 209.0) * x + 1.0
    } else if x < 2.0 {
        let t = x - 1.0;
        ((-6.0 / 11.0 * t + 270.0 / 209.0) * t - 156.0 / 209.0) * t
    } else {
        let t = x - 2.0;
        ((1.0 / 11.0 * t - 45.0 / 209.0) * t + 26.0 / 209.0) * t
    }
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }
    let x = x * PI;
    x.sin() / x
}

/// `sin(πx)/(πx)` multiplied by `window(πx/radius)`, zero beyond `radius`.
fn windowed_sinc(x: f64, radius: f64, window: impl Fn(f64) -> f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }
    if x > radius {
        return 0.0;
    }
    let x = x * PI;
    (x.sin() / x) * window(x / radius)
}

/// The modified Bessel function `I0`, by its power series
/// (`image_filter_kaiser::bessel_i0`).
fn bessel_i0(x: f64) -> f64 {
    const EPSILON: f64 = 1e-12;
    let mut sum = 1.0;
    let y = x * x / 4.0;
    let mut t = y;
    let mut i = 2;
    while t > EPSILON {
        sum += t;
        t *= y / f64::from(i * i);
        i += 1;
    }
    sum
}

/// The Bessel function of the first kind of order `n`, by downward
/// recurrence (Agg's `besj`).
fn besj(x: f64, n: i32) -> f64 {
    const D: f64 = 1e-6;
    if n < 0 {
        return 0.0;
    }
    if x.abs() <= D {
        return if n == 0 { 1.0 } else { 0.0 };
    }
    let mut b = 0.0;
    let mut b1 = 0.0;
    let mut m1 = x.abs() as i32 + 6;
    if x.abs() > 5.0 {
        m1 = (1.4 * x + 60.0 / x).abs() as i32;
    }
    let mut m2 = (f64::from(n + 2) + x.abs() / 4.0) as i32;
    if m1 > m2 {
        m2 = m1;
    }
    loop {
        let mut c3 = 0.0;
        let mut c2 = 1e-30;
        let mut c4 = 0.0;
        let mut m8 = if m2 / 2 * 2 == m2 { -1 } else { 1 };
        for i in 1..=m2 - 2 {
            let c6 = f64::from(2 * (m2 - i)) * c2 / x - c3;
            c3 = c2;
            c2 = c6;
            if m2 - i - 1 == n {
                b = c6;
            }
            m8 = -m8;
            if m8 > 0 {
                c4 += 2.0 * c6;
            }
        }
        let c6 = 2.0 * c2 / x - c3;
        if n == 0 {
            b = c6;
        }
        c4 += c6;
        b /= c4;
        if (b - b1).abs() < D {
            return b;
        }
        b1 = b;
        m2 += 3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(interpolation: Interpolation) -> FilterShape {
        FilterShape::for_interpolation(interpolation, 4.0).expect("not nearest")
    }

    #[test]
    fn nearest_has_no_kernel() {
        assert!(FilterShape::for_interpolation(Interpolation::Nearest, 1.0).is_none());
    }

    #[test]
    fn kernels_peak_at_zero() {
        for &interpolation in Interpolation::ALL.iter().skip(1) {
            let k = shape(interpolation);
            let w0 = k.weight(0.0);
            assert!(w0 > 0.0, "{interpolation:?}");
            assert!(k.weight(0.25) <= w0 + 1e-12, "{interpolation:?}");
        }
    }

    #[test]
    fn interpolating_kernels_vanish_at_integers() {
        for interpolation in [
            Interpolation::Bilinear,
            Interpolation::Catrom,
            Interpolation::Spline16,
            Interpolation::Spline36,
        ] {
            let k = shape(interpolation);
            assert!((k.weight(0.0) - 1.0).abs() < 1e-12, "{interpolation:?}");
            assert!(k.weight(1.0).abs() < 1e-12, "{interpolation:?}");
        }
    }

    #[test]
    fn windowed_radius_is_clamped() {
        let k = FilterShape::for_interpolation(Interpolation::Lanczos, 1.0).expect("filter");
        assert_eq!(k.radius(), 2.0);
        assert_eq!(k.weight(2.5), 0.0);
        let k = FilterShape::for_interpolation(Interpolation::Sinc, 5.5).expect("filter");
        assert_eq!(k.radius(), 5.5);
    }

    #[test]
    fn bessel_functions_match_known_values() {
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008_4).abs() < 1e-12);
        assert!((besj(1.0, 1) - 0.440_050_585_744_933_5).abs() < 1e-6);
        assert!((besj(7.5, 1) - 0.135_248_427_579_6).abs() < 1e-6);
        assert!((shape(Interpolation::Bessel).weight(0.0) - PI / 4.0).abs() < 1e-15);
    }
}
