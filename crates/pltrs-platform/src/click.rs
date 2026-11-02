//! Multi-click detection for platforms that only report single presses.

use crate::event::MouseButton;
use crate::geometry::PhysicalPosition;

/// Longest gap between two presses of a multi-click, in milliseconds,
/// unless the desktop says otherwise (GTK's default double-click time).
const MAX_INTERVAL_MS: u32 = 400;
/// Farthest the pointer may travel between two presses of a multi-click,
/// in physical pixels.
const MAX_DISTANCE: f64 = 4.0;

#[derive(Debug, Clone, Copy)]
struct LastPress {
    button: MouseButton,
    time_ms: u32,
    position: PhysicalPosition,
    count: u32,
}

/// Counts consecutive presses of the same button that are close in time
/// and space (1 = single click, 2 = double click, ...).
///
/// Timestamps are platform millisecond counters that wrap around, as X11,
/// Wayland and Windows deliver them.
#[derive(Debug)]
pub(crate) struct ClickCounter {
    /// Longest gap between two presses of a multi-click, in milliseconds.
    max_interval_ms: u32,
    last: Option<LastPress>,
}

impl Default for ClickCounter {
    fn default() -> Self {
        Self::new(MAX_INTERVAL_MS)
    }
}

impl ClickCounter {
    /// A counter taking presses at most `max_interval_ms` milliseconds
    /// apart for multi-clicks (the desktop's double-click time).
    pub(crate) const fn new(max_interval_ms: u32) -> Self {
        Self {
            max_interval_ms,
            last: None,
        }
    }

    /// Records a press and returns its click count.
    pub(crate) fn press(
        &mut self,
        button: MouseButton,
        time_ms: u32,
        position: PhysicalPosition,
    ) -> u32 {
        let count = match self.last {
            Some(last)
                if last.button == button
                    && time_ms.wrapping_sub(last.time_ms) <= self.max_interval_ms
                    && (position.x - last.position.x).hypot(position.y - last.position.y)
                        <= MAX_DISTANCE =>
            {
                last.count.saturating_add(1)
            }
            _ => 1,
        };
        self.last = Some(LastPress {
            button,
            time_ms,
            position,
            count,
        });
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: PhysicalPosition = PhysicalPosition::new(10.0, 10.0);

    #[test]
    fn quick_presses_count_up() {
        let mut clicks = ClickCounter::default();
        assert_eq!(clicks.press(MouseButton::Left, 1000, ORIGIN), 1);
        assert_eq!(clicks.press(MouseButton::Left, 1200, ORIGIN), 2);
        assert_eq!(clicks.press(MouseButton::Left, 1500, ORIGIN), 3);
    }

    #[test]
    fn slow_far_or_different_presses_restart() {
        let mut clicks = ClickCounter::default();
        clicks.press(MouseButton::Left, 0, ORIGIN);
        assert_eq!(clicks.press(MouseButton::Left, 401, ORIGIN), 1, "too slow");
        let far = PhysicalPosition::new(20.0, 10.0);
        assert_eq!(clicks.press(MouseButton::Left, 500, far), 1, "too far");
        assert_eq!(
            clicks.press(MouseButton::Right, 600, far),
            1,
            "other button"
        );
    }

    #[test]
    fn the_interval_follows_the_desktop() {
        let mut clicks = ClickCounter::new(500);
        clicks.press(MouseButton::Left, 0, ORIGIN);
        assert_eq!(clicks.press(MouseButton::Left, 500, ORIGIN), 2);
        assert_eq!(clicks.press(MouseButton::Left, 1001, ORIGIN), 1);
    }

    #[test]
    fn timestamps_may_wrap() {
        let mut clicks = ClickCounter::default();
        clicks.press(MouseButton::Middle, u32::MAX - 100, ORIGIN);
        assert_eq!(clicks.press(MouseButton::Middle, 100, ORIGIN), 2);
    }
}
