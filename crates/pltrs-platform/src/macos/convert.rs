//! Conversions between `AppKit`'s units and pltrs's: points and physical
//! pixels, wait deadlines and seconds, cursors and their `NSCursor`
//! names.

use std::ffi::CStr;
use std::time::Instant;

use crate::backend::Wait;
use crate::geometry::{LogicalSize, PhysicalRect, PhysicalSize, Size};
use crate::window::CursorIcon;

/// A rectangle in points (a view's flipped coordinates: y grows
/// downwards).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct PointRect {
    /// Left edge.
    pub(crate) x: f64,
    /// Top edge.
    pub(crate) y: f64,
    /// Width.
    pub(crate) width: f64,
    /// Height.
    pub(crate) height: f64,
}

impl PointRect {
    /// The physical pixels this rectangle touches at `scale`.
    pub(crate) fn to_pixels(self, scale: f64) -> PhysicalRect {
        let edge = |value: f64| {
            if value.is_nan() || value <= 0.0 {
                0
            } else {
                value.min(f64::from(u32::MAX)) as u32
            }
        };
        let (left, top) = (
            edge((self.x * scale).floor()),
            edge((self.y * scale).floor()),
        );
        let right = edge(((self.x + self.width) * scale).ceil());
        let bottom = edge(((self.y + self.height) * scale).ceil());
        PhysicalRect::new(
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        )
    }

    /// The rectangle of physical pixels `rect` in points at `scale`.
    pub(crate) fn from_pixels(rect: PhysicalRect, scale: f64) -> Self {
        Self {
            x: f64::from(rect.x) / scale,
            y: f64::from(rect.y) / scale,
            width: f64::from(rect.width) / scale,
            height: f64::from(rect.height) / scale,
        }
    }
}

/// The physical size of a view `width` × `height` points large.
pub(crate) fn pixel_size(width: f64, height: f64, scale: f64) -> PhysicalSize {
    LogicalSize::new(width, height).to_physical(scale)
}

/// `size` in points at `scale`: logical sizes are points already.
pub(crate) fn points(size: Size, scale: f64) -> LogicalSize {
    match size {
        Size::Logical(size) => size,
        Size::Physical(size) => size.to_logical(scale),
    }
}

/// How many seconds to wait for events at `now`: `None` when not to wait
/// at all, infinity without a deadline.
pub(crate) fn seconds_until(wait: Wait, now: Instant) -> Option<f64> {
    match wait {
        Wait::Poll => None,
        Wait::Forever => Some(f64::INFINITY),
        Wait::Until(deadline) => {
            let left = deadline.checked_duration_since(now)?;
            (!left.is_zero()).then_some(left.as_secs_f64())
        }
    }
}

/// The `NSCursor` class method giving the cursor for `icon`. macOS has no
/// busy cursor for applications (it shows its own when one stops
/// answering), so waiting shows the arrow, as in matplotlib.
pub(crate) const fn cursor_name(icon: CursorIcon) -> &'static CStr {
    match icon {
        CursorIcon::Default | CursorIcon::Wait => c"arrowCursor",
        CursorIcon::Pointer => c"pointingHandCursor",
        CursorIcon::Crosshair => c"crosshairCursor",
        CursorIcon::Move => c"openHandCursor",
        CursorIcon::Text => c"IBeamCursor",
        CursorIcon::ResizeHorizontal => c"resizeLeftRightCursor",
        CursorIcon::ResizeVertical => c"resizeUpDownCursor",
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn point_rects_cover_whole_pixels() {
        let rect = PointRect {
            x: 0.5,
            y: 1.0,
            width: 2.0,
            height: 0.25,
        };
        assert_eq!(rect.to_pixels(1.0), PhysicalRect::new(0, 1, 3, 1));
        assert_eq!(rect.to_pixels(2.0), PhysicalRect::new(1, 2, 4, 1));
        let negative = PointRect {
            x: -5.0,
            y: f64::NAN,
            width: 10.0,
            height: 1.0,
        };
        assert_eq!(negative.to_pixels(1.0), PhysicalRect::new(0, 0, 5, 0));
        let pixels = PhysicalRect::new(2, 4, 6, 8);
        let back = PointRect::from_pixels(pixels, 2.0);
        assert_eq!(
            back,
            PointRect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0
            }
        );
        assert_eq!(back.to_pixels(2.0), pixels);
    }

    #[test]
    fn sizes_convert_between_points_and_pixels() {
        assert_eq!(pixel_size(640.0, 480.0, 2.0), PhysicalSize::new(1280, 960));
        let physical = Size::Physical(PhysicalSize::new(300, 200));
        assert_eq!(points(physical, 2.0), LogicalSize::new(150.0, 100.0));
        let logical = Size::Logical(LogicalSize::new(10.0, 20.0));
        assert_eq!(points(logical, 2.0), LogicalSize::new(10.0, 20.0));
    }

    #[test]
    fn waits_become_seconds() {
        let now = Instant::now();
        assert_eq!(seconds_until(Wait::Poll, now), None);
        assert_eq!(seconds_until(Wait::Forever, now), Some(f64::INFINITY));
        assert_eq!(seconds_until(Wait::Until(now), now), None);
        let later = now + Duration::from_millis(250);
        assert_eq!(seconds_until(Wait::Until(later), now), Some(0.25));
        assert_eq!(seconds_until(Wait::Until(now), later), None);
    }

    #[test]
    fn cursors_have_appkit_names() {
        assert_eq!(cursor_name(CursorIcon::Default), c"arrowCursor");
        assert_eq!(cursor_name(CursorIcon::Wait), c"arrowCursor");
        assert_eq!(cursor_name(CursorIcon::Move), c"openHandCursor");
        assert_eq!(cursor_name(CursorIcon::Text), c"IBeamCursor");
    }
}
