//! Two-dimensional points and vectors.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A point (or displacement vector) in a 2-D coordinate space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl Point {
    /// The origin `(0, 0)`.
    pub const ORIGIN: Self = Self::new(0.0, 0.0);

    /// Creates a point from its coordinates.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Returns `true` when both coordinates are finite (not NaN or ±∞).
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Euclidean length of the point interpreted as a vector.
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    /// Euclidean distance to `other`.
    pub fn distance(self, other: Self) -> f64 {
        (other - self).length()
    }

    /// Dot product of the two points interpreted as vectors.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Z component of the cross product of the two vectors.
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Linear interpolation: `self` at `t = 0`, `other` at `t = 1`.
    #[must_use]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self + (other - self) * t
    }

    /// Component-wise minimum.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Component-wise maximum.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }
}

impl From<(f64, f64)> for Point {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl From<[f64; 2]> for Point {
    fn from([x, y]: [f64; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Point> for (f64, f64) {
    fn from(p: Point) -> Self {
        (p.x, p.y)
    }
}

impl Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f64> for Point {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f64> for Point {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Point {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let a = Point::new(1.0, 2.0);
        let b = Point::new(4.0, 6.0);
        assert_eq!(a + b, Point::new(5.0, 8.0));
        assert_eq!(b - a, Point::new(3.0, 4.0));
        assert_eq!(a * 2.0, Point::new(2.0, 4.0));
        assert_eq!(-a, Point::new(-1.0, -2.0));
        assert_eq!(a.distance(b), 5.0);
        assert_eq!(a.lerp(b, 0.5), Point::new(2.5, 4.0));
        assert_eq!(a.cross(b), -2.0);
    }

    #[test]
    fn finiteness() {
        assert!(Point::new(1.0, 2.0).is_finite());
        assert!(!Point::new(f64::NAN, 2.0).is_finite());
        assert!(!Point::new(1.0, f64::INFINITY).is_finite());
    }
}
