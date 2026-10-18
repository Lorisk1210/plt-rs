//! Sizes, rectangles and layout constraints, in window pixels (physical
//! pixels, origin at the top-left corner, y down).

use plt_core::Point;

/// A width and height in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Size {
    /// The empty size.
    pub const ZERO: Self = Self::new(0.0, 0.0);
    /// An unbounded size, for constraints without a maximum.
    pub const INFINITE: Self = Self::new(f64::INFINITY, f64::INFINITY);

    /// Creates a size.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width (never negative for rectangles built by this crate).
    pub width: f64,
    /// Height (never negative for rectangles built by this crate).
    pub height: f64,
}

impl Rect {
    /// Creates a rectangle from its top-left corner and size.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// A rectangle at `origin` of `size`.
    pub const fn from_origin_size(origin: Point, size: Size) -> Self {
        Self::new(origin.x, origin.y, size.width, size.height)
    }

    /// The top-left corner.
    pub const fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// The size.
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    /// The right edge.
    pub fn max_x(self) -> f64 {
        self.x + self.width
    }

    /// The bottom edge.
    pub fn max_y(self) -> f64 {
        self.y + self.height
    }

    /// The centre point.
    pub fn center(self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Whether `point` lies inside (left and top edges inclusive, right
    /// and bottom edges exclusive).
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.x < self.max_x() && point.y >= self.y && point.y < self.max_y()
    }

    /// A rectangle of `size` centred in this one.
    #[must_use]
    pub fn centered(self, size: Size) -> Self {
        Self::new(
            self.x + (self.width - size.width) / 2.0,
            self.y + (self.height - size.height) / 2.0,
            size.width,
            size.height,
        )
    }

    /// The rectangle shrunk by `amount` on every side (never below an
    /// empty rectangle at the centre); a negative `amount` grows it.
    #[must_use]
    pub fn inset(self, amount: f64) -> Self {
        let dx = amount.min(self.width / 2.0);
        let dy = amount.min(self.height / 2.0);
        Self::new(
            self.x + dx,
            self.y + dy,
            self.width - 2.0 * dx,
            self.height - 2.0 * dy,
        )
    }

    /// The rectangle with its edges moved to the nearest whole pixels, so
    /// that outlines drawn along them stay crisp.
    #[must_use]
    pub fn snapped(self) -> Self {
        let (x0, y0) = (self.x.round(), self.y.round());
        let (x1, y1) = (self.max_x().round(), self.max_y().round());
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// The overlap of two rectangles (empty if they are disjoint).
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.max_x().min(other.max_x()) - x).max(0.0);
        let height = (self.max_y().min(other.max_y()) - y).max(0.0);
        Self::new(x, y, width, height)
    }

    /// Whether the rectangle covers no area.
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0 || self.width.is_nan() || self.height.is_nan()
    }
}

/// The sizes a widget may take during layout: from `min` to `max`, per
/// axis (`max` may be infinite).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    /// Smallest allowed size.
    pub min: Size,
    /// Largest allowed size.
    pub max: Size,
}

impl Constraints {
    /// Exactly `size`.
    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    /// Anything from nothing up to `max`.
    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::ZERO,
            max,
        }
    }

    /// The size closest to `size` that satisfies the constraints (NaN
    /// becomes the minimum).
    pub fn constrain(&self, size: Size) -> Size {
        let clamp = |value: f64, min: f64, max: f64| {
            if value.is_nan() {
                min
            } else {
                value.max(min).min(max.max(min))
            }
        };
        Size::new(
            clamp(size.width, self.min.width, self.max.width),
            clamp(size.height, self.min.height, self.max.height),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_queries() {
        let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!((rect.max_x(), rect.max_y()), (40.0, 60.0));
        assert_eq!(rect.center(), Point::new(25.0, 40.0));
        assert!(rect.contains(Point::new(10.0, 20.0)));
        assert!(
            !rect.contains(Point::new(40.0, 30.0)),
            "right edge is exclusive"
        );
        assert!(!rect.contains(Point::new(f64::NAN, 30.0)));
        assert_eq!(rect.inset(5.0), Rect::new(15.0, 25.0, 20.0, 30.0));
        assert_eq!(rect.inset(100.0), Rect::new(25.0, 40.0, 0.0, 0.0));
        assert_eq!(rect.inset(-2.0), Rect::new(8.0, 18.0, 34.0, 44.0));
        assert_eq!(
            Rect::new(0.4, 0.6, 10.2, 9.8).snapped(),
            Rect::new(0.0, 1.0, 11.0, 9.0)
        );
        assert_eq!(
            rect.centered(Size::new(10.0, 20.0)),
            Rect::new(20.0, 30.0, 10.0, 20.0)
        );
    }

    #[test]
    fn intersections() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            a.intersect(Rect::new(5.0, 6.0, 10.0, 10.0)),
            Rect::new(5.0, 6.0, 5.0, 4.0)
        );
        assert!(a.intersect(Rect::new(20.0, 0.0, 5.0, 5.0)).is_empty());
        assert!(Rect::new(0.0, 0.0, f64::NAN, 1.0).is_empty());
    }

    #[test]
    fn constraints_clamp_sizes() {
        let constraints = Constraints {
            min: Size::new(10.0, 10.0),
            max: Size::new(100.0, f64::INFINITY),
        };
        assert_eq!(
            constraints.constrain(Size::new(5.0, 500.0)),
            Size::new(10.0, 500.0)
        );
        assert_eq!(
            constraints.constrain(Size::new(f64::NAN, 50.0)),
            Size::new(10.0, 50.0)
        );
        let tight = Constraints::tight(Size::new(3.0, 4.0));
        assert_eq!(tight.constrain(Size::INFINITE), Size::new(3.0, 4.0));
        assert_eq!(
            Constraints::loose(Size::new(8.0, 8.0)).constrain(Size::ZERO),
            Size::ZERO
        );
    }
}
