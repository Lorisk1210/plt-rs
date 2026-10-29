//! A minimal double-precision complex number.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// A complex number `re + i·im` in binary64, the element type of FFT
/// results and complex spectra (numpy `complex128`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

impl Complex {
    /// Zero.
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// One.
    pub const ONE: Self = Self::new(1.0, 0.0);
    /// The imaginary unit.
    pub const I: Self = Self::new(0.0, 1.0);

    /// Builds `re + i·im`.
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// A purely real number.
    pub const fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }

    /// The complex conjugate `re - i·im`.
    #[must_use]
    pub const fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    /// The modulus `|z|`, computed without undue overflow (numpy `abs`).
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// The squared modulus `re² + im²`.
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// The argument in `(-π, π]` (numpy `angle`).
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    /// Multiplies both parts by a real factor.
    #[must_use]
    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.re * factor, self.im * factor)
    }

    /// Divides both parts by a real divisor.
    #[must_use]
    pub fn unscale(self, divisor: f64) -> Self {
        Self::new(self.re / divisor, self.im / divisor)
    }

    /// `true` when both parts are finite.
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

impl From<f64> for Complex {
    fn from(re: f64) -> Self {
        Self::real(re)
    }
}

impl fmt::Display for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im.is_sign_negative() {
            write!(f, "{}-{}i", self.re, -self.im)
        } else {
            write!(f, "{}+{}i", self.re, self.im)
        }
    }
}

impl Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl AddAssign for Complex {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl SubAssign for Complex {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl MulAssign for Complex {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Mul<f64> for Complex {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        self.scale(rhs)
    }
}

impl Div<f64> for Complex {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        self.unscale(rhs)
    }
}

impl Div for Complex {
    type Output = Self;

    /// Smith's algorithm, as numpy uses for `complex128` division.
    fn div(self, rhs: Self) -> Self {
        let (a, b, c, d) = (self.re, self.im, rhs.re, rhs.im);
        if c.abs() >= d.abs() {
            if c == 0.0 && d == 0.0 {
                return Self::new(a / c.abs(), b / c.abs());
            }
            let rat = d / c;
            let scl = 1.0 / (c + d * rat);
            Self::new((a + b * rat) * scl, (b - a * rat) * scl)
        } else {
            let rat = c / d;
            let scl = 1.0 / (d + c * rat);
            Self::new((a * rat + b) * scl, (b * rat - a) * scl)
        }
    }
}

impl Neg for Complex {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.re, -self.im)
    }
}

impl Sum for Complex {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, -1.0);
        assert_eq!(a + b, Complex::new(4.0, 1.0));
        assert_eq!(a - b, Complex::new(-2.0, 3.0));
        assert_eq!(a * b, Complex::new(5.0, 5.0));
        assert!(((a * b) / b - a).abs() < 1e-15);
        assert_eq!(-a, Complex::new(-1.0, -2.0));
        assert_eq!(a.conj(), Complex::new(1.0, -2.0));
        assert_eq!(a * 2.0, Complex::new(2.0, 4.0));
        assert_eq!(a / 2.0, Complex::new(0.5, 1.0));
        assert_eq!([a, b].into_iter().sum::<Complex>(), a + b);
    }

    #[test]
    fn polar_parts() {
        assert_eq!(Complex::new(3.0, 4.0).abs(), 5.0);
        assert_eq!(Complex::new(3.0, 4.0).norm_sqr(), 25.0);
        assert_eq!(Complex::new(-1.0, 0.0).arg(), std::f64::consts::PI);
        assert_eq!(Complex::I.arg(), std::f64::consts::FRAC_PI_2);
        assert!(!Complex::new(f64::NAN, 0.0).is_finite());
        assert_eq!(Complex::new(1.0, -2.0).to_string(), "1-2i");
    }

    #[test]
    fn division_by_zero_and_imaginary() {
        let q = Complex::ONE / Complex::I;
        assert_eq!(q, Complex::new(0.0, -1.0));
        assert!((Complex::ONE / Complex::ZERO).re.is_infinite());
    }
}
