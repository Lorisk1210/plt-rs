//! Sizes, positions and rectangles in logical and physical pixels.
//!
//! *Physical* pixels are device pixels: frames are presented and pointer
//! positions are reported in them. *Logical* pixels are physical pixels
//! divided by the window's scale factor (1.0 on a 96 DPI display, 2.0 on a
//! typical high-DPI display). Window sizes may be requested in either unit.

/// A size in physical (device) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PhysicalSize {
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
}

impl PhysicalSize {
    /// Creates a size from a width and a height.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Number of pixels covered (`width * height`).
    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Whether either dimension is zero.
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Converts to logical pixels for the given scale factor.
    pub fn to_logical(self, scale_factor: f64) -> LogicalSize {
        LogicalSize::new(
            f64::from(self.width) / scale_factor,
            f64::from(self.height) / scale_factor,
        )
    }
}

/// A size in logical pixels (physical pixels divided by the scale factor).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LogicalSize {
    /// Width in logical pixels.
    pub width: f64,
    /// Height in logical pixels.
    pub height: f64,
}

impl LogicalSize {
    /// Creates a size from a width and a height.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Converts to physical pixels, rounding to the nearest pixel.
    ///
    /// Negative and NaN components become zero; huge values saturate.
    pub fn to_physical(self, scale_factor: f64) -> PhysicalSize {
        PhysicalSize::new(
            scale_to_pixels(self.width, scale_factor),
            scale_to_pixels(self.height, scale_factor),
        )
    }
}

fn scale_to_pixels(value: f64, scale_factor: f64) -> u32 {
    let scaled = (value * scale_factor).round();
    if scaled.is_nan() || scaled <= 0.0 {
        0
    } else if scaled >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        scaled as u32
    }
}

/// A window size expressed in either logical or physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// Logical pixels; scaled by the window's scale factor.
    Logical(LogicalSize),
    /// Physical pixels; used as-is.
    Physical(PhysicalSize),
}

impl Size {
    /// Resolves the size to physical pixels for the given scale factor.
    pub fn to_physical(self, scale_factor: f64) -> PhysicalSize {
        match self {
            Self::Logical(size) => size.to_physical(scale_factor),
            Self::Physical(size) => size,
        }
    }
}

impl From<LogicalSize> for Size {
    fn from(size: LogicalSize) -> Self {
        Self::Logical(size)
    }
}

impl From<PhysicalSize> for Size {
    fn from(size: PhysicalSize) -> Self {
        Self::Physical(size)
    }
}

/// A position in physical pixels relative to the top-left corner of a
/// window's content area (y grows downwards).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhysicalPosition {
    /// Horizontal offset in device pixels.
    pub x: f64,
    /// Vertical offset in device pixels, growing downwards.
    pub y: f64,
}

impl PhysicalPosition {
    /// Creates a position from its coordinates.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle in physical pixels (origin top-left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PhysicalRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PhysicalRect {
    /// Creates a rectangle from its top-left corner and size.
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The rectangle covering a whole surface of `size`.
    pub const fn from_size(size: PhysicalSize) -> Self {
        Self::new(0, 0, size.width, size.height)
    }

    /// Whether the rectangle covers no pixels.
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// The part of this rectangle inside a surface of `bounds` (possibly
    /// empty).
    #[must_use]
    pub fn clip_to(self, bounds: PhysicalSize) -> Self {
        let x0 = self.x.min(bounds.width);
        let y0 = self.y.min(bounds.height);
        let x1 = self.x.saturating_add(self.width).min(bounds.width);
        let y1 = self.y.saturating_add(self.height).min(bounds.height);
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_physical_round_trip() {
        let logical = LogicalSize::new(640.0, 480.0);
        assert_eq!(logical.to_physical(2.0), PhysicalSize::new(1280, 960));
        assert_eq!(PhysicalSize::new(1280, 960).to_logical(2.0), logical);
        assert_eq!(
            LogicalSize::new(100.4, 0.6).to_physical(1.0),
            PhysicalSize::new(100, 1)
        );
    }

    #[test]
    fn degenerate_logical_sizes_saturate() {
        assert_eq!(
            LogicalSize::new(f64::NAN, -3.0).to_physical(1.0),
            PhysicalSize::new(0, 0)
        );
        assert_eq!(
            LogicalSize::new(f64::INFINITY, 1e300).to_physical(1.0),
            PhysicalSize::new(u32::MAX, u32::MAX)
        );
    }

    #[test]
    fn size_enum_resolves_units() {
        let logical: Size = LogicalSize::new(10.0, 20.0).into();
        let physical: Size = PhysicalSize::new(10, 20).into();
        assert_eq!(logical.to_physical(1.5), PhysicalSize::new(15, 30));
        assert_eq!(physical.to_physical(1.5), PhysicalSize::new(10, 20));
    }

    #[test]
    fn rect_clipping() {
        let bounds = PhysicalSize::new(100, 50);
        assert_eq!(
            PhysicalRect::new(90, 40, 20, 20).clip_to(bounds),
            PhysicalRect::new(90, 40, 10, 10)
        );
        assert!(PhysicalRect::new(200, 0, 5, 5).clip_to(bounds).is_empty());
        assert_eq!(
            PhysicalRect::new(0, 0, u32::MAX, u32::MAX).clip_to(bounds),
            PhysicalRect::from_size(bounds)
        );
    }

    #[test]
    fn size_helpers() {
        assert_eq!(PhysicalSize::new(3, 4).area(), 12);
        assert!(PhysicalSize::new(0, 4).is_empty());
        assert!(!PhysicalSize::new(1, 1).is_empty());
    }
}
